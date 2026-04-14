use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Scala language version.
///
/// Detection: `build.sbt`, `.scalafix.conf`, `.scalaenv`, `.sbtenv`, `.metals/`, `.scala`/`.sc` files.
/// Version: Tries `scala-cli version --scala` first (fast, returns just the version),
///   then falls back to `scalac -version` (`Scala compiler version 3.4.1 -- ...` -> `3.4.1`).
pub struct ScalaProvider;

impl ChipProvider for ScalaProvider {
    fn id(&self) -> ChipId {
        "scala"
    }

    fn display_name(&self) -> &str {
        "Scala"
    }

    fn detect_files(&self) -> &[&str] {
        &["build.sbt", ".scalafix.conf", ".scalaenv", ".sbtenv"]
    }

    fn detect_folders(&self) -> &[&str] {
        &[".metals"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["scala", "sc"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        // scala-cli is faster and returns just the version string
        let version = ctx
            .exec_cmd("scala-cli", &["version", "--scala"])
            .map(|o| o.stdout.trim().to_string())
            .filter(|v| !v.is_empty())
            .or_else(|| {
                ctx.exec_cmd("scalac", &["-version"])
                    .and_then(|o| parse_scalac_version(&combined_output(&o.stdout, &o.stderr)))
            })
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("Scala"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("Scala {version}"))
            },
            ..ChipOutput::default()
        }
    }
}

/// Combine stdout and stderr — `scalac -version` may print to either stream.
fn combined_output(stdout: &str, stderr: &str) -> String {
    if stdout.trim().is_empty() {
        stderr.to_string()
    } else {
        stdout.to_string()
    }
}

/// Parse Scala version from `scalac -version` output.
///
/// Input: `Scala compiler version 3.4.1 -- Copyright 2002-2024, LAMP/EPFL`
/// Output: `Some("3.4.1")`
///
/// Also handles Dotty: `Scala compiler version 3.0.0-RC1 -- ...` -> `3.0.0-RC1`
fn parse_scalac_version(output: &str) -> Option<String> {
    // "Scala compiler version 3.4.1 -- ..." -> split and take the 4th word
    let version = output.split_whitespace().nth(3)?;
    Some(version.to_string())
}
