use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Nim language version.
///
/// Detection: `.nim`, `.nimble`, `.nims`, `.nimf`, `nim.cfg` files.
/// Version: `nim --version` -> `Nim Compiler Version 2.0.0 [Linux: amd64]` -> `2.0.0`.
/// Also checks `choosenim` for version manager awareness.
pub struct NimProvider;

impl ChipProvider for NimProvider {
    fn id(&self) -> ChipId {
        "nim"
    }

    fn display_name(&self) -> &str {
        "Nim"
    }

    fn detect_files(&self) -> &[&str] {
        &["nim.cfg"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["nim", "nimble", "nims"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("nim", &["--version"])
            .and_then(|o| parse_nim_version(&o.stdout))
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("Crown"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("Nim {version}"))
            },
            ..ChipOutput::default()
        }
    }
}

/// Parse Nim version from `nim --version` output.
///
/// Input:
/// ```text
/// Nim Compiler Version 2.0.0 [Linux: amd64]
/// Compiled at 2023-08-01
/// Copyright (c) 2006-2023 by Andreas Rumpf
/// ```
/// Output: `Some("2.0.0")`
///
/// Takes the first line and finds the word that is all digits and dots.
fn parse_nim_version(output: &str) -> Option<String> {
    let first_line = output.lines().next()?;
    let version = first_line
        .split(' ')
        .find(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit() || c == '.'))?;
    Some(version.to_string())
}
