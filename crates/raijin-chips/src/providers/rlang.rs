use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for R language version.
///
/// Detection: `DESCRIPTION`, `.Rprofile` files; `R`, `Rd`, `Rmd`, `Rproj` extensions.
/// Version:   `R --version` — parses first line for version number.
///
/// Note: `R --version` prints to stderr on some platforms, stdout on others.
/// We check both stdout and stderr.
pub struct RlangProvider;

impl ChipProvider for RlangProvider {
    fn id(&self) -> ChipId {
        "rlang"
    }

    fn display_name(&self) -> &str {
        "R"
    }

    fn detect_files(&self) -> &[&str] {
        &["DESCRIPTION", ".Rprofile"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["R", "Rd", "Rmd", "Rproj"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("R", &["--version"])
            .and_then(|o| {
                // R --version may output to stdout or stderr depending on platform
                let combined = if o.stdout.is_empty() {
                    o.stderr
                } else {
                    o.stdout
                };
                parse_r_version(&combined)
            })
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("Rlang"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("R {version}"))
            },
            ..ChipOutput::default()
        }
    }
}

/// Parse `R --version` output.
///
/// First line format: "R version 4.1.0 (2021-05-18) -- \"Camp Pontanezen\""
/// Split on whitespace and take the 3rd token (index 2): "4.1.0"
fn parse_r_version(r_version: &str) -> Option<String> {
    r_version
        .lines()
        .next()?
        .split_whitespace()
        .nth(2)
        .map(ToString::to_string)
}
