use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the Odin programming language version.
///
/// Detection: extensions `odin`
/// Version:   `odin version` → "odin version dev-2024-03:abc123" → "dev-2024-03"
pub struct OdinProvider;

impl ChipProvider for OdinProvider {
    fn id(&self) -> ChipId {
        "odin"
    }

    fn display_name(&self) -> &str {
        "Odin"
    }

    fn detect_extensions(&self) -> &[&str] {
        &["odin"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("odin", &["version"])
            .and_then(|o| parse_odin_version(&o.stdout))
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Odin"),
            tooltip: Some("Odin version".into()),
            ..ChipOutput::default()
        }
    }
}

/// Parse `odin version` output.
///
/// Output format: "odin version dev-2024-03:abc123\n"
/// Extracts the last whitespace-separated token, then strips the commit hash
/// after the colon.
fn parse_odin_version(output: &str) -> Option<String> {
    let trimmed_version = output.split(' ').next_back()?.trim().to_string();
    let no_commit = trimmed_version.split(':').next()?.trim().to_string();
    Some(no_commit)
}
