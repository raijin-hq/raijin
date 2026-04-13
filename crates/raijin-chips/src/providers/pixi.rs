use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Pixi package manager version and environment.
///
/// Detection: `pixi.toml`, `pixi.lock`, or `PIXI_ENVIRONMENT_NAME` env var
/// Version:   `pixi --version` → "pixi 0.33.0" → "0.33.0"
/// Environment: `$PIXI_ENVIRONMENT_NAME` (e.g., "py312")
pub struct PixiProvider;

impl ChipProvider for PixiProvider {
    fn id(&self) -> ChipId {
        "pixi"
    }

    fn display_name(&self) -> &str {
        "Pixi"
    }

    fn detect_files(&self) -> &[&str] {
        &["pixi.toml", "pixi.lock"]
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.has_env("PIXI_ENVIRONMENT_NAME")
            || ctx.dir_contents.matches(self.detect_files(), &[], &[])
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("pixi", &["--version"])
            .and_then(|o| parse_pixi_version(&o.stdout))
            .unwrap_or_default();

        let env_name = ctx.get_env("PIXI_ENVIRONMENT_NAME")
            .filter(|name| name != "default");

        let label = match env_name {
            Some(env) => format!("{version} ({env})"),
            None => version,
        };

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Package"),
            tooltip: Some("Pixi version".into()),
            ..ChipOutput::default()
        }
    }
}

/// Parse `pixi --version` output: "pixi 0.33.0\n" → "0.33.0"
fn parse_pixi_version(output: &str) -> Option<String> {
    Some(output.split_once(' ')?.1.trim().to_string())
}
