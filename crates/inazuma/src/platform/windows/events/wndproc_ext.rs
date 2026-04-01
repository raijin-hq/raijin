use super::*;

impl WindowsWindowInner {

    fn handle_calc_client_size(
        &self,
        handle: HWND,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<isize> {
        if !self.hide_title_bar || self.state.is_fullscreen() || wparam.0 == 0 {
            return None;
        }

        unsafe {
            let params = lparam.0 as *mut NCCALCSIZE_PARAMS;
            let saved_top = (*params).rgrc[0].top;
            let result = DefWindowProcW(handle, WM_NCCALCSIZE, wparam, lparam);
            (*params).rgrc[0].top = saved_top;
            if self.state.is_maximized() {
                let dpi = GetDpiForWindow(handle);
                (*params).rgrc[0].top += get_frame_thicknessx(dpi);
            }
            Some(result.0 as isize)
        }
    }

    fn handle_activate_msg(self: &Rc<Self>, wparam: WPARAM) -> Option<isize> {
        let activated = wparam.loword() > 0;
        let this = self.clone();
        self.executor
            .spawn(async move {
                if let Some(mut func) = this.state.callbacks.active_status_change.take() {
                    func(activated);
                    this.state.callbacks.active_status_change.set(Some(func));
                }
            })
            .detach();

        None
    }

    fn handle_create_msg(&self, handle: HWND) -> Option<isize> {
        if self.hide_title_bar {
            notify_frame_changed(handle);
            Some(0)
        } else {
            None
        }
    }

    fn handle_dpi_changed_msg(
        &self,
        handle: HWND,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<isize> {
        let new_dpi = wparam.loword() as f32;

        let is_maximized = self.state.is_maximized();
        let new_scale_factor = new_dpi / USER_DEFAULT_SCREEN_DPI as f32;
        self.state.scale_factor.set(new_scale_factor);
        self.state.border_offset.update(handle).log_err();

        if is_maximized {
            // Get the monitor and its work area at the new DPI
            let monitor = unsafe { MonitorFromWindow(handle, MONITOR_DEFAULTTONEAREST) };
            let mut monitor_info: MONITORINFO = unsafe { std::mem::zeroed() };
            monitor_info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if unsafe { GetMonitorInfoW(monitor, &mut monitor_info) }.as_bool() {
                let work_area = monitor_info.rcWork;
                let width = work_area.right - work_area.left;
                let height = work_area.bottom - work_area.top;

                // Update the window size to match the new monitor work area
                // This will trigger WM_SIZE which will handle the size change
                unsafe {
                    SetWindowPos(
                        handle,
                        None,
                        work_area.left,
                        work_area.top,
                        width,
                        height,
                        SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                    )
                    .context("unable to set maximized window position after dpi has changed")
                    .log_err();
                }

                // SetWindowPos may not send WM_SIZE for maximized windows in some cases,
                // so we manually update the size to ensure proper rendering
                let device_size = size(DevicePixels(width), DevicePixels(height));
                self.handle_size_change(device_size, new_scale_factor, true);
            }
        } else {
            // For non-maximized windows, use the suggested RECT from the system
            let rect = unsafe { &*(lparam.0 as *const RECT) };
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            // this will emit `WM_SIZE` and `WM_MOVE` right here
            // even before this function returns
            // the new size is handled in `WM_SIZE`
            unsafe {
                SetWindowPos(
                    handle,
                    None,
                    rect.left,
                    rect.top,
                    width,
                    height,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                )
                .context("unable to set window position after dpi has changed")
                .log_err();
            }
        }

        Some(0)
    }

    fn handle_display_change_msg(&self, handle: HWND) -> Option<isize> {
        let new_monitor = unsafe { MonitorFromWindow(handle, MONITOR_DEFAULTTONULL) };
        if new_monitor.is_invalid() {
            log::error!("No monitor detected!");
            return None;
        }
        let new_display = WindowsDisplay::new_with_handle(new_monitor).log_err()?;
        self.state.display.set(new_display);
        Some(0)
    }

    fn handle_hit_test_msg(&self, handle: HWND, lparam: LPARAM) -> Option<isize> {
        if !self.is_movable || self.state.is_fullscreen() {
            return None;
        }

        let callback = self.state.callbacks.hit_test_window_control.take();
        let drag_area = if let Some(mut callback) = callback {
            let area = callback();
            self.state
                .callbacks
                .hit_test_window_control
                .set(Some(callback));
            if let Some(area) = area {
                match area {
                    WindowControlArea::Drag => Some(HTCAPTION as _),
                    WindowControlArea::Close => return Some(HTCLOSE as _),
                    WindowControlArea::Max => return Some(HTMAXBUTTON as _),
                    WindowControlArea::Min => return Some(HTMINBUTTON as _),
                }
            } else {
                None
            }
        } else {
            None
        };

        if !self.hide_title_bar {
            // If the OS draws the title bar, we don't need to handle hit test messages.
            return drag_area;
        }

        let dpi = unsafe { GetDpiForWindow(handle) };
        // We do not use the OS title bar, so the default `DefWindowProcW` will only register a 1px edge for resizes
        // We need to calculate the frame thickness ourselves and do the hit test manually.
        let frame_y = get_frame_thicknessx(dpi);
        let frame_x = get_frame_thicknessy(dpi);
        let mut cursor_point = POINT {
            x: lparam.signed_loword().into(),
            y: lparam.signed_hiword().into(),
        };

        unsafe { ScreenToClient(handle, &mut cursor_point).ok().log_err() };
        if !self.state.is_maximized() && 0 <= cursor_point.y && cursor_point.y <= frame_y {
            // x-axis actually goes from -frame_x to 0
            return Some(if cursor_point.x <= 0 {
                HTTOPLEFT
            } else {
                let mut rect = Default::default();
                unsafe { GetWindowRect(handle, &mut rect) }.log_err();
                // right and bottom bounds of RECT are exclusive, thus `-1`
                let right = rect.right - rect.left - 1;
                // the bounds include the padding frames, so accomodate for both of them
                if right - 2 * frame_x <= cursor_point.x {
                    HTTOPRIGHT
                } else {
                    HTTOP
                }
            } as _);
        }

        drag_area
    }

    fn handle_nc_mouse_move_msg(&self, handle: HWND, lparam: LPARAM) -> Option<isize> {
        self.start_tracking_mouse(handle, TME_LEAVE | TME_NONCLIENT);

        let mut func = self.state.callbacks.input.take()?;
        let scale_factor = self.state.scale_factor.get();

        let mut cursor_point = POINT {
            x: lparam.signed_loword().into(),
            y: lparam.signed_hiword().into(),
        };
        unsafe { ScreenToClient(handle, &mut cursor_point).ok().log_err() };
        let input = PlatformInput::MouseMove(MouseMoveEvent {
            position: logical_point(cursor_point.x as f32, cursor_point.y as f32, scale_factor),
            pressed_button: None,
            modifiers: current_modifiers(),
        });
        let handled = !func(input).propagate;
        self.state.callbacks.input.set(Some(func));

        if handled { Some(0) } else { None }
    }

    fn handle_nc_mouse_down_msg(
        &self,
        handle: HWND,
        button: MouseButton,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<isize> {
        if let Some(mut func) = self.state.callbacks.input.take() {
            let scale_factor = self.state.scale_factor.get();
            let mut cursor_point = POINT {
                x: lparam.signed_loword().into(),
                y: lparam.signed_hiword().into(),
            };
            unsafe { ScreenToClient(handle, &mut cursor_point).ok().log_err() };
            let physical_point = point(DevicePixels(cursor_point.x), DevicePixels(cursor_point.y));
            let click_count = self.state.click_state.update(button, physical_point);

            let input = PlatformInput::MouseDown(MouseDownEvent {
                button,
                position: logical_point(cursor_point.x as f32, cursor_point.y as f32, scale_factor),
                modifiers: current_modifiers(),
                click_count,
                first_mouse: false,
            });
            let handled = !func(input).propagate;
            self.state.callbacks.input.set(Some(func));

            if handled {
                return Some(0);
            }
        } else {
        };

        // Since these are handled in handle_nc_mouse_up_msg we must prevent the default window proc
        if button == MouseButton::Left {
            match wparam.0 as u32 {
                HTMINBUTTON => self.state.nc_button_pressed.set(Some(HTMINBUTTON)),
                HTMAXBUTTON => self.state.nc_button_pressed.set(Some(HTMAXBUTTON)),
                HTCLOSE => self.state.nc_button_pressed.set(Some(HTCLOSE)),
                _ => return None,
            };
            Some(0)
        } else {
            None
        }
    }

    fn handle_nc_mouse_up_msg(
        &self,
        handle: HWND,
        button: MouseButton,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<isize> {
        if let Some(mut func) = self.state.callbacks.input.take() {
            let scale_factor = self.state.scale_factor.get();

            let mut cursor_point = POINT {
                x: lparam.signed_loword().into(),
                y: lparam.signed_hiword().into(),
            };
            unsafe { ScreenToClient(handle, &mut cursor_point).ok().log_err() };
            let input = PlatformInput::MouseUp(MouseUpEvent {
                button,
                position: logical_point(cursor_point.x as f32, cursor_point.y as f32, scale_factor),
                modifiers: current_modifiers(),
                click_count: 1,
            });
            let handled = !func(input).propagate;
            self.state.callbacks.input.set(Some(func));

            if handled {
                return Some(0);
            }
        } else {
        }

        let last_pressed = self.state.nc_button_pressed.take();
        if button == MouseButton::Left
            && let Some(last_pressed) = last_pressed
        {
            let handled = match (wparam.0 as u32, last_pressed) {
                (HTMINBUTTON, HTMINBUTTON) => {
                    unsafe { ShowWindowAsync(handle, SW_MINIMIZE).ok().log_err() };
                    true
                }
                (HTMAXBUTTON, HTMAXBUTTON) => {
                    if self.state.is_maximized() {
                        unsafe { ShowWindowAsync(handle, SW_NORMAL).ok().log_err() };
                    } else {
                        unsafe { ShowWindowAsync(handle, SW_MAXIMIZE).ok().log_err() };
                    }
                    true
                }
                (HTCLOSE, HTCLOSE) => {
                    unsafe {
                        PostMessageW(Some(handle), WM_CLOSE, WPARAM::default(), LPARAM::default())
                            .log_err()
                    };
                    true
                }
                _ => false,
            };
            if handled {
                return Some(0);
            }
        }

        None
    }

    fn handle_cursor_changed(&self, lparam: LPARAM) -> Option<isize> {
        let had_cursor = self.state.current_cursor.get().is_some();

        self.state.current_cursor.set(if lparam.0 == 0 {
            None
        } else {
            Some(HCURSOR(lparam.0 as _))
        });

        if had_cursor != self.state.current_cursor.get().is_some() {
            unsafe { SetCursor(self.state.current_cursor.get()) };
        }

        Some(0)
    }

    fn handle_set_cursor(&self, handle: HWND, lparam: LPARAM) -> Option<isize> {
        if unsafe { !IsWindowEnabled(handle).as_bool() }
            || matches!(
                lparam.loword() as u32,
                HTLEFT
                    | HTRIGHT
                    | HTTOP
                    | HTTOPLEFT
                    | HTTOPRIGHT
                    | HTBOTTOM
                    | HTBOTTOMLEFT
                    | HTBOTTOMRIGHT
            )
        {
            return None;
        }
        unsafe {
            SetCursor(self.state.current_cursor.get());
        };
        Some(0)
    }

    fn handle_system_settings_changed(
        &self,
        handle: HWND,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<isize> {
        if wparam.0 != 0 {
            self.state.click_state.system_update(wparam.0);
            self.state.border_offset.update(handle).log_err();
            // system settings may emit a window message which wants to take the refcell self.state, so drop it

            self.system_settings().update(wparam.0);
        } else {
            self.handle_system_theme_changed(handle, lparam)?;
        };

        Some(0)
    }

    fn handle_system_theme_changed(&self, handle: HWND, lparam: LPARAM) -> Option<isize> {
        // lParam is a pointer to a string that indicates the area containing the system parameter
        // that was changed.
        let parameter = PCWSTR::from_raw(lparam.0 as _);
        if unsafe { !parameter.is_null() && !parameter.is_empty() }
            && let Some(parameter_string) = unsafe { parameter.to_string() }.log_err()
        {
            log::info!("System settings changed: {}", parameter_string);
            if parameter_string.as_str() == "ImmersiveColorSet" {
                let new_appearance = system_appearance()
                    .context("unable to get system appearance when handling ImmersiveColorSet")
                    .log_err()?;

                if new_appearance != self.state.appearance.get() {
                    self.state.appearance.set(new_appearance);
                    let mut callback = self.state.callbacks.appearance_changed.take()?;

                    callback();
                    self.state.callbacks.appearance_changed.set(Some(callback));
                    configure_dwm_dark_mode(handle, new_appearance);
                }
            }
        }
        Some(0)
    }

    fn handle_input_language_changed(&self) -> Option<isize> {
        unsafe {
            PostMessageW(
                Some(self.platform_window_handle),
                WM_GPUI_KEYBOARD_LAYOUT_CHANGED,
                WPARAM(self.validation_number),
                LPARAM(0),
            )
            .log_err();
        }
        Some(0)
    }

    fn handle_window_visibility_changed(&self, handle: HWND, wparam: WPARAM) -> Option<isize> {
        if wparam.0 == 1 {
            self.draw_window(handle, false);
        }
        None
    }

    fn handle_device_lost(&self, lparam: LPARAM) -> Option<isize> {
        let devices = lparam.0 as *const DirectXDevices;
        let devices = unsafe { &*devices };
        if let Err(err) = self
            .state
            .renderer
            .borrow_mut()
            .handle_device_lost(&devices)
        {
            panic!("Device lost: {err}");
        }
        Some(0)
    }

    #[inline]
    fn draw_window(&self, handle: HWND, force_render: bool) -> Option<isize> {
        let mut request_frame = self.state.callbacks.request_frame.take()?;

        // we are instructing gpui to force render a frame, this will
        // re-populate all the gpu textures for us so we can resume drawing in
        // case we disabled drawing earlier due to a device loss
        self.state.renderer.borrow_mut().mark_drawable();
        request_frame(RequestFrameOptions {
            require_presentation: false,
            force_render,
        });

        self.state.callbacks.request_frame.set(Some(request_frame));
        self.update_ime_enabled(handle);
        unsafe { ValidateRect(Some(handle), None).ok().log_err() };

        Some(0)
    }

    #[inline]
    fn parse_char_message(&self, wparam: WPARAM) -> Option<String> {
        let code_point = wparam.loword();

        // https://www.unicode.org/versions/Unicode16.0.0/core-spec/chapter-3/#G2630
        match code_point {
            0xD800..=0xDBFF => {
                // High surrogate, wait for low surrogate
                self.state.pending_surrogate.set(Some(code_point));
                None
            }
            0xDC00..=0xDFFF => {
                if let Some(high_surrogate) = self.state.pending_surrogate.take() {
                    // Low surrogate, combine with pending high surrogate
                    String::from_utf16(&[high_surrogate, code_point]).ok()
                } else {
                    // Invalid low surrogate without a preceding high surrogate
                    log::warn!(
                        "Received low surrogate without a preceding high surrogate: {code_point:x}"
                    );
                    None
                }
            }
            _ => {
                self.state.pending_surrogate.set(None);
                char::from_u32(code_point as u32)
                    .filter(|c| !c.is_control())
                    .map(|c| c.to_string())
            }
        }
    }

    fn start_tracking_mouse(&self, handle: HWND, flags: TRACKMOUSEEVENT_FLAGS) {
        if !self.state.hovered.get() {
            self.state.hovered.set(true);
            unsafe {
                TrackMouseEvent(&mut TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: flags,
                    hwndTrack: handle,
                    dwHoverTime: HOVER_DEFAULT,
                })
                .log_err()
            };
            if let Some(mut callback) = self.state.callbacks.hovered_status_change.take() {
                callback(true);
                self.state
                    .callbacks
                    .hovered_status_change
                    .set(Some(callback));
            }
        }
    }

    fn with_input_handler<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut PlatformInputHandler) -> R,
    {
        let mut input_handler = self.state.input_handler.take()?;
        let result = f(&mut input_handler);
        self.state.input_handler.set(Some(input_handler));
        Some(result)
    }

    fn with_input_handler_and_scale_factor<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut PlatformInputHandler, f32) -> Option<R>,
    {
        let mut input_handler = self.state.input_handler.take()?;
        let scale_factor = self.state.scale_factor.get();

        let result = f(&mut input_handler, scale_factor);
        self.state.input_handler.set(Some(input_handler));
        result
    }
}

