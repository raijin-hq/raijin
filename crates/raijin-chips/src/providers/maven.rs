use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Apache Maven version.
///
/// Detection strategy:
///
/// 1. Read `.mvn/wrapper/maven-wrapper.properties` for `distributionUrl`
///    and parse the version from it (e.g., `apache-maven-3.9.6-bin.zip` → `3.9.6`).
/// 2. Fall back to `mvn --version` which outputs:
///    `Apache Maven 3.9.6 (bc0240f3c744dd6b6ec2920b3cd08dcc295161ae)`
///
/// Activates when `pom.xml`, `mvnw`, or `.mvn` directory is present.
pub struct MavenProvider;

impl ChipProvider for MavenProvider {
    fn id(&self) -> ChipId {
        "maven"
    }

    fn display_name(&self) -> &str {
        "Maven"
    }

    fn detect_files(&self) -> &[&str] {
        &["pom.xml", "mvnw"]
    }

    fn detect_folders(&self) -> &[&str] {
        &[".mvn"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = resolve_maven_version(ctx);

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("Maven"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("Apache Maven {version}"))
            },
            ..ChipOutput::default()
        }
    }
}

/// Resolve Maven version using wrapper properties first, then CLI.
fn resolve_maven_version(ctx: &ChipContext) -> String {
    // Strategy 1: Read maven-wrapper.properties
    if let Some(version) = read_wrapper_version(ctx) {
        return version;
    }

    // Strategy 2: `mvn --version` CLI
    if let Some(output) = ctx.exec_cmd("mvn", &["--version"]) {
        return parse_mvn_version(&output.stdout);
    }

    String::new()
}

/// Read version from `.mvn/wrapper/maven-wrapper.properties`.
///
/// The `distributionUrl` line contains URLs like:
/// `https://repo.maven.apache.org/maven2/org/apache/maven/apache-maven/3.9.6/apache-maven-3.9.6-bin.zip`
fn read_wrapper_version(ctx: &ChipContext) -> Option<String> {
    let mut dir = ctx.cwd.as_path();
    loop {
        let props_path = dir.join(".mvn/wrapper/maven-wrapper.properties");
        if let Ok(content) = std::fs::read_to_string(&props_path) {
            if let Some(version) = parse_wrapper_distribution_url(&content) {
                return Some(version);
            }
        }
        dir = dir.parent()?;
    }
}

/// Parse Maven version from `distributionUrl` in wrapper properties.
///
/// Handles URLs like:
/// - `https://repo.maven.apache.org/.../apache-maven-3.9.6-bin.zip`
fn parse_wrapper_distribution_url(content: &str) -> Option<String> {
    let line = content
        .lines()
        .find(|l| l.starts_with("distributionUrl=") || l.starts_with("wrapperUrl="))?;

    // Find "apache-maven-" in the URL and extract the version
    let url_part = line.split_once('=')?.1;
    let filename = url_part.rsplit_once('/')?.1;

    let after_prefix = filename.strip_prefix("apache-maven-")?;
    let version = after_prefix.rsplit_once('-')?.0;

    if version.is_empty() {
        return None;
    }
    Some(version.to_string())
}

/// Parse version from `mvn --version` output.
///
/// First line: `Apache Maven 3.9.6 (bc0240f3c744dd6b6ec2920b3cd08dcc295161ae)`
fn parse_mvn_version(output: &str) -> String {
    let first_line = output.lines().next().unwrap_or("").trim();

    if let Some(after) = first_line.strip_prefix("Apache Maven ") {
        // "3.9.6 (hash...)" → take until space or paren
        let version = after
            .split_once(|c: char| c == ' ' || c == '(')
            .map(|(v, _)| v.trim())
            .unwrap_or(after.trim());

        if !version.is_empty() {
            return version.to_string();
        }
    }

    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mvn_version() {
        let output = "Apache Maven 3.9.6 (bc0240f3c744dd6b6ec2920b3cd08dcc295161ae)\nMaven home: /usr/local/Cellar/maven/3.9.6/libexec\n";
        assert_eq!(parse_mvn_version(output), "3.9.6");
    }

    #[test]
    fn test_parse_mvn_version_no_hash() {
        assert_eq!(parse_mvn_version("Apache Maven 3.9.6\n"), "3.9.6");
    }

    #[test]
    fn test_parse_wrapper_distribution_url() {
        let content = "distributionUrl=https://repo.maven.apache.org/maven2/org/apache/maven/apache-maven/3.9.6/apache-maven-3.9.6-bin.zip\nwrapperVersion=3.2.0";
        assert_eq!(
            parse_wrapper_distribution_url(content),
            Some("3.9.6".into())
        );
    }

    #[test]
    fn test_parse_mvn_empty() {
        assert_eq!(parse_mvn_version(""), "");
    }
}
