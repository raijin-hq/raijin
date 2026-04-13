use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider, parse_version_number};

/// Chip provider for Red language version.
///
/// Detection: `red`, `reds` extensions.
/// Version:   `red --version` → parses version number.
pub struct RedProvider;

impl ChipProvider for RedProvider {
    fn id(&self) -> ChipId {
        "red"
    }

    fn display_name(&self) -> &str {
        "Red"
    }

    fn detect_extensions(&self) -> &[&str] {
        &["red", "reds"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("red", &["--version"])
            .map(|o| parse_version_number(o.stdout.trim()))
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("Red"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("Red {version}"))
            },
            ..ChipOutput::default()
        }
    }
}
