use super::*;

impl WaylandClient {
    pub(crate) fn new() -> Self {
        let conn = Connection::connect_to_env().unwrap();

        let (globals, event_queue) = registry_queue_init::<WaylandClientStatePtr>(&conn).unwrap();
        let qh = event_queue.handle();

        let mut seat: Option<wl_seat::WlSeat> = None;
        #[allow(clippy::mutable_key_type)]
        let mut in_progress_outputs = HashMap::default();
        #[allow(clippy::mutable_key_type)]
        let mut wl_outputs: HashMap<ObjectId, wl_output::WlOutput> = HashMap::default();
        globals.contents().with_list(|list| {
            for global in list {
                match &global.interface[..] {
                    "wl_seat" => {
                        seat = Some(globals.registry().bind::<wl_seat::WlSeat, _, _>(
                            global.name,
                            wl_seat_version(global.version),
                            &qh,
                            (),
                        ));
                    }
                    "wl_output" => {
                        let output = globals.registry().bind::<wl_output::WlOutput, _, _>(
                            global.name,
                            wl_output_version(global.version),
                            &qh,
                            (),
                        );
                        in_progress_outputs.insert(output.id(), InProgressOutput::default());
                        wl_outputs.insert(output.id(), output);
                    }
                    _ => {}
                }
            }
        });

        let event_loop = EventLoop::<WaylandClientStatePtr>::try_new().unwrap();

        let (common, main_receiver) = LinuxCommon::new(event_loop.get_signal());

        let handle = event_loop.handle();
        handle
            .insert_source(main_receiver, {
                let handle = handle.clone();
                move |event, _, _: &mut WaylandClientStatePtr| {
                    if let calloop::channel::Event::Msg(runnable) = event {
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
            .unwrap();

        let compositor_gpu = detect_compositor_gpu();
        let gpu_context = Rc::new(RefCell::new(None));

        let seat = seat.unwrap();
        let globals = Globals::new(
            globals,
            common.foreground_executor.clone(),
            qh.clone(),
            seat.clone(),
        );

        let data_device = globals
            .data_device_manager
            .as_ref()
            .map(|data_device_manager| data_device_manager.get_data_device(&seat, &qh, ()));

        let primary_selection = globals
            .primary_selection_manager
            .as_ref()
            .map(|primary_selection_manager| primary_selection_manager.get_device(&seat, &qh, ()));

        let cursor = Cursor::new(&conn, &globals, 24);

        handle
            .insert_source(XDPEventSource::new(&common.background_executor), {
                move |event, _, client| match event {
                    XDPEvent::WindowAppearance(appearance) => {
                        if let Some(client) = client.0.upgrade() {
                            let mut client = client.borrow_mut();

                            client.common.appearance = appearance;

                            for window in client.windows.values_mut() {
                                window.set_appearance(appearance);
                            }
                        }
                    }
                    XDPEvent::CursorTheme(theme) => {
                        if let Some(client) = client.0.upgrade() {
                            let mut client = client.borrow_mut();
                            client.cursor.set_theme(theme);
                        }
                    }
                    XDPEvent::CursorSize(size) => {
                        if let Some(client) = client.0.upgrade() {
                            let mut client = client.borrow_mut();
                            client.cursor.set_size(size);
                        }
                    }
                }
            })
            .unwrap();

        let state = Rc::new(RefCell::new(WaylandClientState {
            serial_tracker: SerialTracker::new(),
            globals,
            gpu_context,
            compositor_gpu,
            wl_seat: seat,
            wl_pointer: None,
            wl_keyboard: None,
            pinch_gesture: None,
            pinch_scale: 1.0,
            cursor_shape_device: None,
            data_device,
            primary_selection,
            text_input: None,
            pre_edit_text: None,
            ime_pre_edit: None,
            composing: false,
            outputs: HashMap::default(),
            in_progress_outputs,
            wl_outputs,
            windows: HashMap::default(),
            common,
            keyboard_layout: LinuxKeyboardLayout::new(UNKNOWN_KEYBOARD_LAYOUT_NAME),
            keymap_state: None,
            compose_state: None,
            drag: DragState {
                data_offer: None,
                window: None,
                position: Point::default(),
            },
            click: ClickState {
                last_click: Instant::now(),
                last_mouse_button: None,
                last_location: Point::default(),
                current_count: 0,
            },
            repeat: KeyRepeat {
                characters_per_second: 16,
                delay: Duration::from_millis(500),
                current_id: 0,
                current_keycode: None,
            },
            modifiers: Modifiers {
                shift: false,
                control: false,
                alt: false,
                function: false,
                platform: false,
            },
            capslock: Capslock { on: false },
            scroll_event_received: false,
            axis_source: AxisSource::Wheel,
            mouse_location: None,
            continuous_scroll_delta: None,
            discrete_scroll_delta: None,
            vertical_modifier: -1.0,
            horizontal_modifier: -1.0,
            button_pressed: None,
            mouse_focused_window: None,
            keyboard_focused_window: None,
            loop_handle: handle.clone(),
            enter_token: None,
            cursor_style: None,
            clipboard: Clipboard::new(conn.clone(), handle.clone()),
            data_offers: Vec::new(),
            primary_data_offer: None,
            cursor,
            pending_activation: None,
            event_loop: Some(event_loop),
        }));

        WaylandSource::new(conn, event_queue)
            .insert(handle)
            .unwrap();

        Self(state)
    }
}

impl LinuxClient for WaylandClient {
    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        Box::new(self.0.borrow().keyboard_layout.clone())
    }

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        self.0
            .borrow()
            .outputs
            .iter()
            .map(|(id, output)| {
                Rc::new(WaylandDisplay {
                    id: id.clone(),
                    name: output.name.clone(),
                    bounds: output.bounds.to_pixels(output.scale as f32),
                }) as Rc<dyn PlatformDisplay>
            })
            .collect()
    }

    fn display(&self, id: DisplayId) -> Option<Rc<dyn PlatformDisplay>> {
        self.0
            .borrow()
            .outputs
            .iter()
            .find_map(|(object_id, output)| {
                (object_id.protocol_id() == u32::from(id)).then(|| {
                    Rc::new(WaylandDisplay {
                        id: object_id.clone(),
                        name: output.name.clone(),
                        bounds: output.bounds.to_pixels(output.scale as f32),
                    }) as Rc<dyn PlatformDisplay>
                })
            })
    }

    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        None
    }

    #[cfg(feature = "screen-capture")]
    fn screen_capture_sources(
        &self,
    ) -> futures::channel::oneshot::Receiver<anyhow::Result<Vec<Rc<dyn inazuma::ScreenCaptureSource>>>>
    {
        // TODO: Get screen capture working on wayland. Be sure to try window resizing as that may
        // be tricky.
        //
        // start_scap_default_target_source()
        let (sources_tx, sources_rx) = futures::channel::oneshot::channel();
        sources_tx
            .send(Err(anyhow::anyhow!(
                "Wayland screen capture not yet implemented."
            )))
            .ok();
        sources_rx
    }

    fn open_window(
        &self,
        handle: AnyWindowHandle,
        params: WindowParams,
    ) -> anyhow::Result<Box<dyn PlatformWindow>> {
        let mut state = self.0.borrow_mut();

        let parent = state.keyboard_focused_window.clone();

        let target_output = params.display_id.and_then(|display_id| {
            let target_protocol_id: u32 = display_id.into();
            state
                .wl_outputs
                .iter()
                .find(|(id, _)| id.protocol_id() == target_protocol_id)
                .map(|(_, output)| output.clone())
        });

        let appearance = state.common.appearance;
        let compositor_gpu = state.compositor_gpu.take();
        let (window, surface_id) = WaylandWindow::new(
            handle,
            state.globals.clone(),
            state.gpu_context.clone(),
            compositor_gpu,
            WaylandClientStatePtr(Rc::downgrade(&self.0)),
            params,
            appearance,
            parent,
            target_output,
        )?;
        state.windows.insert(surface_id, window.0.clone());

        Ok(Box::new(window))
    }

    fn set_cursor_style(&self, style: CursorStyle) {
        let mut state = self.0.borrow_mut();

        let need_update = state.cursor_style != Some(style)
            && (state.mouse_focused_window.is_none()
                || state
                    .mouse_focused_window
                    .as_ref()
                    .is_some_and(|w| !w.is_blocked()));

        if need_update {
            let serial = state.serial_tracker.get(SerialKind::MouseEnter);
            state.cursor_style = Some(style);

            if let CursorStyle::None = style {
                let wl_pointer = state
                    .wl_pointer
                    .clone()
                    .expect("window is focused by pointer");
                wl_pointer.set_cursor(serial, None, 0, 0);
            } else if let Some(cursor_shape_device) = &state.cursor_shape_device {
                cursor_shape_device.set_shape(serial, to_shape(style));
            } else if let Some(focused_window) = &state.mouse_focused_window {
                // cursor-shape-v1 isn't supported, set the cursor using a surface.
                let wl_pointer = state
                    .wl_pointer
                    .clone()
                    .expect("window is focused by pointer");
                let scale = focused_window.primary_output_scale();
                state.cursor.set_icon(
                    &wl_pointer,
                    serial,
                    cursor_style_to_icon_names(style),
                    scale,
                );
            }
        }
    }

    fn open_uri(&self, uri: &str) {
        let mut state = self.0.borrow_mut();
        if let (Some(activation), Some(window)) = (
            state.globals.activation.clone(),
            state.mouse_focused_window.clone(),
        ) {
            state.pending_activation = Some(PendingActivation::Uri(uri.to_string()));
            let token = activation.get_activation_token(&state.globals.qh, ());
            let serial = state.serial_tracker.get(SerialKind::MousePress);
            token.set_serial(serial, &state.wl_seat);
            token.set_surface(&window.surface());
            token.commit();
        } else {
            let executor = state.common.background_executor.clone();
            open_uri_internal(executor, uri, None);
        }
    }

    fn reveal_path(&self, path: PathBuf) {
        let mut state = self.0.borrow_mut();
        if let (Some(activation), Some(window)) = (
            state.globals.activation.clone(),
            state.mouse_focused_window.clone(),
        ) {
            state.pending_activation = Some(PendingActivation::Path(path));
            let token = activation.get_activation_token(&state.globals.qh, ());
            let serial = state.serial_tracker.get(SerialKind::MousePress);
            token.set_serial(serial, &state.wl_seat);
            token.set_surface(&window.surface());
            token.commit();
        } else {
            let executor = state.common.background_executor.clone();
            reveal_path_internal(executor, path, None);
        }
    }

    fn with_common<R>(&self, f: impl FnOnce(&mut LinuxCommon) -> R) -> R {
        f(&mut self.0.borrow_mut().common)
    }

    fn run(&self) {
        let mut event_loop = self
            .0
            .borrow_mut()
            .event_loop
            .take()
            .expect("App is already running");

        event_loop
            .run(
                None,
                &mut WaylandClientStatePtr(Rc::downgrade(&self.0)),
                |_| {},
            )
            .log_err();
    }

    fn write_to_primary(&self, item: inazuma::ClipboardItem) {
        let mut state = self.0.borrow_mut();
        let (Some(primary_selection_manager), Some(primary_selection)) = (
            state.globals.primary_selection_manager.clone(),
            state.primary_selection.clone(),
        ) else {
            return;
        };
        if state.mouse_focused_window.is_some() || state.keyboard_focused_window.is_some() {
            state.clipboard.set_primary(item);
            let serial = state.serial_tracker.get(SerialKind::KeyPress);
            let data_source = primary_selection_manager.create_source(&state.globals.qh, ());
            for mime_type in TEXT_MIME_TYPES {
                data_source.offer(mime_type.to_string());
            }
            data_source.offer(state.clipboard.self_mime());
            primary_selection.set_selection(Some(&data_source), serial);
        }
    }

    fn write_to_clipboard(&self, item: inazuma::ClipboardItem) {
        let mut state = self.0.borrow_mut();
        let (Some(data_device_manager), Some(data_device)) = (
            state.globals.data_device_manager.clone(),
            state.data_device.clone(),
        ) else {
            return;
        };
        if state.mouse_focused_window.is_some() || state.keyboard_focused_window.is_some() {
            state.clipboard.set(item);
            let serial = state.serial_tracker.get(SerialKind::KeyPress);
            let data_source = data_device_manager.create_data_source(&state.globals.qh, ());
            for mime_type in TEXT_MIME_TYPES {
                data_source.offer(mime_type.to_string());
            }
            data_source.offer(state.clipboard.self_mime());
            data_device.set_selection(Some(&data_source), serial);
        }
    }

    fn read_from_primary(&self) -> Option<inazuma::ClipboardItem> {
        self.0.borrow_mut().clipboard.read_primary()
    }

    fn read_from_clipboard(&self) -> Option<inazuma::ClipboardItem> {
        self.0.borrow_mut().clipboard.read()
    }

    fn active_window(&self) -> Option<AnyWindowHandle> {
        self.0
            .borrow_mut()
            .keyboard_focused_window
            .as_ref()
            .map(|window| window.handle())
    }

    fn window_stack(&self) -> Option<Vec<AnyWindowHandle>> {
        None
    }

    fn compositor_name(&self) -> &'static str {
        "Wayland"
    }

    fn window_identifier(&self) -> impl Future<Output = Option<WindowIdentifier>> + Send + 'static {
        async fn inner(surface: Option<wl_surface::WlSurface>) -> Option<WindowIdentifier> {
            if let Some(surface) = surface {
                ashpd::WindowIdentifier::from_wayland(&surface).await
            } else {
                None
            }
        }

        let client_state = self.0.borrow();
        let active_window = client_state.keyboard_focused_window.as_ref();
        inner(active_window.map(|aw| aw.surface()))
    }
}

