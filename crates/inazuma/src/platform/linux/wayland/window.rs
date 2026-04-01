mod platform_window;
mod state_ptr;
mod types;

use std::{
    cell::{Ref, RefCell, RefMut},
    ffi::c_void,
    ptr::NonNull,
    rc::Rc,
    sync::Arc,
};

use collections::{FxHashSet, HashMap};
use futures::channel::oneshot::Receiver;

use raw_window_handle as rwh;
use wayland_backend::client::ObjectId;
use wayland_client::WEnum;
use wayland_client::{
    Proxy,
    protocol::{wl_output, wl_surface},
};
use wayland_protocols::wp::viewporter::client::wp_viewport;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1;
use wayland_protocols::xdg::shell::client::xdg_surface;
use wayland_protocols::xdg::shell::client::xdg_toplevel::{self};
use wayland_protocols::{
    wp::fractional_scale::v1::client::wp_fractional_scale_v1,
    xdg::dialog::v1::client::xdg_dialog_v1::XdgDialogV1,
};
use wayland_protocols_plasma::blur::client::org_kde_kwin_blur;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1;

use crate::platform::linux::wayland::{display::WaylandDisplay, serial::SerialKind};
use crate::platform::linux::{Globals, Output, WaylandClientStatePtr, get_window};
use crate::platform::wgpu::{CompositorGpuHint, WgpuRenderer, WgpuSurfaceConfig};
use inazuma::{
    AnyWindowHandle, Bounds, Capslock, Decorations, DevicePixels, GpuSpecs, Modifiers, Pixels,
    PlatformAtlas, PlatformDisplay, PlatformInput, PlatformInputHandler, PlatformWindow, Point,
    PromptButton, PromptLevel, RequestFrameOptions, ResizeEdge, Scene, Size, Tiling,
    WindowAppearance, WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowControls,
    WindowDecorations, WindowKind, WindowParams, layer_shell::LayerShellNotSupportedError, px,
    size,
};

pub use types::*;
pub(crate) use platform_window::{ResizeEdgeWaylandExt, WindowDecorationsExt};
