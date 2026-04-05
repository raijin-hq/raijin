use super::*;
use class_registration::GPUIWindow;
use objc2::{ClassType, DefinedClass};

impl PlatformWindow for MacWindow {
    fn bounds(&self) -> Bounds<Pixels> {
        self.0.as_ref().lock().bounds()
    }

    fn window_bounds(&self) -> WindowBounds {
        self.0.as_ref().lock().window_bounds()
    }

    fn is_maximized(&self) -> bool {
        self.0.as_ref().lock().is_maximized()
    }

    fn content_size(&self) -> Size<Pixels> {
        self.0.as_ref().lock().content_size()
    }

    fn resize(&mut self, size: Size<Pixels>) {
        let this = self.0.lock();
        let window = this.native_window;
        this.foreground_executor
            .spawn(async move {
                unsafe {
                    let ns_size = NSSize {
                        width: size.width.as_f32() as f64,
                        height: size.height.as_f32() as f64,
                    };
                    let _: () = msg_send![window, setContentSize: ns_size];
                }
            })
            .detach();
    }

    fn merge_all_windows(&self) {
        let native_window = self.0.lock().native_window;
        extern "C" fn merge_windows_async(context: *mut std::ffi::c_void) {
            unsafe {
                let native_window = context as *mut AnyObject;
                let _: () = msg_send![native_window, mergeAllWindows: ptr::null_mut::<AnyObject>()];
            }
        }

        unsafe {
            DispatchQueue::main()
                .exec_async_f(native_window as *mut std::ffi::c_void, merge_windows_async);
        }
    }

    fn move_tab_to_new_window(&self) {
        let native_window = self.0.lock().native_window;
        extern "C" fn move_tab_async(context: *mut std::ffi::c_void) {
            unsafe {
                let native_window = context as *mut AnyObject;
                let _: () =
                    msg_send![native_window, moveTabToNewWindow: ptr::null_mut::<AnyObject>()];
                let _: () =
                    msg_send![native_window, makeKeyAndOrderFront: ptr::null_mut::<AnyObject>()];
            }
        }

        unsafe {
            DispatchQueue::main()
                .exec_async_f(native_window as *mut std::ffi::c_void, move_tab_async);
        }
    }

    fn toggle_window_tab_overview(&self) {
        let native_window = self.0.lock().native_window;
        unsafe {
            let _: () = msg_send![native_window, toggleTabOverview: ptr::null_mut::<AnyObject>()];
        }
    }

    fn set_tabbing_identifier(&self, tabbing_identifier: Option<String>) {
        let native_window = self.0.lock().native_window;
        unsafe {
            let allows_automatic_window_tabbing = tabbing_identifier.is_some();
            let _: () = msg_send![
                objc2_app_kit::NSWindow::class(),
                setAllowsAutomaticWindowTabbing: allows_automatic_window_tabbing
            ];

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
    }

    fn scale_factor(&self) -> f32 {
        self.0.as_ref().lock().scale_factor()
    }

    fn appearance(&self) -> WindowAppearance {
        unsafe {
            let appearance: *mut AnyObject =
                msg_send![self.0.lock().native_window, effectiveAppearance];
            let appearance: &objc2_app_kit::NSAppearance =
                &*(appearance as *const objc2_app_kit::NSAppearance);
            super::super::window_appearance::window_appearance_from_native(appearance)
        }
    }

    fn display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        unsafe {
            let screen: *mut AnyObject = msg_send![self.0.lock().native_window, screen];
            if screen.is_null() {
                return None;
            }
            let device_description: *mut AnyObject = msg_send![screen, deviceDescription];
            let screen_number_key = objc2_foundation::NSString::from_str("NSScreenNumber");
            let screen_number: *mut AnyObject =
                msg_send![device_description, objectForKey: &*screen_number_key];

            let screen_number: u32 = msg_send![screen_number, unsignedIntValue];

            Some(Rc::new(MacDisplay(screen_number)))
        }
    }

    fn mouse_position(&self) -> Point<Pixels> {
        let position: NSPoint = unsafe {
            msg_send![
                self.0.lock().native_window,
                mouseLocationOutsideOfEventStream
            ]
        };
        convert_mouse_position(position, self.content_size().height)
    }

    fn modifiers(&self) -> Modifiers {
        unsafe {
            let modifiers: objc2_app_kit::NSEventModifierFlags =
                msg_send![objc2_app_kit::NSEvent::class(), modifierFlags];

            let control = modifiers.contains(objc2_app_kit::NSEventModifierFlags::Control);
            let alt = modifiers.contains(objc2_app_kit::NSEventModifierFlags::Option);
            let shift = modifiers.contains(objc2_app_kit::NSEventModifierFlags::Shift);
            let command = modifiers.contains(objc2_app_kit::NSEventModifierFlags::Command);
            let function = modifiers.contains(objc2_app_kit::NSEventModifierFlags::Function);

            Modifiers {
                control,
                alt,
                shift,
                platform: command,
                function,
            }
        }
    }

    fn capslock(&self) -> Capslock {
        unsafe {
            let modifiers: objc2_app_kit::NSEventModifierFlags =
                msg_send![objc2_app_kit::NSEvent::class(), modifierFlags];

            Capslock {
                on: modifiers.contains(objc2_app_kit::NSEventModifierFlags::CapsLock),
            }
        }
    }

    fn set_input_handler(&mut self, input_handler: PlatformInputHandler) {
        self.0.as_ref().lock().input_handler = Some(input_handler);
    }

    fn take_input_handler(&mut self) -> Option<PlatformInputHandler> {
        self.0.as_ref().lock().input_handler.take()
    }

    fn prompt(
        &self,
        level: PromptLevel,
        msg: &str,
        detail: Option<&str>,
        answers: &[PromptButton],
    ) -> Option<oneshot::Receiver<usize>> {
        // macOs applies overrides to modal window buttons after they are added.
        // Two most important for this logic are:
        // * Buttons with "Cancel" title will be displayed as the last buttons in the modal
        // * Last button added to the modal via `addButtonWithTitle` stays focused
        // * Focused buttons react on "space"/" " keypresses
        // * Usage of `keyEquivalent`, `makeFirstResponder` or `setInitialFirstResponder` does not change the focus
        //
        // See also https://developer.apple.com/documentation/appkit/nsalert/1524532-addbuttonwithtitle#discussion
        // ```
        // By default, the first button has a key equivalent of Return,
        // any button with a title of "Cancel" has a key equivalent of Escape,
        // and any button with the title "Don't Save" has a key equivalent of Command-D (but only if it's not the first button).
        // ```
        //
        // To avoid situations when the last element added is "Cancel" and it gets the focus
        // (hence stealing both ESC and Space shortcuts), we find and add one non-Cancel button
        // last, so it gets focus and a Space shortcut.
        // This way, "Save this file? Yes/No/Cancel"-ish modals will get all three buttons mapped with a key.
        let latest_non_cancel_label = answers
            .iter()
            .enumerate()
            .rev()
            .find(|(_, label)| !label.is_cancel())
            .filter(|&(label_index, _)| label_index > 0);

        unsafe {
            let alert: *mut AnyObject =
                msg_send![objc2_app_kit::NSAlert::class(), alloc];
            let alert: *mut AnyObject = msg_send![alert, init];
            let alert_style: isize = match level {
                PromptLevel::Info => 1,
                PromptLevel::Warning => 0,
                PromptLevel::Critical => 2,
            };
            let _: () = msg_send![alert, setAlertStyle: alert_style];
            let msg_str = objc2_foundation::NSString::from_str(msg);
            let _: () = msg_send![alert, setMessageText: &*msg_str];
            if let Some(detail) = detail {
                let detail_str = objc2_foundation::NSString::from_str(detail);
                let _: () = msg_send![alert, setInformativeText: &*detail_str];
            }

            for (ix, answer) in answers
                .iter()
                .enumerate()
                .filter(|&(ix, _)| Some(ix) != latest_non_cancel_label.map(|(ix, _)| ix))
            {
                let label_str = objc2_foundation::NSString::from_str(answer.label());
                let button: *mut AnyObject =
                    msg_send![alert, addButtonWithTitle: &*label_str];
                let _: () = msg_send![button, setTag: ix as isize];

                if answer.is_cancel() {
                    // Bind Escape Key to Cancel Button
                    if let Some(key) =
                        std::char::from_u32(super::super::events::ESCAPE_KEY as u32)
                    {
                        let key_str =
                            objc2_foundation::NSString::from_str(&key.to_string());
                        let _: () = msg_send![button, setKeyEquivalent: &*key_str];
                    }
                }
            }
            if let Some((ix, answer)) = latest_non_cancel_label {
                let label_str = objc2_foundation::NSString::from_str(answer.label());
                let button: *mut AnyObject =
                    msg_send![alert, addButtonWithTitle: &*label_str];
                let _: () = msg_send![button, setTag: ix as isize];
            }

            let (done_tx, done_rx) = oneshot::channel();
            let done_tx = Cell::new(Some(done_tx));
            let block = RcBlock::new(move |answer: isize| {
                let _: () = msg_send![alert, release];
                if let Some(done_tx) = done_tx.take() {
                    let _ = done_tx.send(answer.try_into().unwrap());
                }
            });
            let native_window = self.0.lock().native_window;
            let executor = self.0.lock().foreground_executor.clone();
            executor
                .spawn(async move {
                    let _: () = msg_send![
                        alert,
                        beginSheetModalForWindow: native_window,
                        completionHandler: &*block
                    ];
                })
                .detach();

            Some(done_rx)
        }
    }

    fn activate(&self) {
        let window = self.0.lock().native_window;
        let executor = self.0.lock().foreground_executor.clone();
        executor
            .spawn(async move {
                unsafe {
                    let _: () =
                        msg_send![window, makeKeyAndOrderFront: ptr::null_mut::<AnyObject>()];
                }
            })
            .detach();
    }

    fn is_active(&self) -> bool {
        unsafe { msg_send![self.0.lock().native_window, isKeyWindow] }
    }

    // is_hovered is unused on macOS. See Window::is_window_hovered.
    fn is_hovered(&self) -> bool {
        false
    }

    fn set_title(&mut self, title: &str) {
        unsafe {
            let mtm = objc2::MainThreadMarker::new_unchecked();
            let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
            let window = self.0.lock().native_window;
            let title_str = objc2_foundation::NSString::from_str(title);
            let _: () = msg_send![
                &*app,
                changeWindowsItem: window,
                title: &*title_str,
                filename: false
            ];
            let _: () = msg_send![window, setTitle: &*title_str];
            self.0.lock().move_traffic_light();
        }
    }

    fn get_title(&self) -> String {
        unsafe {
            let title: *mut AnyObject = msg_send![self.0.lock().native_window, title];
            if title.is_null() {
                "".to_string()
            } else {
                let ns_string: &objc2_foundation::NSString =
                    &*(title as *const AnyObject as *const objc2_foundation::NSString);
                ns_string.to_string()
            }
        }
    }

    fn set_app_id(&mut self, _app_id: &str) {}

    fn set_background_appearance(&self, background_appearance: WindowBackgroundAppearance) {
        let mut this = self.0.as_ref().lock();
        this.background_appearance = background_appearance;

        let opaque = background_appearance == WindowBackgroundAppearance::Opaque;
        this.renderer.update_transparency(!opaque);

        unsafe {
            let _: () = msg_send![this.native_window, setOpaque: opaque];
            let background_color: *mut AnyObject = if opaque {
                msg_send![
                    objc2_app_kit::NSColor::class(),
                    colorWithSRGBRed: 0f64,
                    green: 0f64,
                    blue: 0f64,
                    alpha: 1f64
                ]
            } else {
                // Not using `+[NSColor clearColor]` to avoid broken shadow.
                msg_send![
                    objc2_app_kit::NSColor::class(),
                    colorWithSRGBRed: 0f64,
                    green: 0f64,
                    blue: 0f64,
                    alpha: 0.0001f64
                ]
            };
            let _: () = msg_send![this.native_window, setBackgroundColor: background_color];

            // Check AppKit version for blur handling
            let use_legacy_blur = {
                // Fall back to a simpler check via the private API
                let blur_responds: bool = msg_send![
                    objc2_app_kit::NSVisualEffectView::class(),
                    instancesRespondToSelector: sel!(_updateProxyLayer)
                ];
                !blur_responds
            };

            if use_legacy_blur {
                // Whether `-[NSVisualEffectView respondsToSelector:@selector(_updateProxyLayer)]`.
                // On macOS Catalina/Big Sur `NSVisualEffectView` doesn't own concrete sublayers
                // but uses a `CAProxyLayer`. Use the legacy WindowServer API.
                let blur_radius = if background_appearance == WindowBackgroundAppearance::Blurred {
                    80
                } else {
                    0
                };

                let window_number: isize = msg_send![this.native_window, windowNumber];
                CGSSetWindowBackgroundBlurRadius(CGSMainConnectionID(), window_number, blur_radius);
            } else {
                // On newer macOS `NSVisualEffectView` manages the effect layer directly. Using it
                // could have a better performance (it downsamples the backdrop) and more control
                // over the effect layer.
                if background_appearance != WindowBackgroundAppearance::Blurred {
                    if let Some(blur_view) = this.blurred_view {
                        let _: () = msg_send![blur_view, removeFromSuperview];
                        this.blurred_view = None;
                    }
                } else if this.blurred_view.is_none() {
                    let content_view: *mut AnyObject =
                        msg_send![this.native_window, contentView];
                    let frame: NSRect = msg_send![content_view, bounds];
                    let blur_view: *mut AnyObject =
                        msg_send![class_registration::BlurredView::class(), alloc];
                    let blur_view: *mut AnyObject =
                        msg_send![blur_view, initWithFrame: frame];
                    let autoresizing: usize = (1 << 4) | (1 << 1); // NSViewWidthSizable | NSViewHeightSizable
                    let _: () = msg_send![blur_view, setAutoresizingMask: autoresizing];

                    let _: () = msg_send![
                        content_view,
                        addSubview: blur_view,
                        positioned: -1isize, // NSWindowBelow
                        relativeTo: ptr::null_mut::<AnyObject>()
                    ];
                    let blur_view_autoreleased: *mut AnyObject =
                        msg_send![blur_view, autorelease];
                    this.blurred_view = Some(blur_view_autoreleased);
                }
            }
        }
    }

    fn background_appearance(&self) -> WindowBackgroundAppearance {
        self.0.as_ref().lock().background_appearance
    }

    fn is_subpixel_rendering_supported(&self) -> bool {
        false
    }

    fn set_edited(&mut self, edited: bool) {
        unsafe {
            let window = self.0.lock().native_window;
            let _: () = msg_send![window, setDocumentEdited: edited];
        }

        // Changing the document edited state resets the traffic light position,
        // so we have to move it again.
        self.0.lock().move_traffic_light();
    }

    fn show_character_palette(&self) {
        let this = self.0.lock();
        let window = this.native_window;
        this.foreground_executor
            .spawn(async move {
                unsafe {
                    let mtm = objc2::MainThreadMarker::new_unchecked();
                    let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
                    let _: () = msg_send![&*app, orderFrontCharacterPalette: window];
                }
            })
            .detach();
    }

    fn minimize(&self) {
        let window = self.0.lock().native_window;
        unsafe {
            let _: () = msg_send![window, miniaturize: ptr::null_mut::<AnyObject>()];
        }
    }

    fn zoom(&self) {
        let this = self.0.lock();
        let window = this.native_window;
        this.foreground_executor
            .spawn(async move {
                unsafe {
                    let _: () = msg_send![window, zoom: ptr::null_mut::<AnyObject>()];
                }
            })
            .detach();
    }

    fn toggle_fullscreen(&self) {
        let this = self.0.lock();
        let window = this.native_window;
        this.foreground_executor
            .spawn(async move {
                unsafe {
                    let _: () =
                        msg_send![window, toggleFullScreen: ptr::null_mut::<AnyObject>()];
                }
            })
            .detach();
    }

    fn is_fullscreen(&self) -> bool {
        let this = self.0.lock();
        let window = this.native_window;

        unsafe {
            let style_mask: objc2_app_kit::NSWindowStyleMask = msg_send![window, styleMask];
            style_mask.contains(objc2_app_kit::NSWindowStyleMask::FullScreen)
        }
    }

    fn on_request_frame(&self, callback: Box<dyn FnMut(RequestFrameOptions)>) {
        self.0.as_ref().lock().request_frame_callback = Some(callback);
    }

    fn on_input(&self, callback: Box<dyn FnMut(PlatformInput) -> inazuma::DispatchEventResult>) {
        self.0.as_ref().lock().event_callback = Some(callback);
    }

    fn on_active_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        self.0.as_ref().lock().activate_callback = Some(callback);
    }

    fn on_hover_status_change(&self, _: Box<dyn FnMut(bool)>) {}

    fn on_resize(&self, callback: Box<dyn FnMut(Size<Pixels>, f32)>) {
        self.0.as_ref().lock().resize_callback = Some(callback);
    }

    fn on_moved(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().moved_callback = Some(callback);
    }

    fn on_should_close(&self, callback: Box<dyn FnMut() -> bool>) {
        self.0.as_ref().lock().should_close_callback = Some(callback);
    }

    fn on_close(&self, callback: Box<dyn FnOnce()>) {
        self.0.as_ref().lock().close_callback = Some(callback);
    }

    fn on_hit_test_window_control(&self, _callback: Box<dyn FnMut() -> Option<WindowControlArea>>) {
    }

    fn on_appearance_changed(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().appearance_changed_callback = Some(callback);
    }

    fn tabbed_windows(&self) -> Option<Vec<SystemWindowTab>> {
        unsafe {
            let windows: *mut AnyObject =
                msg_send![self.0.lock().native_window, tabbedWindows];
            if windows.is_null() {
                return None;
            }

            let count: usize = msg_send![windows, count];
            let mut result = Vec::new();
            for i in 0..count {
                let window: *mut AnyObject = msg_send![windows, objectAtIndex: i];
                let is_gpui_window: bool =
                    msg_send![window, isKindOfClass: GPUIWindow::class()];
                if is_gpui_window {
                    let window_ref: &GPUIWindow = &*(window as *const GPUIWindow);
                    let handle = window_ref.ivars().get_state().lock().handle;
                    let title: *mut AnyObject = msg_send![window, title];
                    let title_str = if title.is_null() {
                        "".to_string()
                    } else {
                        let ns_string: &objc2_foundation::NSString =
                            &*(title as *const AnyObject as *const objc2_foundation::NSString);
                        ns_string.to_string()
                    };
                    let title = SharedString::from(title_str);

                    result.push(SystemWindowTab::new(title, handle));
                }
            }

            Some(result)
        }
    }

    fn tab_bar_visible(&self) -> bool {
        unsafe {
            let tab_group: *mut AnyObject =
                msg_send![self.0.lock().native_window, tabGroup];
            if tab_group.is_null() {
                false
            } else {
                msg_send![tab_group, isTabBarVisible]
            }
        }
    }

    fn on_move_tab_to_new_window(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().move_tab_to_new_window_callback = Some(callback);
    }

    fn on_merge_all_windows(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().merge_all_windows_callback = Some(callback);
    }

    fn on_select_next_tab(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().select_next_tab_callback = Some(callback);
    }

    fn on_select_previous_tab(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().select_previous_tab_callback = Some(callback);
    }

    fn on_toggle_tab_bar(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().toggle_tab_bar_callback = Some(callback);
    }

    fn draw(&self, scene: &inazuma::Scene) {
        let mut this = self.0.lock();
        this.renderer.draw(scene);
    }

    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        self.0.lock().renderer.sprite_atlas().clone()
    }

    fn gpu_specs(&self) -> Option<inazuma::GpuSpecs> {
        None
    }

    fn update_ime_position(&self, _bounds: Bounds<Pixels>) {
        let executor = self.0.lock().foreground_executor.clone();
        executor
            .spawn(async move {
                unsafe {
                    let input_context: *mut AnyObject = msg_send![
                        objc2_app_kit::NSTextInputContext::class(),
                        currentInputContext
                    ];
                    if input_context.is_null() {
                        return;
                    }
                    let _: () = msg_send![input_context, invalidateCharacterCoordinates];
                }
            })
            .detach()
    }

    fn titlebar_double_click(&self) {
        let this = self.0.lock();
        let window = this.native_window;
        this.foreground_executor
            .spawn(async move {
                let defaults = objc2_foundation::NSUserDefaults::standardUserDefaults();
                let domain = objc2_foundation::NSString::from_str("NSGlobalDomain");
                let key = objc2_foundation::NSString::from_str("AppleActionOnDoubleClick");

                unsafe {
                    let dict: *mut AnyObject =
                        msg_send![&*defaults, persistentDomainForName: &*domain];
                    let action: *mut AnyObject = if !dict.is_null() {
                        msg_send![dict, objectForKey: &*key]
                    } else {
                        ptr::null_mut()
                    };

                    let action_str = if !action.is_null() {
                        let ns_string: &objc2_foundation::NSString =
                            &*(action as *const AnyObject as *const objc2_foundation::NSString);
                        ns_string.to_string()
                    } else {
                        String::new()
                    };

                    match action_str.as_str() {
                        "None" => {
                            // "Do Nothing" selected, so do no action
                        }
                        "Minimize" => {
                            let _: () =
                                msg_send![window, miniaturize: ptr::null_mut::<AnyObject>()];
                        }
                        "Maximize" => {
                            let _: () = msg_send![window, zoom: ptr::null_mut::<AnyObject>()];
                        }
                        "Fill" => {
                            // There is no documented API for "Fill" action, so we'll just zoom the window
                            let _: () = msg_send![window, zoom: ptr::null_mut::<AnyObject>()];
                        }
                        _ => {
                            let _: () = msg_send![window, zoom: ptr::null_mut::<AnyObject>()];
                        }
                    }
                }
            })
            .detach();
    }

    fn start_window_move(&self) {
        let this = self.0.lock();
        let window = this.native_window;

        unsafe {
            let mtm = objc2::MainThreadMarker::new_unchecked();
            let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
            let event: *mut AnyObject = msg_send![&*app, currentEvent];
            let _: () = msg_send![window, performWindowDragWithEvent: event];
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    fn render_to_image(&self, scene: &inazuma::Scene) -> Result<RgbaImage> {
        let mut this = self.0.lock();
        this.renderer.render_to_image(scene)
    }
}

impl rwh::HasWindowHandle for MacWindow {
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        // SAFETY: The AppKitWindowHandle is a wrapper around a pointer to an NSView
        unsafe {
            Ok(rwh::WindowHandle::borrow_raw(rwh::RawWindowHandle::AppKit(
                rwh::AppKitWindowHandle::new(self.0.lock().native_view.cast()),
            )))
        }
    }
}

impl rwh::HasDisplayHandle for MacWindow {
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, rwh::HandleError> {
        // SAFETY: This is a no-op on macOS
        unsafe {
            Ok(rwh::DisplayHandle::borrow_raw(
                rwh::AppKitDisplayHandle::new().into(),
            ))
        }
    }
}
