/// Default font family for terminal rendering.
/// DankMono Nerd Font Mono is bundled with the app — always available on all platforms.
pub const FONT_FAMILY: &str = "DankMono Nerd Font Mono";

/// Default font size in points.
pub const FONT_SIZE: f64 = 15.0;

/// Default line height multiplier (applied to cell height).
pub const LINE_HEIGHT: f64 = 1.2;

/// Default scrollback history lines.
pub const SCROLLBACK_HISTORY: u32 = 10_000;

/// Default theme name.
pub const THEME: &str = "raijin-dark";

// ---------------------------------------------------------------------------
// Theme color defaults (Raijin Dark)
// ---------------------------------------------------------------------------

pub fn theme_accent() -> String { "#14F195".to_string() }
pub fn theme_background() -> String { "#121212".to_string() }
pub fn theme_foreground() -> String { "#f1f1f1".to_string() }
pub fn theme_error() -> String { "#ff5f5f".to_string() }
pub fn theme_metadata_fg() -> String { "hsla(0, 0%, 100%, 0.35)".to_string() }
pub fn theme_selected_bg() -> String { "hsla(153, 93%, 51%, 0.08)".to_string() }
pub fn theme_sticky_hover_bg() -> String { "hsla(153, 40%, 15%, 0.90)".to_string() }
