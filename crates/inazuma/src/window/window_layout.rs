use crate::{
    AnyImageCache, App, AvailableSpace, Bounds, EntityId, InputHandler,
    KeyContext, LayoutId, PlatformInputHandler, Pixels, Size, Style,
};

use super::*;

impl Window {
    /// Add a node to the layout tree for the current frame. Takes the `Style` of the element for which
    /// layout is being requested, along with the layout ids of any children. This method is called during
    /// calls to the [`Element::request_layout`] trait method and enables any element to participate in layout.
    ///
    /// This method should only be called as part of the request_layout or prepaint phase of element drawing.
    #[must_use]
    pub fn request_layout(
        &mut self,
        style: Style,
        children: impl IntoIterator<Item = LayoutId>,
        cx: &mut App,
    ) -> LayoutId {
        self.invalidator.debug_assert_prepaint();

        cx.layout_id_buffer.clear();
        cx.layout_id_buffer.extend(children);
        let rem_size = self.rem_size();
        let scale_factor = self.scale_factor();

        self.layout_engine.as_mut().unwrap().request_layout(
            style,
            rem_size,
            scale_factor,
            &cx.layout_id_buffer,
        )
    }

    /// Add a node to the layout tree for the current frame. Instead of taking a `Style` and children,
    /// this variant takes a function that is invoked during layout so you can use arbitrary logic to
    /// determine the element's size. One place this is used internally is when measuring text.
    ///
    /// The given closure is invoked at layout time with the known dimensions and available space and
    /// returns a `Size`.
    ///
    /// This method should only be called as part of the request_layout or prepaint phase of element drawing.
    pub fn request_measured_layout<F>(&mut self, style: Style, measure: F) -> LayoutId
    where
        F: Fn(Size<Option<Pixels>>, Size<AvailableSpace>, &mut Window, &mut App) -> Size<Pixels>
            + 'static,
    {
        self.invalidator.debug_assert_prepaint();

        let rem_size = self.rem_size();
        let scale_factor = self.scale_factor();
        self.layout_engine
            .as_mut()
            .unwrap()
            .request_measured_layout(style, rem_size, scale_factor, measure)
    }

    /// Compute the layout for the given id within the given available space.
    /// This method is called for its side effect, typically by the framework prior to painting.
    /// After calling it, you can request the bounds of the given layout node id or any descendant.
    ///
    /// This method should only be called as part of the prepaint phase of element drawing.
    pub fn compute_layout(
        &mut self,
        layout_id: LayoutId,
        available_space: Size<AvailableSpace>,
        cx: &mut App,
    ) {
        self.invalidator.debug_assert_prepaint();

        let mut layout_engine = self.layout_engine.take().unwrap();
        layout_engine.compute_layout(layout_id, available_space, self, cx);
        self.layout_engine = Some(layout_engine);
    }

    /// Obtain the bounds computed for the given LayoutId relative to the window. This method will usually be invoked by
    /// GPUI itself automatically in order to pass your element its `Bounds` automatically.
    ///
    /// This method should only be called as part of element drawing.
    pub fn layout_bounds(&mut self, layout_id: LayoutId) -> Bounds<Pixels> {
        self.invalidator.debug_assert_prepaint();

        let scale_factor = self.scale_factor();
        let mut bounds = self
            .layout_engine
            .as_mut()
            .unwrap()
            .layout_bounds(layout_id, scale_factor)
            .map(Into::into);
        bounds.origin += self.element_offset();
        bounds
    }

    /// This method should be called during `prepaint`. You can use
    /// the returned [Hitbox] during `paint` or in an event handler
    /// to determine whether the inserted hitbox was the topmost.
    ///
    /// This method should only be called as part of the prepaint phase of element drawing.
    pub fn insert_hitbox(&mut self, bounds: Bounds<Pixels>, behavior: HitboxBehavior) -> Hitbox {
        self.invalidator.debug_assert_prepaint();

        let content_mask = self.content_mask();
        let mut id = self.next_hitbox_id;
        self.next_hitbox_id = self.next_hitbox_id.next();
        let hitbox = Hitbox {
            id,
            bounds,
            content_mask,
            behavior,
        };
        self.next_frame.hitboxes.push(hitbox.clone());
        hitbox
    }

    /// Set a hitbox which will act as a control area of the platform window.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn insert_window_control_hitbox(&mut self, area: WindowControlArea, hitbox: Hitbox) {
        self.invalidator.debug_assert_paint();
        self.next_frame.window_control_hitboxes.push((area, hitbox));
    }

    /// Sets the key context for the current element. This context will be used to translate
    /// keybindings into actions.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn set_key_context(&mut self, context: KeyContext) {
        self.invalidator.debug_assert_paint();
        self.next_frame.dispatch_tree.set_key_context(context);
    }

    /// Sets the focus handle for the current element. This handle will be used to manage focus state
    /// and keyboard event dispatch for the element.
    ///
    /// This method should only be called as part of the prepaint phase of element drawing.
    pub fn set_focus_handle(&mut self, focus_handle: &FocusHandle, _: &App) {
        self.invalidator.debug_assert_prepaint();
        if focus_handle.is_focused(self) {
            self.next_frame.focus = Some(focus_handle.id);
        }
        self.next_frame.dispatch_tree.set_focus_id(focus_handle.id);
    }

    /// Sets the view id for the current element, which will be used to manage view caching.
    ///
    /// This method should only be called as part of element prepaint. We plan on removing this
    /// method eventually when we solve some issues that require us to construct editor elements
    /// directly instead of always using editors via views.
    pub fn set_view_id(&mut self, view_id: EntityId) {
        self.invalidator.debug_assert_prepaint();
        self.next_frame.dispatch_tree.set_view_id(view_id);
    }

    /// Get the entity ID for the currently rendering view
    pub fn current_view(&self) -> EntityId {
        self.invalidator.debug_assert_paint_or_prepaint();
        self.rendered_entity_stack.last().copied().unwrap()
    }

    #[inline]
    pub(crate) fn with_rendered_view<R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.rendered_entity_stack.push(id);
        let result = f(self);
        self.rendered_entity_stack.pop();
        result
    }

    /// Executes the provided function with the specified image cache.
    pub fn with_image_cache<F, R>(&mut self, image_cache: Option<AnyImageCache>, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        if let Some(image_cache) = image_cache {
            self.image_cache_stack.push(image_cache);
            let result = f(self);
            self.image_cache_stack.pop();
            result
        } else {
            f(self)
        }
    }

    /// Sets an input handler, such as [`ElementInputHandler`][element_input_handler], which interfaces with the
    /// platform to receive textual input with proper integration with concerns such
    /// as IME interactions. This handler will be active for the upcoming frame until the following frame is
    /// rendered.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    ///
    /// [element_input_handler]: crate::ElementInputHandler
    pub fn handle_input(
        &mut self,
        focus_handle: &FocusHandle,
        input_handler: impl InputHandler,
        cx: &App,
    ) {
        self.invalidator.debug_assert_paint();

        if focus_handle.is_focused(self) {
            let cx = self.to_async(cx);
            self.next_frame
                .input_handlers
                .push(Some(PlatformInputHandler::new(cx, Box::new(input_handler))));
        }
    }
}
