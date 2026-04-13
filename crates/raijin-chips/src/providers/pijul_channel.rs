use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the current Pijul channel.
pub struct PijulChannelProvider;

impl ChipProvider for PijulChannelProvider {
    fn id(&self) -> ChipId {
        "pijul"
    }

    fn display_name(&self) -> &str {
        "Pijul"
    }

    fn detect_folders(&self) -> &[&str] {
        &[".pijul"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let channel = ctx
            .exec_cmd("pijul", &["channel"])
            .map(|o| {
                // pijul channel output marks the current channel with "*"
                o.stdout
                    .lines()
                    .find(|l| l.starts_with('*'))
                    .map(|l| l.trim_start_matches('*').trim().to_string())
                    .unwrap_or_else(|| o.stdout.lines().next().unwrap_or_default().trim().to_string())
            })
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: channel,
            icon: Some("GitBranch"),
            tooltip: Some("Pijul channel".into()),
            ..ChipOutput::default()
        }
    }
}
