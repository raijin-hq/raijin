use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use inazuma::{App, AppContext, Context, Entity, Global, Task};
use inazuma_collections::HashMap;
use raijin_client::{Client, UserStore};
use raijin_language::LanguageRegistry;
use raijin_node_runtime::NodeRuntime;
use raijin_project::Project;

/// Shared context for a single git repository.
///
/// Created when the first terminal tab enters a repo,
/// disposed when the last tab leaves it.
pub struct ProjectContext {
    git_root: PathBuf,
    project: Entity<Project>,
    ref_count: usize,
}

impl ProjectContext {
    pub fn git_root(&self) -> &PathBuf {
        &self.git_root
    }

    pub fn project(&self) -> &Entity<Project> {
        &self.project
    }
}

/// Global singleton managing ProjectContexts per git root.
///
/// Terminal tabs call `acquire()` when entering a git repo and
/// `release()` when leaving. The registry handles Project lifecycle
/// (creation, worktree scanning, LSP startup, disposal).
pub struct ProjectRegistry {
    contexts: HashMap<PathBuf, Entity<ProjectContext>>,
    client: Arc<Client>,
    user_store: Entity<UserStore>,
    languages: Arc<LanguageRegistry>,
    fs: Arc<dyn raijin_fs::Fs>,
    node_runtime: NodeRuntime,
}

struct GlobalProjectRegistry(Entity<ProjectRegistry>);
impl Global for GlobalProjectRegistry {}

impl ProjectRegistry {
    pub fn init(
        client: Arc<Client>,
        user_store: Entity<UserStore>,
        languages: Arc<LanguageRegistry>,
        fs: Arc<dyn raijin_fs::Fs>,
        node_runtime: NodeRuntime,
        cx: &mut App,
    ) {
        let registry = cx.new(|_cx| Self {
            contexts: HashMap::default(),
            client,
            user_store,
            languages,
            fs,
            node_runtime,
        });
        cx.set_global(GlobalProjectRegistry(registry));
    }

    pub fn global(cx: &App) -> Entity<Self> {
        cx.global::<GlobalProjectRegistry>().0.clone()
    }

    pub fn try_global(cx: &App) -> Option<Entity<Self>> {
        cx.try_global::<GlobalProjectRegistry>()
            .map(|g| g.0.clone())
    }

    /// Acquire a ProjectContext for the given git root.
    /// If one already exists, increments ref_count and returns it.
    /// If not, creates a new Project with a worktree for the git root.
    pub fn acquire(
        &mut self,
        git_root: &PathBuf,
        cx: &mut Context<Self>,
    ) -> (Entity<ProjectContext>, Task<Result<()>>) {
        if let Some(existing) = self.contexts.get(git_root) {
            existing.update(cx, |ctx, _cx| {
                ctx.ref_count += 1;
                log::debug!(
                    "ProjectRegistry: reuse {:?} (ref_count={})",
                    ctx.git_root,
                    ctx.ref_count
                );
            });
            return (existing.clone(), Task::ready(Ok(())));
        }

        let project = Project::local(
            self.client.clone(),
            self.node_runtime.clone(),
            self.user_store.clone(),
            self.languages.clone(),
            self.fs.clone(),
            None,
            raijin_project::LocalProjectFlags::default(),
            cx,
        );

        let git_root_clone = git_root.clone();
        let worktree_task = project.update(cx, |project, cx| {
            project.find_or_create_worktree(&git_root_clone, true, cx)
        });

        let scan_task = cx.background_spawn(async move {
            worktree_task.await?;
            Ok(())
        });

        let context = cx.new(|_cx| ProjectContext {
            git_root: git_root.clone(),
            project,
            ref_count: 1,
        });

        log::info!("ProjectRegistry: created {:?}", git_root);
        self.contexts.insert(git_root.clone(), context.clone());

        (context, scan_task)
    }

    /// Release a ProjectContext for the given git root.
    /// Decrements ref_count. When it reaches 0, the ProjectContext
    /// is removed and its Project disposed.
    pub fn release(&mut self, git_root: &PathBuf, cx: &mut Context<Self>) {
        let should_remove = if let Some(context) = self.contexts.get(git_root) {
            context.update(cx, |ctx, _cx| {
                ctx.ref_count = ctx.ref_count.saturating_sub(1);
                log::debug!(
                    "ProjectRegistry: release {:?} (ref_count={})",
                    ctx.git_root,
                    ctx.ref_count
                );
                ctx.ref_count == 0
            })
        } else {
            false
        };

        if should_remove {
            log::info!("ProjectRegistry: disposed {:?}", git_root);
            self.contexts.remove(git_root);
        }
    }

    /// Get the ProjectContext for a git root without changing ref_count.
    pub fn get(&self, git_root: &PathBuf) -> Option<&Entity<ProjectContext>> {
        self.contexts.get(git_root)
    }

    /// All active ProjectContexts.
    pub fn contexts(&self) -> impl Iterator<Item = (&PathBuf, &Entity<ProjectContext>)> {
        self.contexts.iter()
    }
}
