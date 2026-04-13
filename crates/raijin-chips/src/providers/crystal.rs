use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

pub struct CrystalProvider;

impl ChipProvider for CrystalProvider {
    fn id(&self) -> ChipId { "crystal" }
    fn display_name(&self) -> &str { "Crystal" }

    fn detect_extensions(&self) -> &[&str] { &["cr"] }
    fn detect_files(&self) -> &[&str] { &["shard.yml"] }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx.exec_cmd("crystal", &["--version"])
            .and_then(|o| parse_crystal_version(&o.stdout))
            .unwrap_or_default();
        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Crystal"),
            tooltip: Some("Crystal version".into()),
            ..ChipOutput::default()
        }
    }
}

/// Parse "Crystal 1.11.0 [abc123]" → "1.11.0"
fn parse_crystal_version(crystal_version: &str) -> Option<String> {
    Some(crystal_version.split_whitespace().nth(1)?.to_string())
}
