use std::{ops::Range, rc::Rc, time::Duration};

use crate::{
    ElementExt, VirtualListScrollHandle,
    h_flex,
    menu::{ContextMenuExt, PopupMenu},
    scroll::ScrollableMask,
    v_flex,
};
use inazuma::{
    Axis, Bounds, Context, EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ListSizingBehavior, ParentElement, Pixels, Render, ScrollStrategy, Styled, Task,
    UniformListScrollHandle, Window, div, prelude::FluentBuilder, uniform_list,
};

use super::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum SelectionMode {
    Column,
    Row,
    Cell,
}

impl SelectionMode {
    #[inline(always)]
    pub(super) fn is_row(&self) -> bool {
        matches!(self, SelectionMode::Row)
    }

    #[inline(always)]
    pub(super) fn is_column(&self) -> bool {
        matches!(self, SelectionMode::Column)
    }

    #[inline(always)]
    pub(super) fn is_cell(&self) -> bool {
        matches!(self, SelectionMode::Cell)
    }
}

/// The Table event.
#[derive(Clone)]
pub enum TableEvent {
    /// Single click or move to selected row.
    SelectRow(usize),
    /// Double click on the row.
    DoubleClickedRow(usize),
    /// Selected column.
    SelectColumn(usize),
    /// A cell has been selected (clicked or navigated to via keyboard).
    ///
    /// Emitted when a cell is selected in cell selection mode.
    /// The first `usize` is the row index, and the second `usize` is the column index.
    ///
    /// This event is also emitted when navigating between cells using keyboard shortcuts.
    SelectCell(usize, usize),
    /// A cell has been double-clicked.
    ///
    /// Emitted when a cell is double-clicked in cell selection mode.
    /// The first `usize` is the row index, and the second `usize` is the column index.
    ///
    /// Use this event to trigger actions like opening a detail view or editing the cell content.
    DoubleClickedCell(usize, usize),
    /// The column widths have changed.
    ///
    /// The `Vec<Pixels>` contains the new widths of all columns.
    ColumnWidthsChanged(Vec<Pixels>),
    /// A column has been moved.
    ///
    /// The first `usize` is the original index of the column,
    /// and the second `usize` is the new index of the column.
    MoveColumn(usize, usize),
    /// A row has been right-clicked.
    ///
    /// Contains the row index, or `None` if right-clicked on an empty area.
    /// Use this event to show context menus for rows.
    RightClickedRow(Option<usize>),
    /// A cell has been right-clicked.
    ///
    /// Emitted when a cell is right-clicked in cell selection mode.
    /// The first `usize` is the row index, and the second `usize` is the column index.
    ///
    /// Use this event to show context menus specific to the cell content.
    /// The right-clicked cell is highlighted with a subtle border until another cell is clicked.
    RightClickedCell(usize, usize),
    /// The selection has been cleared.
    ///
    /// This event is emitted when the selection is cleared.
    ClearSelection,
}

/// The visible range of the rows and columns.
#[derive(Debug, Default)]
pub struct TableVisibleRange {
    /// The visible range of the rows.
    pub(super) rows: Range<usize>,
    /// The visible range of the columns.
    pub(super) cols: Range<usize>,
}

impl TableVisibleRange {
    /// Returns the visible range of the rows.
    pub fn rows(&self) -> &Range<usize> {
        &self.rows
    }

    /// Returns the visible range of the columns.
    pub fn cols(&self) -> &Range<usize> {
        &self.cols
    }
}

/// The state for [`DataTable`].
///
/// # Selection Modes
///
/// The table supports three selection modes:
/// - **Row Selection**: Select entire rows (default mode)
/// - **Column Selection**: Select entire columns
/// - **Cell Selection**: Select individual cells
///
/// ## Cell Selection
///
/// When `cell_selectable` is enabled, users can:
/// - Click on cells to select them
/// - Right-click on cells to mark them for context menus
/// - Double-click on cells to trigger actions
/// - Navigate between cells using keyboard (arrow keys, Home, End, PageUp, PageDown, Tab)
///
/// When in cell selection mode, a row selector column appears on the left side,
/// allowing users to select entire rows by clicking on it.
///
/// # Events
///
/// The table emits the following events related to cell selection:
/// - [`TableEvent::SelectCell`]: Emitted when a cell is selected
/// - [`TableEvent::DoubleClickedCell`]: Emitted when a cell is double-clicked
/// - [`TableEvent::RightClickedCell`]: Emitted when a cell is right-clicked
///
/// # Example
///
/// ```rust,ignore
/// let table_state = cx.new(|cx| {
///     TableState::new(delegate, cx)
///         .cell_selectable(true)
///         .row_selectable(true)
/// });
///
/// // Subscribe to cell events
/// cx.subscribe(&table_state, |this, table, event, cx| {
///     match event {
///         TableEvent::SelectCell(row_ix, col_ix) => {
///             println!("Selected cell: ({}, {})", row_ix, col_ix);
///         }
///         TableEvent::DoubleClickedCell(row_ix, col_ix) => {
///             println!("Double-clicked cell: ({}, {})", row_ix, col_ix);
///         }
///         _ => {}
///     }
/// });
/// ```
pub struct TableState<D: TableDelegate> {
    pub(super) focus_handle: FocusHandle,
    pub(super) delegate: D,
    pub(super) options: TableOptions,
    /// The bounds of the table container.
    pub(super) bounds: Bounds<Pixels>,
    /// The bounds of the fixed head cols.
    pub(super) fixed_head_cols_bounds: Bounds<Pixels>,

    pub(super) col_groups: Vec<ColGroup>,

    /// Whether the table can loop selection, default is true.
    ///
    /// When the prev/next selection is out of the table bounds, the selection will loop to the other side.
    pub loop_selection: bool,
    /// Whether the table can select column.
    pub col_selectable: bool,
    /// Whether the table can select row.
    pub row_selectable: bool,
    /// Whether the table can select cell, default is true.
    ///
    /// When enabled:
    /// - Users can click on individual cells to select them
    /// - A row selector column appears on the left for selecting entire rows
    /// - Keyboard navigation works at the cell level (arrow keys move between cells)
    /// - Right-click and double-click events are supported for cells
    pub cell_selectable: bool,
    /// Whether the table can sort.
    pub sortable: bool,
    /// Whether the table can resize columns.
    pub col_resizable: bool,
    /// Whether the table can move columns.
    pub col_movable: bool,
    /// Enable/disable fixed columns feature.
    pub col_fixed: bool,

    pub vertical_scroll_handle: UniformListScrollHandle,
    pub horizontal_scroll_handle: VirtualListScrollHandle,

    pub(super) selected_row: Option<usize>,
    pub(super) selection_mode: SelectionMode,
    pub(super) right_clicked_row: Option<usize>,
    pub(super) right_clicked_cell: Option<(usize, usize)>,
    pub(super) selected_col: Option<usize>,
    pub(super) selected_cell: Option<(usize, usize)>,

    /// The column index that is being resized.
    pub(super) resizing_col: Option<usize>,

    /// The visible range of the rows and columns.
    pub(super) visible_range: TableVisibleRange,

    pub(super) _measure: Vec<Duration>,
    pub(super) _load_more_task: Task<()>,
}

impl<D> TableState<D>
where
    D: TableDelegate,
{
    /// Create a new TableState with the given delegate.
    pub fn new(delegate: D, _: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            focus_handle: cx.focus_handle().tab_stop(true),
            options: TableOptions::default(),
            delegate,
            col_groups: Vec::new(),
            horizontal_scroll_handle: VirtualListScrollHandle::new(),
            vertical_scroll_handle: UniformListScrollHandle::new(),
            selection_mode: SelectionMode::Row,
            selected_row: None,
            right_clicked_row: None,
            right_clicked_cell: None,
            selected_col: None,
            selected_cell: None,
            resizing_col: None,
            bounds: Bounds::default(),
            fixed_head_cols_bounds: Bounds::default(),
            visible_range: TableVisibleRange::default(),
            loop_selection: true,
            col_selectable: true,
            row_selectable: true,
            cell_selectable: false,
            sortable: true,
            col_movable: true,
            col_resizable: true,
            col_fixed: true,
            _load_more_task: Task::ready(()),
            _measure: Vec::new(),
        };

        this.prepare_col_groups(cx);
        this
    }

    /// Returns a reference to the delegate.
    pub fn delegate(&self) -> &D {
        &self.delegate
    }

    /// Returns a mutable reference to the delegate.
    pub fn delegate_mut(&mut self) -> &mut D {
        &mut self.delegate
    }

    /// Set to loop selection, default to true.
    pub fn loop_selection(mut self, loop_selection: bool) -> Self {
        self.loop_selection = loop_selection;
        self
    }

    /// Set to enable/disable column movable, default to true.
    pub fn col_movable(mut self, col_movable: bool) -> Self {
        self.col_movable = col_movable;
        self
    }

    /// Set to enable/disable column resizable, default to true.
    pub fn col_resizable(mut self, col_resizable: bool) -> Self {
        self.col_resizable = col_resizable;
        self
    }

    /// Set to enable/disable column sortable, default true
    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    /// Set to enable/disable row selectable, default true
    pub fn row_selectable(mut self, row_selectable: bool) -> Self {
        self.row_selectable = row_selectable;
        self
    }

    /// Set to enable/disable column selectable, default true
    pub fn col_selectable(mut self, col_selectable: bool) -> Self {
        self.col_selectable = col_selectable;
        self
    }

    /// Set to enable/disable cell selection, default is true.
    ///
    /// When enabled:
    /// - Individual cells become selectable by clicking
    /// - A row selector column appears on the left side
    /// - Keyboard navigation operates at the cell level
    /// - Cell-specific events (SelectCell, DoubleClickedCell, RightClickedCell) are emitted
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let table_state = cx.new(|cx| {
    ///     TableState::new(delegate, cx)
    ///         .cell_selectable(true)  // Enable cell selection
    ///         .row_selectable(true)   // Also allow row selection via row selector
    /// });
    /// ```
    pub fn cell_selectable(mut self, cell_selectable: bool) -> Self {
        self.cell_selectable = cell_selectable;
        self
    }

    /// When we update columns or rows, we need to refresh the table.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.prepare_col_groups(cx);
    }

    /// Scroll to the row at the given index.
    pub fn scroll_to_row(&mut self, row_ix: usize, cx: &mut Context<Self>) {
        self.vertical_scroll_handle.scroll_to_item(row_ix, ScrollStrategy::Top);
        cx.notify();
    }

    // Scroll to the column at the given index.
    pub fn scroll_to_col(&mut self, col_ix: usize, cx: &mut Context<Self>) {
        let col_ix = col_ix.saturating_sub(self.fixed_left_cols_count());

        self.horizontal_scroll_handle.scroll_to_item(col_ix, ScrollStrategy::Top);
        cx.notify();
    }

    /// Returns the selected row index.
    pub fn selected_row(&self) -> Option<usize> {
        self.selected_row
    }

    /// Sets the selected row to the given index.
    pub fn set_selected_row(&mut self, row_ix: usize, cx: &mut Context<Self>) {
        let is_down = match self.selected_row {
            Some(selected_row) => row_ix > selected_row,
            None => true,
        };

        cx.stop_propagation();
        self.selection_mode = SelectionMode::Row;
        self.right_clicked_row = None;
        self.selected_row = Some(row_ix);
        if let Some(row_ix) = self.selected_row {
            self.vertical_scroll_handle.scroll_to_item(
                row_ix,
                if is_down { ScrollStrategy::Bottom } else { ScrollStrategy::Top },
            );
        }
        cx.emit(TableEvent::SelectRow(row_ix));
        cx.emit(TableEvent::RightClickedRow(None));
        cx.notify();
    }

    /// Returns the row that has been right clicked.
    pub fn right_clicked_row(&self) -> Option<usize> {
        self.right_clicked_row
    }

    /// Returns the selected column index.
    pub fn selected_col(&self) -> Option<usize> {
        self.selected_col
    }

    /// Sets the selected col to the given index.
    pub fn set_selected_col(&mut self, col_ix: usize, cx: &mut Context<Self>) {
        self.selection_mode = SelectionMode::Column;
        self.selected_col = Some(col_ix);
        if let Some(col_ix) = self.selected_col {
            self.scroll_to_col(col_ix, cx);
        }
        cx.emit(TableEvent::SelectColumn(col_ix));
        cx.notify();
    }

    /// Returns the selected cell as `(row_ix, col_ix)`.
    ///
    /// Returns `None` if no cell is currently selected or if the table is in row/column selection mode.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if let Some((row_ix, col_ix)) = table_state.read(cx).selected_cell() {
    ///     println!("Selected cell: ({}, {})", row_ix, col_ix);
    /// }
    /// ```
    pub fn selected_cell(&self) -> Option<(usize, usize)> {
        self.selected_cell
    }

    /// Sets the selected cell to the given row and column indices.
    ///
    /// This method:
    /// - Switches the table to cell selection mode
    /// - Scrolls to make the cell visible (centered vertically)
    /// - Emits a [`TableEvent::SelectCell`] event
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Select the cell at row 5, column 3
    /// table_state.update(cx, |state, cx| {
    ///     state.set_selected_cell(5, 3, cx);
    /// });
    /// ```
    pub fn set_selected_cell(&mut self, row_ix: usize, col_ix: usize, cx: &mut Context<Self>) {
        self.selection_mode = SelectionMode::Cell;
        self.selected_cell = Some((row_ix, col_ix));

        // Scroll to the cell
        self.vertical_scroll_handle.scroll_to_item(row_ix, ScrollStrategy::Center);
        self.scroll_to_col(col_ix, cx);

        cx.emit(TableEvent::SelectCell(row_ix, col_ix));
        cx.notify();
    }

    /// Clear the selection of the table.
    pub fn clear_selection(&mut self, cx: &mut Context<Self>) {
        self.selection_mode = SelectionMode::Row;
        self.selected_row = None;
        self.selected_col = None;
        self.selected_cell = None;
        cx.emit(TableEvent::ClearSelection);
        cx.notify();
    }

    /// Returns the visible range of the rows and columns.
    ///
    /// See [`TableVisibleRange`].
    pub fn visible_range(&self) -> &TableVisibleRange {
        &self.visible_range
    }

    /// Dump table data.
    ///
    /// Returns a tuple of (headers, rows) where each row is a vector of cell values.
    pub fn dump(&self, cx: &App) -> (Vec<String>, Vec<Vec<String>>) {
        // Get header row
        let columns_count = self.delegate.columns_count(cx);
        let mut headers = Vec::with_capacity(columns_count);
        for col_ix in 0..columns_count {
            let column = self.delegate.column(col_ix, cx);
            headers.push(column.name.to_string());
        }

        // Get data rows
        let rows_count = self.delegate.rows_count(cx);
        let mut rows = Vec::with_capacity(rows_count);
        for row_ix in 0..rows_count {
            let mut row = Vec::with_capacity(columns_count);
            for col_ix in 0..columns_count {
                row.push(self.delegate.cell_text(row_ix, col_ix, cx));
            }
            rows.push(row);
        }

        (headers, rows)
    }

    pub(super) fn prepare_col_groups(&mut self, cx: &mut Context<Self>) {
        self.col_groups = (0..self.delegate.columns_count(cx))
            .map(|col_ix| {
                let column = self.delegate().column(col_ix, cx);
                ColGroup { width: column.width, bounds: Bounds::default(), column }
            })
            .collect();
        cx.notify();
    }

    pub(super) fn fixed_left_cols_count(&self) -> usize {
        if !self.col_fixed {
            return 0;
        }

        self.col_groups.iter().filter(|col| col.column.fixed == Some(ColumnFixed::Left)).count()
    }

    pub(super) fn page_item_count(&self) -> usize {
        let row_height = self.options.size.table_row_height();
        let height = self.bounds.size.height;
        let count = (height / row_height).floor() as usize;
        count.saturating_sub(1).max(1)
    }

    pub(super) fn has_selection(&self) -> bool {
        self.selected_row.is_some() || self.selected_col.is_some() || self.selected_cell.is_some()
    }
}

impl<D> Focusable for TableState<D>
where
    D: TableDelegate,
{
    fn focus_handle(&self, _cx: &inazuma::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl<D> EventEmitter<TableEvent> for TableState<D> where D: TableDelegate {}

impl<D> Render for TableState<D>
where
    D: TableDelegate,
{
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.measure(window, cx);

        let columns_count = self.delegate.columns_count(cx);
        let left_columns_count = self
            .col_groups
            .iter()
            .filter(|col| self.col_fixed && col.column.fixed == Some(ColumnFixed::Left))
            .count();
        let rows_count = self.delegate.rows_count(cx);
        let loading = self.delegate.loading(cx);

        let row_height = self.options.size.table_row_height();
        let total_height = self.vertical_scroll_handle.0.borrow().base_handle.bounds().size.height;
        let actual_height = row_height * rows_count as f32;
        let extra_rows_count =
            self.calculate_extra_rows_needed(total_height, actual_height, row_height);
        let render_rows_count =
            if self.options.stripe { rows_count + extra_rows_count } else { rows_count };
        let right_clicked_row = self.right_clicked_row;
        let is_filled = total_height > Pixels::ZERO && total_height <= actual_height;

        let loading_view = if loading {
            Some(self.delegate.render_loading(self.options.size, window, cx).into_any_element())
        } else {
            None
        };

        let empty_view = if rows_count == 0 {
            Some(div().size_full().child(self.delegate.render_empty(window, cx)).into_any_element())
        } else {
            None
        };

        let inner_table = v_flex()
            .id("table-inner")
            .size_full()
            .overflow_hidden()
            .child(self.render_table_header(left_columns_count, window, cx))
            .context_menu({
                let view = cx.entity().clone();
                move |this, window: &mut Window, cx: &mut Context<PopupMenu>| {
                    if let Some(row_ix) = view.read(cx).right_clicked_row {
                        view.update(cx, |menu, cx| {
                            menu.delegate_mut().context_menu(row_ix, this, window, cx)
                        })
                    } else {
                        this
                    }
                }
            })
            .map(|this| {
                if rows_count == 0 {
                    this.children(empty_view)
                } else {
                    this.child(
                        h_flex().id("table-body").flex_grow().size_full().child(
                            uniform_list(
                                "table-uniform-list",
                                render_rows_count,
                                cx.processor(
                                    move |table, visible_range: Range<usize>, window, cx| {
                                        // We must calculate the col sizes here, because the col sizes
                                        // need render_th first, then that method will set the bounds of each col.
                                        let col_sizes: Rc<Vec<inazuma::Size<Pixels>>> = Rc::new(
                                            table
                                                .col_groups
                                                .iter()
                                                .skip(left_columns_count)
                                                .map(|col| col.bounds.size)
                                                .collect(),
                                        );

                                        table.load_more_if_need(
                                            rows_count,
                                            visible_range.end,
                                            window,
                                            cx,
                                        );
                                        table.update_visible_range_if_need(
                                            visible_range.clone(),
                                            Axis::Vertical,
                                            window,
                                            cx,
                                        );

                                        if visible_range.end > rows_count {
                                            table.scroll_to_row(
                                                std::cmp::min(
                                                    visible_range.start,
                                                    rows_count.saturating_sub(1),
                                                ),
                                                cx,
                                            );
                                        }

                                        let mut items = Vec::with_capacity(
                                            visible_range.end.saturating_sub(visible_range.start),
                                        );

                                        // Render fake rows to fill the table
                                        visible_range.for_each(|row_ix| {
                                            // Render real rows for available data
                                            items.push(table.render_table_row(
                                                row_ix,
                                                rows_count,
                                                left_columns_count,
                                                col_sizes.clone(),
                                                columns_count,
                                                is_filled,
                                                window,
                                                cx,
                                            ));
                                        });

                                        items
                                    },
                                ),
                            )
                            .flex_grow()
                            .size_full()
                            .with_sizing_behavior(ListSizingBehavior::Auto)
                            .track_scroll(&self.vertical_scroll_handle)
                            .into_any_element(),
                        ),
                    )
                }
            });

        div()
            .size_full()
            .children(loading_view)
            .when(!loading, |this| {
                this.child(inner_table)
                    .child(ScrollableMask::new(Axis::Horizontal, &self.horizontal_scroll_handle))
                    .when(right_clicked_row.is_some(), |this| {
                        this.on_mouse_down_out(cx.listener(|this, e, window, cx| {
                            this.on_row_right_click(e, None, window, cx);
                            cx.notify();
                        }))
                    })
            })
            .on_prepaint({
                let state = cx.entity();
                move |bounds, _, cx| state.update(cx, |state, _| state.bounds = bounds)
            })
            .when(!window.is_inspector_picking(cx), |this| {
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .size_full()
                        .when(self.options.scrollbar_visible.bottom, |this| {
                            this.child(self.render_horizontal_scrollbar(window, cx))
                        })
                        .when(self.options.scrollbar_visible.right && rows_count > 0, |this| {
                            this.children(self.render_vertical_scrollbar(window, cx))
                        }),
                )
            })
    }
}
