use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for V language version.
///
/// Detection: `v.mod`, `vpkg.json`, `.vpkg-lock.json`, `.v` files.
/// Version: `v version` -> `V 0.4.4 abcdef12` -> `0.4.4`.
///

pub struct VlangProvider;

impl ChipProvider for VlangProvider {
    fn id(&self) -> ChipId {
        "vlang"
    }

    fn display_name(&self) -> &str {
        "V"
    }

    fn detect_files(&self) -> &[&str] {
        &["v.mod", "vpkg.json", ".vpkg-lock.json"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["v"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("v", &["version"])
            .and_then(|o| parse_v_version(&o.stdout))
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("V"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("V {version}"))
            },
            ..ChipOutput::default()
        }
    }
}

/// Parse V version from `v version` output.
///
/// Input: `V 0.4.4 abcdef12\n`
/// Output: `Some("0.4.4")`
fn parse_v_version(stdout: &str) -> Option<String> {
    let version = stdout.split_whitespace().nth(1)?;
    if version.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        Some(version.to_string())
    } else {
        None
    }
}
