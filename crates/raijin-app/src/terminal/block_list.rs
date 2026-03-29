//! Block list view — stateful component for rendering and interacting with terminal blocks.
//!
//! Owns the block list rendering, scrollbar, text selection, and block selection.
//! This is the terminal's equivalent of Rio's Screen — a self-contained view that
//! handles both rendering and mouse interaction for the terminal output area.

use inazuma::{
    div, list, px,
    App, Context, Font, InteractiveElement, IntoElement, ListAlignment, ListState,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels,
    Point as GpuiPoint, Render, Styled, Window,
};
use inazuma_component::scroll::{Scrollbar, ScrollbarShow};
use raijin_term::block_grid::BlockId;
use raijin_term::index::{Column, Line, Point, Side};
use raijin_term::selection::SelectionType;
use raijin_terminal::TerminalHandle;

use super::block_element::render_block;
use super::constants::*;
use super::grid_snapshot::{BlockSnapshotCache, extract_all_block_snapshots};

/// Stateful block list view that owns rendering + mouse interaction.
pub struct BlockListView {
    terminal: TerminalHandle,
    list_state: ListState,
    snapshot_cache: BlockSnapshotCache,

    // Selection state
    selecting_block: Option<BlockId>,
    mouse_down_pos: Option<GpuiPoint<Pixels>>,
    selected_block: Option<usize>,

    // Cached block layout for pixel→grid conversion
    // Updated each frame during render
    block_layout: Vec<BlockLayoutEntry>,
}

/// Cached layout info for one block — used for pixel→grid hit testing.
#[derive(Clone)]
struct BlockLayoutEntry {
    block_id: BlockId,
    block_index: usize,
    content_rows: usize,
}

impl BlockListView {
    pub fn new(terminal: TerminalHandle) -> Self {
        Self {
            terminal,
            list_state: ListState::new(0, ListAlignment::Bottom, px(200.0)),
            snapshot_cache: BlockSnapshotCache::new(),
            selecting_block: None,
            mouse_down_pos: None,
            selected_block: None,
            block_layout: Vec::new(),
        }
    }

    /// Read current font/appearance config and build Font + dimensions.
    fn read_config(cx: &App) -> (Font, f32, f32, Vec<raijin_settings::ResolvedSymbolMap>) {
        let config = cx.global::<raijin_settings::RaijinConfig>();
        let font = Font {
            family: config.appearance.font_family.clone().into(),
            weight: inazuma::FontWeight::NORMAL,
            ..Font::default()
        };
        let symbol_maps = config.appearance.symbol_map
            .iter()
            .filter_map(|entry| entry.resolve())
            .collect();
        (font, config.appearance.font_size as f32, config.appearance.line_height as f32, symbol_maps)
    }

    /// Clear all blocks and reset cache.
    pub fn clear(&mut self) {
        let handle = self.terminal.clone();
        let mut term = handle.lock();
        term.block_router_mut().blocks_mut().clear();
        drop(term);
        self.snapshot_cache = BlockSnapshotCache::new();
        self.selected_block = None;
        self.selecting_block = None;
    }

    /// Get the selected block index.
    pub fn selected_block(&self) -> Option<usize> {
        self.selected_block
    }

    /// Set the selected block index.
    pub fn set_selected_block(&mut self, idx: Option<usize>) {
        self.selected_block = idx;
    }

    /// Copy the current text selection to a string (for Cmd+C).
    pub fn copy_selection_text(&self) -> Option<String> {
        let handle = self.terminal.clone();
        let term = handle.lock();
        for block in term.block_router().blocks() {
            if let Some(text) = block.selection_to_string() {
                return Some(text);
            }
        }
        None
    }

    /// Clear all selections on all blocks.
    fn clear_all_selections(&self) {
        let handle = self.terminal.clone();
        let mut term = handle.lock();
        for block in term.block_router_mut().blocks_mut() {
            block.clear_selection();
        }
    }

    /// Compute cell dimensions from font metrics using current config.
    fn cell_dimensions(font: &Font, font_size: f32, line_height_multiplier: f32, window: &mut Window) -> (Pixels, Pixels) {
        let font_size_px = px(font_size);
        let font_id = window.text_system().resolve_font(font);
        let cell_width = window
            .text_system()
            .advance(font_id, font_size_px, 'm')
            .expect("glyph not found for 'm'")
            .width;
        let ascent = window.text_system().ascent(font_id, font_size_px);
        let descent = window.text_system().descent(font_id, font_size_px);
        let base_height = ascent + descent.abs();
        let cell_height = base_height * line_height_multiplier;
        (cell_width, cell_height)
    }

    /// Convert a pixel position (relative to this view) to a (block_index, grid Point, Side).
    fn hit_test(
        &self,
        pos: GpuiPoint<Pixels>,
        cell_width: Pixels,
        cell_height: Pixels,
    ) -> Option<(usize, BlockId, Point, Side)> {
        // Use Inazuma's own layout calculation — bounds_for_item() returns the
        // rendered pixel bounds of each list item in window coordinates.
        for entry in &self.block_layout {
            if let Some(bounds) = self.list_state.bounds_for_item(entry.block_index) {
                if bounds.contains(&pos) {
                    // Compute actual header height from block bounds:
                    // block_height = header + grid, grid = content_rows * cell_height
                    let grid_height = cell_height * entry.content_rows as f32;
                    let header_height = bounds.size.height - grid_height;
                    let y_in_block = pos.y - bounds.origin.y;

                    // Header click — block selection, not text selection
                    if y_in_block < header_height {
                        return Some((entry.block_index, entry.block_id, Point::new(Line(0), Column(0)), Side::Left));
                    }

                    let row = (f32::from(y_in_block - header_height) / f32::from(cell_height)) as i32;
                    let col_f = (f32::from(pos.x) - BLOCK_HEADER_PAD_X) / f32::from(cell_width);
                    let col = col_f.max(0.0) as usize;
                    let side = if col_f.fract() < 0.5 { Side::Left } else { Side::Right };

                    return Some((
                        entry.block_index,
                        entry.block_id,
                        Point::new(Line(row), Column(col)),
                        side,
                    ));
                }
            }
        }

        None
    }

    /// Sync the ListState item count.
    fn sync_block_count(&self, block_count: usize) {
        let current_count = self.list_state.item_count();
        if block_count != current_count {
            if block_count > current_count {
                self.list_state.splice(current_count..current_count, block_count - current_count);
            } else {
                self.list_state.reset(block_count);
            }
        }
    }
}

impl Render for BlockListView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Read current config every frame (respects live config changes)
        let (font, font_size, line_height_multiplier, symbol_maps) = Self::read_config(cx);

        // Extract snapshots with single lock
        let snapshots = extract_all_block_snapshots(
            &self.terminal,
            &mut self.snapshot_cache,
            &symbol_maps,
        );
        self.sync_block_count(snapshots.len());

        // Cache block layout for hit testing
        self.block_layout = snapshots
            .iter()
            .enumerate()
            .map(|(i, s)| BlockLayoutEntry {
                block_id: s.id,
                block_index: i,
                content_rows: s.grid.content_rows,
            })
            .collect();

        let font = font.clone();
        let selected_block = self.selected_block;

        let block_list = list(self.list_state.clone(), move |ix, _window, _cx| {
            if let Some(snapshot) = snapshots.get(ix).cloned() {
                let is_selected = selected_block == Some(ix);
                render_block(snapshot, &font, font_size, line_height_multiplier, is_selected)
                    .into_any_element()
            } else {
                div().into_any_element()
            }
        });

        div()
            .id("block-list-container")
            .flex_1()
            .min_h_0()
            .relative()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, cx.listener(
                move |view, event: &MouseDownEvent, window, cx| {
                    let (font, fs, lh, _) = Self::read_config(cx);
                    let (cw, ch) = Self::cell_dimensions(&font, fs, lh, window);
                    view.mouse_down_pos = Some(event.position);

                    // Clear previous selections
                    view.clear_all_selections();
                    view.selected_block = None;

                    if let Some((block_idx, block_id, grid_point, side)) =
                        view.hit_test(event.position, cw, ch)
                    {
                        let handle = view.terminal.clone();
                        let mut term = handle.lock();
                        if let Some(block) = term.block_router_mut().blocks_mut().get_mut(block_idx) {
                            block.start_selection(SelectionType::Simple, grid_point, side);
                        }
                        drop(term);
                        view.selecting_block = Some(block_id);
                    }

                    cx.notify();
                },
            ))
            .on_mouse_move(cx.listener(
                move |view, event: &MouseMoveEvent, window, cx| {
                    if view.selecting_block.is_none() || event.pressed_button.is_none() {
                        return;
                    }

                    let (font, fs, lh, _) = Self::read_config(cx);
                    let (cw, ch) = Self::cell_dimensions(&font, fs, lh, window);
                    if let Some((block_idx, _block_id, grid_point, side)) =
                        view.hit_test(event.position, cw, ch)
                    {
                        let handle = view.terminal.clone();
                        let mut term = handle.lock();
                        if let Some(block) = term.block_router_mut().blocks_mut().get_mut(block_idx) {
                            block.update_selection(grid_point, side);
                        }
                        drop(term);
                        // Invalidate snapshot cache to pick up selection changes
                        view.snapshot_cache = BlockSnapshotCache::new();
                        cx.notify();
                    }
                },
            ))
            .on_mouse_up(MouseButton::Left, cx.listener(
                move |view, event: &MouseUpEvent, _window, cx| {
                    let was_selecting = view.selecting_block.take();

                    // Detect click (no drag) vs drag selection
                    if let Some(down_pos) = view.mouse_down_pos.take() {
                        let dx = f32::from(event.position.x - down_pos.x).abs();
                        let dy = f32::from(event.position.y - down_pos.y).abs();
                        let is_click = dx < 3.0 && dy < 3.0;

                        if is_click {
                            // Click without drag — toggle block selection
                            view.clear_all_selections();
                            // Find which block was clicked from the layout
                            // (simplified: use last block for now, will be refined)
                            if let Some(_) = was_selecting {
                                // Toggle: deselect if same, select if different
                                // For now just deselect any selection
                                view.selected_block = None;
                            }
                        }
                    }

                    cx.notify();
                },
            ))
            .child(block_list.size_full())
            .child(Scrollbar::vertical(&self.list_state).scrollbar_show(ScrollbarShow::Always))
    }
}
