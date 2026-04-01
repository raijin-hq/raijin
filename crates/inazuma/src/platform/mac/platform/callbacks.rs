use super::*;

pub(super) unsafe fn path_from_objc(path: id) -> PathBuf {
    let len = msg_send![path, lengthOfBytesUsingEncoding: NSUTF8StringEncoding];
    let bytes = unsafe { path.UTF8String() as *const u8 };
    let path = str::from_utf8(unsafe { slice::from_raw_parts(bytes, len) }).unwrap();
    PathBuf::from(path)
}

pub(super) unsafe fn get_mac_platform(object: &mut Object) -> &MacPlatform {
    unsafe {
        let platform_ptr: *mut c_void = *object.get_ivar(MAC_PLATFORM_IVAR);
        assert!(!platform_ptr.is_null());
        &*(platform_ptr as *const MacPlatform)
    }
}

pub(super) extern "C" fn will_finish_launching(_this: &mut Object, _: Sel, _: id) {
    unsafe {
        let user_defaults: id = msg_send![class!(NSUserDefaults), standardUserDefaults];

        // The autofill heuristic controller causes slowdown and high CPU usage.
        // We don't know exactly why. This disables the full heuristic controller.
        //
        // Adapted from: https://github.com/ghostty-org/ghostty/pull/8625
        let name = ns_string("NSAutoFillHeuristicControllerEnabled");
        let existing_value: id = msg_send![user_defaults, objectForKey: name];
        if existing_value == nil {
            let false_value: id = msg_send![class!(NSNumber), numberWithBool:false];
            let _: () = msg_send![user_defaults, setObject: false_value forKey: name];
        }
    }
}

pub(super) extern "C" fn did_finish_launching(this: &mut Object, _: Sel, _: id) {
    unsafe {
        let app: id = msg_send![APP_CLASS, sharedApplication];
        app.setActivationPolicy_(NSApplicationActivationPolicyRegular);

        let notification_center: *mut Object =
            msg_send![class!(NSNotificationCenter), defaultCenter];
        let name = ns_string("NSTextInputContextKeyboardSelectionDidChangeNotification");
        let _: () = msg_send![notification_center, addObserver: this as id
            selector: sel!(onKeyboardLayoutChange:)
            name: name
            object: nil
        ];

        let thermal_name = ns_string("NSProcessInfoThermalStateDidChangeNotification");
        let process_info: id = msg_send![class!(NSProcessInfo), processInfo];
        let _: () = msg_send![notification_center, addObserver: this as id
            selector: sel!(onThermalStateChange:)
            name: thermal_name
            object: process_info
        ];

        let platform = get_mac_platform(this);
        let callback = platform.0.lock().finish_launching.take();
        if let Some(callback) = callback {
            callback();
        }
    }
}

pub(super) extern "C" fn should_handle_reopen(this: &mut Object, _: Sel, _: id, has_open_windows: bool) {
    if !has_open_windows {
        let platform = unsafe { get_mac_platform(this) };
        let mut lock = platform.0.lock();
        if let Some(mut callback) = lock.reopen.take() {
            drop(lock);
            callback();
            platform.0.lock().reopen.get_or_insert(callback);
        }
    }
}

pub(super) extern "C" fn will_terminate(this: &mut Object, _: Sel, _: id) {
    let platform = unsafe { get_mac_platform(this) };
    let mut lock = platform.0.lock();
    if let Some(mut callback) = lock.quit.take() {
        drop(lock);
        callback();
        platform.0.lock().quit.get_or_insert(callback);
    }
}

pub(super) extern "C" fn on_keyboard_layout_change(this: &mut Object, _: Sel, _: id) {
    let platform = unsafe { get_mac_platform(this) };
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

pub(super) extern "C" fn on_thermal_state_change(this: &mut Object, _: Sel, _: id) {
    // Defer to the next run loop iteration to avoid re-entrant borrows of the App RefCell,
    // as NSNotificationCenter delivers this notification synchronously and it may fire while
    // the App is already borrowed (same pattern as quit() above).
    let platform = unsafe { get_mac_platform(this) };
    let platform_ptr = platform as *const MacPlatform as *mut c_void;
    unsafe {
        DispatchQueue::main().exec_async_f(platform_ptr, on_thermal_state_change);
    }

    extern "C" fn on_thermal_state_change(context: *mut c_void) {
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

pub(super) extern "C" fn open_urls(this: &mut Object, _: Sel, _: id, urls: id) {
    let urls = unsafe {
        (0..urls.count())
            .filter_map(|i| {
                let url = urls.objectAtIndex(i);
                match CStr::from_ptr(url.absoluteString().UTF8String() as *mut c_char).to_str() {
                    Ok(string) => Some(string.to_string()),
                    Err(err) => {
                        log::error!("error converting path to string: {}", err);
                        None
                    }
                }
            })
            .collect::<Vec<_>>()
    };
    let platform = unsafe { get_mac_platform(this) };
    let mut lock = platform.0.lock();
    if let Some(mut callback) = lock.open_urls.take() {
        drop(lock);
        callback(urls);
        platform.0.lock().open_urls.get_or_insert(callback);
    }
}

pub(super) extern "C" fn handle_menu_item(this: &mut Object, _: Sel, item: id) {
    unsafe {
        let platform = get_mac_platform(this);
        let mut lock = platform.0.lock();
        if let Some(mut callback) = lock.menu_command.take() {
            let tag: NSInteger = msg_send![item, tag];
            let index = tag as usize;
            if let Some(action) = lock.menu_actions.get(index) {
                let action = action.boxed_clone();
                drop(lock);
                callback(&*action);
            }
            platform.0.lock().menu_command.get_or_insert(callback);
        }
    }
}

pub(super) extern "C" fn validate_menu_item(this: &mut Object, _: Sel, item: id) -> bool {
    unsafe {
        let mut result = false;
        let platform = get_mac_platform(this);
        let mut lock = platform.0.lock();
        if let Some(mut callback) = lock.validate_menu_command.take() {
            let tag: NSInteger = msg_send![item, tag];
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
}

pub(super) extern "C" fn menu_will_open(this: &mut Object, _: Sel, _: id) {
    unsafe {
        let platform = get_mac_platform(this);
        let mut lock = platform.0.lock();
        if let Some(mut callback) = lock.will_open_menu.take() {
            drop(lock);
            callback();
            platform.0.lock().will_open_menu.get_or_insert(callback);
        }
    }
}

pub(super) extern "C" fn handle_dock_menu(this: &mut Object, _: Sel, _: id) -> id {
    unsafe {
        let platform = get_mac_platform(this);
        let state = platform.0.lock();
        if let Some(id) = state.dock_menu {
            id
        } else {
            nil
        }
    }
}

pub(super) unsafe fn ns_url_to_path(url: id) -> Result<PathBuf> {
    let path: *mut c_char = msg_send![url, fileSystemRepresentation];
    anyhow::ensure!(!path.is_null(), "url is not a file path: {}", unsafe {
        CStr::from_ptr(url.absoluteString().UTF8String()).to_string_lossy()
    });
    Ok(PathBuf::from(OsStr::from_bytes(unsafe {
        CStr::from_ptr(path).to_bytes()
    })))
}

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    pub(crate) fn TISCopyCurrentKeyboardLayoutInputSource() -> *mut Object;
    pub(crate) fn TISGetInputSourceProperty(
        inputSource: *mut Object,
        propertyKey: *const c_void,
    ) -> *mut Object;

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
    pub(crate) static kTISPropertyUnicodeKeyLayoutData: CFStringRef;
    pub(crate) static kTISPropertyInputSourceID: CFStringRef;
    pub(crate) static kTISPropertyLocalizedName: CFStringRef;
}

pub(super) mod security {
    #![allow(non_upper_case_globals)]
    use super::*;

    #[link(name = "Security", kind = "framework")]
    unsafe extern "C" {
        pub static kSecClass: CFStringRef;
        pub static kSecClassInternetPassword: CFStringRef;
        pub static kSecAttrServer: CFStringRef;
        pub static kSecAttrAccount: CFStringRef;
        pub static kSecValueData: CFStringRef;
        pub static kSecReturnAttributes: CFStringRef;
        pub static kSecReturnData: CFStringRef;

        pub fn SecItemAdd(attributes: CFDictionaryRef, result: *mut CFTypeRef) -> OSStatus;
        pub fn SecItemUpdate(query: CFDictionaryRef, attributes: CFDictionaryRef) -> OSStatus;
        pub fn SecItemDelete(query: CFDictionaryRef) -> OSStatus;
        pub fn SecItemCopyMatching(query: CFDictionaryRef, result: *mut CFTypeRef) -> OSStatus;
    }

    pub const errSecSuccess: OSStatus = 0;
    pub const errSecUserCanceled: OSStatus = -128;
    pub const errSecItemNotFound: OSStatus = -25300;
}
