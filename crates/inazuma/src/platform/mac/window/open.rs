use super::*;
use class_registration::{GPUIPanel, GPUIView, GPUIWindow};
use objc2::{ClassType, DefinedClass};

impl MacWindow {
    pub fn open(
        handle: AnyWindowHandle,
        WindowParams {
            bounds,
            titlebar,
            kind,
            is_movable,
            is_resizable,
            is_minimizable,
            focus,
            show,
            display_id,
            window_min_size,
            tabbing_identifier,
        }: WindowParams,
        foreground_executor: ForegroundExecutor,
        background_executor: BackgroundExecutor,
        renderer_context: renderer::Context,
    ) -> Self {
        unsafe {
            let pool = objc2_foundation::NSAutoreleasePool::new();

            let allows_automatic_window_tabbing = tabbing_identifier.is_some();
            let _: () = msg_send![
                objc2_app_kit::NSWindow::class(),
                setAllowsAutomaticWindowTabbing: allows_automatic_window_tabbing
            ];

            let mut style_mask;
            if let Some(titlebar) = titlebar.as_ref() {
                style_mask = objc2_app_kit::NSWindowStyleMask::Closable
                    | objc2_app_kit::NSWindowStyleMask::Titled;

                if is_resizable {
                    style_mask |= objc2_app_kit::NSWindowStyleMask::Resizable;
                }

                if is_minimizable {
                    style_mask |= objc2_app_kit::NSWindowStyleMask::Miniaturizable;
                }

                if titlebar.appears_transparent {
                    style_mask |= objc2_app_kit::NSWindowStyleMask::FullSizeContentView;
                }
            } else {
                style_mask = objc2_app_kit::NSWindowStyleMask::Titled
                    | objc2_app_kit::NSWindowStyleMask::FullSizeContentView;
            }

            let mtm = objc2::MainThreadMarker::new_unchecked();

            let display = display_id
                .and_then(MacDisplay::find_by_id)
                .unwrap_or_else(MacDisplay::primary);

            let mut target_screen: *mut AnyObject = ptr::null_mut();
            let mut screen_frame = None;

            let screens = objc2_app_kit::NSScreen::screens(mtm);
            let count = screens.count();
            for i in 0..count {
                let screen = screens.objectAtIndex(i);
                let frame = screen.frame();
                let screen_ptr: *mut AnyObject = &*screen as *const _ as *mut _;
                let display_id = display_id_for_screen(screen_ptr);
                if display_id == display.0 {
                    screen_frame = Some(frame);
                    target_screen = screen_ptr;
                }
            }

            let screen_frame = screen_frame.unwrap_or_else(|| {
                let screen = objc2_app_kit::NSScreen::mainScreen(mtm);
                if let Some(ref screen) = screen {
                    target_screen = &**screen as *const _ as *mut _;
                    screen.frame()
                } else {
                    NSRect::new(NSPoint::new(0., 0.), NSSize::new(1920., 1080.))
                }
            });

            let window_rect = NSRect::new(
                NSPoint::new(
                    screen_frame.origin.x + bounds.origin.x.as_f32() as f64,
                    screen_frame.origin.y
                        + (display.bounds().size.height - bounds.origin.y).as_f32() as f64,
                ),
                NSSize::new(
                    bounds.size.width.as_f32() as f64,
                    bounds.size.height.as_f32() as f64,
                ),
            );

            // Alloc + set_ivars + init in one step via the typed init method
            // defined in define_class!. set_ivars is called inside init_with_content_rect.
            // Alloc + init with set_ivars handled inside the define_class! init method.
            // We use msg_send! to call initWithContentRect: which internally calls set_ivars.
            let native_window: *mut AnyObject = match kind {
                WindowKind::Normal => {
                    let alloc = mtm.alloc::<GPUIWindow>();
                    let window: Option<objc2::rc::Retained<GPUIWindow>> = msg_send![
                        super(alloc.set_ivars(WindowIvars::default())),
                        initWithContentRect: window_rect,
                        styleMask: style_mask,
                        backing: 2isize,
                        defer: false,
                        screen: target_screen
                    ];
                    objc2::rc::Retained::into_raw(window.expect("failed to init GPUIWindow")) as *mut AnyObject
                }
                WindowKind::PopUp => {
                    style_mask |= NSWindowStyleMaskNonactivatingPanel;
                    let alloc = mtm.alloc::<GPUIPanel>();
                    let window: Option<objc2::rc::Retained<GPUIPanel>> = msg_send![
                        super(alloc.set_ivars(WindowIvars::default())),
                        initWithContentRect: window_rect,
                        styleMask: style_mask,
                        backing: 2isize,
                        defer: false,
                        screen: target_screen
                    ];
                    objc2::rc::Retained::into_raw(window.expect("failed to init GPUIPanel")) as *mut AnyObject
                }
                WindowKind::Floating | WindowKind::Dialog => {
                    let alloc = mtm.alloc::<GPUIPanel>();
                    let window: Option<objc2::rc::Retained<GPUIPanel>> = msg_send![
                        super(alloc.set_ivars(WindowIvars::default())),
                        initWithContentRect: window_rect,
                        styleMask: style_mask,
                        backing: 2isize,
                        defer: false,
                        screen: target_screen
                    ];
                    objc2::rc::Retained::into_raw(window.expect("failed to init GPUIPanel")) as *mut AnyObject
                }
            };
            assert!(!native_window.is_null());

            #[allow(deprecated)]
            let filenames_type: objc2::rc::Retained<objc2_foundation::NSString> = objc2_app_kit::NSFilenamesPboardType.to_owned();
            let types_array = objc2_foundation::NSArray::from_retained_slice(&[filenames_type]);
            let _: () = msg_send![native_window, registerForDraggedTypes: &*types_array];
            let _: () = msg_send![native_window, setReleasedWhenClosed: false];

            let content_view: *mut AnyObject = msg_send![native_window, contentView];
            let content_bounds: NSRect = msg_send![content_view, bounds];
            let native_view: *mut AnyObject = {
                let alloc = mtm.alloc::<GPUIView>();
                let view: Option<objc2::rc::Retained<GPUIView>> = msg_send![
                    super(alloc.set_ivars(WindowIvars::default())),
                    initWithFrame: content_bounds
                ];
                objc2::rc::Retained::into_raw(view.expect("failed to init GPUIView")) as *mut AnyObject
            };
            assert!(!native_view.is_null());

            let mut window = Self(Arc::new(Mutex::new(MacWindowState {
                handle,
                foreground_executor,
                background_executor,
                native_window,
                native_view: NonNull::new_unchecked(native_view),
                blurred_view: None,
                background_appearance: WindowBackgroundAppearance::Opaque,
                display_link: None,
                renderer: renderer::new_renderer(
                    renderer_context,
                    native_window as *mut _,
                    native_view as *mut _,
                    bounds.size.map(|pixels| pixels.as_f32()),
                    false,
                ),
                request_frame_callback: None,
                event_callback: None,
                activate_callback: None,
                resize_callback: None,
                moved_callback: None,
                should_close_callback: None,
                close_callback: None,
                appearance_changed_callback: None,
                input_handler: None,
                last_key_equivalent: None,
                synthetic_drag_counter: 0,
                traffic_light_position: titlebar
                    .as_ref()
                    .and_then(|titlebar| titlebar.traffic_light_position),
                transparent_titlebar: titlebar
                    .as_ref()
                    .is_none_or(|titlebar| titlebar.appears_transparent),
                previous_modifiers_changed_event: None,
                keystroke_for_do_command: None,
                do_command_handled: None,
                external_files_dragged: false,
                first_mouse: false,
                fullscreen_restore_bounds: Bounds::default(),
                move_tab_to_new_window_callback: None,
                merge_all_windows_callback: None,
                select_next_tab_callback: None,
                select_previous_tab_callback: None,
                toggle_tab_bar_callback: None,
                activated_least_once: false,
                sheet_parent: None,
            })));

            // Set window state ivar on the window
            let window_state_ptr = Arc::into_raw(window.0.clone()) as *const c_void;
            let window_obj: &GPUIWindow = &*(native_window as *const GPUIWindow);
            window_obj.ivars().window_state.set(window_state_ptr);
            let _: () = msg_send![native_window, setDelegate: native_window];

            // Set window state ivar on the view
            let view_state_ptr = Arc::into_raw(window.0.clone()) as *const c_void;
            let view_obj: &GPUIView = &*(native_view as *const GPUIView);
            view_obj.ivars().window_state.set(view_state_ptr);

            if let Some(title) = titlebar
                .as_ref()
                .and_then(|t| t.title.as_ref().map(AsRef::as_ref))
            {
                window.set_title(title);
            }

            let _: () = msg_send![native_window, setMovable: is_movable];

            if let Some(window_min_size) = window_min_size {
                let min_size = NSSize {
                    width: window_min_size.width.to_f64(),
                    height: window_min_size.height.to_f64(),
                };
                let _: () = msg_send![native_window, setContentMinSize: min_size];
            }

            if titlebar.is_none_or(|titlebar| titlebar.appears_transparent) {
                let _: () = msg_send![native_window, setTitlebarAppearsTransparent: true];
                let _: () = msg_send![native_window, setTitleVisibility: 1isize]; // NSWindowTitleHidden
            }

            let autoresizing_mask: usize = (1 << 4) | (1 << 1); // NSViewWidthSizable | NSViewHeightSizable
            let _: () = msg_send![native_view, setAutoresizingMask: autoresizing_mask];
            let _: () = msg_send![native_view, setWantsBestResolutionOpenGLSurface: true];

            // From winit crate: On Mojave, views automatically become layer-backed shortly after
            // being added to a native_window. Changing the layer-backedness of a view breaks the
            // association between the view and its associated OpenGL context. To work around this,
            // on we explicitly make the view layer-backed up front so that AppKit doesn't do it
            // itself and break the association with its context.
            let _: () = msg_send![native_view, setWantsLayer: true];
            let _: () = msg_send![
                native_view,
                setLayerContentsRedrawPolicy: NSViewLayerContentsRedrawDuringViewResize
            ];

            let _: () = msg_send![content_view, addSubview: native_view];
            let _: *mut AnyObject = msg_send![native_view, autorelease];
            let _: bool = msg_send![native_window, makeFirstResponder: native_view];

            let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
            let main_window: *mut AnyObject = msg_send![&*app, mainWindow];
            let mut sheet_parent = None;

            match kind {
                WindowKind::Normal | WindowKind::Floating => {
                    if kind == WindowKind::Floating {
                        // Let the window float keep above normal windows.
                        let _: () = msg_send![native_window, setLevel: NSFloatingWindowLevel];
                    } else {
                        let _: () = msg_send![native_window, setLevel: NSNormalWindowLevel];
                    }
                    let _: () = msg_send![native_window, setAcceptsMouseMovedEvents: true];

                    if let Some(tabbing_identifier) = tabbing_identifier {
                        let tabbing_id =
                            objc2_foundation::NSString::from_str(tabbing_identifier.as_str());
                        let _: () = msg_send![native_window, setTabbingIdentifier: &*tabbing_id];
                    } else {
                        let _: () = msg_send![
                            native_window,
                            setTabbingIdentifier: ptr::null_mut::<AnyObject>()
                        ];
                    }
                }
                WindowKind::PopUp => {
                    // Use a tracking area to allow receiving MouseMoved events even when
                    // the window or application aren't active, which is often the case
                    // e.g. for notification windows.
                    let tracking_area: *mut AnyObject =
                        msg_send![objc2_app_kit::NSTrackingArea::class(), alloc];
                    let tracking_options: usize = NSTrackingMouseEnteredAndExited
                        | NSTrackingMouseMoved
                        | NSTrackingActiveAlways
                        | NSTrackingInVisibleRect;
                    let _: *mut AnyObject = msg_send![
                        tracking_area,
                        initWithRect: NSRect::new(NSPoint::new(0., 0.), NSSize::new(0., 0.)),
                        options: tracking_options,
                        owner: native_view,
                        userInfo: ptr::null_mut::<AnyObject>()
                    ];
                    let _: () = msg_send![native_view, addTrackingArea: tracking_area];
                    let _: *mut AnyObject = msg_send![tracking_area, autorelease];

                    let _: () = msg_send![native_window, setLevel: NSPopUpWindowLevel];
                    let _: () = msg_send![
                        native_window,
                        setAnimationBehavior: NSWindowAnimationBehaviorUtilityWindow
                    ];
                    let collection_behavior: usize = (1 << 0) | (1 << 8); // CanJoinAllSpaces | FullScreenAuxiliary
                    let _: () = msg_send![native_window, setCollectionBehavior: collection_behavior];
                }
                WindowKind::Dialog => {
                    if !main_window.is_null() {
                        let parent = {
                            let active_sheet: *mut AnyObject =
                                msg_send![main_window, attachedSheet];
                            if active_sheet.is_null() {
                                main_window
                            } else {
                                active_sheet
                            }
                        };
                        let _: () = msg_send![
                            parent,
                            beginSheet: native_window,
                            completionHandler: ptr::null_mut::<AnyObject>()
                        ];
                        sheet_parent = Some(parent);
                    }
                }
            }

            if allows_automatic_window_tabbing
                && !main_window.is_null()
                && main_window != native_window
            {
                let main_style: objc2_app_kit::NSWindowStyleMask =
                    msg_send![main_window, styleMask];
                let main_window_is_fullscreen =
                    main_style.contains(objc2_app_kit::NSWindowStyleMask::FullScreen);
                let user_tabbing_preference = Self::get_user_tabbing_preference()
                    .unwrap_or(UserTabbingPreference::InFullScreen);
                let should_add_as_tab = user_tabbing_preference == UserTabbingPreference::Always
                    || user_tabbing_preference == UserTabbingPreference::InFullScreen
                        && main_window_is_fullscreen;

                if should_add_as_tab {
                    let main_window_can_tab: bool = msg_send![
                        main_window,
                        respondsToSelector: sel!(addTabbedWindow:ordered:)
                    ];
                    let main_window_visible: bool = msg_send![main_window, isVisible];

                    if main_window_can_tab && main_window_visible {
                        let _: () = msg_send![
                            main_window,
                            addTabbedWindow: native_window,
                            ordered: 1isize // NSWindowAbove
                        ];

                        // Ensure the window is visible immediately after adding the tab, since the tab bar is updated with a new entry at this point.
                        // Note: Calling orderFront here can break fullscreen mode (makes fullscreen windows exit fullscreen), so only do this if the main window is not fullscreen.
                        if !main_window_is_fullscreen {
                            let _: () = msg_send![
                                native_window,
                                orderFront: ptr::null_mut::<AnyObject>()
                            ];
                        }
                    }
                }
            }

            if focus && show {
                let _: () =
                    msg_send![native_window, makeKeyAndOrderFront: ptr::null_mut::<AnyObject>()];
            } else if show {
                let _: () = msg_send![native_window, orderFront: ptr::null_mut::<AnyObject>()];
            }

            // Set the initial position of the window to the specified origin.
            // Although we already specified the position using `initWithContentRect_styleMask_backing_defer_screen_`,
            // the window position might be incorrect if the main screen (the screen that contains the window that has focus)
            //  is different from the primary screen.
            let _: () = msg_send![native_window, setFrameTopLeftPoint: window_rect.origin];
            {
                let mut window_state = window.0.lock();
                window_state.move_traffic_light();
                window_state.sheet_parent = sheet_parent;
            }

            drop(pool);

            window
        }
    }

    pub fn active_window() -> Option<AnyWindowHandle> {
        unsafe {
            let mtm = objc2::MainThreadMarker::new_unchecked();
            let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
            let main_window: *mut AnyObject = msg_send![&*app, mainWindow];
            if main_window.is_null() {
                return None;
            }

            let is_gpui_window: bool =
                msg_send![main_window, isKindOfClass: GPUIWindow::class()];
            if is_gpui_window {
                let window_ref: &GPUIWindow = &*(main_window as *const GPUIWindow);
                let handle = window_ref.ivars().get_state().lock().handle;
                Some(handle)
            } else {
                None
            }
        }
    }

    pub fn ordered_windows() -> Vec<AnyWindowHandle> {
        unsafe {
            let mtm = objc2::MainThreadMarker::new_unchecked();
            let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
            let windows: *mut AnyObject = msg_send![&*app, orderedWindows];
            let count: usize = msg_send![windows, count];

            let mut window_handles = Vec::new();
            for i in 0..count {
                let window: *mut AnyObject = msg_send![windows, objectAtIndex: i];
                let is_gpui_window: bool =
                    msg_send![window, isKindOfClass: GPUIWindow::class()];
                if is_gpui_window {
                    let window_ref: &GPUIWindow = &*(window as *const GPUIWindow);
                    let handle = window_ref.ivars().get_state().lock().handle;
                    window_handles.push(handle);
                }
            }

            window_handles
        }
    }

    pub fn get_user_tabbing_preference() -> Option<UserTabbingPreference> {
        let defaults = objc2_foundation::NSUserDefaults::standardUserDefaults();
        let domain = objc2_foundation::NSString::from_str("NSGlobalDomain");
        let key = objc2_foundation::NSString::from_str("AppleWindowTabbingMode");

        unsafe {
            let dict: *mut AnyObject = msg_send![&*defaults, persistentDomainForName: &*domain];
            let value: *mut AnyObject = if !dict.is_null() {
                msg_send![dict, objectForKey: &*key]
            } else {
                ptr::null_mut()
            };

            if value.is_null() {
                return Some(UserTabbingPreference::InFullScreen);
            }

            let ns_string: &objc2_foundation::NSString =
                &*(value as *const AnyObject as *const objc2_foundation::NSString);
            let value_str = ns_string.to_string();

            match value_str.as_str() {
                "manual" => Some(UserTabbingPreference::Never),
                "always" => Some(UserTabbingPreference::Always),
                _ => Some(UserTabbingPreference::InFullScreen),
            }
        }
    }
}
