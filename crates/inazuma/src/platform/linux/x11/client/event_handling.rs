use super::*;

impl X11Client {
    pub(crate) fn new() -> anyhow::Result<Self> {
        let event_loop = EventLoop::try_new()?;

        let (common, main_receiver) = LinuxCommon::new(event_loop.get_signal());

        let handle = event_loop.handle();

        handle
            .insert_source(main_receiver, {
                let handle = handle.clone();
                move |event, _, _: &mut X11Client| {
                    if let calloop::channel::Event::Msg(runnable) = event {
                        // Insert the runnables as idle callbacks, so we make sure that user-input and X11
                        // events have higher priority and runnables are only worked off after the event
                        // callbacks.
                        handle.insert_idle(|_| {
                            let start = Instant::now();
                            let location = runnable.metadata().location;
                            let mut timing = TaskTiming {
                                location,
                                start,
                                end: None,
                            };
                            profiler::add_task_timing(timing);

                            runnable.run();

                            let end = Instant::now();
                            timing.end = Some(end);
                            profiler::add_task_timing(timing);
                        });
                    }
                }
            })
            .map_err(|err| {
                anyhow!("Failed to initialize event loop handling of foreground tasks: {err:?}")
            })?;

        let (xcb_connection, x_root_index) = XCBConnection::connect(None)?;
        xcb_connection.prefetch_extension_information(xkb::X11_EXTENSION_NAME)?;
        xcb_connection.prefetch_extension_information(randr::X11_EXTENSION_NAME)?;
        xcb_connection.prefetch_extension_information(render::X11_EXTENSION_NAME)?;
        xcb_connection.prefetch_extension_information(xinput::X11_EXTENSION_NAME)?;

        // Announce to X server that XInput up to 2.1 is supported. To increase this to 2.2 and
        // beyond, support for touch events would need to be added.
        let xinput_version = get_reply(
            || "XInput XiQueryVersion failed",
            xcb_connection.xinput_xi_query_version(2, 1),
        )?;
        assert!(
            xinput_version.major_version >= 2,
            "XInput version >= 2 required."
        );

        let pointer_device_states =
            current_pointer_device_states(&xcb_connection, &BTreeMap::new()).unwrap_or_default();

        let atoms = XcbAtoms::new(&xcb_connection)
            .context("Failed to get XCB atoms")?
            .reply()
            .context("Failed to get XCB atoms")?;

        let root = xcb_connection.setup().roots[0].root;
        let compositor_present = check_compositor_present(&xcb_connection, root);
        let gtk_frame_extents_supported =
            check_gtk_frame_extents_supported(&xcb_connection, &atoms, root);
        let client_side_decorations_supported = compositor_present && gtk_frame_extents_supported;
        log::info!(
            "x11: compositor present: {}, gtk_frame_extents_supported: {}",
            compositor_present,
            gtk_frame_extents_supported
        );

        let xkb = get_reply(
            || "Failed to initialize XKB extension",
            xcb_connection
                .xkb_use_extension(XKB_X11_MIN_MAJOR_XKB_VERSION, XKB_X11_MIN_MINOR_XKB_VERSION),
        )?;
        assert!(xkb.supported);

        let events = xkb::EventType::STATE_NOTIFY
            | xkb::EventType::MAP_NOTIFY
            | xkb::EventType::NEW_KEYBOARD_NOTIFY;
        let map_notify_parts = xkb::MapPart::KEY_TYPES
            | xkb::MapPart::KEY_SYMS
            | xkb::MapPart::MODIFIER_MAP
            | xkb::MapPart::EXPLICIT_COMPONENTS
            | xkb::MapPart::KEY_ACTIONS
            | xkb::MapPart::KEY_BEHAVIORS
            | xkb::MapPart::VIRTUAL_MODS
            | xkb::MapPart::VIRTUAL_MOD_MAP;
        check_reply(
            || "Failed to select XKB events",
            xcb_connection.xkb_select_events(
                xkb::ID::USE_CORE_KBD.into(),
                0u8.into(),
                events,
                map_notify_parts,
                map_notify_parts,
                &xkb::SelectEventsAux::new(),
            ),
        )?;

        let xkb_context = xkbc::Context::new(xkbc::CONTEXT_NO_FLAGS);
        let xkb_device_id = xkbc::x11::get_core_keyboard_device_id(&xcb_connection);
        let xkb_state = {
            let xkb_keymap = xkbc::x11::keymap_new_from_device(
                &xkb_context,
                &xcb_connection,
                xkb_device_id,
                xkbc::KEYMAP_COMPILE_NO_FLAGS,
            );
            xkbc::x11::state_new_from_device(&xkb_keymap, &xcb_connection, xkb_device_id)
        };
        let compose_state = get_xkb_compose_state(&xkb_context);
        let layout_idx = xkb_state.serialize_layout(STATE_LAYOUT_EFFECTIVE);
        let layout_name = xkb_state
            .get_keymap()
            .layout_get_name(layout_idx)
            .to_string();
        let keyboard_layout = LinuxKeyboardLayout::new(layout_name.into());

        let resource_database = x11rb::resource_manager::new_from_default(&xcb_connection)
            .context("Failed to create resource database")?;
        let scale_factor = get_scale_factor(&xcb_connection, &resource_database, x_root_index);
        let cursor_handle = cursor::Handle::new(&xcb_connection, x_root_index, &resource_database)
            .context("Failed to initialize cursor theme handler")?
            .reply()
            .context("Failed to initialize cursor theme handler")?;

        let clipboard = Clipboard::new().context("Failed to initialize clipboard")?;

        let screen = &xcb_connection.setup().roots[x_root_index];
        let compositor_gpu = detect_compositor_gpu(&xcb_connection, screen);

        let xcb_connection = Rc::new(xcb_connection);

        let ximc = X11rbClient::init(Rc::clone(&xcb_connection), x_root_index, None).ok();
        let xim_handler = if ximc.is_some() {
            Some(XimHandler::new())
        } else {
            None
        };

        // Safety: Safe if xcb::Connection always returns a valid fd
        let fd = unsafe { FdWrapper::new(Rc::clone(&xcb_connection)) };

        handle
            .insert_source(
                Generic::new_with_error::<EventHandlerError>(
                    fd,
                    calloop::Interest::READ,
                    calloop::Mode::Level,
                ),
                {
                    let xcb_connection = xcb_connection.clone();
                    move |_readiness, _, client| {
                        client.process_x11_events(&xcb_connection)?;
                        Ok(calloop::PostAction::Continue)
                    }
                },
            )
            .map_err(|err| anyhow!("Failed to initialize X11 event source: {err:?}"))?;

        handle
            .insert_source(XDPEventSource::new(&common.background_executor), {
                move |event, _, client| match event {
                    XDPEvent::WindowAppearance(appearance) => {
                        client.with_common(|common| common.appearance = appearance);
                        for window in client.0.borrow_mut().windows.values_mut() {
                            window.window.set_appearance(appearance);
                        }
                    }
                    XDPEvent::CursorTheme(_) | XDPEvent::CursorSize(_) => {
                        // noop, X11 manages this for us.
                    }
                }
            })
            .map_err(|err| anyhow!("Failed to initialize XDP event source: {err:?}"))?;

        xcb_flush(&xcb_connection);

        Ok(X11Client(Rc::new(RefCell::new(X11ClientState {
            modifiers: Modifiers::default(),
            capslock: Capslock::default(),
            last_modifiers_changed_event: Modifiers::default(),
            last_capslock_changed_event: Capslock::default(),
            event_loop: Some(event_loop),
            loop_handle: handle,
            common,
            last_click: Instant::now(),
            last_mouse_button: None,
            last_location: Point::new(px(0.0), px(0.0)),
            current_count: 0,
            gpu_context: Rc::new(RefCell::new(None)),
            compositor_gpu,
            scale_factor,

            xkb_context,
            xcb_connection,
            xkb_device_id,
            client_side_decorations_supported,
            x_root_index,
            _resource_database: resource_database,
            atoms,
            windows: HashMap::default(),
            mouse_focused_window: None,
            keyboard_focused_window: None,
            xkb: xkb_state,
            keyboard_layout,
            ximc,
            xim_handler,

            compose_state,
            pre_edit_text: None,
            pre_key_char_down: None,
            composing: false,

            cursor_handle,
            cursor_styles: HashMap::default(),
            cursor_cache: HashMap::default(),

            pointer_device_states,

            clipboard,
            clipboard_item: None,
            xdnd_state: Xdnd::default(),
        }))))
    }

    pub fn process_x11_events(
        &self,
        xcb_connection: &XCBConnection,
    ) -> Result<(), EventHandlerError> {
        loop {
            let mut events = Vec::new();
            let mut windows_to_refresh = HashSet::new();

            let mut last_key_release = None;

            // event handlers for new keyboard / remapping refresh the state without using event
            // details, this deduplicates them.
            let mut last_keymap_change_event: Option<Event> = None;

            loop {
                match xcb_connection.poll_for_event() {
                    Ok(Some(event)) => {
                        match event {
                            Event::Expose(expose_event) => {
                                windows_to_refresh.insert(expose_event.window);
                            }
                            Event::KeyRelease(_) => {
                                if let Some(last_keymap_change_event) =
                                    last_keymap_change_event.take()
                                {
                                    if let Some(last_key_release) = last_key_release.take() {
                                        events.push(last_key_release);
                                    }
                                    events.push(last_keymap_change_event);
                                }

                                last_key_release = Some(event);
                            }
                            Event::KeyPress(key_press) => {
                                if let Some(last_keymap_change_event) =
                                    last_keymap_change_event.take()
                                {
                                    if let Some(last_key_release) = last_key_release.take() {
                                        events.push(last_key_release);
                                    }
                                    events.push(last_keymap_change_event);
                                }

                                if let Some(Event::KeyRelease(key_release)) =
                                    last_key_release.take()
                                {
                                    // We ignore that last KeyRelease if it's too close to this KeyPress,
                                    // suggesting that it's auto-generated by X11 as a key-repeat event.
                                    if key_release.detail != key_press.detail
                                        || key_press.time.saturating_sub(key_release.time) > 20
                                    {
                                        events.push(Event::KeyRelease(key_release));
                                    }
                                }
                                events.push(Event::KeyPress(key_press));
                            }
                            Event::XkbNewKeyboardNotify(_) | Event::XkbMapNotify(_) => {
                                if let Some(release_event) = last_key_release.take() {
                                    events.push(release_event);
                                }
                                last_keymap_change_event = Some(event);
                            }
                            _ => {
                                if let Some(release_event) = last_key_release.take() {
                                    events.push(release_event);
                                }
                                events.push(event);
                            }
                        }
                    }
                    Ok(None) => {
                        break;
                    }
                    Err(err @ ConnectionError::IoError(..)) => {
                        return Err(EventHandlerError::from(err));
                    }
                    Err(err) => {
                        let err = handle_connection_error(err);
                        log::warn!("error while polling for X11 events: {err:?}");
                        break;
                    }
                }
            }

            if let Some(release_event) = last_key_release.take() {
                events.push(release_event);
            }
            if let Some(keymap_change_event) = last_keymap_change_event.take() {
                events.push(keymap_change_event);
            }

            if events.is_empty() && windows_to_refresh.is_empty() {
                break;
            }

            for window in windows_to_refresh.into_iter() {
                let mut state = self.0.borrow_mut();
                if let Some(window) = state.windows.get_mut(&window) {
                    window.expose_event_received = true;
                }
            }

            for event in events.into_iter() {
                let mut state = self.0.borrow_mut();
                if !state.has_xim() {
                    drop(state);
                    self.handle_event(event);
                    continue;
                }

                let Some((mut ximc, mut xim_handler)) = state.take_xim() else {
                    continue;
                };
                let xim_connected = xim_handler.connected;
                drop(state);

                let xim_filtered = ximc.filter_event(&event, &mut xim_handler);
                let xim_callback_event = xim_handler.last_callback_event.take();

                let mut state = self.0.borrow_mut();
                state.restore_xim(ximc, xim_handler);
                drop(state);

                if let Some(event) = xim_callback_event {
                    self.handle_xim_callback_event(event);
                }

                match xim_filtered {
                    Ok(handled) => {
                        if handled {
                            continue;
                        }
                        if xim_connected {
                            self.xim_handle_event(event);
                        } else {
                            self.handle_event(event);
                        }
                    }
                    Err(err) => {
                        // this might happen when xim server crashes on one of the events
                        // we do lose 1-2 keys when crash happens since there is no reliable way to get that info
                        // luckily, x11 sends us window not found error when xim server crashes upon further key press
                        // hence we fall back to handle_event
                        log::error!("XIMClientError: {}", err);
                        let mut state = self.0.borrow_mut();
                        state.take_xim();
                        drop(state);
                        self.handle_event(event);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn enable_ime(&self) {
        let mut state = self.0.borrow_mut();
        if !state.has_xim() {
            return;
        }

        let Some((mut ximc, xim_handler)) = state.take_xim() else {
            return;
        };
        let mut ic_attributes = ximc
            .build_ic_attributes()
            .push(AttributeName::InputStyle, InputStyle::PREEDIT_CALLBACKS)
            .push(AttributeName::ClientWindow, xim_handler.window)
            .push(AttributeName::FocusWindow, xim_handler.window);

        let window_id = state.keyboard_focused_window;
        drop(state);
        if let Some(window_id) = window_id {
            let Some(window) = self.get_window(window_id) else {
                log::error!("Failed to get window for IME positioning");
                let mut state = self.0.borrow_mut();
                state.ximc = Some(ximc);
                state.xim_handler = Some(xim_handler);
                return;
            };
            if let Some(scaled_area) = window.get_ime_area() {
                ic_attributes =
                    ic_attributes.nested_list(xim::AttributeName::PreeditAttributes, |b| {
                        b.push(
                            xim::AttributeName::SpotLocation,
                            xim::Point {
                                x: u32::from(scaled_area.origin.x + scaled_area.size.width) as i16,
                                y: u32::from(scaled_area.origin.y + scaled_area.size.height) as i16,
                            },
                        );
                    });
            }
        }
        ximc.create_ic(xim_handler.im_id, ic_attributes.build())
            .ok();
        let mut state = self.0.borrow_mut();
        state.restore_xim(ximc, xim_handler);
    }

    pub fn reset_ime(&self) {
        let mut state = self.0.borrow_mut();
        state.composing = false;
        if let Some(mut ximc) = state.ximc.take() {
            if let Some(xim_handler) = state.xim_handler.as_ref() {
                ximc.reset_ic(xim_handler.im_id, xim_handler.ic_id).ok();
            } else {
                log::error!("bug: xim handler not set in reset_ime");
            }
            state.ximc = Some(ximc);
        }
    }

    pub(crate) fn get_window(&self, win: xproto::Window) -> Option<X11WindowStatePtr> {
        let state = self.0.borrow();
        state
            .windows
            .get(&win)
            .filter(|window_reference| !window_reference.window.state.borrow().destroyed)
            .map(|window_reference| window_reference.window.clone())
    }

