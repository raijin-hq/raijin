use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

pub struct GleamProvider;

impl ChipProvider for GleamProvider {
    fn id(&self) -> ChipId {
        "gleam"
    }

    fn display_name(&self) -> &str {
        "Gleam"
    }

    fn detect_files(&self) -> &[&str] {
        &["gleam.toml"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["gleam"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("gleam", &["--version"])
            .and_then(|o| parse_gleam_version(&o.stdout))
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Gleam"),
            tooltip: Some("Gleam version".into()),
            ..ChipOutput::default()
        }
    }
}

/// Parse gleam version from output like "gleam 1.0.0"
fn parse_gleam_version(version: &str) -> Option<String> {
    let version = version.split_whitespace().last()?;
    Some(version.to_string())
}
