//! Stub terminal element — will be replaced in Phase 20.

use inazuma::{App, Oklch, TextStyle};
use raijin_theme::Theme;

/// Stub terminal element. Real implementation lives in raijin-app/src/terminal_element.rs.
pub struct TerminalElement;

impl TerminalElement {
    /// Layout a terminal grid — stub that returns empty layout.
    pub fn layout_grid(
        _grid: &alacritty_terminal::Grid<alacritty_terminal::Term>,
        _scroll_top: usize,
        _text_style: &TextStyle,
        _terminal_theme: Option<&raijin_theme::ThemeColors>,
        _minimum_contrast: f32,
        _cx: &App,
    ) -> Vec<()> {
        Vec::new()
    }
}

/// Convert an ANSI terminal color to Oklch using the active theme.
pub fn convert_color(
    _fg: &raijin_terminal::alacritty_terminal::vte::ansi::Color,
    _theme: &Theme,
) -> Oklch {
    Oklch { l: 0.8, c: 0.0, h: 0.0, a: 1.0 }
}
