use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the current shell name.
///
/// Always visible. Interactive (clickable to switch shells).
/// Reads from `ChipContext::shell_name`.
pub struct ShellProvider;

impl ChipProvider for ShellProvider {
    fn id(&self) -> ChipId {
        "shell"
    }

    fn display_name(&self) -> &str {
        "Shell"
    }

    fn is_available(&self, _ctx: &ChipContext) -> bool {
        true
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        ChipOutput {
            id: self.id(),
            label: ctx.shell_name.clone(),
            interactive: true,
            ..ChipOutput::default()
        }
    }
}
