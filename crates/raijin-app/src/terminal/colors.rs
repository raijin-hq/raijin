//! ANSI color resolution.
//!
//! Named colors (Background, Foreground, Cursor) are read from the
//! resolved theme. The 16 ANSI palette colors are still hardcoded
//! to the Raijin Dark palette for now — they will move to the theme
//! once terminal_colors parsing is wired up.

use inazuma::{Hsla, hsla};
use raijin_settings::ResolvedTheme;
use raijin_term::term::cell::Flags as CellFlags;
use raijin_term::vte::ansi::{Color as AnsiColor, NamedColor};

/// Resolve foreground and background colors for a terminal cell.
pub fn resolve_colors(
    cell: &raijin_term::term::cell::Cell,
    colors: &raijin_term::term::color::Colors,
    theme: &ResolvedTheme,
) -> (Hsla, Hsla) {
    let fg = ansi_color_to_hsla(&cell.fg, colors, cell.flags.contains(CellFlags::DIM), theme);
    let bg = ansi_color_to_hsla(&cell.bg, colors, false, theme);
    (fg, bg)
}

fn ansi_color_to_hsla(
    color: &AnsiColor,
    colors: &raijin_term::term::color::Colors,
    dim: bool,
    theme: &ResolvedTheme,
) -> Hsla {
    match color {
        AnsiColor::Named(name) => named_color_to_hsla(*name, dim, theme),
        AnsiColor::Spec(rgb) => {
            let mut c = rgb_to_hsla(rgb.r, rgb.g, rgb.b);
            if dim {
                c.l *= 0.66;
            }
            c
        }
        AnsiColor::Indexed(idx) => {
            if let Some(rgb) = colors[*idx as usize] {
                let mut c = rgb_to_hsla(rgb.r, rgb.g, rgb.b);
                if dim {
                    c.l *= 0.66;
                }
                c
            } else if *idx < 16 {
                named_color_to_hsla(
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
                let mut c = indexed_256_to_hsla(*idx, theme);
                if dim {
                    c.l *= 0.66;
                }
                c
            }
        }
    }
}

fn named_color_to_hsla(name: NamedColor, dim: bool, theme: &ResolvedTheme) -> Hsla {
    let c = match name {
        // Theme-driven: these three come from the resolved theme
        NamedColor::Background => return theme.background,
        NamedColor::Foreground => return theme.foreground,
        NamedColor::Cursor => return theme.accent,

        // ANSI 16 palette (hardcoded for now, will come from theme.terminal_colors)
        NamedColor::Black => rgb_to_hsla(0x12, 0x12, 0x12),
        NamedColor::Red => rgb_to_hsla(0xff, 0x5f, 0x5f),
        NamedColor::Green => rgb_to_hsla(0x14, 0xF1, 0x95),
        NamedColor::Yellow => rgb_to_hsla(0xff, 0xd7, 0x00),
        NamedColor::Blue => rgb_to_hsla(0x5f, 0x87, 0xff),
        NamedColor::Magenta => rgb_to_hsla(0xd7, 0x5f, 0xff),
        NamedColor::Cyan => rgb_to_hsla(0x00, 0xd7, 0xaf),
        NamedColor::White => rgb_to_hsla(0xf1, 0xf1, 0xf1),
        NamedColor::BrightBlack => rgb_to_hsla(0x66, 0x66, 0x66),
        NamedColor::BrightRed => rgb_to_hsla(0xff, 0x5f, 0x5f),
        NamedColor::BrightGreen => rgb_to_hsla(0x00, 0xff, 0x87),
        NamedColor::BrightYellow => rgb_to_hsla(0xff, 0xff, 0x00),
        NamedColor::BrightBlue => rgb_to_hsla(0x5c, 0x78, 0xff),
        NamedColor::BrightMagenta => rgb_to_hsla(0xca, 0x1f, 0x7b),
        NamedColor::BrightCyan => rgb_to_hsla(0x00, 0xd7, 0xff),
        NamedColor::BrightWhite => rgb_to_hsla(0xff, 0xff, 0xff),
        _ => theme.foreground,
    };
    if dim {
        let mut c = c;
        c.l *= 0.66;
        c
    } else {
        c
    }
}

fn indexed_256_to_hsla(idx: u8, theme: &ResolvedTheme) -> Hsla {
    if idx < 16 {
        return theme.foreground;
    }

    if idx < 232 {
        let idx = idx - 16;
        let r = (idx / 36) * 51;
        let g = ((idx % 36) / 6) * 51;
        let b = (idx % 6) * 51;
        return rgb_to_hsla(r, g, b);
    }

    let gray = 8 + (idx - 232) * 10;
    rgb_to_hsla(gray, gray, gray)
}

pub fn rgb_to_hsla(r: u8, g: u8, b: u8) -> Hsla {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f32::EPSILON {
        return hsla(0.0, 0.0, l, 1.0);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - r).abs() < f32::EPSILON {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < f32::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };

    hsla(h / 6.0, s, l, 1.0)
}
