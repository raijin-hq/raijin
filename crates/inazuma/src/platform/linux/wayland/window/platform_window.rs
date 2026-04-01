use super::*;

impl PlatformWindow for WaylandWindow {
    fn bounds(&self) -> Bounds<Pixels> {
        self.borrow().bounds
    }

    fn is_maximized(&self) -> bool {
        self.borrow().maximized
    }

    fn window_bounds(&self) -> WindowBounds {
        let state = self.borrow();
        if state.fullscreen {
            WindowBounds::Fullscreen(state.window_bounds)
        } else if state.maximized {
            WindowBounds::Maximized(state.window_bounds)
        } else {
            drop(state);
            WindowBounds::Windowed(self.bounds())
        }
    }

    fn inner_window_bounds(&self) -> WindowBounds {
        let state = self.borrow();
        if state.fullscreen {
            WindowBounds::Fullscreen(state.window_bounds)
        } else if state.maximized {
            WindowBounds::Maximized(state.window_bounds)
        } else {
            let inset = state.inset();
            drop(state);
            WindowBounds::Windowed(self.bounds().inset(inset))
        }
    }

    fn content_size(&self) -> Size<Pixels> {
        self.borrow().bounds.size
    }

    fn resize(&mut self, size: Size<Pixels>) {
        let state = self.borrow();
        let state_ptr = self.0.clone();

        // Keep window geometry consistent with configure handling. On Wayland, window geometry is
        // surface-local: resizing should not attempt to translate the window; the compositor
        // controls placement. We also account for client-side decoration insets and tiling.
        let window_geometry = inset_by_tiling(
            Bounds {
                origin: Point::default(),
                size,
            },
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

        state
            .globals
            .executor
            .spawn(async move { state_ptr.resize(size) })
            .detach();
    }

    fn scale_factor(&self) -> f32 {
        self.borrow().scale
    }

    fn appearance(&self) -> WindowAppearance {
        self.borrow().appearance
    }

    fn display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        let state = self.borrow();
        state.display.as_ref().map(|(id, display)| {
            Rc::new(WaylandDisplay {
                id: id.clone(),
                name: display.name.clone(),
                bounds: display.bounds.to_pixels(state.scale),
            }) as Rc<dyn PlatformDisplay>
        })
    }

    fn mouse_position(&self) -> Point<Pixels> {
        self.borrow()
            .client
            .get_client()
            .borrow()
            .mouse_location
            .unwrap_or_default()
    }

    fn modifiers(&self) -> Modifiers {
        self.borrow().client.get_client().borrow().modifiers
    }

    fn capslock(&self) -> Capslock {
        self.borrow().client.get_client().borrow().capslock
    }

    fn set_input_handler(&mut self, input_handler: PlatformInputHandler) {
        self.borrow_mut().input_handler = Some(input_handler);
    }

    fn take_input_handler(&mut self) -> Option<PlatformInputHandler> {
        self.borrow_mut().input_handler.take()
    }

    fn prompt(
        &self,
        _level: PromptLevel,
        _msg: &str,
        _detail: Option<&str>,
        _answers: &[PromptButton],
    ) -> Option<Receiver<usize>> {
        None
    }

    fn activate(&self) {
        // Try to request an activation token. Even though the activation is likely going to be rejected,
        // KWin and Mutter can use the app_id to visually indicate we're requesting attention.
        let state = self.borrow();
        if let (Some(activation), Some(app_id)) = (&state.globals.activation, state.app_id.clone())
        {
            state.client.set_pending_activation(state.surface.id());
            let token = activation.get_activation_token(&state.globals.qh, ());
            // The serial isn't exactly important here, since the activation is probably going to be rejected anyway.
            let serial = state.client.get_serial(SerialKind::MousePress);
            token.set_app_id(app_id);
            token.set_serial(serial, &state.globals.seat);
            token.set_surface(&state.surface);
            token.commit();
        }
    }

    fn is_active(&self) -> bool {
        self.borrow().active
    }

    fn is_hovered(&self) -> bool {
        self.borrow().hovered
    }

    fn set_title(&mut self, title: &str) {
        if let Some(toplevel) = self.borrow().surface_state.toplevel() {
            toplevel.set_title(title.to_string());
        }
    }

    fn set_app_id(&mut self, app_id: &str) {
        let mut state = self.borrow_mut();
        if let Some(toplevel) = state.surface_state.toplevel() {
            toplevel.set_app_id(app_id.to_owned());
        }
        state.app_id = Some(app_id.to_owned());
    }

    fn set_background_appearance(&self, background_appearance: WindowBackgroundAppearance) {
        let mut state = self.borrow_mut();
        state.background_appearance = background_appearance;
        update_window(state);
    }

    fn background_appearance(&self) -> WindowBackgroundAppearance {
        self.borrow().background_appearance
    }

    fn is_subpixel_rendering_supported(&self) -> bool {
        let client = self.borrow().client.get_client();
        let state = client.borrow();
        state
            .gpu_context
            .borrow()
            .as_ref()
            .is_some_and(|ctx| ctx.supports_dual_source_blending())
    }

    fn minimize(&self) {
        if let Some(toplevel) = self.borrow().surface_state.toplevel() {
            toplevel.set_minimized();
        }
    }

    fn zoom(&self) {
        let state = self.borrow();
        if let Some(toplevel) = state.surface_state.toplevel() {
            if !state.maximized {
                toplevel.set_maximized();
            } else {
                toplevel.unset_maximized();
            }
        }
    }

    fn toggle_fullscreen(&self) {
        let state = self.borrow();
        if let Some(toplevel) = state.surface_state.toplevel() {
            if !state.fullscreen {
                toplevel.set_fullscreen(None);
            } else {
                toplevel.unset_fullscreen();
            }
        }
    }

    fn is_fullscreen(&self) -> bool {
        self.borrow().fullscreen
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
        self.0.callbacks.borrow_mut().hover_status_change = Some(callback);
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
        let mut state = self.borrow_mut();

        if state.renderer.device_lost() {
            let raw_window = RawWindow {
                window: state.surface.id().as_ptr().cast::<std::ffi::c_void>(),
                display: state
                    .surface
                    .backend()
                    .upgrade()
                    .unwrap()
                    .display_ptr()
                    .cast::<std::ffi::c_void>(),
            };
            state.renderer.recover(&raw_window).unwrap_or_else(|err| {
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

        state.renderer.draw(scene);
    }

    fn completed_frame(&self) {
        let state = self.borrow();
        state.surface.commit();
    }

    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        let state = self.borrow();
        state.renderer.sprite_atlas().clone()
    }

    fn show_window_menu(&self, position: Point<Pixels>) {
        let state = self.borrow();
        let serial = state.client.get_serial(SerialKind::MousePress);
        if let Some(toplevel) = state.surface_state.toplevel() {
            toplevel.show_window_menu(
                &state.globals.seat,
                serial,
                f32::from(position.x) as i32,
                f32::from(position.y) as i32,
            );
        }
    }

    fn start_window_move(&self) {
        let state = self.borrow();
        let serial = state.client.get_serial(SerialKind::MousePress);
        if let Some(toplevel) = state.surface_state.toplevel() {
            toplevel._move(&state.globals.seat, serial);
        }
    }

    fn start_window_resize(&self, edge: inazuma::ResizeEdge) {
        let state = self.borrow();
        if let Some(toplevel) = state.surface_state.toplevel() {
            toplevel.resize(
                &state.globals.seat,
                state.client.get_serial(SerialKind::MousePress),
                edge.to_xdg(),
            )
        }
    }

    fn window_decorations(&self) -> Decorations {
        let state = self.borrow();
        match state.decorations {
            WindowDecorations::Server => Decorations::Server,
            WindowDecorations::Client => Decorations::Client {
                tiling: state.tiling,
            },
        }
    }

    fn request_decorations(&self, decorations: WindowDecorations) {
        let mut state = self.borrow_mut();
        match state.surface_state.decoration().as_ref() {
            Some(decoration) => {
                decoration.set_mode(decorations.to_xdg());
                state.decorations = decorations;
                update_window(state);
            }
            None => {
                if matches!(decorations, WindowDecorations::Server) {
                    log::info!(
                        "Server-side decorations requested, but the Wayland server does not support them. Falling back to client-side decorations."
                    );
                }
                state.decorations = WindowDecorations::Client;
                update_window(state);
            }
        }
    }

    fn window_controls(&self) -> WindowControls {
        self.borrow().window_controls
    }

    fn set_client_inset(&self, inset: Pixels) {
        let mut state = self.borrow_mut();
        if Some(inset) != state.client_inset {
            state.client_inset = Some(inset);
            update_window(state);
        }
    }

    fn update_ime_position(&self, bounds: Bounds<Pixels>) {
        let state = self.borrow();
        state.client.update_ime_position(bounds);
    }

    fn gpu_specs(&self) -> Option<GpuSpecs> {
        self.borrow().renderer.gpu_specs().into()
    }
}

pub(super) fn update_window(mut state: RefMut<WaylandWindowState>) {
    let opaque = !state.is_transparent();

    state.renderer.update_transparency(!opaque);
    let opaque_area = state.window_bounds.map(|v| f32::from(v) as i32);
    opaque_area.inset(f32::from(state.inset()) as i32);

    let region = state
        .globals
        .compositor
        .create_region(&state.globals.qh, ());
    region.add(
        opaque_area.origin.x,
        opaque_area.origin.y,
        opaque_area.size.width,
        opaque_area.size.height,
    );

    // Note that rounded corners make this rectangle API hard to work with.
    // As this is common when using CSD, let's just disable this API.
    if state.background_appearance == WindowBackgroundAppearance::Opaque
        && state.decorations == WindowDecorations::Server
    {
        // Promise the compositor that this region of the window surface
        // contains no transparent pixels. This allows the compositor to skip
        // updating whatever is behind the surface for better performance.
        state.surface.set_opaque_region(Some(&region));
    } else {
        state.surface.set_opaque_region(None);
    }

    if let Some(ref blur_manager) = state.globals.blur_manager {
        if state.background_appearance == WindowBackgroundAppearance::Blurred {
            if state.blur.is_none() {
                let blur = blur_manager.create(&state.surface, &state.globals.qh, ());
                state.blur = Some(blur);
            }
            state.blur.as_ref().unwrap().commit();
        } else {
            // It probably doesn't hurt to clear the blur for opaque windows
            blur_manager.unset(&state.surface);
            if let Some(b) = state.blur.take() {
                b.release()
            }
        }
    }

    region.destroy();
}

pub(crate) trait WindowDecorationsExt {
    fn to_xdg(self) -> zxdg_toplevel_decoration_v1::Mode;
}

impl WindowDecorationsExt for WindowDecorations {
    fn to_xdg(self) -> zxdg_toplevel_decoration_v1::Mode {
        match self {
            WindowDecorations::Client => zxdg_toplevel_decoration_v1::Mode::ClientSide,
            WindowDecorations::Server => zxdg_toplevel_decoration_v1::Mode::ServerSide,
        }
    }
}

pub(crate) trait ResizeEdgeWaylandExt {
    fn to_xdg(self) -> xdg_toplevel::ResizeEdge;
}

impl ResizeEdgeWaylandExt for ResizeEdge {
    fn to_xdg(self) -> xdg_toplevel::ResizeEdge {
        match self {
            ResizeEdge::Top => xdg_toplevel::ResizeEdge::Top,
            ResizeEdge::TopRight => xdg_toplevel::ResizeEdge::TopRight,
            ResizeEdge::Right => xdg_toplevel::ResizeEdge::Right,
            ResizeEdge::BottomRight => xdg_toplevel::ResizeEdge::BottomRight,
            ResizeEdge::Bottom => xdg_toplevel::ResizeEdge::Bottom,
            ResizeEdge::BottomLeft => xdg_toplevel::ResizeEdge::BottomLeft,
            ResizeEdge::Left => xdg_toplevel::ResizeEdge::Left,
            ResizeEdge::TopLeft => xdg_toplevel::ResizeEdge::TopLeft,
        }
    }
}

/// The configuration event is in terms of the window geometry, which we are constantly
/// updating to account for the client decorations. But that's not the area we want to render
/// to, due to our intrusize CSD. So, here we calculate the 'actual' size, by adding back in the insets
pub(super) fn compute_outer_size(
    inset: Pixels,
    new_size: Option<Size<Pixels>>,
    tiling: Tiling,
) -> Option<Size<Pixels>> {
    new_size.map(|mut new_size| {
        if !tiling.top {
            new_size.height += inset;
        }
        if !tiling.bottom {
            new_size.height += inset;
        }
        if !tiling.left {
            new_size.width += inset;
        }
        if !tiling.right {
            new_size.width += inset;
        }

        new_size
    })
}

pub(super) fn inset_by_tiling(mut bounds: Bounds<Pixels>, inset: Pixels, tiling: Tiling) -> Bounds<Pixels> {
    if !tiling.top {
        bounds.origin.y += inset;
        bounds.size.height -= inset;
    }
    if !tiling.bottom {
        bounds.size.height -= inset;
    }
    if !tiling.left {
        bounds.origin.x += inset;
        bounds.size.width -= inset;
    }
    if !tiling.right {
        bounds.size.width -= inset;
    }

    bounds
}
