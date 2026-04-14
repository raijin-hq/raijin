use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider, parse_version_number};

/// Chip provider for Gradle version.
///
/// Detection strategy (standard
///
/// 1. Read `gradle/wrapper/gradle-wrapper.properties` and parse the version
///    from the `distributionUrl` line (e.g., `gradle-8.5-bin.zip` → `8.5`).
/// 2. Walk parent directories looking for the wrapper properties file.
/// 3. Fall back to `gradle --version` if no wrapper properties found.
///
/// Activates when `build.gradle`, `build.gradle.kts`, or `settings.gradle` is present,
/// or when a `gradle` directory exists.
pub struct GradleProvider;

impl ChipProvider for GradleProvider {
    fn id(&self) -> ChipId {
        "gradle"
    }

    fn display_name(&self) -> &str {
        "Gradle"
    }

    fn detect_files(&self) -> &[&str] {
        &["build.gradle", "build.gradle.kts", "settings.gradle", "settings.gradle.kts"]
    }

    fn detect_folders(&self) -> &[&str] {
        &["gradle"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = resolve_gradle_version(ctx);

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("Gradle"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("Gradle {version}"))
            },
            ..ChipOutput::default()
        }
    }
}

/// Resolve Gradle version using wrapper properties first, then CLI fallback.
fn resolve_gradle_version(ctx: &ChipContext) -> String {
    // Strategy 1: Read gradle-wrapper.properties (no subprocess needed)
    if let Some(version) = read_wrapper_version(ctx) {
        return version;
    }

    // Strategy 2: Fall back to `gradle --version` CLI
    if let Some(output) = ctx.exec_cmd("gradle", &["--version"]) {
        return parse_gradle_cli_version(&output.stdout);
    }

    String::new()
}

/// Read version from `gradle/wrapper/gradle-wrapper.properties`.
///
/// Walks from CWD upward through parent directories, checking each for
/// the wrapper properties file. The `distributionUrl` line contains the
/// Gradle version:
///
/// ```text
/// distributionUrl=https\://services.gradle.org/distributions/gradle-8.5-bin.zip
/// ```
///
/// Parses `gradle-8.5-bin.zip` → `8.5`.
fn read_wrapper_version(ctx: &ChipContext) -> Option<String> {
    let mut dir = ctx.cwd.as_path();
    loop {
        let props_path = dir.join("gradle/wrapper/gradle-wrapper.properties");
        if let Ok(content) = std::fs::read_to_string(&props_path)
            && let Some(version) = parse_distribution_url_version(&content)
        {
            return Some(version);
        }
        dir = dir.parent()?;
    }
}

/// Parse the Gradle version from a `distributionUrl` line.
///
/// Handles URLs like:
/// - `https\://services.gradle.org/distributions/gradle-8.5-bin.zip`
/// - `https\://services.gradle.org/distributions/gradle-7.5.1-all.zip`
fn parse_distribution_url_version(content: &str) -> Option<String> {
    let line = content
        .lines()
        .find(|l| l.starts_with("distributionUrl="))?;

    // Take the filename after the last `/`
    let filename = line.rsplit_once('/')?.1;

    // "gradle-8.5-bin.zip" → strip prefix "gradle-", then strip suffix "-bin.zip" or "-all.zip"
    let after_prefix = filename.strip_prefix("gradle-")?;
    let version = after_prefix.rsplit_once('-')?.0;

    if version.is_empty() {
        return None;
    }
    Some(version.to_string())
}

/// Parse version from `gradle --version` CLI output.
///
/// Output looks like:
/// ```text
/// ------------------------------------------------------------
/// Gradle 8.5
/// ------------------------------------------------------------
/// ```
///
/// We look for the line starting with "Gradle " and extract the version.
fn parse_gradle_cli_version(output: &str) -> String {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(version) = trimmed.strip_prefix("Gradle ") {
            let version = version.trim();
            if !version.is_empty() {
                return version.to_string();
            }
        }
    }
    parse_version_number(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_distribution_url_version() {
        let content = "distributionBase=GRADLE_USER_HOME\n\
                        distributionPath=wrapper/dists\n\
                        distributionUrl=https\\://services.gradle.org/distributions/gradle-8.5-bin.zip\n\
                        zipStoreBase=GRADLE_USER_HOME\n\
                        zipStorePath=wrapper/dists";
        assert_eq!(
            parse_distribution_url_version(content),
            Some("8.5".into())
        );
    }

    #[test]
    fn test_parse_distribution_url_version_with_patch() {
        let content = "distributionUrl=https\\://services.gradle.org/distributions/gradle-7.5.1-all.zip";
        assert_eq!(
            parse_distribution_url_version(content),
            Some("7.5.1".into())
        );
    }

    #[test]
    fn test_parse_gradle_cli_version() {
        let output = "\n------------------------------------------------------------\nGradle 8.5\n------------------------------------------------------------\n\nBuild time: ...\n";
        assert_eq!(parse_gradle_cli_version(output), "8.5");
    }

    #[test]
    fn test_parse_gradle_cli_empty() {
        assert_eq!(parse_gradle_cli_version(""), "");
    }
}
