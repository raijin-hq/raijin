use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the current Mercurial branch.
pub struct HgBranchProvider;

impl ChipProvider for HgBranchProvider {
    fn id(&self) -> ChipId {
        "hg"
    }

    fn display_name(&self) -> &str {
        "Mercurial"
    }

    fn detect_folders(&self) -> &[&str] {
        &[".hg"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let branch = ctx
            .exec_cmd("hg", &["branch"])
            .map(|o| o.stdout.trim().to_string())
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: branch,
            icon: Some("GitBranch"),
            tooltip: Some("Mercurial branch".into()),
            ..ChipOutput::default()
        }
    }
}
