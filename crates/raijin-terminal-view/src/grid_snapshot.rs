//! Grid snapshot extraction for efficient single-lock rendering.
//!
//! Locks the terminal ONCE, extracts all block grid data into snapshots,
//! drops the lock. Grid elements render from snapshots without locking.
//!
//! For finished blocks, snapshots are cached on the Workspace since their
//! grid content never changes. Only new/active blocks are freshly extracted.

use std::sync::Arc;

use inazuma::Oklch;
use raijin_term::block_grid::BlockId;
use raijin_term::grid::Dimensions;
use raijin_term::term::cell::Flags;
use raijin_terminal::TerminalHandle;

use crate::colors::resolve_colors;

/// Pre-extracted cell data — resolved colors, no layout positions.
#[derive(Clone)]
#[allow(dead_code)] // underline/strikeout needed once decoration rendering is added
pub struct SnapshotCell {
    pub c: char,
    pub zerowidth: Vec<char>,
    pub fg: Oklch,
    pub bg: Oklch,
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
    /// Number of prepended command text lines (offset for selection mapping).
    pub command_row_count: usize,
    /// History size from the underlying grid (for mapping snapshot indices to grid Line coordinates).
    pub grid_history_size: usize,
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
///
/// The grid is wrapped in `Arc` — finished blocks share grid data across frames
/// without deep-copying millions of cells. Only header/selection are cheaply cloned.
#[derive(Clone)]
pub struct BlockSnapshot {
    pub id: BlockId,
    /// Raw command text (for sticky header display).
    pub command: String,
    pub header: BlockHeaderSnapshot,
    pub grid: Arc<BlockGridSnapshot>,
    /// Active text selection within this block (if any).
    pub selection: Option<raijin_term::selection::SelectionRange>,
}

/// Cache for finished block snapshots.
///
/// Lives on the Workspace, persists across frames. Finished blocks never
/// change, so their snapshots are extracted once and reused forever.
/// Invalidated on terminal column resize (reflow changes line content).
/// Cache for finished block snapshots.
///
/// Stores `Arc<BlockGridSnapshot>` so that cloning a cached block for rendering
/// is O(1) (Arc increment) instead of O(n) (deep-copying millions of cells).
/// Header metadata is stored separately since it can change after finalization.
pub struct BlockSnapshotCache {
    entries: Vec<CachedBlock>,
    cached_cols: usize,
}

struct CachedBlock {
    id: BlockId,
    command: String,
    header: BlockHeaderSnapshot,
    grid: Arc<BlockGridSnapshot>,
}

impl BlockSnapshotCache {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            cached_cols: 0,
        }
    }

    fn get(&self, id: BlockId) -> Option<&CachedBlock> {
        self.entries.iter().find(|e| e.id == id)
    }

    fn insert(&mut self, snapshot: &BlockSnapshot) {
        let id = snapshot.id;
        let entry = CachedBlock {
            id,
            command: snapshot.command.clone(),
            header: snapshot.header.clone(),
            grid: Arc::clone(&snapshot.grid),
        };
        if let Some(pos) = self.entries.iter().position(|e| e.id == id) {
            self.entries[pos] = entry;
        } else {
            self.entries.push(entry);
        }
    }

    /// Remove cached entries for blocks that no longer exist.
    fn retain_existing(&mut self, existing_ids: &[BlockId]) {
        self.entries.retain(|e| existing_ids.contains(&e.id));
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
    theme: &raijin_theme::Theme,
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
        // Finished blocks: reuse cached Arc<BlockGridSnapshot> (O(1) clone).
        // Only header metadata and selection are freshly constructed (cheap).
        if block.is_finished() {
            if let Some(cached) = cache.get(block.id) {
                snapshots.push(BlockSnapshot {
                    id: block.id,
                    command: cached.command.clone(),
                    header: BlockHeaderSnapshot {
                        is_error: cached.header.is_error,
                        is_running: false,
                        started_at: cached.header.started_at,
                        finished_at: cached.header.finished_at,
                        duration_ms: block.metadata.duration_ms,
                        username: block.metadata.username.clone(),
                        hostname: block.metadata.hostname.clone(),
                        cwd: block.metadata.cwd.clone(),
                        git_branch: block.metadata.git_branch.clone(),
                    },
                    grid: Arc::clone(&cached.grid),
                    selection: block.selection_range(),
                });
                continue;
            }
        }

        // Extract fresh snapshot (active or first-time finished block)
        let snapshot = extract_single_block(block, colors, symbol_maps, theme);

        // Cache finished blocks
        if block.is_finished() {
            cache.insert(&snapshot);
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
    theme: &raijin_theme::Theme,
) -> BlockSnapshot {
    let grid = &block.grid;
    let history_size = grid.history_size();
    let screen_lines = grid.screen_lines() as i32;
    let grid_cols = grid.columns();
    let cursor_line = grid.cursor.point.line.0.max(0) as usize;
    // Use the block's computed content_rows if finalized (trims trailing empty lines),
    // otherwise fall back to cursor position for running blocks.
    // For finished blocks, content_rows already includes history + screen content.
    // For running blocks, use cursor position + history.
    let content_rows = if block.is_finished() {
        block.content_rows
    } else {
        history_size + cursor_line + 1
    };

    // Prepend command text as the first line(s) of the snapshot.
    // This makes the command selectable, uses the same font as output,
    // and preserves multi-line formatting.
    let command_fg = theme.styles.colors.text;
    let bg = theme.styles.colors.terminal_background;
    let mut command_lines: Vec<SnapshotLine> = Vec::new();
    if !block.command.is_empty() {
        for cmd_line in block.command.lines() {
            // Wrap long lines at grid_cols boundary
            let chars: Vec<char> = cmd_line.chars().collect();
            for chunk in chars.chunks(grid_cols.max(1)) {
                let cells: Vec<SnapshotCell> = chunk.iter().map(|&c| {
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

            let (mut fg, mut bg) = resolve_colors(cell, colors, theme);
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
        command: block.command.clone(),
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
        grid: Arc::new(BlockGridSnapshot {
            content_rows: command_row_count + content_rows,
            command_row_count,
            grid_history_size: history_size,
            grid_cols,
            lines,
        }),
        selection: block.selection_range(),
    }
}
