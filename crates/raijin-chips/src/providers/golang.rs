use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Go version.
///
/// Detection: `go.mod`, `go.sum`, `.go` files.
/// Version: Parsed from `go version` output (`go version go1.22.0 darwin/arm64` → `1.22.0`).
/// Reads `go.mod` for the required Go version (shown in tooltip).
pub struct GolangProvider;

impl ChipProvider for GolangProvider {
    fn id(&self) -> ChipId {
        "golang"
    }

    fn display_name(&self) -> &str {
        "Go"
    }

    fn detect_files(&self) -> &[&str] {
        &["go.mod", "go.sum"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["go"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("go", &["version"])
            .and_then(|o| parse_go_version(&o.stdout))
            .unwrap_or_default();

        let mod_version = read_go_mod_version(&ctx.cwd);
        let tooltip = match &mod_version {
            Some(mod_ver) => format!("Go {} (requires >= {})", version, mod_ver),
            None => format!("Go {}", version),
        };

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Go"),
            tooltip: Some(tooltip),
            ..ChipOutput::default()
        }
    }
}

/// Parse Go version from `go version` output.
///
/// Input: `go version go1.22.0 darwin/arm64`
/// Output: Some(`1.22.0`)
fn parse_go_version(stdout: &str) -> Option<String> {
    let version = stdout
        .split_once("go version go")?
        .1
        .split_whitespace()
        .next()?;
    Some(version.to_string())
}

/// Read the `go` directive version from `go.mod`.
///
/// Looks for the line `go 1.22` or `go 1.22.0`.
fn read_go_mod_version(cwd: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(cwd.join("go.mod")).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("go ") {
            let version = rest.trim();
            if !version.is_empty() && version.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                return Some(version.to_string());
            }
        }
    }
    None
}
