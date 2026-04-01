use super::*;

impl Interactivity {
    /// Create an `Interactivity`, capturing the caller location in debug mode.
    #[cfg(any(feature = "inspector", debug_assertions))]
    #[track_caller]
    pub fn new() -> Interactivity {
        Interactivity {
            source_location: Some(core::panic::Location::caller()),
            ..Default::default()
        }
    }

    /// Create an `Interactivity`, capturing the caller location in debug mode.
    #[cfg(not(any(feature = "inspector", debug_assertions)))]
    pub fn new() -> Interactivity {
        Interactivity::default()
    }

    /// Gets the source location of construction. Returns `None` when not in debug mode.
    pub fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        #[cfg(any(feature = "inspector", debug_assertions))]
        {
            self.source_location
        }

        #[cfg(not(any(feature = "inspector", debug_assertions)))]
        {
            None
        }
    }

    /// Bind the given callback to the mouse down event for the given mouse button, during the bubble phase.
    /// The imperative API equivalent of [`InteractiveElement::on_mouse_down`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to the view state from this callback.
    pub fn on_mouse_down(
        &mut self,
        button: MouseButton,
        listener: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_down_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Bubble
                    && event.button == button
                    && hitbox.is_hovered(window)
                {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse down event for any button, during the capture phase.
    /// The imperative API equivalent of [`InteractiveElement::capture_any_mouse_down`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn capture_any_mouse_down(
        &mut self,
        listener: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_down_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Capture && hitbox.is_hovered(window) {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse down event for any button, during the bubble phase.
    /// The imperative API equivalent to [`InteractiveElement::on_any_mouse_down`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_any_mouse_down(
        &mut self,
        listener: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_down_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Bubble && hitbox.is_hovered(window) {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse pressure event, during the bubble phase
    /// the imperative API equivalent to [`InteractiveElement::on_mouse_pressure`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_mouse_pressure(
        &mut self,
        listener: impl Fn(&MousePressureEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_pressure_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Bubble && hitbox.is_hovered(window) {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse pressure event, during the capture phase
    /// the imperative API equivalent to [`Interactivity::on_mouse_pressure`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn capture_mouse_pressure(
        &mut self,
        listener: impl Fn(&MousePressureEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_pressure_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Capture && hitbox.is_hovered(window) {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse up event for the given button, during the bubble phase.
    /// The imperative API equivalent to [`InteractiveElement::on_mouse_up`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_mouse_up(
        &mut self,
        button: MouseButton,
        listener: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_up_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Bubble
                    && event.button == button
                    && hitbox.is_hovered(window)
                {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse up event for any button, during the capture phase.
    /// The imperative API equivalent to [`InteractiveElement::capture_any_mouse_up`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn capture_any_mouse_up(
        &mut self,
        listener: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_up_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Capture && hitbox.is_hovered(window) {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse up event for any button, during the bubble phase.
    /// The imperative API equivalent to [`Interactivity::on_any_mouse_up`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_any_mouse_up(
        &mut self,
        listener: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_up_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Bubble && hitbox.is_hovered(window) {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse down event, on any button, during the capture phase,
    /// when the mouse is outside of the bounds of this element.
    /// The imperative API equivalent to [`InteractiveElement::on_mouse_down_out`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_mouse_down_out(
        &mut self,
        listener: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_down_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Capture && !hitbox.contains(&window.mouse_position()) {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse up event, for the given button, during the capture phase,
    /// when the mouse is outside of the bounds of this element.
    /// The imperative API equivalent to [`InteractiveElement::on_mouse_up_out`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_mouse_up_out(
        &mut self,
        button: MouseButton,
        listener: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_up_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Capture
                    && event.button == button
                    && !hitbox.is_hovered(window)
                {
                    (listener)(event, window, cx);
                }
            }));
    }

    /// Bind the given callback to the mouse move event, during the bubble phase.
    /// The imperative API equivalent to [`InteractiveElement::on_mouse_move`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_mouse_move(
        &mut self,
        listener: impl Fn(&MouseMoveEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_move_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Bubble && hitbox.is_hovered(window) {
                    (listener)(event, window, cx);
                }
            }));
    }

    /// Bind the given callback to the mouse drag event of the given type. Note that this
    /// will be called for all move events, inside or outside of this element, as long as the
    /// drag was started with this element under the mouse. Useful for implementing draggable
    /// UIs that don't conform to a drag and drop style interaction, like resizing.
    /// The imperative API equivalent to [`InteractiveElement::on_drag_move`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_drag_move<T>(
        &mut self,
        listener: impl Fn(&DragMoveEvent<T>, &mut Window, &mut App) + 'static,
    ) where
        T: 'static,
    {
        self.mouse_move_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Capture
                    && let Some(drag) = &cx.active_drag
                    && drag.value.as_ref().type_id() == TypeId::of::<T>()
                {
                    (listener)(
                        &DragMoveEvent {
                            event: event.clone(),
                            bounds: hitbox.bounds,
                            drag: PhantomData,
                            dragged_item: Arc::clone(&drag.value),
                        },
                        window,
                        cx,
                    );
                }
            }));
    }

    /// Bind the given callback to scroll wheel events during the bubble phase.
    /// The imperative API equivalent to [`InteractiveElement::on_scroll_wheel`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_scroll_wheel(
        &mut self,
        listener: impl Fn(&ScrollWheelEvent, &mut Window, &mut App) + 'static,
    ) {
        self.scroll_wheel_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Bubble && hitbox.should_handle_scroll(window) {
                    (listener)(event, window, cx);
                }
            }));
    }

    /// Bind the given callback to pinch gesture events during the bubble phase.
    ///
    /// Note: This event is only available on macOS and Wayland (Linux).
    /// On Windows, pinch gestures are simulated as scroll wheel events with Ctrl held.
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub fn on_pinch(&mut self, listener: impl Fn(&PinchEvent, &mut Window, &mut App) + 'static) {
        self.pinch_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Bubble && hitbox.is_hovered(window) {
                    (listener)(event, window, cx);
                }
            }));
    }

    /// Bind the given callback to pinch gesture events during the capture phase.
    ///
    /// Note: This event is only available on macOS and Wayland (Linux).
    /// On Windows, pinch gestures are simulated as scroll wheel events with Ctrl held.
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub fn capture_pinch(
        &mut self,
        listener: impl Fn(&PinchEvent, &mut Window, &mut App) + 'static,
    ) {
        self.pinch_listeners
            .push(Box::new(move |event, phase, _hitbox, window, cx| {
                if phase == DispatchPhase::Capture {
                    (listener)(event, window, cx);
                } else {
                    cx.propagate();
                }
            }));
    }

    /// Bind the given callback to an action dispatch during the capture phase.
    /// The imperative API equivalent to [`InteractiveElement::capture_action`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn capture_action<A: Action>(
        &mut self,
        listener: impl Fn(&A, &mut Window, &mut App) + 'static,
    ) {
        self.action_listeners.push((
            TypeId::of::<A>(),
            Box::new(move |action, phase, window, cx| {
                let action = action.downcast_ref().unwrap();
                if phase == DispatchPhase::Capture {
                    (listener)(action, window, cx)
                } else {
                    cx.propagate();
                }
            }),
        ));
    }

    /// Bind the given callback to an action dispatch during the bubble phase.
    /// The imperative API equivalent to [`InteractiveElement::on_action`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_action<A: Action>(&mut self, listener: impl Fn(&A, &mut Window, &mut App) + 'static) {
        self.action_listeners.push((
            TypeId::of::<A>(),
            Box::new(move |action, phase, window, cx| {
                let action = action.downcast_ref().unwrap();
                if phase == DispatchPhase::Bubble {
                    (listener)(action, window, cx)
                }
            }),
        ));
    }

    /// Bind the given callback to an action dispatch, based on a dynamic action parameter
    /// instead of a type parameter. Useful for component libraries that want to expose
    /// action bindings to their users.
    /// The imperative API equivalent to [`InteractiveElement::on_boxed_action`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_boxed_action(
        &mut self,
        action: &dyn Action,
        listener: impl Fn(&dyn Action, &mut Window, &mut App) + 'static,
    ) {
        let action = action.boxed_clone();
        self.action_listeners.push((
            (*action).type_id(),
            Box::new(move |_, phase, window, cx| {
                if phase == DispatchPhase::Bubble {
                    (listener)(&*action, window, cx)
                }
            }),
        ));
    }

    /// Bind the given callback to key down events during the bubble phase.
    /// The imperative API equivalent to [`InteractiveElement::on_key_down`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_key_down(
        &mut self,
        listener: impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static,
    ) {
        self.key_down_listeners
            .push(Box::new(move |event, phase, window, cx| {
                if phase == DispatchPhase::Bubble {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to key down events during the capture phase.
    /// The imperative API equivalent to [`InteractiveElement::capture_key_down`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn capture_key_down(
        &mut self,
        listener: impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static,
    ) {
        self.key_down_listeners
            .push(Box::new(move |event, phase, window, cx| {
                if phase == DispatchPhase::Capture {
                    listener(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to key up events during the bubble phase.
    /// The imperative API equivalent to [`InteractiveElement::on_key_up`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_key_up(&mut self, listener: impl Fn(&KeyUpEvent, &mut Window, &mut App) + 'static) {
        self.key_up_listeners
            .push(Box::new(move |event, phase, window, cx| {
                if phase == DispatchPhase::Bubble {
                    listener(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to key up events during the capture phase.
    /// The imperative API equivalent to [`InteractiveElement::on_key_up`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn capture_key_up(
        &mut self,
        listener: impl Fn(&KeyUpEvent, &mut Window, &mut App) + 'static,
    ) {
        self.key_up_listeners
            .push(Box::new(move |event, phase, window, cx| {
                if phase == DispatchPhase::Capture {
                    listener(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to modifiers changing events.
    /// The imperative API equivalent to [`InteractiveElement::on_modifiers_changed`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_modifiers_changed(
        &mut self,
        listener: impl Fn(&ModifiersChangedEvent, &mut Window, &mut App) + 'static,
    ) {
        self.modifiers_changed_listeners
            .push(Box::new(move |event, window, cx| {
                listener(event, window, cx)
            }));
    }

    /// Bind the given callback to drop events of the given type, whether or not the drag started on this element.
    /// The imperative API equivalent to [`InteractiveElement::on_drop`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_drop<T: 'static>(&mut self, listener: impl Fn(&T, &mut Window, &mut App) + 'static) {
        self.drop_listeners.push((
            TypeId::of::<T>(),
            Box::new(move |dragged_value, window, cx| {
                listener(dragged_value.downcast_ref().unwrap(), window, cx);
            }),
        ));
    }

    /// Use the given predicate to determine whether or not a drop event should be dispatched to this element.
    /// The imperative API equivalent to [`InteractiveElement::can_drop`].
    pub fn can_drop(
        &mut self,
        predicate: impl Fn(&dyn Any, &mut Window, &mut App) -> bool + 'static,
    ) {
        self.can_drop_predicate = Some(Box::new(predicate));
    }

    /// Bind the given callback to click events of this element.
    /// The imperative API equivalent to [`StatefulInteractiveElement::on_click`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_click(&mut self, listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static)
    where
        Self: Sized,
    {
        self.click_listeners.push(Rc::new(move |event, window, cx| {
            listener(event, window, cx)
        }));
    }

    /// Bind the given callback to non-primary click events of this element.
    /// The imperative API equivalent to [`StatefulInteractiveElement::on_aux_click`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_aux_click(&mut self, listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static)
    where
        Self: Sized,
    {
        self.aux_click_listeners
            .push(Rc::new(move |event, window, cx| {
                listener(event, window, cx)
            }));
    }

    /// On drag initiation, this callback will be used to create a new view to render the dragged value for a
    /// drag and drop operation. This API should also be used as the equivalent of 'on drag start' with
    /// the [`Self::on_drag_move`] API.
    /// The imperative API equivalent to [`StatefulInteractiveElement::on_drag`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_drag<T, W>(
        &mut self,
        value: T,
        constructor: impl Fn(&T, Point<Pixels>, &mut Window, &mut App) -> Entity<W> + 'static,
    ) where
        Self: Sized,
        T: 'static,
        W: 'static + Render,
    {
        debug_assert!(
            self.drag_listener.is_none(),
            "calling on_drag more than once on the same element is not supported"
        );
        self.drag_listener = Some((
            Arc::new(value),
            Box::new(move |value, offset, window, cx| {
                constructor(value.downcast_ref().unwrap(), offset, window, cx).into()
            }),
        ));
    }

    /// Bind the given callback on the hover start and end events of this element. Note that the boolean
    /// passed to the callback is true when the hover starts and false when it ends.
    /// The imperative API equivalent to [`StatefulInteractiveElement::on_hover`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_hover(&mut self, listener: impl Fn(&bool, &mut Window, &mut App) + 'static)
    where
        Self: Sized,
    {
        debug_assert!(
            self.hover_listener.is_none(),
            "calling on_hover more than once on the same element is not supported"
        );
        self.hover_listener = Some(Box::new(listener));
    }

    /// Use the given callback to construct a new tooltip view when the mouse hovers over this element.
    /// The imperative API equivalent to [`StatefulInteractiveElement::tooltip`].
    pub fn tooltip(&mut self, build_tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static)
    where
        Self: Sized,
    {
        debug_assert!(
            self.tooltip_builder.is_none(),
            "calling tooltip more than once on the same element is not supported"
        );
        self.tooltip_builder = Some(TooltipBuilder {
            build: Rc::new(build_tooltip),
            hoverable: false,
        });
    }

    /// Use the given callback to construct a new tooltip view when the mouse hovers over this element.
    /// The tooltip itself is also hoverable and won't disappear when the user moves the mouse into
    /// the tooltip. The imperative API equivalent to [`StatefulInteractiveElement::hoverable_tooltip`].
    pub fn hoverable_tooltip(
        &mut self,
        build_tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    ) where
        Self: Sized,
    {
        debug_assert!(
            self.tooltip_builder.is_none(),
            "calling tooltip more than once on the same element is not supported"
        );
        self.tooltip_builder = Some(TooltipBuilder {
            build: Rc::new(build_tooltip),
            hoverable: true,
        });
    }

    /// Block the mouse from all interactions with elements behind this element's hitbox. Typically
    /// `block_mouse_except_scroll` should be preferred.
    ///
    /// The imperative API equivalent to [`InteractiveElement::occlude`]
    pub fn occlude_mouse(&mut self) {
        self.hitbox_behavior = HitboxBehavior::BlockMouse;
    }

    /// Set the bounds of this element as a window control area for the platform window.
    /// The imperative API equivalent to [`InteractiveElement::window_control_area`]
    pub fn window_control_area(&mut self, area: WindowControlArea) {
        self.window_control = Some(area);
    }

    /// Block non-scroll mouse interactions with elements behind this element's hitbox.
    /// The imperative API equivalent to [`InteractiveElement::block_mouse_except_scroll`].
    ///
    /// See [`Hitbox::is_hovered`] for details.
    pub fn block_mouse_except_scroll(&mut self) {
        self.hitbox_behavior = HitboxBehavior::BlockMouseExceptScroll;
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub(super) fn has_pinch_listeners(&self) -> bool {
        !self.pinch_listeners.is_empty()
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    pub(super) fn has_pinch_listeners(&self) -> bool {
        false
    }
}
