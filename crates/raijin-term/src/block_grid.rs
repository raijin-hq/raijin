//! Block-based grid routing for Raijin's terminal emulation.
//!
//! Each command execution gets its own grid with independent cursor,
//! scroll region, and damage tracking. Mode, charset, tabs, and colors
//! are SHARED on the Terminal level (VT100 spec).
//!
//! BlockGrid is a display abstraction, not an independent terminal emulator.

use std::time::Instant;

use crate::grid::{Dimensions, Grid};
use crate::index::Line;
use crate::term::cell::Cell;

/// Unique identifier for a block grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

/// Per-block display state — only what varies between blocks.
///
/// Mode, charset, tabs, and colors are SHARED across all blocks on
/// the Terminal level (VT100 spec: one mode state per terminal instance).
/// Only the grid, cursor, scroll region, and damage are per-block.
#[derive(Debug, Clone)]
pub struct BlockDisplayState {
    /// Scroll region (top..bottom, indexed from viewport top).
    pub scroll_region: std::ops::Range<Line>,
}

impl BlockDisplayState {
    fn new(screen_lines: usize) -> Self {
        Self {
            scroll_region: Line(0)..Line(screen_lines as i32),
        }
    }
}

/// Metadata associated with a block.
#[derive(Debug, Clone, Default)]
pub struct BlockMetadata {
    pub cwd: Option<String>,
    pub username: Option<String>,
    pub hostname: Option<String>,
    pub git_branch: Option<String>,
    pub shell: Option<String>,
    pub duration_ms: Option<u64>,
}

/// A single block's grid with its own cursor, mode state, and metadata.
///
/// Each block is an independent terminal "session" that receives output
/// from one command execution. The grid owns its cursor (inside `Grid`),
/// and the mode state is stored alongside.
pub struct BlockGrid {
    pub id: BlockId,
    /// The terminal grid with cursor and scrollback.
    pub grid: Grid<Cell>,
    /// Per-block terminal mode state.
    pub display_state: BlockDisplayState,
    /// The command text that generated this block.
    pub command: String,
    /// Exit code (None while still running).
    pub exit_code: Option<i32>,
    /// Shell metadata snapshot at block creation time.
    pub metadata: BlockMetadata,
    /// When this block started.
    pub started_at: Instant,
    /// When this block finished (exit code received).
    pub finished_at: Option<Instant>,
    /// Whether synchronized rendering is active for this block.
    pub sync_rendering: bool,
}

impl BlockGrid {
    /// Create a new block grid with the given dimensions.
    fn new(id: BlockId, cols: usize, rows: usize, max_scrollback: usize) -> Self {
        let grid = Grid::new(rows, cols, max_scrollback); // lines, columns, scroll_limit
        Self {
            id,
            grid,
            display_state: BlockDisplayState::new(rows),
            command: String::new(),
            exit_code: None,
            metadata: BlockMetadata::default(),
            started_at: Instant::now(),
            finished_at: None,
            sync_rendering: false,
        }
    }

    /// Whether this block has finished executing.
    pub fn is_finished(&self) -> bool {
        self.exit_code.is_some()
    }

    /// Whether this block had a non-zero exit code.
    pub fn is_error(&self) -> bool {
        matches!(self.exit_code, Some(c) if c != 0)
    }

    // --- Grid Operations (Display Abstraction) ---
    //
    // These are pure grid+cursor operations that don't need shared terminal state.
    // They operate on the block's own grid and cursor.
    // Mode checks, damage tracking, and selection updates stay on Terminal.

    // --- Grid Operations (Display Abstraction) ---
    //
    // Pure grid+cursor operations. Mode checks, damage tracking, and
    // selection updates stay on Terminal.

    /// Move cursor to absolute position (clamped to grid bounds).
    pub fn move_cursor_to(&mut self, line: Line, col: crate::index::Column) {
        let max_line = Line(self.grid.screen_lines() as i32 - 1);
        let max_col = crate::index::Column(self.grid.columns() - 1);
        self.grid.cursor.point.line = std::cmp::min(line, max_line);
        self.grid.cursor.point.column = std::cmp::min(col, max_col);
        self.grid.cursor.input_needs_wrap = false;
    }

    /// Move cursor backward (left) by n columns, clamped to column 0.
    pub fn move_cursor_backward(&mut self, cols: usize) {
        let col = self.grid.cursor.point.column.0.saturating_sub(cols);
        self.grid.cursor.point.column = crate::index::Column(col);
        self.grid.cursor.input_needs_wrap = false;
    }

    /// Move cursor forward (right) by n columns, clamped to last column.
    pub fn move_cursor_forward(&mut self, cols: usize) {
        let last_col = self.grid.columns() - 1;
        let col = std::cmp::min(self.grid.cursor.point.column.0 + cols, last_col);
        self.grid.cursor.point.column = crate::index::Column(col);
        self.grid.cursor.input_needs_wrap = false;
    }

    /// Carriage return — move cursor to column 0.
    pub fn carriage_return(&mut self) {
        self.grid.cursor.point.column = crate::index::Column(0);
        self.grid.cursor.input_needs_wrap = false;
    }

    /// Backspace — move cursor left by 1 if not at column 0.
    pub fn backspace(&mut self) {
        if self.grid.cursor.point.column > crate::index::Column(0) {
            self.grid.cursor.point.column -= 1;
            self.grid.cursor.input_needs_wrap = false;
        }
    }

    /// Scroll the grid up within the scroll region.
    pub fn scroll_up(&mut self, region: &std::ops::Range<Line>, lines: usize) {
        self.grid.scroll_up(region, lines);
    }

    /// Scroll the grid down within the scroll region.
    pub fn scroll_down(&mut self, region: &std::ops::Range<Line>, lines: usize) {
        self.grid.scroll_down(region, lines);
    }
}

/// Routes VTE output to the appropriate block grid.
///
/// Manages the lifecycle of block grids:
/// - `prompt_grid`: catches prompt bytes (Starship etc.), never rendered
/// - Active block grid: receives live command output
/// - Finished block grids: immutable, only read for rendering
pub struct BlockGridRouter {
    /// All block grids (finished + active), in chronological order.
    pub(crate) blocks: Vec<BlockGrid>,
    /// ID of the currently active block (receiving output), or None.
    pub(crate) active_block_id: Option<BlockId>,
    /// Prompt grid: absorbs prompt output to keep VTE parser state consistent.
    /// Reset at each PromptStart. Never rendered.
    pub(crate) prompt_grid: BlockGrid,
    /// Command text to assign to the next block (set from UI on Enter).
    pending_command: Option<String>,
    /// Next block ID to assign.
    next_id: usize,
    /// Terminal column count (for creating new grids).
    cols: usize,
    /// Initial row count for new block grids.
    initial_rows: usize,
    /// Maximum scrollback rows per block grid.
    max_scrollback_per_block: usize,
    /// Maximum number of blocks to keep in memory.
    max_block_count: usize,
}

impl BlockGridRouter {
    /// Create a new router with the given terminal dimensions and scrollback.
    pub fn new(cols: usize, rows: usize, scrollback: usize) -> Self {
        let prompt_grid = BlockGrid::new(BlockId(0), cols, rows, scrollback);
        Self {
            blocks: Vec::new(),
            active_block_id: None,
            prompt_grid,
            pending_command: None,
            next_id: 1,
            cols,
            initial_rows: rows.max(24),
            max_scrollback_per_block: 10_000,
            max_block_count: 200,
        }
    }

    /// Set memory limits.
    pub fn set_limits(&mut self, max_blocks: usize, max_scrollback: usize) {
        self.max_block_count = max_blocks;
        self.max_scrollback_per_block = max_scrollback;
    }

    // --- Grid Routing ---

    /// Get a mutable reference to the currently active grid for writing.
    ///
    /// Returns the active block's grid if a command is running,
    /// otherwise returns the prompt grid.
    pub fn active_grid(&self) -> &Grid<Cell> {
        self.active_block_id
            .and_then(|id| self.blocks.iter().find(|b| b.id == id))
            .map_or(&self.prompt_grid.grid, |block| &block.grid)
    }

    pub fn active_grid_mut(&mut self) -> &mut Grid<Cell> {
        let id = self.active_block_id;
        if let Some(block) = id.and_then(|id| self.blocks.iter_mut().find(|b| b.id == id)) {
            &mut block.grid
        } else {
            &mut self.prompt_grid.grid
        }
    }

    /// Get the display state for the currently active grid.
    pub fn active_display_state(&self) -> &BlockDisplayState {
        self.active_block_id
            .and_then(|id| self.blocks.iter().find(|b| b.id == id))
            .map_or(&self.prompt_grid.display_state, |block| &block.display_state)
    }

    /// Get a mutable reference to the active display state.
    pub fn active_display_state_mut(&mut self) -> &mut BlockDisplayState {
        let id = self.active_block_id;
        if let Some(block) = id.and_then(|id| self.blocks.iter_mut().find(|b| b.id == id)) {
            &mut block.display_state
        } else {
            &mut self.prompt_grid.display_state
        }
    }

    /// Get the active block (if a command is running).
    pub fn active_block(&self) -> Option<&BlockGrid> {
        self.active_block_id
            .and_then(|id| self.blocks.iter().find(|b| b.id == id))
    }

    /// Get a mutable reference to the active block.
    pub fn active_block_mut(&mut self) -> Option<&mut BlockGrid> {
        let id = self.active_block_id?;
        self.blocks.iter_mut().find(|b| b.id == id)
    }

    /// Get the active block grid or the prompt grid (for BlockGrid-level methods).
    pub fn active_or_prompt(&self) -> &BlockGrid {
        self.active_block_id
            .and_then(|id| self.blocks.iter().find(|b| b.id == id))
            .unwrap_or(&self.prompt_grid)
    }

    /// Get the active block grid or the prompt grid (mutable).
    pub fn active_or_prompt_mut(&mut self) -> &mut BlockGrid {
        let id = self.active_block_id;
        if let Some(block) = id.and_then(|id| self.blocks.iter_mut().find(|b| b.id == id)) {
            block
        } else {
            &mut self.prompt_grid
        }
    }

    // --- Block Lifecycle ---

    /// Called at PromptStart: reset the prompt grid and route output there.
    pub fn switch_to_prompt(&mut self) {
        log::debug!("BlockGridRouter: switch_to_prompt (blocks={})", self.blocks.len());
        self.active_block_id = None;
        // Reset prompt grid — new prompt, fresh state
        let new_prompt = BlockGrid::new(BlockId(0), self.cols, self.initial_rows, 0);
        self.prompt_grid = new_prompt;
    }

    /// Called at CommandStart: create a new block grid and route output there.
    /// Uses pending_command if set (from UI Enter), otherwise uses the provided command.
    pub fn start_new_block(&mut self, command: String) -> BlockId {
        let id = BlockId(self.next_id);
        self.next_id += 1;

        // Use current prompt_grid dimensions (reflects latest resize)
        let current_rows = self.prompt_grid.grid.screen_lines().max(self.initial_rows);
        let current_cols = self.prompt_grid.grid.columns().max(self.cols);
        let mut block = BlockGrid::new(
            id,
            current_cols,
            current_rows,
            self.max_scrollback_per_block,
        );
        block.command = self.pending_command.take().unwrap_or(command);

        log::debug!(
            "BlockGridRouter: start_new_block id={:?}, cols={}, rows={}, cmd='{}'",
            id, self.cols, self.initial_rows, block.command
        );

        self.blocks.push(block);
        self.active_block_id = Some(id);

        // Evict oldest blocks if over limit
        self.evict_old_blocks();

        id
    }

    /// Called at CommandEnd: finalize the active block with an exit code.
    pub fn finalize_block(&mut self, exit_code: i32) {
        log::debug!("BlockGridRouter: finalize_block exit_code={}, active={:?}", exit_code, self.active_block_id);
        if let Some(block) = self.active_block_mut() {
            block.exit_code = Some(exit_code);
            block.finished_at = Some(Instant::now());
        }
        // Route subsequent bytes to prompt grid until next PromptStart
        self.active_block_id = None;
    }

    /// Set metadata on the most recent block (from OSC 7777).
    pub fn set_block_metadata(&mut self, metadata: BlockMetadata) {
        if let Some(block) = self.blocks.last_mut() {
            block.metadata = metadata;
        }
    }

    /// Set the command text that will be assigned to the next block.
    pub fn set_pending_command(&mut self, command: String) {
        self.pending_command = Some(command);
    }

    /// Set the command text on the active block.
    pub fn set_active_command(&mut self, command: String) {
        if let Some(block) = self.active_block_mut() {
            block.command = command;
        }
    }

    // --- Query ---

    /// All blocks in chronological order.
    pub fn blocks(&self) -> &[BlockGrid] {
        &self.blocks
    }

    /// Number of blocks.
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Whether a command is currently running.
    pub fn has_active_block(&self) -> bool {
        self.active_block_id.is_some()
    }

    /// Get a block by ID.
    pub fn block(&self, id: BlockId) -> Option<&BlockGrid> {
        self.blocks.iter().find(|b| b.id == id)
    }

    // --- Resize ---

    /// Resize all grids to new column count.
    /// Active grid is resized immediately; finished grids are resized lazily.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.cols = cols;
        self.initial_rows = rows.max(24);

        // Resize prompt grid
        self.prompt_grid.grid.resize(true, rows, cols);
        self.prompt_grid.display_state.scroll_region = Line(0)..Line(rows as i32);

        // Resize active block grid immediately
        if let Some(block) = self.active_block_mut() {
            let current_rows = block.grid.screen_lines();
            block.grid.resize(true, current_rows.max(rows), cols);
            block.display_state.scroll_region = Line(0)..Line(rows as i32);
        }

        // Finished blocks: only resize columns (lazy — rows stay as-is).
        // Full resize happens only when a block is actually rendered.
        for block in &mut self.blocks {
            if block.is_finished() {
                let current_cols = block.grid.columns();
                if current_cols != cols {
                    let current_rows = block.grid.screen_lines();
                    block.grid.resize(true, current_rows, cols);
                }
            }
        }
    }

    // --- Memory Management ---

    /// Drop oldest blocks if over the limit.
    fn evict_old_blocks(&mut self) {
        while self.blocks.len() > self.max_block_count {
            // Only evict finished blocks
            if let Some(pos) = self.blocks.iter().position(|b| b.is_finished()) {
                self.blocks.remove(pos);
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_lifecycle() {
        let mut router = BlockGridRouter::new(80, 24, 1000);

        // Initially no active block
        assert!(!router.has_active_block());
        assert_eq!(router.block_count(), 0);

        // PromptStart — prompt grid active
        router.switch_to_prompt();
        assert!(!router.has_active_block());

        // CommandStart — new block
        let _id = router.start_new_block("ls -la".into());
        assert!(router.has_active_block());
        assert_eq!(router.block_count(), 1);
        assert_eq!(router.active_block().unwrap().command, "ls -la");

        // CommandEnd — finalize block
        router.finalize_block(0);
        assert!(!router.has_active_block());
        assert!(router.blocks()[0].is_finished());
        assert!(!router.blocks()[0].is_error());

        // Another command with error
        router.switch_to_prompt();
        router.start_new_block("bad_cmd".into());
        router.finalize_block(127);
        assert_eq!(router.block_count(), 2);
        assert!(router.blocks()[1].is_error());
    }

    #[test]
    fn test_memory_eviction() {
        let mut router = BlockGridRouter::new(80, 24, 1000);
        router.set_limits(3, 1000);

        for i in 0..5 {
            router.start_new_block(format!("cmd_{}", i));
            router.finalize_block(0);
        }

        // Should have evicted oldest, keeping max 3
        assert_eq!(router.block_count(), 3);
        assert_eq!(router.blocks()[0].command, "cmd_2");
    }

    #[test]
    fn test_prompt_grid_routing() {
        let mut router = BlockGridRouter::new(80, 24, 1000);

        // When no command active, routing goes to prompt grid
        assert!(!router.has_active_block());
        // active_grid() returns prompt grid
        let _ = router.active_grid();

        // After starting a block, routing goes to block grid
        router.start_new_block("echo hi".into());
        assert!(router.has_active_block());

        // After finalizing, back to prompt grid
        router.finalize_block(0);
        assert!(!router.has_active_block());
    }
}
