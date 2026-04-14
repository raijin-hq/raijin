use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Zig language version.
///
/// Detection: `build.zig`, `build.zig.zon`, `.zig` files.
/// Version: `zig version` outputs a clean version string like `0.13.0` or `0.14.0-dev.1234+abcdef`.
/// Also reads `build.zig.zon` for minimum Zig version if present.
pub struct ZigProvider;

impl ChipProvider for ZigProvider {
    fn id(&self) -> ChipId {
        "zig"
    }

    fn display_name(&self) -> &str {
        "Zig"
    }

    fn detect_files(&self) -> &[&str] {
        &["build.zig", "build.zig.zon"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["zig"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        // `zig version` outputs a clean version string directly
        let version = ctx
            .exec_cmd("zig", &["version"])
            .map(|o| o.stdout.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_default();

        let min_version = read_zig_zon_min_version(&ctx.cwd);
        let tooltip = match (&version, &min_version) {
            (v, Some(min)) if !v.is_empty() => Some(format!("Zig {v} (requires >= {min})")),
            (v, None) if !v.is_empty() => Some(format!("Zig {v}")),
            _ => None,
        };

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Zap"),
            tooltip,
            ..ChipOutput::default()
        }
    }
}

/// Read the minimum Zig version from `build.zig.zon`.
///
/// Looks for `.minimum_zig_version = "0.13.0"` in the ZON file.
fn read_zig_zon_min_version(cwd: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(cwd.join("build.zig.zon")).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(".minimum_zig_version") {
            // Extract value between quotes: .minimum_zig_version = "0.13.0",
            let start = trimmed.find('"')? + 1;
            let end = trimmed[start..].find('"')? + start;
            let version = &trimmed[start..end];
            if !version.is_empty() {
                return Some(version.to_string());
            }
        }
    }
    None
}
