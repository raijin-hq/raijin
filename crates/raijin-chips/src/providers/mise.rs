use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the mise (dev tool manager) configuration.
///
/// Detection: `.mise.toml`, `mise.toml`, `.mise.local.toml`
/// Health:    `mise doctor` success → "healthy", failure → "unhealthy"
pub struct MiseProvider;

impl ChipProvider for MiseProvider {
    fn id(&self) -> ChipId {
        "mise"
    }

    fn display_name(&self) -> &str {
        "Mise"
    }

    fn detect_files(&self) -> &[&str] {
        &[".mise.toml", "mise.toml", ".mise.local.toml"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let health = if ctx.exec_cmd("mise", &["doctor"]).is_some() {
            "healthy"
        } else {
            "unhealthy"
        };

        ChipOutput {
            id: self.id(),
            label: format!("mise {health}"),
            icon: Some("Settings"),
            tooltip: Some(format!("Mise: {health}")),
            ..ChipOutput::default()
        }
    }
}
