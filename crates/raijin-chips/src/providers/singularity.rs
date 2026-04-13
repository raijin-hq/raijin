use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Singularity/Apptainer container name.
///
/// Availability: `$SINGULARITY_NAME` or `$APPTAINER_NAME` environment variable is set.
/// Label: container name from the environment variable.
pub struct SingularityProvider;

impl ChipProvider for SingularityProvider {
    fn id(&self) -> ChipId {
        "singularity"
    }

    fn display_name(&self) -> &str {
        "Singularity"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.has_env("SINGULARITY_NAME") || ctx.has_env("APPTAINER_NAME")
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let name = ctx
            .get_env("SINGULARITY_NAME")
            .or_else(|| ctx.get_env("APPTAINER_NAME"))
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: name.clone(),
            icon: Some("Container"),
            tooltip: if name.is_empty() {
                None
            } else {
                Some(format!("Singularity container: {name}"))
            },
            ..ChipOutput::default()
        }
    }
}
