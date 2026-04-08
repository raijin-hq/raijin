mod callbacks;
mod platform_impl;
mod state;

use super::{MacDispatcher, MacDisplay, MacKeyboardLayout, MacKeyboardMapper, pasteboard::Pasteboard, renderer};
use crate::command::{new_command, new_std_command};
use anyhow::{Context as _, anyhow};
use block2::RcBlock;
use objc2_core_foundation::{
    CFBoolean, CFData, CFDictionary, CFMutableDictionary, CFRunLoop, CFString,
    kCFBooleanTrue, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks,
};
use dispatch2::DispatchQueue;
use futures::channel::oneshot;
use inazuma::{
    Action, AnyWindowHandle, BackgroundExecutor, ClipboardItem, CursorStyle, ForegroundExecutor,
    KeyContext, Keymap, Menu, MenuItem, OsMenu, OwnedMenu, PathPromptOptions, Platform,
    PlatformDisplay, PlatformKeyboardLayout, PlatformKeyboardMapper, PlatformTextSystem,
    PlatformWindow, Result, SystemMenuType, Task, ThermalState, WindowAppearance, WindowParams,
};
use itertools::Itertools;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{define_class, msg_send, sel, ClassType, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSCursor,
    NSDocumentController, NSEventModifierFlags, NSMenu, NSMenuDelegate, NSMenuItem,
    NSModalResponse, NSModalResponseOK, NSOpenPanel, NSSavePanel, NSScroller, NSScrollerStyle,
    NSWorkspace,
};
use objc2_foundation::{
    NSArray, NSBundle, NSError, NSInteger, NSNotification, NSNotificationCenter,
    NSNotificationName, NSObject, NSObjectProtocol, NSProcessInfo, NSProcessInfoThermalState,
    NSProcessInfoThermalStateDidChangeNotification, NSString, NSUserDefaults, NSURL,
};
use parking_lot::Mutex;
use semver::Version;
use std::{
    cell::Cell,
    ffi::{CStr, OsStr, c_void},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    ptr,
    rc::Rc,
    sync::{Arc, OnceLock},
};
use inazuma_util::ResultExt;

use callbacks::*;
pub use state::MacPlatform;
pub(crate) use callbacks::{
    TISCopyCurrentKeyboardLayoutInputSource, TISGetInputSourceProperty, UCKeyTranslate,
    LMGetKbdType, kTISPropertyUnicodeKeyLayoutData, kTISPropertyInputSourceID,
    kTISPropertyLocalizedName,
};

use super::events::key_to_native;

/// Ivars for GPUIApplication — stores a raw pointer to the MacPlatform.
#[derive(Default)]
struct AppIvars {
    platform: Cell<*const c_void>,
}

/// Ivars for GPUIApplicationDelegate — stores a raw pointer to the MacPlatform.
#[derive(Default)]
struct DelegateIvars {
    platform: Cell<*const c_void>,
}

define_class!(
    #[unsafe(super(NSApplication))]
    #[name = "GPUIApplication"]
    #[ivars = AppIvars]
    #[thread_kind = MainThreadOnly]
    struct GPUIApplication;

    impl GPUIApplication {}
);

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "GPUIApplicationDelegate"]
    #[ivars = DelegateIvars]
    #[thread_kind = MainThreadOnly]
    struct GPUIApplicationDelegate;

    impl GPUIApplicationDelegate {
        #[unsafe(method(applicationWillFinishLaunching:))]
        fn application_will_finish_launching(&self, _notification: &NSNotification) {
            will_finish_launching(self);
        }

        #[unsafe(method(applicationDidFinishLaunching:))]
        fn application_did_finish_launching(&self, _notification: &NSNotification) {
            did_finish_launching(self);
        }

        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn application_should_handle_reopen(
            &self,
            _sender: &NSApplication,
            has_visible_windows: bool,
        ) -> bool {
            should_handle_reopen(self, has_visible_windows);
            true
        }

        #[unsafe(method(applicationWillTerminate:))]
        fn application_will_terminate(&self, _notification: &NSNotification) {
            will_terminate(self);
        }

        #[unsafe(method(handleGPUIMenuItem:))]
        fn handle_gpui_menu_item(&self, item: &NSMenuItem) {
            handle_menu_item(self, item);
        }

        #[unsafe(method(cut:))]
        fn cut(&self, item: &NSMenuItem) {
            handle_menu_item(self, item);
        }

        #[unsafe(method(copy:))]
        fn copy(&self, item: &NSMenuItem) {
            handle_menu_item(self, item);
        }

        #[unsafe(method(paste:))]
        fn paste(&self, item: &NSMenuItem) {
            handle_menu_item(self, item);
        }

        #[unsafe(method(selectAll:))]
        fn select_all(&self, item: &NSMenuItem) {
            handle_menu_item(self, item);
        }

        #[unsafe(method(undo:))]
        fn undo(&self, item: &NSMenuItem) {
            handle_menu_item(self, item);
        }

        #[unsafe(method(redo:))]
        fn redo(&self, item: &NSMenuItem) {
            handle_menu_item(self, item);
        }

        #[unsafe(method(validateMenuItem:))]
        fn validate_menu_item(&self, item: &NSMenuItem) -> bool {
            validate_menu_item_callback(self, item)
        }

        #[unsafe(method(menuWillOpen:))]
        fn menu_will_open(&self, _menu: &NSMenu) {
            menu_will_open_callback(self);
        }

        #[unsafe(method(applicationDockMenu:))]
        fn application_dock_menu(&self, _sender: &NSApplication) -> *mut NSMenu {
            handle_dock_menu(self)
        }

        #[unsafe(method(application:openURLs:))]
        fn application_open_urls(&self, _application: &NSApplication, urls: &NSArray<NSURL>) {
            open_urls_callback(self, urls);
        }

        #[unsafe(method(onKeyboardLayoutChange:))]
        fn on_keyboard_layout_change(&self, _notification: &NSNotification) {
            keyboard_layout_change(self);
        }

        #[unsafe(method(onThermalStateChange:))]
        fn on_thermal_state_change(&self, _notification: &NSNotification) {
            thermal_state_change(self);
        }
    }

    unsafe impl NSObjectProtocol for GPUIApplicationDelegate {}

    unsafe impl NSApplicationDelegate for GPUIApplicationDelegate {}

    unsafe impl NSMenuDelegate for GPUIApplicationDelegate {}
);
