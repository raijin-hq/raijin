use super::*;

pub(crate) struct Callbacks {
    pub(super) request_frame: Option<Box<dyn FnMut(RequestFrameOptions)>>,
    pub(super) input: Option<Box<dyn FnMut(inazuma::PlatformInput) -> inazuma::DispatchEventResult>>,
    pub(super) active_status_change: Option<Box<dyn FnMut(bool)>>,
    pub(super) hover_status_change: Option<Box<dyn FnMut(bool)>>,
    pub(super) resize: Option<Box<dyn FnMut(Size<Pixels>, f32)>>,
    pub(super) moved: Option<Box<dyn FnMut()>>,
    pub(super) should_close: Option<Box<dyn FnMut() -> bool>>,
    pub(super) close: Option<Box<dyn FnOnce()>>,
    pub(super) appearance_changed: Option<Box<dyn FnMut()>>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct RawWindow {
    pub(super) window: *mut c_void,
    pub(super) display: *mut c_void,
}

// Safety: The raw pointers in RawWindow point to Wayland surface/display
// which are valid for the window's lifetime. These are used only for
// passing to wgpu which needs Send+Sync for surface creation.
unsafe impl Send for RawWindow {}
unsafe impl Sync for RawWindow {}

impl rwh::HasWindowHandle for RawWindow {
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        let window = NonNull::new(self.window).unwrap();
        let handle = rwh::WaylandWindowHandle::new(window);
        Ok(unsafe { rwh::WindowHandle::borrow_raw(handle.into()) })
    }
}
impl rwh::HasDisplayHandle for RawWindow {
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, rwh::HandleError> {
        let display = NonNull::new(self.display).unwrap();
        let handle = rwh::WaylandDisplayHandle::new(display);
        Ok(unsafe { rwh::DisplayHandle::borrow_raw(handle.into()) })
    }
}

#[derive(Debug)]
pub(super) struct InProgressConfigure {
    pub(super) size: Option<Size<Pixels>>,
    pub(super) fullscreen: bool,
    pub(super) maximized: bool,
    pub(super) resizing: bool,
    pub(super) tiling: Tiling,
}

pub struct WaylandWindowState {
    pub(super) surface_state: WaylandSurfaceState,
    pub(super) acknowledged_first_configure: bool,
    pub(super) parent: Option<WaylandWindowStatePtr>,
    pub(super) children: FxHashSet<ObjectId>,
    pub surface: wl_surface::WlSurface,
    pub(super) app_id: Option<String>,
    pub(super) appearance: WindowAppearance,
    pub(super) blur: Option<org_kde_kwin_blur::OrgKdeKwinBlur>,
    pub(super) viewport: Option<wp_viewport::WpViewport>,
    pub(super) outputs: HashMap<ObjectId, Output>,
    pub(super) display: Option<(ObjectId, Output)>,
    pub(super) globals: Globals,
    pub(super) renderer: WgpuRenderer,
    pub(super) bounds: Bounds<Pixels>,
    pub(super) scale: f32,
    pub(super) input_handler: Option<PlatformInputHandler>,
    pub(super) decorations: WindowDecorations,
    pub(super) background_appearance: WindowBackgroundAppearance,
    pub(super) fullscreen: bool,
    pub(super) maximized: bool,
    pub(super) tiling: Tiling,
    pub(super) window_bounds: Bounds<Pixels>,
    pub(super) client: WaylandClientStatePtr,
    pub(super) handle: AnyWindowHandle,
    pub(super) active: bool,
    pub(super) hovered: bool,
    pub(super) in_progress_configure: Option<InProgressConfigure>,
    pub(super) resize_throttle: bool,
    pub(super) in_progress_window_controls: Option<WindowControls>,
    pub(super) window_controls: WindowControls,
    pub(super) client_inset: Option<Pixels>,
}

pub enum WaylandSurfaceState {
    Xdg(WaylandXdgSurfaceState),
    LayerShell(WaylandLayerSurfaceState),
}

impl WaylandSurfaceState {
    fn new(
        surface: &wl_surface::WlSurface,
        globals: &Globals,
        params: &WindowParams,
        parent: Option<WaylandWindowStatePtr>,
        target_output: Option<wl_output::WlOutput>,
    ) -> anyhow::Result<Self> {
        // For layer_shell windows, create a layer surface instead of an xdg surface
        if let WindowKind::LayerShell(options) = &params.kind {
            let Some(layer_shell) = globals.layer_shell.as_ref() else {
                return Err(LayerShellNotSupportedError.into());
            };

            let layer_surface = layer_shell.get_layer_surface(
                &surface,
                target_output.as_ref(),
                super::layer_shell::wayland_layer(options.layer),
                options.namespace.clone(),
                &globals.qh,
                surface.id(),
            );

            let width = f32::from(params.bounds.size.width);
            let height = f32::from(params.bounds.size.height);
            layer_surface.set_size(width as u32, height as u32);

            layer_surface.set_anchor(super::layer_shell::wayland_anchor(options.anchor));
            layer_surface.set_keyboard_interactivity(
                super::layer_shell::wayland_keyboard_interactivity(options.keyboard_interactivity),
            );

            if let Some(margin) = options.margin {
                layer_surface.set_margin(
                    f32::from(margin.0) as i32,
                    f32::from(margin.1) as i32,
                    f32::from(margin.2) as i32,
                    f32::from(margin.3) as i32,
                )
            }

            if let Some(exclusive_zone) = options.exclusive_zone {
                layer_surface.set_exclusive_zone(f32::from(exclusive_zone) as i32);
            }

            if let Some(exclusive_edge) = options.exclusive_edge {
                layer_surface
                    .set_exclusive_edge(super::layer_shell::wayland_anchor(exclusive_edge));
            }

            return Ok(WaylandSurfaceState::LayerShell(WaylandLayerSurfaceState {
                layer_surface,
            }));
        }

        // All other WindowKinds result in a regular xdg surface
        let xdg_surface = globals
            .wm_base
            .get_xdg_surface(&surface, &globals.qh, surface.id());

        let toplevel = xdg_surface.get_toplevel(&globals.qh, surface.id());
        let xdg_parent = parent.as_ref().and_then(|w| w.toplevel());

        if params.kind == WindowKind::Floating || params.kind == WindowKind::Dialog {
            toplevel.set_parent(xdg_parent.as_ref());
        }

        let dialog = if params.kind == WindowKind::Dialog {
            let dialog = globals.dialog.as_ref().map(|dialog| {
                let xdg_dialog = dialog.get_xdg_dialog(&toplevel, &globals.qh, ());
                xdg_dialog.set_modal();
                xdg_dialog
            });

            if let Some(parent) = parent.as_ref() {
                parent.add_child(surface.id());
            }

            dialog
        } else {
            None
        };

        if let Some(size) = params.window_min_size {
            toplevel.set_min_size(f32::from(size.width) as i32, f32::from(size.height) as i32);
        }

        // Attempt to set up window decorations based on the requested configuration
        let decoration = globals
            .decoration_manager
            .as_ref()
            .map(|decoration_manager| {
                decoration_manager.get_toplevel_decoration(&toplevel, &globals.qh, surface.id())
            });

        Ok(WaylandSurfaceState::Xdg(WaylandXdgSurfaceState {
            xdg_surface,
            toplevel,
            decoration,
            dialog,
        }))
    }
}

pub struct WaylandXdgSurfaceState {
    pub(super) xdg_surface: xdg_surface::XdgSurface,
    pub(super) toplevel: xdg_toplevel::XdgToplevel,
    pub(super) decoration: Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>,
    pub(super) dialog: Option<XdgDialogV1>,
}

pub struct WaylandLayerSurfaceState {
    pub(super) layer_surface: zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
}

impl WaylandSurfaceState {
    fn ack_configure(&self, serial: u32) {
        match self {
            WaylandSurfaceState::Xdg(WaylandXdgSurfaceState { xdg_surface, .. }) => {
                xdg_surface.ack_configure(serial);
            }
            WaylandSurfaceState::LayerShell(WaylandLayerSurfaceState { layer_surface, .. }) => {
                layer_surface.ack_configure(serial);
            }
        }
    }

    fn decoration(&self) -> Option<&zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1> {
        if let WaylandSurfaceState::Xdg(WaylandXdgSurfaceState { decoration, .. }) = self {
            decoration.as_ref()
        } else {
            None
        }
    }

    fn toplevel(&self) -> Option<&xdg_toplevel::XdgToplevel> {
        if let WaylandSurfaceState::Xdg(WaylandXdgSurfaceState { toplevel, .. }) = self {
            Some(toplevel)
        } else {
            None
        }
    }

    fn set_geometry(&self, x: i32, y: i32, width: i32, height: i32) {
        match self {
            WaylandSurfaceState::Xdg(WaylandXdgSurfaceState { xdg_surface, .. }) => {
                xdg_surface.set_window_geometry(x, y, width, height);
            }
            WaylandSurfaceState::LayerShell(WaylandLayerSurfaceState { layer_surface, .. }) => {
                // cannot set window position of a layer surface
                layer_surface.set_size(width as u32, height as u32);
            }
        }
    }

    fn destroy(&mut self) {
        match self {
            WaylandSurfaceState::Xdg(WaylandXdgSurfaceState {
                xdg_surface,
                toplevel,
                decoration: _decoration,
                dialog,
            }) => {
                // drop the dialog before toplevel so compositor can explicitly unapply it's effects
                if let Some(dialog) = dialog {
                    dialog.destroy();
                }

                // The role object (toplevel) must always be destroyed before the xdg_surface.
                // See https://wayland.app/protocols/xdg-shell#xdg_surface:request:destroy
                toplevel.destroy();
                xdg_surface.destroy();
            }
            WaylandSurfaceState::LayerShell(WaylandLayerSurfaceState { layer_surface }) => {
                layer_surface.destroy();
            }
        }
    }
}

#[derive(Clone)]
pub struct WaylandWindowStatePtr {
    pub(super) state: Rc<RefCell<WaylandWindowState>>,
    pub(super) callbacks: Rc<RefCell<Callbacks>>,
}

impl WaylandWindowState {
    pub(crate) fn new(
        handle: AnyWindowHandle,
        surface: wl_surface::WlSurface,
        surface_state: WaylandSurfaceState,
        appearance: WindowAppearance,
        viewport: Option<wp_viewport::WpViewport>,
        client: WaylandClientStatePtr,
        globals: Globals,
        gpu_context: crate::platform::wgpu::GpuContext,
        compositor_gpu: Option<CompositorGpuHint>,
        options: WindowParams,
        parent: Option<WaylandWindowStatePtr>,
    ) -> anyhow::Result<Self> {
        let renderer = {
            let raw_window = RawWindow {
                window: surface.id().as_ptr().cast::<c_void>(),
                display: surface
                    .backend()
                    .upgrade()
                    .unwrap()
                    .display_ptr()
                    .cast::<c_void>(),
            };
            let config = WgpuSurfaceConfig {
                size: Size {
                    width: DevicePixels(f32::from(options.bounds.size.width) as i32),
                    height: DevicePixels(f32::from(options.bounds.size.height) as i32),
                },
                transparent: true,
            };
            WgpuRenderer::new(gpu_context, &raw_window, config, compositor_gpu)?
        };

        if let WaylandSurfaceState::Xdg(ref xdg_state) = surface_state {
            if let Some(title) = options.titlebar.and_then(|titlebar| titlebar.title) {
                xdg_state.toplevel.set_title(title.to_string());
            }
            // Set max window size based on the GPU's maximum texture dimension.
            // This prevents the window from being resized larger than what the GPU can render.
            let max_texture_size = renderer.max_texture_size() as i32;
            xdg_state
                .toplevel
                .set_max_size(max_texture_size, max_texture_size);
        }

        Ok(Self {
            surface_state,
            acknowledged_first_configure: false,
            parent,
            children: FxHashSet::default(),
            surface,
            app_id: None,
            blur: None,
            viewport,
            globals,
            outputs: HashMap::default(),
            display: None,
            renderer,
            bounds: options.bounds,
            scale: 1.0,
            input_handler: None,
            decorations: WindowDecorations::Client,
            background_appearance: WindowBackgroundAppearance::Opaque,
            fullscreen: false,
            maximized: false,
            tiling: Tiling::default(),
            window_bounds: options.bounds,
            in_progress_configure: None,
            resize_throttle: false,
            client,
            appearance,
            handle,
            active: false,
            hovered: false,
            in_progress_window_controls: None,
            window_controls: WindowControls::default(),
            client_inset: None,
        })
    }

    pub fn is_transparent(&self) -> bool {
        self.decorations == WindowDecorations::Client
            || self.background_appearance != WindowBackgroundAppearance::Opaque
    }

    pub fn primary_output_scale(&mut self) -> i32 {
        let mut scale = 1;
        let mut current_output = self.display.take();
        for (id, output) in self.outputs.iter() {
            if let Some((_, output_data)) = &current_output {
                if output.scale > output_data.scale {
                    current_output = Some((id.clone(), output.clone()));
                }
            } else {
                current_output = Some((id.clone(), output.clone()));
            }
            scale = scale.max(output.scale);
        }
        self.display = current_output;
        scale
    }

    pub fn inset(&self) -> Pixels {
        match self.decorations {
            WindowDecorations::Server => px(0.0),
            WindowDecorations::Client => self.client_inset.unwrap_or(px(0.0)),
        }
    }
}

pub(crate) struct WaylandWindow(pub WaylandWindowStatePtr);
pub enum ImeInput {
    InsertText(String),
    SetMarkedText(String),
    UnmarkText,
    DeleteText,
}

impl Drop for WaylandWindow {
    fn drop(&mut self) {
        let mut state = self.0.state.borrow_mut();
        let surface_id = state.surface.id();
        if let Some(parent) = state.parent.as_ref() {
            parent.state.borrow_mut().children.remove(&surface_id);
        }

        let client = state.client.clone();

        state.renderer.destroy();

        // Destroy blur first, this has no dependencies.
        if let Some(blur) = &state.blur {
            blur.release();
        }

        // Decorations must be destroyed before the xdg state.
        // See https://wayland.app/protocols/xdg-decoration-unstable-v1#zxdg_toplevel_decoration_v1
        if let Some(decoration) = &state.surface_state.decoration() {
            decoration.destroy();
        }

        // Surface state might contain xdg_toplevel/xdg_surface which can be destroyed now that
        // decorations are gone. layer_surface has no dependencies.
        state.surface_state.destroy();

        // Viewport must be destroyed before the wl_surface.
        // See https://wayland.app/protocols/viewporter#wp_viewport
        if let Some(viewport) = &state.viewport {
            viewport.destroy();
        }

        // The wl_surface itself should always be destroyed last.
        state.surface.destroy();

        let state_ptr = self.0.clone();
        state
            .globals
            .executor
            .spawn(async move {
                state_ptr.close();
                client.drop_window(&surface_id)
            })
            .detach();
        drop(state);
    }
}

impl WaylandWindow {
    fn borrow(&self) -> Ref<'_, WaylandWindowState> {
        self.0.state.borrow()
    }

    fn borrow_mut(&self) -> RefMut<'_, WaylandWindowState> {
        self.0.state.borrow_mut()
    }

    pub fn new(
        handle: AnyWindowHandle,
        globals: Globals,
        gpu_context: crate::platform::wgpu::GpuContext,
        compositor_gpu: Option<CompositorGpuHint>,
        client: WaylandClientStatePtr,
        params: WindowParams,
        appearance: WindowAppearance,
        parent: Option<WaylandWindowStatePtr>,
        target_output: Option<wl_output::WlOutput>,
    ) -> anyhow::Result<(Self, ObjectId)> {
        let surface = globals.compositor.create_surface(&globals.qh, ());
        let surface_state =
            WaylandSurfaceState::new(&surface, &globals, &params, parent.clone(), target_output)?;

        if let Some(fractional_scale_manager) = globals.fractional_scale_manager.as_ref() {
            fractional_scale_manager.get_fractional_scale(&surface, &globals.qh, surface.id());
        }

        let viewport = globals
            .viewporter
            .as_ref()
            .map(|viewporter| viewporter.get_viewport(&surface, &globals.qh, ()));

        let this = Self(WaylandWindowStatePtr {
            state: Rc::new(RefCell::new(WaylandWindowState::new(
                handle,
                surface.clone(),
                surface_state,
                appearance,
                viewport,
                client,
                globals,
                gpu_context,
                compositor_gpu,
                params,
                parent,
            )?)),
            callbacks: Rc::new(RefCell::new(Callbacks::default())),
        });

        // Kick things off
        surface.commit();

        Ok((this, surface.id()))
    }
}

