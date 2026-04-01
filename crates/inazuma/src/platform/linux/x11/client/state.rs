use super::*;

impl X11ClientState {
    fn has_xim(&self) -> bool {
        self.ximc.is_some() && self.xim_handler.is_some()
    }

    fn take_xim(&mut self) -> Option<(X11rbClient<Rc<XCBConnection>>, XimHandler)> {
        let ximc = self
            .ximc
            .take()
            .ok_or(anyhow!("bug: XIM connection not set"))
            .log_err()?;
        if let Some(xim_handler) = self.xim_handler.take() {
            Some((ximc, xim_handler))
        } else {
            self.ximc = Some(ximc);
            log::error!("bug: XIM handler not set");
            None
        }
    }

    fn restore_xim(&mut self, ximc: X11rbClient<Rc<XCBConnection>>, xim_handler: XimHandler) {
        self.ximc = Some(ximc);
        self.xim_handler = Some(xim_handler);
    }

    fn update_refresh_loop(&mut self, x_window: xproto::Window) {
        let Some(window_ref) = self.windows.get_mut(&x_window) else {
            return;
        };
        let is_visible = window_ref.is_mapped
            && !matches!(window_ref.last_visibility, Visibility::FULLY_OBSCURED);
        match (is_visible, window_ref.refresh_state.take()) {
            (false, refresh_state @ Some(RefreshState::Hidden { .. }))
            | (false, refresh_state @ None)
            | (true, refresh_state @ Some(RefreshState::PeriodicRefresh { .. })) => {
                window_ref.refresh_state = refresh_state;
            }
            (
                false,
                Some(RefreshState::PeriodicRefresh {
                    refresh_rate,
                    event_loop_token,
                }),
            ) => {
                self.loop_handle.remove(event_loop_token);
                window_ref.refresh_state = Some(RefreshState::Hidden { refresh_rate });
            }
            (true, Some(RefreshState::Hidden { refresh_rate })) => {
                let event_loop_token = self.start_refresh_loop(x_window, refresh_rate);
                let Some(window_ref) = self.windows.get_mut(&x_window) else {
                    return;
                };
                window_ref.refresh_state = Some(RefreshState::PeriodicRefresh {
                    refresh_rate,
                    event_loop_token,
                });
            }
            (true, None) => {
                let Some(screen_resources) = get_reply(
                    || "Failed to get screen resources",
                    self.xcb_connection
                        .randr_get_screen_resources_current(x_window),
                )
                .log_err() else {
                    return;
                };

                // Ideally this would be re-queried when the window changes screens, but there
                // doesn't seem to be an efficient / straightforward way to do this. Should also be
                // updated when screen configurations change.
                let mode_info = screen_resources.crtcs.iter().find_map(|crtc| {
                    let crtc_info = self
                        .xcb_connection
                        .randr_get_crtc_info(*crtc, x11rb::CURRENT_TIME)
                        .ok()?
                        .reply()
                        .ok()?;

                    screen_resources
                        .modes
                        .iter()
                        .find(|m| m.id == crtc_info.mode)
                });
                let refresh_rate = match mode_info {
                    Some(mode_info) => mode_refresh_rate(mode_info),
                    None => {
                        log::error!(
                            "Failed to get screen mode info from xrandr, \
                            defaulting to 60hz refresh rate."
                        );
                        Duration::from_micros(1_000_000 / 60)
                    }
                };

                let event_loop_token = self.start_refresh_loop(x_window, refresh_rate);
                let Some(window_ref) = self.windows.get_mut(&x_window) else {
                    return;
                };
                window_ref.refresh_state = Some(RefreshState::PeriodicRefresh {
                    refresh_rate,
                    event_loop_token,
                });
            }
        }
    }

    #[must_use]
    fn start_refresh_loop(
        &self,
        x_window: xproto::Window,
        refresh_rate: Duration,
    ) -> RegistrationToken {
        self.loop_handle
            .insert_source(calloop::timer::Timer::immediate(), {
                move |mut instant, (), client| {
                    let xcb_connection = {
                        let mut state = client.0.borrow_mut();
                        let xcb_connection = state.xcb_connection.clone();
                        if let Some(window) = state.windows.get_mut(&x_window) {
                            let expose_event_received = window.expose_event_received;
                            window.expose_event_received = false;
                            let window = window.window.clone();
                            drop(state);
                            window.refresh(RequestFrameOptions {
                                require_presentation: expose_event_received,
                                force_render: false,
                            });
                        }
                        xcb_connection
                    };
                    client.process_x11_events(&xcb_connection).log_err();

                    // Take into account that some frames have been skipped
                    let now = Instant::now();
                    while instant < now {
                        instant += refresh_rate;
                    }
                    calloop::timer::TimeoutAction::ToInstant(instant)
                }
            })
            .expect("Failed to initialize window refresh timer")
    }

    fn get_cursor_icon(&mut self, style: CursorStyle) -> Option<xproto::Cursor> {
        if let Some(cursor) = self.cursor_cache.get(&style) {
            return *cursor;
        }

        let result;
        match style {
            CursorStyle::None => match create_invisible_cursor(&self.xcb_connection) {
                Ok(loaded_cursor) => result = Ok(loaded_cursor),
                Err(err) => result = Err(err.context("X11: error while creating invisible cursor")),
            },
            _ => 'outer: {
                let mut errors = String::new();
                let cursor_icon_names = cursor_style_to_icon_names(style);
                for cursor_icon_name in cursor_icon_names {
                    match self
                        .cursor_handle
                        .load_cursor(&self.xcb_connection, cursor_icon_name)
                    {
                        Ok(loaded_cursor) => {
                            if loaded_cursor != x11rb::NONE {
                                result = Ok(loaded_cursor);
                                break 'outer;
                            }
                        }
                        Err(err) => {
                            errors.push_str(&err.to_string());
                            errors.push('\n');
                        }
                    }
                }
                if errors.is_empty() {
                    result = Err(anyhow!(
                        "errors while loading cursor icons {:?}:\n{}",
                        cursor_icon_names,
                        errors
                    ));
                } else {
                    result = Err(anyhow!("did not find cursor icons {:?}", cursor_icon_names));
                }
            }
        };

        let cursor = match result {
            Ok(cursor) => Some(cursor),
            Err(err) => {
                match self
                    .cursor_handle
                    .load_cursor(&self.xcb_connection, DEFAULT_CURSOR_ICON_NAME)
                {
                    Ok(default) => {
                        log_cursor_icon_warning(err.context(format!(
                            "X11: error loading cursor icon, falling back on default icon '{}'",
                            DEFAULT_CURSOR_ICON_NAME
                        )));
                        Some(default)
                    }
                    Err(default_err) => {
                        log_cursor_icon_warning(err.context(default_err).context(format!(
                            "X11: error loading default cursor fallback '{}'",
                            DEFAULT_CURSOR_ICON_NAME
                        )));
                        None
                    }
                }
            }
        };

        self.cursor_cache.insert(style, cursor);
        cursor
    }
}

// Adapted from:
// https://docs.rs/winit/0.29.11/src/winit/platform_impl/linux/x11/monitor.rs.html#103-111
