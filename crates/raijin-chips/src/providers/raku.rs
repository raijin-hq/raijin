use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Raku (Perl 6) version.
///
/// Detection: `p6`, `pm6`, `pod6`, `raku`, `rakumod` extensions.
/// Version:   `raku --version` — parses Raku language version and VM from multi-line output.
pub struct RakuProvider;

impl ChipProvider for RakuProvider {
    fn id(&self) -> ChipId {
        "raku"
    }

    fn display_name(&self) -> &str {
        "Raku"
    }

    fn detect_extensions(&self) -> &[&str] {
        &["p6", "pm6", "pod6", "raku", "rakumod"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let (label, tooltip) = ctx
            .exec_cmd("raku", &["--version"])
            .and_then(|o| parse_raku_version(&o.stdout))
            .map(|(raku_ver, vm_ver)| {
                let label = format!("{raku_ver}-{vm_ver}");
                let tooltip = format!("Raku {raku_ver} on {vm_ver}");
                (label, Some(tooltip))
            })
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Raku"),
            tooltip,
            ..ChipOutput::default()
        }
    }
}

/// Parse `raku --version` output.
///
/// Example output:
/// ```text
/// Welcome to Rakudo™ v2021.12.
/// Implementing the Raku® Programming Language v6.d.
/// Built on MoarVM version 2021.12.
/// ```
///
/// Returns (raku_version, vm_version) e.g. ("v6.d", "moar").
fn parse_raku_version(version: &str) -> Option<(String, String)> {
    let mut lines = version.lines();
    // skip 1st line ("Welcome to Rakudo™ ...")
    let _ = lines.next()?;
    // split 2nd line, take "v6.d." at index 5, strip trailing "."
    let raku_version = lines
        .next()?
        .split_whitespace()
        .nth(5)?
        .strip_suffix('.')?
        .to_string();

    // split 3rd line, take VM name at index 2
    // "MoarVM" → "Moar" (community preference), leave other VMs as-is
    let vm_version = lines
        .next()?
        .split_whitespace()
        .nth(2)?
        .replace("MoarVM", "Moar");

    Some((raku_version.to_lowercase(), vm_version.to_lowercase()))
}
