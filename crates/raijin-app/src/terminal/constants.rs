//! Terminal rendering constants and theme colors.

use inazuma::{Hsla, hsla, rgb};

// ---------------------------------------------------------------------------
// Layout Constants
// ---------------------------------------------------------------------------

pub const FONT_SIZE: f32 = 14.0;
pub const CELL_PADDING: f32 = 2.0;

/// Base height for block headers (metadata + single-line command).
pub const BLOCK_HEADER_HEIGHT: f32 = 50.0;
pub const BLOCK_HEADER_PAD_X: f32 = 16.0;
pub const BLOCK_GAP: f32 = 0.0;
pub const BLOCK_LEFT_BORDER: f32 = 4.0;
pub const BLOCK_BODY_PAD_BOTTOM: f32 = 8.0;

pub const HEADER_META_FONT_SIZE: f32 = 11.0;
pub const HEADER_CMD_FONT_SIZE: f32 = 13.0;

/// Font families to try in order. First match wins.
pub const FONT_FAMILIES: &[&str] = &[
    "JetBrainsMono Nerd Font",
    "JetBrains Mono",
    "Menlo",
    "Monaco",
];

// ---------------------------------------------------------------------------
// Raijin Dark Theme
// ---------------------------------------------------------------------------

pub fn terminal_bg() -> Hsla {
    rgb(0x121212).into()
}

pub fn terminal_fg() -> Hsla {
    rgb(0xf1f1f1).into()
}

pub fn cursor_color() -> Hsla {
    rgb(0x14F195).into()
}

pub fn error_color() -> Hsla {
    rgb(0xff5f5f).into()
}

pub fn block_body_bg() -> Hsla {
    hsla(0.0, 0.0, 1.0, 0.04)
}

pub fn block_selected_bg() -> Hsla {
    hsla(153.0 / 360.0, 0.93, 0.51, 0.08)
}

pub fn block_header_error_bg() -> Hsla {
    hsla(0.0, 0.7, 0.45, 0.12)
}

pub fn header_command_fg() -> Hsla {
    rgb(0xf1f1f1).into()
}

pub fn header_metadata_fg() -> Hsla {
    hsla(0.0, 0.0, 1.0, 0.35)
}
