use super::*;

impl LinuxClient for X11Client {
    fn compositor_name(&self) -> &'static str {
        "X11"
    }

    fn with_common<R>(&self, f: impl FnOnce(&mut LinuxCommon) -> R) -> R {
        f(&mut self.0.borrow_mut().common)
    }

    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        let state = self.0.borrow();
        Box::new(state.keyboard_layout.clone())
    }

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        let state = self.0.borrow();
        let setup = state.xcb_connection.setup();
        setup
            .roots
            .iter()
            .enumerate()
            .filter_map(|(root_id, _)| {
                Some(Rc::new(
                    X11Display::new(&state.xcb_connection, state.scale_factor, root_id).ok()?,
                ) as Rc<dyn PlatformDisplay>)
            })
            .collect()
    }

    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        let state = self.0.borrow();
        X11Display::new(
            &state.xcb_connection,
            state.scale_factor,
            state.x_root_index,
        )
        .log_err()
        .map(|display| Rc::new(display) as Rc<dyn PlatformDisplay>)
    }

    fn display(&self, id: DisplayId) -> Option<Rc<dyn PlatformDisplay>> {
        let state = self.0.borrow();

        Some(Rc::new(
            X11Display::new(
                &state.xcb_connection,
                state.scale_factor,
                u32::from(id) as usize,
            )
            .ok()?,
        ))
    }

    #[cfg(feature = "screen-capture")]
    fn is_screen_capture_supported(&self) -> bool {
        true
    }

    #[cfg(feature = "screen-capture")]
    fn screen_capture_sources(
        &self,
    ) -> futures::channel::oneshot::Receiver<anyhow::Result<Vec<Rc<dyn inazuma::ScreenCaptureSource>>>>
    {
        inazuma::scap_screen_capture::scap_screen_sources(&self.0.borrow().common.foreground_executor)
    }

    fn open_window(
        &self,
        handle: AnyWindowHandle,
        params: WindowParams,
    ) -> anyhow::Result<Box<dyn PlatformWindow>> {
        let mut state = self.0.borrow_mut();
        let parent_window = state
            .keyboard_focused_window
            .and_then(|focused_window| state.windows.get(&focused_window))
            .map(|w| w.window.clone());
        let x_window = state
            .xcb_connection
            .generate_id()
            .context("X11: Failed to generate window ID")?;

        let xcb_connection = state.xcb_connection.clone();
        let client_side_decorations_supported = state.client_side_decorations_supported;
        let x_root_index = state.x_root_index;
        let atoms = state.atoms;
        let scale_factor = state.scale_factor;
        let appearance = state.common.appearance;
        let compositor_gpu = state.compositor_gpu.take();
        let window = X11Window::new(
            handle,
            X11ClientStatePtr(Rc::downgrade(&self.0)),
            state.common.foreground_executor.clone(),
            state.gpu_context.clone(),
            compositor_gpu,
            params,
            &xcb_connection,
            client_side_decorations_supported,
            x_root_index,
            x_window,
            &atoms,
            scale_factor,
            appearance,
            parent_window,
        )?;
        check_reply(
            || "Failed to set XdndAware property",
            state.xcb_connection.change_property32(
                xproto::PropMode::REPLACE,
                x_window,
                state.atoms.XdndAware,
                state.atoms.XA_ATOM,
                &[5],
            ),
        )
        .log_err();
        xcb_flush(&state.xcb_connection);

        let window_ref = WindowRef {
            window: window.0.clone(),
            refresh_state: None,
            expose_event_received: false,
            last_visibility: Visibility::UNOBSCURED,
            is_mapped: false,
        };

        state.windows.insert(x_window, window_ref);
        Ok(Box::new(window))
    }

    fn set_cursor_style(&self, style: CursorStyle) {
        let mut state = self.0.borrow_mut();
        let Some(focused_window) = state.mouse_focused_window else {
            return;
        };
        let current_style = state
            .cursor_styles
            .get(&focused_window)
            .unwrap_or(&CursorStyle::Arrow);

        let window = state
            .mouse_focused_window
            .and_then(|w| state.windows.get(&w));

        let should_change = *current_style != style
            && (window.is_none() || window.is_some_and(|w| !w.is_blocked()));

        if !should_change {
            return;
        }

        let Some(cursor) = state.get_cursor_icon(style) else {
            return;
        };

        state.cursor_styles.insert(focused_window, style);
        check_reply(
            || "Failed to set cursor style",
            state.xcb_connection.change_window_attributes(
                focused_window,
                &ChangeWindowAttributesAux {
                    cursor: Some(cursor),
                    ..Default::default()
                },
            ),
        )
        .log_err();
        state.xcb_connection.flush().log_err();
    }

    fn open_uri(&self, uri: &str) {
        #[cfg(any(feature = "wayland", feature = "x11"))]
        open_uri_internal(
            self.with_common(|c| c.background_executor.clone()),
            uri,
            None,
        );
    }

    fn reveal_path(&self, path: PathBuf) {
        #[cfg(any(feature = "x11", feature = "wayland"))]
        reveal_path_internal(
            self.with_common(|c| c.background_executor.clone()),
            path,
            None,
        );
    }

    fn write_to_primary(&self, item: inazuma::ClipboardItem) {
        let state = self.0.borrow_mut();
        state
            .clipboard
            .set_text(
                std::borrow::Cow::Owned(item.text().unwrap_or_default()),
                clipboard::ClipboardKind::Primary,
                clipboard::WaitConfig::None,
            )
            .context("X11 Failed to write to clipboard (primary)")
            .log_with_level(log::Level::Debug);
    }

    fn write_to_clipboard(&self, item: inazuma::ClipboardItem) {
        let mut state = self.0.borrow_mut();
        state
            .clipboard
            .set_text(
                std::borrow::Cow::Owned(item.text().unwrap_or_default()),
                clipboard::ClipboardKind::Clipboard,
                clipboard::WaitConfig::None,
            )
            .context("X11: Failed to write to clipboard (clipboard)")
            .log_with_level(log::Level::Debug);
        state.clipboard_item.replace(item);
    }

    fn read_from_primary(&self) -> Option<inazuma::ClipboardItem> {
        let state = self.0.borrow_mut();
        state
            .clipboard
            .get_any(clipboard::ClipboardKind::Primary)
            .context("X11: Failed to read from clipboard (primary)")
            .log_with_level(log::Level::Debug)
    }

    fn read_from_clipboard(&self) -> Option<inazuma::ClipboardItem> {
        let state = self.0.borrow_mut();
        // if the last copy was from this app, return our cached item
        // which has metadata attached.
        if state
            .clipboard
            .is_owner(clipboard::ClipboardKind::Clipboard)
        {
            return state.clipboard_item.clone();
        }
        state
            .clipboard
            .get_any(clipboard::ClipboardKind::Clipboard)
            .context("X11: Failed to read from clipboard (clipboard)")
            .log_with_level(log::Level::Debug)
    }

    fn run(&self) {
        let Some(mut event_loop) = self
            .0
            .borrow_mut()
            .event_loop
            .take()
            .context("X11Client::run called but it's already running")
            .log_err()
        else {
            return;
        };

        event_loop.run(None, &mut self.clone(), |_| {}).log_err();
    }

    fn active_window(&self) -> Option<AnyWindowHandle> {
        let state = self.0.borrow();
        state.keyboard_focused_window.and_then(|focused_window| {
            state
                .windows
                .get(&focused_window)
                .map(|window| window.handle())
        })
    }

    fn window_stack(&self) -> Option<Vec<AnyWindowHandle>> {
        let state = self.0.borrow();
        let root = state.xcb_connection.setup().roots[state.x_root_index].root;

        let reply = state
            .xcb_connection
            .get_property(
                false,
                root,
                state.atoms._NET_CLIENT_LIST_STACKING,
                xproto::AtomEnum::WINDOW,
                0,
                u32::MAX,
            )
            .ok()?
            .reply()
            .ok()?;

        let window_ids = reply
            .value
            .chunks_exact(4)
            .filter_map(|chunk| chunk.try_into().ok().map(u32::from_ne_bytes))
            .collect::<Vec<xproto::Window>>();

        let mut handles = Vec::new();

        // We need to reverse, since _NET_CLIENT_LIST_STACKING has
        // a back-to-front order.
        // See: https://specifications.freedesktop.org/wm-spec/1.3/ar01s03.html
        for window_ref in window_ids
            .iter()
            .rev()
            .filter_map(|&win| state.windows.get(&win))
        {
            if !window_ref.window.state.borrow().destroyed {
                handles.push(window_ref.handle());
            }
        }

        Some(handles)
    }

    fn window_identifier(&self) -> impl Future<Output = Option<WindowIdentifier>> + Send + 'static {
        let state = self.0.borrow();
        state
            .keyboard_focused_window
            .and_then(|focused_window| state.windows.get(&focused_window))
            .map(|window| window.window.x_window as u64)
            .map(|x_window| std::future::ready(Some(WindowIdentifier::from_xid(x_window))))
            .unwrap_or(std::future::ready(None))
    }
}

