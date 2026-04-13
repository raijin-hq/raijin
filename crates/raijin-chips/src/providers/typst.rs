use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Typst version.
///
/// Detection: `typ` extension.
/// Version:   `typst --version` → "typst 0.10.0 (..." → "0.10.0"
pub struct TypstProvider;

impl ChipProvider for TypstProvider {
    fn id(&self) -> ChipId {
        "typst"
    }

    fn display_name(&self) -> &str {
        "Typst"
    }

    fn detect_extensions(&self) -> &[&str] {
        &["typ"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("typst", &["--version"])
            .and_then(|o| parse_typst_version(&o.stdout))
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("Typst"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("Typst {version}"))
            },
            ..ChipOutput::default()
        }
    }
}

/// Parse `typst --version` output.
///
/// Format: "typst 0.10.0 (abcdef12)" → "0.10.0"
/// Strips the "typst " prefix, then takes the first whitespace-delimited token.
fn parse_typst_version(output: &str) -> Option<String> {
    output
        .trim()
        .strip_prefix("typst ")
        .and_then(|version| version.split_whitespace().next().map(ToOwned::to_owned))
}
