//! Block list view — stateful component for rendering and interacting with terminal blocks.
//!
//! Owns the block list rendering, scrollbar, text selection, and block selection.
//! This is the terminal's equivalent of Rio's Screen — a self-contained view that
//! handles both rendering and mouse interaction for the terminal output area.

use inazuma::{
    div, hsla, list, px,
    App, Context, Font, InteractiveElement, IntoElement, ListAlignment, ListState,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels,
    Point as GpuiPoint, Render, Styled, Window,
    prelude::FluentBuilder,
};
use inazuma_component::scroll::{Scrollbar, ScrollbarShow};
use raijin_term::block_grid::BlockId;
use raijin_term::index::{Column, Line, Point, Side};
use raijin_term::selection::SelectionType;
use raijin_terminal::TerminalHandle;

use super::block_element::{render_block, render_fold_line, render_fold_counter};
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

    // Fold area: show all fold-lines when counter is expanded
    fold_show_all: bool,
}

/// Cached layout info for one block — used for pixel→grid hit testing.
#[derive(Clone)]
struct BlockLayoutEntry {
    block_id: BlockId,
    block_index: usize,
    content_rows: usize,
    command_row_count: usize,
    grid_history_size: usize,
}

impl BlockListView {
    pub fn new(terminal: TerminalHandle) -> Self {
        Self {
            terminal,
            list_state: ListState::new(0, ListAlignment::Bottom, px(200.0)).measure_all(),
            snapshot_cache: BlockSnapshotCache::new(),
            selecting_block: None,
            mouse_down_pos: None,
            selected_block: None,
            block_layout: Vec::new(),
            fold_show_all: false,
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

                    let visual_row = (f32::from(y_in_block - header_height) / f32::from(cell_height)) as i32;
                    // Convert visual row to grid Line coordinate:
                    // visual_row 0 = command text, after that = grid history + screen.
                    // Grid Line(-history) = first history line, Line(0) = first screen line.
                    let row = visual_row - entry.command_row_count as i32 - entry.grid_history_size as i32;
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

    /// Find indices of all blocks fully scrolled above the viewport.
    ///
    /// Uses `logical_scroll_top()` which tracks the item at the viewport's top edge.
    /// All items before that index are fully above the viewport — no pixel bounds needed,
    /// works reliably with virtualized lists where off-screen items have no layout data.
    fn find_folded_block_indices(&self) -> Vec<usize> {
        let scroll_top = self.list_state.logical_scroll_top();
        // Items 0..scroll_top.item_ix are fully above the viewport
        (0..scroll_top.item_ix).collect()
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

        let theme = cx.global::<raijin_settings::ResolvedTheme>().clone();

        // Extract snapshots with single lock
        let snapshots = extract_all_block_snapshots(
            &self.terminal,
            &mut self.snapshot_cache,
            &symbol_maps,
            &theme,
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
                command_row_count: s.grid.command_row_count,
                grid_history_size: s.grid.grid_history_size,
            })
            .collect();

        let font = font.clone();
        let selected_block = self.selected_block;

        // --- Fold system: detect blocks fully scrolled above viewport ---
        // Suppress fold-lines when a full-screen app (vim, less, htop) is active.
        let is_alt_screen = {
            let term = self.terminal.lock();
            term.mode().contains(raijin_term::term::TermMode::ALT_SCREEN)
        };
        let folded_indices = if is_alt_screen {
            Vec::new()
        } else {
            self.find_folded_block_indices()
        };

        // Extract fold-line data BEFORE snapshots move into the list closure.
        // When fold_show_all is true, extract ALL folded blocks. Otherwise only last FOLD_MAX_VISIBLE.
        let fold_show_all = self.fold_show_all;
        let visible_fold_start = if fold_show_all { 0 } else { folded_indices.len().saturating_sub(FOLD_MAX_VISIBLE) };
        let fold_data: Vec<(usize, super::grid_snapshot::BlockHeaderSnapshot, String)> = folded_indices[visible_fold_start..]
            .iter()
            .map(|&ix| (ix, snapshots[ix].header.clone(), snapshots[ix].command.clone()))
            .collect();
        let fold_hidden_count = visible_fold_start;

        let list_theme = theme.clone();
        let block_list = list(self.list_state.clone(), move |ix, _window, _cx| {
            if let Some(snapshot) = snapshots.get(ix).cloned() {
                let is_selected = selected_block == Some(ix);
                render_block(snapshot, &font, font_size, line_height_multiplier, is_selected, &list_theme)
                    .into_any_element()
            } else {
                div().into_any_element()
            }
        });

        div()
            .id("block-list-container")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            // Fold area: compact fold-lines for blocks scrolled above viewport.
            .when(!folded_indices.is_empty(), |container| {
                let fold_theme = theme.clone();

                let mut fold_area = div()
                    .w_full()
                    .flex_shrink_0()
                    .flex_col()
                    .border_b_1()
                    .border_color(hsla(0.0, 0.0, 1.0, 0.12));

                if fold_hidden_count > 0 {
                    fold_area = fold_area.child(render_fold_counter(
                        fold_hidden_count,
                        &fold_theme,
                        cx.listener(|view, _ev, _win, _cx| {
                            view.fold_show_all = !view.fold_show_all;
                        }),
                    ));
                } else if fold_show_all && folded_indices.len() > FOLD_MAX_VISIBLE {
                    // Show "collapse" counter when all are expanded
                    fold_area = fold_area.child(render_fold_counter(
                        0,
                        &fold_theme,
                        cx.listener(|view, _ev, _win, _cx| {
                            view.fold_show_all = false;
                        }),
                    ));
                }

                for (ix, header, command) in &fold_data {
                    let ix = *ix;
                    fold_area = fold_area.child(render_fold_line(
                        header,
                        command,
                        ix,
                        &fold_theme,
                        cx.listener(move |view, _ev, _win, _cx| {
                            view.list_state.scroll_to_reveal_item(ix);
                        }),
                    ));
                }

                container.child(fold_area)
            })
            // List area fills remaining space, naturally clipped below sticky header
            .child(
                div()
                    .id("block-list-scroll-area")
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
                        move |view, event: &MouseUpEvent, window, cx| {
                            view.selecting_block.take();

                            // Detect click (no drag) vs drag selection
                            if let Some(down_pos) = view.mouse_down_pos.take() {
                                let dx = f32::from(event.position.x - down_pos.x).abs();
                                let dy = f32::from(event.position.y - down_pos.y).abs();
                                let is_click = dx < 3.0 && dy < 3.0;

                                if is_click {
                                    // Click without drag — clear text selection
                                    view.clear_all_selections();

                                    // Find which finished block was clicked → select it
                                    let (font, fs, lh, _) = Self::read_config(cx);
                                    let (cw, ch) = Self::cell_dimensions(&font, fs, lh, window);
                                    if let Some((block_idx, _, _, _)) = view.hit_test(event.position, cw, ch) {
                                        // Only select finished blocks
                                        let is_finished = {
                                            let handle = view.terminal.clone();
                                            let term = handle.lock();
                                            term.block_router().blocks()
                                                .get(block_idx)
                                                .is_some_and(|b| b.is_finished())
                                        };
                                        if is_finished {
                                            // Toggle: deselect if already selected
                                            if view.selected_block == Some(block_idx) {
                                                view.selected_block = None;
                                            } else {
                                                view.selected_block = Some(block_idx);
                                            }
                                        }
                                    } else {
                                        view.selected_block = None;
                                    }
                                }
                            }

                            cx.notify();
                        },
                    ))
                    .child(block_list.size_full())
                    .child(
                        Scrollbar::vertical(&self.list_state)
                            .scrollbar_show(ScrollbarShow::Always)
                    )
            )
    }
}
