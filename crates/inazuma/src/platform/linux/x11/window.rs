mod platform_window;
mod state_impl;
mod state_ptr;
mod types;
mod window_impl;

use anyhow::{Context as _, anyhow};
use x11rb::connection::RequestConnection;

use crate::platform::linux::X11ClientStatePtr;
use crate::platform::wgpu::{CompositorGpuHint, WgpuRenderer, WgpuSurfaceConfig};
use inazuma::{
    AnyWindowHandle, Bounds, Decorations, DevicePixels, ForegroundExecutor, GpuSpecs, Modifiers,
    Pixels, PlatformAtlas, PlatformDisplay, PlatformInput, PlatformInputHandler, PlatformWindow,
    Point, PromptButton, PromptLevel, RequestFrameOptions, ResizeEdge, ScaledPixels, Scene, Size,
    Tiling, WindowAppearance, WindowBackgroundAppearance, WindowBounds, WindowControlArea,
    WindowDecorations, WindowKind, WindowParams, px,
};

use inazuma_collections::FxHashSet;
use raw_window_handle as rwh;
use inazuma_util::{ResultExt, maybe};
use x11rb::{
    connection::Connection,
    cookie::{Cookie, VoidCookie},
    errors::ConnectionError,
    properties::WmSizeHints,
    protocol::{
        sync,
        xinput::{self, ConnectionExt as _},
        xproto::{self, ClientMessageEvent, ConnectionExt, TranslateCoordinatesReply},
    },
    wrapper::ConnectionExt as _,
    xcb_ffi::XCBConnection,
};

use std::{
    cell::RefCell, ffi::c_void, fmt::Display, num::NonZeroU32, ptr::NonNull, rc::Rc, sync::Arc,
};

use super::{X11Display, XINPUT_ALL_DEVICE_GROUPS, XINPUT_ALL_DEVICES};

x11rb::atom_manager! {
    pub XcbAtoms: AtomsCookie {
        XA_ATOM,
        XdndAware,
        XdndStatus,
        XdndEnter,
        XdndLeave,
        XdndPosition,
        XdndSelection,
        XdndDrop,
        XdndFinished,
        XdndTypeList,
        XdndActionCopy,
        TextUriList: b"text/uri-list",
        UTF8_STRING,
        TEXT,
        STRING,
        TEXT_PLAIN_UTF8: b"text/plain;charset=utf-8",
        TEXT_PLAIN: b"text/plain",
        XDND_DATA,
        WM_PROTOCOLS,
        WM_DELETE_WINDOW,
        WM_CHANGE_STATE,
        WM_TRANSIENT_FOR,
        _NET_WM_PID,
        _NET_WM_NAME,
        _NET_WM_STATE,
        _NET_WM_STATE_MAXIMIZED_VERT,
        _NET_WM_STATE_MAXIMIZED_HORZ,
        _NET_WM_STATE_FULLSCREEN,
        _NET_WM_STATE_HIDDEN,
        _NET_WM_STATE_FOCUSED,
        _NET_ACTIVE_WINDOW,
        _NET_WM_SYNC_REQUEST,
        _NET_WM_SYNC_REQUEST_COUNTER,
        _NET_WM_BYPASS_COMPOSITOR,
        _NET_WM_MOVERESIZE,
        _NET_WM_WINDOW_TYPE,
        _NET_WM_WINDOW_TYPE_NOTIFICATION,
        _NET_WM_WINDOW_TYPE_DIALOG,
        _NET_WM_STATE_MODAL,
        _NET_WM_SYNC,
        _NET_SUPPORTED,
        _MOTIF_WM_HINTS,
        _GTK_SHOW_WINDOW_MENU,
        _GTK_FRAME_EXTENTS,
        _GTK_EDGE_CONSTRAINTS,
        _NET_CLIENT_LIST_STACKING,
    }
}

pub use types::*;
pub(crate) use window_impl::*;
