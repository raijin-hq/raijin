use super::*;

pub(super) struct MacWindowState {
    pub(super) handle: AnyWindowHandle,
    pub(super) foreground_executor: ForegroundExecutor,
    pub(super) background_executor: BackgroundExecutor,
    pub(super) native_window: *mut AnyObject,
    pub(super) native_view: NonNull<AnyObject>,
    pub(super) blurred_view: Option<*mut AnyObject>,
    pub(super) background_appearance: WindowBackgroundAppearance,
    pub(super) display_link: Option<DisplayLink>,
    pub(super) renderer: renderer::Renderer,
    pub(super) request_frame_callback: Option<Box<dyn FnMut(RequestFrameOptions)>>,
    pub(super) event_callback: Option<Box<dyn FnMut(PlatformInput) -> inazuma::DispatchEventResult>>,
    pub(super) activate_callback: Option<Box<dyn FnMut(bool)>>,
    pub(super) resize_callback: Option<Box<dyn FnMut(Size<Pixels>, f32)>>,
    pub(super) moved_callback: Option<Box<dyn FnMut()>>,
    pub(super) should_close_callback: Option<Box<dyn FnMut() -> bool>>,
    pub(super) close_callback: Option<Box<dyn FnOnce()>>,
    pub(super) appearance_changed_callback: Option<Box<dyn FnMut()>>,
    pub(super) input_handler: Option<PlatformInputHandler>,
    pub(super) last_key_equivalent: Option<KeyDownEvent>,
    pub(super) synthetic_drag_counter: usize,
    pub(super) traffic_light_position: Option<Point<Pixels>>,
    pub(super) transparent_titlebar: bool,
    pub(super) previous_modifiers_changed_event: Option<PlatformInput>,
    pub(super) keystroke_for_do_command: Option<Keystroke>,
    pub(super) do_command_handled: Option<bool>,
    pub(super) external_files_dragged: bool,
    // Whether the next left-mouse click is also the focusing click.
    pub(super) first_mouse: bool,
    pub(super) fullscreen_restore_bounds: Bounds<Pixels>,
    pub(super) move_tab_to_new_window_callback: Option<Box<dyn FnMut()>>,
    pub(super) merge_all_windows_callback: Option<Box<dyn FnMut()>>,
    pub(super) select_next_tab_callback: Option<Box<dyn FnMut()>>,
    pub(super) select_previous_tab_callback: Option<Box<dyn FnMut()>>,
    pub(super) toggle_tab_bar_callback: Option<Box<dyn FnMut()>>,
    pub(super) activated_least_once: bool,
    // The parent window if this window is a sheet (Dialog kind)
    pub(super) sheet_parent: Option<*mut AnyObject>,
}

impl MacWindowState {
    pub(super) fn move_traffic_light(&self) {
        if let Some(traffic_light_position) = self.traffic_light_position {
            if self.is_fullscreen() {
                // Moving traffic lights while fullscreen doesn't work,
                // see https://github.com/zed-industries/zed/issues/4712
                return;
            }

            let titlebar_height = self.titlebar_height();

            unsafe {
                let close_button: *mut AnyObject = msg_send![
                    self.native_window,
                    standardWindowButton: 7isize // NSWindowButton::NSWindowCloseButton
                ];
                let min_button: *mut AnyObject = msg_send![
                    self.native_window,
                    standardWindowButton: 8isize // NSWindowButton::NSWindowMiniaturizeButton
                ];
                let zoom_button: *mut AnyObject = msg_send![
                    self.native_window,
                    standardWindowButton: 9isize // NSWindowButton::NSWindowZoomButton
                ];

                if close_button.is_null() || min_button.is_null() || zoom_button.is_null() {
                    return;
                }

                let mut close_button_frame: NSRect = msg_send![close_button, frame];
                let mut min_button_frame: NSRect = msg_send![min_button, frame];
                let mut zoom_button_frame: NSRect = msg_send![zoom_button, frame];
                let mut origin = point(
                    traffic_light_position.x,
                    titlebar_height
                        - traffic_light_position.y
                        - px(close_button_frame.size.height as f32),
                );
                let button_spacing =
                    px((min_button_frame.origin.x - close_button_frame.origin.x) as f32);

                close_button_frame.origin = NSPoint::new(origin.x.into(), origin.y.into());
                let _: () = msg_send![close_button, setFrame: close_button_frame];
                origin.x += button_spacing;

                min_button_frame.origin = NSPoint::new(origin.x.into(), origin.y.into());
                let _: () = msg_send![min_button, setFrame: min_button_frame];
                origin.x += button_spacing;

                zoom_button_frame.origin = NSPoint::new(origin.x.into(), origin.y.into());
                let _: () = msg_send![zoom_button, setFrame: zoom_button_frame];
            }
        }
    }

    pub(super) fn start_display_link(&mut self) {
        self.stop_display_link();
        unsafe {
            let occlusion_state: objc2_app_kit::NSWindowOcclusionState =
                msg_send![self.native_window, occlusionState];
            if !occlusion_state.contains(
                objc2_app_kit::NSWindowOcclusionState::Visible,
            ) {
                return;
            }
        }
        let screen: *mut AnyObject = unsafe { msg_send![self.native_window, screen] };
        let display_id = unsafe { display_id_for_screen(screen) };
        if let Some(mut display_link) =
            DisplayLink::new(display_id, self.native_view.as_ptr() as *mut c_void, step).log_err()
        {
            display_link.start().log_err();
            self.display_link = Some(display_link);
        }
    }

    pub(super) fn stop_display_link(&mut self) {
        self.display_link = None;
    }

    pub(super) fn is_maximized(&self) -> bool {
        let bounds = self.bounds();
        let screen_size = unsafe {
            let screen: *mut AnyObject = msg_send![self.native_window, screen];
            let visible_frame: NSRect = msg_send![screen, visibleFrame];
            size(
                px(visible_frame.size.width as f32),
                px(visible_frame.size.height as f32),
            )
        };
        bounds.size == screen_size
    }

    pub(super) fn is_fullscreen(&self) -> bool {
        unsafe {
            let style_mask: objc2_app_kit::NSWindowStyleMask =
                msg_send![self.native_window, styleMask];
            style_mask.contains(objc2_app_kit::NSWindowStyleMask::FullScreen)
        }
    }

    pub(super) fn bounds(&self) -> Bounds<Pixels> {
        unsafe {
            let window_frame: NSRect = msg_send![self.native_window, frame];
            let screen: *mut AnyObject = msg_send![self.native_window, screen];
            if screen.is_null() {
                return Bounds::new(point(px(0.), px(0.)), inazuma::DEFAULT_WINDOW_SIZE);
            }
            let screen_frame: NSRect = msg_send![screen, frame];

            // Flip the y coordinate to be top-left origin
            let flipped_y =
                screen_frame.size.height - window_frame.origin.y - window_frame.size.height;

            Bounds::new(
                point(
                    px((window_frame.origin.x - screen_frame.origin.x) as f32),
                    px((flipped_y + screen_frame.origin.y) as f32),
                ),
                size(
                    px(window_frame.size.width as f32),
                    px(window_frame.size.height as f32),
                ),
            )
        }
    }

    pub(super) fn content_size(&self) -> Size<Pixels> {
        unsafe {
            let content_view: *mut AnyObject = msg_send![self.native_window, contentView];
            let frame: NSRect = msg_send![content_view, frame];
            size(px(frame.size.width as f32), px(frame.size.height as f32))
        }
    }

    pub(super) fn scale_factor(&self) -> f32 {
        get_scale_factor(self.native_window)
    }

    pub(super) fn titlebar_height(&self) -> Pixels {
        unsafe {
            let frame: NSRect = msg_send![self.native_window, frame];
            let content_layout_rect: NSRect = msg_send![self.native_window, contentLayoutRect];
            px((frame.size.height - content_layout_rect.size.height) as f32)
        }
    }

    pub(super) fn window_bounds(&self) -> WindowBounds {
        if self.is_fullscreen() {
            WindowBounds::Fullscreen(self.fullscreen_restore_bounds)
        } else {
            WindowBounds::Windowed(self.bounds())
        }
    }
}

unsafe impl Send for MacWindowState {}

pub(crate) struct MacWindow(pub(super) Arc<Mutex<MacWindowState>>);

impl Drop for MacWindow {
    fn drop(&mut self) {
        let mut this = self.0.lock();
        this.renderer.destroy();
        let window = this.native_window;
        let sheet_parent = this.sheet_parent.take();
        this.display_link.take();
        unsafe {
            let _: () = msg_send![this.native_window, setDelegate: ptr::null_mut::<AnyObject>()];
        }
        this.input_handler.take();
        this.foreground_executor
            .spawn(async move {
                unsafe {
                    if let Some(parent) = sheet_parent {
                        let _: () = msg_send![parent, endSheet: window];
                    }
                    let _: () = msg_send![window, close];
                    let _: *mut AnyObject = msg_send![window, autorelease];
                }
            })
            .detach();
    }
}

pub(crate) fn convert_mouse_position(position: NSPoint, window_height: Pixels) -> Point<Pixels> {
    point(
        px(position.x as f32),
        // macOS screen coordinates are relative to bottom left
        window_height - px(position.y as f32),
    )
}

pub(super) fn get_scale_factor(native_window: *mut AnyObject) -> f32 {
    let factor = unsafe {
        let screen: *mut AnyObject = msg_send![native_window, screen];
        if screen.is_null() {
            return 2.0;
        }
        let factor: f64 = msg_send![screen, backingScaleFactor];
        factor as f32
    };

    // We are not certain what triggers this, but it seems that sometimes
    // this method would return 0 (https://github.com/zed-industries/zed/issues/6412)
    // It seems most likely that this would happen if the window has no screen
    // (if it is off-screen), though we'd expect to see viewDidChangeBackingProperties before
    // it was rendered for real.
    // Regardless, attempt to avoid the issue here.
    if factor == 0.0 { 2. } else { factor }
}

impl super::class_registration::WindowIvars {
    pub(super) fn get_state(&self) -> Arc<Mutex<MacWindowState>> {
        let raw = self.window_state.get();
        assert!(!raw.is_null(), "window state not initialized");
        unsafe {
            let rc = Arc::from_raw(raw as *const Mutex<MacWindowState>);
            let clone = rc.clone();
            mem::forget(rc);
            clone
        }
    }
}

pub(super) unsafe fn display_id_for_screen(screen: *mut AnyObject) -> CGDirectDisplayID {
    unsafe {
        let device_description: *mut AnyObject = msg_send![screen, deviceDescription];
        let screen_number_key = objc2_foundation::NSString::from_str("NSScreenNumber");
        let screen_number: *mut AnyObject =
            msg_send![device_description, objectForKey: &*screen_number_key];
        let screen_number: usize = msg_send![screen_number, unsignedIntegerValue];
        screen_number as CGDirectDisplayID
    }
}