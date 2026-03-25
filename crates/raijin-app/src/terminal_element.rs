use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::Flags as CellFlags;
use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor};
use inazuma::{
    point, px, size, App, Bounds, Element, ElementId, Font, FontStyle, FontWeight,
    GlobalElementId, Hsla, InspectorElementId, IntoElement, LayoutId, Pixels, Point,
    SharedString, ShapedLine, Size, StrikethroughStyle, Style, TextAlign, TextRun,
    UnderlineStyle, Window, fill, hsla, rgb,
};
use raijin_terminal::TerminalHandle;

const FONT_SIZE: f32 = 14.0;
const CELL_PADDING: f32 = 2.0;

/// Font families to try in order. First match wins.
const FONT_FAMILIES: &[&str] = &[
    "JetBrainsMono Nerd Font",
    "JetBrains Mono",
    "Menlo",
    "Monaco",
];

/// Terminal grid dimensions computed from font metrics.
pub struct TerminalLayout {
    pub font: Font,
    pub font_size: Pixels,
    pub cell_width: Pixels,
    pub cell_height: Pixels,
    pub line_height: Pixels,
}

/// Pre-painted state: shaped text lines and background rects.
pub struct TerminalPrepaint {
    pub lines: Vec<(Point<Pixels>, ShapedLine)>,
    pub backgrounds: Vec<(Bounds<Pixels>, Hsla)>,
    pub cursor_rect: Option<Bounds<Pixels>>,
    pub line_height: Pixels,
}

/// Custom Inazuma element that renders the terminal grid.
///
/// Uses `TerminalHandle` to read grid state for rendering and
/// automatically resize the terminal when element bounds change.
pub struct TerminalElement {
    handle: TerminalHandle,
}

impl TerminalElement {
    pub fn new(handle: TerminalHandle) -> Self {
        Self { handle }
    }

    fn compute_layout(&self, window: &mut Window) -> TerminalLayout {
        let font_size = px(FONT_SIZE);

        // Try fonts in priority order
        let mut font = Font {
            family: FONT_FAMILIES[0].into(),
            weight: FontWeight::NORMAL,
            ..Font::default()
        };

        let font_id = window.text_system().resolve_font(&font);

        // Verify font resolved to something with the right family by checking advance
        // If the first font isn't installed, resolve_font still returns a fallback
        // We just use whatever it resolved to
        let cell_width = window
            .text_system()
            .advance(font_id, font_size, 'M')
            .expect("failed to get advance width for 'M'")
            .width;

        let ascent = window.text_system().ascent(font_id, font_size);
        let descent = window.text_system().descent(font_id, font_size);
        let cell_height = ascent + descent.abs() + px(CELL_PADDING);
        let line_height = cell_height;

        // Update font to match what was actually resolved
        if let Some(resolved) = window.text_system().get_font_for_id(font_id) {
            font = resolved;
        }

        // font_id is used only for metric computation above,
        // not stored — shape_line resolves fonts per-run via Font struct
        let _ = font_id;

        TerminalLayout {
            font,
            font_size,
            cell_width,
            cell_height,
            line_height,
        }
    }
}

impl IntoElement for TerminalElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TerminalElement {
    type RequestLayoutState = TerminalLayout;
    type PrepaintState = TerminalPrepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let layout = self.compute_layout(window);

        let style = Style {
            size: Size {
                width: inazuma::relative(1.0).into(),
                height: inazuma::relative(1.0).into(),
            },
            ..Style::default()
        };

        let layout_id = window.request_layout(style, [], cx);
        (layout_id, layout)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        // Resize terminal grid + PTY if bounds changed (Zed pattern: recalculate every frame)
        let new_cols = (bounds.size.width / layout.cell_width).floor() as u16;
        let new_rows = (bounds.size.height / layout.cell_height).floor() as u16;
        self.handle.set_size(new_rows, new_cols);

        let term = self.handle.lock();
        let content = term.renderable_content();

        let grid_rows = term.screen_lines();
        let grid_cols = term.columns();
        let colors = content.colors;
        let display_offset = content.display_offset;

        let bg_color = terminal_bg();

        let mut lines = Vec::with_capacity(grid_rows);
        let mut backgrounds = Vec::new();
        let mut cursor_rect = None;

        // Bottom-grow layout (Warp-style): only render rows with content,
        // positioned at the bottom of the container so output grows upward.
        let cursor = &content.cursor;
        let cursor_point = cursor.point;
        let cursor_visual_row = (cursor_point.line.0 as usize)
            .saturating_add(display_offset);

        // Content rows = everything from row 0 up to and including the cursor row
        let content_rows = (cursor_visual_row + 1).min(grid_rows);
        let content_height = layout.cell_height * content_rows as f32;

        // Y offset: push content to the bottom of the bounds
        let y_offset = bounds.size.height - content_height;

        if content.mode.contains(alacritty_terminal::term::TermMode::SHOW_CURSOR) {
            let cx_px = bounds.origin.x + layout.cell_width * cursor_point.column.0 as f32;
            let cy_px = bounds.origin.y + y_offset + layout.cell_height * cursor_visual_row as f32;
            cursor_rect = Some(Bounds::new(
                point(cx_px, cy_px),
                size(px(2.0), layout.cell_height),
            ));
        }

        // Only render rows with content (0..content_rows), skip empty rows below cursor
        let grid = term.grid();
        for row_idx in 0..content_rows {
            let mut line_text = String::with_capacity(grid_cols);
            let mut runs: Vec<TextRun> = Vec::new();
            let mut skip_next = false;

            let line = Line(row_idx as i32 - display_offset as i32);

            for col_idx in 0..grid_cols {
                // Skip wide char spacer cells
                if skip_next {
                    skip_next = false;
                    continue;
                }

                let cell = &grid[line][Column(col_idx)];
                let c = cell.c;
                let flags = cell.flags;

                // Mark next cell for skip if this is a wide character
                if flags.contains(CellFlags::WIDE_CHAR) {
                    skip_next = true;
                }

                let (mut fg, mut bg) = resolve_colors(cell, colors);

                if flags.contains(CellFlags::INVERSE) {
                    std::mem::swap(&mut fg, &mut bg);
                }

                // Background rect if non-default
                if bg != bg_color {
                    let x = bounds.origin.x + layout.cell_width * col_idx as f32;
                    let y = bounds.origin.y + y_offset + layout.cell_height * row_idx as f32;
                    let width = if flags.contains(CellFlags::WIDE_CHAR) {
                        layout.cell_width * 2.0
                    } else {
                        layout.cell_width
                    };
                    backgrounds.push((
                        Bounds::new(point(x, y), size(width, layout.cell_height)),
                        bg,
                    ));
                }

                let ch = if c == '\0' { ' ' } else { c };
                let byte_start = line_text.len();
                line_text.push(ch);
                let byte_len = line_text.len() - byte_start;

                let font_weight = if flags.contains(CellFlags::BOLD) {
                    FontWeight::BOLD
                } else {
                    FontWeight::NORMAL
                };

                let font = Font {
                    family: layout.font.family.clone(),
                    weight: font_weight,
                    style: if flags.contains(CellFlags::ITALIC) {
                        FontStyle::Italic
                    } else {
                        FontStyle::Normal
                    },
                    ..Font::default()
                };

                let underline = if flags.contains(CellFlags::UNDERLINE) {
                    Some(UnderlineStyle {
                        thickness: px(1.0),
                        color: Some(fg),
                        wavy: false,
                    })
                } else {
                    None
                };

                let strikethrough = if flags.contains(CellFlags::STRIKEOUT) {
                    Some(StrikethroughStyle {
                        thickness: px(1.0),
                        color: Some(fg),
                    })
                } else {
                    None
                };

                // Merge consecutive cells with identical styling into one TextRun
                if let Some(last) = runs.last_mut() {
                    let same_style = last.color == fg
                        && last.font.weight == font.weight
                        && last.font.style == font.style
                        && last.underline == underline
                        && last.strikethrough == strikethrough;

                    if same_style {
                        last.len += byte_len;
                        continue;
                    }
                }

                runs.push(TextRun {
                    len: byte_len,
                    font,
                    color: fg,
                    background_color: None,
                    underline,
                    strikethrough,
                });
            }

            if !line_text.is_empty() {
                let shaped = window.text_system().shape_line(
                    SharedString::from(line_text),
                    layout.font_size,
                    &runs,
                    None,
                );
                let origin = point(
                    bounds.origin.x,
                    bounds.origin.y + y_offset + layout.cell_height * row_idx as f32,
                );
                lines.push((origin, shaped));
            }
        }

        TerminalPrepaint {
            lines,
            backgrounds,
            cursor_rect,
            line_height: layout.line_height,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        // Clip all rendering to element bounds — prevents text bleeding outside
        let content_mask = inazuma::ContentMask { bounds };
        window.with_content_mask(Some(content_mask), |window| {
            // 1. Overall background
            window.paint_quad(fill(bounds, terminal_bg()));

            // 2. Cell backgrounds
            for (rect, color) in &prepaint.backgrounds {
                window.paint_quad(fill(*rect, *color));
            }

            // 3. Cursor (BEFORE text so text renders on top of block cursor)
            if let Some(cursor_rect) = prepaint.cursor_rect {
                window.paint_quad(fill(cursor_rect, cursor_color()));
            }

            // 4. Text
            for (origin, shaped_line) in &prepaint.lines {
                let _ = shaped_line.paint(
                    *origin,
                    prepaint.line_height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
            );
        }
        }); // end content_mask
    }
}

// --- Color helpers ---

fn terminal_bg() -> Hsla {
    rgb(0x1a1b26).into()
}

fn terminal_fg() -> Hsla {
    rgb(0xc0caf5).into()
}

fn cursor_color() -> Hsla {
    // Warp-style green beam cursor
    rgb(0x9ece6a).into()
}

fn resolve_colors(
    cell: &alacritty_terminal::term::cell::Cell,
    colors: &alacritty_terminal::term::color::Colors,
) -> (Hsla, Hsla) {
    let fg = ansi_color_to_hsla(&cell.fg, colors, cell.flags.contains(CellFlags::DIM));
    let bg = ansi_color_to_hsla(&cell.bg, colors, false);
    (fg, bg)
}

fn ansi_color_to_hsla(
    color: &AnsiColor,
    colors: &alacritty_terminal::term::color::Colors,
    dim: bool,
) -> Hsla {
    match color {
        AnsiColor::Named(name) => named_color_to_hsla(*name, dim),
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
                )
            } else {
                let mut c = indexed_256_to_hsla(*idx);
                if dim {
                    c.l *= 0.66;
                }
                c
            }
        }
    }
}

fn named_color_to_hsla(name: NamedColor, dim: bool) -> Hsla {
    let (r, g, b) = match name {
        // Tokyo Night palette
        NamedColor::Black => (0x15, 0x16, 0x1e),
        NamedColor::Red => (0xf7, 0x76, 0x8e),
        NamedColor::Green => (0x9e, 0xce, 0x6a),
        NamedColor::Yellow => (0xe0, 0xaf, 0x68),
        NamedColor::Blue => (0x7a, 0xa2, 0xf7),
        NamedColor::Magenta => (0xbb, 0x9a, 0xf7),
        NamedColor::Cyan => (0x7d, 0xcf, 0xff),
        NamedColor::White => (0xa9, 0xb1, 0xd6),
        NamedColor::BrightBlack => (0x41, 0x48, 0x68),
        NamedColor::BrightRed => (0xf7, 0x76, 0x8e),
        NamedColor::BrightGreen => (0x9e, 0xce, 0x6a),
        NamedColor::BrightYellow => (0xe0, 0xaf, 0x68),
        NamedColor::BrightBlue => (0x7a, 0xa2, 0xf7),
        NamedColor::BrightMagenta => (0xbb, 0x9a, 0xf7),
        NamedColor::BrightCyan => (0x7d, 0xcf, 0xff),
        NamedColor::BrightWhite => (0xc0, 0xca, 0xf5),
        NamedColor::Foreground => (0xc0, 0xca, 0xf5),
        NamedColor::Background => (0x1a, 0x1b, 0x26),
        NamedColor::Cursor => (0x7d, 0xcf, 0xff),
        _ => (0xc0, 0xca, 0xf5),
    };
    let mut c = rgb_to_hsla(r, g, b);
    if dim {
        c.l *= 0.66;
    }
    c
}

fn indexed_256_to_hsla(idx: u8) -> Hsla {
    if idx < 16 {
        return terminal_fg();
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

fn rgb_to_hsla(r: u8, g: u8, b: u8) -> Hsla {
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
