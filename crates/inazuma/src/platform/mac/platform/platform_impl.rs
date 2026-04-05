use super::*;
use objc2::DefinedClass;

impl Platform for MacPlatform {
    fn background_executor(&self) -> BackgroundExecutor {
        self.0.lock().background_executor.clone()
    }

    fn foreground_executor(&self) -> inazuma::ForegroundExecutor {
        self.0.lock().foreground_executor.clone()
    }

    fn text_system(&self) -> Arc<dyn PlatformTextSystem> {
        self.0.lock().text_system.clone()
    }

    fn run(&self, on_finish_launching: Box<dyn FnOnce()>) {
        let mut state = self.0.lock();
        if state.headless {
            drop(state);
            on_finish_launching();
            CFRunLoop::run();
        } else {
            state.finish_launching = Some(on_finish_launching);
            drop(state);
        }

        unsafe {
            let mtm = MainThreadMarker::new_unchecked();

            // Register and get the shared GPUIApplication instance
            let app: Retained<GPUIApplication> = {
                // Force the class to be registered before calling sharedApplication
                let _cls = GPUIApplication::class();
                let raw_app = NSApplication::sharedApplication(mtm);
                // The shared application was created as GPUIApplication since we registered it
                Retained::cast_unchecked(raw_app)
            };

            // Create the delegate
            let delegate = {
                let alloc = mtm.alloc::<GPUIApplicationDelegate>();
                let delegate: Retained<GPUIApplicationDelegate> =
                    msg_send![super(alloc.set_ivars(DelegateIvars::default())), init];
                delegate
            };

            // Set platform pointer on both app and delegate
            let self_ptr = self as *const Self as *const c_void;
            app.ivars().platform.set(self_ptr);
            delegate.ivars().platform.set(self_ptr);

            // Set delegate on app
            let delegate_protocol: &ProtocolObject<dyn NSApplicationDelegate> =
                ProtocolObject::from_ref(&*delegate);
            app.setDelegate(Some(delegate_protocol));

            app.run();

            // Clear platform pointers after run returns
            app.ivars().platform.set(ptr::null());
            delegate.ivars().platform.set(ptr::null());
        }
    }

    fn quit(&self) {
        // Quitting the app causes us to close windows, which invokes `Window::on_close` callbacks
        // synchronously before this method terminates. If we call `Platform::quit` while holding a
        // borrow of the app state (which most of the time we will do), we will end up
        // double-borrowing the app state in the `on_close` callbacks for our open windows. To solve
        // this, we make quitting the application asynchronous so that we aren't holding borrows to
        // the app state on the stack when we actually terminate the app.

        unsafe {
            DispatchQueue::main().exec_async_f(ptr::null_mut(), quit);
        }

        extern "C" fn quit(_: *mut c_void) {
            unsafe {
                let mtm = MainThreadMarker::new_unchecked();
                let app = NSApplication::sharedApplication(mtm);
                app.terminate(None);
            }
        }
    }

    fn restart(&self, _binary_path: Option<PathBuf>) {
        use std::os::unix::process::CommandExt as _;

        let app_pid = std::process::id().to_string();
        let app_path = self
            .app_path()
            .ok()
            // When the app is not bundled, `app_path` returns the
            // directory containing the executable. Disregard this
            // and get the path to the executable itself.
            .and_then(|path| (path.extension()?.to_str()? == "app").then_some(path))
            .unwrap_or_else(|| std::env::current_exe().unwrap());

        // Wait until this process has exited and then re-open this path.
        let script = r#"
            while kill -0 $0 2> /dev/null; do
                sleep 0.1
            done
            open "$1"
        "#;

        #[allow(
            clippy::disallowed_methods,
            reason = "We are restarting ourselves, using std command thus is fine"
        )]
        let restart_process = new_std_command("/bin/bash")
            .arg("-c")
            .arg(script)
            .arg(app_pid)
            .arg(app_path)
            .process_group(0)
            .spawn();

        match restart_process {
            Ok(_) => self.quit(),
            Err(e) => log::error!("failed to spawn restart script: {:?}", e),
        }
    }

    fn activate(&self, _ignoring_other_apps: bool) {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            let app = NSApplication::sharedApplication(mtm);
            app.activate();
        }
    }

    fn hide(&self) {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            let app = NSApplication::sharedApplication(mtm);
            app.hide(None);
        }
    }

    fn hide_other_apps(&self) {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            let app = NSApplication::sharedApplication(mtm);
            app.hideOtherApplications(None);
        }
    }

    fn unhide_other_apps(&self) {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            let app = NSApplication::sharedApplication(mtm);
            app.unhideAllApplications(None);
        }
    }

    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(Rc::new(MacDisplay::primary()))
    }

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        MacDisplay::all()
            .map(|screen| Rc::new(screen) as Rc<_>)
            .collect()
    }

    #[cfg(feature = "screen-capture")]
    fn is_screen_capture_supported(&self) -> bool {
        let version = NSProcessInfo::processInfo().operatingSystemVersion();
        version.majorVersion >= 12 && version.minorVersion >= 3
    }

    #[cfg(feature = "screen-capture")]
    fn screen_capture_sources(
        &self,
    ) -> oneshot::Receiver<Result<Vec<Rc<dyn inazuma::ScreenCaptureSource>>>> {
        crate::screen_capture::get_sources()
    }

    fn active_window(&self) -> Option<AnyWindowHandle> {
        super::super::MacWindow::active_window()
    }

    // Returns the windows ordered front-to-back, meaning that the active
    // window is the first one in the returned vec.
    fn window_stack(&self) -> Option<Vec<AnyWindowHandle>> {
        Some(super::super::MacWindow::ordered_windows())
    }

    fn open_window(
        &self,
        handle: AnyWindowHandle,
        options: WindowParams,
    ) -> Result<Box<dyn PlatformWindow>> {
        let renderer_context = self.0.lock().renderer_context.clone();
        Ok(Box::new(super::super::MacWindow::open(
            handle,
            options,
            self.foreground_executor(),
            self.background_executor(),
            renderer_context,
        )))
    }

    fn window_appearance(&self) -> WindowAppearance {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            let app = NSApplication::sharedApplication(mtm);
            let appearance = app.effectiveAppearance();
            super::super::window_appearance::window_appearance_from_native(&appearance)
        }
    }

    fn open_url(&self, url: &str) {
        let ns_url_string = NSString::from_str(url);
        let Some(ns_url) = NSURL::URLWithString(&ns_url_string) else {
            log::error!("Failed to create NSURL from string: {}", url);
            return;
        };
        let workspace = NSWorkspace::sharedWorkspace();
        workspace.openURL(&ns_url);
    }

    fn register_url_scheme(&self, scheme: &str) -> Task<anyhow::Result<()>> {
        // API only available post Monterey
        // https://developer.apple.com/documentation/appkit/nsworkspace/3753004-setdefaultapplicationaturl
        let (done_tx, done_rx) = oneshot::channel();
        if Self::os_version() < Version::new(12, 0, 0) {
            return Task::ready(Err(anyhow!(
                "macOS 12.0 or later is required to register URL schemes"
            )));
        }

        let bundle = NSBundle::mainBundle();
        let Some(bundle_id) = bundle.bundleIdentifier() else {
            return Task::ready(Err(anyhow!("Can only register URL scheme in bundled apps")));
        };

        let workspace = NSWorkspace::sharedWorkspace();
        let scheme_str = NSString::from_str(scheme);

        // Get the app URL for the bundle identifier
        let app_urls =
            workspace.URLsForApplicationsWithBundleIdentifier(&bundle_id);
        let Some(app_url) = app_urls.firstObject() else {
            return Task::ready(Err(anyhow!(
                "Cannot register URL scheme until app is installed"
            )));
        };

        let done_tx = Cell::new(Some(done_tx));
        let block = RcBlock::new(move |error: *mut NSError| {
            let result = if error.is_null() {
                Ok(())
            } else {
                let error_ref = unsafe { &*error };
                let msg = error_ref.localizedDescription().to_string();
                Err(anyhow!("Failed to register: {msg}"))
            };

            if let Some(done_tx) = done_tx.take() {
                let _ = done_tx.send(result);
            }
        });

        workspace.setDefaultApplicationAtURL_toOpenURLsWithScheme_completionHandler(
            &app_url,
            &scheme_str,
            Some(&block),
        );

        self.background_executor()
            .spawn(async { done_rx.await.map_err(|e| anyhow!(e))? })
    }

    fn on_open_urls(&self, callback: Box<dyn FnMut(Vec<String>)>) {
        self.0.lock().open_urls = Some(callback);
    }

    fn prompt_for_paths(
        &self,
        options: PathPromptOptions,
    ) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>> {
        let (done_tx, done_rx) = oneshot::channel();
        self.foreground_executor()
            .spawn(async move {
                unsafe {
                    let mtm = MainThreadMarker::new_unchecked();
                    let panel = NSOpenPanel::openPanel(mtm);
                    panel.setCanChooseDirectories(options.directories);
                    panel.setCanChooseFiles(options.files);
                    panel.setAllowsMultipleSelection(options.multiple);
                    panel.setCanCreateDirectories(true);
                    panel.setResolvesAliases(false);

                    if let Some(prompt) = options.prompt {
                        panel.setPrompt(Some(&NSString::from_str(&prompt)));
                    }

                    let done_tx = Cell::new(Some(done_tx));
                    let panel_clone = panel.clone();
                    let block = RcBlock::new(move |response: NSModalResponse| {
                        let result = if response == NSModalResponseOK {
                            let mut result = Vec::new();
                            let urls = panel_clone.URLs();
                            for i in 0..urls.count() {
                                let url = urls.objectAtIndex(i);
                                if url.isFileURL() {
                                    if let Ok(path) = ns_url_to_path(&url) {
                                        result.push(path);
                                    }
                                }
                            }
                            Some(result)
                        } else {
                            None
                        };

                        if let Some(done_tx) = done_tx.take() {
                            let _ = done_tx.send(Ok(result));
                        }
                    });

                    panel.beginWithCompletionHandler(&block);
                }
            })
            .detach();
        done_rx
    }

    fn prompt_for_new_path(
        &self,
        directory: &Path,
        suggested_name: Option<&str>,
    ) -> oneshot::Receiver<Result<Option<PathBuf>>> {
        let directory = directory.to_owned();
        let suggested_name = suggested_name.map(|s| s.to_owned());
        let (done_tx, done_rx) = oneshot::channel();
        self.foreground_executor()
            .spawn(async move {
                unsafe {
                    let mtm = MainThreadMarker::new_unchecked();
                    let panel = NSSavePanel::savePanel(mtm);
                    let path = NSString::from_str(directory.to_string_lossy().as_ref());
                    let url = NSURL::fileURLWithPath_isDirectory(&path, true);
                    panel.setDirectoryURL(Some(&url));

                    if let Some(suggested_name) = suggested_name {
                        let name_string = NSString::from_str(&suggested_name);
                        panel.setNameFieldStringValue(&name_string);
                    }

                    let done_tx = Cell::new(Some(done_tx));
                    let panel_clone = panel.clone();
                    let block = RcBlock::new(move |response: NSModalResponse| {
                        let mut result = None;
                        if response == NSModalResponseOK {
                            if let Some(url) = panel_clone.URL() {
                                if url.isFileURL() {
                                    result = ns_url_to_path(&url).ok().map(|mut result| {
                                        let Some(filename) = result.file_name() else {
                                            return result;
                                        };
                                        let chunks = filename
                                            .as_bytes()
                                            .split(|&b| b == b'.')
                                            .collect::<Vec<_>>();

                                        // https://github.com/zed-industries/zed/issues/16969
                                        // Workaround a bug in macOS Sequoia that adds an extra file-extension
                                        // sometimes.
                                        if chunks.len() == 3
                                            && chunks[1].starts_with(chunks[2])
                                            && MacPlatform::os_version()
                                                >= Version::new(15, 0, 0)
                                        {
                                            let new_filename = OsStr::from_bytes(
                                                &filename.as_bytes()
                                                    [..chunks[0].len() + 1 + chunks[1].len()],
                                            )
                                            .to_owned();
                                            result.set_file_name(&new_filename);
                                        }
                                        result
                                    });
                                }
                            }
                        }

                        if let Some(done_tx) = done_tx.take() {
                            let _ = done_tx.send(Ok(result));
                        }
                    });
                    panel.beginWithCompletionHandler(&block);
                }
            })
            .detach();

        done_rx
    }

    fn can_select_mixed_files_and_dirs(&self) -> bool {
        true
    }

    fn reveal_path(&self, path: &Path) {
        let path = path.to_path_buf();
        self.0
            .lock()
            .background_executor
            .spawn(async move {
                let full_path = NSString::from_str(path.to_str().unwrap_or(""));
                let root_full_path = NSString::from_str("");
                let workspace = NSWorkspace::sharedWorkspace();
                workspace.selectFile_inFileViewerRootedAtPath(
                    Some(&full_path),
                    &root_full_path,
                );
            })
            .detach();
    }

    fn open_with_system(&self, path: &Path) {
        let path = path.to_owned();
        self.0
            .lock()
            .background_executor
            .spawn(async move {
                let mut child: Option<smol::process::Child> = new_command("open")
                    .arg(path)
                    .spawn()
                    .context("invoking open command")
                    .log_err();
                if let Some(mut child) = child {
                    child.status().await.log_err();
                }
            })
            .detach();
    }

    fn on_quit(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().quit = Some(callback);
    }

    fn on_reopen(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().reopen = Some(callback);
    }

    fn on_keyboard_layout_change(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().on_keyboard_layout_change = Some(callback);
    }

    fn on_app_menu_action(&self, callback: Box<dyn FnMut(&dyn Action)>) {
        self.0.lock().menu_command = Some(callback);
    }

    fn on_will_open_app_menu(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().will_open_menu = Some(callback);
    }

    fn on_validate_app_menu_command(&self, callback: Box<dyn FnMut(&dyn Action) -> bool>) {
        self.0.lock().validate_menu_command = Some(callback);
    }

    fn on_thermal_state_change(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().on_thermal_state_change = Some(callback);
    }

    fn thermal_state(&self) -> ThermalState {
        let process_info = NSProcessInfo::processInfo();
        let state = process_info.thermalState();
        match state {
            NSProcessInfoThermalState::Nominal => ThermalState::Nominal,
            NSProcessInfoThermalState::Fair => ThermalState::Fair,
            NSProcessInfoThermalState::Serious => ThermalState::Serious,
            NSProcessInfoThermalState::Critical => ThermalState::Critical,
            _ => ThermalState::Nominal,
        }
    }

    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        Box::new(MacKeyboardLayout::new())
    }

    fn keyboard_mapper(&self) -> Rc<dyn PlatformKeyboardMapper> {
        self.0.lock().keyboard_mapper.clone()
    }

    fn app_path(&self) -> Result<PathBuf> {
        let bundle = NSBundle::mainBundle();
        let bundle_path = bundle.bundlePath();
        let path_str = bundle_path.to_string();
        anyhow::ensure!(!path_str.is_empty(), "app is not running inside a bundle");
        Ok(PathBuf::from(path_str))
    }

    fn set_menus(&self, menus: Vec<Menu>, keymap: &Keymap) {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            let app = NSApplication::sharedApplication(mtm);
            let mut state = self.0.lock();
            let actions = &mut state.menu_actions;

            // Get the delegate as our concrete type, then convert to NSMenuDelegate
            let delegate_obj = app.delegate().expect("app delegate should be set");
            let delegate_ref: &GPUIApplicationDelegate =
                &*Retained::as_ptr(&Retained::cast_unchecked(delegate_obj.clone()));
            let delegate: &ProtocolObject<dyn NSMenuDelegate> =
                ProtocolObject::from_ref(delegate_ref);

            let menu = self.create_menu_bar(&menus, delegate, actions, keymap);
            drop(state);
            app.setMainMenu(Some(&menu));
        }
        self.0.lock().menus = Some(menus.into_iter().map(|menu| menu.owned()).collect());
    }

    fn get_menus(&self) -> Option<Vec<OwnedMenu>> {
        self.0.lock().menus.clone()
    }

    fn set_dock_menu(&self, menu: Vec<MenuItem>, keymap: &Keymap) {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            let app = NSApplication::sharedApplication(mtm);
            let mut state = self.0.lock();
            let actions = &mut state.menu_actions;

            let delegate_obj = app.delegate().expect("app delegate should be set");
            let delegate_ref: &GPUIApplicationDelegate =
                &*Retained::as_ptr(&Retained::cast_unchecked(delegate_obj.clone()));
            let delegate: &ProtocolObject<dyn NSMenuDelegate> =
                ProtocolObject::from_ref(delegate_ref);

            let new = self.create_dock_menu(menu, delegate, actions, keymap);
            state.dock_menu = Some(new);
        }
    }

    fn add_recent_document(&self, path: &Path) {
        if let Some(path_str) = path.to_str() {
            unsafe {
                let mtm = MainThreadMarker::new_unchecked();
                let document_controller = NSDocumentController::sharedDocumentController(mtm);
                let url = NSURL::fileURLWithPath(&NSString::from_str(path_str));
                document_controller.noteNewRecentDocumentURL(&url);
            }
        }
    }

    fn path_for_auxiliary_executable(&self, name: &str) -> Result<PathBuf> {
        let bundle = NSBundle::mainBundle();
        let name = NSString::from_str(name);
        let url = bundle
            .URLForAuxiliaryExecutable(&name)
            .context("resource not found")?;
        ns_url_to_path(&url)
    }

    /// Match cursor style to one of the styles available
    /// in macOS's [NSCursor](https://developer.apple.com/documentation/appkit/nscursor).
    fn set_cursor_style(&self, style: CursorStyle) {
        if style == CursorStyle::None {
            NSCursor::setHiddenUntilMouseMoves(true);
            return;
        }

        // The NSCursor resize methods (resizeLeftRightCursor, resizeUpDownCursor, etc.)
        // are deprecated in macOS 15 in favor of columnResizeCursor/rowResizeCursor and
        // columnResizeCursorInDirections/rowResizeCursorInDirections. However, the new
        // APIs require macOS 15+ and our deployment target is 10.15.7, so we must keep
        // using the deprecated methods until the deployment target is raised.
        #[allow(deprecated)]
        let new_cursor: Retained<NSCursor> = match style {
            CursorStyle::Arrow => NSCursor::arrowCursor(),
            CursorStyle::IBeam => NSCursor::IBeamCursor(),
            CursorStyle::Crosshair => NSCursor::crosshairCursor(),
            CursorStyle::ClosedHand => NSCursor::closedHandCursor(),
            CursorStyle::OpenHand => NSCursor::openHandCursor(),
            CursorStyle::PointingHand => NSCursor::pointingHandCursor(),
            CursorStyle::ResizeLeftRight => NSCursor::resizeLeftRightCursor(),
            CursorStyle::ResizeUpDown => NSCursor::resizeUpDownCursor(),
            CursorStyle::ResizeLeft => NSCursor::resizeLeftCursor(),
            CursorStyle::ResizeRight => NSCursor::resizeRightCursor(),
            CursorStyle::ResizeColumn => NSCursor::resizeLeftRightCursor(),
            CursorStyle::ResizeRow => NSCursor::resizeUpDownCursor(),
            CursorStyle::ResizeUp => NSCursor::resizeUpCursor(),
            CursorStyle::ResizeDown => NSCursor::resizeDownCursor(),

            // Undocumented, private class methods:
            // https://stackoverflow.com/questions/27242353/cocoa-predefined-resize-mouse-cursor
            CursorStyle::ResizeUpLeftDownRight => unsafe {
                msg_send![objc2::class!(NSCursor), _windowResizeNorthWestSouthEastCursor]
            },
            CursorStyle::ResizeUpRightDownLeft => unsafe {
                msg_send![objc2::class!(NSCursor), _windowResizeNorthEastSouthWestCursor]
            },

            CursorStyle::IBeamCursorForVerticalLayout => {
                NSCursor::IBeamCursorForVerticalLayout()
            }
            CursorStyle::OperationNotAllowed => NSCursor::operationNotAllowedCursor(),
            CursorStyle::DragLink => NSCursor::dragLinkCursor(),
            CursorStyle::DragCopy => NSCursor::dragCopyCursor(),
            CursorStyle::ContextualMenu => NSCursor::contextualMenuCursor(),
            CursorStyle::None => unreachable!(),
        };

        let old_cursor = NSCursor::currentCursor();
        if *new_cursor != *old_cursor {
            new_cursor.set();
        }
    }

    fn should_auto_hide_scrollbars(&self) -> bool {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            let style = NSScroller::preferredScrollerStyle(mtm);
            style == NSScrollerStyle::Overlay
        }
    }

    fn read_from_clipboard(&self) -> Option<ClipboardItem> {
        let state = self.0.lock();
        state.general_pasteboard.read()
    }

    fn write_to_clipboard(&self, item: ClipboardItem) {
        let state = self.0.lock();
        state.general_pasteboard.write(item);
    }

    fn read_from_find_pasteboard(&self) -> Option<ClipboardItem> {
        let state = self.0.lock();
        state.find_pasteboard.read()
    }

    fn write_to_find_pasteboard(&self, item: ClipboardItem) {
        let state = self.0.lock();
        state.find_pasteboard.write(item);
    }

    fn write_credentials(&self, url: &str, username: &str, password: &[u8]) -> Task<Result<()>> {
        let url = url.to_string();
        let username = username.to_string();
        let password = password.to_vec();
        self.background_executor().spawn(async move {
            unsafe {
                use security::*;

                let url = CFString::from_str(url.as_str());
                let username = CFString::from_str(username.as_str());
                let password = CFData::new(None, password.as_ptr(), password.len() as _)
                    .context("failed to create CFData for password")?;

                // First, check if there are already credentials for the given server. If so, then
                // update the username and password.
                let mut verb = "updating";
                let query_attrs = CFMutableDictionary::new(
                    None, 2,
                    &kCFTypeDictionaryKeyCallBacks,
                    &kCFTypeDictionaryValueCallBacks,
                ).context("failed to create query dictionary")?;
                CFMutableDictionary::set_value(Some(&query_attrs), kSecClass as *const _, kSecClassInternetPassword as *const _);
                CFMutableDictionary::set_value(Some(&query_attrs), kSecAttrServer as *const _, &*url as *const CFString as *const _);

                let attrs = CFMutableDictionary::new(
                    None, 4,
                    &kCFTypeDictionaryKeyCallBacks,
                    &kCFTypeDictionaryValueCallBacks,
                ).context("failed to create attributes dictionary")?;
                CFMutableDictionary::set_value(Some(&attrs), kSecClass as *const _, kSecClassInternetPassword as *const _);
                CFMutableDictionary::set_value(Some(&attrs), kSecAttrServer as *const _, &*url as *const CFString as *const _);
                CFMutableDictionary::set_value(Some(&attrs), kSecAttrAccount as *const _, &*username as *const CFString as *const _);
                CFMutableDictionary::set_value(Some(&attrs), kSecValueData as *const _, &*password as *const CFData as *const _);

                let mut status = SecItemUpdate(
                    &*query_attrs as *const CFMutableDictionary as *const CFDictionary,
                    &*attrs as *const CFMutableDictionary as *const CFDictionary,
                );

                // If there were no existing credentials for the given server, then create them.
                if status == errSecItemNotFound {
                    verb = "creating";
                    status = SecItemAdd(
                        &*attrs as *const CFMutableDictionary as *const CFDictionary,
                        ptr::null_mut(),
                    );
                }
                anyhow::ensure!(status == errSecSuccess, "{verb} password failed: {status}");
            }
            Ok(())
        })
    }

    fn read_credentials(&self, url: &str) -> Task<Result<Option<(String, Vec<u8>)>>> {
        let url = url.to_string();
        self.background_executor().spawn(async move {
            let url = CFString::from_str(url.as_str());
            let cf_true = unsafe { kCFBooleanTrue }
                .expect("kCFBooleanTrue should be available");

            unsafe {
                use security::*;

                // Find any credentials for the given server URL.
                let attrs = CFMutableDictionary::new(
                    None, 5,
                    &kCFTypeDictionaryKeyCallBacks,
                    &kCFTypeDictionaryValueCallBacks,
                ).context("failed to create query dictionary")?;
                CFMutableDictionary::set_value(Some(&attrs), kSecClass as *const _, kSecClassInternetPassword as *const _);
                CFMutableDictionary::set_value(Some(&attrs), kSecAttrServer as *const _, &*url as *const CFString as *const _);
                CFMutableDictionary::set_value(Some(&attrs), kSecReturnAttributes as *const _, cf_true as *const CFBoolean as *const _);
                CFMutableDictionary::set_value(Some(&attrs), kSecReturnData as *const _, cf_true as *const CFBoolean as *const _);

                let mut result: *const c_void = ptr::null();
                let status = SecItemCopyMatching(
                    &*attrs as *const CFMutableDictionary as *const CFDictionary,
                    &mut result,
                );
                match status {
                    security::errSecSuccess => {}
                    security::errSecItemNotFound | security::errSecUserCanceled => return Ok(None),
                    _ => anyhow::bail!("reading password failed: {status}"),
                }

                anyhow::ensure!(!result.is_null(), "keychain returned null result");

                // The result is a CFDictionary (owned, follows the Create Rule).
                let result_dict = &*(result as *const CFDictionary);

                let username_ptr = result_dict.value(kSecAttrAccount as *const _);
                anyhow::ensure!(!username_ptr.is_null(), "account was missing from keychain item");
                let username_cf = &*(username_ptr as *const CFString);
                let username = username_cf.to_string();

                let password_ptr = result_dict.value(kSecValueData as *const _);
                anyhow::ensure!(!password_ptr.is_null(), "password was missing from keychain item");
                let password_cf = &*(password_ptr as *const CFData);
                let password_bytes = password_cf.as_bytes_unchecked().to_vec();

                // Release the result dictionary (we own it from SecItemCopyMatching)
                CFRelease(result);

                Ok(Some((username, password_bytes)))
            }
        })
    }

    fn delete_credentials(&self, url: &str) -> Task<Result<()>> {
        let url = url.to_string();

        self.background_executor().spawn(async move {
            unsafe {
                use security::*;

                let url = CFString::from_str(url.as_str());
                let query_attrs = CFMutableDictionary::new(
                    None, 2,
                    &kCFTypeDictionaryKeyCallBacks,
                    &kCFTypeDictionaryValueCallBacks,
                ).context("failed to create query dictionary")?;
                CFMutableDictionary::set_value(Some(&query_attrs), kSecClass as *const _, kSecClassInternetPassword as *const _);
                CFMutableDictionary::set_value(Some(&query_attrs), kSecAttrServer as *const _, &*url as *const CFString as *const _);

                let status = SecItemDelete(
                    &*query_attrs as *const CFMutableDictionary as *const CFDictionary,
                );
                anyhow::ensure!(status == errSecSuccess, "delete password failed: {status}");
            }
            Ok(())
        })
    }
}

unsafe extern "C" {
    fn CFRelease(cf: *const c_void);
}
