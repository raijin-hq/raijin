//! Terminal rendering constants and theme color accessors.
//!
//! Layout constants are plain values. All colors are read from
//! `ResolvedTheme` — never hardcoded.

use inazuma::Hsla;
use raijin_settings::ResolvedTheme;

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

pub fn terminal_bg(t: &ResolvedTheme) -> Hsla { t.background }
pub fn accent_color(t: &ResolvedTheme) -> Hsla { t.accent }
pub fn error_color(t: &ResolvedTheme) -> Hsla { t.error }
pub fn block_body_bg(t: &ResolvedTheme) -> Hsla { t.block_bg }
pub fn block_selected_bg(t: &ResolvedTheme) -> Hsla { t.selected_bg }
pub fn header_command_fg(t: &ResolvedTheme) -> Hsla { t.command_fg }
pub fn header_metadata_fg(t: &ResolvedTheme) -> Hsla { t.metadata_fg }
// Fold system colors
pub fn fold_line_bg(t: &ResolvedTheme) -> Hsla {
    let mut bg = t.block_bg;
    bg.a = (bg.a + 0.05).min(1.0);
    bg
}
pub fn fold_line_error_bg(t: &ResolvedTheme) -> Hsla {
    let mut c = t.error;
    c.a = 0.10;
    c
}
pub fn fold_line_hover_bg(t: &ResolvedTheme) -> Hsla {
    let mut c = t.accent;
    c.a = 0.10;
    c
}
pub fn fold_badge_success(t: &ResolvedTheme) -> Hsla { t.accent }
pub fn fold_badge_error(t: &ResolvedTheme) -> Hsla { t.error }
pub fn fold_badge_running(_t: &ResolvedTheme) -> Hsla {
    // Warm yellow for running indicator
    Hsla { h: 45.0 / 360.0, s: 0.9, l: 0.65, a: 1.0 }
}
