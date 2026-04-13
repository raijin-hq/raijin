use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider, parse_version_number};

/// Chip provider for Helm version.
///
/// Runs `helm version --short` and parses the version number.
/// Detects based on `Chart.yaml` and `helmfile.yaml` presence.
pub struct HelmProvider;

impl ChipProvider for HelmProvider {
    fn id(&self) -> ChipId {
        "helm"
    }

    fn display_name(&self) -> &str {
        "Helm"
    }

    fn detect_files(&self) -> &[&str] {
        &["Chart.yaml", "helmfile.yaml"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("helm", &["version", "--short"])
            .map(|o| parse_helm_version(&o.stdout))
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Anchor"),
            tooltip: Some("Helm version".into()),
            ..ChipOutput::default()
        }
    }
}

/// Parse Helm version from `helm version --short` output.
///
/// Output looks like: `v3.14.2+gc309b6f` or `v3.14.2`
/// We extract just the semver part.
fn parse_helm_version(output: &str) -> String {
    let trimmed = output.trim();
    // Strip leading 'v', strip everything after '+'
    let version = trimmed
        .trim_start_matches('v')
        .split('+')
        .next()
        .unwrap_or(trimmed);

    if version.is_empty() {
        parse_version_number(output)
    } else {
        version.to_string()
    }
}
