use crate::{
    ActiveTheme, ElementExt, Icon, IconName, h_flex,
    scroll::Scrollbar,
    v_flex,
};

use super::PanelEvent;
use inazuma::{
    AnyElement, App, AppContext, Context, DismissEvent, Div, DragMoveEvent, EntityId,
    EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, MouseUpEvent, ParentElement, Pixels, Point, Render, Size,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px, size,
};

use super::tiles::{
    AnyDrag, DragDrop, DragMoving, DragResizing, ResizeDrag, ResizeSide, TileChange,
    DRAG_BAR_HEIGHT, HANDLE_SIZE, MINIMUM_SIZE,
    round_point_to_nearest_ten,
};
use super::Tiles;

impl Tiles {
    /// Produce a vector of AnyElement representing the three possible resize handles
    pub(super) fn render_resize_handles(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
        entity_id: EntityId,
        item: &super::tiles::TileItem,
    ) -> Vec<AnyElement> {
        let item_id = item.id;
        let item_bounds = item.bounds;
        let handle_offset = -HANDLE_SIZE + px(1.);

        let mut elements = Vec::new();

        // Left resize handle
        elements.push(
            div()
                .id("left-resize-handle")
                .cursor_ew_resize()
                .absolute()
                .top_0()
                .left(handle_offset)
                .w(HANDLE_SIZE)
                .h(item_bounds.size.height)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener({
                        move |this, event: &MouseDownEvent, window, cx| {
                            this.on_resize_handle_mouse_down(
                                ResizeSide::Left,
                                item_id,
                                item_bounds,
                                event,
                                window,
                                cx,
                            );
                        }
                    }),
                )
                .on_drag(DragResizing(entity_id), |drag, _, _, cx| {
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .on_drag_move(cx.listener(
                    move |this, e: &DragMoveEvent<DragResizing>, window, cx| match e.drag(cx) {
                        DragResizing(id) => {
                            if *id != entity_id {
                                return;
                            }

                            let Some(ref drag_data) = this.resizing_drag_data else {
                                return;
                            };
                            if drag_data.side != ResizeSide::Left {
                                return;
                            }

                            let pos = e.event.position;
                            let delta = drag_data.last_position.x - pos.x;
                            let new_x = (drag_data.last_bounds.origin.x - delta).max(px(0.0));
                            let size_delta = drag_data.last_bounds.origin.x - new_x;
                            let new_width = (drag_data.last_bounds.size.width + size_delta)
                                .max(MINIMUM_SIZE.width);
                            this.resize(Some(new_x), None, Some(new_width), None, window, cx);
                        }
                    },
                ))
                .into_any_element(),
        );

        // Right resize handle
        elements.push(
            div()
                .id("right-resize-handle")
                .cursor_ew_resize()
                .absolute()
                .top_0()
                .right(handle_offset)
                .w(HANDLE_SIZE)
                .h(item_bounds.size.height)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener({
                        move |this, event: &MouseDownEvent, window, cx| {
                            this.on_resize_handle_mouse_down(
                                ResizeSide::Right,
                                item_id,
                                item_bounds,
                                event,
                                window,
                                cx,
                            );
                        }
                    }),
                )
                .on_drag(DragResizing(entity_id), |drag, _, _, cx| {
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .on_drag_move(cx.listener(
                    move |this, e: &DragMoveEvent<DragResizing>, window, cx| match e.drag(cx) {
                        DragResizing(id) => {
                            if *id != entity_id {
                                return;
                            }

                            let Some(ref drag_data) = this.resizing_drag_data else {
                                return;
                            };

                            if drag_data.side != ResizeSide::Right {
                                return;
                            }

                            let pos = e.event.position;
                            let delta = pos.x - drag_data.last_position.x;
                            let new_width =
                                (drag_data.last_bounds.size.width + delta).max(MINIMUM_SIZE.width);
                            this.resize(None, None, Some(new_width), None, window, cx);
                        }
                    },
                ))
                .into_any_element(),
        );

        // Top resize handle
        elements.push(
            div()
                .id("top-resize-handle")
                .cursor_ns_resize()
                .absolute()
                .left(px(0.0))
                .top(handle_offset)
                .w(item_bounds.size.width)
                .h(HANDLE_SIZE)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener({
                        move |this, event: &MouseDownEvent, window, cx| {
                            this.on_resize_handle_mouse_down(
                                ResizeSide::Top,
                                item_id,
                                item_bounds,
                                event,
                                window,
                                cx,
                            );
                        }
                    }),
                )
                .on_drag(DragResizing(entity_id), |drag, _, _, cx| {
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .on_drag_move(cx.listener(
                    move |this, e: &DragMoveEvent<DragResizing>, window, cx| match e.drag(cx) {
                        DragResizing(id) => {
                            if *id != entity_id {
                                return;
                            }

                            let Some(ref drag_data) = this.resizing_drag_data else {
                                return;
                            };

                            if drag_data.side != ResizeSide::Top {
                                return;
                            }

                            let pos = e.event.position;
                            let delta = drag_data.last_position.y - pos.y;
                            let new_y = (drag_data.last_bounds.origin.y - delta).max(px(0.));
                            let size_delta = drag_data.last_position.y - new_y;
                            let new_height = (drag_data.last_bounds.size.height + size_delta)
                                .max(MINIMUM_SIZE.width);
                            this.resize(None, Some(new_y), None, Some(new_height), window, cx);
                        }
                    },
                ))
                .into_any_element(),
        );

        // Bottom resize handle
        elements.push(
            div()
                .id("bottom-resize-handle")
                .cursor_ns_resize()
                .absolute()
                .left(px(0.0))
                .bottom(handle_offset)
                .w(item_bounds.size.width)
                .h(HANDLE_SIZE)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener({
                        move |this, event: &MouseDownEvent, window, cx| {
                            this.on_resize_handle_mouse_down(
                                ResizeSide::Bottom,
                                item_id,
                                item_bounds,
                                event,
                                window,
                                cx,
                            );
                        }
                    }),
                )
                .on_drag(DragResizing(entity_id), |drag, _, _, cx| {
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .on_drag_move(cx.listener(
                    move |this, e: &DragMoveEvent<DragResizing>, window, cx| match e.drag(cx) {
                        DragResizing(id) => {
                            if *id != entity_id {
                                return;
                            }

                            let Some(ref drag_data) = this.resizing_drag_data else {
                                return;
                            };

                            if drag_data.side != ResizeSide::Bottom {
                                return;
                            }

                            let pos = e.event.position;
                            let delta = pos.y - drag_data.last_position.y;
                            let new_height =
                                (drag_data.last_bounds.size.height + delta).max(MINIMUM_SIZE.width);
                            this.resize(None, None, None, Some(new_height), window, cx);
                        }
                    },
                ))
                .into_any_element(),
        );

        // Corner resize handle
        elements.push(
            div()
                .child(
                    Icon::new(IconName::ResizeCorner)
                        .size_3()
                        .absolute()
                        .right(px(1.))
                        .bottom(px(1.))
                        .text_color(cx.theme().muted_foreground.opacity(0.5)),
                )
                .child(
                    div()
                        .id("corner-resize-handle")
                        .cursor_nwse_resize()
                        .absolute()
                        .right(handle_offset)
                        .bottom(handle_offset)
                        .size_3()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener({
                                move |this, event: &MouseDownEvent, window, cx| {
                                    this.on_resize_handle_mouse_down(
                                        ResizeSide::BottomRight,
                                        item_id,
                                        item_bounds,
                                        event,
                                        window,
                                        cx,
                                    );
                                }
                            }),
                        )
                        .on_drag(DragResizing(entity_id), |drag, _, _, cx| {
                            cx.stop_propagation();
                            cx.new(|_| drag.clone())
                        })
                        .on_drag_move(cx.listener(
                            move |this, e: &DragMoveEvent<DragResizing>, window, cx| {
                                match e.drag(cx) {
                                    DragResizing(id) => {
                                        if *id != entity_id {
                                            return;
                                        }

                                        let Some(ref drag_data) = this.resizing_drag_data else {
                                            return;
                                        };

                                        if drag_data.side != ResizeSide::BottomRight {
                                            return;
                                        }

                                        let pos = e.event.position;
                                        let delta_x = pos.x - drag_data.last_position.x;
                                        let delta_y = pos.y - drag_data.last_position.y;
                                        let new_width = (drag_data.last_bounds.size.width
                                            + delta_x)
                                            .max(MINIMUM_SIZE.width);
                                        let new_height = (drag_data.last_bounds.size.height
                                            + delta_y)
                                            .max(MINIMUM_SIZE.height);
                                        this.resize(
                                            None,
                                            None,
                                            Some(new_width),
                                            Some(new_height),
                                            window,
                                            cx,
                                        );
                                    }
                                }
                            },
                        )),
                )
                .into_any_element(),
        );

        elements
    }

    pub(super) fn on_resize_handle_mouse_down(
        &mut self,
        side: ResizeSide,
        item_id: EntityId,
        item_bounds: inazuma::Bounds<Pixels>,
        event: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let last_position = event.position;
        self.resizing_id = Some(item_id);
        self.resizing_drag_data = Some(ResizeDrag {
            side,
            last_position,
            last_bounds: item_bounds,
        });

        if let Some(new_id) = self.bring_to_front(self.resizing_id, cx) {
            self.resizing_id = Some(new_id);
        }
        cx.stop_propagation();
    }

    /// Produce the drag-bar element for the given panel item
    pub(super) fn render_drag_bar(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
        entity_id: EntityId,
        item: &super::tiles::TileItem,
    ) -> AnyElement {
        let item_id = item.id;
        let item_bounds = item.bounds;

        h_flex()
            .id("drag-bar")
            .absolute()
            .w_full()
            .h(DRAG_BAR_HEIGHT)
            .bg(cx.theme().transparent)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    let inner_pos = event.position - this.bounds.origin;
                    this.dragging_id = Some(item_id);
                    this.dragging_initial_mouse = inner_pos;
                    this.dragging_initial_bounds = item_bounds;

                    if let Some(new_id) = this.bring_to_front(Some(item_id), cx) {
                        this.dragging_id = Some(new_id);
                    }
                }),
            )
            .on_drag(DragMoving(entity_id), |drag, _, _, cx| {
                cx.stop_propagation();
                cx.new(|_| drag.clone())
            })
            .on_drag_move(
                cx.listener(
                    move |this, e: &DragMoveEvent<DragMoving>, _, cx| match e.drag(cx) {
                        DragMoving(id) => {
                            if *id != entity_id {
                                return;
                            }
                            this.update_position(e.event.position, cx);
                        }
                    },
                ),
            )
            .into_any_element()
    }

    pub(super) fn render_panel(
        &mut self,
        item: &super::tiles::TileItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let entity_id = cx.entity_id();
        let item_id = item.id;
        let panel_view = item.panel.view();

        v_flex()
            .occlude()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .absolute()
            .left(item.bounds.origin.x)
            .top(item.bounds.origin.y)
            // More 1px to account for the border width when 2 panels are too close
            .w(item.bounds.size.width + px(1.))
            .h(item.bounds.size.height + px(1.))
            .rounded(cx.theme().tile_radius)
            .child(h_flex().overflow_hidden().size_full().child(panel_view))
            .children(self.render_resize_handles(window, cx, entity_id, &item))
            .child(self.render_drag_bar(window, cx, entity_id, &item))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, _| {
                    this.dragging_id = Some(item_id);
                }),
            )
            // Here must be mouse up for avoid conflict with Drag event
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    if this.dragging_id == Some(item_id) {
                        this.dragging_id = None;
                        this.bring_to_front(Some(item_id), cx);
                    }
                }),
            )
    }

    /// Handle the mouse up event to finalize drag or resize operations
    pub(super) fn on_mouse_up(&mut self, _: &mut Window, cx: &mut Context<'_, Tiles>) {
        // Check if a drag or resize was active
        if self.dragging_id.is_some()
            || self.resizing_id.is_some()
            || self.resizing_drag_data.is_some()
        {
            let mut changes_to_push = vec![];

            // Handle dragging
            if let Some(dragging_id) = self.dragging_id {
                if let Some(idx) = self.panels.iter().position(|p| p.id == dragging_id) {
                    let initial_bounds = self.dragging_initial_bounds;
                    let current_bounds = self.panels[idx].bounds;

                    // Apply grid alignment to final position
                    let aligned_origin = round_point_to_nearest_ten(current_bounds.origin, cx);

                    if initial_bounds.origin != aligned_origin
                        || initial_bounds.size != current_bounds.size
                    {
                        self.panels[idx].bounds.origin = aligned_origin;

                        changes_to_push.push(TileChange {
                            tile_id: self.panels[idx].panel.view().entity_id(),
                            old_bounds: Some(initial_bounds),
                            new_bounds: Some(self.panels[idx].bounds),
                            old_order: None,
                            new_order: None,
                            version: 0,
                        });
                    }
                }
            }

            // Handle resizing
            if let Some(resizing_id) = self.resizing_id {
                if let Some(drag_data) = &self.resizing_drag_data {
                    if let Some(item) = self.panel(&resizing_id) {
                        let initial_bounds = drag_data.last_bounds;
                        let current_bounds = item.bounds;
                        if initial_bounds.size != current_bounds.size {
                            changes_to_push.push(TileChange {
                                tile_id: item.panel.view().entity_id(),
                                old_bounds: Some(initial_bounds),
                                new_bounds: Some(current_bounds),
                                old_order: None,
                                new_order: None,
                                version: 0,
                            });
                        }
                    }
                }
            }

            // Push changes to history if any
            if !changes_to_push.is_empty() {
                for change in changes_to_push {
                    self.history.push(change);
                }
            }

            // Reset drag and resize state
            self.reset_current_index();
            self.resizing_drag_data = None;
            cx.emit(PanelEvent::LayoutChanged);
            cx.notify();
        }
    }
}

impl Focusable for Tiles {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl EventEmitter<PanelEvent> for Tiles {}
impl EventEmitter<DismissEvent> for Tiles {}
impl Render for Tiles {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();
        let panels = self.sorted_panels();
        let scroll_bounds =
            self.panels
                .iter()
                .fold(inazuma::Bounds::default(), |acc: inazuma::Bounds<Pixels>, item| inazuma::Bounds {
                    origin: Point {
                        x: acc.origin.x.min(item.bounds.origin.x),
                        y: acc.origin.y.min(item.bounds.origin.y),
                    },
                    size: Size {
                        width: acc.size.width.max(item.bounds.right()),
                        height: acc.size.height.max(item.bounds.bottom()),
                    },
                });
        let scroll_size = scroll_bounds.size - size(scroll_bounds.origin.x, scroll_bounds.origin.y);

        div()
            .relative()
            .bg(cx.theme().tiles)
            .child(
                div()
                    .id("tiles")
                    .track_scroll(&self.scroll_handle)
                    .size_full()
                    .top(-px(1.))
                    .left(-px(1.))
                    .overflow_scroll()
                    .children(
                        panels
                            .into_iter()
                            .map(|item| self.render_panel(&item, window, cx)),
                    )
                    .on_prepaint(move |bounds, _, cx| view.update(cx, |r, _| r.bounds = bounds))
                    .on_drop(cx.listener(move |_, item: &AnyDrag, _, cx| {
                        cx.emit(DragDrop(item.clone()));
                    })),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, _event: &MouseUpEvent, window, cx| {
                    this.on_mouse_up(window, cx);
                }),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .child(
                        Scrollbar::new(&self.scroll_handle)
                            .scroll_size(scroll_size)
                            .when_some(self.scrollbar_show, |this, scrollbar_show| {
                                this.scrollbar_show(scrollbar_show)
                            }),
                    ),
            )
            .size_full()
    }
}
