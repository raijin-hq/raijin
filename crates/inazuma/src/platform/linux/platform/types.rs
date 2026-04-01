use super::*;

pub(crate) const SCROLL_LINES: f32 = 3.0;

// Values match the defaults on GTK.
// Taken from https://github.com/GNOME/gtk/blob/main/gtk/gtksettings.c#L320
#[cfg(any(feature = "wayland", feature = "x11"))]
pub(crate) const DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(400);
#[cfg(any(feature = "wayland", feature = "x11"))]
pub(crate) const DOUBLE_CLICK_DISTANCE: Pixels = px(5.0);
pub(crate) const KEYRING_LABEL: &str = "zed-github-account";

#[cfg(any(feature = "wayland", feature = "x11"))]
pub(super) const FILE_PICKER_PORTAL_MISSING: &str =
    "Couldn't open file picker due to missing xdg-desktop-portal implementation.";

pub(crate) trait LinuxClient {
    fn compositor_name(&self) -> &'static str;
    fn with_common<R>(&self, f: impl FnOnce(&mut LinuxCommon) -> R) -> R;
    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout>;
    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>>;
    #[allow(unused)]
    fn display(&self, id: DisplayId) -> Option<Rc<dyn PlatformDisplay>>;
    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>>;

    #[cfg(feature = "screen-capture")]
    fn is_screen_capture_supported(&self) -> bool {
        true
    }

    #[cfg(feature = "screen-capture")]
    fn screen_capture_sources(
        &self,
    ) -> oneshot::Receiver<Result<Vec<Rc<dyn inazuma::ScreenCaptureSource>>>> {
        let (sources_tx, sources_rx) = oneshot::channel();
        sources_tx
            .send(Err(anyhow::anyhow!(
                "gpui_linux was compiled without the screen-capture feature"
            )))
            .ok();
        sources_rx
    }

    fn open_window(
        &self,
        handle: AnyWindowHandle,
        options: WindowParams,
    ) -> anyhow::Result<Box<dyn PlatformWindow>>;
    fn set_cursor_style(&self, style: CursorStyle);
    fn open_uri(&self, uri: &str);
    fn reveal_path(&self, path: PathBuf);
    fn write_to_primary(&self, item: ClipboardItem);
    fn write_to_clipboard(&self, item: ClipboardItem);
    fn read_from_primary(&self) -> Option<ClipboardItem>;
    fn read_from_clipboard(&self) -> Option<ClipboardItem>;
    fn active_window(&self) -> Option<AnyWindowHandle>;
    fn window_stack(&self) -> Option<Vec<AnyWindowHandle>>;
    fn run(&self);

    #[cfg(any(feature = "wayland", feature = "x11"))]
    fn window_identifier(
        &self,
    ) -> impl Future<Output = Option<ashpd::WindowIdentifier>> + Send + 'static {
        std::future::ready::<Option<ashpd::WindowIdentifier>>(None)
    }
}

#[derive(Default)]
pub(crate) struct PlatformHandlers {
    pub(crate) open_urls: Option<Box<dyn FnMut(Vec<String>)>>,
    pub(crate) quit: Option<Box<dyn FnMut()>>,
    pub(crate) reopen: Option<Box<dyn FnMut()>>,
    pub(crate) app_menu_action: Option<Box<dyn FnMut(&dyn Action)>>,
    pub(crate) will_open_app_menu: Option<Box<dyn FnMut()>>,
    pub(crate) validate_app_menu_command: Option<Box<dyn FnMut(&dyn Action) -> bool>>,
    pub(crate) keyboard_layout_change: Option<Box<dyn FnMut()>>,
}

pub(crate) struct LinuxCommon {
    pub(crate) background_executor: BackgroundExecutor,
    pub(crate) foreground_executor: ForegroundExecutor,
    pub(crate) text_system: Arc<dyn PlatformTextSystem>,
    pub(crate) appearance: WindowAppearance,
    pub(crate) auto_hide_scrollbars: bool,
    pub(crate) callbacks: PlatformHandlers,
    pub(crate) signal: LoopSignal,
    pub(crate) menus: Vec<OwnedMenu>,
}

impl LinuxCommon {
    pub fn new(signal: LoopSignal) -> (Self, PriorityQueueCalloopReceiver<RunnableVariant>) {
        let (main_sender, main_receiver) = PriorityQueueCalloopReceiver::new();

        #[cfg(any(feature = "wayland", feature = "x11"))]
        let text_system = Arc::new(crate::platform::linux::CosmicTextSystem::new(
            "IBM Plex Sans",
        ));
        #[cfg(not(any(feature = "wayland", feature = "x11")))]
        let text_system = Arc::new(inazuma::NoopTextSystem::new());

        let callbacks = PlatformHandlers::default();

        let dispatcher = Arc::new(LinuxDispatcher::new(main_sender));

        let background_executor = BackgroundExecutor::new(dispatcher.clone());

        let common = LinuxCommon {
            background_executor,
            foreground_executor: ForegroundExecutor::new(dispatcher),
            text_system,
            appearance: WindowAppearance::Light,
            auto_hide_scrollbars: false,
            callbacks,
            signal,
            menus: Vec::new(),
        };

        (common, main_receiver)
    }
}

pub(crate) struct LinuxPlatform<P> {
    pub(crate) inner: P,
}

