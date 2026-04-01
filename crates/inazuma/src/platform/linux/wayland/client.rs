mod client_impl;
mod dispatch_core;
mod dispatch_input;
mod dispatch_misc;
mod types;

use std::{
    cell::{RefCell, RefMut},
    hash::Hash,
    os::fd::{AsRawFd, BorrowedFd},
    path::PathBuf,
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

use ashpd::WindowIdentifier;
use calloop::{
    EventLoop, LoopHandle,
    timer::{TimeoutAction, Timer},
};
use calloop_wayland_source::WaylandSource;
use collections::HashMap;
use filedescriptor::Pipe;
use http_client::Url;
use smallvec::SmallVec;
use util::ResultExt as _;
use wayland_backend::client::ObjectId;
use wayland_backend::protocol::WEnum;
use wayland_client::event_created_child;
use wayland_client::globals::{GlobalList, GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_callback::{self, WlCallback};
use wayland_client::protocol::wl_data_device_manager::DndAction;
use wayland_client::protocol::wl_data_offer::WlDataOffer;
use wayland_client::protocol::wl_pointer::AxisSource;
use wayland_client::protocol::{
    wl_data_device, wl_data_device_manager, wl_data_offer, wl_data_source, wl_output, wl_region,
};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, delegate_noop,
    protocol::{
        wl_buffer, wl_compositor, wl_keyboard, wl_pointer, wl_registry, wl_seat, wl_shm,
        wl_shm_pool, wl_surface,
    },
};
use wayland_protocols::wp::pointer_gestures::zv1::client::{
    zwp_pointer_gesture_pinch_v1, zwp_pointer_gestures_v1,
};
use wayland_protocols::wp::primary_selection::zv1::client::zwp_primary_selection_offer_v1::{
    self, ZwpPrimarySelectionOfferV1,
};
use wayland_protocols::wp::primary_selection::zv1::client::{
    zwp_primary_selection_device_manager_v1, zwp_primary_selection_device_v1,
    zwp_primary_selection_source_v1,
};
use wayland_protocols::wp::text_input::zv3::client::zwp_text_input_v3::{
    ContentHint, ContentPurpose,
};
use wayland_protocols::wp::text_input::zv3::client::{
    zwp_text_input_manager_v3, zwp_text_input_v3,
};
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};
use wayland_protocols::xdg::activation::v1::client::{xdg_activation_token_v1, xdg_activation_v1};
use wayland_protocols::xdg::decoration::zv1::client::{
    zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};
use wayland_protocols::{
    wp::cursor_shape::v1::client::{wp_cursor_shape_device_v1, wp_cursor_shape_manager_v1},
    xdg::dialog::v1::client::xdg_wm_dialog_v1::{self, XdgWmDialogV1},
};
use wayland_protocols::{
    wp::fractional_scale::v1::client::{wp_fractional_scale_manager_v1, wp_fractional_scale_v1},
    xdg::dialog::v1::client::xdg_dialog_v1::XdgDialogV1,
};
use wayland_protocols_plasma::blur::client::{org_kde_kwin_blur, org_kde_kwin_blur_manager};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};
use xkbcommon::xkb::ffi::XKB_KEYMAP_FORMAT_TEXT_V1;
use xkbcommon::xkb::{self, KEYMAP_COMPILE_NO_FLAGS, Keycode};

use super::{
    display::WaylandDisplay,
    window::{ImeInput, WaylandWindowStatePtr},
};

use crate::platform::linux::{
    DOUBLE_CLICK_INTERVAL, LinuxClient, LinuxCommon, LinuxKeyboardLayout, SCROLL_LINES,
    capslock_from_xkb, cursor_style_to_icon_names, get_xkb_compose_state, is_within_click_distance,
    keystroke_from_xkb, keystroke_underlying_dead_key, modifiers_from_xkb, open_uri_internal,
    read_fd, reveal_path_internal,
    wayland::{
        clipboard::{Clipboard, DataOffer, FILE_LIST_MIME_TYPE, TEXT_MIME_TYPES},
        cursor::Cursor,
        serial::{SerialKind, SerialTracker},
        to_shape,
        window::WaylandWindow,
    },
    xdg_desktop_portal::{Event as XDPEvent, XDPEventSource},
};
use crate::platform::wgpu::{CompositorGpuHint, GpuContext};
use inazuma::{
    AnyWindowHandle, Bounds, Capslock, CursorStyle, DevicePixels, DisplayId, FileDropEvent,
    ForegroundExecutor, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, ModifiersChangedEvent,
    MouseButton, MouseDownEvent, MouseExitEvent, MouseMoveEvent, MouseUpEvent, NavigationDirection,
    Pixels, PlatformDisplay, PlatformInput, PlatformKeyboardLayout, PlatformWindow, Point,
    ScrollDelta, ScrollWheelEvent, SharedString, Size, TaskTiming, TouchPhase, WindowParams, point,
    profiler, px, size,
};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_dmabuf_feedback_v1, zwp_linux_dmabuf_v1,
};

pub use types::*;
pub(crate) use dispatch_core::get_window;
