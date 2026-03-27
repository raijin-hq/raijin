//! Custom Inazuma Element that renders a terminal grid (per-cell ANSI colors).
//!
//! This is the inner content renderer for a single block. It handles:
//! - Per-cell foreground/background colors
//! - Wide character support
//! - Cursor rendering (for active blocks)
//! - TextRun merging for performance
//!
//! Layout and headers are handled by the parent div (block_element).

use std::sync::{Arc, Mutex};

use inazuma::{
    point, px, size, App, Bounds, Element, ElementId, Font, FontWeight,
    GlobalElementId, Hsla, InspectorElementId, IntoElement, LayoutId, Pixels, Point,
    SharedString, ShapedLine, Size, Style, TextAlign, Window, fill,
};
use raijin_term::grid::{Dimensions, Grid};
use raijin_term::index::Line;
use raijin_term::term::cell::Cell;
use raijin_term::term::color::Colors;

use super::constants::*;
use super::text_rendering::shape_grid_row;

/// Pre-painted state for the grid element.
pub struct GridPrepaint {
    pub lines: Vec<(Point<Pixels>, ShapedLine)>,
    pub backgrounds: Vec<(Bounds<Pixels>, Hsla)>,
    pub cursor_rect: Option<Bounds<Pixels>>,
    pub line_height: Pixels,
}

/// Custom element that renders terminal grid cells with ANSI colors.
///
/// Created per-block — each block has its own TerminalGridElement
/// that reads from the block's Grid<Cell>.
pub struct TerminalGridElement {
    /// Grid data to render (borrowed via Arc for thread safety).
    grid: Arc<Mutex<GridSnapshot>>,
    /// Whether to show cursor.
    show_cursor: bool,
    /// Whether cursor is beam style.
    cursor_beam: bool,
    /// Font for rendering.
    font: Font,
    /// Font size.
    font_size: f32,
}

/// Snapshot of grid data needed for rendering.
/// Taken while the Term lock is held, then rendered without the lock.
pub struct GridSnapshot {
    pub lines: Vec<Vec<CellSnapshot>>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub cols: usize,
}

/// Snapshot of a single cell's rendering data.
#[derive(Clone)]
pub struct CellSnapshot {
    pub c: char,
    pub fg: Hsla,
    pub bg: Hsla,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikeout: bool,
    pub wide: bool,
    pub wide_spacer: bool,
}

impl TerminalGridElement {
    pub fn new(grid: Arc<Mutex<GridSnapshot>>, font: Font, font_size: f32) -> Self {
        Self {
            grid,
            show_cursor: false,
            cursor_beam: true,
            font,
            font_size,
        }
    }

    pub fn with_cursor(mut self, show: bool, beam: bool) -> Self {
        self.show_cursor = show;
        self.cursor_beam = beam;
        self
    }
}

impl IntoElement for TerminalGridElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TerminalGridElement {
    type RequestLayoutState = ();
    type PrepaintState = GridPrepaint;

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
        _window: &mut Window,
        _cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style::default();
        let layout_id = _window.request_layout(style, None, _cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        let font_size = px(self.font_size);
        let font_id = window.text_system().resolve_font(&self.font);
        let cell_width = window
            .text_system()
            .advance(font_id, font_size, 'M')
            .unwrap_or_default()
            .width;
        let ascent = window.text_system().ascent(font_id, font_size);
        let descent = window.text_system().descent(font_id, font_size);
        let cell_height = ascent + descent.abs() + px(CELL_PADDING);
        let line_height = cell_height;

        let mut lines = Vec::new();
        let mut backgrounds = Vec::new();
        let mut cursor_rect = None;

        let bg_color = terminal_bg();
        let text_x = bounds.origin.x + px(BLOCK_HEADER_PAD_X);

        let Ok(snapshot) = self.grid.lock() else {
            return GridPrepaint { lines, backgrounds, cursor_rect, line_height };
        };

        let mut current_y = bounds.origin.y;

        for (row_idx, row) in snapshot.lines.iter().enumerate() {
            let mut line_text = String::with_capacity(snapshot.cols);
            let mut runs = Vec::new();
            let mut skip_next = false;

            for (col_idx, cell) in row.iter().enumerate() {
                if skip_next {
                    skip_next = false;
                    continue;
                }

                if cell.wide {
                    skip_next = true;
                }

                if cell.bg != bg_color {
                    let x = text_x + cell_width * col_idx as f32;
                    let width = if cell.wide { cell_width * 2.0 } else { cell_width };
                    backgrounds.push((
                        Bounds::new(point(x, current_y), size(width, cell_height)),
                        cell.bg,
                    ));
                }

                let ch = if cell.c == '\0' { ' ' } else { cell.c };
                let byte_start = line_text.len();
                line_text.push(ch);
                let byte_len = line_text.len() - byte_start;

                let font = Font {
                    family: self.font.family.clone(),
                    weight: if cell.bold { FontWeight::BOLD } else { FontWeight::NORMAL },
                    style: if cell.italic { inazuma::FontStyle::Italic } else { inazuma::FontStyle::Normal },
                    ..Font::default()
                };

                // Merge with previous run if same style
                let can_merge = runs.last().map_or(false, |last: &inazuma::TextRun| {
                    last.color == cell.fg
                        && last.font.weight == font.weight
                        && last.font.style == font.style
                });

                if can_merge {
                    runs.last_mut().unwrap().len += byte_len;
                } else {
                    runs.push(inazuma::TextRun {
                        len: byte_len,
                        font,
                        color: cell.fg,
                        background_color: None,
                        underline: if cell.underline {
                            Some(inazuma::UnderlineStyle {
                                thickness: px(1.0),
                                color: Some(cell.fg),
                                wavy: false,
                            })
                        } else {
                            None
                        },
                        strikethrough: if cell.strikeout {
                            Some(inazuma::StrikethroughStyle {
                                thickness: px(1.0),
                                color: Some(cell.fg),
                            })
                        } else {
                            None
                        },
                    });
                }
            }

            if !line_text.is_empty() {
                let shaped = window.text_system().shape_line(
                    SharedString::from(line_text),
                    font_size,
                    &runs,
                    None,
                );
                lines.push((point(text_x, current_y), shaped));
            }

            // Cursor
            if self.show_cursor && row_idx == snapshot.cursor_line {
                let cursor_x = text_x + cell_width * snapshot.cursor_col as f32;
                let cursor_width = if self.cursor_beam { px(2.0) } else { cell_width };
                cursor_rect = Some(Bounds::new(
                    point(cursor_x, current_y),
                    size(cursor_width, cell_height),
                ));
            }

            current_y += cell_height;
        }

        GridPrepaint { lines, backgrounds, cursor_rect, line_height }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        // Cell backgrounds
        for (rect, color) in &prepaint.backgrounds {
            window.paint_quad(fill(*rect, *color));
        }

        // Cursor
        if let Some(cursor_rect) = prepaint.cursor_rect {
            window.paint_quad(fill(cursor_rect, cursor_color()));
        }

        // Text
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
    }
}
