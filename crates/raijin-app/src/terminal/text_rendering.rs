//! Grid row → ShapedLine conversion for terminal rendering.

use inazuma::{
    point, px, Bounds, Font, FontStyle, FontWeight, Hsla, Pixels, Point,
    SharedString, ShapedLine, StrikethroughStyle, TextRun, UnderlineStyle, Window,
    size,
};
use raijin_term::grid::{Dimensions, Grid};
use raijin_term::index::{Column, Line};
use raijin_term::term::cell::{Cell, Flags as CellFlags};
use raijin_term::term::color::Colors;

use super::colors::resolve_colors;
// Constants are passed as parameters, not imported directly.

/// Shape a single grid row into text + runs for rendering.
///
/// Returns the shaped line and any background rects for non-default colors.
pub fn shape_grid_row(
    grid: &Grid<Cell>,
    line: Line,
    colors: &Colors,
    text_x: Pixels,
    row_y: Pixels,
    layout_font: &Font,
    font_size: Pixels,
    cell_width: Pixels,
    cell_height: Pixels,
    bg_color: Hsla,
    window: &mut Window,
) -> (Option<(Point<Pixels>, ShapedLine)>, Vec<(Bounds<Pixels>, Hsla)>) {
    let grid_cols = grid.columns();
    let mut line_text = String::with_capacity(grid_cols);
    let mut runs: Vec<TextRun> = Vec::new();
    let mut backgrounds = Vec::new();
    let mut skip_next = false;

    for col_idx in 0..grid_cols {
        if skip_next {
            skip_next = false;
            continue;
        }

        let cell = &grid[line][Column(col_idx)];
        let c = cell.c;
        let flags = cell.flags;

        if flags.contains(CellFlags::WIDE_CHAR) {
            skip_next = true;
        }

        let (mut fg, mut bg) = resolve_colors(cell, colors);

        if flags.contains(CellFlags::INVERSE) {
            std::mem::swap(&mut fg, &mut bg);
        }

        if bg != bg_color {
            let x = text_x + cell_width * col_idx as f32;
            let width = if flags.contains(CellFlags::WIDE_CHAR) {
                cell_width * 2.0
            } else {
                cell_width
            };
            backgrounds.push((
                Bounds::new(point(x, row_y), size(width, cell_height)),
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
            family: layout_font.family.clone(),
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

    if line_text.is_empty() {
        return (None, backgrounds);
    }

    let shaped = window.text_system().shape_line(
        SharedString::from(line_text),
        font_size,
        &runs,
        None,
    );
    let origin = point(text_x, row_y);

    (Some((origin, shaped)), backgrounds)
}
