use super::*;

impl WaylandWindowStatePtr {
    pub fn handle(&self) -> AnyWindowHandle {
        self.state.borrow().handle
    }

    pub fn surface(&self) -> wl_surface::WlSurface {
        self.state.borrow().surface.clone()
    }

    pub fn toplevel(&self) -> Option<xdg_toplevel::XdgToplevel> {
        self.state.borrow().surface_state.toplevel().cloned()
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }

    pub fn add_child(&self, child: ObjectId) {
        let mut state = self.state.borrow_mut();
        state.children.insert(child);
    }

    pub fn is_blocked(&self) -> bool {
        let state = self.state.borrow();
        !state.children.is_empty()
    }

    pub fn frame(&self) {
        let mut state = self.state.borrow_mut();
        state.surface.frame(&state.globals.qh, state.surface.id());
        state.resize_throttle = false;
        drop(state);

        let mut cb = self.callbacks.borrow_mut();
        if let Some(fun) = cb.request_frame.as_mut() {
            fun(Default::default());
        }
    }

    pub fn handle_xdg_surface_event(&self, event: xdg_surface::Event) {
        if let xdg_surface::Event::Configure { serial } = event {
            {
                let mut state = self.state.borrow_mut();
                if let Some(window_controls) = state.in_progress_window_controls.take() {
                    state.window_controls = window_controls;

                    drop(state);
                    let mut callbacks = self.callbacks.borrow_mut();
                    if let Some(appearance_changed) = callbacks.appearance_changed.as_mut() {
                        appearance_changed();
                    }
                }
            }
            {
                let mut state = self.state.borrow_mut();

                if let Some(mut configure) = state.in_progress_configure.take() {
                    let got_unmaximized = state.maximized && !configure.maximized;
                    state.fullscreen = configure.fullscreen;
                    state.maximized = configure.maximized;
                    state.tiling = configure.tiling;
                    // Limit interactive resizes to once per vblank
                    if configure.resizing && state.resize_throttle {
                        state.surface_state.ack_configure(serial);
                        return;
                    } else if configure.resizing {
                        state.resize_throttle = true;
                    }
                    if !configure.fullscreen && !configure.maximized {
                        configure.size = if got_unmaximized {
                            Some(state.window_bounds.size)
                        } else {
                            compute_outer_size(state.inset(), configure.size, state.tiling)
                        };
                        if let Some(size) = configure.size {
                            state.window_bounds = Bounds {
                                origin: Point::default(),
                                size,
                            };
                        }
                    }
                    drop(state);
                    if let Some(size) = configure.size {
                        self.resize(size);
                    }
                }
            }
            let mut state = self.state.borrow_mut();
            state.surface_state.ack_configure(serial);

            let window_geometry = inset_by_tiling(
                state.bounds.map_origin(|_| px(0.0)),
                state.inset(),
                state.tiling,
            )
            .map(|v| f32::from(v) as i32)
            .map_size(|v| if v <= 0 { 1 } else { v });

            state.surface_state.set_geometry(
                window_geometry.origin.x,
                window_geometry.origin.y,
                window_geometry.size.width,
                window_geometry.size.height,
            );

            let request_frame_callback = !state.acknowledged_first_configure;
            if request_frame_callback {
                state.acknowledged_first_configure = true;
                drop(state);
                self.frame();
            }
        }
    }

    pub fn handle_toplevel_decoration_event(&self, event: zxdg_toplevel_decoration_v1::Event) {
        if let zxdg_toplevel_decoration_v1::Event::Configure { mode } = event {
            match mode {
                WEnum::Value(zxdg_toplevel_decoration_v1::Mode::ServerSide) => {
                    self.state.borrow_mut().decorations = WindowDecorations::Server;
                    let callback = self.callbacks.borrow_mut().appearance_changed.take();
                    if let Some(mut fun) = callback {
                        fun();
                        self.callbacks.borrow_mut().appearance_changed = Some(fun);
                    }
                }
                WEnum::Value(zxdg_toplevel_decoration_v1::Mode::ClientSide) => {
                    self.state.borrow_mut().decorations = WindowDecorations::Client;
                    // Update background to be transparent
                    let callback = self.callbacks.borrow_mut().appearance_changed.take();
                    if let Some(mut fun) = callback {
                        fun();
                        self.callbacks.borrow_mut().appearance_changed = Some(fun);
                    }
                }
                WEnum::Value(_) => {
                    log::warn!("Unknown decoration mode");
                }
                WEnum::Unknown(v) => {
                    log::warn!("Unknown decoration mode: {}", v);
                }
            }
        }
    }

    pub fn handle_fractional_scale_event(&self, event: wp_fractional_scale_v1::Event) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            self.rescale(scale as f32 / 120.0);
        }
    }

    pub fn handle_toplevel_event(&self, event: xdg_toplevel::Event) -> bool {
        match event {
            xdg_toplevel::Event::Configure {
                width,
                height,
                states,
            } => {
                let size = if width == 0 || height == 0 {
                    None
                } else {
                    Some(size(px(width as f32), px(height as f32)))
                };

                let states = extract_states::<xdg_toplevel::State>(&states);

                let mut tiling = Tiling::default();
                let mut fullscreen = false;
                let mut maximized = false;
                let mut resizing = false;

                for state in states {
                    match state {
                        xdg_toplevel::State::Maximized => {
                            maximized = true;
                        }
                        xdg_toplevel::State::Fullscreen => {
                            fullscreen = true;
                        }
                        xdg_toplevel::State::Resizing => resizing = true,
                        xdg_toplevel::State::TiledTop => {
                            tiling.top = true;
                        }
                        xdg_toplevel::State::TiledLeft => {
                            tiling.left = true;
                        }
                        xdg_toplevel::State::TiledRight => {
                            tiling.right = true;
                        }
                        xdg_toplevel::State::TiledBottom => {
                            tiling.bottom = true;
                        }
                        _ => {
                            // noop
                        }
                    }
                }

                if fullscreen || maximized {
                    tiling = Tiling::tiled();
                }

                let mut state = self.state.borrow_mut();
                state.in_progress_configure = Some(InProgressConfigure {
                    size,
                    fullscreen,
                    maximized,
                    resizing,
                    tiling,
                });

                false
            }
            xdg_toplevel::Event::Close => {
                let mut cb = self.callbacks.borrow_mut();
                if let Some(mut should_close) = cb.should_close.take() {
                    let result = (should_close)();
                    cb.should_close = Some(should_close);
                    if result {
                        drop(cb);
                        self.close();
                    }
                    result
                } else {
                    true
                }
            }
            xdg_toplevel::Event::WmCapabilities { capabilities } => {
                let mut window_controls = WindowControls::default();

                let states = extract_states::<xdg_toplevel::WmCapabilities>(&capabilities);

                for state in states {
                    match state {
                        xdg_toplevel::WmCapabilities::Maximize => {
                            window_controls.maximize = true;
                        }
                        xdg_toplevel::WmCapabilities::Minimize => {
                            window_controls.minimize = true;
                        }
                        xdg_toplevel::WmCapabilities::Fullscreen => {
                            window_controls.fullscreen = true;
                        }
                        xdg_toplevel::WmCapabilities::WindowMenu => {
                            window_controls.window_menu = true;
                        }
                        _ => {}
                    }
                }

                let mut state = self.state.borrow_mut();
                state.in_progress_window_controls = Some(window_controls);
                false
            }
            _ => false,
        }
    }

    pub fn handle_layersurface_event(&self, event: zwlr_layer_surface_v1::Event) -> bool {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                width,
                height,
                serial,
            } => {
                let size = if width == 0 || height == 0 {
                    None
                } else {
                    Some(size(px(width as f32), px(height as f32)))
                };

                let mut state = self.state.borrow_mut();
                state.in_progress_configure = Some(InProgressConfigure {
                    size,
                    fullscreen: false,
                    maximized: false,
                    resizing: false,
                    tiling: Tiling::default(),
                });
                drop(state);

                // just do the same thing we'd do as an xdg_surface
                self.handle_xdg_surface_event(xdg_surface::Event::Configure { serial });

                false
            }
            zwlr_layer_surface_v1::Event::Closed => {
                // unlike xdg, we don't have a choice here: the surface is closing.
                true
            }
            _ => false,
        }
    }

    #[allow(clippy::mutable_key_type)]
    pub fn handle_surface_event(
        &self,
        event: wl_surface::Event,
        outputs: HashMap<ObjectId, Output>,
    ) {
        let mut state = self.state.borrow_mut();

        match event {
            wl_surface::Event::Enter { output } => {
                let id = output.id();

                let Some(output) = outputs.get(&id) else {
                    return;
                };

                state.outputs.insert(id, output.clone());

                let scale = state.primary_output_scale();

                // We use `PreferredBufferScale` instead to set the scale if it's available
                if state.surface.version() < wl_surface::EVT_PREFERRED_BUFFER_SCALE_SINCE {
                    state.surface.set_buffer_scale(scale);
                    drop(state);
                    self.rescale(scale as f32);
                }
            }
            wl_surface::Event::Leave { output } => {
                state.outputs.remove(&output.id());

                let scale = state.primary_output_scale();

                // We use `PreferredBufferScale` instead to set the scale if it's available
                if state.surface.version() < wl_surface::EVT_PREFERRED_BUFFER_SCALE_SINCE {
                    state.surface.set_buffer_scale(scale);
                    drop(state);
                    self.rescale(scale as f32);
                }
            }
            wl_surface::Event::PreferredBufferScale { factor } => {
                // We use `WpFractionalScale` instead to set the scale if it's available
                if state.globals.fractional_scale_manager.is_none() {
                    state.surface.set_buffer_scale(factor);
                    drop(state);
                    self.rescale(factor as f32);
                }
            }
            _ => {}
        }
    }

    pub fn handle_ime(&self, ime: ImeInput) {
        if self.is_blocked() {
            return;
        }
        let mut state = self.state.borrow_mut();
        if let Some(mut input_handler) = state.input_handler.take() {
            drop(state);
            match ime {
                ImeInput::InsertText(text) => {
                    input_handler.replace_text_in_range(None, &text);
                }
                ImeInput::SetMarkedText(text) => {
                    input_handler.replace_and_mark_text_in_range(None, &text, None);
                }
                ImeInput::UnmarkText => {
                    input_handler.unmark_text();
                }
                ImeInput::DeleteText => {
                    if let Some(marked) = input_handler.marked_text_range() {
                        input_handler.replace_text_in_range(Some(marked), "");
                    }
                }
            }
            self.state.borrow_mut().input_handler = Some(input_handler);
        }
    }

    pub fn get_ime_area(&self) -> Option<Bounds<Pixels>> {
        let mut state = self.state.borrow_mut();
        let mut bounds: Option<Bounds<Pixels>> = None;
        if let Some(mut input_handler) = state.input_handler.take() {
            drop(state);
            if let Some(selection) = input_handler.marked_text_range() {
                bounds = input_handler.bounds_for_range(selection.start..selection.start);
            }
            self.state.borrow_mut().input_handler = Some(input_handler);
        }
        bounds
    }

    pub fn set_size_and_scale(&self, size: Option<Size<Pixels>>, scale: Option<f32>) {
        let (size, scale) = {
            let mut state = self.state.borrow_mut();
            if size.is_none_or(|size| size == state.bounds.size)
                && scale.is_none_or(|scale| scale == state.scale)
            {
                return;
            }
            if let Some(size) = size {
                state.bounds.size = size;
            }
            if let Some(scale) = scale {
                state.scale = scale;
            }
            let device_bounds = state.bounds.to_device_pixels(state.scale);
            state.renderer.update_drawable_size(device_bounds.size);
            (state.bounds.size, state.scale)
        };

        let callback = self.callbacks.borrow_mut().resize.take();
        if let Some(mut fun) = callback {
            fun(size, scale);
            self.callbacks.borrow_mut().resize = Some(fun);
        }

        {
            let state = self.state.borrow();
            if let Some(viewport) = &state.viewport {
                viewport
                    .set_destination(f32::from(size.width) as i32, f32::from(size.height) as i32);
            }
        }
    }

    pub fn resize(&self, size: Size<Pixels>) {
        self.set_size_and_scale(Some(size), None);
    }

    pub fn rescale(&self, scale: f32) {
        self.set_size_and_scale(None, Some(scale));
    }

    pub fn close(&self) {
        let state = self.state.borrow();
        let client = state.client.get_client();
        #[allow(clippy::mutable_key_type)]
        let children = state.children.clone();
        drop(state);

        for child in children {
            let mut client_state = client.borrow_mut();
            let window = get_window(&mut client_state, &child);
            drop(client_state);

            if let Some(child) = window {
                child.close();
            }
        }
        let mut callbacks = self.callbacks.borrow_mut();
        if let Some(fun) = callbacks.close.take() {
            fun()
        }
    }

    pub fn handle_input(&self, input: PlatformInput) {
        if self.is_blocked() {
            return;
        }
        let callback = self.callbacks.borrow_mut().input.take();
        if let Some(mut fun) = callback {
            let result = fun(input.clone());
            self.callbacks.borrow_mut().input = Some(fun);
            if !result.propagate {
                return;
            }
        }
        if let PlatformInput::KeyDown(event) = input
            && event.keystroke.modifiers.is_subset_of(&Modifiers::shift())
            && let Some(key_char) = &event.keystroke.key_char
        {
            let mut state = self.state.borrow_mut();
            if let Some(mut input_handler) = state.input_handler.take() {
                drop(state);
                input_handler.replace_text_in_range(None, key_char);
                self.state.borrow_mut().input_handler = Some(input_handler);
            }
        }
    }

    pub fn set_focused(&self, focus: bool) {
        self.state.borrow_mut().active = focus;
        let callback = self.callbacks.borrow_mut().active_status_change.take();
        if let Some(mut fun) = callback {
            fun(focus);
            self.callbacks.borrow_mut().active_status_change = Some(fun);
        }
    }

    pub fn set_hovered(&self, focus: bool) {
        let callback = self.callbacks.borrow_mut().hover_status_change.take();
        if let Some(mut fun) = callback {
            fun(focus);
            self.callbacks.borrow_mut().hover_status_change = Some(fun);
        }
    }

    pub fn set_appearance(&mut self, appearance: WindowAppearance) {
        self.state.borrow_mut().appearance = appearance;

        let callback = self.callbacks.borrow_mut().appearance_changed.take();
        if let Some(mut fun) = callback {
            fun();
            self.callbacks.borrow_mut().appearance_changed = Some(fun);
        }
    }

    pub fn primary_output_scale(&self) -> i32 {
        self.state.borrow_mut().primary_output_scale()
    }
}

pub(super) fn extract_states<'a, S: TryFrom<u32> + 'a>(states: &'a [u8]) -> impl Iterator<Item = S> + 'a
where
    <S as TryFrom<u32>>::Error: 'a,
{
    states
        .chunks_exact(4)
        .flat_map(TryInto::<[u8; 4]>::try_into)
        .map(u32::from_ne_bytes)
        .flat_map(S::try_from)
}

impl rwh::HasWindowHandle for WaylandWindow {
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        let surface = self.0.surface().id().as_ptr() as *mut libc::c_void;
        let c_ptr = NonNull::new(surface).ok_or(rwh::HandleError::Unavailable)?;
        let handle = rwh::WaylandWindowHandle::new(c_ptr);
        let raw_handle = rwh::RawWindowHandle::Wayland(handle);
        Ok(unsafe { rwh::WindowHandle::borrow_raw(raw_handle) })
    }
}

impl rwh::HasDisplayHandle for WaylandWindow {
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, rwh::HandleError> {
        let display = self
            .0
            .surface()
            .backend()
            .upgrade()
            .ok_or(rwh::HandleError::Unavailable)?
            .display_ptr() as *mut libc::c_void;

        let c_ptr = NonNull::new(display).ok_or(rwh::HandleError::Unavailable)?;
        let handle = rwh::WaylandDisplayHandle::new(c_ptr);
        let raw_handle = rwh::RawDisplayHandle::Wayland(handle);
        Ok(unsafe { rwh::DisplayHandle::borrow_raw(raw_handle) })
    }
}

