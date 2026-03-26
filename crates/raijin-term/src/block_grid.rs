//! Block-based grid routing for Raijin's terminal emulation.
//!
//! Instead of a single global grid, each command execution gets its own
//! grid with independent cursor, mode state, and scrollback. The
//! `BlockGridRouter` manages the collection of block grids and routes
//! VTE handler calls to the currently active grid.

use std::sync::RwLock;
use std::time::Instant;

use crate::grid::{Dimensions, Grid};
use crate::index::Line;
use crate::term::cell::Cell;
use crate::term::TermMode;

/// Unique identifier for a block grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

/// Per-block terminal mode state that must be saved/restored when switching grids.
///
/// In the original alacritty_terminal, these fields live in `Term` and are global.
/// With grid-per-block, each block needs its own copy.
#[derive(Debug, Clone)]
pub struct BlockModeState {
    /// Terminal mode flags (insert, origin, line wrap, etc.)
    pub mode: TermMode,
    /// Active charset index.
    pub active_charset: vte::ansi::CharsetIndex,
    /// Scroll region (top..bottom, indexed from viewport top).
    pub scroll_region: std::ops::Range<Line>,
}

impl BlockModeState {
    fn new(screen_lines: usize) -> Self {
        Self {
            mode: TermMode::default(),
            active_charset: Default::default(),
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
    /// Wrapped in RwLock for concurrent read access from render thread
    /// while write thread updates the active block.
    pub grid: RwLock<Grid<Cell>>,
    /// Per-block terminal mode state.
    pub mode_state: BlockModeState,
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
            grid: RwLock::new(grid),
            mode_state: BlockModeState::new(rows),
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
}

/// Routes VTE output to the appropriate block grid.
///
/// Manages the lifecycle of block grids:
/// - `prompt_grid`: catches prompt bytes (Starship etc.), never rendered
/// - Active block grid: receives live command output
/// - Finished block grids: immutable, only read for rendering
pub struct BlockGridRouter {
    /// All block grids (finished + active), in chronological order.
    blocks: Vec<BlockGrid>,
    /// ID of the currently active block (receiving output), or None.
    active_block_id: Option<BlockId>,
    /// Prompt grid: absorbs prompt output to keep VTE parser state consistent.
    /// Reset at each PromptStart. Never rendered.
    prompt_grid: BlockGrid,
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
    /// Create a new router with the given terminal dimensions.
    pub fn new(cols: usize, rows: usize) -> Self {
        let prompt_grid = BlockGrid::new(BlockId(0), cols, rows, 0);
        Self {
            blocks: Vec::new(),
            active_block_id: None,
            prompt_grid,
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
    pub fn active_grid(&self) -> &RwLock<Grid<Cell>> {
        self.active_block_id
            .and_then(|id| self.blocks.iter().find(|b| b.id == id))
            .map_or(&self.prompt_grid.grid, |block| &block.grid)
    }

    /// Get the mode state for the currently active grid.
    pub fn active_mode_state(&self) -> &BlockModeState {
        self.active_block_id
            .and_then(|id| self.blocks.iter().find(|b| b.id == id))
            .map_or(&self.prompt_grid.mode_state, |block| &block.mode_state)
    }

    /// Get a mutable reference to the active mode state.
    pub fn active_mode_state_mut(&mut self) -> &mut BlockModeState {
        let id = self.active_block_id;
        if let Some(block) = id.and_then(|id| self.blocks.iter_mut().find(|b| b.id == id)) {
            &mut block.mode_state
        } else {
            &mut self.prompt_grid.mode_state
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

    // --- Block Lifecycle ---

    /// Called at PromptStart: reset the prompt grid and route output there.
    pub fn switch_to_prompt(&mut self) {
        self.active_block_id = None;
        // Reset prompt grid — new prompt, fresh state
        let new_prompt = BlockGrid::new(BlockId(0), self.cols, self.initial_rows, 0);
        self.prompt_grid = new_prompt;
    }

    /// Called at CommandStart: create a new block grid and route output there.
    pub fn start_new_block(&mut self, command: String) -> BlockId {
        let id = BlockId(self.next_id);
        self.next_id += 1;

        let mut block = BlockGrid::new(
            id,
            self.cols,
            self.initial_rows,
            self.max_scrollback_per_block,
        );
        block.command = command;

        self.blocks.push(block);
        self.active_block_id = Some(id);

        // Evict oldest blocks if over limit
        self.evict_old_blocks();

        id
    }

    /// Called at CommandEnd: finalize the active block with an exit code.
    pub fn finalize_block(&mut self, exit_code: i32) {
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
        if let Ok(mut grid) = self.prompt_grid.grid.write() {
            grid.resize(true, rows, cols);
        }
        self.prompt_grid.mode_state.scroll_region = Line(0)..Line(rows as i32);

        // Resize active block grid immediately
        if let Some(block) = self.active_block_mut() {
            if let Ok(mut grid) = block.grid.write() {
                let current_rows = grid.screen_lines();
                grid.resize(true, current_rows.max(rows), cols);
            }
            block.mode_state.scroll_region = Line(0)..Line(rows as i32);
        }

        // Finished blocks: mark for lazy resize (done when rendered)
        // For now, resize them all — can be optimized later
        for block in &self.blocks {
            if block.is_finished() && let Ok(mut grid) = block.grid.write() {
                let current_rows = grid.screen_lines();
                grid.resize(true, current_rows, cols);
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
        let mut router = BlockGridRouter::new(80, 24);

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
        let mut router = BlockGridRouter::new(80, 24);
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
        let mut router = BlockGridRouter::new(80, 24);

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
