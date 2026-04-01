mod callbacks;
mod platform_impl;
mod state;

use super::{
    BoolExt, MacDispatcher, MacDisplay, MacKeyboardLayout, MacKeyboardMapper, MacWindow,
    events::key_to_native, ns_string, pasteboard::Pasteboard, renderer,
};
use crate::command::{new_command, new_std_command};
use anyhow::{Context as _, anyhow};
use block::ConcreteBlock;
use cocoa::{
    appkit::{
        NSApplication, NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular,
        NSControl as _, NSEventModifierFlags, NSMenu, NSMenuItem, NSModalResponse, NSOpenPanel,
        NSSavePanel, NSVisualEffectState, NSVisualEffectView, NSWindow,
    },
    base::{BOOL, NO, YES, id, nil, selector},
    foundation::{
        NSArray, NSAutoreleasePool, NSBundle, NSInteger, NSProcessInfo, NSString, NSUInteger, NSURL,
    },
};
use core_foundation::{
    base::{CFRelease, CFType, CFTypeRef, OSStatus, TCFType},
    boolean::CFBoolean,
    data::CFData,
    dictionary::{CFDictionary, CFDictionaryRef, CFMutableDictionary},
    runloop::CFRunLoopRun,
    string::{CFString, CFStringRef},
};
use ctor::ctor;
use dispatch2::DispatchQueue;
use futures::channel::oneshot;
use inazuma::{
    Action, AnyWindowHandle, BackgroundExecutor, ClipboardItem, CursorStyle, ForegroundExecutor,
    KeyContext, Keymap, Menu, MenuItem, OsMenu, OwnedMenu, PathPromptOptions, Platform,
    PlatformDisplay, PlatformKeyboardLayout, PlatformKeyboardMapper, PlatformTextSystem,
    PlatformWindow, Result, SystemMenuType, Task, ThermalState, WindowAppearance, WindowParams,
};
use itertools::Itertools;
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{Class, Object, Sel},
    sel, sel_impl,
};
use parking_lot::Mutex;
use ptr::null_mut;
use semver::Version;
use std::{
    cell::Cell,
    ffi::{CStr, OsStr, c_void},
    os::{raw::c_char, unix::ffi::OsStrExt},
    path::{Path, PathBuf},
    ptr,
    rc::Rc,
    slice, str,
    sync::{Arc, OnceLock},
};
use util::ResultExt;

#[allow(non_upper_case_globals)]
const NSUTF8StringEncoding: NSUInteger = 4;

use callbacks::*;
use state::*;
pub use state::MacPlatform;
pub(crate) use callbacks::{
    TISCopyCurrentKeyboardLayoutInputSource, TISGetInputSourceProperty, UCKeyTranslate,
    LMGetKbdType, kTISPropertyUnicodeKeyLayoutData, kTISPropertyInputSourceID,
    kTISPropertyLocalizedName,
};

#[ctor]
unsafe fn build_classes() {
    unsafe {
        APP_CLASS = {
            let mut decl = ClassDecl::new("GPUIApplication", class!(NSApplication)).unwrap();
            decl.add_ivar::<*mut c_void>(MAC_PLATFORM_IVAR);
            decl.register()
        }
    };
    unsafe {
        APP_DELEGATE_CLASS = unsafe {
            let mut decl = ClassDecl::new("GPUIApplicationDelegate", class!(NSResponder)).unwrap();
            decl.add_ivar::<*mut c_void>(MAC_PLATFORM_IVAR);
            decl.add_method(
                sel!(applicationWillFinishLaunching:),
                will_finish_launching as extern "C" fn(&mut Object, Sel, id),
            );
            decl.add_method(
                sel!(applicationDidFinishLaunching:),
                did_finish_launching as extern "C" fn(&mut Object, Sel, id),
            );
            decl.add_method(
                sel!(applicationShouldHandleReopen:hasVisibleWindows:),
                should_handle_reopen as extern "C" fn(&mut Object, Sel, id, bool),
            );
            decl.add_method(
                sel!(applicationWillTerminate:),
                will_terminate as extern "C" fn(&mut Object, Sel, id),
            );
            decl.add_method(
                sel!(handleGPUIMenuItem:),
                handle_menu_item as extern "C" fn(&mut Object, Sel, id),
            );
            // Add menu item handlers so that OS save panels have the correct key commands
            decl.add_method(
                sel!(cut:),
                handle_menu_item as extern "C" fn(&mut Object, Sel, id),
            );
            decl.add_method(
                sel!(copy:),
                handle_menu_item as extern "C" fn(&mut Object, Sel, id),
            );
            decl.add_method(
                sel!(paste:),
                handle_menu_item as extern "C" fn(&mut Object, Sel, id),
            );
            decl.add_method(
                sel!(selectAll:),
                handle_menu_item as extern "C" fn(&mut Object, Sel, id),
            );
            decl.add_method(
                sel!(undo:),
                handle_menu_item as extern "C" fn(&mut Object, Sel, id),
            );
            decl.add_method(
                sel!(redo:),
                handle_menu_item as extern "C" fn(&mut Object, Sel, id),
            );
            decl.add_method(
                sel!(validateMenuItem:),
                validate_menu_item as extern "C" fn(&mut Object, Sel, id) -> bool,
            );
            decl.add_method(
                sel!(menuWillOpen:),
                menu_will_open as extern "C" fn(&mut Object, Sel, id),
            );
            decl.add_method(
                sel!(applicationDockMenu:),
                handle_dock_menu as extern "C" fn(&mut Object, Sel, id) -> id,
            );
            decl.add_method(
                sel!(application:openURLs:),
                open_urls as extern "C" fn(&mut Object, Sel, id, id),
            );

            decl.add_method(
                sel!(onKeyboardLayoutChange:),
                on_keyboard_layout_change as extern "C" fn(&mut Object, Sel, id),
            );

            decl.add_method(
                sel!(onThermalStateChange:),
                on_thermal_state_change as extern "C" fn(&mut Object, Sel, id),
            );

            decl.register()
        }
    }
}
