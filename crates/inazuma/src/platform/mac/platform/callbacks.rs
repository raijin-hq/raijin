use super::*;
use objc2::DefinedClass;

/// Retrieves the MacPlatform reference from a GPUIApplicationDelegate's ivars.
fn get_mac_platform(delegate: &GPUIApplicationDelegate) -> &MacPlatform {
    let platform_ptr = delegate.ivars().platform.get();
    assert!(!platform_ptr.is_null());
    unsafe { &*(platform_ptr as *const MacPlatform) }
}

/// Retrieves the MacPlatform reference from a GPUIApplication's ivars.
#[allow(dead_code)]
fn get_mac_platform_from_app(app: &GPUIApplication) -> &MacPlatform {
    let platform_ptr = app.ivars().platform.get();
    assert!(!platform_ptr.is_null());
    unsafe { &*(platform_ptr as *const MacPlatform) }
}

pub(super) fn will_finish_launching(_delegate: &GPUIApplicationDelegate) {
    let user_defaults = NSUserDefaults::standardUserDefaults();

    // The autofill heuristic controller causes slowdown and high CPU usage.
    // We don't know exactly why. This disables the full heuristic controller.
    //
    // Adapted from: https://github.com/ghostty-org/ghostty/pull/8625
    let name = NSString::from_str("NSAutoFillHeuristicControllerEnabled");
    let existing_value = user_defaults.objectForKey(&name);
    if existing_value.is_none() {
        user_defaults.setBool_forKey(false, &name);
    }
}

pub(super) fn did_finish_launching(delegate: &GPUIApplicationDelegate) {
    unsafe {
        let mtm = MainThreadMarker::new_unchecked();
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

        let notification_center = NSNotificationCenter::defaultCenter();
        let name =
            NSString::from_str("NSTextInputContextKeyboardSelectionDidChangeNotification");
        let notification_name: &NSNotificationName = &name;

        notification_center.addObserver_selector_name_object(
            delegate as &AnyObject,
            sel!(onKeyboardLayoutChange:),
            Some(notification_name),
            None,
        );

        let thermal_name: &NSNotificationName =
            NSProcessInfoThermalStateDidChangeNotification;
        let process_info = NSProcessInfo::processInfo();
        notification_center.addObserver_selector_name_object(
            delegate as &AnyObject,
            sel!(onThermalStateChange:),
            Some(thermal_name),
            Some(&process_info as &AnyObject),
        );

        let platform = get_mac_platform(delegate);
        let callback = platform.0.lock().finish_launching.take();
        if let Some(callback) = callback {
            callback();
        }
    }
}

pub(super) fn should_handle_reopen(delegate: &GPUIApplicationDelegate, has_open_windows: bool) {
    if !has_open_windows {
        let platform = get_mac_platform(delegate);
        let mut lock = platform.0.lock();
        if let Some(mut callback) = lock.reopen.take() {
            drop(lock);
            callback();
            platform.0.lock().reopen.get_or_insert(callback);
        }
    }
}

pub(super) fn will_terminate(delegate: &GPUIApplicationDelegate) {
    let platform = get_mac_platform(delegate);
    let mut lock = platform.0.lock();
    if let Some(mut callback) = lock.quit.take() {
        drop(lock);
        callback();
        platform.0.lock().quit.get_or_insert(callback);
    }
}

pub(super) fn keyboard_layout_change(delegate: &GPUIApplicationDelegate) {
    let platform = get_mac_platform(delegate);
    let mut lock = platform.0.lock();
    let keyboard_layout = MacKeyboardLayout::new();
    lock.keyboard_mapper = Rc::new(MacKeyboardMapper::new(keyboard_layout.id()));
    if let Some(mut callback) = lock.on_keyboard_layout_change.take() {
        drop(lock);
        callback();
        platform
            .0
            .lock()
            .on_keyboard_layout_change
            .get_or_insert(callback);
    }
}

pub(super) fn thermal_state_change(delegate: &GPUIApplicationDelegate) {
    // Defer to the next run loop iteration to avoid re-entrant borrows of the App RefCell,
    // as NSNotificationCenter delivers this notification synchronously and it may fire while
    // the App is already borrowed (same pattern as quit() above).
    let platform = get_mac_platform(delegate);
    let platform_ptr = platform as *const MacPlatform as *mut c_void;
    unsafe {
        DispatchQueue::main().exec_async_f(platform_ptr, on_thermal_state_change_deferred);
    }

    extern "C" fn on_thermal_state_change_deferred(context: *mut c_void) {
        let platform = unsafe { &*(context as *const MacPlatform) };
        let mut lock = platform.0.lock();
        if let Some(mut callback) = lock.on_thermal_state_change.take() {
            drop(lock);
            callback();
            platform
                .0
                .lock()
                .on_thermal_state_change
                .get_or_insert(callback);
        }
    }
}

pub(super) fn open_urls_callback(
    delegate: &GPUIApplicationDelegate,
    urls: &NSArray<NSURL>,
) {
    let url_strings: Vec<String> = (0..urls.count())
        .filter_map(|i| {
            let url = urls.objectAtIndex(i);
            url.absoluteString().map(|s| s.to_string())
        })
        .collect();

    let platform = get_mac_platform(delegate);
    let mut lock = platform.0.lock();
    if let Some(mut callback) = lock.open_urls.take() {
        drop(lock);
        callback(url_strings);
        platform.0.lock().open_urls.get_or_insert(callback);
    }
}

pub(super) fn handle_menu_item(delegate: &GPUIApplicationDelegate, item: &NSMenuItem) {
    let platform = get_mac_platform(delegate);
    let mut lock = platform.0.lock();
    if let Some(mut callback) = lock.menu_command.take() {
        let tag = item.tag();
        let index = tag as usize;
        if let Some(action) = lock.menu_actions.get(index) {
            let action = action.boxed_clone();
            drop(lock);
            callback(&*action);
        }
        platform.0.lock().menu_command.get_or_insert(callback);
    }
}

pub(super) fn validate_menu_item_callback(
    delegate: &GPUIApplicationDelegate,
    item: &NSMenuItem,
) -> bool {
    let platform = get_mac_platform(delegate);
    let mut lock = platform.0.lock();
    let mut result = false;
    if let Some(mut callback) = lock.validate_menu_command.take() {
        let tag = item.tag();
        let index = tag as usize;
        if let Some(action) = lock.menu_actions.get(index) {
            let action = action.boxed_clone();
            drop(lock);
            result = callback(action.as_ref());
        }
        platform
            .0
            .lock()
            .validate_menu_command
            .get_or_insert(callback);
    }
    result
}

pub(super) fn menu_will_open_callback(delegate: &GPUIApplicationDelegate) {
    let platform = get_mac_platform(delegate);
    let mut lock = platform.0.lock();
    if let Some(mut callback) = lock.will_open_menu.take() {
        drop(lock);
        callback();
        platform.0.lock().will_open_menu.get_or_insert(callback);
    }
}

pub(super) fn handle_dock_menu(delegate: &GPUIApplicationDelegate) -> *mut NSMenu {
    let platform = get_mac_platform(delegate);
    let state = platform.0.lock();
    if let Some(ref menu) = state.dock_menu {
        Retained::as_ptr(menu) as *mut NSMenu
    } else {
        ptr::null_mut()
    }
}

pub(super) fn ns_url_to_path(url: &NSURL) -> Result<PathBuf> {
    let path_ptr = url.fileSystemRepresentation();
    let path = unsafe { CStr::from_ptr(path_ptr.as_ptr()) };
    Ok(PathBuf::from(OsStr::from_bytes(path.to_bytes())))
}

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    pub(crate) fn TISCopyCurrentKeyboardLayoutInputSource() -> *mut AnyObject;
    pub(crate) fn TISGetInputSourceProperty(
        inputSource: *mut AnyObject,
        propertyKey: *const c_void,
    ) -> *mut AnyObject;

    pub(crate) fn UCKeyTranslate(
        keyLayoutPtr: *const ::std::os::raw::c_void,
        virtualKeyCode: u16,
        keyAction: u16,
        modifierKeyState: u32,
        keyboardType: u32,
        keyTranslateOptions: u32,
        deadKeyState: *mut u32,
        maxStringLength: usize,
        actualStringLength: *mut usize,
        unicodeString: *mut u16,
    ) -> u32;
    pub(crate) fn LMGetKbdType() -> u16;
    pub(crate) static kTISPropertyUnicodeKeyLayoutData: *const CFString;
    pub(crate) static kTISPropertyInputSourceID: *const CFString;
    pub(crate) static kTISPropertyLocalizedName: *const CFString;
}

pub(super) mod security {
    #![allow(non_upper_case_globals)]
    use super::*;

    /// OSStatus is a 32-bit signed integer used by macOS frameworks.
    pub type OSStatus = i32;

    #[link(name = "Security", kind = "framework")]
    unsafe extern "C" {
        pub static kSecClass: *const CFString;
        pub static kSecClassInternetPassword: *const CFString;
        pub static kSecAttrServer: *const CFString;
        pub static kSecAttrAccount: *const CFString;
        pub static kSecValueData: *const CFString;
        pub static kSecReturnAttributes: *const CFString;
        pub static kSecReturnData: *const CFString;

        pub fn SecItemAdd(attributes: *const CFDictionary, result: *mut *const c_void) -> OSStatus;
        pub fn SecItemUpdate(query: *const CFDictionary, attributes: *const CFDictionary) -> OSStatus;
        pub fn SecItemDelete(query: *const CFDictionary) -> OSStatus;
        pub fn SecItemCopyMatching(query: *const CFDictionary, result: *mut *const c_void) -> OSStatus;
    }

    pub const errSecSuccess: OSStatus = 0;
    pub const errSecUserCanceled: OSStatus = -128;
    pub const errSecItemNotFound: OSStatus = -25300;
}
