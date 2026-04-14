use std::path::{Path, PathBuf};

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Java runtime version.
///
/// Detection priority (mirrors Java module):
/// 1. `$JAVA_HOME/bin/java -Xinternalversion` (respects user's JAVA_HOME)
/// 2. `java -Xinternalversion` fallback (system PATH)
/// 3. `.java-version` file (jenv/jabba local version)
///
/// Parses the internal JVM version string which covers OpenJDK, Oracle HotSpot,
/// Eclipse OpenJ9, GraalVM, Zulu, Corretto, and SapMachine distributions.
pub struct JavaProvider;

impl ChipProvider for JavaProvider {
    fn id(&self) -> ChipId {
        "java"
    }

    fn display_name(&self) -> &str {
        "Java"
    }

    fn detect_files(&self) -> &[&str] {
        &[
            "pom.xml",
            "build.gradle",
            "build.gradle.kts",
            ".java-version",
            "build.sbt",
            ".deps.edn",
            ".sdkmanrc",
        ]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["java", "class", "jar"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = get_java_version(ctx);

        let label = version.clone().unwrap_or_default();

        let tooltip = match &version {
            Some(ver) => format!("Java {ver}"),
            None => "Java".into(),
        };

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Coffee"),
            tooltip: Some(tooltip),
            ..ChipOutput::default()
        }
    }
}

/// Resolve the Java version string.
///
/// Priority order :
/// 1. `$JAVA_HOME/bin/java` — respects user's configured JAVA_HOME
/// 2. `java` on PATH — system default
///
/// Uses `-Xinternalversion` which produces a single-line string containing
/// the JVM version in a parseable format across all major JDK distributions.
fn get_java_version(ctx: &ChipContext) -> Option<String> {
    // 1. Check .java-version file (jenv/jabba local config)
    if let Some(ver) = read_java_version_file(&ctx.cwd) {
        return Some(ver);
    }

    // 2. Try $JAVA_HOME/bin/java first
    if let Some(java_home) = ctx.get_env("JAVA_HOME") {
        let java_bin = PathBuf::from(java_home)
            .join("bin")
            .join("java");
        if let Some(path_str) = java_bin.to_str()
            && let Some(ver) = exec_java_version(ctx, path_str)
        {
            return Some(ver);
        }
    }

    // 3. Fall back to java on PATH
    exec_java_version(ctx, "java")
}

/// Execute `<java_binary> -Xinternalversion` and parse the version.
fn exec_java_version(ctx: &ChipContext, binary: &str) -> Option<String> {
    let output = ctx.exec_cmd(binary, &["-Xinternalversion"])?;
    let combined = if output.stdout.is_empty() {
        &output.stderr
    } else {
        &output.stdout
    };
    parse_java_version(combined)
}

/// Parse version from JVM internal version string.
///
/// Handles all major JDK distributions:
/// - OpenJDK: `OpenJDK 64-Bit Server VM (11.0.4+11-LTS) for linux-amd64 JRE (11.0.4+11-LTS)`
/// - Oracle HotSpot: `Java HotSpot(TM) Client VM (25.65-b01) for linux-arm JRE (1.8.0_65-b17)`
/// - Eclipse OpenJ9: `Eclipse OpenJ9 ... for Eclipse OpenJ9 11.0.4.0, built on`
/// - GraalVM: `GraalVM CE 19.2.0.1 (25.222-b08) for linux-amd64 JRE (8u222)`
///
/// The pattern extracts version from `JRE (X.Y.Z` or `OpenJ9 X.Y.Z`.
fn parse_java_version(version_string: &str) -> Option<String> {
    // Try "JRE (X.Y.Z" pattern first (covers OpenJDK, Oracle, Zulu, Corretto, SapMachine)
    if let Some(version) = extract_jre_version(version_string) {
        return Some(normalize_java_version(&version));
    }

    // Try "OpenJ9 X.Y.Z" pattern (Eclipse OpenJ9)
    if let Some(version) = extract_openj9_version(version_string) {
        return Some(version);
    }

    // Try "JRE (Xu222)" pattern (GraalVM short format)
    if let Some(version) = extract_graalvm_short_version(version_string) {
        return Some(version);
    }

    None
}

/// Extract version from `JRE (X.Y.Z` pattern.
fn extract_jre_version(s: &str) -> Option<String> {
    let jre_marker = "JRE (";
    let pos = s.find(jre_marker)?;
    let after = &s[pos + jre_marker.len()..];
    let version_end = after.find(|c: char| !c.is_ascii_digit() && c != '.')?;
    let version = &after[..version_end];
    if version.is_empty() {
        return None;
    }
    Some(version.to_string())
}

/// Extract version from `OpenJ9 X.Y.Z.W, built on` pattern.
fn extract_openj9_version(s: &str) -> Option<String> {
    let marker = "OpenJ9 ";
    // Find the last occurrence of "OpenJ9 " followed by a version number
    let mut search_from = 0;
    let mut result = None;
    while let Some(pos) = s[search_from..].find(marker) {
        let abs_pos = search_from + pos + marker.len();
        let after = &s[abs_pos..];
        if after.starts_with(|c: char| c.is_ascii_digit()) {
            let version_end = after
                .find(|c: char| !c.is_ascii_digit() && c != '.')
                .unwrap_or(after.len());
            let version = &after[..version_end];
            if !version.is_empty() {
                result = Some(version.to_string());
            }
        }
        search_from = abs_pos;
    }
    result
}

/// Extract version from GraalVM short format `JRE (8u222)`.
fn extract_graalvm_short_version(s: &str) -> Option<String> {
    let jre_marker = "JRE (";
    let pos = s.find(jre_marker)?;
    let after = &s[pos + jre_marker.len()..];
    let end = after.find(')')?;
    let version_part = &after[..end];
    // "8u222" → just "8"
    if version_part.contains('u') {
        let major = version_part.split('u').next()?;
        return Some(major.to_string());
    }
    None
}

/// Normalize old-style Java version numbers.
///
/// Java versions before 9 used `1.X.Y` format (e.g., `1.8.0` = Java 8).
/// We keep them as-is since that's the canonical version string for those releases.
fn normalize_java_version(version: &str) -> String {
    version.to_string()
}

/// Read `.java-version` file from CWD (jenv/jabba local config).
fn read_java_version_file(cwd: &Path) -> Option<String> {
    let content = std::fs::read_to_string(cwd.join(".java-version")).ok()?;
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
    fn parse_openjdk_8() {
        let input = "OpenJDK 64-Bit Server VM (25.222-b10) for linux-amd64 JRE (1.8.0_222-b10), built on Jul 11 2019 10:18:43 by \"openjdk\" with gcc 4.4.7 20120313 (Red Hat 4.4.7-23)";
        assert_eq!(parse_java_version(input), Some("1.8.0".to_string()));
    }

    #[test]
    fn parse_openjdk_11() {
        let input = "OpenJDK 64-Bit Server VM (11.0.4+11-post-Ubuntu-1ubuntu219.04) for linux-amd64 JRE (11.0.4+11-post-Ubuntu-1ubuntu219.04), built on Jul 18 2019 18:21:46 by \"build\" with gcc 8.3.0";
        assert_eq!(parse_java_version(input), Some("11.0.4".to_string()));
    }

    #[test]
    fn parse_oracle_hotspot_8() {
        let input = "Java HotSpot(TM) Client VM (25.65-b01) for linux-arm-vfp-hflt JRE (1.8.0_65-b17), built on Oct  6 2015 16:19:04 by \"java_re\" with gcc 4.7.2 20120910 (prerelease)";
        assert_eq!(parse_java_version(input), Some("1.8.0".to_string()));
    }

    #[test]
    fn parse_openjdk_12() {
        let input = "OpenJDK 64-Bit Server VM (12.0.2+10) for linux-amd64 JRE (12.0.2+10), built on Jul 18 2019 14:41:47 by \"jenkins\" with gcc 7.3.1 20180303 (Red Hat 7.3.1-5)";
        assert_eq!(parse_java_version(input), Some("12.0.2".to_string()));
    }

    #[test]
    fn parse_zulu_17() {
        let input = "OpenJDK 64-Bit Server VM (17.0.5+8-LTS) for bsd-amd64 JRE (17.0.5+8-LTS) (Zulu17.38+21-CA), built on Oct  7 2022 06:03:12 by \"zulu_re\" with clang 4.2.1 Compatible Apple LLVM 11.0.0 (clang-1100.0.33.17)";
        assert_eq!(parse_java_version(input), Some("17.0.5".to_string()));
    }

    #[test]
    fn parse_eclipse_openj9_8() {
        let input = "Eclipse OpenJ9 OpenJDK 64-bit Server VM (1.8.0_222-b10) from linux-amd64 JRE with Extensions for OpenJDK for Eclipse OpenJ9 8.0.222.0, built on Jul 17 2019 21:29:18 by jenkins with g++ (GCC) 7.3.1 20180303 (Red Hat 7.3.1-5)";
        assert_eq!(parse_java_version(input), Some("8.0.222".to_string()));
    }

    #[test]
    fn parse_eclipse_openj9_11() {
        let input = "Eclipse OpenJ9 OpenJDK 64-bit Server VM (11.0.4+11) from linux-amd64 JRE with Extensions for OpenJDK for Eclipse OpenJ9 11.0.4.0, built on Jul 17 2019 21:51:37 by jenkins with g++ (GCC) 7.3.1 20180303 (Red Hat 7.3.1-5)";
        assert_eq!(parse_java_version(input), Some("11.0.4".to_string()));
    }

    #[test]
    fn parse_graalvm_8() {
        let input = "OpenJDK 64-Bit GraalVM CE 19.2.0.1 (25.222-b08-jvmci-19.2-b02) for linux-amd64 JRE (8u222), built on Jul 19 2019 17:37:13 by \"buildslave\" with gcc 7.3.0";
        assert_eq!(parse_java_version(input), Some("8".to_string()));
    }

    #[test]
    fn parse_sapmachine_11() {
        let input = "OpenJDK 64-Bit Server VM (11.0.4+11-LTS-sapmachine) for linux-amd64 JRE (11.0.4+11-LTS-sapmachine), built on Jul 17 2019 08:58:43 by \"\" with gcc 7.3.0";
        assert_eq!(parse_java_version(input), Some("11.0.4".to_string()));
    }

    #[test]
    fn parse_android_studio_jdk() {
        let input = "OpenJDK 64-Bit Server VM (11.0.15+0-b2043.56-8887301) for linux-amd64 JRE (11.0.15+0-b2043.56-8887301), built on Jul 29 2022 22:12:21 by \"androidbuild\" with gcc Android (7284624, based on r416183b) Clang 12.0.5";
        assert_eq!(parse_java_version(input), Some("11.0.15".to_string()));
    }

    #[test]
    fn parse_unknown_jre() {
        assert_eq!(parse_java_version("Unknown JRE"), None);
    }

    #[test]
    fn parse_empty_string() {
        assert_eq!(parse_java_version(""), None);
    }
}
