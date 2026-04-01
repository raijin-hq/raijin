use super::*;

pub struct WindowsPlatform {
    pub(super) inner: Rc<WindowsPlatformInner>,
    pub(super) raw_window_handles: Arc<RwLock<SmallVec<[SafeHwnd; 4]>>>,
    // The below members will never change throughout the entire lifecycle of the app.
    pub(super) headless: bool,
    pub(super) icon: HICON,
    pub(super) background_executor: BackgroundExecutor,
    pub(super) foreground_executor: ForegroundExecutor,
    pub(super) text_system: Arc<dyn PlatformTextSystem>,
    pub(super) direct_write_text_system: Option<Arc<DirectWriteTextSystem>>,
    pub(super) drop_target_helper: Option<IDropTargetHelper>,
    /// Flag to instruct the `VSyncProvider` thread to invalidate the directx devices
    /// as resizing them has failed, causing us to have lost at least the render target.
    pub(super) invalidate_devices: Arc<AtomicBool>,
    pub(super) handle: HWND,
    pub(super) disable_direct_composition: bool,
}

pub(super) struct WindowsPlatformInner {
    pub(super) state: WindowsPlatformState,
    pub(super) raw_window_handles: std::sync::Weak<RwLock<SmallVec<[SafeHwnd; 4]>>>,
    // The below members will never change throughout the entire lifecycle of the app.
    pub(super) validation_number: usize,
    pub(super) main_receiver: PriorityQueueReceiver<RunnableVariant>,
    pub(super) dispatcher: Arc<WindowsDispatcher>,
}

pub(crate) struct WindowsPlatformState {
    pub(super) callbacks: PlatformCallbacks,
    pub(super) menus: RefCell<Vec<OwnedMenu>>,
    pub(super) jump_list: RefCell<JumpList>,
    // NOTE: standard cursor handles don't need to close.
    pub(crate) current_cursor: Cell<Option<HCURSOR>>,
    pub(super) directx_devices: RefCell<Option<DirectXDevices>>,
}

#[derive(Default)]
pub(super) struct PlatformCallbacks {
    pub(super) open_urls: Cell<Option<Box<dyn FnMut(Vec<String>)>>>,
    pub(super) quit: Cell<Option<Box<dyn FnMut()>>>,
    pub(super) reopen: Cell<Option<Box<dyn FnMut()>>>,
    pub(super) app_menu_action: Cell<Option<Box<dyn FnMut(&dyn Action)>>>,
    pub(super) will_open_app_menu: Cell<Option<Box<dyn FnMut()>>>,
    pub(super) validate_app_menu_command: Cell<Option<Box<dyn FnMut(&dyn Action) -> bool>>>,
    pub(super) keyboard_layout_change: Cell<Option<Box<dyn FnMut()>>>,
}

impl WindowsPlatformState {
    fn new(directx_devices: Option<DirectXDevices>) -> Self {
        let callbacks = PlatformCallbacks::default();
        let jump_list = JumpList::new();
        let current_cursor = load_cursor(CursorStyle::Arrow);

        Self {
            callbacks,
            jump_list: RefCell::new(jump_list),
            current_cursor: Cell::new(current_cursor),
            directx_devices: RefCell::new(directx_devices),
            menus: RefCell::new(Vec::new()),
        }
    }
}

