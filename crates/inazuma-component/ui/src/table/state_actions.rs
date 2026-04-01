use inazuma::{Context, MouseDownEvent, Pixels, Point, Window, px};

use crate::actions::{
    Cancel, SelectDown, SelectFirst, SelectLast, SelectNextColumn, SelectPageDown, SelectPageUp,
    SelectPrevColumn, SelectUp,
};

use super::*;

use super::state::{TableEvent, TableState};

impl<D> TableState<D>
where
    D: TableDelegate,
{
    pub(super) fn on_row_right_click(
        &mut self,
        _: &MouseDownEvent,
        row_ix: Option<usize>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.right_clicked_row = row_ix;
        self.right_clicked_cell = None;
        cx.emit(TableEvent::RightClickedRow(row_ix));
    }

    pub(super) fn on_cell_right_click(
        &mut self,
        _: &MouseDownEvent,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.cell_selectable {
            return;
        }

        cx.stop_propagation();
        self.right_clicked_cell = Some((row_ix, col_ix));
        self.right_clicked_row = None;
        cx.emit(TableEvent::RightClickedCell(row_ix, col_ix));
    }

    pub(super) fn on_row_left_click(
        &mut self,
        e: &inazuma::ClickEvent,
        row_ix: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.row_selectable {
            return;
        }

        self.set_selected_row(row_ix, cx);

        if e.click_count() == 2 {
            cx.emit(TableEvent::DoubleClickedRow(row_ix));
        }
    }

    pub(super) fn on_col_head_click(
        &mut self,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.col_selectable {
            return;
        }

        let Some(col_group) = self.col_groups.get(col_ix) else {
            return;
        };

        if !col_group.column.selectable {
            return;
        }

        self.set_selected_col(col_ix, cx)
    }

    pub(super) fn on_cell_click(
        &mut self,
        e: &inazuma::ClickEvent,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.cell_selectable {
            return;
        }

        cx.stop_propagation();
        self.set_selected_cell(row_ix, col_ix, cx);

        if e.click_count() == 2 {
            cx.emit(TableEvent::DoubleClickedCell(row_ix, col_ix));
        }
    }

    pub(super) fn action_cancel(
        &mut self,
        _: &Cancel,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.has_selection() {
            self.clear_selection(cx);
            return;
        }
        cx.propagate();
    }

    pub(super) fn action_select_prev(
        &mut self,
        _: &SelectUp,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rows_count = self.delegate.rows_count(cx);
        if rows_count < 1 {
            return;
        }

        // Cell selection mode: move up within the same column
        if self.selection_mode.is_cell() {
            if let Some((row_ix, col_ix)) = self.selected_cell {
                let new_row = if row_ix > 0 {
                    row_ix.saturating_sub(1)
                } else if self.loop_selection {
                    rows_count.saturating_sub(1)
                } else {
                    row_ix
                };
                self.set_selected_cell(new_row, col_ix, cx);
            } else {
                // No cell selected, select first cell
                self.set_selected_cell(0, 0, cx);
            }
            return;
        }

        // Row selection mode
        let mut selected_row = self.selected_row.unwrap_or(0);
        if selected_row > 0 {
            selected_row = selected_row.saturating_sub(1);
        } else {
            if self.loop_selection {
                selected_row = rows_count.saturating_sub(1);
            }
        }

        self.set_selected_row(selected_row, cx);
    }

    pub(super) fn action_select_next(
        &mut self,
        _: &SelectDown,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rows_count = self.delegate.rows_count(cx);
        if rows_count < 1 {
            return;
        }

        // Cell selection mode: move down within the same column
        if self.selection_mode.is_cell() {
            if let Some((row_ix, col_ix)) = self.selected_cell {
                let new_row = if row_ix < rows_count.saturating_sub(1) {
                    row_ix + 1
                } else if self.loop_selection {
                    0
                } else {
                    row_ix
                };
                self.set_selected_cell(new_row, col_ix, cx);
            } else {
                // No cell selected, select first cell
                self.set_selected_cell(0, 0, cx);
            }
            return;
        }

        // Row selection mode
        let selected_row = match self.selected_row {
            Some(selected_row) if selected_row < rows_count.saturating_sub(1) => selected_row + 1,
            Some(selected_row) => {
                if self.loop_selection {
                    0
                } else {
                    selected_row
                }
            }
            _ => 0,
        };

        self.set_selected_row(selected_row, cx);
    }

    pub(super) fn action_select_first_column(
        &mut self,
        _: &SelectFirst,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Cell selection mode: move to first cell in current row
        if self.selection_mode.is_cell() {
            if let Some((row_ix, _)) = self.selected_cell {
                self.set_selected_cell(row_ix, 0, cx);
            } else {
                // No cell selected, select first cell of first row
                self.set_selected_cell(0, 0, cx);
            }
            return;
        }

        // Column selection mode
        self.set_selected_col(0, cx);
    }

    pub(super) fn action_select_last_column(
        &mut self,
        _: &SelectLast,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let columns_count = self.delegate.columns_count(cx);

        // Cell selection mode: move to last cell in current row
        if self.selection_mode.is_cell() {
            if let Some((row_ix, _)) = self.selected_cell {
                self.set_selected_cell(row_ix, columns_count.saturating_sub(1), cx);
            } else {
                // No cell selected, select last cell of first row
                self.set_selected_cell(0, columns_count.saturating_sub(1), cx);
            }
            return;
        }

        // Column selection mode
        self.set_selected_col(columns_count.saturating_sub(1), cx);
    }

    pub(super) fn action_select_page_up(
        &mut self,
        _: &SelectPageUp,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let step = self.page_item_count();

        // Cell selection mode: move up by page within the same column
        if self.selection_mode.is_cell() {
            if let Some((row_ix, col_ix)) = self.selected_cell {
                let target = row_ix.saturating_sub(step);
                self.set_selected_cell(target, col_ix, cx);
            } else {
                // No cell selected, select first cell
                self.set_selected_cell(0, 0, cx);
            }
            return;
        }

        // Row selection mode
        let current = self.selected_row.unwrap_or(0);
        let target = current.saturating_sub(step);
        self.set_selected_row(target, cx);
    }

    pub(super) fn action_select_page_down(
        &mut self,
        _: &SelectPageDown,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rows_count = self.delegate.rows_count(cx);
        if rows_count == 0 {
            return;
        }

        let step = self.page_item_count();

        // Cell selection mode: move down by page within the same column
        if self.selection_mode.is_cell() {
            if let Some((row_ix, col_ix)) = self.selected_cell {
                let max_row = rows_count.saturating_sub(1);
                let target = (row_ix + step).min(max_row);
                self.set_selected_cell(target, col_ix, cx);
            } else {
                // No cell selected, select first cell
                self.set_selected_cell(0, 0, cx);
            }
            return;
        }

        // Row selection mode
        let current = self.selected_row.unwrap_or(0);
        let max_row = rows_count.saturating_sub(1);
        let target = (current + step).min(max_row);
        self.set_selected_row(target, cx);
    }

    pub(super) fn action_select_prev_col(
        &mut self,
        _: &SelectPrevColumn,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let columns_count = self.delegate.columns_count(cx);

        // Cell selection mode: move left within the same row
        if self.selection_mode.is_cell() {
            if let Some((row_ix, col_ix)) = self.selected_cell {
                let new_col = if col_ix > 0 {
                    col_ix.saturating_sub(1)
                } else if self.loop_selection {
                    columns_count.saturating_sub(1)
                } else {
                    col_ix
                };
                self.set_selected_cell(row_ix, new_col, cx);
            } else {
                // No cell selected, select first cell
                self.set_selected_cell(0, 0, cx);
            }
            return;
        }

        // Column selection mode
        let mut selected_col = self.selected_col.unwrap_or(0);
        if selected_col > 0 {
            selected_col = selected_col.saturating_sub(1);
        } else {
            if self.loop_selection {
                selected_col = columns_count.saturating_sub(1);
            }
        }
        self.set_selected_col(selected_col, cx);
    }

    pub(super) fn action_select_next_col(
        &mut self,
        _: &SelectNextColumn,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let columns_count = self.delegate.columns_count(cx);

        // Cell selection mode: move right within the same row
        if self.selection_mode.is_cell() {
            if let Some((row_ix, col_ix)) = self.selected_cell {
                let new_col = if col_ix < columns_count.saturating_sub(1) {
                    col_ix + 1
                } else if self.loop_selection {
                    0
                } else {
                    col_ix
                };
                self.set_selected_cell(row_ix, new_col, cx);
            } else {
                // No cell selected, select first cell
                self.set_selected_cell(0, 0, cx);
            }
            return;
        }

        // Column selection mode
        let mut selected_col = self.selected_col.unwrap_or(0);
        if selected_col < columns_count.saturating_sub(1) {
            selected_col += 1;
        } else {
            if self.loop_selection {
                selected_col = 0;
            }
        }

        self.set_selected_col(selected_col, cx);
    }

    /// Scroll table when mouse position is near the edge of the table bounds.
    pub(super) fn scroll_table_by_col_resizing(
        &mut self,
        mouse_position: Point<Pixels>,
        col_group: &ColGroup,
    ) {
        // Do nothing if pos out of the table bounds right for avoid scroll to the right.
        if mouse_position.x > self.bounds.right() {
            return;
        }

        let mut offset = self.horizontal_scroll_handle.offset();
        let col_bounds = col_group.bounds;

        if mouse_position.x < self.bounds.left()
            && col_bounds.right() < self.bounds.left() + px(20.)
        {
            offset.x += px(1.);
        } else if mouse_position.x > self.bounds.right()
            && col_bounds.right() > self.bounds.right() - px(20.)
        {
            offset.x -= px(1.);
        }

        self.horizontal_scroll_handle.set_offset(offset);
    }

    /// The `ix`` is the index of the col to resize,
    /// and the `size` is the new size for the col.
    pub(super) fn resize_cols(
        &mut self,
        ix: usize,
        size: Pixels,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.col_resizable {
            return;
        }

        let Some(col_group) = self.col_groups.get_mut(ix) else {
            return;
        };

        if !col_group.is_resizable() {
            return;
        }

        let new_width = size.clamp(col_group.column.min_width, col_group.column.max_width);

        // Only update if it actually changed
        if col_group.width != new_width {
            col_group.width = new_width;
            cx.notify();
        }
    }

    pub(super) fn perform_sort(
        &mut self,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.sortable {
            return;
        }

        let sort = self.col_groups.get(col_ix).and_then(|g| g.column.sort);
        if sort.is_none() {
            return;
        }

        let sort = sort.unwrap();
        let sort = match sort {
            ColumnSort::Ascending => ColumnSort::Default,
            ColumnSort::Descending => ColumnSort::Ascending,
            ColumnSort::Default => ColumnSort::Descending,
        };

        for (ix, col_group) in self.col_groups.iter_mut().enumerate() {
            if ix == col_ix {
                col_group.column.sort = Some(sort);
            } else {
                if col_group.column.sort.is_some() {
                    col_group.column.sort = Some(ColumnSort::Default);
                }
            }
        }

        self.delegate_mut().perform_sort(col_ix, sort, window, cx);

        cx.notify();
    }

    pub(super) fn move_column(
        &mut self,
        col_ix: usize,
        to_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if col_ix == to_ix {
            return;
        }

        self.delegate.move_column(col_ix, to_ix, window, cx);
        let col_group = self.col_groups.remove(col_ix);
        self.col_groups.insert(to_ix, col_group);

        cx.emit(TableEvent::MoveColumn(col_ix, to_ix));
        cx.notify();
    }

    /// Dispatch delegate's `load_more` method when the visible range is near the end.
    pub(super) fn load_more_if_need(
        &mut self,
        rows_count: usize,
        visible_end: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let threshold = self.delegate.load_more_threshold();
        // Securely handle subtract logic to prevent attempt to subtract with overflow
        if visible_end >= rows_count.saturating_sub(threshold) {
            if !self.delegate.has_more(cx) {
                return;
            }

            self._load_more_task = cx.spawn_in(window, async move |view, window| {
                _ = view.update_in(window, |view, window, cx| {
                    view.delegate.load_more(window, cx);
                });
            });
        }
    }

    pub(super) fn update_visible_range_if_need(
        &mut self,
        visible_range: std::ops::Range<usize>,
        axis: inazuma::Axis,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Skip when visible range is only 1 item.
        // The visual_list will use first item to measure.
        if visible_range.len() <= 1 {
            return;
        }

        if axis == inazuma::Axis::Vertical {
            if self.visible_range.rows == visible_range {
                return;
            }
            self.delegate_mut().visible_rows_changed(visible_range.clone(), window, cx);
            self.visible_range.rows = visible_range;
        } else {
            if self.visible_range.cols == visible_range {
                return;
            }
            self.delegate_mut().visible_columns_changed(visible_range.clone(), window, cx);
            self.visible_range.cols = visible_range;
        }
    }
}
