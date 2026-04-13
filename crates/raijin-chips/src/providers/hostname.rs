use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the current hostname.
///
/// Always visible. Reads from `ShellContext::hostname`.
pub struct HostnameProvider;

impl ChipProvider for HostnameProvider {
    fn id(&self) -> ChipId {
        "hostname"
    }

    fn display_name(&self) -> &str {
        "Hostname"
    }

    fn is_available(&self, _ctx: &ChipContext) -> bool {
        true
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        ChipOutput {
            id: self.id(),
            label: ctx.shell_context.hostname.clone(),
            ..ChipOutput::default()
        }
    }
}
