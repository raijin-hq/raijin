use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Dart SDK version.
///
/// Detection: `pubspec.yaml`, `pubspec.yml`, `pubspec.lock` files; `dart` extension
/// Version:   `dart --version` → "Dart SDK version: 3.3.0 (stable) ..." → "3.3.0"
///
/// Note: Before Dart 2.15, version was on stderr. After 2.15, it moved to stdout.
/// We merge stdout+stderr; we check stdout first, then stderr.
pub struct DartProvider;

impl ChipProvider for DartProvider {
    fn id(&self) -> ChipId {
        "dart"
    }

    fn display_name(&self) -> &str {
        "Dart"
    }

    fn detect_files(&self) -> &[&str] {
        &["pubspec.yaml", "pubspec.yml", "pubspec.lock"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["dart"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("dart", &["--version"])
            .and_then(|o| {
                // Try stdout first (Dart >= 2.15), fall back to stderr (older Dart)
                parse_dart_version(&o.stdout).or_else(|| parse_dart_version(&o.stderr))
            })
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Target"),
            tooltip: Some("Dart SDK version".into()),
            ..ChipOutput::default()
        }
    }
}

/// Parse `dart --version` output.
///
/// Format: "Dart SDK version: 3.3.0 (stable) (Tue Feb 13 ...)" → "3.3.0"
/// We split on whitespace and takes the 4th token (index 3).
fn parse_dart_version(output: &str) -> Option<String> {
    let version = output.split_whitespace().nth(3)?;
    // Sanity check: version should start with a digit
    if version.starts_with(|c: char| c.is_ascii_digit()) {
        Some(version.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stable() {
        assert_eq!(
            parse_dart_version("Dart SDK version: 3.3.0 (stable) (Tue Feb 13 09:00:00 2024)"),
            Some("3.3.0".to_string()),
        );
    }

    #[test]
    fn parse_older_format() {
        assert_eq!(
            parse_dart_version(
                "Dart VM version: 2.8.4 (stable) (Wed Jun 3 12:26:04 2020 +0200) on \"linux_x64\""
            ),
            Some("2.8.4".to_string()),
        );
    }

    #[test]
    fn parse_newer_stdout() {
        assert_eq!(
            parse_dart_version(
                "Dart SDK version: 2.15.1 (stable) (Tue Dec 14 13:32:21 2021 +0100) on \"linux_x64\""
            ),
            Some("2.15.1".to_string()),
        );
    }

    #[test]
    fn parse_empty() {
        assert_eq!(parse_dart_version(""), None);
    }

    #[test]
    fn parse_garbage() {
        assert_eq!(parse_dart_version("not a dart version"), None);
    }
}
