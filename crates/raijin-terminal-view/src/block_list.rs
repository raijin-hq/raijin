//! Block list view — stateful component for rendering and interacting with terminal blocks.
//!
//! Owns the block list rendering, scrollbar, text selection, and block selection.
//! This is the terminal's equivalent of Rio's Screen — a self-contained view that
//! handles both rendering and mouse interaction for the terminal output area.

use inazuma::{
    div, Oklch, list, px,
    App, Context, Font, InteractiveElement, IntoElement, ListAlignment, ListState,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels,
    Point as GpuiPoint, Render, Styled, Window,
    prelude::FluentBuilder,
};
use raijin_ui::{Scrollbar, ScrollbarShow};
use raijin_term::block_grid::BlockId;
use raijin_term::index::{Column, Line, Point, Side};
use raijin_term::selection::SelectionType;
use raijin_terminal::TerminalHandle;

use crate::block_element::{render_block, render_fold_line, render_fold_counter};
use crate::constants::*;
use crate::grid_snapshot::{BlockSnapshotCache, extract_all_block_snapshots};

/// Stateful block list view that owns rendering + mouse interaction.
pub struct BlockListView {
    terminal: TerminalHandle,
    list_state: ListState,
    snapshot_cache: BlockSnapshotCache,

    // Selection state
    selecting_block: Option<BlockId>,
    mouse_down_pos: Option<GpuiPoint<Pixels>>,
    selected_block: Option<usize>,

    /// Pending single-click block toggle — cancelled if a double-click follows.
    /// Stores (block_index, timer_task). The task fires after 300ms to confirm
    /// it was a real single-click, not the first half of a double-click.
    pending_block_toggle: Option<(usize, inazuma::Task<()>)>,

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
    /// Shared store: TerminalGridElement writes actual grid origin Y during prepaint,
    /// hit_test reads it for exact pixel→row mapping without sub-pixel drift.
    grid_origin_store: crate::grid_element::GridOriginStore,
}

impl BlockListView {
    pub fn new(terminal: TerminalHandle) -> Self {
        Self {
            terminal,
            list_state: ListState::new(0, ListAlignment::Bottom, px(200.0)).measure_all(),
            snapshot_cache: BlockSnapshotCache::new(),
            selecting_block: None,
            mouse_down_pos: None,
            pending_block_toggle: None,
            selected_block: None,
            block_layout: Vec::new(),
            fold_show_all: false,
        }
    }

    /// Read current font/appearance config and build Font + dimensions.
    fn read_config(cx: &App) -> (Font, f32, f32, Vec<raijin_settings::ResolvedSymbolMap>) {
        let config = cx.global::<raijin_settings::RaijinSettings>();
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
                    // Use the ACTUAL grid origin from the last prepaint (exact, no drift).
                    // Falls back to bottom-up calculation if not yet painted.
                    let grid_height = cell_height * entry.content_rows as f32;
                    let grid_start_y = entry.grid_origin_store.get().unwrap_or_else(|| {
                        bounds.origin.y + bounds.size.height
                            - px(BLOCK_BODY_PAD_BOTTOM) - grid_height
                    });
                    let y_in_grid = pos.y - grid_start_y;

                    log::debug!(
                        "hit_test: pos.y={:.1} grid_origin={:.1} y_in_grid={:.1} cell_h={:.1} visual_row={}",
                        f32::from(pos.y), f32::from(grid_start_y), f32::from(y_in_grid),
                        f32::from(cell_height), (f32::from(y_in_grid) / f32::from(cell_height)) as i32,
                    );
                    // Click above grid = header area → block selection
                    if y_in_grid < px(0.0) {
                        return Some((entry.block_index, entry.block_id, Point::new(Line(0), Column(0)), Side::Left));
                    }

                    let visual_row = (f32::from(y_in_grid) / f32::from(cell_height)) as i32;
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

        let theme = raijin_theme::GlobalTheme::theme(cx).clone();

        // Extract snapshots with single lock
        let snapshots = extract_all_block_snapshots(
            &self.terminal,
            &mut self.snapshot_cache,
            &symbol_maps,
            &theme,
        );
        self.sync_block_count(snapshots.len());

        // Cache block layout for hit testing — each entry gets a shared GridOriginStore.
        // TerminalGridElement writes the actual origin Y during prepaint, hit_test reads it.
        self.block_layout = snapshots
            .iter()
            .enumerate()
            .map(|(i, s)| BlockLayoutEntry {
                block_id: s.id,
                block_index: i,
                content_rows: s.grid.content_rows,
                command_row_count: s.grid.command_row_count,
                grid_history_size: s.grid.grid_history_size,
                grid_origin_store: std::rc::Rc::new(std::cell::Cell::new(None)),
            })
            .collect();
        // Clone the Rc stores for the render_item closure
        let origin_stores: Vec<crate::grid_element::GridOriginStore> = self.block_layout
            .iter()
            .map(|e| e.grid_origin_store.clone())
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
        let fold_data: Vec<(usize, crate::grid_snapshot::BlockHeaderSnapshot, String)> = folded_indices[visible_fold_start..]
            .iter()
            .map(|&ix| (ix, snapshots[ix].header.clone(), snapshots[ix].command.clone()))
            .collect();
        let fold_hidden_count = visible_fold_start;

        let list_theme = theme.clone();
        let block_list = list(self.list_state.clone(), move |ix, _window, _cx| {
            if let Some(snapshot) = snapshots.get(ix).cloned() {
                let is_selected = selected_block == Some(ix);
                let store = origin_stores.get(ix).cloned();
                render_block(snapshot, &font, font_size, line_height_multiplier, is_selected, &list_theme, store)
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
                    .border_color(Oklch::white().opacity(0.12));

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

                            // Clear previous text selections (not block selection —
                            // that's toggled in mouse_up to allow deselect on re-click)
                            view.clear_all_selections();

                            // Double/triple click: cancel pending block-toggle from single-click
                            if event.click_count >= 2 {
                                view.pending_block_toggle = None;
                                view.selected_block = None;
                            }

                            if let Some((block_idx, block_id, grid_point, side)) =
                                view.hit_test(event.position, cw, ch)
                            {
                                // Selection type based on click count:
                                // 1 = simple drag selection
                                // 2 = word (semantic) selection
                                // 3 = line selection
                                let sel_type = match event.click_count {
                                    2 => SelectionType::Semantic,
                                    3 => SelectionType::Lines,
                                    _ => SelectionType::Simple,
                                };

                                let handle = view.terminal.clone();
                                let mut term = handle.lock();
                                if let Some(block) = term.block_router_mut().blocks_mut().get_mut(block_idx) {
                                    block.start_selection(sel_type, grid_point, side);
                                    // For semantic/lines, also set the end point immediately
                                    // so the selection is visible without requiring a drag.
                                    if matches!(sel_type, SelectionType::Semantic | SelectionType::Lines) {
                                        block.update_selection(grid_point, side);
                                    }
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
                                // Selection is read fresh each frame from BlockGrid.selection_range()
                                // in extract_all_block_snapshots — no cache invalidation needed.
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

                                if is_click && event.click_count <= 1 {
                                    // Defer block-toggle by 300ms — if a double-click
                                    // follows, the timer is cancelled in mouse_down.
                                    view.clear_all_selections();

                                    let (font, fs, lh, _) = Self::read_config(cx);
                                    let (cw, ch) = Self::cell_dimensions(&font, fs, lh, window);
                                    if let Some((block_idx, _, _, _)) = view.hit_test(event.position, cw, ch) {
                                        let is_finished = {
                                            let handle = view.terminal.clone();
                                            let term = handle.lock();
                                            term.block_router().blocks()
                                                .get(block_idx)
                                                .is_some_and(|b| b.is_finished())
                                        };
                                        if is_finished {
                                            let task = cx.spawn(async move |this, cx| {
                                                cx.background_executor().timer(std::time::Duration::from_millis(300)).await;
                                                if let Some(this) = this.upgrade() {
                                                    this.update(cx, |view, cx| {
                                                        // Only toggle if this pending task wasn't cancelled
                                                        if view.pending_block_toggle.as_ref()
                                                            .is_some_and(|(idx, _)| *idx == block_idx)
                                                        {
                                                            if view.selected_block == Some(block_idx) {
                                                                view.selected_block = None;
                                                            } else {
                                                                view.selected_block = Some(block_idx);
                                                            }
                                                            view.pending_block_toggle = None;
                                                            cx.notify();
                                                        }
                                                    });
                                                }
                                            });
                                            view.pending_block_toggle = Some((block_idx, task));
                                        }
                                    } else {
                                        view.selected_block = None;
                                        view.pending_block_toggle = None;
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
