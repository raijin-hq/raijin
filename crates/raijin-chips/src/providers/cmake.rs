use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for CMake version.
///
/// Parses `cmake --version` output which looks like:
/// ```text
/// cmake version 3.28.0
/// CMake suite maintained and supported by Kitware (kitware.com/cmake).
/// ```
///
/// Only the first line's version number is extracted.
/// Activates when `CMakeLists.txt` or `CMakePresets.json` is present.
pub struct CmakeProvider;

impl ChipProvider for CmakeProvider {
    fn id(&self) -> ChipId {
        "cmake"
    }

    fn display_name(&self) -> &str {
        "CMake"
    }

    fn detect_files(&self) -> &[&str] {
        &["CMakeLists.txt", "CMakePresets.json"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("cmake", &["--version"])
            .and_then(|o| parse_cmake_version(&o.stdout))
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("Triangle"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("CMake {version}"))
            },
            ..ChipOutput::default()
        }
    }
}

/// Parse version from `cmake --version` output.
///
/// The first line is always `cmake version X.Y.Z`, sometimes with
/// `-rc1` or other suffixes.
fn parse_cmake_version(output: &str) -> Option<String> {
    let first_line = output.lines().next()?.trim();
    // "cmake version 3.28.0" → split and take the last token
    let version = first_line
        .strip_prefix("cmake version ")?
        .trim();

    if version.is_empty() {
        return None;
    }
    Some(version.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cmake_version() {
        let output = "cmake version 3.28.0\n\nCMake suite maintained and supported by Kitware (kitware.com/cmake).\n";
        assert_eq!(parse_cmake_version(output), Some("3.28.0".into()));
    }

    #[test]
    fn test_parse_cmake_version_rc() {
        let output = "cmake version 3.29.0-rc1\n";
        assert_eq!(parse_cmake_version(output), Some("3.29.0-rc1".into()));
    }

    #[test]
    fn test_parse_cmake_version_empty() {
        assert_eq!(parse_cmake_version(""), None);
    }
}
