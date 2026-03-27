//! Custom Inazuma Element that renders a terminal grid (per-cell ANSI colors).
//!
//! Renders from a pre-extracted BlockGridSnapshot — no mutex locking here.
//! The snapshot is created in block_list.rs with a single lock for all blocks.

use inazuma::{
    point, px, relative, size, App, Bounds, Element, ElementId, Font, FontWeight,
    GlobalElementId, Hsla, InspectorElementId, IntoElement, LayoutId, Pixels, Point,
    SharedString, ShapedLine, Style, TextAlign, Window, fill,
};

use super::constants::*;
use super::grid_snapshot::BlockGridSnapshot;

/// Pre-painted state for the grid element.
pub struct GridPrepaint {
    pub lines: Vec<(Point<Pixels>, ShapedLine)>,
    pub backgrounds: Vec<(Bounds<Pixels>, Hsla)>,
    pub line_height: Pixels,
}

/// Custom element that renders a block's terminal grid from a snapshot.
///
/// No mutex locking happens here — all data comes from the snapshot
/// which was extracted with a single lock in block_list.rs.
pub struct TerminalGridElement {
    snapshot: BlockGridSnapshot,
    font: Font,
    font_size: f32,
}

impl TerminalGridElement {
    pub fn new(snapshot: BlockGridSnapshot, font: Font, font_size: f32) -> Self {
        Self {
            snapshot,
            font,
            font_size,
        }
    }

    /// Compute cell dimensions from font metrics.
    fn cell_dimensions(&self, window: &mut Window) -> (Pixels, Pixels) {
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
        (cell_width, cell_height)
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
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let (_cell_width, cell_height) = self.cell_dimensions(window);
        let height = cell_height * self.snapshot.content_rows as f32;

        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = height.into();

        let layout_id = window.request_layout(style, None, cx);
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
        let (cell_width, cell_height) = self.cell_dimensions(window);
        let font_size = px(self.font_size);
        let line_height = cell_height;

        // Viewport-Culling: skip entire block if completely outside visible area
        let viewport = window.content_mask().bounds;
        let block_bottom = bounds.origin.y + cell_height * self.snapshot.content_rows as f32;
        if block_bottom < viewport.origin.y || bounds.origin.y > viewport.origin.y + viewport.size.height {
            return GridPrepaint { lines: vec![], backgrounds: vec![], line_height };
        }

        let bg_color = terminal_bg();
        let text_x = bounds.origin.x + px(BLOCK_HEADER_PAD_X);
        let mut current_y = bounds.origin.y;

        let mut lines = Vec::new();
        let mut backgrounds = Vec::new();

        // Per-line viewport culling: skip lines above/below visible area
        let viewport_top = viewport.origin.y;
        let viewport_bottom = viewport.origin.y + viewport.size.height;

        for snap_line in self.snapshot.lines.iter() {
            let line_bottom = current_y + cell_height;

            // Skip lines completely above viewport
            if line_bottom < viewport_top {
                current_y += cell_height;
                continue;
            }
            // Stop rendering once we're below viewport
            if current_y > viewport_bottom {
                break;
            }

            if snap_line.cells.is_empty() {
                current_y += cell_height;
                continue;
            }

            let mut line_text = String::with_capacity(self.snapshot.grid_cols);
            let mut runs = Vec::new();
            let mut col_x = 0usize;

            for cell in &snap_line.cells {
                // Background
                if cell.bg != bg_color {
                    let x = text_x + cell_width * col_x as f32;
                    let width = if cell.wide { cell_width * 2.0 } else { cell_width };
                    backgrounds.push((
                        Bounds::new(point(x, current_y), size(width, cell_height)),
                        cell.bg,
                    ));
                }

                // Text
                let byte_start = line_text.len();
                line_text.push(cell.c);
                for zw in &cell.zerowidth {
                    line_text.push(*zw);
                }
                let byte_len = line_text.len() - byte_start;

                let family = match &cell.font_family_override {
                    Some(override_family) => override_family.clone().into(),
                    None => self.font.family.clone(),
                };
                let font = Font {
                    family,
                    weight: if cell.bold { FontWeight::BOLD } else { FontWeight::NORMAL },
                    style: if cell.italic {
                        inazuma::FontStyle::Italic
                    } else {
                        inazuma::FontStyle::Normal
                    },
                    ..Font::default()
                };

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

                col_x += if cell.wide { 2 } else { 1 };
            }

            if !line_text.is_empty() {
                let text_hash = {
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::hash::DefaultHasher::new();
                    line_text.hash(&mut hasher);
                    hasher.finish()
                };
                let text_len = line_text.len();
                let shaped = window.text_system().shape_line_by_hash(
                    text_hash,
                    text_len,
                    font_size,
                    &runs,
                    None,
                    || SharedString::from(line_text),
                );
                lines.push((point(text_x, current_y), shaped));
            }

            current_y += cell_height;
        }

        GridPrepaint { lines, backgrounds, line_height }
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
        for (rect, color) in &prepaint.backgrounds {
            window.paint_quad(fill(*rect, *color));
        }

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
