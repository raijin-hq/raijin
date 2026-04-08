mod callbacks;
mod callbacks_mouse;
mod class_registration;
mod constants;
mod open;
mod platform_impl;
mod state;

use super::{
    DisplayLink, MacDisplay, events::platform_input_from_native,
    renderer,
};
#[cfg(any(test, feature = "test-support"))]
use anyhow::Result;
use block2::RcBlock;
use objc2_core_graphics::CGDirectDisplayID;
use dispatch2::DispatchQueue;
use futures::channel::oneshot;
use inazuma::{
    AnyWindowHandle, BackgroundExecutor, Bounds, Capslock, ExternalPaths, FileDropEvent,
    ForegroundExecutor, KeyDownEvent, Keystroke, Modifiers, ModifiersChangedEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, PlatformAtlas, PlatformDisplay,
    PlatformInput, PlatformInputHandler, PlatformWindow, Point, PromptButton, PromptLevel,
    RequestFrameOptions, SharedString, Size, SystemWindowTab, WindowAppearance,
    WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowKind, WindowParams, point,
    px, size,
};
#[cfg(any(test, feature = "test-support"))]
use image::RgbaImage;
use objc2::msg_send;
use objc2::runtime::AnyObject;
use objc2::sel;
use objc2::ClassType;
use parking_lot::Mutex;
use raw_window_handle as rwh;
use smallvec::SmallVec;
use std::{
    cell::Cell,
    ffi::c_void,
    mem,
    ops::Range,
    path::PathBuf,
    ptr::{self, NonNull},
    rc::Rc,
    sync::{Arc, Weak},
    time::Duration,
};
use inazuma_util::ResultExt;

use objc2_foundation::{NSNotFound, NSPoint, NSRange, NSRect, NSSize};

/// Create an NSRange representing "not found" (NSNotFound location, 0 length).
fn ns_range_invalid() -> NSRange {
    NSRange::new(NSNotFound as usize, 0)
}

/// Convert an NSRange to a Rust Range, returning None if location is NSNotFound.
fn ns_range_to_range(range: NSRange) -> Option<Range<usize>> {
    if range.location == NSNotFound as usize {
        None
    } else {
        Some(range.location..(range.location + range.length))
    }
}

use callbacks::*;
use class_registration::WindowIvars;
use constants::*;
use state::*;
pub(crate) use state::{MacWindow, convert_mouse_position};
