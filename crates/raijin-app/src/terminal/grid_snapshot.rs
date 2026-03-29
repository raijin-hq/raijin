//! Grid snapshot extraction for efficient single-lock rendering.
//!
//! Locks the terminal ONCE, extracts all block grid data into snapshots,
//! drops the lock. Grid elements render from snapshots without locking.
//!
//! For finished blocks, snapshots are cached on the Workspace since their
//! grid content never changes. Only new/active blocks are freshly extracted.

use inazuma::Hsla;
use raijin_term::block_grid::BlockId;
use raijin_term::grid::Dimensions;
use raijin_term::term::cell::Flags;
use raijin_terminal::TerminalHandle;

use super::colors::resolve_colors;

/// Pre-extracted cell data — resolved colors, no layout positions.
#[derive(Clone)]
#[allow(dead_code)] // underline/strikeout needed once decoration rendering is added
pub struct SnapshotCell {
    pub c: char,
    pub zerowidth: Vec<char>,
    pub fg: Hsla,
    pub bg: Hsla,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikeout: bool,
    pub wide: bool,
    /// Override font family from symbol map (Nerd Fonts, Powerline).
    pub font_family_override: Option<String>,
}

/// Pre-extracted line data from a block's grid.
#[derive(Clone)]
pub struct SnapshotLine {
    pub cells: Vec<SnapshotCell>,
}

/// All data needed to render one block's grid content.
/// Extracted while holding the term lock, then used without locking.
#[derive(Clone)]
#[allow(dead_code)] // grid_cols needed for text selection
pub struct BlockGridSnapshot {
    pub content_rows: usize,
    pub grid_cols: usize,
    pub lines: Vec<SnapshotLine>,
}

/// Block metadata snapshot for the header.
#[derive(Clone)]
pub struct BlockHeaderSnapshot {
    pub is_error: bool,
    pub is_running: bool,
    pub started_at: std::time::Instant,
    pub finished_at: Option<std::time::Instant>,
    /// Shell-measured command duration in milliseconds (from shell hooks).
    /// More accurate than Instant-based measurement because it includes shell overhead.
    pub duration_ms: Option<u64>,
    pub username: Option<String>,
    pub hostname: Option<String>,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
}

/// Complete snapshot of one block (header + grid).
#[derive(Clone)]
pub struct BlockSnapshot {
    pub id: BlockId,
    pub header: BlockHeaderSnapshot,
    pub grid: BlockGridSnapshot,
    /// Active text selection within this block (if any).
    pub selection: Option<raijin_term::selection::SelectionRange>,
}

/// Cache for finished block snapshots.
///
/// Lives on the Workspace, persists across frames. Finished blocks never
/// change, so their snapshots are extracted once and reused forever.
/// Invalidated on terminal column resize (reflow changes line content).
pub struct BlockSnapshotCache {
    entries: Vec<(BlockId, BlockSnapshot)>,
    cached_cols: usize,
}

impl BlockSnapshotCache {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            cached_cols: 0,
        }
    }

    fn get(&self, id: BlockId) -> Option<&BlockSnapshot> {
        self.entries.iter().find(|(bid, _)| *bid == id).map(|(_, s)| s)
    }

    fn insert(&mut self, snapshot: BlockSnapshot) {
        let id = snapshot.id;
        if let Some(pos) = self.entries.iter().position(|(bid, _)| *bid == id) {
            self.entries[pos].1 = snapshot;
        } else {
            self.entries.push((id, snapshot));
        }
    }

    /// Remove cached entries for blocks that no longer exist.
    fn retain_existing(&mut self, existing_ids: &[BlockId]) {
        self.entries.retain(|(id, _)| existing_ids.contains(id));
    }
}

/// Extract snapshots for ALL blocks in a single lock.
///
/// Finished blocks are served from the cache. Only new/active blocks
/// are freshly extracted from the grid. Cache is invalidated when
/// terminal columns change (reflow).
pub fn extract_all_block_snapshots(
    handle: &TerminalHandle,
    cache: &mut BlockSnapshotCache,
    symbol_maps: &[raijin_settings::ResolvedSymbolMap],
) -> Vec<BlockSnapshot> {
    let term = handle.lock();
    let colors = term.colors();
    let blocks = term.block_router().blocks();
    let current_cols = if let Some(b) = blocks.first() {
        b.grid.columns()
    } else {
        80
    };

    // Invalidate cache on column change (reflow)
    if current_cols != cache.cached_cols {
        cache.entries.clear();
        cache.cached_cols = current_cols;
    }

    // Collect existing block IDs for cache cleanup
    let existing_ids: Vec<BlockId> = blocks.iter().map(|b| b.id).collect();
    cache.retain_existing(&existing_ids);

    let mut snapshots = Vec::with_capacity(blocks.len());

    for block in blocks {
        // Finished blocks: serve from cache if available, but update header
        // (metadata like duration_ms may arrive after the block is cached)
        if block.is_finished() {
            if let Some(cached) = cache.get(block.id) {
                let mut snapshot = cached.clone();
                // Refresh header fields that can change after finalization
                snapshot.header.duration_ms = block.metadata.duration_ms;
                snapshot.header.username = block.metadata.username.clone();
                snapshot.header.hostname = block.metadata.hostname.clone();
                snapshot.header.cwd = block.metadata.cwd.clone();
                snapshot.header.git_branch = block.metadata.git_branch.clone();
                snapshot.selection = block.selection_range();
                snapshots.push(snapshot);
                continue;
            }
        }

        // Extract fresh snapshot
        let snapshot = extract_single_block(block, colors, symbol_maps);

        // Cache finished blocks
        if block.is_finished() {
            cache.insert(snapshot.clone());
        }

        snapshots.push(snapshot);
    }

    drop(term);
    snapshots
}

/// Extract a single block's snapshot while the term lock is held.
fn extract_single_block(
    block: &raijin_term::block_grid::BlockGrid,
    colors: &raijin_term::term::color::Colors,
    symbol_maps: &[raijin_settings::ResolvedSymbolMap],
) -> BlockSnapshot {
    let grid = &block.grid;
    let history_size = grid.history_size();
    let screen_lines = grid.screen_lines() as i32;
    let grid_cols = grid.columns();
    let cursor_line = grid.cursor.point.line.0.max(0) as usize;
    // Use the block's computed content_rows if finalized (trims trailing empty lines),
    // otherwise fall back to cursor position for running blocks.
    let visible_rows = if block.is_finished() {
        block.content_rows
    } else {
        cursor_line + 1
    };
    let content_rows = history_size + visible_rows;

    // Prepend command text as the first line(s) of the snapshot.
    // This makes the command selectable, uses the same font as output,
    // and preserves multi-line formatting.
    let command_fg = super::constants::header_command_fg();
    let bg = super::constants::terminal_bg();
    let mut command_lines: Vec<SnapshotLine> = Vec::new();
    if !block.command.is_empty() {
        for cmd_line in block.command.lines() {
            let cells: Vec<SnapshotCell> = cmd_line.chars().map(|c| {
                SnapshotCell {
                    c,
                    zerowidth: vec![],
                    fg: command_fg,
                    bg,
                    bold: false,
                    italic: false,
                    underline: false,
                    strikeout: false,
                    wide: false,
                    font_family_override: None,
                }
            }).collect();
            command_lines.push(SnapshotLine { cells });
        }
    }
    let command_row_count = command_lines.len();

    let mut lines = Vec::with_capacity(command_row_count + content_rows);
    lines.extend(command_lines);

    for row_offset in 0..content_rows {
        let line_idx = row_offset as i32 - history_size as i32;
        let line = raijin_term::index::Line(line_idx);

        if line.0 >= screen_lines || line.0 < -(history_size as i32) {
            lines.push(SnapshotLine { cells: vec![] });
            continue;
        }

        let mut cells = Vec::with_capacity(grid_cols);
        let mut skip_next = false;

        for col_idx in 0..grid_cols {
            if skip_next {
                skip_next = false;
                continue;
            }

            let cell = &grid[line][raijin_term::index::Column(col_idx)];
            let flags = cell.flags;
            let wide = flags.contains(Flags::WIDE_CHAR);

            if wide {
                skip_next = true;
            }

            let (mut fg, mut bg) = resolve_colors(cell, colors);
            if flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
            }

            let zerowidth: Vec<char> = cell
                .zerowidth()
                .into_iter()
                .flatten()
                .copied()
                .collect();

            let ch = if cell.c == '\0' { ' ' } else { cell.c };
            let font_family_override = symbol_maps
                .iter()
                .find_map(|sm| sm.match_char(ch).map(|s| s.to_string()));

            cells.push(SnapshotCell {
                c: ch,
                zerowidth,
                fg,
                bg,
                bold: flags.contains(Flags::BOLD),
                italic: flags.contains(Flags::ITALIC),
                underline: flags.contains(Flags::UNDERLINE),
                strikeout: flags.contains(Flags::STRIKEOUT),
                wide,
                font_family_override,
            });
        }

        lines.push(SnapshotLine { cells });
    }

    BlockSnapshot {
        id: block.id,
        header: BlockHeaderSnapshot {
            is_error: block.is_error(),
            is_running: !block.is_finished(),
            started_at: block.started_at,
            finished_at: block.finished_at,
            duration_ms: block.metadata.duration_ms,
            username: block.metadata.username.clone(),
            hostname: block.metadata.hostname.clone(),
            cwd: block.metadata.cwd.clone(),
            git_branch: block.metadata.git_branch.clone(),
        },
        grid: BlockGridSnapshot {
            content_rows: command_row_count + content_rows,
            grid_cols,
            lines,
        },
        selection: block.selection_range(),
    }
}
