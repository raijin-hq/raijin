use std::path::PathBuf;
use std::sync::Arc;

use inazuma_collections::HashMap;

use anyhow::{Context as _, Result};
use inazuma::{
    App, AppContext as _, AsyncApp, Context, Entity, PromptLevel, Task, WindowBounds, WindowHandle,
};
use inazuma_settings_framework::Settings;

use raijin_workspace::{
    AppState, CloseIntent, ItemHandle, OpenVisible,
    SerializedWorkspaceLocation, Workspace, WorkspaceDb, WorkspaceId, WorkspaceLocation,
    WorkspaceSettings, open_items,
    read_default_dock_state, read_default_window_bounds, window_bounds_env_override,
};
use raijin_project::{Project, ProjectPath};
use raijin_remote::RemoteConnectionOptions;
use inazuma_util::ResultExt;

use crate::AppShell;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn notify_if_database_failed(workspace: Entity<Workspace>, cx: &mut inazuma::AsyncApp) {
    let _ = workspace
        .update(cx, |workspace, cx| {
            if (*raijin_db::ALL_FILE_DB_FAILED).load(std::sync::atomic::Ordering::Acquire) {
                use raijin_workspace::notifications::{
                    NotificationId,
                    simple_message_notification::MessageNotification,
                };

                workspace.show_notification(
                    NotificationId::unique::<DatabaseFailedNotification>(),
                    cx,
                    |cx| {
                        cx.new(|cx| {
                            MessageNotification::new("Failed to load the database file.", cx)
                                .primary_message("File an Issue")
                                .primary_icon(raijin_ui::IconName::Plus)
                                .primary_on_click(|window, cx| {
                                    window.dispatch_action(
                                        Box::new(raijin_actions::feedback::FileBugReport),
                                        cx,
                                    )
                                })
                        })
                    },
                );
            }
        });
}

struct DatabaseFailedNotification;

// ---------------------------------------------------------------------------
// OpenOptions / OpenResult — the public types for window-opening functions
// ---------------------------------------------------------------------------

#[derive(Default, Clone)]
pub struct OpenOptions {
    pub visible: Option<OpenVisible>,
    pub focus: Option<bool>,
    pub open_new_workspace: Option<bool>,
    pub wait: bool,
    pub replace_window: Option<WindowHandle<AppShell>>,
    pub env: Option<HashMap<String, String>>,
}

/// The result of opening a workspace via [`open_paths`], [`new_local`],
/// or [`Workspace::open_workspace_for_paths`].
pub struct OpenResult {
    pub window: WindowHandle<AppShell>,
    pub workspace: Entity<Workspace>,
    pub opened_items: Vec<Option<anyhow::Result<Box<dyn ItemHandle>>>>,
}

// ---------------------------------------------------------------------------
// Window-finding helpers
// ---------------------------------------------------------------------------

pub fn activate_any_workspace_window(cx: &mut AsyncApp) -> Option<WindowHandle<AppShell>> {
    cx.update(|cx| {
        if let Some(workspace_window) = cx
            .active_window()
            .and_then(|window| window.downcast::<AppShell>())
        {
            return Some(workspace_window);
        }

        for window in cx.windows() {
            if let Some(workspace_window) = window.downcast::<AppShell>() {
                workspace_window
                    .update(cx, |_, window, _| window.activate_window())
                    .ok();
                return Some(workspace_window);
            }
        }
        None
    })
}

pub async fn get_any_active_workspace(
    app_state: Arc<AppState>,
    mut cx: AsyncApp,
) -> anyhow::Result<WindowHandle<AppShell>> {
    // find an existing workspace to focus and show call controls
    let active_window = activate_any_workspace_window(&mut cx);
    if active_window.is_none() {
        cx.update(|cx| new_local(vec![], app_state.clone(), None, None, None, true, cx))
            .await?;
    }
    activate_any_workspace_window(&mut cx).context("could not open raijin")
}

pub fn local_workspace_windows(cx: &App) -> Vec<WindowHandle<AppShell>> {
    workspace_windows_for_location(&SerializedWorkspaceLocation::Local, cx)
}

pub fn workspace_windows_for_location(
    serialized_location: &SerializedWorkspaceLocation,
    cx: &App,
) -> Vec<WindowHandle<AppShell>> {
    cx.windows()
        .into_iter()
        .filter_map(|window| window.downcast::<AppShell>())
        .filter(|shell_window| {
            let same_host = |left: &RemoteConnectionOptions, right: &RemoteConnectionOptions| match (left, right) {
                (RemoteConnectionOptions::Ssh(a), RemoteConnectionOptions::Ssh(b)) => {
                    (&a.host, &a.username, &a.port) == (&b.host, &b.username, &b.port)
                }
                (RemoteConnectionOptions::Wsl(a), RemoteConnectionOptions::Wsl(b)) => {
                    // The WSL username is not consistently populated in the workspace location, so ignore it for now.
                    a.distro_name == b.distro_name
                }
                (RemoteConnectionOptions::Docker(a), RemoteConnectionOptions::Docker(b)) => {
                    a.container_id == b.container_id
                }
                #[cfg(any(test, feature = "test-support"))]
                (RemoteConnectionOptions::Mock(a), RemoteConnectionOptions::Mock(b)) => {
                    a.id == b.id
                }
                _ => false,
            };

            shell_window.read(cx).is_ok_and(|shell| {
                if let Some(workspace) = shell.view().clone().downcast::<Workspace>().ok() {
                    match workspace.read(cx).workspace_location(cx) {
                        WorkspaceLocation::Location(location, _) => {
                            match (&location, serialized_location) {
                                (
                                    SerializedWorkspaceLocation::Local,
                                    SerializedWorkspaceLocation::Local,
                                ) => true,
                                (
                                    SerializedWorkspaceLocation::Remote(a),
                                    SerializedWorkspaceLocation::Remote(b),
                                ) => same_host(a, b),
                                _ => false,
                            }
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            })
        })
        .collect()
}

/// Helper to extract the workspace entity from a window handle.
fn workspace_from_window(
    window: WindowHandle<AppShell>,
    cx: &App,
) -> Option<Entity<Workspace>> {
    window.read(cx).ok().and_then(|shell| {
        shell.view().clone().downcast::<Workspace>().ok()
    })
}

pub async fn find_existing_workspace(
    abs_paths: &[PathBuf],
    open_options: &OpenOptions,
    location: &SerializedWorkspaceLocation,
    cx: &mut AsyncApp,
) -> (
    Option<(WindowHandle<AppShell>, Entity<Workspace>)>,
    OpenVisible,
) {
    let mut existing: Option<(WindowHandle<AppShell>, Entity<Workspace>)> = None;
    let mut open_visible = OpenVisible::All;
    let mut best_match = None;

    if open_options.open_new_workspace != Some(true) {
        cx.update(|cx| {
            for window in workspace_windows_for_location(location, cx) {
                if let Some(workspace) = workspace_from_window(window, cx) {
                    let project = workspace.read(cx).project().read(cx);
                    let m = project.visibility_for_paths(
                        abs_paths,
                        open_options.open_new_workspace == None,
                        cx,
                    );
                    if m > best_match {
                        existing = Some((window, workspace.clone()));
                        best_match = m;
                    } else if best_match.is_none()
                        && open_options.open_new_workspace == Some(false)
                    {
                        existing = Some((window, workspace.clone()))
                    }
                }
            }
        });

        let all_paths_are_files = existing
            .as_ref()
            .and_then(|(_, target_workspace)| {
                cx.update(|cx| {
                    let workspace = target_workspace.read(cx);
                    let project = workspace.project().read(cx);
                    let path_style = workspace.path_style(cx);
                    Some(!abs_paths.iter().any(|path| {
                        let path = inazuma_util::paths::SanitizedPath::new(path);
                        project.worktrees(cx).any(|worktree| {
                            let worktree = worktree.read(cx);
                            let abs_path = worktree.abs_path();
                            path_style
                                .strip_prefix(path.as_ref(), abs_path.as_ref())
                                .and_then(|rel| worktree.entry_for_path(&rel))
                                .is_some_and(|e| e.is_dir())
                        })
                    }))
                })
            })
            .unwrap_or(false);

        if open_options.open_new_workspace.is_none()
            && existing.is_some()
            && open_options.wait
            && all_paths_are_files
        {
            cx.update(|cx| {
                let windows = workspace_windows_for_location(location, cx);
                let window = cx
                    .active_window()
                    .and_then(|window| window.downcast::<AppShell>())
                    .filter(|window| windows.contains(window))
                    .or_else(|| windows.into_iter().next());
                if let Some(window) = window {
                    if let Some(workspace) = workspace_from_window(window, cx) {
                        existing = Some((window, workspace));
                        open_visible = OpenVisible::None;
                    }
                }
            });
        }
    }
    (existing, open_visible)
}

// ---------------------------------------------------------------------------
// new_local — creates a new local workspace window (was Workspace::new_local)
// ---------------------------------------------------------------------------

pub fn new_local(
    abs_paths: Vec<PathBuf>,
    app_state: Arc<AppState>,
    requesting_window: Option<WindowHandle<AppShell>>,
    env: Option<HashMap<String, String>>,
    init: Option<Box<dyn FnOnce(&mut Workspace, &mut inazuma::Window, &mut Context<Workspace>) + Send>>,
    _activate: bool,
    cx: &mut App,
) -> Task<anyhow::Result<OpenResult>> {
    let project_handle = Project::local(
        app_state.client.clone(),
        app_state.node_runtime.clone(),
        app_state.user_store.clone(),
        app_state.languages.clone(),
        app_state.fs.clone(),
        env,
        Default::default(),
        cx,
    );

    let db = WorkspaceDb::global(cx);
    let kvp = raijin_db::kvp::KeyValueStore::global(cx);
    cx.spawn(async move |cx| {
        let mut paths_to_open = Vec::with_capacity(abs_paths.len());
        for path in abs_paths.into_iter() {
            if let Some(canonical) = app_state.fs.canonicalize(&path).await.ok() {
                paths_to_open.push(canonical)
            } else {
                paths_to_open.push(path)
            }
        }

        let serialized_workspace = db.workspace_for_roots(paths_to_open.as_slice());

        if let Some(paths) = serialized_workspace.as_ref().map(|ws| &ws.paths) {
            paths_to_open = paths.ordered_paths().cloned().collect();
            if !paths.is_lexicographically_ordered() {
                project_handle.update(cx, |project, cx| {
                    project.set_worktrees_reordered(true, cx);
                });
            }
        }

        // Get project paths for all of the abs_paths
        let mut project_paths: Vec<(PathBuf, Option<ProjectPath>)> =
            Vec::with_capacity(paths_to_open.len());

        for path in paths_to_open.into_iter() {
            if let Some((_, project_entry)) = cx
                .update(|cx| {
                    Workspace::project_path_for_path(project_handle.clone(), &path, true, cx)
                })
                .await
                .log_err()
            {
                project_paths.push((path, Some(project_entry)));
            } else {
                project_paths.push((path, None));
            }
        }

        let workspace_id = if let Some(serialized_workspace) = serialized_workspace.as_ref() {
            serialized_workspace.id
        } else {
            db.next_id().await.unwrap_or_else(|_| Default::default())
        };

        let toolchains = db.toolchains(workspace_id).await?;

        for (toolchain, worktree_path, path) in toolchains {
            let toolchain_path = PathBuf::from(toolchain.path.clone().to_string());
            let Some(worktree_id) = project_handle.read_with(cx, |this, cx| {
                this.find_worktree(&worktree_path, cx)
                    .and_then(|(worktree, rel_path)| {
                        if rel_path.is_empty() {
                            Some(worktree.read(cx).id())
                        } else {
                            None
                        }
                    })
            }) else {
                // We did not find a worktree with a given path, but that's whatever.
                continue;
            };
            if !app_state.fs.is_file(toolchain_path.as_path()).await {
                continue;
            }

            project_handle
                .update(cx, |this, cx| {
                    this.activate_toolchain(ProjectPath { worktree_id, path }, toolchain, cx)
                })
                .await;
        }
        if let Some(workspace) = serialized_workspace.as_ref() {
            project_handle.update(cx, |this, cx| {
                for (scope, toolchains) in &workspace.user_toolchains {
                    for toolchain in toolchains {
                        this.add_toolchain(toolchain.clone(), scope.clone(), cx);
                    }
                }
            });
        }

        let (window, workspace): (WindowHandle<AppShell>, Entity<Workspace>) =
            if let Some(window) = requesting_window {
                let centered_layout = serialized_workspace
                    .as_ref()
                    .map(|w| w.centered_layout)
                    .unwrap_or(false);

                let workspace = window.update(cx, |_shell, window, cx| {
                    let workspace = cx.new(|cx| {
                        let mut workspace = Workspace::new(
                            Some(workspace_id),
                            project_handle.clone(),
                            app_state.clone(),
                            window,
                            cx,
                        );

                        workspace.centered_layout = centered_layout;

                        // Call init callback to add items before window renders
                        if let Some(init) = init {
                            init(&mut workspace, window, cx);
                        }

                        workspace
                    });
                    // Single workspace per window in Raijin — just return it
                    workspace
                })?;
                (window, workspace)
            } else {
                let window_bounds_override = window_bounds_env_override();

                let (window_bounds, display) = if let Some(bounds) = window_bounds_override {
                    (Some(WindowBounds::Windowed(bounds)), None)
                } else if let Some(workspace) = serialized_workspace.as_ref()
                    && let Some(display) = workspace.display
                    && let Some(bounds) = workspace.window_bounds.as_ref()
                {
                    // Reopening an existing workspace - restore its saved bounds
                    (Some(bounds.0), Some(display))
                } else if let Some((display, bounds)) =
                    read_default_window_bounds(&kvp)
                {
                    // New or empty workspace - use the last known window bounds
                    (Some(bounds), Some(display))
                } else {
                    // New window - let GPUI's default_bounds() handle cascading
                    (None, None)
                };

                // Use the serialized workspace to construct the new window
                let mut options = cx.update(|cx| (app_state.build_window_options)(display, cx));
                options.window_bounds = window_bounds;
                let centered_layout = serialized_workspace
                    .as_ref()
                    .map(|w| w.centered_layout)
                    .unwrap_or(false);
                let window = cx.open_window(options, {
                    let app_state = app_state.clone();
                    let project_handle = project_handle.clone();
                    move |window, cx| {
                        let workspace = cx.new(|cx| {
                            let mut workspace = Workspace::new(
                                Some(workspace_id),
                                project_handle,
                                app_state,
                                window,
                                cx,
                            );
                            workspace.centered_layout = centered_layout;

                            // Call init callback to add items before window renders
                            if let Some(init) = init {
                                init(&mut workspace, window, cx);
                            }

                            workspace
                        });
                        cx.new(|cx| AppShell::new(workspace, window, cx))
                    }
                })?;
                let workspace =
                    window.update(cx, |shell: &mut AppShell, _, _cx| {
                        shell.view().clone().downcast::<Workspace>().expect("AppShell view should be a Workspace")
                    })?;
                (window, workspace)
            };

        notify_if_database_failed(workspace.clone(), cx);
        // Check if this is an empty workspace (no paths to open)
        // An empty workspace is one where project_paths is empty
        let is_empty_workspace = project_paths.is_empty();
        // Check if serialized workspace has paths before it's moved
        let serialized_workspace_has_paths = serialized_workspace
            .as_ref()
            .map(|ws| !ws.paths.is_empty())
            .unwrap_or(false);

        let opened_items = window
            .update(cx, |_, window, cx| {
                workspace.update(cx, |_workspace: &mut Workspace, cx| {
                    open_items(serialized_workspace, project_paths, window, cx)
                })
            })?
            .await
            .unwrap_or_default();

        // Restore default dock state for empty workspaces
        // Only restore if:
        // 1. This is an empty workspace (no paths), AND
        // 2. The serialized workspace either doesn't exist or has no paths
        if is_empty_workspace && !serialized_workspace_has_paths {
            if let Some(default_docks) = read_default_dock_state(&kvp) {
                window
                    .update(cx, |_, window, cx| {
                        workspace.update(cx, |workspace, cx| {
                            for (dock, serialized_dock) in [
                                (workspace.right_dock().clone(), &default_docks.right),
                                (workspace.left_dock().clone(), &default_docks.left),
                                (workspace.bottom_dock().clone(), &default_docks.bottom),
                            ] {
                                dock.update(cx, |dock: &mut raijin_workspace::dock::Dock, cx| {
                                    dock.serialized_dock = Some(serialized_dock.clone());
                                    dock.restore_state(window, cx);
                                });
                            }
                            cx.notify();
                        });
                    })
                    .log_err();
            }
        }

        window
            .update(cx, |_, _window, cx| {
                workspace.update(cx, |this: &mut Workspace, cx| {
                    this.update_history(cx);
                });
            })
            .log_err();
        Ok(OpenResult {
            window,
            workspace,
            opened_items,
        })
    })
}

// ---------------------------------------------------------------------------
// open_workspace_by_id — restore an empty workspace with unsaved content
// ---------------------------------------------------------------------------

/// Opens a workspace by its database ID, used for restoring empty workspaces with unsaved content.
pub fn open_workspace_by_id(
    workspace_id: WorkspaceId,
    app_state: Arc<AppState>,
    requesting_window: Option<WindowHandle<AppShell>>,
    cx: &mut App,
) -> Task<anyhow::Result<WindowHandle<AppShell>>> {
    let project_handle = Project::local(
        app_state.client.clone(),
        app_state.node_runtime.clone(),
        app_state.user_store.clone(),
        app_state.languages.clone(),
        app_state.fs.clone(),
        None,
        raijin_project::LocalProjectFlags {
            init_worktree_trust: true,
            ..raijin_project::LocalProjectFlags::default()
        },
        cx,
    );

    let db = WorkspaceDb::global(cx);
    let kvp = raijin_db::kvp::KeyValueStore::global(cx);
    cx.spawn(async move |cx| {
        let serialized_workspace = db
            .workspace_for_id(workspace_id)
            .with_context(|| format!("Workspace {workspace_id:?} not found"))?;

        let centered_layout = serialized_workspace.centered_layout;

        let (window, workspace) = if let Some(window) = requesting_window {
            let workspace = window.update(cx, |_shell, window, cx| {
                let workspace = cx.new(|cx| {
                    let mut workspace = Workspace::new(
                        Some(workspace_id),
                        project_handle.clone(),
                        app_state.clone(),
                        window,
                        cx,
                    );
                    workspace.centered_layout = centered_layout;
                    workspace
                });
                workspace
            })?;
            (window, workspace)
        } else {
            let window_bounds_override = window_bounds_env_override();

            let (window_bounds, display) = if let Some(bounds) = window_bounds_override {
                (Some(WindowBounds::Windowed(bounds)), None)
            } else if let Some(display) = serialized_workspace.display
                && let Some(bounds) = serialized_workspace.window_bounds.as_ref()
            {
                (Some(bounds.0), Some(display))
            } else if let Some((display, bounds)) = read_default_window_bounds(&kvp) {
                (Some(bounds), Some(display))
            } else {
                (None, None)
            };

            let options = cx.update(|cx| {
                let mut options = (app_state.build_window_options)(display, cx);
                options.window_bounds = window_bounds;
                options
            });

            let window = cx.open_window(options, {
                let app_state = app_state.clone();
                let project_handle = project_handle.clone();
                move |window, cx| {
                    let workspace = cx.new(|cx| {
                        let mut workspace = Workspace::new(
                            Some(workspace_id),
                            project_handle,
                            app_state,
                            window,
                            cx,
                        );
                        workspace.centered_layout = centered_layout;
                        workspace
                    });
                    cx.new(|cx| AppShell::new(workspace, window, cx))
                }
            })?;

            let workspace = window.update(cx, |shell: &mut AppShell, _, _cx| {
                shell.view().clone().downcast::<Workspace>().expect("AppShell view should be a Workspace")
            })?;

            (window, workspace)
        };

        notify_if_database_failed(workspace.clone(), cx);

        // Restore items from the serialized workspace
        window
            .update(cx, |_, window, cx| {
                workspace.update(cx, |_workspace, cx| {
                    open_items(Some(serialized_workspace), vec![], window, cx)
                })
            })?
            .await?;

        window.update(cx, |_, window, cx| {
            workspace.update(cx, |workspace, cx| {
                workspace.serialize_workspace(window, cx);
            });
        })?;

        Ok(window)
    })
}

// ---------------------------------------------------------------------------
// open_paths — the free function that finds/creates a workspace for paths
// ---------------------------------------------------------------------------

#[allow(clippy::type_complexity)]
pub fn open_paths(
    abs_paths: &[PathBuf],
    app_state: Arc<AppState>,
    open_options: OpenOptions,
    cx: &mut App,
) -> Task<anyhow::Result<OpenResult>> {
    let abs_paths = abs_paths.to_vec();
    #[cfg(target_os = "windows")]
    let wsl_path = abs_paths
        .iter()
        .find_map(|p| inazuma_util::paths::WslPath::from_path(p));

    cx.spawn(async move |cx| {
        let (mut existing, mut open_visible) = find_existing_workspace(
            &abs_paths,
            &open_options,
            &SerializedWorkspaceLocation::Local,
            cx,
        )
        .await;

        // Fallback: if no workspace contains the paths and all paths are files,
        // prefer an existing local workspace window (active window first).
        if open_options.open_new_workspace.is_none() && existing.is_none() {
            let all_paths = abs_paths.iter().map(|path| app_state.fs.metadata(path));
            let all_metadatas = futures::future::join_all(all_paths)
                .await
                .into_iter()
                .filter_map(|result| result.ok().flatten())
                .collect::<Vec<_>>();

            if all_metadatas.iter().all(|file| !file.is_dir) {
                cx.update(|cx| {
                    let windows = workspace_windows_for_location(
                        &SerializedWorkspaceLocation::Local,
                        cx,
                    );
                    let window = cx
                        .active_window()
                        .and_then(|window| window.downcast::<AppShell>())
                        .filter(|window| windows.contains(window))
                        .or_else(|| windows.into_iter().next());
                    if let Some(window) = window {
                        if let Some(workspace) = workspace_from_window(window, cx) {
                            existing = Some((window, workspace));
                            open_visible = OpenVisible::None;
                        }
                    }
                });
            }
        }

        let result = if let Some((existing, target_workspace)) = existing {
            let open_task = existing
                .update(cx, |_shell, window, cx| {
                    window.activate_window();
                    target_workspace.update(cx, |workspace, cx| {
                        workspace.open_paths(
                            abs_paths,
                            raijin_workspace::OpenOptions {
                                visible: Some(open_visible),
                                ..Default::default()
                            },
                            None,
                            window,
                            cx,
                        )
                    })
                })?
                .await;

            _ = existing.update(cx, |_shell, _, cx| {
                target_workspace.update(cx, |workspace, cx| {
                    for item in open_task.iter().flatten() {
                        if let Err(e) = item {
                            workspace.show_error(&e, cx);
                        }
                    }
                });
            });

            Ok(OpenResult { window: existing, workspace: target_workspace, opened_items: open_task })
        } else {
            let result = cx
                .update(move |cx| {
                    new_local(
                        abs_paths,
                        app_state.clone(),
                        open_options.replace_window,
                        open_options.env,
                        None,
                        true,
                        cx,
                    )
                })
                .await;

            if let Ok(ref result) = result {
                result.window
                    .update(cx, |_, window, _cx| {
                        window.activate_window();
                    })
                    .log_err();
            }

            result
        };

        #[cfg(target_os = "windows")]
        if let Some(inazuma_util::paths::WslPath{distro, path}) = wsl_path
            && let Ok(ref result) = result
        {
            result.window
                .update(cx, move |_shell, _window, cx| {
                    if let Some(workspace) = AppShell::workspace(_window, cx) {
                        workspace.update(cx, |workspace, cx| {
                            workspace.show_notification(raijin_workspace::notifications::NotificationId::unique::<OpenInWsl>(), cx, move |cx| {
                                let display_path = inazuma_util::markdown::MarkdownInlineCode(&path.to_string_lossy());
                                let msg = format!("{display_path} is inside a WSL filesystem, some features may not work unless you open it with WSL remote");
                                cx.new(move |cx| {
                                    raijin_workspace::notifications::simple_message_notification::MessageNotification::new(msg, cx)
                                        .primary_message("Open in WSL")
                                        .primary_icon(raijin_ui::IconName::FolderOpen)
                                        .primary_on_click(move |window, cx| {
                                            window.dispatch_action(Box::new(raijin_remote::OpenWslPath {
                                                    distro: raijin_remote::WslConnectionOptions {
                                                            distro_name: distro.clone(),
                                                        user: None,
                                                    },
                                                    paths: vec![path.clone().into()],
                                                }), cx)
                                        })
                                })
                            });
                        });
                    }
                })
                .unwrap();
        };
        result
    })
}

#[cfg(target_os = "windows")]
struct OpenInWsl;

// ---------------------------------------------------------------------------
// open_new — open a fresh workspace window
// ---------------------------------------------------------------------------

pub fn open_new(
    open_options: OpenOptions,
    app_state: Arc<AppState>,
    cx: &mut App,
    init: impl FnOnce(&mut Workspace, &mut inazuma::Window, &mut Context<Workspace>) + 'static + Send,
) -> Task<anyhow::Result<()>> {
    let task = new_local(
        Vec::new(),
        app_state,
        open_options.replace_window,
        open_options.env,
        Some(Box::new(init)),
        true,
        cx,
    );
    cx.spawn(async move |cx| {
        let OpenResult { window, .. } = task.await?;
        window
            .update(cx, |_, window, _cx| {
                window.activate_window();
            })
            .ok();
        Ok(())
    })
}

// ---------------------------------------------------------------------------
// join_in_room_project — join a shared project in a call
// ---------------------------------------------------------------------------

pub fn join_in_room_project(
    project_id: u64,
    follow_user_id: u64,
    app_state: Arc<AppState>,
    cx: &mut App,
) -> Task<Result<()>> {
    let windows = cx.windows();
    cx.spawn(async move |cx| {
        let existing_window_and_workspace: Option<(
            WindowHandle<AppShell>,
            Entity<Workspace>,
        )> = windows.into_iter().find_map(|window_handle| {
            window_handle
                .downcast::<AppShell>()
                .and_then(|window_handle| {
                    window_handle
                        .update(cx, |_shell, _window, cx| {
                            if let Some(workspace) = AppShell::workspace(_window, cx) {
                                if workspace.read(cx).project().read(cx).remote_id()
                                    == Some(project_id)
                                {
                                    return Some((window_handle, workspace));
                                }
                            }
                            None
                        })
                        .unwrap_or(None)
                })
        });

        let shell_window = if let Some((existing_window, _target_workspace)) =
            existing_window_and_workspace
        {
            existing_window
        } else {
            let active_call = cx.update(|cx| raijin_workspace::GlobalAnyActiveCall::global(cx).clone());
            let project = cx
                .update(|cx| {
                    active_call.0.join_project(
                        project_id,
                        app_state.languages.clone(),
                        app_state.fs.clone(),
                        cx,
                    )
                })
                .await?;

            let window_bounds_override = window_bounds_env_override();
            cx.update(|cx| {
                let mut options = (app_state.build_window_options)(None, cx);
                options.window_bounds = window_bounds_override.map(WindowBounds::Windowed);
                cx.open_window(options, |window, cx| {
                    let workspace = cx.new(|cx| {
                        Workspace::new(Default::default(), project, app_state.clone(), window, cx)
                    });
                    cx.new(|cx| AppShell::new(workspace, window, cx))
                })
            })?
        };

        shell_window.update(cx, |_shell, window, cx| {
            cx.activate(true);
            window.activate_window();

            if let Some(workspace) = AppShell::workspace(window, cx) {
                workspace.update(cx, |workspace, cx| {
                    let follow_peer_id = raijin_workspace::GlobalAnyActiveCall::try_global(cx)
                        .and_then(|call| call.0.peer_id_for_user_in_room(follow_user_id, cx))
                        .or_else(|| {
                            // If we couldn't follow the given user, follow the host instead.
                            let collaborator = workspace
                                .project()
                                .read(cx)
                                .collaborators()
                                .values()
                                .find(|collaborator| collaborator.is_host)?;
                            Some(collaborator.peer_id)
                        });

                    if let Some(follow_peer_id) = follow_peer_id {
                        workspace.follow(follow_peer_id, window, cx);
                    }
                });
            }
        })?;

        anyhow::Ok(())
    })
}

// ---------------------------------------------------------------------------
// reload — restart the application
// ---------------------------------------------------------------------------

pub fn reload(cx: &mut App) {
    let should_confirm = WorkspaceSettings::get_global(cx).confirm_quit;
    let mut workspace_windows = cx
        .windows()
        .into_iter()
        .filter_map(|window| window.downcast::<AppShell>())
        .collect::<Vec<_>>();

    // If multiple windows have unsaved changes, and need a save prompt,
    // prompt in the active window before switching to a different window.
    workspace_windows.sort_by_key(|window| window.is_active(cx) == Some(false));

    let mut prompt = None;
    if let (true, Some(window)) = (should_confirm, workspace_windows.first()) {
        prompt = window
            .update(cx, |_, window, cx| {
                window.prompt(
                    PromptLevel::Info,
                    "Are you sure you want to restart?",
                    None,
                    &["Restart", "Cancel"],
                    cx,
                )
            })
            .ok();
    }

    cx.spawn(async move |cx| {
        if let Some(prompt) = prompt {
            let answer = prompt.await?;
            if answer != 0 {
                return anyhow::Ok(());
            }
        }

        // If the user cancels any save prompt, then keep the app open.
        for window in workspace_windows {
            if let Ok(should_close) = window.update(cx, |_shell, window, cx| {
                if let Some(workspace) = AppShell::workspace(window, cx) {
                    workspace.update(cx, |workspace, cx| {
                        workspace.prepare_to_close(CloseIntent::Quit, window, cx)
                    })
                } else {
                    Task::ready(Ok(true))
                }
            }) && !should_close.await?
            {
                return anyhow::Ok(());
            }
        }
        cx.update(|cx| cx.restart());
        anyhow::Ok(())
    })
    .detach_and_log_err(cx);
}
