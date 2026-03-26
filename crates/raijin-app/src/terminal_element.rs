use raijin_term::grid::Dimensions;
use raijin_term::index::{Column, Line};
use raijin_term::term::cell::Flags as CellFlags;
use raijin_term::vte::ansi::{Color as AnsiColor, NamedColor};
use inazuma::{
    point, px, size, App, Bounds, Element, ElementId, Font, FontStyle, FontWeight,
    GlobalElementId, Hsla, InspectorElementId, IntoElement, LayoutId, Pixels, Point,
    SharedString, ShapedLine, Size, StrikethroughStyle, Style, TextAlign, TextRun,
    UnderlineStyle, Window, fill, hsla, rgb,
};
use raijin_terminal::TerminalHandle;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FONT_SIZE: f32 = 14.0;
const CELL_PADDING: f32 = 2.0;

/// Two-line block header like Warp: metadata line + command line.
const BLOCK_HEADER_HEIGHT: f32 = 50.0;
const BLOCK_HEADER_PAD_X: f32 = 16.0;
const BLOCK_GAP: f32 = 0.0;
const BLOCK_LEFT_BORDER: f32 = 4.0;

const HEADER_META_FONT_SIZE: f32 = 11.0;
const HEADER_CMD_FONT_SIZE: f32 = 13.0;

/// Font families to try in order. First match wins.
const FONT_FAMILIES: &[&str] = &[
    "JetBrainsMono Nerd Font",
    "JetBrains Mono",
    "Menlo",
    "Monaco",
];

// ---------------------------------------------------------------------------
// Raijin Dark Theme
// ---------------------------------------------------------------------------

fn terminal_bg() -> Hsla {
    rgb(0x121212).into()
}

fn terminal_fg() -> Hsla {
    rgb(0xf1f1f1).into()
}

fn cursor_color() -> Hsla {
    rgb(0x14F195).into()
}

fn error_color() -> Hsla {
    rgb(0xff5f5f).into()
}

fn block_body_bg() -> Hsla {
    hsla(0.0, 0.0, 1.0, 0.04)
}

/// Accent-tinted background when a block is selected/clicked.
fn block_selected_bg() -> Hsla {
    hsla(153.0 / 360.0, 0.93, 0.51, 0.08)
}

fn block_header_error_bg() -> Hsla {
    // Semi-transparent red overlay — the dark terminal bg (#121212) shines through
    hsla(0.0, 0.7, 0.45, 0.12)
}

fn header_command_fg() -> Hsla {
    rgb(0xf1f1f1).into()
}

fn header_metadata_fg() -> Hsla {
    hsla(0.0, 0.0, 1.0, 0.35)
}

// ---------------------------------------------------------------------------
// Block render info (passed from workspace)
// ---------------------------------------------------------------------------

/// Metadata for rendering a block header overlay.
#[derive(Clone)]
pub struct BlockRenderInfo {
    pub command: String,
    pub duration_display: String,
    pub exit_code: Option<i32>,
    /// Absolute row where this block's output starts (history_size + cursor_line at marker time).
    pub abs_start_row: usize,
    /// Absolute row where this block's output ends (None if still running).
    pub abs_end_row: Option<usize>,
    /// CWD at the time this block was created (from shell metadata).
    pub cwd_short: Option<String>,
    /// Git branch at block creation time.
    pub git_branch: Option<String>,
    /// Username at block creation time.
    pub username: Option<String>,
    /// Hostname at block creation time.
    pub hostname: Option<String>,
    /// Time when the block was created (e.g. "17:33").
    pub time_display: String,
}

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

/// Bottom padding inside a block body (space after last output line).
const BLOCK_BODY_PAD_BOTTOM: f32 = 8.0;

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

/// Pre-painted state: shaped text lines, background rects, block headers.
pub struct TerminalPrepaint {
    pub lines: Vec<(Point<Pixels>, ShapedLine)>,
    pub backgrounds: Vec<(Bounds<Pixels>, Hsla)>,
    pub cursor_rect: Option<Bounds<Pixels>>,
    pub line_height: Pixels,
    pub block_headers: Vec<BlockHeaderPaint>,
    /// Full block body backgrounds (header + output area).
    pub block_bodies: Vec<BlockBodyPaint>,
}

// ---------------------------------------------------------------------------
// TerminalElement
// ---------------------------------------------------------------------------

/// Custom Inazuma element that renders the terminal grid with block headers.
pub struct TerminalElement {
    handle: TerminalHandle,
    blocks: Vec<BlockRenderInfo>,
    /// Hide all rows before this absolute row (hides initial prompt in Raijin mode).
    hide_before_row: Option<usize>,
    /// Prompt regions to hide: (start_row, end_row) inclusive absolute rows.
    hidden_prompt_regions: Vec<(usize, usize)>,
    /// Font family from config.
    font_family: String,
    /// Font size from config.
    font_size: f32,
    /// Cursor style from config.
    cursor_beam: bool,
    /// Index of the selected/clicked block (green highlight).
    selected_block: Option<usize>,
}

impl TerminalElement {
    pub fn new(handle: TerminalHandle) -> Self {
        Self {
            handle,
            blocks: Vec::new(),
            hide_before_row: None,
            hidden_prompt_regions: Vec::new(),
            font_family: FONT_FAMILIES[0].to_string(),
            font_size: FONT_SIZE,
            cursor_beam: true,
            selected_block: None,
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

    pub fn with_blocks(mut self, blocks: Vec<BlockRenderInfo>) -> Self {
        self.blocks = blocks;
        self
    }

    pub fn with_hide_before_row(mut self, row: Option<usize>) -> Self {
        self.hide_before_row = row;
        self
    }

    pub fn with_hidden_prompt_regions(mut self, regions: Vec<(usize, usize)>) -> Self {
        self.hidden_prompt_regions = regions;
        self
    }

    /// Check if an absolute row falls inside a hidden prompt region or
    /// the current pending prompt (prompt_start_row to present).
    /// Rows that belong to a block are NEVER hidden.
    fn is_in_hidden_prompt_region(&self, abs_row: usize) -> bool {
        // Never hide rows that belong to a block's output range
        for block in &self.blocks {
            let end = block.abs_end_row.unwrap_or(usize::MAX);
            if abs_row >= block.abs_start_row && abs_row <= end {
                return false;
            }
        }

        // Closed prompt regions (between PromptStart and CommandStart)
        for &(start, end) in &self.hidden_prompt_regions {
            if abs_row >= start && abs_row <= end {
                return true;
            }
        }
        // Current open prompt (no CommandStart yet — user is still at prompt)
        if let Some(prompt_start) = self.hide_before_row {
            if abs_row >= prompt_start {
                return true;
            }
        }
        false
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

    /// Compute visual row for an absolute row given current grid state.
    fn abs_to_visual(abs_row: usize, history_size: usize, display_offset: usize) -> i64 {
        abs_row as i64 - history_size as i64 + display_offset as i64
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
        // Resize terminal grid + PTY if bounds changed
        let new_cols = (bounds.size.width / layout.cell_width).floor() as u16;
        let new_rows = (bounds.size.height / layout.cell_height).floor() as u16;
        self.handle.set_size(new_rows, new_cols);

        let term = self.handle.lock();
        let content = term.renderable_content();

        let grid_rows = term.screen_lines();
        let grid_cols = term.columns();
        let colors = content.colors;
        let display_offset = content.display_offset;
        let history_size = term.grid().history_size();

        let bg_color = terminal_bg();

        let mut lines = Vec::with_capacity(grid_rows);
        let mut backgrounds = Vec::new();
        let mut cursor_rect = None;
        let mut block_headers: Vec<BlockHeaderPaint> = Vec::new();
        let mut block_bodies: Vec<BlockBodyPaint> = Vec::new();

        // Track block body Y positions: (block_idx, header_y, last_output_y)
        let mut block_body_tracking: Vec<(usize, Pixels, Pixels)> = Vec::new();

        // Compute which rows to hide (initial prompt)
        let first_block_start = self.blocks.first().map(|b| b.abs_start_row);
        let hide_up_to = match (self.hide_before_row, first_block_start) {
            (Some(prompt_row), Some(block_start)) => Some(block_start.min(prompt_row + grid_rows)),
            (Some(prompt_row), None) => Some(prompt_row + grid_rows), // no blocks yet, hide prompt area
            _ => None,
        };

        // Build a set of visual rows where block headers should appear
        let mut header_at_visual_row: Vec<(i64, usize)> = Vec::new(); // (visual_row, block_index)
        for (idx, block) in self.blocks.iter().enumerate() {
            let visual = Self::abs_to_visual(block.abs_start_row, history_size, display_offset);
            header_at_visual_row.push((visual, idx));
        }
        header_at_visual_row.sort_by_key(|(v, _)| *v);

        // Bottom-grow layout: only render rows with content
        let cursor = &content.cursor;
        let cursor_point = cursor.point;
        let cursor_visual_row =
            (cursor_point.line.0 as usize).saturating_add(display_offset);

        // Cap content_rows: when all blocks are finished, stop at the last
        // block's end row. This eliminates the gap between the last block and
        // the input area — like Warp, blocks are tight discrete units.
        let has_active_block = self.blocks.last().is_some_and(|b| b.exit_code.is_none());
        let content_rows = if !self.blocks.is_empty() && !has_active_block {
            // All blocks finished — cap at last block's end_row + 1
            let last_end = self.blocks.iter()
                .filter_map(|b| b.abs_end_row)
                .max()
                .unwrap_or(0);
            let last_visual = Self::abs_to_visual(last_end, history_size, display_offset);
            ((last_visual as usize) + 1).min(grid_rows)
        } else {
            // Active block or no blocks — extend to cursor
            (cursor_visual_row + 1).min(grid_rows)
        };

        // Count how many rows will be hidden (prompt regions + empty block rows)
        let hidden_row_count = (0..content_rows)
            .filter(|&row_idx| {
                let abs_row = history_size + row_idx - display_offset.min(row_idx);
                // Hidden prompt regions
                if self.is_in_hidden_prompt_region(abs_row) {
                    return true;
                }
                // Initial prompt hiding
                if hide_up_to.is_some_and(|h| abs_row < h) && self.blocks.is_empty() {
                    return true;
                }
                // Empty no-output block rows (only header renders, no grid row)
                if self.blocks.iter().any(|b| {
                    abs_row >= b.abs_start_row
                        && abs_row <= b.abs_end_row.unwrap_or(usize::MAX)
                        && b.abs_end_row == Some(b.abs_start_row)
                        && b.exit_code.is_some()
                }) {
                    return true;
                }
                false
            })
            .count();
        let visible_content_rows = content_rows.saturating_sub(hidden_row_count);

        // Calculate total extra height from block headers that will be visible
        let visible_headers: Vec<(usize, usize)> = header_at_visual_row
            .iter()
            .filter(|(v, _)| *v >= 0 && (*v as usize) < content_rows)
            .map(|(v, idx)| (*v as usize, *idx))
            .collect();

        let total_header_height: f32 = visible_headers
            .iter()
            .map(|(_, idx)| {
                let cmd_lines = self.blocks[*idx].command.lines().count().max(1);
                BLOCK_HEADER_HEIGHT
                    + (cmd_lines.saturating_sub(1) as f32 * (HEADER_CMD_FONT_SIZE + 4.0))
                    + BLOCK_GAP
            })
            .sum();
        let content_height =
            layout.cell_height * visible_content_rows as f32 + px(total_header_height);

        // Y offset: push content to the bottom of the bounds
        let y_offset = bounds.size.height - content_height;

        // Track accumulated header offset as we iterate rows
        let mut header_offset = px(0.0);
        let mut next_header_idx = 0;

        // Cursor rect (adjusted for header offsets).
        // Don't render cursor if it's in a hidden prompt region.
        let cursor_abs_row = history_size + cursor_visual_row - display_offset.min(cursor_visual_row);
        let cursor_hidden = self.is_in_hidden_prompt_region(cursor_abs_row);
        if content.mode.contains(raijin_term::term::TermMode::SHOW_CURSOR) && !cursor_hidden {
            // Compute header offset at cursor row
            let mut cursor_header_offset = px(0.0);
            for (visual_row, idx) in &visible_headers {
                if *visual_row <= cursor_visual_row {
                    let cmd_lines = self.blocks[*idx].command.lines().count().max(1);
                    let h = BLOCK_HEADER_HEIGHT
                        + (cmd_lines.saturating_sub(1) as f32 * (HEADER_CMD_FONT_SIZE + 4.0));
                    cursor_header_offset += px(h + BLOCK_GAP);
                }
            }

            // Count hidden rows before the cursor to adjust visual position
            let hidden_before_cursor = (0..cursor_visual_row)
                .filter(|&r| {
                    let abs = history_size + r - display_offset.min(r);
                    self.is_in_hidden_prompt_region(abs)
                })
                .count();
            let cursor_y_row = cursor_visual_row.saturating_sub(hidden_before_cursor);

            let cx_px = bounds.origin.x + layout.cell_width * cursor_point.column.0 as f32;
            let cy_px = bounds.origin.y
                + y_offset
                + layout.cell_height * cursor_y_row as f32
                + cursor_header_offset;
            let cursor_width = if self.cursor_beam {
                px(2.0)
            } else {
                layout.cell_width
            };
            cursor_rect = Some(Bounds::new(
                point(cx_px, cy_px),
                size(cursor_width, layout.cell_height),
            ));
        }

        let grid = term.grid();

        // Track rendered row position separately from grid row index.
        // Hidden prompt rows are skipped but the visual layout stays compact.
        let mut visual_y_row: usize = 0;

        for row_idx in 0..content_rows {
            // Check if this row should be hidden (prompt area)
            let abs_row = history_size + row_idx - display_offset.min(row_idx);

            // Hide initial prompt (before any blocks)
            if let Some(hide_up) = hide_up_to {
                if abs_row < hide_up && self.blocks.is_empty() {
                    continue;
                }
            }

            // Hide prompt regions between blocks — rows where the shell prompt
            // (Starship, P10k, etc.) rendered. Like Warp, we don't render these.
            if self.is_in_hidden_prompt_region(abs_row) {
                continue;
            }

            // Check if a block header should be inserted before this row
            while next_header_idx < visible_headers.len()
                && visible_headers[next_header_idx].0 == row_idx
            {
                let block_idx = visible_headers[next_header_idx].1;
                let block = &self.blocks[block_idx];

                let header_y =
                    bounds.origin.y + y_offset + layout.cell_height * visual_y_row as f32 + header_offset;

                let is_error = block.exit_code.map_or(false, |c| c != 0);
                let is_running = block.exit_code.is_none();

                // Calculate dynamic header height based on command line count
                let cmd_line_count = block.command.lines().count().max(1);
                let header_height = BLOCK_HEADER_HEIGHT
                    + (cmd_line_count.saturating_sub(1) as f32 * (HEADER_CMD_FONT_SIZE + 4.0));

                let text_x = bounds.origin.x + px(BLOCK_HEADER_PAD_X);

                // --- Line 1: metadata (Warp-style) ---
                // Format: "nyxb MacBook-Pro.fritz.box ~ 17:33 (0.032s)"
                let mut meta_parts: Vec<String> = Vec::new();
                if let Some(ref user) = block.username {
                    meta_parts.push(user.clone());
                }
                if let Some(ref host) = block.hostname {
                    meta_parts.push(host.clone());
                }
                if let Some(ref cwd) = block.cwd_short {
                    meta_parts.push(cwd.clone());
                }
                if let Some(ref branch) = block.git_branch {
                    meta_parts.push(format!(" {}", branch));
                }
                meta_parts.push(block.time_display.clone());
                let dur_text = if is_running {
                    "running...".to_string()
                } else {
                    format!("({})", block.duration_display)
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

                // --- Lines 2+: command text (bright, larger, may be multi-line) ---
                let cmd_text = if block.command.is_empty() {
                    "(empty)".to_string()
                } else {
                    block.command.clone()
                };
                let cmd_color = if is_error {
                    header_command_fg()
                } else {
                    header_command_fg()
                };

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

                // Track the start of this block body for later bounds calculation
                block_body_tracking.push((block_idx, header_y, header_y + px(header_height)));

                header_offset += px(header_height + BLOCK_GAP);
                next_header_idx += 1;
            }

            // Skip empty grid rows that belong to no-output blocks.
            // Only the header renders — no cell_height gap below it.
            let in_empty_block = self.blocks.iter().any(|b| {
                abs_row >= b.abs_start_row
                    && abs_row <= b.abs_end_row.unwrap_or(usize::MAX)
                    && b.abs_end_row == Some(b.abs_start_row)
                    && b.exit_code.is_some()
            });
            if in_empty_block {
                continue;
            }

            // Render the grid row
            let mut line_text = String::with_capacity(grid_cols);
            let mut runs: Vec<TextRun> = Vec::new();
            let mut skip_next = false;

            let line = Line(row_idx as i32 - display_offset as i32);

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
                    let x = bounds.origin.x + layout.cell_width * col_idx as f32;
                    let y = bounds.origin.y
                        + y_offset
                        + layout.cell_height * visual_y_row as f32
                        + header_offset;
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

                // Check if this row belongs to a block → add left padding
                let in_block = self.blocks.iter().any(|b| {
                    abs_row >= b.abs_start_row
                        && abs_row <= b.abs_end_row.unwrap_or(usize::MAX)
                });
                let text_x = if in_block {
                    bounds.origin.x + px(BLOCK_HEADER_PAD_X)
                } else {
                    bounds.origin.x
                };

                let row_y = bounds.origin.y
                    + y_offset
                    + layout.cell_height * visual_y_row as f32
                    + header_offset;
                let origin = point(text_x, row_y);
                lines.push((origin, shaped));

                // Update block body tracking: extend last_output_y
                for (bidx, _, last_y) in block_body_tracking.iter_mut() {
                    let block = &self.blocks[*bidx];
                    if abs_row >= block.abs_start_row
                        && abs_row <= block.abs_end_row.unwrap_or(usize::MAX)
                    {
                        *last_y = row_y + layout.cell_height;
                    }
                }
            }

            visual_y_row += 1;
        }

        // Finalize block body bounds
        for (bidx, header_y, last_y) in &block_body_tracking {
            let block = &self.blocks[*bidx];
            let is_error = block.exit_code.map_or(false, |c| c != 0);
            let is_selected = self.selected_block == Some(*bidx);
            // Add bottom padding to the block body
            let body_height = *last_y - *header_y + px(BLOCK_BODY_PAD_BOTTOM);
            if body_height > px(0.0) {
                let body_bounds = Bounds::new(
                    point(bounds.origin.x, *header_y),
                    size(bounds.size.width, body_height),
                );
                let left_border = if is_error {
                    Some(Bounds::new(
                        point(bounds.origin.x, *header_y),
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
        }

        TerminalPrepaint {
            lines,
            backgrounds,
            cursor_rect,
            line_height: layout.line_height,
            block_headers,
            block_bodies,
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
        });
    }
}

// ---------------------------------------------------------------------------
// Color helpers — Raijin Dark Theme
// ---------------------------------------------------------------------------

fn resolve_colors(
    cell: &raijin_term::term::cell::Cell,
    colors: &raijin_term::term::color::Colors,
) -> (Hsla, Hsla) {
    let fg = ansi_color_to_hsla(&cell.fg, colors, cell.flags.contains(CellFlags::DIM));
    let bg = ansi_color_to_hsla(&cell.bg, colors, false);
    (fg, bg)
}

fn ansi_color_to_hsla(
    color: &AnsiColor,
    colors: &raijin_term::term::color::Colors,
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
    // Raijin Dark palette
    let (r, g, b) = match name {
        NamedColor::Black => (0x12, 0x12, 0x12),
        NamedColor::Red => (0xff, 0x5f, 0x5f),
        NamedColor::Green => (0x14, 0xF1, 0x95),
        NamedColor::Yellow => (0xff, 0xd7, 0x00),
        NamedColor::Blue => (0x5f, 0x87, 0xff),
        NamedColor::Magenta => (0xd7, 0x5f, 0xff),
        NamedColor::Cyan => (0x00, 0xd7, 0xaf),
        NamedColor::White => (0xf1, 0xf1, 0xf1),
        NamedColor::BrightBlack => (0x66, 0x66, 0x66),
        NamedColor::BrightRed => (0xff, 0x5f, 0x5f),
        NamedColor::BrightGreen => (0x00, 0xff, 0x87),
        NamedColor::BrightYellow => (0xff, 0xff, 0x00),
        NamedColor::BrightBlue => (0x5c, 0x78, 0xff),
        NamedColor::BrightMagenta => (0xca, 0x1f, 0x7b),
        NamedColor::BrightCyan => (0x00, 0xd7, 0xff),
        NamedColor::BrightWhite => (0xff, 0xff, 0xff),
        NamedColor::Foreground => (0xf1, 0xf1, 0xf1),
        NamedColor::Background => (0x12, 0x12, 0x12),
        NamedColor::Cursor => (0x14, 0xF1, 0x95),
        _ => (0xf1, 0xf1, 0xf1),
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
