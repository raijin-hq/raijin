use super::*;

pub(crate) const XINPUT_ALL_DEVICES: xinput::DeviceId = 0;

/// Value for DeviceId parameters which selects all device groups. Events that
/// occur within the group are emitted by the group itself.
///
/// In XInput 2's interface, these are referred to as "master devices", but that
/// terminology is both archaic and unclear.
pub(crate) const XINPUT_ALL_DEVICE_GROUPS: xinput::DeviceId = 1;

pub(super) const GPUI_X11_SCALE_FACTOR_ENV: &str = "GPUI_X11_SCALE_FACTOR";

pub(crate) struct WindowRef {
    pub(super) window: X11WindowStatePtr,
    pub(super) refresh_state: Option<RefreshState>,
    pub(super) expose_event_received: bool,
    pub(super) last_visibility: Visibility,
    pub(super) is_mapped: bool,
}

impl WindowRef {
    pub fn handle(&self) -> AnyWindowHandle {
        self.window.state.borrow().handle
    }
}

impl Deref for WindowRef {
    type Target = X11WindowStatePtr;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

pub(super) enum RefreshState {
    Hidden {
        refresh_rate: Duration,
    },
    PeriodicRefresh {
        refresh_rate: Duration,
        event_loop_token: RegistrationToken,
    },
}

#[derive(Debug)]
#[non_exhaustive]
pub enum EventHandlerError {
    XCBConnectionError(ConnectionError),
    XIMClientError(xim::ClientError),
}

impl std::error::Error for EventHandlerError {}

impl std::fmt::Display for EventHandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventHandlerError::XCBConnectionError(err) => err.fmt(f),
            EventHandlerError::XIMClientError(err) => err.fmt(f),
        }
    }
}

impl From<ConnectionError> for EventHandlerError {
    fn from(err: ConnectionError) -> Self {
        EventHandlerError::XCBConnectionError(err)
    }
}

impl From<xim::ClientError> for EventHandlerError {
    fn from(err: xim::ClientError) -> Self {
        EventHandlerError::XIMClientError(err)
    }
}

#[derive(Debug, Default)]
pub struct Xdnd {
    pub(super) other_window: xproto::Window,
    pub(super) drag_type: u32,
    pub(super) retrieved: bool,
    pub(super) position: Point<Pixels>,
}

#[derive(Debug)]
pub(super) struct PointerDeviceState {
    pub(super) horizontal: ScrollAxisState,
    pub(super) vertical: ScrollAxisState,
}

#[derive(Debug, Default)]
pub(super) struct ScrollAxisState {
    /// Valuator number for looking up this axis's scroll value.
    pub(super) valuator_number: Option<u16>,
    /// Conversion factor from scroll units to lines.
    pub(super) multiplier: f32,
    /// Last scroll value for calculating scroll delta.
    ///
    /// This gets set to `None` whenever it might be invalid - when devices change or when window focus changes.
    /// The logic errs on the side of invalidating this, since the consequence is just skipping the delta of one scroll event.
    /// The consequence of not invalidating it can be large invalid deltas, which are much more user visible.
    pub(super) scroll_value: Option<f32>,
}

pub struct X11ClientState {
    pub(crate) loop_handle: LoopHandle<'static, X11Client>,
    pub(crate) event_loop: Option<calloop::EventLoop<'static, X11Client>>,

    pub(crate) last_click: Instant,
    pub(crate) last_mouse_button: Option<MouseButton>,
    pub(crate) last_location: Point<Pixels>,
    pub(crate) current_count: usize,

    pub(crate) gpu_context: GpuContext,
    pub(crate) compositor_gpu: Option<CompositorGpuHint>,

    pub(crate) scale_factor: f32,

    pub(super) xkb_context: xkbc::Context,
    pub(crate) xcb_connection: Rc<XCBConnection>,
    pub(super) xkb_device_id: i32,
    pub(super) client_side_decorations_supported: bool,
    pub(crate) x_root_index: usize,
    pub(crate) _resource_database: Database,
    pub(crate) atoms: XcbAtoms,
    pub(crate) windows: HashMap<xproto::Window, WindowRef>,
    pub(crate) mouse_focused_window: Option<xproto::Window>,
    pub(crate) keyboard_focused_window: Option<xproto::Window>,
    pub(crate) xkb: xkbc::State,
    pub(super) keyboard_layout: LinuxKeyboardLayout,
    pub(crate) ximc: Option<X11rbClient<Rc<XCBConnection>>>,
    pub(crate) xim_handler: Option<XimHandler>,
    pub modifiers: Modifiers,
    pub capslock: Capslock,
    // TODO: Can the other updates to `modifiers` be removed so that this is unnecessary?
    // capslock logic was done analog to modifiers
    pub last_modifiers_changed_event: Modifiers,
    pub last_capslock_changed_event: Capslock,

    pub(crate) compose_state: Option<xkbc::compose::State>,
    pub(crate) pre_edit_text: Option<String>,
    pub(crate) composing: bool,
    pub(crate) pre_key_char_down: Option<Keystroke>,
    pub(crate) cursor_handle: cursor::Handle,
    pub(crate) cursor_styles: HashMap<xproto::Window, CursorStyle>,
    pub(crate) cursor_cache: HashMap<CursorStyle, Option<xproto::Cursor>>,

    pub(super) pointer_device_states: BTreeMap<xinput::DeviceId, PointerDeviceState>,

    pub(crate) common: LinuxCommon,
    pub(crate) clipboard: Clipboard,
    pub(crate) clipboard_item: Option<ClipboardItem>,
    pub(crate) xdnd_state: Xdnd,
}

#[derive(Clone)]
pub struct X11ClientStatePtr(pub Weak<RefCell<X11ClientState>>);

impl X11ClientStatePtr {
    pub fn get_client(&self) -> Option<X11Client> {
        self.0.upgrade().map(X11Client)
    }

    pub fn drop_window(&self, x_window: u32) {
        let Some(client) = self.get_client() else {
            return;
        };
        let mut state = client.0.borrow_mut();

        if let Some(window_ref) = state.windows.remove(&x_window)
            && let Some(RefreshState::PeriodicRefresh {
                event_loop_token, ..
            }) = window_ref.refresh_state
        {
            state.loop_handle.remove(event_loop_token);
        }
        if state.mouse_focused_window == Some(x_window) {
            state.mouse_focused_window = None;
        }
        if state.keyboard_focused_window == Some(x_window) {
            state.keyboard_focused_window = None;
        }
        state.cursor_styles.remove(&x_window);
    }

    pub fn update_ime_position(&self, bounds: Bounds<Pixels>) {
        let Some(client) = self.get_client() else {
            return;
        };
        let mut state = client.0.borrow_mut();
        if state.composing || state.ximc.is_none() {
            return;
        }

        let Some(mut ximc) = state.ximc.take() else {
            log::error!("bug: xim connection not set");
            return;
        };
        let Some(xim_handler) = state.xim_handler.take() else {
            log::error!("bug: xim handler not set");
            state.ximc = Some(ximc);
            return;
        };
        let scaled_bounds = bounds.scale(state.scale_factor);
        let ic_attributes = ximc
            .build_ic_attributes()
            .push(
                xim::AttributeName::InputStyle,
                xim::InputStyle::PREEDIT_CALLBACKS,
            )
            .push(xim::AttributeName::ClientWindow, xim_handler.window)
            .push(xim::AttributeName::FocusWindow, xim_handler.window)
            .nested_list(xim::AttributeName::PreeditAttributes, |b| {
                b.push(
                    xim::AttributeName::SpotLocation,
                    xim::Point {
                        x: u32::from(scaled_bounds.origin.x + scaled_bounds.size.width) as i16,
                        y: u32::from(scaled_bounds.origin.y + scaled_bounds.size.height) as i16,
                    },
                );
            })
            .build();
        let _ = ximc
            .set_ic_values(xim_handler.im_id, xim_handler.ic_id, ic_attributes)
            .log_err();
        state.ximc = Some(ximc);
        state.xim_handler = Some(xim_handler);
    }
}
