mod event_handling;
mod helpers;
mod input_handling;
mod linux_client_impl;
mod state;
mod types;

use anyhow::{Context as _, anyhow};
use ashpd::WindowIdentifier;
use calloop::{
    EventLoop, LoopHandle, RegistrationToken,
    generic::{FdWrapper, Generic},
};
use collections::HashMap;
use core::str;
use inazuma::{Capslock, TaskTiming, profiler};
use http_client::Url;
use log::Level;
use smallvec::SmallVec;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashSet},
    ops::Deref,
    path::PathBuf,
    rc::{Rc, Weak},
    time::{Duration, Instant},
};
use util::ResultExt as _;

use x11rb::{
    connection::{Connection, RequestConnection},
    cursor,
    errors::ConnectionError,
    protocol::randr::ConnectionExt as _,
    protocol::xinput::ConnectionExt,
    protocol::xkb::ConnectionExt as _,
    protocol::xproto::{
        AtomEnum, ChangeWindowAttributesAux, ClientMessageData, ClientMessageEvent,
        ConnectionExt as _, EventMask, ModMask, Visibility,
    },
    protocol::{Event, dri3, randr, render, xinput, xkb, xproto},
    resource_manager::Database,
    wrapper::ConnectionExt as _,
    xcb_ffi::XCBConnection,
};
use xim::{AttributeName, Client, InputStyle, x11rb::X11rbClient};
use xkbc::x11::ffi::{XKB_X11_MIN_MAJOR_XKB_VERSION, XKB_X11_MIN_MINOR_XKB_VERSION};
use xkbcommon::xkb::{self as xkbc, STATE_LAYOUT_EFFECTIVE};

use super::{
    ButtonOrScroll, ScrollDirection, X11Display, X11WindowStatePtr, XcbAtoms, XimCallbackEvent,
    XimHandler, button_or_scroll_from_event_detail, check_reply,
    clipboard::{self, Clipboard},
    get_reply, get_valuator_axis_index, handle_connection_error, modifiers_from_state,
    pressed_button_from_mask, xcb_flush,
};

use crate::platform::linux::{
    DEFAULT_CURSOR_ICON_NAME, LinuxClient, capslock_from_xkb, cursor_style_to_icon_names,
    get_xkb_compose_state, is_within_click_distance, keystroke_from_xkb,
    keystroke_underlying_dead_key, log_cursor_icon_warning, modifiers_from_xkb, open_uri_internal,
    platform::{DOUBLE_CLICK_INTERVAL, SCROLL_LINES},
    reveal_path_internal,
    xdg_desktop_portal::{Event as XDPEvent, XDPEventSource},
};
use crate::platform::linux::{
    LinuxCommon, LinuxKeyboardLayout, X11Window, modifiers_from_xinput_info,
};

use crate::platform::wgpu::{CompositorGpuHint, GpuContext};
use inazuma::{
    AnyWindowHandle, Bounds, ClipboardItem, CursorStyle, DisplayId, FileDropEvent, Keystroke,
    Modifiers, ModifiersChangedEvent, MouseButton, Pixels, PlatformDisplay, PlatformInput,
    PlatformKeyboardLayout, PlatformWindow, Point, RequestFrameOptions, ScrollDelta, Size,
    TouchPhase, WindowParams, point, px,
};

pub use helpers::mode_refresh_rate;
pub use types::*;

pub(crate) use helpers::*;

#[derive(Clone)]
pub(crate) struct X11Client(pub(crate) Rc<RefCell<X11ClientState>>);
