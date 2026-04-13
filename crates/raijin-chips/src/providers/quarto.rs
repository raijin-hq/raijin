use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider, parse_version_number};

/// Chip provider for Quarto CLI version.
///
/// Detection: `_quarto.yml` file; `qmd` extension.
/// Version:   `quarto --version` → "1.4.549" → "1.4.549"
pub struct QuartoProvider;

impl ChipProvider for QuartoProvider {
    fn id(&self) -> ChipId {
        "quarto"
    }

    fn display_name(&self) -> &str {
        "Quarto"
    }

    fn detect_files(&self) -> &[&str] {
        &["_quarto.yml"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["qmd"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("quarto", &["--version"])
            .map(|o| parse_version_number(o.stdout.trim()))
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("Quarto"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("Quarto {version}"))
            },
            ..ChipOutput::default()
        }
    }
}
