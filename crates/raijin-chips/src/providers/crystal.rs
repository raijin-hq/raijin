use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Crystal language version.
///
/// Detection: `shard.yml`, `.cr` files.
/// Version: `crystal --version` -> `Crystal 1.11.0 (2024-01-08)` -> `1.11.0`.
/// Also reads `shard.yml` for the `crystal:` version constraint.
///

pub struct CrystalProvider;

impl ChipProvider for CrystalProvider {
    fn id(&self) -> ChipId {
        "crystal"
    }

    fn display_name(&self) -> &str {
        "Crystal"
    }

    fn detect_files(&self) -> &[&str] {
        &["shard.yml"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["cr"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("crystal", &["--version"])
            .and_then(|o| parse_crystal_version(&o.stdout))
            .unwrap_or_default();

        let shard_version = read_shard_crystal_version(&ctx.cwd);
        let tooltip = match (&version, &shard_version) {
            (v, Some(sv)) if !v.is_empty() => {
                Some(format!("Crystal {v} (requires {sv})"))
            }
            (v, None) if !v.is_empty() => Some(format!("Crystal {v}")),
            _ => None,
        };

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Crystal"),
            tooltip,
            ..ChipOutput::default()
        }
    }
}

/// Parse Crystal version from `crystal --version` output.
///
/// Input: `Crystal 1.11.0 (2024-01-08)\n\nLLVM: 17.0.6\nDefault target: ...`
/// Output: `Some("1.11.0")`
fn parse_crystal_version(stdout: &str) -> Option<String> {
    let first_line = stdout.lines().next()?;
    let version = first_line.split_whitespace().nth(1)?;
    if version.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        Some(version.to_string())
    } else {
        None
    }
}

/// Read the Crystal version constraint from `shard.yml`.
///
/// Looks for `crystal: ">= 1.0.0, < 2.0"` or `crystal: 1.10.0`.
fn read_shard_crystal_version(cwd: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(cwd.join("shard.yml")).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("crystal:") {
            let value = rest.trim().trim_matches('"').trim_matches('\'').trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}
