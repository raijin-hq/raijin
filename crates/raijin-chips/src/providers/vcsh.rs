use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the active VCSH repository.
///
/// Availability: `$VCSH_REPO_NAME` environment variable is set.
/// Label: repository name from the environment variable.
pub struct VcshProvider;

impl ChipProvider for VcshProvider {
    fn id(&self) -> ChipId {
        "vcsh"
    }

    fn display_name(&self) -> &str {
        "VCSH"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.has_env("VCSH_REPO_NAME")
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let repo = ctx.get_env("VCSH_REPO_NAME").unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: repo.clone(),
            icon: Some("GitBranch"),
            tooltip: if repo.is_empty() {
                None
            } else {
                Some(format!("VCSH repository: {repo}"))
            },
            ..ChipOutput::default()
        }
    }
}
