use super::*;

pub(crate) const WM_GPUI_CURSOR_STYLE_CHANGED: u32 = WM_USER + 1;
pub(crate) const WM_GPUI_CLOSE_ONE_WINDOW: u32 = WM_USER + 2;
pub(crate) const WM_GPUI_TASK_DISPATCHED_ON_MAIN_THREAD: u32 = WM_USER + 3;
pub(crate) const WM_GPUI_DOCK_MENU_ACTION: u32 = WM_USER + 4;
pub(crate) const WM_GPUI_FORCE_UPDATE_WINDOW: u32 = WM_USER + 5;
pub(crate) const WM_GPUI_KEYBOARD_LAYOUT_CHANGED: u32 = WM_USER + 6;
pub(crate) const WM_GPUI_GPU_DEVICE_LOST: u32 = WM_USER + 7;
pub(crate) const WM_GPUI_KEYDOWN: u32 = WM_USER + 8;

pub(super) const SIZE_MOVE_LOOP_TIMER_ID: usize = 1;


impl WindowsWindowInner {
    pub(crate) fn handle_msg(
        self: &Rc<Self>,
        handle: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let handled = match msg {
            // eagerly activate the window, so calls to `active_window` will work correctly
            WM_MOUSEACTIVATE => {
                unsafe { SetActiveWindow(handle).ok() };
                None
            }
            WM_ACTIVATE => self.handle_activate_msg(wparam),
            WM_CREATE => self.handle_create_msg(handle),
            WM_MOVE => self.handle_move_msg(handle, lparam),
            WM_SIZE => self.handle_size_msg(wparam, lparam),
            WM_GETMINMAXINFO => self.handle_get_min_max_info_msg(lparam),
            WM_ENTERSIZEMOVE | WM_ENTERMENULOOP => self.handle_size_move_loop(handle),
            WM_EXITSIZEMOVE | WM_EXITMENULOOP => self.handle_size_move_loop_exit(handle),
            WM_TIMER => self.handle_timer_msg(handle, wparam),
            WM_NCCALCSIZE => self.handle_calc_client_size(handle, wparam, lparam),
            WM_DPICHANGED => self.handle_dpi_changed_msg(handle, wparam, lparam),
            WM_DISPLAYCHANGE => self.handle_display_change_msg(handle),
            WM_NCHITTEST => self.handle_hit_test_msg(handle, lparam),
            WM_PAINT => self.handle_paint_msg(handle),
            WM_CLOSE => self.handle_close_msg(),
            WM_DESTROY => self.handle_destroy_msg(handle),
            WM_MOUSEMOVE => self.handle_mouse_move_msg(handle, lparam, wparam),
            WM_MOUSELEAVE | WM_NCMOUSELEAVE => self.handle_mouse_leave_msg(),
            WM_NCMOUSEMOVE => self.handle_nc_mouse_move_msg(handle, lparam),
            // Treat double click as a second single click, since we track the double clicks ourselves.
            // If you don't interact with any elements, this will fall through to the windows default
            // behavior of toggling whether the window is maximized.
            WM_NCLBUTTONDBLCLK | WM_NCLBUTTONDOWN => {
                self.handle_nc_mouse_down_msg(handle, MouseButton::Left, wparam, lparam)
            }
            WM_NCRBUTTONDOWN => {
                self.handle_nc_mouse_down_msg(handle, MouseButton::Right, wparam, lparam)
            }
            WM_NCMBUTTONDOWN => {
                self.handle_nc_mouse_down_msg(handle, MouseButton::Middle, wparam, lparam)
            }
            WM_NCLBUTTONUP => {
                self.handle_nc_mouse_up_msg(handle, MouseButton::Left, wparam, lparam)
            }
            WM_NCRBUTTONUP => {
                self.handle_nc_mouse_up_msg(handle, MouseButton::Right, wparam, lparam)
            }
            WM_NCMBUTTONUP => {
                self.handle_nc_mouse_up_msg(handle, MouseButton::Middle, wparam, lparam)
            }
            WM_LBUTTONDOWN => self.handle_mouse_down_msg(handle, MouseButton::Left, lparam),
            WM_RBUTTONDOWN => self.handle_mouse_down_msg(handle, MouseButton::Right, lparam),
            WM_MBUTTONDOWN => self.handle_mouse_down_msg(handle, MouseButton::Middle, lparam),
            WM_XBUTTONDOWN => {
                self.handle_xbutton_msg(handle, wparam, lparam, Self::handle_mouse_down_msg)
            }
            WM_LBUTTONUP => self.handle_mouse_up_msg(handle, MouseButton::Left, lparam),
            WM_RBUTTONUP => self.handle_mouse_up_msg(handle, MouseButton::Right, lparam),
            WM_MBUTTONUP => self.handle_mouse_up_msg(handle, MouseButton::Middle, lparam),
            WM_XBUTTONUP => {
                self.handle_xbutton_msg(handle, wparam, lparam, Self::handle_mouse_up_msg)
            }
            WM_MOUSEWHEEL => self.handle_mouse_wheel_msg(handle, wparam, lparam),
            WM_MOUSEHWHEEL => self.handle_mouse_horizontal_wheel_msg(handle, wparam, lparam),
            WM_SYSKEYUP => self.handle_syskeyup_msg(wparam, lparam),
            WM_KEYUP => self.handle_keyup_msg(wparam, lparam),
            WM_GPUI_KEYDOWN => self.handle_keydown_msg(wparam, lparam),
            WM_CHAR => self.handle_char_msg(wparam),
            WM_IME_STARTCOMPOSITION => self.handle_ime_position(handle),
            WM_IME_COMPOSITION => self.handle_ime_composition(handle, lparam),
            WM_SETCURSOR => self.handle_set_cursor(handle, lparam),
            WM_SETTINGCHANGE => self.handle_system_settings_changed(handle, wparam, lparam),
            WM_INPUTLANGCHANGE => self.handle_input_language_changed(),
            WM_SHOWWINDOW => self.handle_window_visibility_changed(handle, wparam),
            WM_GPUI_CURSOR_STYLE_CHANGED => self.handle_cursor_changed(lparam),
            WM_GPUI_FORCE_UPDATE_WINDOW => self.draw_window(handle, true),
            WM_GPUI_GPU_DEVICE_LOST => self.handle_device_lost(lparam),
            _ => None,
        };
        if let Some(n) = handled {
            LRESULT(n)
        } else {
            unsafe { DefWindowProcW(handle, msg, wparam, lparam) }
        }
    }

    fn handle_move_msg(&self, handle: HWND, lparam: LPARAM) -> Option<isize> {
        let origin = logical_point(
            lparam.signed_loword() as f32,
            lparam.signed_hiword() as f32,
            self.state.scale_factor.get(),
        );
        self.state.origin.set(origin);
        let size = self.state.logical_size.get();
        let center_x = origin.x.as_f32() + size.width.as_f32() / 2.;
        let center_y = origin.y.as_f32() + size.height.as_f32() / 2.;
        let monitor_bounds = self.state.display.get().bounds();
        if center_x < monitor_bounds.left().as_f32()
            || center_x > monitor_bounds.right().as_f32()
            || center_y < monitor_bounds.top().as_f32()
            || center_y > monitor_bounds.bottom().as_f32()
        {
            // center of the window may have moved to another monitor
            let monitor = unsafe { MonitorFromWindow(handle, MONITOR_DEFAULTTONULL) };
            // minimize the window can trigger this event too, in this case,
            // monitor is invalid, we do nothing.
            if !monitor.is_invalid() && self.state.display.get().handle != monitor {
                // we will get the same monitor if we only have one
                self.state
                    .display
                    .set(WindowsDisplay::new_with_handle(monitor).log_err()?);
            }
        }
        if let Some(mut callback) = self.state.callbacks.moved.take() {
            callback();
            self.state.callbacks.moved.set(Some(callback));
        }
        Some(0)
    }

    fn handle_get_min_max_info_msg(&self, lparam: LPARAM) -> Option<isize> {
        let min_size = self.state.min_size?;
        let scale_factor = self.state.scale_factor.get();
        let boarder_offset = &self.state.border_offset;

        unsafe {
            let minmax_info = &mut *(lparam.0 as *mut MINMAXINFO);
            minmax_info.ptMinTrackSize.x = min_size.width.scale(scale_factor).as_f32() as i32
                + boarder_offset.width_offset.get();
            minmax_info.ptMinTrackSize.y = min_size.height.scale(scale_factor).as_f32() as i32
                + boarder_offset.height_offset.get();
        }
        Some(0)
    }

    fn handle_size_msg(&self, wparam: WPARAM, lparam: LPARAM) -> Option<isize> {
        // Don't resize the renderer when the window is minimized, but record that it was minimized so
        // that on restore the swap chain can be recreated via `update_drawable_size_even_if_unchanged`.
        if wparam.0 == SIZE_MINIMIZED as usize {
            self.state
                .restore_from_minimized
                .set(self.state.callbacks.request_frame.take());
            return Some(0);
        }

        let width = lparam.loword().max(1) as i32;
        let height = lparam.hiword().max(1) as i32;
        let new_size = size(DevicePixels(width), DevicePixels(height));

        let scale_factor = self.state.scale_factor.get();
        let mut should_resize_renderer = false;
        if let Some(restore_from_minimized) = self.state.restore_from_minimized.take() {
            self.state
                .callbacks
                .request_frame
                .set(Some(restore_from_minimized));
        } else {
            should_resize_renderer = true;
        }

        self.handle_size_change(new_size, scale_factor, should_resize_renderer);
        Some(0)
    }

    fn handle_size_change(
        &self,
        device_size: Size<DevicePixels>,
        scale_factor: f32,
        should_resize_renderer: bool,
    ) {
        let new_logical_size = device_size.to_pixels(scale_factor);

        self.state.logical_size.set(new_logical_size);
        if should_resize_renderer
            && let Err(e) = self.state.renderer.borrow_mut().resize(device_size)
        {
            log::error!("Failed to resize renderer, invalidating devices: {}", e);
            self.state
                .invalidate_devices
                .store(true, std::sync::atomic::Ordering::Release);
        }
        if let Some(mut callback) = self.state.callbacks.resize.take() {
            callback(new_logical_size, scale_factor);
            self.state.callbacks.resize.set(Some(callback));
        }
    }

    fn handle_size_move_loop(&self, handle: HWND) -> Option<isize> {
        unsafe {
            let ret = SetTimer(
                Some(handle),
                SIZE_MOVE_LOOP_TIMER_ID,
                USER_TIMER_MINIMUM,
                None,
            );
            if ret == 0 {
                log::error!(
                    "unable to create timer: {}",
                    std::io::Error::last_os_error()
                );
            }
        }
        None
    }

    fn handle_size_move_loop_exit(&self, handle: HWND) -> Option<isize> {
        unsafe {
            KillTimer(Some(handle), SIZE_MOVE_LOOP_TIMER_ID).log_err();
        }
        None
    }

    fn handle_timer_msg(&self, handle: HWND, wparam: WPARAM) -> Option<isize> {
        if wparam.0 == SIZE_MOVE_LOOP_TIMER_ID {
            let mut runnables = self.main_receiver.clone().try_iter();
            while let Some(Ok(runnable)) = runnables.next() {
                WindowsDispatcher::execute_runnable(runnable);
            }
            self.handle_paint_msg(handle)
        } else {
            None
        }
    }

    fn handle_paint_msg(&self, handle: HWND) -> Option<isize> {
        self.draw_window(handle, false)
    }

    fn handle_close_msg(&self) -> Option<isize> {
        let mut callback = self.state.callbacks.should_close.take()?;
        let should_close = callback();
        self.state.callbacks.should_close.set(Some(callback));
        if should_close { None } else { Some(0) }
    }

    fn handle_destroy_msg(&self, handle: HWND) -> Option<isize> {
        let callback = { self.state.callbacks.close.take() };
        // Re-enable parent window if this was a modal dialog
        if let Some(parent_hwnd) = self.parent_hwnd {
            unsafe {
                let _ = EnableWindow(parent_hwnd, true);
                let _ = SetForegroundWindow(parent_hwnd);
            }
        }

        if let Some(callback) = callback {
            callback();
        }
        unsafe {
            PostMessageW(
                Some(self.platform_window_handle),
                WM_GPUI_CLOSE_ONE_WINDOW,
                WPARAM(self.validation_number),
                LPARAM(handle.0 as isize),
            )
            .log_err();
        }
        Some(0)
    }

    fn handle_mouse_move_msg(&self, handle: HWND, lparam: LPARAM, wparam: WPARAM) -> Option<isize> {
        self.start_tracking_mouse(handle, TME_LEAVE);

        let Some(mut func) = self.state.callbacks.input.take() else {
            return Some(1);
        };
        let scale_factor = self.state.scale_factor.get();

        let pressed_button = match MODIFIERKEYS_FLAGS(wparam.loword() as u32) {
            flags if flags.contains(MK_LBUTTON) => Some(MouseButton::Left),
            flags if flags.contains(MK_RBUTTON) => Some(MouseButton::Right),
            flags if flags.contains(MK_MBUTTON) => Some(MouseButton::Middle),
            flags if flags.contains(MK_XBUTTON1) => {
                Some(MouseButton::Navigate(NavigationDirection::Back))
            }
            flags if flags.contains(MK_XBUTTON2) => {
                Some(MouseButton::Navigate(NavigationDirection::Forward))
            }
            _ => None,
        };
        let x = lparam.signed_loword() as f32;
        let y = lparam.signed_hiword() as f32;
        let input = PlatformInput::MouseMove(MouseMoveEvent {
            position: logical_point(x, y, scale_factor),
            pressed_button,
            modifiers: current_modifiers(),
        });
        let handled = !func(input).propagate;
        self.state.callbacks.input.set(Some(func));

        if handled { Some(0) } else { Some(1) }
    }

    fn handle_mouse_leave_msg(&self) -> Option<isize> {
        self.state.hovered.set(false);
        if let Some(mut callback) = self.state.callbacks.hovered_status_change.take() {
            callback(false);
            self.state
                .callbacks
                .hovered_status_change
                .set(Some(callback));
        }

        Some(0)
    }

    fn handle_syskeyup_msg(&self, wparam: WPARAM, lparam: LPARAM) -> Option<isize> {
        let input = handle_key_event(wparam, lparam, &self.state, |keystroke, _| {
            PlatformInput::KeyUp(KeyUpEvent { keystroke })
        })?;
        let mut func = self.state.callbacks.input.take()?;

        func(input);
        self.state.callbacks.input.set(Some(func));

        // Always return 0 to indicate that the message was handled, so we could properly handle `ModifiersChanged` event.
        Some(0)
    }

    // It's a known bug that you can't trigger `ctrl-shift-0`. See:
    // https://superuser.com/questions/1455762/ctrl-shift-number-key-combination-has-stopped-working-for-a-few-numbers
    fn handle_keydown_msg(&self, wparam: WPARAM, lparam: LPARAM) -> Option<isize> {
        let Some(input) = handle_key_event(
            wparam,
            lparam,
            &self.state,
            |keystroke, prefer_character_input| {
                PlatformInput::KeyDown(KeyDownEvent {
                    keystroke,
                    is_held: lparam.0 & (0x1 << 30) > 0,
                    prefer_character_input,
                })
            },
        ) else {
            return Some(1);
        };

        let Some(mut func) = self.state.callbacks.input.take() else {
            return Some(1);
        };

        let handled = !func(input).propagate;

        self.state.callbacks.input.set(Some(func));

        if handled { Some(0) } else { Some(1) }
    }

    fn handle_keyup_msg(&self, wparam: WPARAM, lparam: LPARAM) -> Option<isize> {
        let Some(input) = handle_key_event(wparam, lparam, &self.state, |keystroke, _| {
            PlatformInput::KeyUp(KeyUpEvent { keystroke })
        }) else {
            return Some(1);
        };

        let Some(mut func) = self.state.callbacks.input.take() else {
            return Some(1);
        };

        let handled = !func(input).propagate;
        self.state.callbacks.input.set(Some(func));

        if handled { Some(0) } else { Some(1) }
    }

    fn handle_char_msg(&self, wparam: WPARAM) -> Option<isize> {
        let input = self.parse_char_message(wparam)?;
        self.with_input_handler(|input_handler| {
            input_handler.replace_text_in_range(None, &input);
        });

        Some(0)
    }

    fn handle_mouse_down_msg(
        &self,
        handle: HWND,
        button: MouseButton,
        lparam: LPARAM,
    ) -> Option<isize> {
        unsafe { SetCapture(handle) };

        let Some(mut func) = self.state.callbacks.input.take() else {
            return Some(1);
        };
        let x = lparam.signed_loword();
        let y = lparam.signed_hiword();
        let physical_point = point(DevicePixels(x as i32), DevicePixels(y as i32));
        let click_count = self.state.click_state.update(button, physical_point);
        let scale_factor = self.state.scale_factor.get();

        let input = PlatformInput::MouseDown(MouseDownEvent {
            button,
            position: logical_point(x as f32, y as f32, scale_factor),
            modifiers: current_modifiers(),
            click_count,
            first_mouse: false,
        });
        let handled = !func(input).propagate;
        self.state.callbacks.input.set(Some(func));

        if handled { Some(0) } else { Some(1) }
    }

    fn handle_mouse_up_msg(
        &self,
        _handle: HWND,
        button: MouseButton,
        lparam: LPARAM,
    ) -> Option<isize> {
        unsafe { ReleaseCapture().log_err() };

        let Some(mut func) = self.state.callbacks.input.take() else {
            return Some(1);
        };
        let x = lparam.signed_loword() as f32;
        let y = lparam.signed_hiword() as f32;
        let click_count = self.state.click_state.current_count.get();
        let scale_factor = self.state.scale_factor.get();

        let input = PlatformInput::MouseUp(MouseUpEvent {
            button,
            position: logical_point(x, y, scale_factor),
            modifiers: current_modifiers(),
            click_count,
        });
        let handled = !func(input).propagate;
        self.state.callbacks.input.set(Some(func));

        if handled { Some(0) } else { Some(1) }
    }

    fn handle_xbutton_msg(
        &self,
        handle: HWND,
        wparam: WPARAM,
        lparam: LPARAM,
        handler: impl Fn(&Self, HWND, MouseButton, LPARAM) -> Option<isize>,
    ) -> Option<isize> {
        let nav_dir = match wparam.hiword() {
            XBUTTON1 => NavigationDirection::Back,
            XBUTTON2 => NavigationDirection::Forward,
            _ => return Some(1),
        };
        handler(self, handle, MouseButton::Navigate(nav_dir), lparam)
    }

    fn handle_mouse_wheel_msg(
        &self,
        handle: HWND,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<isize> {
        let modifiers = current_modifiers();

        let Some(mut func) = self.state.callbacks.input.take() else {
            return Some(1);
        };
        let scale_factor = self.state.scale_factor.get();
        let wheel_scroll_amount = match modifiers.shift {
            true => self
                .system_settings()
                .mouse_wheel_settings
                .wheel_scroll_chars
                .get(),
            false => self
                .system_settings()
                .mouse_wheel_settings
                .wheel_scroll_lines
                .get(),
        };

        let wheel_distance =
            (wparam.signed_hiword() as f32 / WHEEL_DELTA as f32) * wheel_scroll_amount as f32;
        let mut cursor_point = POINT {
            x: lparam.signed_loword().into(),
            y: lparam.signed_hiword().into(),
        };
        unsafe { ScreenToClient(handle, &mut cursor_point).ok().log_err() };
        let input = PlatformInput::ScrollWheel(ScrollWheelEvent {
            position: logical_point(cursor_point.x as f32, cursor_point.y as f32, scale_factor),
            delta: ScrollDelta::Lines(match modifiers.shift {
                true => Point {
                    x: wheel_distance,
                    y: 0.0,
                },
                false => Point {
                    y: wheel_distance,
                    x: 0.0,
                },
            }),
            modifiers,
            touch_phase: TouchPhase::Moved,
        });
        let handled = !func(input).propagate;
        self.state.callbacks.input.set(Some(func));

        if handled { Some(0) } else { Some(1) }
    }

    fn handle_mouse_horizontal_wheel_msg(
        &self,
        handle: HWND,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<isize> {
        let Some(mut func) = self.state.callbacks.input.take() else {
            return Some(1);
        };
        let scale_factor = self.state.scale_factor.get();
        let wheel_scroll_chars = self
            .system_settings()
            .mouse_wheel_settings
            .wheel_scroll_chars
            .get();

        let wheel_distance =
            (-wparam.signed_hiword() as f32 / WHEEL_DELTA as f32) * wheel_scroll_chars as f32;
        let mut cursor_point = POINT {
            x: lparam.signed_loword().into(),
            y: lparam.signed_hiword().into(),
        };
        unsafe { ScreenToClient(handle, &mut cursor_point).ok().log_err() };
        let event = PlatformInput::ScrollWheel(ScrollWheelEvent {
            position: logical_point(cursor_point.x as f32, cursor_point.y as f32, scale_factor),
            delta: ScrollDelta::Lines(Point {
                x: wheel_distance,
                y: 0.0,
            }),
            modifiers: current_modifiers(),
            touch_phase: TouchPhase::Moved,
        });
        let handled = !func(event).propagate;
        self.state.callbacks.input.set(Some(func));

        if handled { Some(0) } else { Some(1) }
    }

    fn retrieve_caret_position(&self) -> Option<POINT> {
        self.with_input_handler_and_scale_factor(|input_handler, scale_factor| {
            let caret_range = input_handler.selected_text_range(false)?;
            let caret_position = input_handler.bounds_for_range(caret_range.range)?;
            Some(POINT {
                // logical to physical
                x: (caret_position.origin.x.as_f32() * scale_factor) as i32,
                y: (caret_position.origin.y.as_f32() * scale_factor) as i32
                    + ((caret_position.size.height.as_f32() * scale_factor) as i32 / 2),
            })
        })
    }

    fn handle_ime_position(&self, handle: HWND) -> Option<isize> {
        if let Some(caret_position) = self.retrieve_caret_position() {
            self.update_ime_position(handle, caret_position);
        }
        Some(0)
    }

    pub(crate) fn update_ime_position(&self, handle: HWND, caret_position: POINT) {
        let Some(ctx) = ImeContext::get(handle) else {
            return;
        };
        unsafe {
            ImmSetCompositionWindow(
                *ctx,
                &COMPOSITIONFORM {
                    dwStyle: CFS_POINT,
                    ptCurrentPos: caret_position,
                    ..Default::default()
                },
            )
            .ok()
            .log_err();

            ImmSetCandidateWindow(
                *ctx,
                &CANDIDATEFORM {
                    dwStyle: CFS_CANDIDATEPOS,
                    ptCurrentPos: caret_position,
                    ..Default::default()
                },
            )
            .ok()
            .log_err();
        }
    }

    fn update_ime_enabled(&self, handle: HWND) {
        let ime_enabled = self
            .with_input_handler(|input_handler| input_handler.query_accepts_text_input())
            .unwrap_or(false);
        if ime_enabled == self.state.ime_enabled.get() {
            return;
        }
        self.state.ime_enabled.set(ime_enabled);
        unsafe {
            if ime_enabled {
                ImmAssociateContextEx(handle, HIMC::default(), IACE_DEFAULT)
                    .ok()
                    .log_err();
            } else {
                if let Some(ctx) = ImeContext::get(handle) {
                    ImmNotifyIME(*ctx, NI_COMPOSITIONSTR, CPS_COMPLETE, 0)
                        .ok()
                        .log_err();
                }
                ImmAssociateContextEx(handle, HIMC::default(), 0)
                    .ok()
                    .log_err();
            }
        }
    }

    fn handle_ime_composition(&self, handle: HWND, lparam: LPARAM) -> Option<isize> {
        let ctx = ImeContext::get(handle)?;
        self.handle_ime_composition_inner(*ctx, lparam)
    }

    fn handle_ime_composition_inner(&self, ctx: HIMC, lparam: LPARAM) -> Option<isize> {
        let lparam = lparam.0 as u32;
        if lparam == 0 {
            // Japanese IME may send this message with lparam = 0, which indicates that
            // there is no composition string.
            self.with_input_handler(|input_handler| {
                input_handler.replace_text_in_range(None, "");
            })?;
            Some(0)
        } else {
            if lparam & GCS_RESULTSTR.0 > 0 {
                let comp_result = parse_ime_composition_string(ctx, GCS_RESULTSTR)?;
                self.with_input_handler(|input_handler| {
                    input_handler
                        .replace_text_in_range(None, &String::from_utf16_lossy(&comp_result));
                })?;
            }
            if lparam & GCS_COMPSTR.0 > 0 {
                let comp_string = parse_ime_composition_string(ctx, GCS_COMPSTR)?;
                let caret_pos =
                    (!comp_string.is_empty() && lparam & GCS_CURSORPOS.0 > 0).then(|| {
                        let cursor_pos = retrieve_composition_cursor_position(ctx);
                        let pos = if should_use_ime_cursor_position(ctx, cursor_pos) {
                            cursor_pos
                        } else {
                            comp_string.len()
                        };
                        pos..pos
                    });
                self.with_input_handler(|input_handler| {
                    input_handler.replace_and_mark_text_in_range(
                        None,
                        &String::from_utf16_lossy(&comp_string),
                        caret_pos,
                    );
                })?;
            }
            if lparam & (GCS_RESULTSTR.0 | GCS_COMPSTR.0) > 0 {
                return Some(0);
            }

            // currently, we don't care other stuff
            None
        }
    }
