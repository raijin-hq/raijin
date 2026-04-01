use super::*;

impl PlatformWindow for X11Window {
    fn bounds(&self) -> Bounds<Pixels> {
        self.0.state.borrow().bounds
    }

    fn is_maximized(&self) -> bool {
        let state = self.0.state.borrow();

        // A maximized window that gets minimized will still retain its maximized state.
        !state.hidden && state.maximized_vertical && state.maximized_horizontal
    }

    fn window_bounds(&self) -> WindowBounds {
        let state = self.0.state.borrow();
        if self.is_maximized() {
            WindowBounds::Maximized(state.bounds)
        } else {
            WindowBounds::Windowed(state.bounds)
        }
    }

    fn inner_window_bounds(&self) -> WindowBounds {
        let state = self.0.state.borrow();
        if self.is_maximized() {
            WindowBounds::Maximized(state.bounds)
        } else {
            let mut bounds = state.bounds;
            let [left, right, top, bottom] = state.last_insets;

            let [left, right, top, bottom] = [
                px((left as f32) / state.scale_factor),
                px((right as f32) / state.scale_factor),
                px((top as f32) / state.scale_factor),
                px((bottom as f32) / state.scale_factor),
            ];

            bounds.origin.x += left;
            bounds.origin.y += top;
            bounds.size.width -= left + right;
            bounds.size.height -= top + bottom;

            WindowBounds::Windowed(bounds)
        }
    }

    fn content_size(&self) -> Size<Pixels> {
        // After the wgpu migration, X11WindowState::content_size() returns logical pixels
        // (bounds.size is already divided by scale_factor in set_bounds), so no further
        // division is needed here. This matches the Wayland implementation.
        self.0.state.borrow().content_size()
    }

    fn resize(&mut self, size: Size<Pixels>) {
        let state = self.0.state.borrow();
        let size = size.to_device_pixels(state.scale_factor);
        let width = size.width.0 as u32;
        let height = size.height.0 as u32;

        check_reply(
            || {
                format!(
                    "X11 ConfigureWindow failed. width: {}, height: {}",
                    width, height
                )
            },
            self.0.xcb.configure_window(
                self.0.x_window,
                &xproto::ConfigureWindowAux::new()
                    .width(width)
                    .height(height),
            ),
        )
        .log_err();
        xcb_flush(&self.0.xcb);
    }

    fn scale_factor(&self) -> f32 {
        self.0.state.borrow().scale_factor
    }

    fn appearance(&self) -> WindowAppearance {
        self.0.state.borrow().appearance
    }

    fn display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(self.0.state.borrow().display.clone())
    }

    fn mouse_position(&self) -> Point<Pixels> {
        get_reply(
            || "X11 QueryPointer failed.",
            self.0.xcb.query_pointer(self.0.x_window),
        )
        .log_err()
        .map_or(Point::new(Pixels::ZERO, Pixels::ZERO), |reply| {
            Point::new((reply.root_x as u32).into(), (reply.root_y as u32).into())
        })
    }

    fn modifiers(&self) -> Modifiers {
        self.0
            .state
            .borrow()
            .client
            .0
            .upgrade()
            .map(|ref_cell| ref_cell.borrow().modifiers)
            .unwrap_or_default()
    }

    fn capslock(&self) -> inazuma::Capslock {
        self.0
            .state
            .borrow()
            .client
            .0
            .upgrade()
            .map(|ref_cell| ref_cell.borrow().capslock)
            .unwrap_or_default()
    }

    fn set_input_handler(&mut self, input_handler: PlatformInputHandler) {
        self.0.state.borrow_mut().input_handler = Some(input_handler);
    }

    fn take_input_handler(&mut self) -> Option<PlatformInputHandler> {
        self.0.state.borrow_mut().input_handler.take()
    }

    fn prompt(
        &self,
        _level: PromptLevel,
        _msg: &str,
        _detail: Option<&str>,
        _answers: &[PromptButton],
    ) -> Option<futures::channel::oneshot::Receiver<usize>> {
        None
    }

    fn activate(&self) {
        let data = [1, xproto::Time::CURRENT_TIME.into(), 0, 0, 0];
        let message = xproto::ClientMessageEvent::new(
            32,
            self.0.x_window,
            self.0.state.borrow().atoms._NET_ACTIVE_WINDOW,
            data,
        );
        self.0
            .xcb
            .send_event(
                false,
                self.0.state.borrow().x_root_window,
                xproto::EventMask::SUBSTRUCTURE_REDIRECT | xproto::EventMask::SUBSTRUCTURE_NOTIFY,
                message,
            )
            .log_err();
        self.0
            .xcb
            .set_input_focus(
                xproto::InputFocus::POINTER_ROOT,
                self.0.x_window,
                xproto::Time::CURRENT_TIME,
            )
            .log_err();
        xcb_flush(&self.0.xcb);
    }

    fn is_active(&self) -> bool {
        self.0.state.borrow().active
    }

    fn is_hovered(&self) -> bool {
        self.0.state.borrow().hovered
    }

    fn set_title(&mut self, title: &str) {
        check_reply(
            || "X11 ChangeProperty8 on WM_NAME failed.",
            self.0.xcb.change_property8(
                xproto::PropMode::REPLACE,
                self.0.x_window,
                xproto::AtomEnum::WM_NAME,
                xproto::AtomEnum::STRING,
                title.as_bytes(),
            ),
        )
        .log_err();

        check_reply(
            || "X11 ChangeProperty8 on _NET_WM_NAME failed.",
            self.0.xcb.change_property8(
                xproto::PropMode::REPLACE,
                self.0.x_window,
                self.0.state.borrow().atoms._NET_WM_NAME,
                self.0.state.borrow().atoms.UTF8_STRING,
                title.as_bytes(),
            ),
        )
        .log_err();
        xcb_flush(&self.0.xcb);
    }

    fn set_app_id(&mut self, app_id: &str) {
        let mut data = Vec::with_capacity(app_id.len() * 2 + 1);
        data.extend(app_id.bytes()); // instance https://unix.stackexchange.com/a/494170
        data.push(b'\0');
        data.extend(app_id.bytes()); // class

        check_reply(
            || "X11 ChangeProperty8 for WM_CLASS failed.",
            self.0.xcb.change_property8(
                xproto::PropMode::REPLACE,
                self.0.x_window,
                xproto::AtomEnum::WM_CLASS,
                xproto::AtomEnum::STRING,
                &data,
            ),
        )
        .log_err();
    }

    fn map_window(&mut self) -> anyhow::Result<()> {
        check_reply(
            || "X11 MapWindow failed.",
            self.0.xcb.map_window(self.0.x_window),
        )?;
        Ok(())
    }

    fn set_background_appearance(&self, background_appearance: WindowBackgroundAppearance) {
        let mut state = self.0.state.borrow_mut();
        state.background_appearance = background_appearance;
        let transparent = state.is_transparent();
        state.renderer.update_transparency(transparent);
    }

    fn background_appearance(&self) -> WindowBackgroundAppearance {
        self.0.state.borrow().background_appearance
    }

    fn is_subpixel_rendering_supported(&self) -> bool {
        self.0
            .state
            .borrow()
            .client
            .0
            .upgrade()
            .map(|ref_cell| {
                let state = ref_cell.borrow();
                state
                    .gpu_context
                    .borrow()
                    .as_ref()
                    .is_some_and(|ctx| ctx.supports_dual_source_blending())
            })
            .unwrap_or_default()
    }

    fn minimize(&self) {
        let state = self.0.state.borrow();
        const WINDOW_ICONIC_STATE: u32 = 3;
        let message = ClientMessageEvent::new(
            32,
            self.0.x_window,
            state.atoms.WM_CHANGE_STATE,
            [WINDOW_ICONIC_STATE, 0, 0, 0, 0],
        );
        check_reply(
            || "X11 SendEvent to minimize window failed.",
            self.0.xcb.send_event(
                false,
                state.x_root_window,
                xproto::EventMask::SUBSTRUCTURE_REDIRECT | xproto::EventMask::SUBSTRUCTURE_NOTIFY,
                message,
            ),
        )
        .log_err();
    }

    fn zoom(&self) {
        let state = self.0.state.borrow();
        self.set_wm_hints(
            || "X11 SendEvent to maximize a window failed.",
            WmHintPropertyState::Toggle,
            state.atoms._NET_WM_STATE_MAXIMIZED_VERT,
            state.atoms._NET_WM_STATE_MAXIMIZED_HORZ,
        )
        .log_err();
    }

    fn toggle_fullscreen(&self) {
        let state = self.0.state.borrow();
        self.set_wm_hints(
            || "X11 SendEvent to fullscreen a window failed.",
            WmHintPropertyState::Toggle,
            state.atoms._NET_WM_STATE_FULLSCREEN,
            xproto::AtomEnum::NONE.into(),
        )
        .log_err();
    }

    fn is_fullscreen(&self) -> bool {
        self.0.state.borrow().fullscreen
    }

    fn on_request_frame(&self, callback: Box<dyn FnMut(RequestFrameOptions)>) {
        self.0.callbacks.borrow_mut().request_frame = Some(callback);
    }

    fn on_input(&self, callback: Box<dyn FnMut(PlatformInput) -> inazuma::DispatchEventResult>) {
        self.0.callbacks.borrow_mut().input = Some(callback);
    }

    fn on_active_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        self.0.callbacks.borrow_mut().active_status_change = Some(callback);
    }

    fn on_hover_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        self.0.callbacks.borrow_mut().hovered_status_change = Some(callback);
    }

    fn on_resize(&self, callback: Box<dyn FnMut(Size<Pixels>, f32)>) {
        self.0.callbacks.borrow_mut().resize = Some(callback);
    }

    fn on_moved(&self, callback: Box<dyn FnMut()>) {
        self.0.callbacks.borrow_mut().moved = Some(callback);
    }

    fn on_should_close(&self, callback: Box<dyn FnMut() -> bool>) {
        self.0.callbacks.borrow_mut().should_close = Some(callback);
    }

    fn on_close(&self, callback: Box<dyn FnOnce()>) {
        self.0.callbacks.borrow_mut().close = Some(callback);
    }

    fn on_hit_test_window_control(&self, _callback: Box<dyn FnMut() -> Option<WindowControlArea>>) {
    }

    fn on_appearance_changed(&self, callback: Box<dyn FnMut()>) {
        self.0.callbacks.borrow_mut().appearance_changed = Some(callback);
    }

    fn draw(&self, scene: &Scene) {
        let mut inner = self.0.state.borrow_mut();

        if inner.renderer.device_lost() {
            let raw_window = RawWindow {
                connection: as_raw_xcb_connection::AsRawXcbConnection::as_raw_xcb_connection(
                    &*self.0.xcb,
                ) as *mut _,
                screen_id: inner.x_screen_index,
                window_id: self.0.x_window,
                visual_id: inner.visual_id,
            };
            inner.renderer.recover(&raw_window).unwrap_or_else(|err| {
                panic!(
                    "GPU device lost and recovery failed. \
                        This may happen after system suspend/resume. \
                        Please restart the application.\n\nError: {err}"
                )
            });

            // The current scene references atlas textures that were cleared during recovery.
            // Skip this frame and let the next frame rebuild the scene with fresh textures.
            return;
        }

        inner.renderer.draw(scene);
    }

    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        let inner = self.0.state.borrow();
        inner.renderer.sprite_atlas().clone()
    }

    fn show_window_menu(&self, position: Point<Pixels>) {
        let state = self.0.state.borrow();

        check_reply(
            || "X11 UngrabPointer failed.",
            self.0.xcb.ungrab_pointer(x11rb::CURRENT_TIME),
        )
        .log_err();

        let Some(coords) = self.get_root_position(position).log_err() else {
            return;
        };
        let message = ClientMessageEvent::new(
            32,
            self.0.x_window,
            state.atoms._GTK_SHOW_WINDOW_MENU,
            [
                XINPUT_ALL_DEVICE_GROUPS as u32,
                coords.dst_x as u32,
                coords.dst_y as u32,
                0,
                0,
            ],
        );
        check_reply(
            || "X11 SendEvent to show window menu failed.",
            self.0.xcb.send_event(
                false,
                state.x_root_window,
                xproto::EventMask::SUBSTRUCTURE_REDIRECT | xproto::EventMask::SUBSTRUCTURE_NOTIFY,
                message,
            ),
        )
        .log_err();
    }

    fn start_window_move(&self) {
        const MOVERESIZE_MOVE: u32 = 8;
        self.send_moveresize(MOVERESIZE_MOVE).log_err();
    }

    fn start_window_resize(&self, edge: ResizeEdge) {
        self.send_moveresize(resize_edge_to_moveresize(edge))
            .log_err();
    }

    fn window_decorations(&self) -> inazuma::Decorations {
        let state = self.0.state.borrow();

        // Client window decorations require compositor support
        if !state.client_side_decorations_supported {
            return Decorations::Server;
        }

        match state.decorations {
            WindowDecorations::Server => Decorations::Server,
            WindowDecorations::Client => {
                let tiling = if state.fullscreen {
                    Tiling::tiled()
                } else if let Some(edge_constraints) = &state.edge_constraints {
                    edge_constraints.to_tiling()
                } else {
                    // https://source.chromium.org/chromium/chromium/src/+/main:ui/ozone/platform/x11/x11_window.cc;l=2519;drc=1f14cc876cc5bf899d13284a12c451498219bb2d
                    Tiling {
                        top: state.maximized_vertical,
                        bottom: state.maximized_vertical,
                        left: state.maximized_horizontal,
                        right: state.maximized_horizontal,
                    }
                };
                Decorations::Client { tiling }
            }
        }
    }

    fn set_client_inset(&self, inset: Pixels) {
        let mut state = self.0.state.borrow_mut();

        let dp = (f32::from(inset) * state.scale_factor) as u32;

        let insets = if state.fullscreen {
            [0, 0, 0, 0]
        } else if let Some(edge_constraints) = &state.edge_constraints {
            let left = if edge_constraints.left_tiled { 0 } else { dp };
            let top = if edge_constraints.top_tiled { 0 } else { dp };
            let right = if edge_constraints.right_tiled { 0 } else { dp };
            let bottom = if edge_constraints.bottom_tiled { 0 } else { dp };

            [left, right, top, bottom]
        } else {
            let (left, right) = if state.maximized_horizontal {
                (0, 0)
            } else {
                (dp, dp)
            };
            let (top, bottom) = if state.maximized_vertical {
                (0, 0)
            } else {
                (dp, dp)
            };
            [left, right, top, bottom]
        };

        if state.last_insets != insets {
            state.last_insets = insets;

            check_reply(
                || "X11 ChangeProperty for _GTK_FRAME_EXTENTS failed.",
                self.0.xcb.change_property(
                    xproto::PropMode::REPLACE,
                    self.0.x_window,
                    state.atoms._GTK_FRAME_EXTENTS,
                    xproto::AtomEnum::CARDINAL,
                    size_of::<u32>() as u8 * 8,
                    4,
                    bytemuck::cast_slice::<u32, u8>(&insets),
                ),
            )
            .log_err();
        }
    }

    fn request_decorations(&self, mut decorations: inazuma::WindowDecorations) {
        let mut state = self.0.state.borrow_mut();

        if matches!(decorations, inazuma::WindowDecorations::Client)
            && !state.client_side_decorations_supported
        {
            log::info!(
                "x11: no compositor present, falling back to server-side window decorations"
            );
            decorations = inazuma::WindowDecorations::Server;
        }

        // https://github.com/rust-windowing/winit/blob/master/src/platform_impl/linux/x11/util/hint.rs#L53-L87
        let hints_data: [u32; 5] = match decorations {
            WindowDecorations::Server => [1 << 1, 0, 1, 0, 0],
            WindowDecorations::Client => [1 << 1, 0, 0, 0, 0],
        };

        let success = check_reply(
            || "X11 ChangeProperty for _MOTIF_WM_HINTS failed.",
            self.0.xcb.change_property(
                xproto::PropMode::REPLACE,
                self.0.x_window,
                state.atoms._MOTIF_WM_HINTS,
                state.atoms._MOTIF_WM_HINTS,
                size_of::<u32>() as u8 * 8,
                5,
                bytemuck::cast_slice::<u32, u8>(&hints_data),
            ),
        )
        .log_err();

        let Some(()) = success else {
            return;
        };

        match decorations {
            WindowDecorations::Server => {
                state.decorations = WindowDecorations::Server;
                let is_transparent = state.is_transparent();
                state.renderer.update_transparency(is_transparent);
            }
            WindowDecorations::Client => {
                state.decorations = WindowDecorations::Client;
                let is_transparent = state.is_transparent();
                state.renderer.update_transparency(is_transparent);
            }
        }

        drop(state);
        let mut callbacks = self.0.callbacks.borrow_mut();
        if let Some(appearance_changed) = callbacks.appearance_changed.as_mut() {
            appearance_changed();
        }
    }

    fn update_ime_position(&self, bounds: Bounds<Pixels>) {
        let state = self.0.state.borrow();
        let client = state.client.clone();
        drop(state);
        client.update_ime_position(bounds);
    }

    fn gpu_specs(&self) -> Option<GpuSpecs> {
        self.0.state.borrow().renderer.gpu_specs().into()
    }
}
