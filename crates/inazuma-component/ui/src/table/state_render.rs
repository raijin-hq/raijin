use std::{rc::Rc, time::Duration};

use crate::{
    ActiveTheme, ElementExt, Icon, IconName, StyleSized as _, StyledExt,
    h_flex,
    scroll::Scrollbar,
};
use inazuma::{
    AppContext, Context, Div, DragMoveEvent, InteractiveElement, IntoElement, MouseButton, Pixels,
    ParentElement, SharedString, Stateful, StatefulInteractiveElement as _, Styled, Window, div,
    prelude::FluentBuilder, px,
};

use super::*;

use super::state::{TableEvent, TableState};

impl<D> TableState<D>
where
    D: TableDelegate,
{
    pub(super) fn render_cell(
        &self,
        _row_ix: Option<usize>,
        col_ix: usize,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Div {
        let Some(col_group) = self.col_groups.get(col_ix) else {
            return div();
        };

        let col_width = col_group.width;
        let col_padding = col_group.column.paddings;

        div()
            .w(col_width)
            .h_full()
            .flex_shrink_0()
            .overflow_hidden()
            .whitespace_nowrap()
            .table_cell_size(self.options.size)
            .map(|this| match col_padding {
                Some(padding) => {
                    this.pl(padding.left).pr(padding.right).pt(padding.top).pb(padding.bottom)
                }
                None => this,
            })
    }

    /// Show Column selection style, when the column is selected and the selection state is Column.
    /// Note: When a cell is selected, column selection style is not shown.
    pub(super) fn render_col_wrap(
        &self,
        _row_ix: Option<usize>,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let el = h_flex().h_full();
        let selectable = self.col_selectable
            && self
                .col_groups
                .get(col_ix)
                .map(|col_group| col_group.column.selectable)
                .unwrap_or(false);

        // Don't show column selection if a cell is selected
        if self.selection_mode.is_cell() {
            return el;
        }

        if selectable && self.selected_col == Some(col_ix) && self.selection_mode.is_column() {
            el.bg(cx.theme().table_active)
        } else {
            el
        }
    }

    pub(super) fn render_resize_handle(
        &self,
        ix: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        const HANDLE_SIZE: Pixels = px(2.);

        let resizable = self.col_resizable
            && self.col_groups.get(ix).map(|col| col.is_resizable()).unwrap_or(false);
        if !resizable {
            return div().into_any_element();
        }

        let group_id = SharedString::from(format!("resizable-handle:{}", ix));

        h_flex()
            .id(("resizable-handle", ix))
            .group(group_id.clone())
            .occlude()
            .cursor_col_resize()
            .h_full()
            .w(HANDLE_SIZE)
            .ml(-(HANDLE_SIZE))
            .justify_end()
            .items_center()
            .child(
                div()
                    .h_full()
                    .justify_center()
                    .bg(cx.theme().table_row_border)
                    .group_hover(&group_id, |this| this.bg(cx.theme().border).h_full())
                    .w(px(1.)),
            )
            .on_drag_move(cx.listener(move |view, e: &DragMoveEvent<ResizeColumn>, window, cx| {
                match e.drag(cx) {
                    ResizeColumn((entity_id, ix)) => {
                        if cx.entity_id() != *entity_id {
                            return;
                        }

                        // sync col widths into real widths
                        // TODO: Consider to remove this, this may not need now.
                        // for (_, col_group) in view.col_groups.iter_mut().enumerate() {
                        //     col_group.width = col_group.bounds.size.width;
                        // }

                        let ix = *ix;
                        view.resizing_col = Some(ix);

                        let col_group =
                            view.col_groups.get(ix).expect("BUG: invalid col index").clone();

                        view.resize_cols(
                            ix,
                            e.event.position.x - HANDLE_SIZE - col_group.bounds.left(),
                            window,
                            cx,
                        );

                        // scroll the table if the drag is near the edge
                        view.scroll_table_by_col_resizing(e.event.position, &col_group);
                    }
                };
            }))
            .on_drag(ResizeColumn((cx.entity_id(), ix)), |drag, _, _, cx| {
                cx.stop_propagation();
                cx.new(|_| drag.clone())
            })
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|view, _, _, cx| {
                    if view.resizing_col.is_none() {
                        return;
                    }

                    view.resizing_col = None;

                    let new_widths = view.col_groups.iter().map(|g| g.width).collect();
                    cx.emit(TableEvent::ColumnWidthsChanged(new_widths));
                    cx.notify();
                }),
            )
            .into_any_element()
    }

    /// Render the row selector cell (when cell_selectable is enabled)
    pub(super) fn render_row_selector_cell(
        &self,
        row_ix: usize,
        is_head: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id(("row-selector", row_ix))
            .w_3()
            .h_full()
            .border_r_1()
            .border_color(cx.theme().table_row_border)
            .bg(cx.theme().table_head)
            .flex_shrink_0()
            .table_cell_size(self.options.size)
            .when(!is_head, |this| {
                this.when(self.row_selectable, |this| {
                    this.on_click(cx.listener(move |table, _, _window, cx| {
                        table.set_selected_row(row_ix, cx);
                    }))
                })
            })
    }

    pub(super) fn render_sort_icon(
        &self,
        col_ix: usize,
        col_group: &ColGroup,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        if !self.sortable {
            return None;
        }

        let Some(sort) = col_group.column.sort else {
            return None;
        };

        let (icon, is_on) = match sort {
            ColumnSort::Ascending => (IconName::SortAscending, true),
            ColumnSort::Descending => (IconName::SortDescending, true),
            ColumnSort::Default => (IconName::ChevronsUpDown, false),
        };

        Some(
            div()
                .id(("icon-sort", col_ix))
                .p(px(2.))
                .rounded(cx.theme().radius / 2.)
                .map(|this| match is_on {
                    true => this,
                    false => this.opacity(0.5),
                })
                .hover(|this| this.bg(cx.theme().secondary).opacity(7.))
                .active(|this| this.bg(cx.theme().secondary_active).opacity(1.))
                .on_click(
                    cx.listener(move |table, _, window, cx| table.perform_sort(col_ix, window, cx)),
                )
                .child(Icon::new(icon).size_3().text_color(cx.theme().secondary_foreground)),
        )
    }

    /// Render the column header.
    /// The children must be one by one items.
    /// Because the horizontal scroll handle will use the child_item_bounds to
    /// calculate the item position for itself's `scroll_to_item` method.
    pub(super) fn render_th(
        &mut self,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let entity_id = cx.entity_id();
        let col_group = self.col_groups.get(col_ix).expect("BUG: invalid col index");

        let movable = self.col_movable && col_group.column.movable;
        let paddings = col_group.column.paddings;
        let name = col_group.column.name.clone();

        h_flex()
            .h_full()
            .child(
                self.render_cell(None, col_ix, window, cx)
                    .id(("col-header", col_ix))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.on_col_head_click(col_ix, window, cx);
                    }))
                    .child(
                        h_flex()
                            .size_full()
                            .justify_between()
                            .items_center()
                            .child(self.delegate.render_th(col_ix, window, cx))
                            .when_some(paddings, |this, paddings| {
                                // Leave right space for the sort icon, if this column have custom padding
                                let offset_pr =
                                    self.options.size.table_cell_padding().right - paddings.right;
                                this.pr(offset_pr.max(px(0.)))
                            })
                            .children(self.render_sort_icon(col_ix, &col_group, window, cx)),
                    )
                    .when(movable, |this| {
                        this.on_drag(
                            DragColumn { entity_id, col_ix, name, width: col_group.width },
                            |drag, _, _, cx| {
                                cx.stop_propagation();
                                cx.new(|_| drag.clone())
                            },
                        )
                        .drag_over::<DragColumn>(|this, _, _, cx| {
                            this.rounded_l_none()
                                .border_l_2()
                                .border_r_0()
                                .border_color(cx.theme().drag_border)
                        })
                        .on_drop(cx.listener(
                            move |table, drag: &DragColumn, window, cx| {
                                // If the drag col is not the same as the drop col, then swap the cols.
                                if drag.entity_id != cx.entity_id() {
                                    return;
                                }

                                table.move_column(drag.col_ix, col_ix, window, cx);
                            },
                        ))
                    }),
            )
            // resize handle
            .child(self.render_resize_handle(col_ix, window, cx))
            // to save the bounds of this col.
            .on_prepaint({
                let view = cx.entity().clone();
                move |bounds, _, cx| view.update(cx, |r, _| r.col_groups[col_ix].bounds = bounds)
            })
    }

    pub(super) fn render_table_header(
        &mut self,
        left_columns_count: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity().clone();
        let horizontal_scroll_handle = self.horizontal_scroll_handle.clone();

        // Reset fixed head columns bounds, if no fixed columns are present
        if left_columns_count == 0 {
            self.fixed_head_cols_bounds = inazuma::Bounds::default();
        }

        let mut header = self.delegate_mut().render_header(window, cx);
        let style = header.style().clone();

        header
            .h_flex()
            .w_full()
            .h(self.options.size.table_row_height())
            .flex_shrink_0()
            .border_b_1()
            .border_color(cx.theme().border)
            .text_color(cx.theme().table_head_foreground)
            .refine_style(&style)
            .when(self.cell_selectable, |this| {
                this.child(self.render_row_selector_cell(0, true, cx))
            })
            .when(left_columns_count > 0, |this| {
                let view = view.clone();
                // Render left fixed columns
                this.child(
                    h_flex()
                        .relative()
                        .h_full()
                        .bg(cx.theme().table_head)
                        .children(
                            self.col_groups
                                .clone()
                                .into_iter()
                                .filter(|col| col.column.fixed == Some(ColumnFixed::Left))
                                .enumerate()
                                .map(|(col_ix, _)| self.render_th(col_ix, window, cx)),
                        )
                        .child(
                            // Fixed columns border
                            div()
                                .absolute()
                                .top_0()
                                .right_0()
                                .bottom_0()
                                .w_0()
                                .flex_shrink_0()
                                .border_r_1()
                                .border_color(cx.theme().border),
                        )
                        .on_prepaint(move |bounds, _, cx| {
                            view.update(cx, |r, _| r.fixed_head_cols_bounds = bounds)
                        }),
                )
            })
            .child(
                // Columns
                h_flex()
                    .id("table-head")
                    .size_full()
                    .overflow_scroll()
                    .relative()
                    .track_scroll(&horizontal_scroll_handle)
                    .bg(cx.theme().table_head)
                    .child(
                        h_flex()
                            .relative()
                            .children(
                                self.col_groups
                                    .clone()
                                    .into_iter()
                                    .skip(left_columns_count)
                                    .enumerate()
                                    .map(|(col_ix, _)| {
                                        self.render_th(left_columns_count + col_ix, window, cx)
                                    }),
                            )
                            .child(self.delegate.render_last_empty_col(window, cx)),
                    ),
            )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_table_row(
        &mut self,
        row_ix: usize,
        rows_count: usize,
        left_columns_count: usize,
        col_sizes: Rc<Vec<inazuma::Size<Pixels>>>,
        columns_count: usize,
        is_filled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let horizontal_scroll_handle = self.horizontal_scroll_handle.clone();
        let is_stripe_row = self.options.stripe && !row_ix.is_multiple_of(2);
        let is_selected = self.selected_row == Some(row_ix);
        let view = cx.entity().clone();
        let row_height = self.options.size.table_row_height();

        if row_ix < rows_count {
            let is_last_row = row_ix + 1 == rows_count;
            let need_render_border = is_selected || !is_last_row || !is_filled;

            let mut tr = self.delegate.render_tr(row_ix, window, cx);
            let style = tr.style().clone();

            tr.h_flex()
                .w_full()
                .h(row_height)
                .when(need_render_border, |this| {
                    this.border_b_1().border_color(cx.theme().table_row_border)
                })
                .when(is_stripe_row, |this| this.bg(cx.theme().table_even))
                .refine_style(&style)
                .hover(|this| {
                    if is_selected || self.right_clicked_row == Some(row_ix) {
                        this
                    } else {
                        this.bg(cx.theme().table_hover)
                    }
                })
                .when(self.cell_selectable, |this| {
                    this.child(self.render_row_selector_cell(row_ix, false, cx))
                })
                .when(left_columns_count > 0, |this| {
                    // Left fixed columns
                    this.child(
                        h_flex()
                            .relative()
                            .h_full()
                            .children({
                                let mut items = Vec::with_capacity(left_columns_count);

                                (0..left_columns_count).for_each(|col_ix| {
                                    let is_cell_selected = self.selected_cell
                                        == Some((row_ix, col_ix))
                                        && self.selection_mode.is_cell();
                                    let is_cell_right_clicked =
                                        self.right_clicked_cell == Some((row_ix, col_ix));

                                    items.push(
                                        self.render_col_wrap(Some(row_ix), col_ix, window, cx)
                                            .child(
                                                self.render_cell(Some(row_ix), col_ix, window, cx)
                                                    .id(format!("table-cell:{}:{}", row_ix, col_ix))
                                                    .relative()
                                                    .child(self.measure_render_td(
                                                        row_ix, col_ix, window, cx,
                                                    ))
                                                    .when(is_cell_selected, |this| {
                                                        this.child(
                                                            div()
                                                                .absolute()
                                                                .inset_0()
                                                                .bg(cx.theme().table_active)
                                                                .border_1()
                                                                .border_color(
                                                                    cx.theme().table_active_border,
                                                                ),
                                                        )
                                                    })
                                                    .when(
                                                        is_cell_right_clicked && !is_cell_selected,
                                                        |this| {
                                                            this.child(
                                                                div()
                                                                    .absolute()
                                                                    .inset_0()
                                                                    .border_1()
                                                                    .border_color(
                                                                        cx.theme()
                                                                            .table_active_border
                                                                            .opacity(0.5),
                                                                    ),
                                                            )
                                                        },
                                                    )
                                                    .when(self.cell_selectable, |this| {
                                                        this.on_click(cx.listener(
                                                            move |table, e, window, cx| {
                                                                table.on_cell_click(
                                                                    e, row_ix, col_ix, window, cx,
                                                                );
                                                            },
                                                        ))
                                                        .on_mouse_down(
                                                            MouseButton::Right,
                                                            cx.listener(
                                                                move |table, e, window, cx| {
                                                                    table.on_cell_right_click(
                                                                        e, row_ix, col_ix, window,
                                                                        cx,
                                                                    );
                                                                },
                                                            ),
                                                        )
                                                    }),
                                            ),
                                    );
                                });

                                items
                            })
                            .child(
                                // Fixed columns border
                                div()
                                    .absolute()
                                    .top_0()
                                    .right_0()
                                    .bottom_0()
                                    .w_0()
                                    .flex_shrink_0()
                                    .border_r_1()
                                    .border_color(cx.theme().border),
                            ),
                    )
                })
                .child(
                    h_flex()
                        .flex_1()
                        .h_full()
                        .overflow_hidden()
                        .relative()
                        .child(
                            crate::virtual_list::virtual_list(
                                view,
                                row_ix,
                                inazuma::Axis::Horizontal,
                                col_sizes,
                                {
                                    move |table, visible_range: std::ops::Range<usize>, window, cx| {
                                        table.update_visible_range_if_need(
                                            visible_range.clone(),
                                            inazuma::Axis::Horizontal,
                                            window,
                                            cx,
                                        );

                                        let mut items = Vec::with_capacity(
                                            visible_range.end - visible_range.start,
                                        );

                                        visible_range.for_each(|col_ix| {
                                            let col_ix = col_ix + left_columns_count;
                                            let is_cell_selected = table.selected_cell
                                                == Some((row_ix, col_ix))
                                                && table.selection_mode.is_cell();
                                            let is_cell_right_clicked =
                                                table.right_clicked_cell == Some((row_ix, col_ix));

                                            let el = table
                                                .render_col_wrap(Some(row_ix), col_ix, window, cx)
                                                .child(
                                                    table
                                                        .render_cell(
                                                            Some(row_ix),
                                                            col_ix,
                                                            window,
                                                            cx,
                                                        )
                                                        .id(format!(
                                                            "table-cell-{}:{}",
                                                            row_ix, col_ix
                                                        ))
                                                        .relative()
                                                        .child(table.measure_render_td(
                                                            row_ix, col_ix, window, cx,
                                                        ))
                                                        .when(is_cell_selected, |this| {
                                                            this.child(
                                                                div()
                                                                    .absolute()
                                                                    .inset_0()
                                                                    .bg(cx.theme().table_active)
                                                                    .border_1()
                                                                    .border_color(
                                                                        cx.theme()
                                                                            .table_active_border,
                                                                    ),
                                                            )
                                                        })
                                                        .when(
                                                            is_cell_right_clicked
                                                                && !is_cell_selected,
                                                            |this| {
                                                                this.child(
                                                                    div()
                                                                        .absolute()
                                                                        .inset_0()
                                                                        .border_1()
                                                                        .border_color(
                                                                            cx.theme()
                                                                                .table_active_border
                                                                                .opacity(0.5),
                                                                        ),
                                                                )
                                                            },
                                                        )
                                                        .when(table.cell_selectable, |this| {
                                                            this.on_click(cx.listener(
                                                                move |table, e, window, cx| {
                                                                    cx.stop_propagation();
                                                                    table.on_cell_click(
                                                                        e, row_ix, col_ix, window,
                                                                        cx,
                                                                    );
                                                                },
                                                            ))
                                                            .on_mouse_down(
                                                                MouseButton::Right,
                                                                cx.listener(
                                                                    move |table, e, window, cx| {
                                                                        table.on_cell_right_click(
                                                                            e, row_ix, col_ix,
                                                                            window, cx,
                                                                        );
                                                                    },
                                                                ),
                                                            )
                                                        }),
                                                );

                                            items.push(el);
                                        });

                                        items
                                    }
                                },
                            )
                            .with_scroll_handle(&self.horizontal_scroll_handle),
                        )
                        .child(self.delegate.render_last_empty_col(window, cx)),
                )
                // Row selected style
                // Note: Don't show row selection if a cell is selected
                .when_some(self.selected_row, |this, _| {
                    this.when(is_selected && self.selection_mode.is_row(), |this| {
                        this.map(|this| {
                            if cx.theme().list.active_highlight {
                                this.border_color(inazuma::transparent_white()).child(
                                    div()
                                        .top(if row_ix == 0 { px(0.) } else { px(-1.) })
                                        .left(px(0.))
                                        .right(px(0.))
                                        .bottom(px(-1.))
                                        .absolute()
                                        .bg(cx.theme().table_active)
                                        .border_1()
                                        .border_color(cx.theme().table_active_border),
                                )
                            } else {
                                this.bg(cx.theme().accent)
                            }
                        })
                    })
                })
                // Row right click row style
                .when(self.right_clicked_row == Some(row_ix), |this| {
                    this.border_color(inazuma::transparent_white()).child(
                        div()
                            .top(if row_ix == 0 { px(0.) } else { px(-1.) })
                            .left(px(0.))
                            .right(px(0.))
                            .bottom(px(-1.))
                            .absolute()
                            .border_1()
                            .border_color(cx.theme().selection),
                    )
                })
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, e, window, cx| {
                        this.on_row_right_click(e, Some(row_ix), window, cx);
                    }),
                )
                .on_click(cx.listener(move |this, e, window, cx| {
                    this.on_row_left_click(e, row_ix, window, cx);
                }))
        } else {
            // Render fake rows to fill the rest table space
            self.delegate
                .render_tr(row_ix, window, cx)
                .h_flex()
                .w_full()
                .h(row_height)
                .border_b_1()
                .border_color(cx.theme().table_row_border)
                .when(is_stripe_row, |this| this.bg(cx.theme().table_even))
                .when(self.cell_selectable, |this| {
                    // Render empty row selector cell for fake rows
                    this.child(
                        div()
                            .w(px(40.))
                            .h_full()
                            .flex_shrink_0()
                            .table_cell_size(self.options.size),
                    )
                })
                .children((0..columns_count).map(|col_ix| {
                    h_flex()
                        .left(horizontal_scroll_handle.offset().x)
                        .child(self.render_cell(None, col_ix, window, cx))
                }))
                .child(self.delegate.render_last_empty_col(window, cx))
        }
    }

    /// Calculate the extra rows needed to fill the table empty space when `stripe` is true.
    pub(super) fn calculate_extra_rows_needed(
        &self,
        total_height: Pixels,
        actual_height: Pixels,
        row_height: Pixels,
    ) -> usize {
        let mut extra_rows_needed = 0;

        let remaining_height = total_height - actual_height;
        if remaining_height > px(0.) {
            extra_rows_needed = (remaining_height / row_height).floor() as usize;
        }

        extra_rows_needed
    }

    #[inline]
    pub(super) fn measure_render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if !crate::measure_enable() {
            return self.delegate.render_td(row_ix, col_ix, window, cx).into_any_element();
        }

        let start = std::time::Instant::now();
        let el = self.delegate.render_td(row_ix, col_ix, window, cx);
        self._measure.push(start.elapsed());
        el.into_any_element()
    }

    pub(super) fn measure(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        if !crate::measure_enable() {
            return;
        }

        // Print avg measure time of each td
        if self._measure.len() > 0 {
            let total = self._measure.iter().fold(Duration::default(), |acc, d| acc + *d);
            let avg = total / self._measure.len() as u32;
            eprintln!(
                "last render {} cells total: {:?}, avg: {:?}",
                self._measure.len(),
                total,
                avg,
            );
        }
        self._measure.clear();
    }

    pub(super) fn render_vertical_scrollbar(
        &mut self,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        Some(
            div()
                .absolute()
                .top(self.options.size.table_row_height())
                .right_0()
                .bottom_0()
                .w(Scrollbar::width())
                .child(Scrollbar::vertical(&self.vertical_scroll_handle).max_fps(60)),
        )
    }

    pub(super) fn render_horizontal_scrollbar(
        &mut self,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .absolute()
            .left(self.fixed_head_cols_bounds.size.width)
            .right_0()
            .bottom_0()
            .h(Scrollbar::width())
            .child(Scrollbar::horizontal(&self.horizontal_scroll_handle))
    }
}
