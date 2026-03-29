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
    FontId, GlyphId, GlobalElementId, Hsla, InspectorElementId, IntoElement, LayoutId,
    Pixels, Point, SharedString, Style, Window, fill,
};

use super::builtin_font::{self, BuiltinChar};
use super::constants::*;
use super::grid_snapshot::BlockGridSnapshot;

// ── Prepaint data ─────────────────────────────────────────────────────────────

/// A single cell glyph to paint at an exact grid position.
struct CellGlyph {
    origin: Point<Pixels>,
    font_id: FontId,
    glyph_id: GlyphId,
    color: Hsla,
    is_emoji: bool,
}

/// A built-in glyph to render as GPU primitives.
struct BuiltinGlyph {
    bc: BuiltinChar,
    x: Pixels,
    y: Pixels,
    color: Hsla,
}

/// Pre-painted state for the grid element.
pub struct GridPrepaint {
    backgrounds: Vec<(Bounds<Pixels>, Hsla)>,
    glyphs: Vec<CellGlyph>,
    builtins: Vec<BuiltinGlyph>,
    font_size: Pixels,
    cell_width: Pixels,
    cell_height: Pixels,
    pub line_height: Pixels,
}

// ── Element ───────────────────────────────────────────────────────────────────

/// Custom element that renders a block's terminal grid from a snapshot.
///
/// No mutex locking happens here — all data comes from the snapshot
/// which was extracted with a single lock in block_list.rs.
pub struct TerminalGridElement {
    snapshot: BlockGridSnapshot,
    font: Font,
    font_size: f32,
    line_height_multiplier: f32,
}

impl TerminalGridElement {
    pub fn new(snapshot: BlockGridSnapshot, font: Font, font_size: f32, line_height_multiplier: f32) -> Self {
        Self {
            snapshot,
            font,
            font_size,
            line_height_multiplier,
        }
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
        let line_height = cell_height;

        let empty = GridPrepaint {
            backgrounds: vec![], glyphs: vec![], builtins: vec![],
            font_size, cell_width, cell_height, line_height,
        };

        // Viewport culling: skip entire block if outside visible area
        let viewport = window.content_mask().bounds;
        let block_bottom = bounds.origin.y + cell_height * self.snapshot.content_rows as f32;
        if block_bottom < viewport.origin.y || bounds.origin.y > viewport.origin.y + viewport.size.height {
            return empty;
        }

        let bg_color = terminal_bg();
        let text_x = bounds.origin.x + px(BLOCK_HEADER_PAD_X);
        let mut current_y = bounds.origin.y;

        // Resolve the base font once for the block
        let base_font_id = window.text_system().resolve_font(&self.font);
        let ascent = window.text_system().ascent(base_font_id, font_size);

        let mut backgrounds = Vec::new();
        let mut glyphs = Vec::new();
        let mut builtins = Vec::new();

        let viewport_top = viewport.origin.y;
        let viewport_bottom = viewport.origin.y + viewport.size.height;

        for snap_line in self.snapshot.lines.iter() {
            let line_bottom = current_y + cell_height;

            if line_bottom < viewport_top {
                current_y += cell_height;
                continue;
            }
            if current_y > viewport_bottom {
                break;
            }

            if snap_line.cells.is_empty() {
                current_y += cell_height;
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

                // Skip spaces and null chars — nothing to render
                let skip = cell.c == ' ' || cell.c == '\0';

                if !skip {
                    // Built-in box-drawing / block elements
                    if let Some(bc) = builtin_font::builtin_char(cell.c) {
                        builtins.push(BuiltinGlyph { bc, x, y: current_y, color: cell.fg });
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
                                color: cell.fg,
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
                                color: cell.fg,
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
                                        color: cell.fg,
                                        is_emoji: window.text_system().is_emoji(run.font_id),
                                    });
                                }
                            }
                        }
                    }
                }

                col_x += if cell.wide { 2 } else { 1 };
            }

            current_y += cell_height;
        }

        GridPrepaint {
            backgrounds, glyphs, builtins,
            font_size, cell_width, cell_height, line_height,
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
