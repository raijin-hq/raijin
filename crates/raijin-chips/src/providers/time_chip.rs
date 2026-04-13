use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the current time.
///
/// Always visible. Reads from `ChipContext::time_str` (formatted HH:MM).
pub struct TimeProvider;

impl ChipProvider for TimeProvider {
    fn id(&self) -> ChipId {
        "time"
    }

    fn display_name(&self) -> &str {
        "Time"
    }

    fn is_available(&self, _ctx: &ChipContext) -> bool {
        true
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        ChipOutput {
            id: self.id(),
            label: ctx.time_str.clone(),
            icon: Some("CountdownTimer"),
            ..ChipOutput::default()
        }
    }
}
