use raijin_term::grid::Dimensions;
use raijin_term::index::{Column, Line};
use raijin_term::term::cell::Flags as CellFlags;
use inazuma::{
    point, px, size, App, Bounds, Element, ElementId, Font, FontStyle, FontWeight,
    GlobalElementId, Hsla, InspectorElementId, IntoElement, LayoutId, Pixels, Point,
    SharedString, ShapedLine, Size, StrikethroughStyle, Style, TextAlign, TextRun,
    UnderlineStyle, Window, fill, hsla,
};
use raijin_terminal::TerminalHandle;

use crate::terminal::colors::resolve_colors;
use crate::terminal::constants::*;

// ---------------------------------------------------------------------------
// Layout / prepaint state
// ---------------------------------------------------------------------------

/// Terminal grid dimensions computed from font metrics.
pub struct TerminalLayout {
    pub font: Font,
    pub font_size: Pixels,
    pub cell_width: Pixels,
    pub cell_height: Pixels,
    pub line_height: Pixels,
}

/// A two-line block header like Warp:
/// Line 1 (dimmed): username hostname cwd time (duration)
/// Line 2 (bright): command text
pub(crate) struct BlockHeaderPaint {
    /// Line 1: metadata (username, hostname, cwd, time, duration)
    metadata_line: ShapedLine,
    metadata_origin: Point<Pixels>,
    /// Lines 2+: command text (may be multi-line for shell scripts)
    command_lines: Vec<(ShapedLine, Point<Pixels>)>,
}

/// Full block body paint info (Header + Command + Output as one visual unit).
pub(crate) struct BlockBodyPaint {
    /// Background over the entire block (header + output + bottom padding).
    pub bounds: Bounds<Pixels>,
    /// Whether this block is selected/clicked (green highlight).
    pub selected: bool,
    /// Whether this block has an error exit code.
    pub is_error: bool,
    /// Left border for error blocks (spans entire block height).
    pub left_border: Option<Bounds<Pixels>>,
}

/// Sticky header overlay — shown when a block's header scrolls out of view.
pub(crate) struct StickyHeaderPaint {
    pub bg_bounds: Bounds<Pixels>,
    pub metadata_line: ShapedLine,
    pub metadata_origin: Point<Pixels>,
    pub command_line: ShapedLine,
    pub command_origin: Point<Pixels>,
    pub is_error: bool,
}

/// Pre-painted state: shaped text lines, background rects, block headers.
pub struct TerminalPrepaint {
    pub lines: Vec<(Point<Pixels>, ShapedLine)>,
    pub backgrounds: Vec<(Bounds<Pixels>, Hsla)>,
    pub cursor_rect: Option<Bounds<Pixels>>,
    pub line_height: Pixels,
    pub block_headers: Vec<BlockHeaderPaint>,
    /// Full block body backgrounds (header + output area).
    pub block_bodies: Vec<BlockBodyPaint>,
    /// Sticky header overlay (if a block header scrolled out of view).
    pub sticky_header: Option<StickyHeaderPaint>,
}

// ---------------------------------------------------------------------------
// TerminalElement
// ---------------------------------------------------------------------------

/// Custom Inazuma element that renders the terminal grid with block headers.
/// Cached shaped lines for a finished block (never changes after finalization).
struct BlockShapeCache {
    lines: Vec<(Point<Pixels>, ShapedLine)>,
    backgrounds: Vec<(Bounds<Pixels>, Hsla)>,
    /// Number of content rows when cached — invalidate if grid resizes.
    content_rows: usize,
    /// Grid columns when cached — invalidate on column resize.
    cols: usize,
}

pub struct TerminalElement {
    handle: TerminalHandle,
    /// Font family from config.
    font_family: String,
    /// Font size from config.
    font_size: f32,
    /// Cursor style from config.
    cursor_beam: bool,
    /// Index of the selected/clicked block (green highlight).
    selected_block: Option<usize>,
    /// Last terminal dimensions to avoid redundant resize calls.
    last_size: Option<(u16, u16)>,
    /// Shaped line cache for finished blocks. Key = block index.
    shape_cache: std::collections::HashMap<usize, BlockShapeCache>,
}

impl TerminalElement {
    pub fn new(handle: TerminalHandle) -> Self {
        Self {
            handle,
            font_family: FONT_FAMILIES[0].to_string(),
            font_size: FONT_SIZE,
            cursor_beam: true,
            selected_block: None,
            last_size: None,
            shape_cache: std::collections::HashMap::new(),
        }
    }

    /// Set the selected block index (for green highlight on click).
    pub fn with_selected_block(mut self, selected: Option<usize>) -> Self {
        self.selected_block = selected;
        self
    }

    /// Set font family and size from config.
    pub fn with_font(mut self, family: &str, size: f32) -> Self {
        self.font_family = family.to_string();
        self.font_size = size;
        self
    }

    /// Set cursor style (true = beam, false = block).
    pub fn with_cursor_beam(mut self, beam: bool) -> Self {
        self.cursor_beam = beam;
        self
    }

    fn compute_layout(&self, window: &mut Window) -> TerminalLayout {
        let font_size = px(self.font_size);

        let mut font = Font {
            family: self.font_family.clone().into(),
            weight: FontWeight::NORMAL,
            ..Font::default()
        };

        let font_id = window.text_system().resolve_font(&font);

        let cell_width = window
            .text_system()
            .advance(font_id, font_size, 'M')
            .expect("failed to get advance width for 'M'")
            .width;

        let ascent = window.text_system().ascent(font_id, font_size);
        let descent = window.text_system().descent(font_id, font_size);
        let cell_height = ascent + descent.abs() + px(CELL_PADDING);
        let line_height = cell_height;

        if let Some(resolved) = window.text_system().get_font_for_id(font_id) {
            font = resolved;
        }

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
        // Resize terminal grid + PTY only if bounds actually changed
        let new_cols = (bounds.size.width / layout.cell_width).floor() as u16;
        let new_rows = (bounds.size.height / layout.cell_height).floor() as u16;
        if self.last_size != Some((new_rows, new_cols)) {
            self.handle.set_size(new_rows, new_cols);
            self.last_size = Some((new_rows, new_cols));
        }

        let term = self.handle.lock();
        let content = term.renderable_content();
        let colors = content.colors;
        let mode = content.mode;
        let router = term.block_router();
        let router_blocks = router.blocks();

        let bg_color = terminal_bg();

        let mut lines = Vec::new();
        let mut backgrounds = Vec::new();
        let mut cursor_rect = None;
        let mut block_headers: Vec<BlockHeaderPaint> = Vec::new();
        let mut block_bodies: Vec<BlockBodyPaint> = Vec::new();

        // Pair metadata (from workspace BlockManager) with grid data (from block_router).
        // Both are populated from the same OSC events, so indices correspond.
        let block_count = router_blocks.len();

        if block_count == 0 {
            return TerminalPrepaint {
                lines,
                backgrounds,
                cursor_rect,
                line_height: layout.line_height,
                block_headers,
                block_bodies,
                sticky_header: None,
            };
        }

        // First pass: compute total content height for bottom-grow layout
        struct BlockLayoutInfo {
            header_height: f32,
            content_rows: usize,
        }
        let mut block_layouts: Vec<BlockLayoutInfo> = Vec::with_capacity(block_count);
        let mut total_height = px(0.0);

        for i in 0..block_count {
            let block_grid = &router_blocks[i];

            let cmd_lines = block_grid.command.lines().count().max(1);
            let header_height = BLOCK_HEADER_HEIGHT
                + (cmd_lines.saturating_sub(1) as f32 * (HEADER_CMD_FONT_SIZE + 4.0));

            // Content rows: scrollback history + visible rows up to cursor position
            let cursor_line = block_grid.grid.cursor.point.line.0.max(0) as usize;
            let history = block_grid.grid.history_size();
            let content_rows = history + cursor_line + 1;

            total_height += px(header_height + BLOCK_GAP)
                + layout.cell_height * content_rows as f32
                + px(BLOCK_BODY_PAD_BOTTOM);

            block_layouts.push(BlockLayoutInfo { header_height, content_rows });
        }

        // Bottom-grow layout: push content to the bottom of the bounds
        let y_offset = (bounds.size.height - total_height).max(px(0.0));
        let mut current_y = bounds.origin.y + y_offset;

        // Second pass: render each block (header + grid content)
        let viewport_top = bounds.origin.y;
        let viewport_bottom = bounds.origin.y + bounds.size.height;

        for i in 0..block_count {
            let bl = &block_layouts[i];
            let block_height = px(bl.header_height + BLOCK_GAP)
                + layout.cell_height * bl.content_rows as f32
                + px(BLOCK_BODY_PAD_BOTTOM);

            // Viewport culling: skip blocks entirely outside the visible area
            if current_y + block_height < viewport_top {
                current_y += block_height;
                continue;
            }
            if current_y > viewport_bottom {
                break; // All remaining blocks are below viewport
            }

            let block_grid = &router_blocks[i];
            let meta = &block_grid.metadata;
            let grid = &block_grid.grid;
            let is_error = block_grid.exit_code.map_or(false, |c| c != 0);
            let is_running = block_grid.exit_code.is_none();
            let is_selected = self.selected_block == Some(i);

            let header_y = current_y;
            let text_x = bounds.origin.x + px(BLOCK_HEADER_PAD_X);

            // ---- Block Header ----
            // Line 1: metadata (Warp-style)
            let mut meta_parts: Vec<String> = Vec::new();
            if let Some(ref user) = meta.username {
                meta_parts.push(user.clone());
            }
            if let Some(ref host) = meta.hostname {
                meta_parts.push(host.clone());
            }
            if let Some(ref cwd) = meta.cwd {
                meta_parts.push(cwd.clone());
            }
            if let Some(ref branch) = meta.git_branch {
                meta_parts.push(format!(" {}", branch));
            }
            // Time display from block's started_at
            let elapsed = block_grid.started_at.elapsed();
            let now = time::OffsetDateTime::now_local()
                .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
            let at_start = now - elapsed;
            let time_display = at_start
                .format(time::macros::format_description!("[hour]:[minute]"))
                .unwrap_or_else(|_| "--:--".to_string());
            meta_parts.push(time_display);

            let dur_text = if is_running {
                "running...".to_string()
            } else {
                let duration = block_grid.finished_at
                    .map(|f| f.duration_since(block_grid.started_at))
                    .unwrap_or_default();
                format!("({:.3}s)", duration.as_secs_f64())
            };
            meta_parts.push(dur_text);

            let meta_text = meta_parts.join("  ");
            let meta_runs = vec![TextRun {
                len: meta_text.len(),
                font: Font {
                    family: layout.font.family.clone(),
                    weight: FontWeight::NORMAL,
                    ..Font::default()
                },
                color: header_metadata_fg(),
                background_color: None,
                underline: None,
                strikethrough: None,
            }];
            let metadata_line = window.text_system().shape_line(
                SharedString::from(meta_text),
                px(HEADER_META_FONT_SIZE),
                &meta_runs,
                None,
            );
            let metadata_origin = point(text_x, header_y + px(4.0));

            // Lines 2+: command text (bright, larger, may be multi-line)
            let cmd_text = if block_grid.command.is_empty() {
                "(empty)".to_string()
            } else {
                block_grid.command.clone()
            };
            let cmd_color = header_command_fg();
            let cmd_lines_text: Vec<&str> = cmd_text.lines().collect();
            let mut command_lines = Vec::with_capacity(cmd_lines_text.len());
            let cmd_base_y = header_y + px(4.0 + HEADER_META_FONT_SIZE + 8.0);

            for (line_idx, line_text) in cmd_lines_text.iter().enumerate() {
                let text = if line_text.is_empty() { " " } else { line_text };
                let runs = vec![TextRun {
                    len: text.len(),
                    font: Font {
                        family: layout.font.family.clone(),
                        weight: FontWeight::MEDIUM,
                        ..Font::default()
                    },
                    color: cmd_color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                }];
                let shaped = window.text_system().shape_line(
                    SharedString::from(text.to_string()),
                    px(HEADER_CMD_FONT_SIZE),
                    &runs,
                    None,
                );
                let origin = point(
                    text_x,
                    cmd_base_y + px(line_idx as f32 * (HEADER_CMD_FONT_SIZE + 4.0)),
                );
                command_lines.push((shaped, origin));
            }

            block_headers.push(BlockHeaderPaint {
                metadata_line,
                metadata_origin,
                command_lines,
            });

            current_y += px(bl.header_height + BLOCK_GAP);
            let output_start_y = current_y;

            // ---- Block Grid Content ----
            let history_size = grid.history_size();
            let grid_cols = grid.columns();
            let screen_lines = grid.screen_lines() as i32;

            // Use cached shaped lines for finished blocks
            let is_finished = block_grid.is_finished();
            let cache_valid = is_finished && self.shape_cache.get(&i).is_some_and(|c| {
                c.content_rows == bl.content_rows && c.cols == grid_cols
            });

            if cache_valid {
                // Reuse cached lines with adjusted Y positions
                let cache = self.shape_cache.get(&i).unwrap();
                for (cached_origin, shaped) in &cache.lines {
                    let base_y = cache.lines.first().map_or(px(0.0), |l| l.0.y);
                    lines.push((point(cached_origin.x, output_start_y + (cached_origin.y - base_y)), shaped.clone()));
                }
                for (cached_bounds, color) in &cache.backgrounds {
                    let dy = output_start_y - cache.lines.first().map_or(px(0.0), |l| l.0.y);
                    backgrounds.push((
                        Bounds::new(point(cached_bounds.origin.x, cached_bounds.origin.y + dy), cached_bounds.size),
                        *color,
                    ));
                }
                current_y += layout.cell_height * bl.content_rows as f32;
            } else {

            let block_lines_start = lines.len();
            let block_bgs_start = backgrounds.len();

            for row_offset in 0..bl.content_rows {
                let line_idx = row_offset as i32 - history_size as i32;
                let line = Line(line_idx);

                // Bounds check
                if line.0 >= screen_lines || line.0 < -(history_size as i32) {
                    current_y += layout.cell_height;
                    continue;
                }

                let mut line_text = String::with_capacity(grid_cols);
                let mut runs: Vec<TextRun> = Vec::new();
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

                    // Background rect if non-default
                    if bg != bg_color {
                        let x = text_x + layout.cell_width * col_idx as f32;
                        let width = if flags.contains(CellFlags::WIDE_CHAR) {
                            layout.cell_width * 2.0
                        } else {
                            layout.cell_width
                        };
                        backgrounds.push((
                            Bounds::new(point(x, current_y), size(width, layout.cell_height)),
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
                    let origin = point(text_x, current_y);
                    lines.push((origin, shaped));
                }

                current_y += layout.cell_height;
            }

            // Cache shaped lines for finished blocks
            if is_finished && !cache_valid {
                let block_lines: Vec<_> = lines[block_lines_start..].to_vec();
                let block_bgs: Vec<_> = backgrounds[block_bgs_start..].to_vec();
                self.shape_cache.insert(i, BlockShapeCache {
                    lines: block_lines,
                    backgrounds: block_bgs,
                    content_rows: bl.content_rows,
                    cols: grid_cols,
                });
            }

            } // end of else (non-cached path)

            // Block body bounds (header + output + padding)
            let body_height = current_y - header_y + px(BLOCK_BODY_PAD_BOTTOM);
            current_y += px(BLOCK_BODY_PAD_BOTTOM);

            if body_height > px(0.0) {
                let body_bounds = Bounds::new(
                    point(bounds.origin.x, header_y),
                    size(bounds.size.width, body_height),
                );
                let left_border = if is_error {
                    Some(Bounds::new(
                        point(bounds.origin.x, header_y),
                        size(px(BLOCK_LEFT_BORDER), body_height),
                    ))
                } else {
                    None
                };
                block_bodies.push(BlockBodyPaint {
                    bounds: body_bounds,
                    selected: is_selected,
                    is_error,
                    left_border,
                });
            }

            // Cursor for the active (running) block
            if is_running && mode.contains(raijin_term::term::TermMode::SHOW_CURSOR) {
                let cursor_point = grid.cursor.point;
                let cursor_row_y = output_start_y
                    + layout.cell_height * (history_size as f32 + cursor_point.line.0 as f32);
                let cursor_x = text_x + layout.cell_width * cursor_point.column.0 as f32;
                let cursor_width = if self.cursor_beam {
                    px(2.0)
                } else {
                    layout.cell_width
                };
                cursor_rect = Some(Bounds::new(
                    point(cursor_x, cursor_row_y),
                    size(cursor_width, layout.cell_height),
                ));
            }
        }

        // Sticky header: if a block's header scrolled above the viewport
        // but its body is still visible, show a compact overlay at the top.
        let sticky_header = block_bodies.iter().zip(block_headers.iter()).find_map(|(body, header)| {
            if header.metadata_origin.y < bounds.origin.y
                && body.bounds.origin.y + body.bounds.size.height > bounds.origin.y
            {
                let sticky_y = bounds.origin.y;
                let sticky_height = px(BLOCK_HEADER_HEIGHT.min(36.0));
                let text_x = bounds.origin.x + px(BLOCK_HEADER_PAD_X);

                let sticky_bg = Bounds::new(
                    point(bounds.origin.x, sticky_y),
                    size(bounds.size.width, sticky_height),
                );

                let meta_line = header.metadata_line.clone();
                let meta_origin = point(text_x, sticky_y + px(4.0));

                let cmd_line = header.command_lines.first()
                    .map(|(line, _)| line.clone())
                    .unwrap_or_else(|| header.metadata_line.clone());
                let cmd_origin = point(text_x, sticky_y + px(4.0 + HEADER_META_FONT_SIZE + 4.0));

                Some(StickyHeaderPaint {
                    bg_bounds: sticky_bg,
                    metadata_line: meta_line,
                    metadata_origin: meta_origin,
                    command_line: cmd_line,
                    command_origin: cmd_origin,
                    is_error: body.is_error,
                })
            } else {
                None
            }
        });

        TerminalPrepaint {
            lines,
            backgrounds,
            cursor_rect,
            line_height: layout.line_height,
            block_headers,
            block_bodies,
            sticky_header,
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
        let content_mask = inazuma::ContentMask { bounds };
        window.with_content_mask(Some(content_mask), |window| {
            // 1. Overall background
            window.paint_quad(fill(bounds, terminal_bg()));

            // 2. Block body backgrounds (entire block: header + output)
            for body in &prepaint.block_bodies {
                let bg = if body.selected {
                    block_selected_bg()
                } else if body.is_error {
                    block_header_error_bg()
                } else {
                    block_body_bg()
                };
                window.paint_quad(fill(body.bounds, bg));

                // Left border (error indicator, spans entire block height)
                if let Some(border) = body.left_border {
                    window.paint_quad(fill(border, error_color()));
                }
            }

            // 3. Block header text (metadata + command)
            for header in &prepaint.block_headers {

                // Line 1: metadata (dimmed)
                let _ = header.metadata_line.paint(
                    header.metadata_origin,
                    prepaint.line_height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                );

                // Lines 2+: command text (bright, may be multi-line)
                for (cmd_line, cmd_origin) in &header.command_lines {
                    let _ = cmd_line.paint(
                        *cmd_origin,
                        prepaint.line_height,
                        TextAlign::Left,
                        None,
                        window,
                        cx,
                    );
                }
            }

            // 4. Cell backgrounds
            for (rect, color) in &prepaint.backgrounds {
                window.paint_quad(fill(*rect, *color));
            }

            // 5. Cursor
            if let Some(cursor_rect) = prepaint.cursor_rect {
                window.paint_quad(fill(cursor_rect, cursor_color()));
            }

            // 6. Text
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

            // 7. Sticky header overlay (always on top)
            if let Some(sticky) = &prepaint.sticky_header {
                let bg = if sticky.is_error {
                    block_header_error_bg()
                } else {
                    // Slightly more opaque than normal block bg for sticky visibility
                    hsla(0.0, 0.0, 0.1, 0.95)
                };
                window.paint_quad(fill(sticky.bg_bounds, bg));

                let _ = sticky.metadata_line.paint(
                    sticky.metadata_origin,
                    prepaint.line_height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                );
                let _ = sticky.command_line.paint(
                    sticky.command_origin,
                    prepaint.line_height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            }
        });
    }
}

// Color helpers are now in terminal/colors.rs

