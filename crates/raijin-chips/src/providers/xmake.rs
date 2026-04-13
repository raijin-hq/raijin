use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for XMake build system version.
///
/// Detection: `xmake.lua` file.
/// Version:   `xmake --version` → "xmake v2.9.5+HEAD.0db4fe6, ..." → "2.9.5"
pub struct XmakeProvider;

impl ChipProvider for XmakeProvider {
    fn id(&self) -> ChipId {
        "xmake"
    }

    fn display_name(&self) -> &str {
        "XMake"
    }

    fn detect_files(&self) -> &[&str] {
        &["xmake.lua"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("xmake", &["--version"])
            .and_then(|o| parse_xmake_version(&o.stdout))
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("Code"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("XMake {version}"))
            },
            ..ChipOutput::default()
        }
    }
}

/// Parse `xmake --version` output.
///
/// Format: "xmake v2.9.5+HEAD.0db4fe6, A cross-platform build utility based on Lua"
/// Split on whitespace, take 2nd token "v2.9.5+HEAD.0db4fe6",
/// strip "v" prefix, split on '+', take first part "2.9.5".
fn parse_xmake_version(xmake_version: &str) -> Option<String> {
    Some(
        xmake_version
            .split_whitespace()
            .nth(1)?
            .trim_start_matches('v')
            .split('+')
            .next()?
            .to_string(),
    )
}
