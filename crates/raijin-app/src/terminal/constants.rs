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
pub fn sticky_header_hover_bg(t: &ResolvedTheme) -> Hsla { t.sticky_hover_bg }
