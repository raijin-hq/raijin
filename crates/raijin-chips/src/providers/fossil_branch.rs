use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the current Fossil branch.
pub struct FossilBranchProvider;

impl ChipProvider for FossilBranchProvider {
    fn id(&self) -> ChipId {
        "fossil"
    }

    fn display_name(&self) -> &str {
        "Fossil"
    }

    fn detect_files(&self) -> &[&str] {
        &[".fslckout", "_FOSSIL_"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let branch = ctx
            .exec_cmd("fossil", &["branch", "current"])
            .map(|o| o.stdout.trim().to_string())
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: branch,
            icon: Some("GitBranch"),
            tooltip: Some("Fossil branch".into()),
            ..ChipOutput::default()
        }
    }
}
