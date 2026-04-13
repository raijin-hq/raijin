use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the current username.
///
/// Always visible. Reads from `ShellContext::username`.
pub struct UsernameProvider;

impl ChipProvider for UsernameProvider {
    fn id(&self) -> ChipId {
        "username"
    }

    fn display_name(&self) -> &str {
        "Username"
    }

    fn is_available(&self, _ctx: &ChipContext) -> bool {
        true
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        ChipOutput {
            id: self.id(),
            label: ctx.shell_context.username.clone(),
            ..ChipOutput::default()
        }
    }
}
