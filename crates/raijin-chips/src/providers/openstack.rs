use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the active OpenStack cloud.
///
/// Shows the cloud name from the `$OS_CLOUD` environment variable.
/// Only visible when the variable is set.
pub struct OpenstackProvider;

impl ChipProvider for OpenstackProvider {
    fn id(&self) -> ChipId {
        "openstack"
    }

    fn display_name(&self) -> &str {
        "OpenStack"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.has_env("OS_CLOUD")
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let cloud = ctx
            .get_env("OS_CLOUD")
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: cloud,
            icon: Some("Cloud"),
            tooltip: Some("OpenStack cloud".into()),
            ..ChipOutput::default()
        }
    }
}
