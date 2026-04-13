use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the current git commit hash on detached HEAD.
///
/// Only activates when HEAD is detached (not on a named branch). This is useful
/// when checking out tags, specific commits, or during rebase operations.
///
/// Uses `git rev-parse --short HEAD` for the abbreviated hash, and also
/// checks if HEAD points to a tag via `git describe --tags --exact-match`.
///
/// Display examples:
/// - Detached at commit: `"a1b2c3d"`
/// - Detached at tag: `"v1.2.3 (a1b2c3d)"`
pub struct GitCommitProvider;

impl ChipProvider for GitCommitProvider {
    fn id(&self) -> ChipId {
        "git_commit"
    }

    fn display_name(&self) -> &str {
        "Git Commit"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        is_detached_head(ctx)
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let short_hash = ctx
            .exec_cmd("git", &["rev-parse", "--short", "HEAD"])
            .map(|o| o.stdout.trim().to_string())
            .unwrap_or_default();

        // Check if HEAD points to a tag
        let tag = ctx
            .exec_cmd("git", &["describe", "--tags", "--exact-match", "HEAD"])
            .map(|o| o.stdout.trim().to_string())
            .filter(|t| !t.is_empty());

        let label = match &tag {
            Some(tag_name) => format!("{tag_name} ({short_hash})"),
            None => short_hash.clone(),
        };

        let tooltip = match &tag {
            Some(tag_name) => format!("Detached HEAD at tag {tag_name} ({short_hash})"),
            None => format!("Detached HEAD at {short_hash}"),
        };

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("GitCommit"),
            tooltip: if short_hash.is_empty() {
                None
            } else {
                Some(tooltip)
            },
            ..ChipOutput::default()
        }
    }
}

/// Check if git HEAD is detached.
///
/// `git symbolic-ref HEAD` fails (non-zero exit) when HEAD is detached.
/// We also check the shell context's git_branch as a fast path.
fn is_detached_head(ctx: &ChipContext) -> bool {
    // Fast path: shell hook already detected "HEAD" as branch name
    if let Some(branch) = ctx.shell_context.git_branch.as_deref() {
        if branch == "HEAD" || branch.is_empty() {
            return true;
        }
        // If we have a concrete branch name, HEAD is not detached
        return false;
    }

    // Slow path: check git directly
    // git symbolic-ref HEAD returns non-zero when detached
    ctx.exec_cmd("git", &["symbolic-ref", "HEAD"]).is_none()
        && ctx.cwd.join(".git").exists()
}
