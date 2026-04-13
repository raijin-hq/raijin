use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for shell nesting level.
///
/// Reads `$SHLVL` and only shows when the level exceeds a threshold (default: 1).
/// This indicates the user is inside a nested shell (e.g., `bash` inside `zsh`).
///
/// The first shell session is typically SHLVL=1, so we only show at >= 2
/// to avoid clutter on normal sessions.
///
/// Display: shows the nesting depth with a nested-arrow icon.
pub struct ShlvlProvider;

/// Minimum SHLVL before the chip becomes visible.
const MIN_SHLVL: u32 = 2;

impl ChipProvider for ShlvlProvider {
    fn id(&self) -> ChipId {
        "shlvl"
    }

    fn display_name(&self) -> &str {
        "Shell Level"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        get_shlvl(ctx).is_some_and(|n| n >= MIN_SHLVL)
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let level = get_shlvl(ctx).unwrap_or(1);

        ChipOutput {
            id: self.id(),
            label: level.to_string(),
            icon: Some("CornerDownRight"),
            tooltip: Some(format!("Shell nesting level {level}")),
            ..ChipOutput::default()
        }
    }
}

fn get_shlvl(ctx: &ChipContext) -> Option<u32> {
    ctx.get_env("SHLVL").and_then(|v| v.parse::<u32>().ok())
}
