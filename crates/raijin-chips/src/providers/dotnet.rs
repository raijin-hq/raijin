use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for .NET SDK version.
///
/// Detection: `global.json`, `Directory.Build.props`, `Directory.Build.targets`,
///            `.csproj`, `.fsproj`, `.sln` files; `cs`, `fs`, `vb` extensions
///
/// Version resolution (standard priority):
/// 1. If `global.json` exists in cwd, read pinned SDK version from it
/// 2. Fall back to `dotnet --version` CLI output
pub struct DotnetProvider;

impl ChipProvider for DotnetProvider {
    fn id(&self) -> ChipId {
        "dotnet"
    }

    fn display_name(&self) -> &str {
        ".NET"
    }

    fn detect_files(&self) -> &[&str] {
        &[
            "global.json",
            "Directory.Build.props",
            "Directory.Build.targets",
            ".csproj",
            ".fsproj",
            ".sln",
        ]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["cs", "fs", "vb"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = get_dotnet_version(ctx).unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Dotnet"),
            tooltip: Some(".NET SDK version".into()),
            ..ChipOutput::default()
        }
    }
}

/// Resolve .NET version: check `global.json` for pinned SDK, fall back to CLI.
fn get_dotnet_version(ctx: &ChipContext) -> Option<String> {
    // Try pinned version from global.json first (standard heuristic)
    if let Some(pinned) = get_pinned_sdk_version(ctx) {
        return Some(pinned);
    }

    // Fall back to `dotnet --version`
    ctx.exec_cmd("dotnet", &["--version"])
        .and_then(|o| parse_dotnet_cli_version(&o.stdout))
}

/// Read `global.json` from cwd and extract `sdk.version`.
fn get_pinned_sdk_version(ctx: &ChipContext) -> Option<String> {
    let global_json_path = ctx.cwd.join("global.json");
    let contents = std::fs::read_to_string(global_json_path).ok()?;
    parse_global_json_version(&contents)
}

/// Parse `{"sdk": {"version": "8.0.301"}}` → "8.0.301"
fn parse_global_json_version(json: &str) -> Option<String> {
    // Minimal JSON parsing without pulling in serde_json — scan for "version": "..."
    // within a "sdk" block. This handles the standard global.json format.
    let sdk_idx = json.find("\"sdk\"")?;
    let rest = &json[sdk_idx..];
    let version_idx = rest.find("\"version\"")?;
    let after_key = &rest[version_idx + "\"version\"".len()..];
    // Skip whitespace and colon
    let after_colon = after_key.find(':').map(|i| &after_key[i + 1..])?;
    let trimmed = after_colon.trim_start();
    // Extract quoted string value
    if !trimmed.starts_with('"') {
        return None;
    }
    let value_start = &trimmed[1..];
    let end = value_start.find('"')?;
    let version = &value_start[..end];
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

/// Parse `dotnet --version` output: "8.0.301\n" → "8.0.301"
fn parse_dotnet_cli_version(output: &str) -> Option<String> {
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
    fn parse_cli_version() {
        assert_eq!(
            parse_dotnet_cli_version("8.0.301\n"),
            Some("8.0.301".to_string()),
        );
    }

    #[test]
    fn parse_cli_empty() {
        assert_eq!(parse_dotnet_cli_version(""), None);
    }

    #[test]
    fn parse_global_json_pinned() {
        let json = r#"
        {
            "sdk": {
                "version": "6.0.400"
            }
        }
        "#;
        assert_eq!(
            parse_global_json_version(json),
            Some("6.0.400".to_string()),
        );
    }

    #[test]
    fn parse_global_json_with_roll_forward() {
        let json = r#"
        {
            "sdk": {
                "version": "8.0.100",
                "rollForward": "latestMajor"
            }
        }
        "#;
        assert_eq!(
            parse_global_json_version(json),
            Some("8.0.100".to_string()),
        );
    }

    #[test]
    fn parse_global_json_empty() {
        assert_eq!(parse_global_json_version("{}"), None);
    }

    #[test]
    fn parse_global_json_no_version() {
        let json = r#"{"sdk": {}}"#;
        assert_eq!(parse_global_json_version(json), None);
    }
}
