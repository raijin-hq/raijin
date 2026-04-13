use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Minimum duration in ms to show the chip (default: 2 seconds).
const MIN_DURATION_MS: u64 = 2000;

pub struct CmdDurationProvider;

impl ChipProvider for CmdDurationProvider {
    fn id(&self) -> ChipId { "cmd_duration" }
    fn display_name(&self) -> &str { "Command Duration" }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.last_duration_ms
            .map(|ms| ms >= MIN_DURATION_MS)
            .unwrap_or(false)
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let ms = ctx.last_duration_ms.unwrap_or(0);
        let label = inazuma_util::time::render_time(ms as u128, false);

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Timer"),
            tooltip: Some(format!("{}ms", ms)),
            ..ChipOutput::default()
        }
    }
}
