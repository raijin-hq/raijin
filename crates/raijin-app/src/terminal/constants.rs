//! Terminal rendering constants and theme colors.

use inazuma::{Hsla, hsla, rgb};

// ---------------------------------------------------------------------------
// Layout Constants
// ---------------------------------------------------------------------------

pub const CELL_PADDING: f32 = 4.0;
pub const BLOCK_HEADER_PAD_X: f32 = 16.0;
pub const BLOCK_LEFT_BORDER: f32 = 4.0;
pub const BLOCK_BODY_PAD_BOTTOM: f32 = 1.0;

// ---------------------------------------------------------------------------
// Raijin Dark Theme
// ---------------------------------------------------------------------------

pub fn terminal_bg() -> Hsla {
    rgb(0x121212).into()
}

pub fn terminal_fg() -> Hsla {
    rgb(0xf1f1f1).into()
}

pub fn error_color() -> Hsla {
    rgb(0xff5f5f).into()
}

pub fn block_body_bg() -> Hsla {
    hsla(0.0, 0.0, 0.0, 0.0)
}

pub fn block_selected_bg() -> Hsla {
    hsla(153.0 / 360.0, 0.93, 0.51, 0.08)
}

pub fn header_command_fg() -> Hsla {
    rgb(0xf1f1f1).into()
}

pub fn header_metadata_fg() -> Hsla {
    hsla(0.0, 0.0, 1.0, 0.35)
}
