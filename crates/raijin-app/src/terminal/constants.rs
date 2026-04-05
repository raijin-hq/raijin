//! Terminal rendering constants and theme color accessors.
//!
//! Layout constants are plain values. All colors are read from
//! `raijin_theme::Theme` via `GlobalTheme` — never hardcoded.

use inazuma::Oklch;
use raijin_theme::Theme;

// ---------------------------------------------------------------------------
// Layout Constants
// ---------------------------------------------------------------------------

pub const BLOCK_HEADER_PAD_X: f32 = 16.0;
pub const BLOCK_LEFT_BORDER: f32 = 4.0;
pub const BLOCK_BODY_PAD_BOTTOM: f32 = 1.0;

// Fold system
pub const FOLD_LINE_HEIGHT: f32 = 22.0;
pub const FOLD_MAX_VISIBLE: usize = 3;
pub const FOLD_COUNTER_HEIGHT: f32 = 20.0;

// ---------------------------------------------------------------------------
// Theme color accessors
// ---------------------------------------------------------------------------

pub fn terminal_bg(t: &Theme) -> Oklch { t.styles.colors.terminal_background }
pub fn accent_color(t: &Theme) -> Oklch { t.styles.colors.terminal_accent }
pub fn error_color(t: &Theme) -> Oklch { t.styles.status.error.color }
pub fn block_selected_bg(t: &Theme) -> Oklch { t.styles.colors.terminal_accent.opacity(0.08) }
pub fn header_command_fg(t: &Theme) -> Oklch { t.styles.colors.text }
pub fn header_metadata_fg(t: &Theme) -> Oklch { t.styles.colors.text.opacity(0.35) }
// Fold system colors
pub fn fold_line_error_bg(t: &Theme) -> Oklch {
    let mut c = t.styles.status.error.color;
    c.a = 0.10;
    c
}
pub fn fold_line_hover_bg(t: &Theme) -> Oklch {
    t.styles.colors.terminal_accent.opacity(0.10)
}
pub fn fold_badge_success(t: &Theme) -> Oklch { t.styles.colors.block_success_badge }
pub fn fold_badge_error(t: &Theme) -> Oklch { t.styles.colors.block_error_badge }
pub fn fold_badge_running(t: &Theme) -> Oklch { t.styles.colors.block_running_badge }
