use super::*;

pub(super) fn query_render_extent(
    xcb: &Rc<XCBConnection>,
    x_window: xproto::Window,
) -> anyhow::Result<Size<DevicePixels>> {
    let reply = get_reply(|| "X11 GetGeometry failed.", xcb.get_geometry(x_window))?;
    Ok(Size {
        width: DevicePixels(reply.width as i32),
        height: DevicePixels(reply.height as i32),
    })
}

pub(super) fn resize_edge_to_moveresize(edge: ResizeEdge) -> u32 {
    match edge {
        ResizeEdge::TopLeft => 0,
        ResizeEdge::Top => 1,
        ResizeEdge::TopRight => 2,
        ResizeEdge::Right => 3,
        ResizeEdge::BottomRight => 4,
        ResizeEdge::Bottom => 5,
        ResizeEdge::BottomLeft => 6,
        ResizeEdge::Left => 7,
    }
}

#[derive(Debug)]
pub(super) struct EdgeConstraints {
    pub(super) top_tiled: bool,
    #[allow(dead_code)]
    pub(super) top_resizable: bool,

    pub(super) right_tiled: bool,
    #[allow(dead_code)]
    pub(super) right_resizable: bool,

    pub(super) bottom_tiled: bool,
    #[allow(dead_code)]
    pub(super) bottom_resizable: bool,

    pub(super) left_tiled: bool,
    #[allow(dead_code)]
    pub(super) left_resizable: bool,
}

impl EdgeConstraints {
    fn from_atom(atom: u32) -> Self {
        EdgeConstraints {
            top_tiled: (atom & (1 << 0)) != 0,
            top_resizable: (atom & (1 << 1)) != 0,
            right_tiled: (atom & (1 << 2)) != 0,
            right_resizable: (atom & (1 << 3)) != 0,
            bottom_tiled: (atom & (1 << 4)) != 0,
            bottom_resizable: (atom & (1 << 5)) != 0,
            left_tiled: (atom & (1 << 6)) != 0,
            left_resizable: (atom & (1 << 7)) != 0,
        }
    }

    fn to_tiling(&self) -> Tiling {
        Tiling {
            top: self.top_tiled,
            right: self.right_tiled,
            bottom: self.bottom_tiled,
            left: self.left_tiled,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub(super) struct Visual {
    pub(super) id: xproto::Visualid,
    pub(super) colormap: u32,
    pub(super) depth: u8,
}

pub(super) struct VisualSet {
    pub(super) inherit: Visual,
    pub(super) opaque: Option<Visual>,
    pub(super) transparent: Option<Visual>,
    pub(super) root: u32,
    pub(super) black_pixel: u32,
}

pub(super) fn find_visuals(xcb: &XCBConnection, screen_index: usize) -> VisualSet {
    let screen = &xcb.setup().roots[screen_index];
    let mut set = VisualSet {
        inherit: Visual {
            id: screen.root_visual,
            colormap: screen.default_colormap,
            depth: screen.root_depth,
        },
        opaque: None,
        transparent: None,
        root: screen.root,
        black_pixel: screen.black_pixel,
    };

    for depth_info in screen.allowed_depths.iter() {
        for visual_type in depth_info.visuals.iter() {
            let visual = Visual {
                id: visual_type.visual_id,
                colormap: 0,
                depth: depth_info.depth,
            };
            log::debug!(
                "Visual id: {}, class: {:?}, depth: {}, bits_per_value: {}, masks: 0x{:x} 0x{:x} 0x{:x}",
                visual_type.visual_id,
                visual_type.class,
                depth_info.depth,
                visual_type.bits_per_rgb_value,
                visual_type.red_mask,
                visual_type.green_mask,
                visual_type.blue_mask,
            );

            if (
                visual_type.red_mask,
                visual_type.green_mask,
                visual_type.blue_mask,
            ) != (0xFF0000, 0xFF00, 0xFF)
            {
                continue;
            }
            let color_mask = visual_type.red_mask | visual_type.green_mask | visual_type.blue_mask;
            let alpha_mask = color_mask as usize ^ ((1usize << depth_info.depth) - 1);

            if alpha_mask == 0 {
                if set.opaque.is_none() {
                    set.opaque = Some(visual);
                }
            } else {
                if set.transparent.is_none() {
                    set.transparent = Some(visual);
                }
            }
        }
    }

    set
}

#[derive(Debug, Clone, Copy)]
pub(super) struct RawWindow {
    pub(super) connection: *mut c_void,
    pub(super) screen_id: usize,
    pub(super) window_id: u32,
    pub(super) visual_id: u32,
}

// Safety: The raw pointers in RawWindow point to X11 connection
// which is valid for the window's lifetime. These are used only for
// passing to wgpu which needs Send+Sync for surface creation.
unsafe impl Send for RawWindow {}
unsafe impl Sync for RawWindow {}

#[derive(Default)]
pub struct Callbacks {
    pub(super) request_frame: Option<Box<dyn FnMut(RequestFrameOptions)>>,
    pub(super) input: Option<Box<dyn FnMut(PlatformInput) -> inazuma::DispatchEventResult>>,
    pub(super) active_status_change: Option<Box<dyn FnMut(bool)>>,
    pub(super) hovered_status_change: Option<Box<dyn FnMut(bool)>>,
    pub(super) resize: Option<Box<dyn FnMut(Size<Pixels>, f32)>>,
    pub(super) moved: Option<Box<dyn FnMut()>>,
    pub(super) should_close: Option<Box<dyn FnMut() -> bool>>,
    pub(super) close: Option<Box<dyn FnOnce()>>,
    pub(super) appearance_changed: Option<Box<dyn FnMut()>>,
}

pub struct X11WindowState {
    pub destroyed: bool,
    pub(super) parent: Option<X11WindowStatePtr>,
    pub(super) children: FxHashSet<xproto::Window>,
    pub(super) client: X11ClientStatePtr,
    pub(super) executor: ForegroundExecutor,
    pub(super) atoms: XcbAtoms,
    pub(super) x_root_window: xproto::Window,
    pub(super) x_screen_index: usize,
    pub(super) visual_id: u32,
    pub(crate) counter_id: sync::Counter,
    pub(crate) last_sync_counter: Option<sync::Int64>,
    pub(super) bounds: Bounds<Pixels>,
    pub(super) scale_factor: f32,
    pub(super) renderer: WgpuRenderer,
    pub(super) display: Rc<dyn PlatformDisplay>,
    pub(super) input_handler: Option<PlatformInputHandler>,
    pub(super) appearance: WindowAppearance,
    pub(super) background_appearance: WindowBackgroundAppearance,
    pub(super) maximized_vertical: bool,
    pub(super) maximized_horizontal: bool,
    pub(super) hidden: bool,
    pub(super) active: bool,
    pub(super) hovered: bool,
    pub(super) fullscreen: bool,
    pub(super) client_side_decorations_supported: bool,
    pub(super) decorations: WindowDecorations,
    pub(super) edge_constraints: Option<EdgeConstraints>,
    pub handle: AnyWindowHandle,
    pub(super) last_insets: [u32; 4],
}

impl X11WindowState {
    fn is_transparent(&self) -> bool {
        self.background_appearance != WindowBackgroundAppearance::Opaque
    }
}

#[derive(Clone)]
pub(crate) struct X11WindowStatePtr {
    pub state: Rc<RefCell<X11WindowState>>,
    pub(crate) callbacks: Rc<RefCell<Callbacks>>,
    pub(super) xcb: Rc<XCBConnection>,
    pub(crate) x_window: xproto::Window,
}

impl rwh::HasWindowHandle for RawWindow {
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        let Some(non_zero) = NonZeroU32::new(self.window_id) else {
            log::error!("RawWindow.window_id zero when getting window handle.");
            return Err(rwh::HandleError::Unavailable);
        };
        let mut handle = rwh::XcbWindowHandle::new(non_zero);
        handle.visual_id = NonZeroU32::new(self.visual_id);
        Ok(unsafe { rwh::WindowHandle::borrow_raw(handle.into()) })
    }
}
impl rwh::HasDisplayHandle for RawWindow {
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, rwh::HandleError> {
        let Some(non_zero) = NonNull::new(self.connection) else {
            log::error!("Null RawWindow.connection when getting display handle.");
            return Err(rwh::HandleError::Unavailable);
        };
        let handle = rwh::XcbDisplayHandle::new(Some(non_zero), self.screen_id as i32);
        Ok(unsafe { rwh::DisplayHandle::borrow_raw(handle.into()) })
    }
}

impl rwh::HasWindowHandle for X11Window {
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        let Some(non_zero) = NonZeroU32::new(self.0.x_window) else {
            return Err(rwh::HandleError::Unavailable);
        };
        let handle = rwh::XcbWindowHandle::new(non_zero);
        Ok(unsafe { rwh::WindowHandle::borrow_raw(handle.into()) })
    }
}

impl rwh::HasDisplayHandle for X11Window {
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, rwh::HandleError> {
        let connection =
            as_raw_xcb_connection::AsRawXcbConnection::as_raw_xcb_connection(&*self.0.xcb)
                as *mut _;
        let Some(non_zero) = NonNull::new(connection) else {
            return Err(rwh::HandleError::Unavailable);
        };
        let screen_id = {
            let state = self.0.state.borrow();
            u32::from(state.display.id()) as i32
        };
        let handle = rwh::XcbDisplayHandle::new(Some(non_zero), screen_id);
        Ok(unsafe { rwh::DisplayHandle::borrow_raw(handle.into()) })
    }
}

