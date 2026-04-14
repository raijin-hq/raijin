use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Kotlin language version.
///
/// Detection: `build.gradle.kts`, `settings.gradle.kts`, `.kt`/`.kts` files.
/// Version: Parsed from `kotlin -version` or `kotlinc -version`.
///   - `kotlin -version`: `Kotlin version 2.0.0-release-341 (JRE 21.0.3+9)` -> `2.0.0`
///   - `kotlinc -version`: `info: kotlinc-jvm 2.0.0 (JRE 21.0.3+9)` -> `2.0.0`
pub struct KotlinProvider;

impl ChipProvider for KotlinProvider {
    fn id(&self) -> ChipId {
        "kotlin"
    }

    fn display_name(&self) -> &str {
        "Kotlin"
    }

    fn detect_files(&self) -> &[&str] {
        &["build.gradle.kts", "settings.gradle.kts"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["kt", "kts"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        // Try `kotlin -version` first (runtime), then `kotlinc -version` (compiler)
        let version = ctx
            .exec_cmd("kotlin", &["-version"])
            .and_then(|o| parse_kotlin_version(&combined_output(&o.stdout, &o.stderr)))
            .or_else(|| {
                ctx.exec_cmd("kotlinc", &["-version"])
                    .and_then(|o| parse_kotlin_version(&combined_output(&o.stdout, &o.stderr)))
            })
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("Kotlin"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("Kotlin {version}"))
            },
            ..ChipOutput::default()
        }
    }
}

/// Combine stdout and stderr — `kotlin -version` may output to either stream.
fn combined_output(stdout: &str, stderr: &str) -> String {
    if stdout.trim().is_empty() {
        stderr.to_string()
    } else {
        stdout.to_string()
    }
}

/// Parse Kotlin version from `kotlin -version` or `kotlinc -version` output.
///
/// Extracts the first version-like pattern (digits and dots) from the output.
///
/// - `Kotlin version 2.0.0-release-341 (JRE 21.0.3+9)` -> `2.0.0`
/// - `info: kotlinc-jvm 2.0.0 (JRE 21.0.3+9)` -> `2.0.0`
fn parse_kotlin_version(output: &str) -> Option<String> {
    // Find the first word that looks like a version number (digits and dots)
    for word in output.split_whitespace() {
        // Strip release suffixes like "2.0.0-release-341"
        let base = word.split('-').next().unwrap_or(word);
        if !base.is_empty()
            && base.chars().next().is_some_and(|c| c.is_ascii_digit())
            && base.contains('.')
            && base.chars().all(|c| c.is_ascii_digit() || c == '.')
        {
            return Some(base.to_string());
        }
    }
    None
}
