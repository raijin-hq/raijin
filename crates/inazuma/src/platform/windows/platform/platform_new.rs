use super::*;

impl WindowsPlatform {
    pub fn new(headless: bool) -> Result<Self> {
        unsafe {
            OleInitialize(None).context("unable to initialize Windows OLE")?;
        }
        let (directx_devices, text_system, direct_write_text_system) = if !headless {
            let devices = DirectXDevices::new().context("Creating DirectX devices")?;
            let dw_text_system = Arc::new(
                DirectWriteTextSystem::new(&devices)
                    .context("Error creating DirectWriteTextSystem")?,
            );
            (
                Some(devices),
                dw_text_system.clone() as Arc<dyn PlatformTextSystem>,
                Some(dw_text_system),
            )
        } else {
            (
                None,
                Arc::new(inazuma::NoopTextSystem::new()) as Arc<dyn PlatformTextSystem>,
                None,
            )
        };

        let (main_sender, main_receiver) = PriorityQueueReceiver::new();
        let validation_number = if usize::BITS == 64 {
            rand::random::<u64>() as usize
        } else {
            rand::random::<u32>() as usize
        };
        let raw_window_handles = Arc::new(RwLock::new(SmallVec::new()));

        register_platform_window_class();
        let mut context = PlatformWindowCreateContext {
            inner: None,
            raw_window_handles: Arc::downgrade(&raw_window_handles),
            validation_number,
            main_sender: Some(main_sender),
            main_receiver: Some(main_receiver),
            directx_devices,
            dispatcher: None,
        };
        let result = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PLATFORM_WINDOW_CLASS_NAME,
                None,
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                None,
                Some(&raw const context as *const _),
            )
        };
        let inner = context
            .inner
            .take()
            .context("CreateWindowExW did not run correctly")??;
        let dispatcher = context
            .dispatcher
            .take()
            .context("CreateWindowExW did not run correctly")?;
        let handle = result?;

        let disable_direct_composition = std::env::var(DISABLE_DIRECT_COMPOSITION)
            .is_ok_and(|value| value == "true" || value == "1");
        let background_executor = BackgroundExecutor::new(dispatcher.clone());
        let foreground_executor = ForegroundExecutor::new(dispatcher);

        let drop_target_helper: Option<IDropTargetHelper> = if !headless {
            Some(unsafe {
                CoCreateInstance(&CLSID_DragDropHelper, None, CLSCTX_INPROC_SERVER)
                    .context("Error creating drop target helper.")?
            })
        } else {
            None
        };
        let icon = if !headless {
            load_icon().unwrap_or_default()
        } else {
            HICON::default()
        };

        Ok(Self {
            inner,
            handle,
            raw_window_handles,
            headless,
            icon,
            background_executor,
            foreground_executor,
            text_system,
            direct_write_text_system,
            disable_direct_composition,
            drop_target_helper,
            invalidate_devices: Arc::new(AtomicBool::new(false)),
        })
    }

    pub(crate) fn window_from_hwnd(&self, hwnd: HWND) -> Option<Rc<WindowsWindowInner>> {
        self.raw_window_handles
            .read()
            .iter()
            .find(|entry| entry.as_raw() == hwnd)
            .and_then(|hwnd| window_from_hwnd(hwnd.as_raw()))
    }

    #[inline]
    fn post_message(&self, message: u32, wparam: WPARAM, lparam: LPARAM) {
        self.raw_window_handles
            .read()
            .iter()
            .for_each(|handle| unsafe {
                PostMessageW(Some(handle.as_raw()), message, wparam, lparam).log_err();
            });
    }

    fn generate_creation_info(&self) -> WindowCreationInfo {
        WindowCreationInfo {
            icon: self.icon,
            executor: self.foreground_executor.clone(),
            current_cursor: self.inner.state.current_cursor.get(),
            drop_target_helper: self.drop_target_helper.clone().unwrap(),
            validation_number: self.inner.validation_number,
            main_receiver: self.inner.main_receiver.clone(),
            platform_window_handle: self.handle,
            disable_direct_composition: self.disable_direct_composition,
            directx_devices: self.inner.state.directx_devices.borrow().clone().unwrap(),
            invalidate_devices: self.invalidate_devices.clone(),
        }
    }

    fn set_dock_menus(&self, menus: Vec<MenuItem>) {
        let mut actions = Vec::new();
        menus.into_iter().for_each(|menu| {
            if let Some(dock_menu) = DockMenuItem::new(menu).log_err() {
                actions.push(dock_menu);
            }
        });
        self.inner.state.jump_list.borrow_mut().dock_menus = actions;
        let borrow = self.inner.state.jump_list.borrow();
        let dock_menus = borrow
            .dock_menus
            .iter()
            .map(|menu| (menu.name.clone(), menu.description.clone()))
            .collect::<Vec<_>>();
        let recent_workspaces = borrow.recent_workspaces.clone();
        self.background_executor
            .spawn(async move {
                update_jump_list(&recent_workspaces, &dock_menus).log_err();
            })
            .detach();
    }

    fn update_jump_list(
        &self,
        menus: Vec<MenuItem>,
        entries: Vec<SmallVec<[PathBuf; 2]>>,
    ) -> Task<Vec<SmallVec<[PathBuf; 2]>>> {
        let mut actions = Vec::new();
        menus.into_iter().for_each(|menu| {
            if let Some(dock_menu) = DockMenuItem::new(menu).log_err() {
                actions.push(dock_menu);
            }
        });
        let mut jump_list = self.inner.state.jump_list.borrow_mut();
        jump_list.dock_menus = actions;
        jump_list.recent_workspaces = entries.into();
        let dock_menus = jump_list
            .dock_menus
            .iter()
            .map(|menu| (menu.name.clone(), menu.description.clone()))
            .collect::<Vec<_>>();
        let recent_workspaces = jump_list.recent_workspaces.clone();
        self.background_executor.spawn(async move {
            update_jump_list(&recent_workspaces, &dock_menus)
                .log_err()
                .unwrap_or_default()
        })
    }

    fn find_current_active_window(&self) -> Option<HWND> {
        let active_window_hwnd = unsafe { GetActiveWindow() };
        if active_window_hwnd.is_invalid() {
            return None;
        }
        self.raw_window_handles
            .read()
            .iter()
            .find(|hwnd| hwnd.as_raw() == active_window_hwnd)
            .map(|hwnd| hwnd.as_raw())
    }

    fn begin_vsync_thread(&self) {
        let Some(directx_devices) = self.inner.state.directx_devices.borrow().clone() else {
            return;
        };
        let Some(direct_write_text_system) = &self.direct_write_text_system else {
            return;
        };
        let mut directx_device = directx_devices;
        let platform_window: SafeHwnd = self.handle.into();
        let validation_number = self.inner.validation_number;
        let all_windows = Arc::downgrade(&self.raw_window_handles);
        let text_system = Arc::downgrade(direct_write_text_system);
        let invalidate_devices = self.invalidate_devices.clone();

        std::thread::Builder::new()
            .name("VSyncProvider".to_owned())
            .spawn(move || {
                let vsync_provider = VSyncProvider::new();
                loop {
                    vsync_provider.wait_for_vsync();
                    if check_device_lost(&directx_device.device)
                        || invalidate_devices.fetch_and(false, Ordering::Acquire)
                    {
                        if let Err(err) = handle_gpu_device_lost(
                            &mut directx_device,
                            platform_window.as_raw(),
                            validation_number,
                            &all_windows,
                            &text_system,
                        ) {
                            panic!("Device lost: {err}");
                        }
                    }
                    let Some(all_windows) = all_windows.upgrade() else {
                        break;
                    };
                    for hwnd in all_windows.read().iter() {
                        unsafe {
                            let _ = RedrawWindow(Some(hwnd.as_raw()), None, None, RDW_INVALIDATE);
                        }
                    }
                }
            })
            .unwrap();
    }
}

pub(super) fn translate_accelerator(msg: &MSG) -> Option<()> {
    if msg.message != WM_KEYDOWN && msg.message != WM_SYSKEYDOWN {
        return None;
    }

    let result = unsafe {
        SendMessageW(
            msg.hwnd,
            WM_GPUI_KEYDOWN,
            Some(msg.wParam),
            Some(msg.lParam),
        )
    };
    (result.0 == 0).then_some(())
}

