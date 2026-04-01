use super::*;

impl<P: LinuxClient + 'static> Platform for LinuxPlatform<P> {
    fn background_executor(&self) -> BackgroundExecutor {
        self.inner
            .with_common(|common| common.background_executor.clone())
    }

    fn foreground_executor(&self) -> ForegroundExecutor {
        self.inner
            .with_common(|common| common.foreground_executor.clone())
    }

    fn text_system(&self) -> Arc<dyn PlatformTextSystem> {
        self.inner.with_common(|common| common.text_system.clone())
    }

    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        self.inner.keyboard_layout()
    }

    fn keyboard_mapper(&self) -> Rc<dyn PlatformKeyboardMapper> {
        Rc::new(inazuma::DummyKeyboardMapper)
    }

    fn on_keyboard_layout_change(&self, callback: Box<dyn FnMut()>) {
        self.inner
            .with_common(|common| common.callbacks.keyboard_layout_change = Some(callback));
    }

    fn on_thermal_state_change(&self, _callback: Box<dyn FnMut()>) {}

    fn thermal_state(&self) -> ThermalState {
        ThermalState::Nominal
    }

    fn run(&self, on_finish_launching: Box<dyn FnOnce()>) {
        on_finish_launching();

        LinuxClient::run(&self.inner);

        let quit = self
            .inner
            .with_common(|common| common.callbacks.quit.take());
        if let Some(mut fun) = quit {
            fun();
        }
    }

    fn quit(&self) {
        self.inner.with_common(|common| common.signal.stop());
    }

    fn compositor_name(&self) -> &'static str {
        self.inner.compositor_name()
    }

    fn restart(&self, binary_path: Option<PathBuf>) {
        use std::os::unix::process::CommandExt as _;

        // get the process id of the current process
        let app_pid = std::process::id().to_string();
        // get the path to the executable
        let app_path = if let Some(path) = binary_path {
            path
        } else {
            match self.app_path() {
                Ok(path) => path,
                Err(err) => {
                    log::error!("Failed to get app path: {:?}", err);
                    return;
                }
            }
        };

        log::info!("Restarting process, using app path: {:?}", app_path);

        // Script to wait for the current process to exit and then restart the app.
        // Pass dynamic values as positional parameters to avoid shell interpolation issues.
        let script = r#"
            while kill -0 "$0" 2>/dev/null; do
                sleep 0.1
            done

            "$1"
            "#;

        #[allow(
            clippy::disallowed_methods,
            reason = "We are restarting ourselves, using std command thus is fine"
        )]
        let restart_process = new_std_command("/usr/bin/env")
            .arg("bash")
            .arg("-c")
            .arg(script)
            .arg(&app_pid)
            .arg(&app_path)
            .process_group(0)
            .spawn();

        match restart_process {
            Ok(_) => self.quit(),
            Err(e) => log::error!("failed to spawn restart script: {:?}", e),
        }
    }

    fn activate(&self, _ignoring_other_apps: bool) {
        log::info!("activate is not implemented on Linux, ignoring the call")
    }

    fn hide(&self) {
        log::info!("hide is not implemented on Linux, ignoring the call")
    }

    fn hide_other_apps(&self) {
        log::info!("hide_other_apps is not implemented on Linux, ignoring the call")
    }

    fn unhide_other_apps(&self) {
        log::info!("unhide_other_apps is not implemented on Linux, ignoring the call")
    }

    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        self.inner.primary_display()
    }

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        self.inner.displays()
    }

    #[cfg(feature = "screen-capture")]
    fn is_screen_capture_supported(&self) -> bool {
        self.inner.is_screen_capture_supported()
    }

    #[cfg(feature = "screen-capture")]
    fn screen_capture_sources(
        &self,
    ) -> oneshot::Receiver<Result<Vec<Rc<dyn inazuma::ScreenCaptureSource>>>> {
        self.inner.screen_capture_sources()
    }

    fn active_window(&self) -> Option<AnyWindowHandle> {
        self.inner.active_window()
    }

    fn window_stack(&self) -> Option<Vec<AnyWindowHandle>> {
        self.inner.window_stack()
    }

    fn open_window(
        &self,
        handle: AnyWindowHandle,
        options: WindowParams,
    ) -> anyhow::Result<Box<dyn PlatformWindow>> {
        self.inner.open_window(handle, options)
    }

    fn open_url(&self, url: &str) {
        self.inner.open_uri(url);
    }

    fn on_open_urls(&self, callback: Box<dyn FnMut(Vec<String>)>) {
        self.inner
            .with_common(|common| common.callbacks.open_urls = Some(callback));
    }

    fn prompt_for_paths(
        &self,
        options: PathPromptOptions,
    ) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>> {
        let (done_tx, done_rx) = oneshot::channel();

        #[cfg(not(any(feature = "wayland", feature = "x11")))]
        let _ = (done_tx.send(Ok(None)), options);

        #[cfg(any(feature = "wayland", feature = "x11"))]
        let identifier = self.inner.window_identifier();

        #[cfg(any(feature = "wayland", feature = "x11"))]
        self.foreground_executor()
            .spawn(async move {
                let title = if options.directories {
                    "Open Folder"
                } else {
                    "Open File"
                };

                let request = match ashpd::desktop::file_chooser::OpenFileRequest::default()
                    .identifier(identifier.await)
                    .modal(true)
                    .title(title)
                    .accept_label(options.prompt.as_ref().map(inazuma::SharedString::as_str))
                    .multiple(options.multiple)
                    .directory(options.directories)
                    .send()
                    .await
                {
                    Ok(request) => request,
                    Err(err) => {
                        let result = match err {
                            ashpd::Error::PortalNotFound(_) => anyhow!(FILE_PICKER_PORTAL_MISSING),
                            err => err.into(),
                        };
                        let _ = done_tx.send(Err(result));
                        return;
                    }
                };

                let result = match request.response() {
                    Ok(response) => Ok(Some(
                        response
                            .uris()
                            .iter()
                            .filter_map(|uri: &ashpd::Uri| url::Url::parse(uri.as_str()).ok())
                            .filter_map(|uri: url::Url| uri.to_file_path().ok())
                            .collect::<Vec<_>>(),
                    )),
                    Err(ashpd::Error::Response(_)) => Ok(None),
                    Err(e) => Err(e.into()),
                };
                let _ = done_tx.send(result);
            })
            .detach();
        done_rx
    }

    fn prompt_for_new_path(
        &self,
        directory: &Path,
        suggested_name: Option<&str>,
    ) -> oneshot::Receiver<Result<Option<PathBuf>>> {
        let (done_tx, done_rx) = oneshot::channel();

        #[cfg(not(any(feature = "wayland", feature = "x11")))]
        let _ = (done_tx.send(Ok(None)), directory, suggested_name);

        #[cfg(any(feature = "wayland", feature = "x11"))]
        let identifier = self.inner.window_identifier();

        #[cfg(any(feature = "wayland", feature = "x11"))]
        self.foreground_executor()
            .spawn({
                let directory = directory.to_owned();
                let suggested_name = suggested_name.map(|s| s.to_owned());

                async move {
                    let mut request_builder =
                        ashpd::desktop::file_chooser::SaveFileRequest::default()
                            .identifier(identifier.await)
                            .modal(true)
                            .title("Save File")
                            .current_folder(directory)
                            .expect("pathbuf should not be nul terminated");

                    if let Some(suggested_name) = suggested_name {
                        request_builder = request_builder.current_name(suggested_name.as_str());
                    }

                    let request = match request_builder.send().await {
                        Ok(request) => request,
                        Err(err) => {
                            let result = match err {
                                ashpd::Error::PortalNotFound(_) => {
                                    anyhow!(FILE_PICKER_PORTAL_MISSING)
                                }
                                err => err.into(),
                            };
                            let _ = done_tx.send(Err(result));
                            return;
                        }
                    };

                    let result = match request.response() {
                        Ok(response) => Ok(response
                            .uris()
                            .first()
                            .and_then(|uri: &ashpd::Uri| url::Url::parse(uri.as_str()).ok())
                            .and_then(|uri: url::Url| uri.to_file_path().ok())),
                        Err(ashpd::Error::Response(_)) => Ok(None),
                        Err(e) => Err(e.into()),
                    };
                    let _ = done_tx.send(result);
                }
            })
            .detach();

        done_rx
    }

    fn can_select_mixed_files_and_dirs(&self) -> bool {
        // org.freedesktop.portal.FileChooser only supports "pick files" and "pick directories".
        false
    }

    fn reveal_path(&self, path: &Path) {
        self.inner.reveal_path(path.to_owned());
    }

    fn open_with_system(&self, path: &Path) {
        let path = path.to_owned();
        self.background_executor()
            .spawn(async move {
                let _ = new_command("xdg-open")
                    .arg(path)
                    .spawn()
                    .context("invoking xdg-open")
                    .log_err()?
                    .status()
                    .await
                    .log_err()?;
                Some(())
            })
            .detach();
    }

    fn on_quit(&self, callback: Box<dyn FnMut()>) {
        self.inner.with_common(|common| {
            common.callbacks.quit = Some(callback);
        });
    }

    fn on_reopen(&self, callback: Box<dyn FnMut()>) {
        self.inner.with_common(|common| {
            common.callbacks.reopen = Some(callback);
        });
    }

    fn on_app_menu_action(&self, callback: Box<dyn FnMut(&dyn Action)>) {
        self.inner.with_common(|common| {
            common.callbacks.app_menu_action = Some(callback);
        });
    }

    fn on_will_open_app_menu(&self, callback: Box<dyn FnMut()>) {
        self.inner.with_common(|common| {
            common.callbacks.will_open_app_menu = Some(callback);
        });
    }

    fn on_validate_app_menu_command(&self, callback: Box<dyn FnMut(&dyn Action) -> bool>) {
        self.inner.with_common(|common| {
            common.callbacks.validate_app_menu_command = Some(callback);
        });
    }

    fn app_path(&self) -> Result<PathBuf> {
        // get the path of the executable of the current process
        let app_path = env::current_exe()?;
        Ok(app_path)
    }

    fn set_menus(&self, menus: Vec<Menu>, _keymap: &Keymap) {
        self.inner.with_common(|common| {
            common.menus = menus.into_iter().map(|menu| menu.owned()).collect();
        })
    }

    fn get_menus(&self) -> Option<Vec<OwnedMenu>> {
        self.inner.with_common(|common| Some(common.menus.clone()))
    }

    fn set_dock_menu(&self, _menu: Vec<MenuItem>, _keymap: &Keymap) {
        // todo(linux)
    }

    fn path_for_auxiliary_executable(&self, _name: &str) -> Result<PathBuf> {
        Err(anyhow::Error::msg(
            "Platform<LinuxPlatform>::path_for_auxiliary_executable is not implemented yet",
        ))
    }

    fn set_cursor_style(&self, style: CursorStyle) {
        self.inner.set_cursor_style(style)
    }

    fn should_auto_hide_scrollbars(&self) -> bool {
        self.inner.with_common(|common| common.auto_hide_scrollbars)
    }

    fn write_credentials(&self, url: &str, username: &str, password: &[u8]) -> Task<Result<()>> {
        let url = url.to_string();
        let username = username.to_string();
        let password = password.to_vec();
        self.background_executor().spawn(async move {
            let keyring = oo7::Keyring::new().await?;
            keyring.unlock().await?;
            keyring
                .create_item(
                    KEYRING_LABEL,
                    &vec![("url", &url), ("username", &username)],
                    password,
                    true,
                )
                .await?;
            Ok(())
        })
    }

    fn read_credentials(&self, url: &str) -> Task<Result<Option<(String, Vec<u8>)>>> {
        let url = url.to_string();
        self.background_executor().spawn(async move {
            let keyring = oo7::Keyring::new().await?;
            keyring.unlock().await?;

            let items = keyring.search_items(&vec![("url", &url)]).await?;

            for item in items.into_iter() {
                if item.label().await.is_ok_and(|label| label == KEYRING_LABEL) {
                    let attributes = item.attributes().await?;
                    let username = attributes
                        .get("username")
                        .context("Cannot find username in stored credentials")?;
                    item.unlock().await?;
                    let secret = item.secret().await?;

                    // we lose the zeroizing capabilities at this boundary,
                    // a current limitation GPUI's credentials api
                    return Ok(Some((username.to_string(), secret.to_vec())));
                } else {
                    continue;
                }
            }
            Ok(None)
        })
    }

    fn delete_credentials(&self, url: &str) -> Task<Result<()>> {
        let url = url.to_string();
        self.background_executor().spawn(async move {
            let keyring = oo7::Keyring::new().await?;
            keyring.unlock().await?;

            let items = keyring.search_items(&vec![("url", &url)]).await?;

            for item in items.into_iter() {
                if item.label().await.is_ok_and(|label| label == KEYRING_LABEL) {
                    item.delete().await?;
                    return Ok(());
                }
            }

            Ok(())
        })
    }

    fn window_appearance(&self) -> WindowAppearance {
        self.inner.with_common(|common| common.appearance)
    }

    fn register_url_scheme(&self, _: &str) -> Task<anyhow::Result<()>> {
        Task::ready(Err(anyhow!("register_url_scheme unimplemented")))
    }

    fn write_to_primary(&self, item: ClipboardItem) {
        self.inner.write_to_primary(item)
    }

    fn write_to_clipboard(&self, item: ClipboardItem) {
        self.inner.write_to_clipboard(item)
    }

    fn read_from_primary(&self) -> Option<ClipboardItem> {
        self.inner.read_from_primary()
    }

    fn read_from_clipboard(&self) -> Option<ClipboardItem> {
        self.inner.read_from_clipboard()
    }

    fn add_recent_document(&self, _path: &Path) {}
}

#[cfg(any(feature = "wayland", feature = "x11"))]
