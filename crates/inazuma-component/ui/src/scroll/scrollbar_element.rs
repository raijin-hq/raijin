use instant::{Duration, Instant};

use crate::{ActiveTheme, AxisExt};
use inazuma::{
    App, BorderStyle, Bounds, ContentMask, Corner, CursorStyle, Edges, Element,
    GlobalElementId, HitboxBehavior, InspectorElementId, IsZero, LayoutId,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Position,
    ScrollWheelEvent, Size, Style, Window, fill, point, px, size,
};

use super::scrollbar::{
    AxisPrepaintState, PrepaintState, Scrollbar, ScrollbarState,
    FADE_OUT_DELAY, FADE_OUT_DURATION, MIN_THUMB_SIZE, WIDTH,
};

impl Element for Scrollbar {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<inazuma::ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.position = Position::Absolute;
        // Anchor to all edges of the parent so the scrollbar overlays the
        // entire container regardless of sibling layout.
        style.inset.top = px(0.).into();
        style.inset.right = px(0.).into();
        style.inset.bottom = px(0.).into();
        style.inset.left = px(0.).into();

        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let hitbox = window.with_content_mask(Some(ContentMask { bounds }), |window| {
            window.insert_hitbox(bounds, HitboxBehavior::Normal)
        });

        let state = window
            .use_state(cx, |_, _| ScrollbarState::default())
            .read(cx)
            .clone();

        let mut states = vec![];
        let mut has_both = self.axis.is_both();
        let scroll_size = self
            .scroll_size
            .unwrap_or(self.scroll_handle.content_size());

        for axis in self.axis.all().into_iter() {
            let is_vertical = axis.is_vertical();
            let (scroll_area_size, container_size, scroll_position) = if is_vertical {
                (
                    scroll_size.height,
                    hitbox.size.height,
                    self.scroll_handle.offset().y,
                )
            } else {
                (
                    scroll_size.width,
                    hitbox.size.width,
                    self.scroll_handle.offset().x,
                )
            };

            // The horizontal scrollbar is set avoid overlapping with the vertical scrollbar, if the vertical scrollbar is visible.
            let margin_end = if has_both && !is_vertical {
                WIDTH
            } else {
                px(0.)
            };

            // Hide scrollbar, if the scroll area is smaller than the container.
            if scroll_area_size <= container_size {
                has_both = false;
                continue;
            }

            let thumb_length =
                (container_size / scroll_area_size * container_size).max(px(MIN_THUMB_SIZE));
            let thumb_start = -(scroll_position / (scroll_area_size - container_size)
                * (container_size - margin_end - thumb_length));
            let thumb_end = (thumb_start + thumb_length).min(container_size - margin_end);

            let bounds = Bounds {
                origin: if is_vertical {
                    point(hitbox.origin.x + hitbox.size.width - WIDTH, hitbox.origin.y)
                } else {
                    point(
                        hitbox.origin.x,
                        hitbox.origin.y + hitbox.size.height - WIDTH,
                    )
                },
                size: Size {
                    width: if is_vertical {
                        WIDTH
                    } else {
                        hitbox.size.width
                    },
                    height: if is_vertical {
                        hitbox.size.height
                    } else {
                        WIDTH
                    },
                },
            };

            let scrollbar_show = self.scrollbar_show.unwrap_or(cx.theme().scrollbar_show);
            let is_always_to_show = scrollbar_show.is_always();
            let is_hover_to_show = scrollbar_show.is_hover();
            let is_hovered_on_bar = state.get().hovered_axis == Some(axis);
            let is_hovered_on_thumb = state.get().hovered_on_thumb == Some(axis);
            let is_offset_changed = state.get().last_scroll_offset != self.scroll_handle.offset();

            let (thumb_bg, bar_bg, bar_border, thumb_width, inset, radius) =
                if state.get().dragged_axis == Some(axis) {
                    Scrollbar::style_for_active(cx)
                } else if is_hover_to_show && (is_hovered_on_bar || is_hovered_on_thumb) {
                    if is_hovered_on_thumb {
                        Scrollbar::style_for_hovered_thumb(cx)
                    } else {
                        Scrollbar::style_for_hovered_bar(cx)
                    }
                } else if is_offset_changed {
                    self.style_for_normal(cx)
                } else if is_always_to_show {
                    if is_hovered_on_thumb {
                        Scrollbar::style_for_hovered_thumb(cx)
                    } else {
                        Scrollbar::style_for_hovered_bar(cx)
                    }
                } else {
                    let mut idle_state = self.style_for_idle(cx);
                    // Delay 2s to fade out the scrollbar thumb (in 1s)
                    if let Some(last_time) = state.get().last_scroll_time {
                        let elapsed = Instant::now().duration_since(last_time).as_secs_f32();
                        if is_hovered_on_bar {
                            state.set(state.get().with_last_scroll_time(Some(Instant::now())));
                            idle_state = if is_hovered_on_thumb {
                                Scrollbar::style_for_hovered_thumb(cx)
                            } else {
                                Scrollbar::style_for_hovered_bar(cx)
                            };
                        } else if elapsed < FADE_OUT_DELAY {
                            idle_state.0 = cx.theme().scrollbar_thumb;

                            if !state.get().idle_timer_scheduled {
                                let state = state.clone();
                                state.set(state.get().with_idle_timer_scheduled(true));
                                let current_view = window.current_view();
                                let next_delay = Duration::from_secs_f32(FADE_OUT_DELAY - elapsed);
                                window
                                    .spawn(cx, async move |cx| {
                                        cx.background_executor().timer(next_delay).await;
                                        state.set(state.get().with_idle_timer_scheduled(false));
                                        cx.update(|_, cx| cx.notify(current_view)).ok();
                                    })
                                    .detach();
                            }
                        } else if elapsed < FADE_OUT_DURATION {
                            let opacity = 1.0 - (elapsed - FADE_OUT_DELAY).powi(10);
                            idle_state.0 = cx.theme().scrollbar_thumb.opacity(opacity);

                            window.request_animation_frame();
                        }
                    }

                    idle_state
                };

            // The clickable area of the thumb
            let thumb_length = thumb_end - thumb_start - inset * 2;
            let thumb_bounds = if is_vertical {
                Bounds::from_corner_and_size(
                    Corner::TopRight,
                    bounds.top_right() + point(-inset, inset + thumb_start),
                    size(WIDTH, thumb_length),
                )
            } else {
                Bounds::from_corner_and_size(
                    Corner::BottomLeft,
                    bounds.bottom_left() + point(inset + thumb_start, -inset),
                    size(thumb_length, WIDTH),
                )
            };

            // The actual render area of the thumb
            let thumb_fill_bounds = if is_vertical {
                Bounds::from_corner_and_size(
                    Corner::TopRight,
                    bounds.top_right() + point(-inset, inset + thumb_start),
                    size(thumb_width, thumb_length),
                )
            } else {
                Bounds::from_corner_and_size(
                    Corner::BottomLeft,
                    bounds.bottom_left() + point(inset + thumb_start, -inset),
                    size(thumb_length, thumb_width),
                )
            };

            let bar_hitbox = window.with_content_mask(Some(ContentMask { bounds }), |window| {
                window.insert_hitbox(bounds, HitboxBehavior::Normal)
            });

            states.push(AxisPrepaintState {
                axis,
                bar_hitbox,
                bounds,
                radius,
                bg: bar_bg,
                border: bar_border,
                thumb_bounds,
                thumb_fill_bounds,
                thumb_bg,
                scroll_size: scroll_area_size,
                container_size,
                thumb_size: thumb_length,
                margin_end,
            })
        }

        PrepaintState {
            hitbox,
            states,
            scrollbar_state: state,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let scrollbar_state = &prepaint.scrollbar_state;
        let scrollbar_show = self.scrollbar_show.unwrap_or(cx.theme().scrollbar_show);
        let view_id = window.current_view();
        let hitbox_bounds = prepaint.hitbox.bounds;
        let is_visible = scrollbar_state.get().is_scrollbar_visible() || scrollbar_show.is_always();
        let is_hover_to_show = scrollbar_show.is_hover();

        // Update last_scroll_time when offset is changed.
        if self.scroll_handle.offset() != scrollbar_state.get().last_scroll_offset {
            scrollbar_state.set(
                scrollbar_state
                    .get()
                    .with_last_scroll(self.scroll_handle.offset(), Some(Instant::now())),
            );
            cx.notify(view_id);
        }

        window.with_content_mask(
            Some(ContentMask {
                bounds: hitbox_bounds,
            }),
            |window| {
                for state in prepaint.states.iter() {
                    let axis = state.axis;
                    let mut radius = state.radius;
                    if cx.theme().radius.is_zero() {
                        radius = px(0.);
                    }
                    let bounds = state.bounds;
                    let thumb_bounds = state.thumb_bounds;
                    let scroll_area_size = state.scroll_size;
                    let container_size = state.container_size;
                    let thumb_size = state.thumb_size;
                    let margin_end = state.margin_end;
                    let is_vertical = axis.is_vertical();

                    window.set_cursor_style(CursorStyle::default(), &state.bar_hitbox);

                    window.paint_layer(hitbox_bounds, |cx| {
                        cx.paint_quad(fill(state.bounds, state.bg));

                        cx.paint_quad(PaintQuad {
                            bounds,
                            corner_radii: (0.).into(),
                            background: inazuma::transparent_black().into(),
                            border_widths: if is_vertical {
                                Edges {
                                    top: px(0.),
                                    right: px(0.),
                                    bottom: px(0.),
                                    left: px(0.),
                                }
                            } else {
                                Edges {
                                    top: px(0.),
                                    right: px(0.),
                                    bottom: px(0.),
                                    left: px(0.),
                                }
                            },
                            border_colors: Edges { top: state.border, right: state.border, bottom: state.border, left: state.border },
                            border_style: BorderStyle::default(),
                        });

                        cx.paint_quad(
                            fill(state.thumb_fill_bounds, state.thumb_bg).corner_radii(radius),
                        );
                    });

                    window.on_mouse_event({
                        let state = scrollbar_state.clone();
                        let scroll_handle = self.scroll_handle.clone();

                        move |event: &ScrollWheelEvent, phase, _, cx| {
                            if phase.bubble() && hitbox_bounds.contains(&event.position) {
                                if scroll_handle.offset() != state.get().last_scroll_offset {
                                    state.set(state.get().with_last_scroll(
                                        scroll_handle.offset(),
                                        Some(Instant::now()),
                                    ));
                                    cx.notify(view_id);
                                }
                            }
                        }
                    });

                    let safe_range = (-scroll_area_size + container_size)..px(0.);

                    if is_hover_to_show || is_visible {
                        window.on_mouse_event({
                            let state = scrollbar_state.clone();
                            let scroll_handle = self.scroll_handle.clone();

                            move |event: &MouseDownEvent, phase, _, cx| {
                                if phase.bubble() && bounds.contains(&event.position) {
                                    cx.stop_propagation();

                                    if thumb_bounds.contains(&event.position) {
                                        // click on the thumb bar, set the drag position
                                        let pos = event.position - thumb_bounds.origin;

                                        scroll_handle.start_drag();
                                        state.set(state.get().with_drag_pos(axis, pos));

                                        cx.notify(view_id);
                                    } else {
                                        // click on the scrollbar, jump to the position
                                        // Set the thumb bar center to the click position
                                        let offset = scroll_handle.offset();
                                        let percentage = if is_vertical {
                                            (event.position.y - thumb_size / 2. - bounds.origin.y)
                                                / (bounds.size.height - thumb_size)
                                        } else {
                                            (event.position.x - thumb_size / 2. - bounds.origin.x)
                                                / (bounds.size.width - thumb_size)
                                        }
                                        .min(1.);

                                        if is_vertical {
                                            scroll_handle.set_offset(point(
                                                offset.x,
                                                (-scroll_area_size * percentage)
                                                    .clamp(safe_range.start, safe_range.end),
                                            ));
                                        } else {
                                            scroll_handle.set_offset(point(
                                                (-scroll_area_size * percentage)
                                                    .clamp(safe_range.start, safe_range.end),
                                                offset.y,
                                            ));
                                        }
                                    }
                                }
                            }
                        });
                    }

                    window.on_mouse_event({
                        let scroll_handle = self.scroll_handle.clone();
                        let state = scrollbar_state.clone();
                        let max_fps_duration = Duration::from_millis((1000 / self.max_fps) as u64);

                        move |event: &MouseMoveEvent, _, _, cx| {
                            let mut notify = false;
                            // When is hover to show mode or it was visible,
                            // we need to update the hovered state and increase the last_scroll_time.
                            let need_hover_to_update = is_hover_to_show || is_visible;
                            // Update hovered state for scrollbar
                            if bounds.contains(&event.position) && need_hover_to_update {
                                state.set(state.get().with_hovered(Some(axis)));

                                if state.get().hovered_axis != Some(axis) {
                                    notify = true;
                                }
                            } else {
                                if state.get().hovered_axis == Some(axis) {
                                    if state.get().hovered_axis.is_some() {
                                        state.set(state.get().with_hovered(None));
                                        notify = true;
                                    }
                                }
                            }

                            // Update hovered state for scrollbar thumb
                            if thumb_bounds.contains(&event.position) {
                                if state.get().hovered_on_thumb != Some(axis) {
                                    state.set(state.get().with_hovered_on_thumb(Some(axis)));
                                    notify = true;
                                }
                            } else {
                                if state.get().hovered_on_thumb == Some(axis) {
                                    state.set(state.get().with_hovered_on_thumb(None));
                                    notify = true;
                                }
                            }

                            // Move thumb position on dragging
                            if state.get().dragged_axis == Some(axis) && event.dragging() {
                                // Stop the event propagation to avoid selecting text or other side effects.
                                cx.stop_propagation();

                                // drag_pos is the position of the mouse down event
                                // We need to keep the thumb bar still at the origin down position
                                let drag_pos = state.get().drag_pos;

                                let percentage = (if is_vertical {
                                    (event.position.y - drag_pos.y - bounds.origin.y)
                                        / (bounds.size.height - thumb_size)
                                } else {
                                    (event.position.x - drag_pos.x - bounds.origin.x)
                                        / (bounds.size.width - thumb_size - margin_end)
                                })
                                .clamp(0., 1.);

                                let offset = if is_vertical {
                                    point(
                                        scroll_handle.offset().x,
                                        (-(scroll_area_size - container_size) * percentage)
                                            .clamp(safe_range.start, safe_range.end),
                                    )
                                } else {
                                    point(
                                        (-(scroll_area_size - container_size) * percentage)
                                            .clamp(safe_range.start, safe_range.end),
                                        scroll_handle.offset().y,
                                    )
                                };

                                if (scroll_handle.offset().y - offset.y).abs() > px(1.)
                                    || (scroll_handle.offset().x - offset.x).abs() > px(1.)
                                {
                                    // Limit update rate
                                    if state.get().last_update.elapsed() > max_fps_duration {
                                        scroll_handle.set_offset(offset);
                                        state.set(state.get().with_last_update(Instant::now()));
                                        notify = true;
                                    }
                                }
                            }

                            if notify {
                                cx.notify(view_id);
                            }
                        }
                    });

                    window.on_mouse_event({
                        let state = scrollbar_state.clone();
                        let scroll_handle = self.scroll_handle.clone();

                        move |_event: &MouseUpEvent, phase, _, cx| {
                            if phase.bubble() {
                                scroll_handle.end_drag();
                                state.set(state.get().with_unset_drag_pos());
                                cx.notify(view_id);
                            }
                        }
                    });
                }
            },
        );
    }
}
