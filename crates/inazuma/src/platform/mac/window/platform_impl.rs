use super::*;
use class_registration::GPUIWindow;
use objc2::{ClassType, DefinedClass};
use objc2_app_kit::{
    NSAlertStyle, NSAppearanceCustomization, NSAutoresizingMaskOptions, NSEvent,
    NSWindowOrderingMode,
};

/// Reinterpret a raw `*mut AnyObject` as `&NSWindow`.
///
/// # Safety
/// The caller must guarantee that `ptr` points to a valid, live `NSWindow`
/// (or subclass) instance and that no mutable alias exists for the duration
/// of the returned reference.
unsafe fn ns_window_ref<'a>(ptr: *mut AnyObject) -> &'a objc2_app_kit::NSWindow {
    unsafe { &*(ptr as *const objc2_app_kit::NSWindow) }
}

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
                    ns_window_ref(window).setContentSize(ns_size);
                }
            })
            .detach();
    }

    fn merge_all_windows(&self) {
        let native_window = self.0.lock().native_window;
        extern "C" fn merge_windows_async(context: *mut std::ffi::c_void) {
            unsafe {
                let native_window = context as *mut AnyObject;
                ns_window_ref(native_window).mergeAllWindows(None);
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
                let window = ns_window_ref(native_window);
                window.moveTabToNewWindow(None);
                window.makeKeyAndOrderFront(None);
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
            ns_window_ref(native_window).toggleTabOverview(None);
        }
    }

    fn set_tabbing_identifier(&self, tabbing_identifier: Option<String>) {
        let native_window = self.0.lock().native_window;
        unsafe {
            let mtm = objc2::MainThreadMarker::new_unchecked();
            let allows_automatic_window_tabbing = tabbing_identifier.is_some();
            objc2_app_kit::NSWindow::setAllowsAutomaticWindowTabbing(
                allows_automatic_window_tabbing,
                mtm,
            );

            let window = ns_window_ref(native_window);
            if let Some(tabbing_identifier) = tabbing_identifier {
                let tabbing_id =
                    objc2_foundation::NSString::from_str(tabbing_identifier.as_str());
                window.setTabbingIdentifier(&tabbing_id);
            } else {
                let empty = objc2_foundation::NSString::from_str("");
                window.setTabbingIdentifier(&empty);
            }
        }
    }

    fn scale_factor(&self) -> f32 {
        self.0.as_ref().lock().scale_factor()
    }

    fn appearance(&self) -> WindowAppearance {
        unsafe {
            let window = ns_window_ref(self.0.lock().native_window);
            let appearance = window.effectiveAppearance();
            super::super::window_appearance::window_appearance_from_native(&appearance)
        }
    }

    fn display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        unsafe {
            let window = ns_window_ref(self.0.lock().native_window);
            let screen = window.screen()?;
            let device_description = screen.deviceDescription();
            let screen_number_key = objc2_foundation::NSString::from_str("NSScreenNumber");
            let screen_number = device_description.objectForKey(&screen_number_key)?;
            let screen_number: &objc2_foundation::NSNumber =
                &*((&*screen_number as *const AnyObject) as *const objc2_foundation::NSNumber);
            let screen_number = screen_number.unsignedIntValue();

            Some(Rc::new(MacDisplay(screen_number)))
        }
    }

    fn mouse_position(&self) -> Point<Pixels> {
        let position = unsafe {
            ns_window_ref(self.0.lock().native_window).mouseLocationOutsideOfEventStream()
        };
        convert_mouse_position(position, self.content_size().height)
    }

    fn modifiers(&self) -> Modifiers {
        let modifiers = NSEvent::modifierFlags_class();

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

    fn capslock(&self) -> Capslock {
        let modifiers = NSEvent::modifierFlags_class();

        Capslock {
            on: modifiers.contains(objc2_app_kit::NSEventModifierFlags::CapsLock),
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
            let mtm = objc2::MainThreadMarker::new_unchecked();
            let alert = objc2_app_kit::NSAlert::new(mtm);
            let alert_style = match level {
                PromptLevel::Info => NSAlertStyle::Informational,
                PromptLevel::Warning => NSAlertStyle::Warning,
                PromptLevel::Critical => NSAlertStyle::Critical,
            };
            alert.setAlertStyle(alert_style);
            let msg_str = objc2_foundation::NSString::from_str(msg);
            alert.setMessageText(&msg_str);
            if let Some(detail) = detail {
                let detail_str = objc2_foundation::NSString::from_str(detail);
                alert.setInformativeText(&detail_str);
            }

            for (ix, answer) in answers
                .iter()
                .enumerate()
                .filter(|&(ix, _)| Some(ix) != latest_non_cancel_label.map(|(ix, _)| ix))
            {
                let label_str = objc2_foundation::NSString::from_str(answer.label());
                let button = alert.addButtonWithTitle(&label_str);
                button.setTag(ix as isize);

                if answer.is_cancel() {
                    // Bind Escape Key to Cancel Button
                    if let Some(key) =
                        std::char::from_u32(super::super::events::ESCAPE_KEY as u32)
                    {
                        let key_str =
                            objc2_foundation::NSString::from_str(&key.to_string());
                        button.setKeyEquivalent(&key_str);
                    }
                }
            }
            if let Some((ix, answer)) = latest_non_cancel_label {
                let label_str = objc2_foundation::NSString::from_str(answer.label());
                let button = alert.addButtonWithTitle(&label_str);
                button.setTag(ix as isize);
            }

            let (done_tx, done_rx) = oneshot::channel();
            let done_tx = Cell::new(Some(done_tx));
            let block = RcBlock::new(move |answer: isize| {
                if let Some(done_tx) = done_tx.take() {
                    let _ = done_tx.send(answer.try_into().unwrap());
                }
            });
            let native_window = self.0.lock().native_window;
            let executor = self.0.lock().foreground_executor.clone();
            executor
                .spawn(async move {
                    let window = ns_window_ref(native_window);
                    alert.beginSheetModalForWindow_completionHandler(window, Some(&block));
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
                    ns_window_ref(window).makeKeyAndOrderFront(None);
                }
            })
            .detach();
    }

    fn is_active(&self) -> bool {
        unsafe { ns_window_ref(self.0.lock().native_window).isKeyWindow() }
    }

    // is_hovered is unused on macOS. See Window::is_window_hovered.
    fn is_hovered(&self) -> bool {
        false
    }

    fn set_title(&mut self, title: &str) {
        unsafe {
            let mtm = objc2::MainThreadMarker::new_unchecked();
            let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
            let native_window = self.0.lock().native_window;
            let window = ns_window_ref(native_window);
            let title_str = objc2_foundation::NSString::from_str(title);
            app.changeWindowsItem_title_filename(window, &title_str, false);
            window.setTitle(&title_str);
            self.0.lock().move_traffic_light();
        }
    }

    fn get_title(&self) -> String {
        unsafe {
            let window = ns_window_ref(self.0.lock().native_window);
            window.title().to_string()
        }
    }

    fn set_app_id(&mut self, _app_id: &str) {}

    fn set_background_appearance(&self, background_appearance: WindowBackgroundAppearance) {
        let mut this = self.0.as_ref().lock();
        this.background_appearance = background_appearance;

        let opaque = background_appearance == WindowBackgroundAppearance::Opaque;
        this.renderer.update_transparency(!opaque);

        unsafe {
            let window = ns_window_ref(this.native_window);
            window.setOpaque(opaque);
            let background_color = if opaque {
                objc2_app_kit::NSColor::colorWithSRGBRed_green_blue_alpha(
                    0.0, 0.0, 0.0, 1.0,
                )
            } else {
                // Not using `+[NSColor clearColor]` to avoid broken shadow.
                objc2_app_kit::NSColor::colorWithSRGBRed_green_blue_alpha(
                    0.0, 0.0, 0.0, 0.0001,
                )
            };
            window.setBackgroundColor(Some(&background_color));

            // Check AppKit version for blur handling.
            // `instancesRespondToSelector:` is a runtime introspection method with no
            // typed binding in objc2-app-kit, so msg_send! is required.
            let use_legacy_blur = {
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

                let window_number = window.windowNumber();
                CGSSetWindowBackgroundBlurRadius(CGSMainConnectionID(), window_number, blur_radius);
            } else {
                // On newer macOS `NSVisualEffectView` manages the effect layer directly. Using it
                // could have a better performance (it downsamples the backdrop) and more control
                // over the effect layer.
                if background_appearance != WindowBackgroundAppearance::Blurred {
                    if let Some(blur_view) = this.blurred_view {
                        let blur_view_ref: &objc2_app_kit::NSView =
                            &*(blur_view as *const objc2_app_kit::NSView);
                        blur_view_ref.removeFromSuperview();
                        this.blurred_view = None;
                    }
                } else if this.blurred_view.is_none() {
                    let content_view = window.contentView();
                    if let Some(content_view) = &content_view {
                        let frame = content_view.bounds();
                        // BlurredView is a custom subclass of NSVisualEffectView registered
                        // via objc2's define_class! macro. There are no typed alloc/initWithFrame
                        // bindings for this custom class, so msg_send! is required here.
                        let blur_view: *mut AnyObject =
                            msg_send![class_registration::BlurredView::class(), alloc];
                        let blur_view: *mut AnyObject =
                            msg_send![blur_view, initWithFrame: frame];
                        let blur_view_ref: &objc2_app_kit::NSView =
                            &*(blur_view as *const objc2_app_kit::NSView);
                        let autoresizing = NSAutoresizingMaskOptions::ViewWidthSizable
                            | NSAutoresizingMaskOptions::ViewHeightSizable;
                        blur_view_ref.setAutoresizingMask(autoresizing);

                        content_view.addSubview_positioned_relativeTo(
                            blur_view_ref,
                            NSWindowOrderingMode::Below,
                            None,
                        );
                        let blur_view_autoreleased: *mut AnyObject =
                            msg_send![blur_view, autorelease];
                        this.blurred_view = Some(blur_view_autoreleased);
                    }
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
            let window = ns_window_ref(self.0.lock().native_window);
            window.setDocumentEdited(edited);
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
                    app.orderFrontCharacterPalette(Some(&*(window as *const AnyObject)));
                }
            })
            .detach();
    }

    fn minimize(&self) {
        let window = self.0.lock().native_window;
        unsafe {
            ns_window_ref(window).miniaturize(None);
        }
    }

    fn zoom(&self) {
        let this = self.0.lock();
        let window = this.native_window;
        this.foreground_executor
            .spawn(async move {
                unsafe {
                    ns_window_ref(window).zoom(None);
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
                    ns_window_ref(window).toggleFullScreen(None);
                }
            })
            .detach();
    }

    fn is_fullscreen(&self) -> bool {
        let this = self.0.lock();
        unsafe {
            let style_mask = ns_window_ref(this.native_window).styleMask();
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
            let window = ns_window_ref(self.0.lock().native_window);
            let windows = window.tabbedWindows()?;

            let mut result = Vec::new();
            for i in 0..windows.count() {
                let tabbed_window = windows.objectAtIndex(i);
                let tabbed_ptr = &*tabbed_window as *const objc2_app_kit::NSWindow
                    as *const GPUIWindow;
                let is_gpui_window: bool =
                    msg_send![&*tabbed_window, isKindOfClass: GPUIWindow::class()];
                if is_gpui_window {
                    let window_ref: &GPUIWindow = &*tabbed_ptr;
                    let handle = window_ref.ivars().get_state().lock().handle;
                    let title = SharedString::from(tabbed_window.title().to_string());
                    result.push(SystemWindowTab::new(title, handle));
                }
            }

            Some(result)
        }
    }

    fn tab_bar_visible(&self) -> bool {
        unsafe {
            let window = ns_window_ref(self.0.lock().native_window);
            match window.tabGroup() {
                Some(tab_group) => tab_group.isTabBarVisible(),
                None => false,
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
                    let mtm = objc2::MainThreadMarker::new_unchecked();
                    if let Some(input_context) =
                        objc2_app_kit::NSTextInputContext::currentInputContext(mtm)
                    {
                        input_context.invalidateCharacterCoordinates();
                    }
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
                    let action_str = defaults
                        .persistentDomainForName(&domain)
                        .and_then(|dict| dict.objectForKey(&key))
                        .map(|obj| {
                            let ns_string: &objc2_foundation::NSString =
                                &*((&*obj as *const AnyObject)
                                    as *const objc2_foundation::NSString);
                            ns_string.to_string()
                        })
                        .unwrap_or_default();

                    let window = ns_window_ref(window);
                    match action_str.as_str() {
                        "None" => {
                            // "Do Nothing" selected, so do no action
                        }
                        "Minimize" => {
                            window.miniaturize(None);
                        }
                        _ => {
                            // Handles "Maximize", "Fill", and any other/unknown action
                            window.zoom(None);
                        }
                    }
                }
            })
            .detach();
    }

    fn start_window_move(&self) {
        let this = self.0.lock();
        let window_ptr = this.native_window;

        unsafe {
            let mtm = objc2::MainThreadMarker::new_unchecked();
            let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
            if let Some(event) = app.currentEvent() {
                ns_window_ref(window_ptr).performWindowDragWithEvent(&event);
            }
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
