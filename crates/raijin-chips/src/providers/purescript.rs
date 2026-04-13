use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for PureScript compiler version.
///
/// Detection: `spago.dhall`, `spago.yaml`, `spago.lock`, extension `purs`
/// Version:   `purs --version` → "0.13.5" → "0.13.5"
pub struct PurescriptProvider;

impl ChipProvider for PurescriptProvider {
    fn id(&self) -> ChipId {
        "purescript"
    }

    fn display_name(&self) -> &str {
        "PureScript"
    }

    fn detect_files(&self) -> &[&str] {
        &["spago.dhall", "spago.yaml", "spago.lock"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["purs"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("purs", &["--version"])
            .map(|o| o.stdout.trim().to_string())
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Purescript"),
            tooltip: Some("PureScript version".into()),
            ..ChipOutput::default()
        }
    }
}
