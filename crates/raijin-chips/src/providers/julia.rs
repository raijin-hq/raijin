use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Julia language version.
///
/// Detection: `Project.toml`, `Manifest.toml`, `JuliaProject.toml`, `.jl` files.
/// Version: `julia --version` -> `julia version 1.10.0` -> `1.10.0`.
/// Also reads `Project.toml` `[compat]` section for the required Julia version range.
pub struct JuliaProvider;

impl ChipProvider for JuliaProvider {
    fn id(&self) -> ChipId {
        "julia"
    }

    fn display_name(&self) -> &str {
        "Julia"
    }

    fn detect_files(&self) -> &[&str] {
        &["Project.toml", "Manifest.toml", "JuliaProject.toml"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["jl"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("julia", &["--version"])
            .and_then(|o| parse_julia_version(&o.stdout))
            .unwrap_or_default();

        let compat_version = read_project_toml_compat(&ctx.cwd);
        let tooltip = match (&version, &compat_version) {
            (v, Some(compat)) if !v.is_empty() => {
                Some(format!("Julia {v} (compat: {compat})"))
            }
            (v, None) if !v.is_empty() => Some(format!("Julia {v}")),
            _ => None,
        };

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Julia"),
            tooltip,
            ..ChipOutput::default()
        }
    }
}

/// Parse Julia version from `julia --version` output.
///
/// Input: `julia version 1.10.0`
/// Output: `Some("1.10.0")`
fn parse_julia_version(stdout: &str) -> Option<String> {
    let version = stdout.split_once("julia version")?.1.trim();
    let version = version.split_whitespace().next()?;
    Some(version.to_string())
}

/// Read the Julia compat version from `Project.toml`.
///
/// Looks for `julia = "1.6"` or `julia = ">= 1.9"` in the `[compat]` section.
fn read_project_toml_compat(cwd: &std::path::Path) -> Option<String> {
    // Try Project.toml first, then JuliaProject.toml
    let content = std::fs::read_to_string(cwd.join("Project.toml"))
        .or_else(|_| std::fs::read_to_string(cwd.join("JuliaProject.toml")))
        .ok()?;

    let mut in_compat = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[compat]" {
            in_compat = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_compat = false;
            continue;
        }
        if in_compat
            && let Some(rest) = trimmed.strip_prefix("julia")
        {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix('=') {
                let value = rest.trim().trim_matches('"').trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}
