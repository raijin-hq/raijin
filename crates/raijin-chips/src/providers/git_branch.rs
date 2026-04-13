use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the current git branch.
///
/// Visible when inside a git repository (git_branch is Some).
pub struct GitBranchProvider;

impl ChipProvider for GitBranchProvider {
    fn id(&self) -> ChipId {
        "git_branch"
    }

    fn display_name(&self) -> &str {
        "Git Branch"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.shell_context.git_branch.is_some()
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let branch = ctx
            .shell_context
            .git_branch
            .as_deref()
            .unwrap_or("HEAD");

        ChipOutput {
            id: self.id(),
            label: branch.to_string(),
            icon: Some("GitBranch"),
            tooltip: Some("Switch branch".to_string()),
            interactive: true,
            ..ChipOutput::default()
        }
    }
}
