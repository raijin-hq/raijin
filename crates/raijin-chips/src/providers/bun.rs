use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Bun runtime version.
///
/// Detection: `bun.lockb`, `bun.lock`, `bunfig.toml`
/// Version:   `bun --version` → "0.1.4\n" → "0.1.4"
pub struct BunProvider;

impl ChipProvider for BunProvider {
    fn id(&self) -> ChipId {
        "bun"
    }

    fn display_name(&self) -> &str {
        "Bun"
    }

    fn detect_files(&self) -> &[&str] {
        &["bun.lockb", "bun.lock", "bunfig.toml"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("bun", &["--version"])
            .and_then(|o| parse_bun_version(&o.stdout))
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Zap"),
            tooltip: Some("Bun version".into()),
            ..ChipOutput::default()
        }
    }
}

/// Parse `bun --version` output: "0.1.4\n" → "0.1.4"
///
/// Bun outputs a bare version string (no prefix). standard just trims it.
fn parse_bun_version(output: &str) -> Option<String> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version() {
        assert_eq!(
            parse_bun_version("0.1.4\n"),
            Some("0.1.4".to_string()),
        );
    }

    #[test]
    fn parse_version_1x() {
        assert_eq!(
            parse_bun_version("1.0.0\n"),
            Some("1.0.0".to_string()),
        );
    }

    #[test]
    fn parse_empty() {
        assert_eq!(parse_bun_version(""), None);
    }

    #[test]
    fn parse_whitespace_only() {
        assert_eq!(parse_bun_version("  \n"), None);
    }
}
