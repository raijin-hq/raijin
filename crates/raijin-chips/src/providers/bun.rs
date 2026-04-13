use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

pub struct BunProvider;

impl ChipProvider for BunProvider {
    fn id(&self) -> ChipId { "bun" }
    fn display_name(&self) -> &str { "Bun" }

    fn detect_files(&self) -> &[&str] {
        &["bun.lockb", "bun.lock", "bunfig.toml"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx.exec_cmd("bun", &["--version"])
            .map(|o| o.stdout.trim().to_string())
            .unwrap_or_default();
        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Bun"),
            tooltip: Some("Bun version".into()),
            ..ChipOutput::default()
        }
    }
}
