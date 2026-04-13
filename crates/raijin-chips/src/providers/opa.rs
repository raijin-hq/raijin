use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Open Policy Agent (OPA) version.
///
/// Detection: files `.rego`, extensions `rego`
/// Version:   `opa version` → "Version: 0.44.0\n..." → "0.44.0"
pub struct OpaProvider;

impl ChipProvider for OpaProvider {
    fn id(&self) -> ChipId {
        "opa"
    }

    fn display_name(&self) -> &str {
        "OPA"
    }

    fn detect_files(&self) -> &[&str] {
        &[".rego"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["rego"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("opa", &["version"])
            .and_then(|o| parse_opa_version(&o.stdout))
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Opa"),
            tooltip: Some("OPA version".into()),
            ..ChipOutput::default()
        }
    }
}

/// Parse `opa version` output: "Version: 0.44.0\nBuild Commit: ..." → "0.44.0"
fn parse_opa_version(version_output: &str) -> Option<String> {
    Some(version_output.split_whitespace().nth(1)?.to_string())
}
