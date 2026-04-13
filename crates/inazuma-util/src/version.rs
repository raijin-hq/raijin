use versions::Versioning;

/// Format a version string using a template with variable substitution.
///
/// Supported variables:
/// - `${raw}` — original version string unchanged
/// - `${major}` — major version number
/// - `${minor}` — minor version number
/// - `${patch}` — patch version number
///
/// # Examples
///
/// ```
/// use inazuma_util::version::format_version;
///
/// assert_eq!(format_version("1.22.3", "v${major}.${minor}"), "v1.22");
/// assert_eq!(format_version("1.22.3", "v${raw}"), "v1.22.3");
/// assert_eq!(format_version("1.22.3", "${major}.${minor}.${patch}"), "1.22.3");
/// assert_eq!(format_version("2.0", "v${major}"), "v2");
/// ```
pub fn format_version(version: &str, format: &str) -> String {
    let parsed = Versioning::new(version);

    let (major, minor, patch) = match &parsed {
        Some(Versioning::Ideal(v)) => (
            Some(v.major.to_string()),
            Some(v.minor.to_string()),
            Some(v.patch.to_string()),
        ),
        Some(Versioning::General(v)) => (
            v.nth_lenient(0).map(|n| n.to_string()),
            v.nth_lenient(1).map(|n| n.to_string()),
            v.nth_lenient(2).map(|n| n.to_string()),
        ),
        _ => (None, None, None),
    };

    format
        .replace("${raw}", version)
        .replace("${major}", &major.unwrap_or_default())
        .replace("${minor}", &minor.unwrap_or_default())
        .replace("${patch}", &patch.unwrap_or_default())
}

/// Format a version for a named module, with fallback to `"v{version}"` on error.
///
/// Uses the given format template. If the version can't be parsed, falls back
/// to prepending `v` to the raw version string.
pub fn format_module_version(module_name: &str, version: &str, version_format: &str) -> String {
    let result = format_version(version, version_format);
    if result.is_empty() {
        log::warn!("Error formatting `{module_name}` version: empty result for `{version}`");
        format!("v{version}")
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_full() {
        assert_eq!(
            format_version("1.2.3", "v${major}.${minor}.${patch}"),
            "v1.2.3"
        );
    }

    #[test]
    fn test_semver_major_minor() {
        assert_eq!(format_version("1.22.3", "v${major}.${minor}"), "v1.22");
    }

    #[test]
    fn test_raw() {
        assert_eq!(format_version("1.22.3-beta", "v${raw}"), "v1.22.3-beta");
    }

    #[test]
    fn test_partial_version() {
        assert_eq!(format_version("1.2", "v${major}.${minor}"), "v1.2");
    }

    #[test]
    fn test_major_only() {
        assert_eq!(format_version("3.0.0", "${major}"), "3");
    }

    #[test]
    fn test_non_semver() {
        // Non-parseable versions still work via ${raw}
        assert_eq!(format_version("nightly", "v${raw}"), "vnightly");
    }

    #[test]
    fn test_module_version_fallback() {
        assert_eq!(
            format_module_version("test", "1.2.3", "v${raw}"),
            "v1.2.3"
        );
    }
}
