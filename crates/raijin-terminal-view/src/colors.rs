//! ANSI color resolution.
//!
//! Named colors (Background, Foreground, Cursor) and the 16 ANSI palette
//! colors are read from `raijin_theme::Theme` via `GlobalTheme`.

use inazuma::Oklch;
use raijin_theme::Theme;
use raijin_term::term::cell::Flags as CellFlags;
use raijin_term::vte::ansi::{Color as AnsiColor, NamedColor};

/// Convert an ANSI color to a theme-aware Oklch color.
///
/// Maps Named, Spec (true color), and Indexed (256) ANSI colors to Oklch
/// using the active theme's terminal palette. Used by debugger console and
/// other non-terminal UI that needs ANSI→theme color mapping.
pub fn convert_color(color: &AnsiColor, theme: &Theme) -> Oklch {
    ansi_color_to_oklch(color, &Default::default(), false, theme)
}

/// Resolve foreground and background colors for a terminal cell.
pub fn resolve_colors(
    cell: &raijin_term::term::cell::Cell,
    colors: &raijin_term::term::color::Colors,
    theme: &Theme,
) -> (Oklch, Oklch) {
    let fg = ansi_color_to_oklch(&cell.fg, colors, cell.flags.contains(CellFlags::DIM), theme);
    let bg = ansi_color_to_oklch(&cell.bg, colors, false, theme);
    (fg, bg)
}

fn ansi_color_to_oklch(
    color: &AnsiColor,
    colors: &raijin_term::term::color::Colors,
    dim: bool,
    theme: &Theme,
) -> Oklch {
    match color {
        AnsiColor::Named(name) => named_color_to_oklch(*name, dim, theme),
        AnsiColor::Spec(rgb) => {
            let mut c = rgb_to_oklch(rgb.r, rgb.g, rgb.b);
            if dim {
                c.l *= 0.66;
            }
            c
        }
        AnsiColor::Indexed(idx) => {
            if let Some(rgb) = colors[*idx as usize] {
                let mut c = rgb_to_oklch(rgb.r, rgb.g, rgb.b);
                if dim {
                    c.l *= 0.66;
                }
                c
            } else if *idx < 16 {
                named_color_to_oklch(
                    match idx {
                        0 => NamedColor::Black,
                        1 => NamedColor::Red,
                        2 => NamedColor::Green,
                        3 => NamedColor::Yellow,
                        4 => NamedColor::Blue,
                        5 => NamedColor::Magenta,
                        6 => NamedColor::Cyan,
                        7 => NamedColor::White,
                        8 => NamedColor::BrightBlack,
                        9 => NamedColor::BrightRed,
                        10 => NamedColor::BrightGreen,
                        11 => NamedColor::BrightYellow,
                        12 => NamedColor::BrightBlue,
                        13 => NamedColor::BrightMagenta,
                        14 => NamedColor::BrightCyan,
                        15 => NamedColor::BrightWhite,
                        _ => NamedColor::White,
                    },
                    dim,
                    theme,
                )
            } else {
                let mut c = indexed_256_to_oklch(*idx, theme);
                if dim {
                    c.l *= 0.66;
                }
                c
            }
        }
    }
}

fn named_color_to_oklch(name: NamedColor, dim: bool, theme: &Theme) -> Oklch {
    let tc = &theme.styles.colors;
    let c = match name {
        // Theme-driven special colors
        NamedColor::Background => return tc.terminal.background,
        NamedColor::Foreground => return tc.terminal.foreground,
        NamedColor::Cursor => return tc.text_accent,

        // Normal ANSI 8 — from theme
        NamedColor::Black => tc.terminal.ansi.black,
        NamedColor::Red => tc.terminal.ansi.red,
        NamedColor::Green => tc.terminal.ansi.green,
        NamedColor::Yellow => tc.terminal.ansi.yellow,
        NamedColor::Blue => tc.terminal.ansi.blue,
        NamedColor::Magenta => tc.terminal.ansi.magenta,
        NamedColor::Cyan => tc.terminal.ansi.cyan,
        NamedColor::White => tc.terminal.ansi.white,

        // Bright ANSI 8 — from theme
        NamedColor::BrightBlack => tc.terminal.ansi.bright_black,
        NamedColor::BrightRed => tc.terminal.ansi.bright_red,
        NamedColor::BrightGreen => tc.terminal.ansi.bright_green,
        NamedColor::BrightYellow => tc.terminal.ansi.bright_yellow,
        NamedColor::BrightBlue => tc.terminal.ansi.bright_blue,
        NamedColor::BrightMagenta => tc.terminal.ansi.bright_magenta,
        NamedColor::BrightCyan => tc.terminal.ansi.bright_cyan,
        NamedColor::BrightWhite => tc.terminal.ansi.bright_white,

        _ => tc.terminal.foreground,
    };
    if dim {
        let mut c = c;
        c.l *= 0.66;
        c
    } else {
        c
    }
}

fn indexed_256_to_oklch(idx: u8, theme: &Theme) -> Oklch {
    if idx < 16 {
        return theme.styles.colors.terminal.foreground;
    }

    if idx < 232 {
        let idx = idx - 16;
        let r = (idx / 36) * 51;
        let g = ((idx % 36) / 6) * 51;
        let b = (idx % 6) * 51;
        return rgb_to_oklch(r, g, b);
    }

    let gray = 8 + (idx - 232) * 10;
    rgb_to_oklch(gray, gray, gray)
}

/// Convert sRGB bytes to Oklch via inazuma's Rgba -> Oklch conversion.
pub fn rgb_to_oklch(r: u8, g: u8, b: u8) -> Oklch {
    let rgba = inazuma::Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    };
    Oklch::from(rgba)
}
