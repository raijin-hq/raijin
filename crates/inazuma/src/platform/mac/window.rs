mod callbacks;
mod callbacks_mouse;
mod class_registration;
mod constants;
mod open;
mod platform_impl;
mod state;

use super::{
    BoolExt, DisplayLink, MacDisplay, NSRange, NSStringExt, events::platform_input_from_native,
    ns_string, renderer,
};
#[cfg(any(test, feature = "test-support"))]
use anyhow::Result;
use block::ConcreteBlock;
use cocoa::{
    appkit::{
        NSAppKitVersionNumber, NSAppKitVersionNumber12_0, NSApplication, NSBackingStoreBuffered,
        NSColor, NSEvent, NSEventModifierFlags, NSFilenamesPboardType, NSPasteboard, NSScreen,
        NSView, NSViewHeightSizable, NSViewWidthSizable, NSVisualEffectMaterial,
        NSVisualEffectState, NSVisualEffectView, NSWindow, NSWindowButton,
        NSWindowCollectionBehavior, NSWindowOcclusionState, NSWindowOrderingMode,
        NSWindowStyleMask, NSWindowTitleVisibility,
    },
    base::{id, nil},
    foundation::{
        NSArray, NSAutoreleasePool, NSDictionary, NSFastEnumeration, NSInteger, NSNotFound,
        NSOperatingSystemVersion, NSPoint, NSProcessInfo, NSRect, NSSize, NSString, NSUInteger,
        NSUserDefaults,
    },
};
use dispatch2::DispatchQueue;
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

use core_graphics::display::{CGDirectDisplayID, CGPoint, CGRect};
use ctor::ctor;
use futures::channel::oneshot;
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{BOOL, Class, NO, Object, Protocol, Sel, YES},
    sel, sel_impl,
};
use parking_lot::Mutex;
use raw_window_handle as rwh;
use smallvec::SmallVec;
use std::{
    cell::Cell,
    ffi::{CStr, c_void},
    mem,
    ops::Range,
    path::PathBuf,
    ptr::{self, NonNull},
    rc::Rc,
    sync::{Arc, Weak},
    time::Duration,
};
use util::ResultExt;

use callbacks::*;
use callbacks_mouse::*;
use constants::*;
use state::*;
pub(crate) use state::{MacWindow, convert_mouse_position};
