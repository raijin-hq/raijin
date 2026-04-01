use super::*;

impl X11Client {
    fn handle_event(&self, event: Event) -> Option<()> {
        match event {
            Event::UnmapNotify(event) => {
                let mut state = self.0.borrow_mut();
                if let Some(window_ref) = state.windows.get_mut(&event.window) {
                    window_ref.is_mapped = false;
                }
                state.update_refresh_loop(event.window);
            }
            Event::MapNotify(event) => {
                let mut state = self.0.borrow_mut();
                if let Some(window_ref) = state.windows.get_mut(&event.window) {
                    window_ref.is_mapped = true;
                }
                state.update_refresh_loop(event.window);
            }
            Event::VisibilityNotify(event) => {
                let mut state = self.0.borrow_mut();
                if let Some(window_ref) = state.windows.get_mut(&event.window) {
                    window_ref.last_visibility = event.state;
                }
                state.update_refresh_loop(event.window);
            }
            Event::ClientMessage(event) => {
                let window = self.get_window(event.window)?;
                let [atom, arg1, arg2, arg3, arg4] = event.data.as_data32();
                let mut state = self.0.borrow_mut();

                if atom == state.atoms.WM_DELETE_WINDOW && window.should_close() {
                    // window "x" button clicked by user
                    // Rest of the close logic is handled in drop_window()
                    drop(state);
                    window.close();
                    state = self.0.borrow_mut();
                } else if atom == state.atoms._NET_WM_SYNC_REQUEST {
                    window.state.borrow_mut().last_sync_counter =
                        Some(x11rb::protocol::sync::Int64 {
                            lo: arg2,
                            hi: arg3 as i32,
                        })
                }

                if event.type_ == state.atoms.XdndEnter {
                    state.xdnd_state.other_window = atom;
                    if (arg1 & 0x1) == 0x1 {
                        state.xdnd_state.drag_type = xdnd_get_supported_atom(
                            &state.xcb_connection,
                            &state.atoms,
                            state.xdnd_state.other_window,
                        );
                    } else {
                        if let Some(atom) = [arg2, arg3, arg4]
                            .into_iter()
                            .find(|atom| xdnd_is_atom_supported(*atom, &state.atoms))
                        {
                            state.xdnd_state.drag_type = atom;
                        }
                    }
                } else if event.type_ == state.atoms.XdndLeave {
                    let position = state.xdnd_state.position;
                    drop(state);
                    window
                        .handle_input(PlatformInput::FileDrop(FileDropEvent::Pending { position }));
                    window.handle_input(PlatformInput::FileDrop(FileDropEvent::Exited {}));
                    self.0.borrow_mut().xdnd_state = Xdnd::default();
                } else if event.type_ == state.atoms.XdndPosition {
                    if let Ok(pos) = get_reply(
                        || "Failed to query pointer position",
                        state.xcb_connection.query_pointer(event.window),
                    ) {
                        state.xdnd_state.position =
                            Point::new(px(pos.win_x as f32), px(pos.win_y as f32));
                    }
                    if !state.xdnd_state.retrieved {
                        check_reply(
                            || "Failed to convert selection for drag and drop",
                            state.xcb_connection.convert_selection(
                                event.window,
                                state.atoms.XdndSelection,
                                state.xdnd_state.drag_type,
                                state.atoms.XDND_DATA,
                                arg3,
                            ),
                        )
                        .log_err();
                    }
                    xdnd_send_status(
                        &state.xcb_connection,
                        &state.atoms,
                        event.window,
                        state.xdnd_state.other_window,
                        arg4,
                    );
                    let position = state.xdnd_state.position;
                    drop(state);
                    window
                        .handle_input(PlatformInput::FileDrop(FileDropEvent::Pending { position }));
                } else if event.type_ == state.atoms.XdndDrop {
                    xdnd_send_finished(
                        &state.xcb_connection,
                        &state.atoms,
                        event.window,
                        state.xdnd_state.other_window,
                    );
                    let position = state.xdnd_state.position;
                    drop(state);
                    window
                        .handle_input(PlatformInput::FileDrop(FileDropEvent::Submit { position }));
                    self.0.borrow_mut().xdnd_state = Xdnd::default();
                }
            }
            Event::SelectionNotify(event) => {
                let window = self.get_window(event.requestor)?;
                let state = self.0.borrow_mut();
                let reply = get_reply(
                    || "Failed to get XDND_DATA",
                    state.xcb_connection.get_property(
                        false,
                        event.requestor,
                        state.atoms.XDND_DATA,
                        AtomEnum::ANY,
                        0,
                        1024,
                    ),
                )
                .log_err();
                let Some(reply) = reply else {
                    return Some(());
                };
                if let Ok(file_list) = str::from_utf8(&reply.value) {
                    let paths: SmallVec<[_; 2]> = file_list
                        .lines()
                        .filter_map(|path| Url::parse(path).log_err())
                        .filter_map(|url| url.to_file_path().log_err())
                        .collect();
                    let input = PlatformInput::FileDrop(FileDropEvent::Entered {
                        position: state.xdnd_state.position,
                        paths: inazuma::ExternalPaths(paths),
                    });
                    drop(state);
                    window.handle_input(input);
                    self.0.borrow_mut().xdnd_state.retrieved = true;
                }
            }
            Event::ConfigureNotify(event) => {
                let bounds = Bounds {
                    origin: Point {
                        x: event.x.into(),
                        y: event.y.into(),
                    },
                    size: Size {
                        width: event.width.into(),
                        height: event.height.into(),
                    },
                };
                let window = self.get_window(event.window)?;
                window
                    .set_bounds(bounds)
                    .context("X11: Failed to set window bounds")
                    .log_err();
            }
            Event::PropertyNotify(event) => {
                let window = self.get_window(event.window)?;
                window
                    .property_notify(event)
                    .context("X11: Failed to handle property notify")
                    .log_err();
            }
            Event::FocusIn(event) => {
                let window = self.get_window(event.event)?;
                window.set_active(true);
                let mut state = self.0.borrow_mut();
                state.keyboard_focused_window = Some(event.event);
                if let Some(handler) = state.xim_handler.as_mut() {
                    handler.window = event.event;
                }
                drop(state);
                self.enable_ime();
            }
            Event::FocusOut(event) => {
                let window = self.get_window(event.event)?;
                window.set_active(false);
                let mut state = self.0.borrow_mut();
                // Set last scroll values to `None` so that a large delta isn't created if scrolling is done outside the window (the valuator is global)
                reset_all_pointer_device_scroll_positions(&mut state.pointer_device_states);
                state.keyboard_focused_window = None;
                if let Some(compose_state) = state.compose_state.as_mut() {
                    compose_state.reset();
                }
                state.pre_edit_text.take();
                drop(state);
                self.reset_ime();
                window.handle_ime_delete();
            }
            Event::XkbNewKeyboardNotify(_) | Event::XkbMapNotify(_) => {
                let mut state = self.0.borrow_mut();
                let xkb_state = {
                    let xkb_keymap = xkbc::x11::keymap_new_from_device(
                        &state.xkb_context,
                        &state.xcb_connection,
                        state.xkb_device_id,
                        xkbc::KEYMAP_COMPILE_NO_FLAGS,
                    );
                    xkbc::x11::state_new_from_device(
                        &xkb_keymap,
                        &state.xcb_connection,
                        state.xkb_device_id,
                    )
                };
                state.xkb = xkb_state;
                drop(state);
                self.handle_keyboard_layout_change();
            }
            Event::XkbStateNotify(event) => {
                let mut state = self.0.borrow_mut();
                let old_layout = state.xkb.serialize_layout(STATE_LAYOUT_EFFECTIVE);
                let new_layout = u32::from(event.group);
                state.xkb.update_mask(
                    event.base_mods.into(),
                    event.latched_mods.into(),
                    event.locked_mods.into(),
                    event.base_group as u32,
                    event.latched_group as u32,
                    event.locked_group.into(),
                );
                let modifiers = modifiers_from_xkb(&state.xkb);
                let capslock = capslock_from_xkb(&state.xkb);
                if state.last_modifiers_changed_event == modifiers
                    && state.last_capslock_changed_event == capslock
                {
                    drop(state);
                } else {
                    let focused_window_id = state.keyboard_focused_window?;
                    state.modifiers = modifiers;
                    state.last_modifiers_changed_event = modifiers;
                    state.capslock = capslock;
                    state.last_capslock_changed_event = capslock;
                    drop(state);

                    let focused_window = self.get_window(focused_window_id)?;
                    focused_window.handle_input(PlatformInput::ModifiersChanged(
                        ModifiersChangedEvent {
                            modifiers,
                            capslock,
                        },
                    ));
                }

                if new_layout != old_layout {
                    self.handle_keyboard_layout_change();
                }
            }
            Event::KeyPress(event) => {
                let window = self.get_window(event.event)?;
                let mut state = self.0.borrow_mut();

                let modifiers = modifiers_from_state(event.state);
                state.modifiers = modifiers;
                state.pre_key_char_down.take();

                // Macros containing modifiers might result in
                // the modifiers missing from the event.
                // We therefore update the mask from the global state.
                update_xkb_mask_from_event_state(&mut state.xkb, event.state);

                let keystroke = {
                    let code = event.detail.into();
                    let mut keystroke = keystroke_from_xkb(&state.xkb, modifiers, code);
                    let keysym = state.xkb.key_get_one_sym(code);

                    if keysym.is_modifier_key() {
                        return Some(());
                    }

                    // should be called after key_get_one_sym
                    state.xkb.update_key(code, xkbc::KeyDirection::Down);

                    if let Some(mut compose_state) = state.compose_state.take() {
                        compose_state.feed(keysym);
                        match compose_state.status() {
                            xkbc::Status::Composed => {
                                state.pre_edit_text.take();
                                keystroke.key_char = compose_state.utf8();
                                if let Some(keysym) = compose_state.keysym() {
                                    keystroke.key = xkbc::keysym_get_name(keysym);
                                }
                            }
                            xkbc::Status::Composing => {
                                keystroke.key_char = None;
                                state.pre_edit_text = compose_state
                                    .utf8()
                                    .or(keystroke_underlying_dead_key(keysym));
                                let pre_edit =
                                    state.pre_edit_text.clone().unwrap_or(String::default());
                                drop(state);
                                window.handle_ime_preedit(pre_edit);
                                state = self.0.borrow_mut();
                            }
                            xkbc::Status::Cancelled => {
                                let pre_edit = state.pre_edit_text.take();
                                drop(state);
                                if let Some(pre_edit) = pre_edit {
                                    window.handle_ime_commit(pre_edit);
                                }
                                if let Some(current_key) = keystroke_underlying_dead_key(keysym) {
                                    window.handle_ime_preedit(current_key);
                                }
                                state = self.0.borrow_mut();
                                compose_state.feed(keysym);
                            }
                            _ => {}
                        }
                        state.compose_state = Some(compose_state);
                    }
                    keystroke
                };
                drop(state);
                window.handle_input(PlatformInput::KeyDown(inazuma::KeyDownEvent {
                    keystroke,
                    is_held: false,
                    prefer_character_input: false,
                }));
            }
            Event::KeyRelease(event) => {
                let window = self.get_window(event.event)?;
                let mut state = self.0.borrow_mut();

                let modifiers = modifiers_from_state(event.state);
                state.modifiers = modifiers;

                // Macros containing modifiers might result in
                // the modifiers missing from the event.
                // We therefore update the mask from the global state.
                update_xkb_mask_from_event_state(&mut state.xkb, event.state);

                let keystroke = {
                    let code = event.detail.into();
                    let keystroke = keystroke_from_xkb(&state.xkb, modifiers, code);
                    let keysym = state.xkb.key_get_one_sym(code);

                    if keysym.is_modifier_key() {
                        return Some(());
                    }

                    // should be called after key_get_one_sym
                    state.xkb.update_key(code, xkbc::KeyDirection::Up);

                    keystroke
                };
                drop(state);
                window.handle_input(PlatformInput::KeyUp(inazuma::KeyUpEvent { keystroke }));
            }
            Event::XinputButtonPress(event) => {
                let window = self.get_window(event.event)?;
                let mut state = self.0.borrow_mut();

                let modifiers = modifiers_from_xinput_info(event.mods);
                state.modifiers = modifiers;

                let position = point(
                    px(event.event_x as f32 / u16::MAX as f32 / state.scale_factor),
                    px(event.event_y as f32 / u16::MAX as f32 / state.scale_factor),
                );

                if state.composing && state.ximc.is_some() {
                    drop(state);
                    self.reset_ime();
                    window.handle_ime_unmark();
                    state = self.0.borrow_mut();
                } else if let Some(text) = state.pre_edit_text.take() {
                    if let Some(compose_state) = state.compose_state.as_mut() {
                        compose_state.reset();
                    }
                    drop(state);
                    window.handle_ime_commit(text);
                    state = self.0.borrow_mut();
                }
                match button_or_scroll_from_event_detail(event.detail) {
                    Some(ButtonOrScroll::Button(button)) => {
                        let click_elapsed = state.last_click.elapsed();
                        if click_elapsed < DOUBLE_CLICK_INTERVAL
                            && state
                                .last_mouse_button
                                .is_some_and(|prev_button| prev_button == button)
                            && is_within_click_distance(state.last_location, position)
                        {
                            state.current_count += 1;
                        } else {
                            state.current_count = 1;
                        }

                        state.last_click = Instant::now();
                        state.last_mouse_button = Some(button);
                        state.last_location = position;
                        let current_count = state.current_count;

                        drop(state);
                        window.handle_input(PlatformInput::MouseDown(inazuma::MouseDownEvent {
                            button,
                            position,
                            modifiers,
                            click_count: current_count,
                            first_mouse: false,
                        }));
                    }
                    Some(ButtonOrScroll::Scroll(direction)) => {
                        drop(state);
                        // Emulated scroll button presses are sent simultaneously with smooth scrolling XinputMotion events.
                        // Since handling those events does the scrolling, they are skipped here.
                        if !event
                            .flags
                            .contains(xinput::PointerEventFlags::POINTER_EMULATED)
                        {
                            let scroll_delta = match direction {
                                ScrollDirection::Up => Point::new(0.0, SCROLL_LINES),
                                ScrollDirection::Down => Point::new(0.0, -SCROLL_LINES),
                                ScrollDirection::Left => Point::new(SCROLL_LINES, 0.0),
                                ScrollDirection::Right => Point::new(-SCROLL_LINES, 0.0),
                            };
                            window.handle_input(PlatformInput::ScrollWheel(
                                make_scroll_wheel_event(position, scroll_delta, modifiers),
                            ));
                        }
                    }
                    None => {
                        log::error!("Unknown x11 button: {}", event.detail);
                    }
                }
            }
            Event::XinputButtonRelease(event) => {
                let window = self.get_window(event.event)?;
                let mut state = self.0.borrow_mut();
                let modifiers = modifiers_from_xinput_info(event.mods);
                state.modifiers = modifiers;

                let position = point(
                    px(event.event_x as f32 / u16::MAX as f32 / state.scale_factor),
                    px(event.event_y as f32 / u16::MAX as f32 / state.scale_factor),
                );
                match button_or_scroll_from_event_detail(event.detail) {
                    Some(ButtonOrScroll::Button(button)) => {
                        let click_count = state.current_count;
                        drop(state);
                        window.handle_input(PlatformInput::MouseUp(inazuma::MouseUpEvent {
                            button,
                            position,
                            modifiers,
                            click_count,
                        }));
                    }
                    Some(ButtonOrScroll::Scroll(_)) => {}
                    None => {}
                }
            }
            Event::XinputMotion(event) => {
                let window = self.get_window(event.event)?;
                let mut state = self.0.borrow_mut();
                if window.is_blocked() {
                    // We want to set the cursor to the default arrow
                    // when the window is blocked
                    let style = CursorStyle::Arrow;

                    let current_style = state
                        .cursor_styles
                        .get(&window.x_window)
                        .unwrap_or(&CursorStyle::Arrow);
                    if *current_style != style
                        && let Some(cursor) = state.get_cursor_icon(style)
                    {
                        state.cursor_styles.insert(window.x_window, style);
                        check_reply(
                            || "Failed to set cursor style",
                            state.xcb_connection.change_window_attributes(
                                window.x_window,
                                &ChangeWindowAttributesAux {
                                    cursor: Some(cursor),
                                    ..Default::default()
                                },
                            ),
                        )
                        .log_err();
                        state.xcb_connection.flush().log_err();
                    };
                }
                let pressed_button = pressed_button_from_mask(event.button_mask[0]);
                let position = point(
                    px(event.event_x as f32 / u16::MAX as f32 / state.scale_factor),
                    px(event.event_y as f32 / u16::MAX as f32 / state.scale_factor),
                );
                let modifiers = modifiers_from_xinput_info(event.mods);
                state.modifiers = modifiers;
                drop(state);

                if event.valuator_mask[0] & 3 != 0 {
                    window.handle_input(PlatformInput::MouseMove(inazuma::MouseMoveEvent {
                        position,
                        pressed_button,
                        modifiers,
                    }));
                }

                state = self.0.borrow_mut();
                if let Some(pointer) = state.pointer_device_states.get_mut(&event.sourceid) {
                    let scroll_delta = get_scroll_delta_and_update_state(pointer, &event);
                    drop(state);
                    if let Some(scroll_delta) = scroll_delta {
                        window.handle_input(PlatformInput::ScrollWheel(make_scroll_wheel_event(
                            position,
                            scroll_delta,
                            modifiers,
                        )));
                    }
                }
            }
            Event::XinputEnter(event) if event.mode == xinput::NotifyMode::NORMAL => {
                let window = self.get_window(event.event)?;
                window.set_hovered(true);
                let mut state = self.0.borrow_mut();
                state.mouse_focused_window = Some(event.event);
            }
            Event::XinputLeave(event) if event.mode == xinput::NotifyMode::NORMAL => {
                let mut state = self.0.borrow_mut();

                // Set last scroll values to `None` so that a large delta isn't created if scrolling is done outside the window (the valuator is global)
                reset_all_pointer_device_scroll_positions(&mut state.pointer_device_states);
                state.mouse_focused_window = None;
                let pressed_button = pressed_button_from_mask(event.buttons[0]);
                let position = point(
                    px(event.event_x as f32 / u16::MAX as f32 / state.scale_factor),
                    px(event.event_y as f32 / u16::MAX as f32 / state.scale_factor),
                );
                let modifiers = modifiers_from_xinput_info(event.mods);
                state.modifiers = modifiers;
                drop(state);

                let window = self.get_window(event.event)?;
                window.handle_input(PlatformInput::MouseExited(inazuma::MouseExitEvent {
                    pressed_button,
                    position,
                    modifiers,
                }));
                window.set_hovered(false);
            }
            Event::XinputHierarchy(event) => {
                let mut state = self.0.borrow_mut();
                // Temporarily use `state.pointer_device_states` to only store pointers that still have valid scroll values.
                // Any change to a device invalidates its scroll values.
                for info in event.infos {
                    if is_pointer_device(info.type_) {
                        state.pointer_device_states.remove(&info.deviceid);
                    }
                }
                if let Some(pointer_device_states) = current_pointer_device_states(
                    &state.xcb_connection,
                    &state.pointer_device_states,
                ) {
                    state.pointer_device_states = pointer_device_states;
                }
            }
            Event::XinputDeviceChanged(event) => {
                let mut state = self.0.borrow_mut();
                if let Some(pointer) = state.pointer_device_states.get_mut(&event.sourceid) {
                    reset_pointer_device_scroll_positions(pointer);
                }
            }
            _ => {}
        };

        Some(())
    }

    fn handle_xim_callback_event(&self, event: XimCallbackEvent) {
        match event {
            XimCallbackEvent::XimXEvent(event) => {
                self.handle_event(event);
            }
            XimCallbackEvent::XimCommitEvent(window, text) => {
                self.xim_handle_commit(window, text);
            }
            XimCallbackEvent::XimPreeditEvent(window, text) => {
                self.xim_handle_preedit(window, text);
            }
        };
    }

    fn xim_handle_event(&self, event: Event) -> Option<()> {
        match event {
            Event::KeyPress(event) | Event::KeyRelease(event) => {
                let mut state = self.0.borrow_mut();
                state.pre_key_char_down = Some(keystroke_from_xkb(
                    &state.xkb,
                    state.modifiers,
                    event.detail.into(),
                ));
                let (mut ximc, mut xim_handler) = state.take_xim()?;
                drop(state);
                xim_handler.window = event.event;
                ximc.forward_event(
                    xim_handler.im_id,
                    xim_handler.ic_id,
                    xim::ForwardEventFlag::empty(),
                    &event,
                )
                .context("X11: Failed to forward XIM event")
                .log_err();
                let mut state = self.0.borrow_mut();
                state.restore_xim(ximc, xim_handler);
                drop(state);
            }
            event => {
                self.handle_event(event);
            }
        }
        Some(())
    }

    fn xim_handle_commit(&self, window: xproto::Window, text: String) -> Option<()> {
        let Some(window) = self.get_window(window) else {
            log::error!("bug: Failed to get window for XIM commit");
            return None;
        };
        let mut state = self.0.borrow_mut();
        state.composing = false;
        drop(state);
        window.handle_ime_commit(text);
        Some(())
    }

    fn xim_handle_preedit(&self, window: xproto::Window, text: String) -> Option<()> {
        let Some(window) = self.get_window(window) else {
            log::error!("bug: Failed to get window for XIM preedit");
            return None;
        };

        let mut state = self.0.borrow_mut();
        let (mut ximc, xim_handler) = state.take_xim()?;
        state.composing = !text.is_empty();
        drop(state);
        window.handle_ime_preedit(text);

        if let Some(scaled_area) = window.get_ime_area() {
            let ic_attributes = ximc
                .build_ic_attributes()
                .push(
                    xim::AttributeName::InputStyle,
                    xim::InputStyle::PREEDIT_CALLBACKS,
                )
                .push(xim::AttributeName::ClientWindow, xim_handler.window)
                .push(xim::AttributeName::FocusWindow, xim_handler.window)
                .nested_list(xim::AttributeName::PreeditAttributes, |b| {
                    b.push(
                        xim::AttributeName::SpotLocation,
                        xim::Point {
                            x: u32::from(scaled_area.origin.x + scaled_area.size.width) as i16,
                            y: u32::from(scaled_area.origin.y + scaled_area.size.height) as i16,
                        },
                    );
                })
                .build();
            ximc.set_ic_values(xim_handler.im_id, xim_handler.ic_id, ic_attributes)
                .ok();
        }
        let mut state = self.0.borrow_mut();
        state.restore_xim(ximc, xim_handler);
        drop(state);
        Some(())
    }

    fn handle_keyboard_layout_change(&self) {
        let mut state = self.0.borrow_mut();
        let layout_idx = state.xkb.serialize_layout(STATE_LAYOUT_EFFECTIVE);
        let keymap = state.xkb.get_keymap();
        let layout_name = keymap.layout_get_name(layout_idx);
        if layout_name != state.keyboard_layout.name() {
            state.keyboard_layout = LinuxKeyboardLayout::new(layout_name.to_string().into());
            if let Some(mut callback) = state.common.callbacks.keyboard_layout_change.take() {
                drop(state);
                callback();
                state = self.0.borrow_mut();
                state.common.callbacks.keyboard_layout_change = Some(callback);
            }
        }
    }
}

