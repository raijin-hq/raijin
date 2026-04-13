use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the current working directory.
///
/// Always visible. Reads from `ShellContext::cwd_short` (tilde-shortened path).
pub struct DirectoryProvider;

impl ChipProvider for DirectoryProvider {
    fn id(&self) -> ChipId {
        "directory"
    }

    fn display_name(&self) -> &str {
        "Directory"
    }

    fn is_available(&self, _ctx: &ChipContext) -> bool {
        true
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        ChipOutput {
            id: self.id(),
            label: ctx.shell_context.cwd_short.clone(),
            icon: Some("Folder"),
            ..ChipOutput::default()
        }
    }
}
