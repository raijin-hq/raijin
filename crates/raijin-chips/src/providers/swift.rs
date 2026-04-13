use std::path::Path;

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Swift language version.
///
/// Detection (mirrors standard Swift module):
/// - Files: `Package.swift`, `.swift-version`
/// - Extensions: `.swift`
///
/// Parses version from `swift --version` output which varies between
/// Apple Swift and open-source Swift toolchains.
pub struct SwiftProvider;

impl ChipProvider for SwiftProvider {
    fn id(&self) -> ChipId {
        "swift"
    }

    fn display_name(&self) -> &str {
        "Swift"
    }

    fn detect_files(&self) -> &[&str] {
        &["Package.swift", ".swift-version"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["swift"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = get_swift_version(ctx);

        let label = version.clone().unwrap_or_default();

        let tooltip = match &version {
            Some(ver) => format!("Swift {ver}"),
            None => "Swift".into(),
        };

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Bird"),
            tooltip: Some(tooltip),
            ..ChipOutput::default()
        }
    }
}

/// Resolve the Swift version string.
///
/// Priority order:
/// 1. `.swift-version` file (swiftenv local config)
/// 2. `swift --version` command output
fn get_swift_version(ctx: &ChipContext) -> Option<String> {
    // 1. .swift-version file (swiftenv local config)
    if let Some(ver) = read_swift_version_file(&ctx.cwd) {
        return Some(ver);
    }

    // 2. swift --version
    let output = ctx.exec_cmd("swift", &["--version"])?;
    // swift --version may output to stdout or stderr depending on toolchain
    let text = if output.stdout.is_empty() {
        &output.stderr
    } else {
        &output.stdout
    };
    parse_swift_version_output(text)
}

/// Parse version from `swift --version` output.
///
/// Handles multiple output formats:
/// - Apple: "Apple Swift version 5.9.2 (swiftlang-5.9.2.2.56 clang-1500.1.0.2.5)"
/// - Open source: "Swift version 5.3-dev (LLVM ..., Swift ...)"
/// - Xcode: "swift-driver version: 1.87.3 Apple Swift version 5.9.2 ..."
///
/// Finds "version" keyword and takes the next token as the version string.
fn parse_swift_version_output(output: &str) -> Option<String> {
    // Take the first line (swift --version can be multiline)
    let first_line = output.lines().next()?;

    // Find "version" keyword, take the next whitespace-delimited token
    let mut tokens = first_line.split_whitespace();
    let _ = tokens.position(|t| t == "version" || t == "version:")?;
    let version = tokens.next()?;

    if version.is_empty() {
        return None;
    }

    Some(version.to_string())
}

/// Read `.swift-version` file from CWD (swiftenv local config).
fn read_swift_version_file(cwd: &Path) -> Option<String> {
    let content = std::fs::read_to_string(cwd.join(".swift-version")).ok()?;
    let line = content
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty() && !l.starts_with('#'))?;
    if line.is_empty() {
        return None;
    }
    Some(line.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_apple_swift() {
        assert_eq!(
            parse_swift_version_output("Apple Swift version 5.9.2 (swiftlang-5.9.2.2.56 clang-1500.1.0.2.5)"),
            Some("5.9.2".to_string()),
        );
    }

    #[test]
    fn parse_apple_swift_5_2() {
        assert_eq!(
            parse_swift_version_output("Apple Swift version 5.2.2 (swiftlang-1103.0.32.6 clang-1103.0.32.51)"),
            Some("5.2.2".to_string()),
        );
    }

    #[test]
    fn parse_open_source_swift_dev() {
        assert_eq!(
            parse_swift_version_output("Swift version 5.3-dev (LLVM abc123, Swift def456)"),
            Some("5.3-dev".to_string()),
        );
    }

    #[test]
    fn parse_xcode_swift_driver_prefix() {
        // Xcode 15+ prepends swift-driver version before the actual Swift version
        let output = "Apple Swift version 5.9.2 (swiftlang-5.9.2.2.56 clang-1500.1.0.2.5)";
        assert_eq!(
            parse_swift_version_output(output),
            Some("5.9.2".to_string()),
        );
    }

    #[test]
    fn parse_empty_output() {
        assert_eq!(parse_swift_version_output(""), None);
    }

    #[test]
    fn parse_no_version_keyword() {
        assert_eq!(parse_swift_version_output("Swift compiler"), None);
    }
}
