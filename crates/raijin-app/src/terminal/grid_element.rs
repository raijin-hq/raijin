//! Per-cell terminal grid renderer.
//!
//! Renders terminal output cell-by-cell at exact grid positions, like
//! Rio, Alacritty, Kitty, and Ghostty. Each character is positioned at
//! `col * cell_width`, ensuring pixel-perfect monospace alignment regardless
//! of font metrics, emoji width, or special characters.
//!
//! Box-drawing characters (U+2500-U+259F) are rendered as GPU primitives
//! via `builtin_font.rs`. All other characters use `paint_glyph` / `paint_emoji`
//! from Inazuma's text system with CoreText font fallback.

use inazuma::{
    point, px, relative, size, App, Bounds, Element, ElementId, Font, FontWeight,
    FontId, GlyphId, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Oklch, Pixels, Point, SharedString, Style, Window, fill,
};

use std::sync::Arc;

use super::builtin_font::{self, BuiltinChar};
use super::constants::*;
use super::grid_snapshot::BlockGridSnapshot;

// ── Prepaint data ─────────────────────────────────────────────────────────────

/// A single cell glyph to paint at an exact grid position.
struct CellGlyph {
    origin: Point<Pixels>,
    font_id: FontId,
    glyph_id: GlyphId,
    color: Oklch,
    is_emoji: bool,
}

/// A built-in glyph to render as GPU primitives.
struct BuiltinGlyph {
    bc: BuiltinChar,
    x: Pixels,
    y: Pixels,
    color: Oklch,
}

/// Pre-painted state for the grid element.
pub struct GridPrepaint {
    backgrounds: Vec<(Bounds<Pixels>, Oklch)>,
    selections: Vec<(Bounds<Pixels>, Oklch)>,
    glyphs: Vec<CellGlyph>,
    builtins: Vec<BuiltinGlyph>,
    font_size: Pixels,
    cell_width: Pixels,
    cell_height: Pixels,
}

// ── Element ───────────────────────────────────────────────────────────────────

/// Custom element that renders a block's terminal grid from a snapshot.
///
/// No mutex locking happens here — all data comes from the snapshot
/// which was extracted with a single lock in block_list.rs.
/// Shared storage for the grid element's actual rendered origin Y coordinate.
/// Written during prepaint, read by hit_test in block_list.rs for exact
/// pixel→row mapping without sub-pixel rounding drift.
pub type GridOriginStore = std::rc::Rc<std::cell::Cell<Option<Pixels>>>;

pub struct TerminalGridElement {
    snapshot: Arc<BlockGridSnapshot>,
    selection: Option<raijin_term::selection::SelectionRange>,
    font: Font,
    font_size: f32,
    line_height_multiplier: f32,
    /// Terminal background color — cells with this bg skip painting.
    terminal_bg: Oklch,
    /// Selection highlight color (terminal_accent @ 45% alpha).
    selection_color: Oklch,
    /// If set, the actual bounds.origin.y is written here during prepaint.
    origin_store: Option<GridOriginStore>,
}

impl TerminalGridElement {
    pub fn new(
        snapshot: Arc<BlockGridSnapshot>,
        selection: Option<raijin_term::selection::SelectionRange>,
        font: Font,
        font_size: f32,
        line_height_multiplier: f32,
        terminal_bg: Oklch,
        selection_color: Oklch,
    ) -> Self {
        Self {
            snapshot,
            selection,
            font,
            font_size,
            line_height_multiplier,
            terminal_bg,
            selection_color,
            origin_store: None,
        }
    }

    /// Attach a store to capture the grid's actual origin Y during prepaint.
    pub fn with_origin_store(mut self, store: GridOriginStore) -> Self {
        self.origin_store = Some(store);
        self
    }

    /// Compute cell dimensions from font metrics.
    fn cell_dimensions(&self, window: &mut Window) -> (Pixels, Pixels) {
        let font_size = px(self.font_size);
        let font_id = window.text_system().resolve_font(&self.font);
        let cell_width = window
            .text_system()
            .advance(font_id, font_size, 'm')
            .expect("glyph not found for 'm'")
            .width;
        let ascent = window.text_system().ascent(font_id, font_size);
        let descent = window.text_system().descent(font_id, font_size);
        let base_height = ascent + descent.abs();
        let cell_height = base_height * self.line_height_multiplier;
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


        let empty = GridPrepaint {
            backgrounds: vec![], selections: vec![], glyphs: vec![], builtins: vec![],
            font_size, cell_width, cell_height,
        };

        // Viewport culling: skip entire block if outside visible area
        let viewport = window.content_mask().bounds;
        let block_bottom = bounds.origin.y + cell_height * self.snapshot.content_rows as f32;
        if block_bottom < viewport.origin.y || bounds.origin.y > viewport.origin.y + viewport.size.height {
            return empty;
        }

        let bg_color = self.terminal_bg;
        let text_x = bounds.origin.x + px(BLOCK_HEADER_PAD_X);
        // Store actual grid origin for exact hit_test mapping
        if let Some(store) = &self.origin_store {
            store.set(Some(bounds.origin.y));
        }

        // Resolve the base font once for the block
        let base_font_id = window.text_system().resolve_font(&self.font);
        let ascent = window.text_system().ascent(base_font_id, font_size);

        let mut backgrounds = Vec::new();
        let mut selections = Vec::new();
        let mut glyphs = Vec::new();
        let mut builtins = Vec::new();

        let selection_color = self.selection_color;
        let selection = &self.selection;

        let viewport_top = viewport.origin.y;
        let viewport_bottom = viewport.origin.y + viewport.size.height;

        let grid_history_size = self.snapshot.grid_history_size;
        let command_offset = self.snapshot.command_row_count;
        let grid_origin_y = bounds.origin.y;
        for (line_idx, snap_line) in self.snapshot.lines.iter().enumerate() {
            // Position via multiplication — no float accumulation drift over thousands of rows.
            // This matches the hit_test division: visual_row = y_in_grid / cell_height.
            let current_y = grid_origin_y + cell_height * line_idx as f32;

            // Selection lives on BlockGrid which doesn't have command text lines.
            // Subtract command_row_count so snapshot line indices map to grid selection indices.
            let grid_line = raijin_term::index::Line((line_idx as i32) - (grid_history_size as i32) - (command_offset as i32));
            let line_bottom = current_y + cell_height;

            if line_bottom < viewport_top {
                continue;
            }
            if current_y > viewport_bottom {
                break;
            }

            if snap_line.cells.is_empty() {
                continue;
            }

            let mut col_x = 0usize;

            for cell in &snap_line.cells {
                let x = text_x + cell_width * col_x as f32;

                // Background
                if cell.bg != bg_color {
                    let width = if cell.wide { cell_width * 2.0 } else { cell_width };
                    backgrounds.push((
                        Bounds::new(point(x, current_y), size(width, cell_height)),
                        cell.bg,
                    ));
                }

                // Selection highlight
                if let Some(sel) = selection {
                    let cell_point = raijin_term::index::Point::new(grid_line, raijin_term::index::Column(col_x));
                    if sel.contains(cell_point) {
                        let width = if cell.wide { cell_width * 2.0 } else { cell_width };
                        selections.push((
                            Bounds::new(point(x, current_y), size(width, cell_height)),
                            selection_color,
                        ));
                    }
                }

                // Skip spaces and null chars — nothing to render
                let skip = cell.c == ' ' || cell.c == '\0';

                if !skip {
                    // Built-in box-drawing / block elements
                    if let Some(bc) = builtin_font::builtin_char(cell.c) {
                        builtins.push(BuiltinGlyph { bc, x, y: current_y, color: cell.fg.into() });
                    } else {
                        // Regular character — resolve font and glyph, position at grid cell
                        let font_id = if cell.font_family_override.is_some() || cell.bold || cell.italic {
                            let family = match &cell.font_family_override {
                                Some(f) => f.clone().into(),
                                None => self.font.family.clone(),
                            };
                            let font = Font {
                                family,
                                weight: if cell.bold { FontWeight::BOLD } else { FontWeight::NORMAL },
                                style: if cell.italic { inazuma::FontStyle::Italic } else { inazuma::FontStyle::Normal },
                                ..Font::default()
                            };
                            window.text_system().resolve_font(&font)
                        } else {
                            base_font_id
                        };

                        // Build the full character string including zero-width
                        // combiners (e.g., U+FE0F variation selector for emoji).
                        let has_zerowidth = !cell.zerowidth.is_empty();
                        let char_str = if has_zerowidth {
                            let mut s = String::with_capacity(cell.c.len_utf8() + cell.zerowidth.len() * 4);
                            s.push(cell.c);
                            for &zw in &cell.zerowidth {
                                s.push(zw);
                            }
                            s
                        } else {
                            String::new() // unused, avoid allocation
                        };

                        // Try direct glyph lookup (fast path for primary font).
                        // Skip if there are zero-width combiners — those need shaping
                        // for correct emoji/variation selector handling.
                        let direct_glyph = if !has_zerowidth {
                            window.text_system().glyph_for_char(font_id, cell.c)
                        } else {
                            None
                        };

                        if let Some(glyph_id) = direct_glyph {
                            let is_emoji = window.text_system().is_emoji(font_id);
                            let baseline_y = current_y + ascent;
                            glyphs.push(CellGlyph {
                                origin: point(x, baseline_y),
                                font_id,
                                glyph_id,
                                color: cell.fg.into(),
                                is_emoji,
                            });
                        } else {
                            // Font doesn't have this glyph, or char has zero-width
                            // combiners — shape via CoreText for font fallback and
                            // proper variation selector / emoji handling.
                            let text = if has_zerowidth {
                                SharedString::from(char_str)
                            } else {
                                SharedString::from(String::from(cell.c))
                            };
                            let text_len = text.len();
                            let family = match &cell.font_family_override {
                                Some(f) => f.clone().into(),
                                None => self.font.family.clone(),
                            };
                            let runs = &[inazuma::TextRun {
                                len: text_len,
                                font: Font {
                                    family,
                                    weight: if cell.bold { FontWeight::BOLD } else { FontWeight::NORMAL },
                                    style: if cell.italic { inazuma::FontStyle::Italic } else { inazuma::FontStyle::Normal },
                                    ..Font::default()
                                },
                                color: cell.fg.into(),
                                background_color: None,
                                underline: None,
                                strikethrough: None,
                            }];
                            let shaped = window.text_system().shape_line(text, font_size, runs, None);
                            if let Some(run) = shaped.runs.first() {
                                if let Some(glyph) = run.glyphs.first() {
                                    let baseline_y = current_y + ascent;
                                    glyphs.push(CellGlyph {
                                        origin: point(x, baseline_y),
                                        font_id: run.font_id,
                                        glyph_id: glyph.id,
                                        color: cell.fg.into(),
                                        is_emoji: window.text_system().is_emoji(run.font_id),
                                    });
                                }
                            }
                        }
                    }
                }

                col_x += if cell.wide { 2 } else { 1 };
            }
        }

        GridPrepaint {
            backgrounds, selections, glyphs, builtins,
            font_size, cell_width, cell_height,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        // Backgrounds
        for (rect, color) in &prepaint.backgrounds {
            window.paint_quad(fill(*rect, *color));
        }

        // Selection highlights (on top of backgrounds, under text)
        for (rect, color) in &prepaint.selections {
            window.paint_quad(fill(*rect, *color));
        }

        // Text glyphs — per-cell at exact grid positions
        for glyph in &prepaint.glyphs {
            if glyph.is_emoji {
                let _ = window.paint_emoji(
                    glyph.origin,
                    glyph.font_id,
                    glyph.glyph_id,
                    prepaint.font_size,
                );
            } else {
                let _ = window.paint_glyph(
                    glyph.origin,
                    glyph.font_id,
                    glyph.glyph_id,
                    prepaint.font_size,
                    glyph.color,
                );
            }
        }

        // Built-in characters (box drawing, blocks, shades)
        for glyph in &prepaint.builtins {
            builtin_font::draw_builtin(
                glyph.bc,
                glyph.x,
                glyph.y,
                prepaint.cell_width,
                prepaint.cell_height,
                glyph.color,
                window,
            );
        }
    }
}
