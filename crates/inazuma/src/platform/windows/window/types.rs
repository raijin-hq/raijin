use super::*;

pub(crate) struct WindowsWindow(pub Rc<WindowsWindowInner>);

pub(super) impl std::ops::Deref for WindowsWindow {
    type Target = WindowsWindowInner;

    pub(super) fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct WindowsWindowState {
    pub origin: Cell<Point<Pixels>>,
    pub logical_size: Cell<Size<Pixels>>,
    pub min_size: Option<Size<Pixels>>,
    pub fullscreen_restore_bounds: Cell<Bounds<Pixels>>,
    pub border_offset: WindowBorderOffset,
    pub appearance: Cell<WindowAppearance>,
    pub background_appearance: Cell<WindowBackgroundAppearance>,
    pub scale_factor: Cell<f32>,
    pub restore_from_minimized: Cell<Option<Box<dyn FnMut(RequestFrameOptions)>>>,

    pub callbacks: Callbacks,
    pub input_handler: Cell<Option<PlatformInputHandler>>,
    pub ime_enabled: Cell<bool>,
    pub pending_surrogate: Cell<Option<u16>>,
    pub last_reported_modifiers: Cell<Option<Modifiers>>,
    pub last_reported_capslock: Cell<Option<Capslock>>,
    pub hovered: Cell<bool>,

    pub renderer: RefCell<DirectXRenderer>,

    pub click_state: ClickState,
    pub current_cursor: Cell<Option<HCURSOR>>,
    pub nc_button_pressed: Cell<Option<u32>>,

    pub display: Cell<WindowsDisplay>,
    /// Flag to instruct the `VSyncProvider` thread to invalidate the directx devices
    /// as resizing them has failed, causing us to have lost at least the render target.
    pub invalidate_devices: Arc<AtomicBool>,
    pub(super) fullscreen: Cell<Option<StyleAndBounds>>,
    pub(super) initial_placement: Cell<Option<WindowOpenStatus>>,
    pub(super) hwnd: HWND,
}

pub(crate) struct WindowsWindowInner {
    pub(super) hwnd: HWND,
    pub(super) drop_target_helper: IDropTargetHelper,
    pub(crate) state: WindowsWindowState,
    pub(super) system_settings: WindowsSystemSettings,
    pub(crate) handle: AnyWindowHandle,
    pub(crate) hide_title_bar: bool,
    pub(crate) is_movable: bool,
    pub(crate) executor: ForegroundExecutor,
    pub(crate) validation_number: usize,
    pub(crate) main_receiver: PriorityQueueReceiver<RunnableVariant>,
    pub(crate) platform_window_handle: HWND,
    pub(crate) parent_hwnd: Option<HWND>,
}

impl WindowsWindowState {
    fn new(
        hwnd: HWND,
        directx_devices: &DirectXDevices,
        window_params: &CREATESTRUCTW,
        current_cursor: Option<HCURSOR>,
        display: WindowsDisplay,
        min_size: Option<Size<Pixels>>,
        appearance: WindowAppearance,
        disable_direct_composition: bool,
        invalidate_devices: Arc<AtomicBool>,
    ) -> Result<Self> {
        let scale_factor = {
            let monitor_dpi = unsafe { GetDpiForWindow(hwnd) } as f32;
            monitor_dpi / USER_DEFAULT_SCREEN_DPI as f32
        };
        let origin = logical_point(window_params.x as f32, window_params.y as f32, scale_factor);
        let logical_size = {
            let physical_size = size(
                DevicePixels(window_params.cx),
                DevicePixels(window_params.cy),
            );
            physical_size.to_pixels(scale_factor)
        };
        let fullscreen_restore_bounds = Bounds {
            origin,
            size: logical_size,
        };
        let border_offset = WindowBorderOffset::default();
        let restore_from_minimized = None;
        let renderer = DirectXRenderer::new(hwnd, directx_devices, disable_direct_composition)
            .context("Creating DirectX renderer")?;
        let callbacks = Callbacks::default();
        let input_handler = None;
        let pending_surrogate = None;
        let last_reported_modifiers = None;
        let last_reported_capslock = None;
        let hovered = false;
        let click_state = ClickState::new();
        let nc_button_pressed = None;
        let fullscreen = None;
        let initial_placement = None;

        Ok(Self {
            origin: Cell::new(origin),
            logical_size: Cell::new(logical_size),
            fullscreen_restore_bounds: Cell::new(fullscreen_restore_bounds),
            border_offset,
            appearance: Cell::new(appearance),
            background_appearance: Cell::new(WindowBackgroundAppearance::Opaque),
            scale_factor: Cell::new(scale_factor),
            restore_from_minimized: Cell::new(restore_from_minimized),
            min_size,
            callbacks,
            input_handler: Cell::new(input_handler),
            ime_enabled: Cell::new(true),
            pending_surrogate: Cell::new(pending_surrogate),
            last_reported_modifiers: Cell::new(last_reported_modifiers),
            last_reported_capslock: Cell::new(last_reported_capslock),
            hovered: Cell::new(hovered),
            renderer: RefCell::new(renderer),
            click_state,
            current_cursor: Cell::new(current_cursor),
            nc_button_pressed: Cell::new(nc_button_pressed),
            display: Cell::new(display),
            fullscreen: Cell::new(fullscreen),
            initial_placement: Cell::new(initial_placement),
            hwnd,
            invalidate_devices,
        })
    }

    #[inline]
    pub(crate) fn is_fullscreen(&self) -> bool {
        self.fullscreen.get().is_some()
    }

    pub(crate) fn is_maximized(&self) -> bool {
        !self.is_fullscreen() && unsafe { IsZoomed(self.hwnd) }.as_bool()
    }

    fn bounds(&self) -> Bounds<Pixels> {
        Bounds {
            origin: self.origin.get(),
            size: self.logical_size.get(),
        }
    }

    // Calculate the bounds used for saving and whether the window is maximized.
    fn calculate_window_bounds(&self) -> (Bounds<Pixels>, bool) {
        let placement = unsafe {
            let mut placement = WINDOWPLACEMENT {
                length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
                ..Default::default()
            };
            GetWindowPlacement(self.hwnd, &mut placement)
                .context("failed to get window placement")
                .log_err();
            placement
        };
        (
            calculate_client_rect(
                placement.rcNormalPosition,
                &self.border_offset,
                self.scale_factor.get(),
            ),
            placement.showCmd == SW_SHOWMAXIMIZED.0 as u32,
        )
    }

    fn window_bounds(&self) -> WindowBounds {
        let (bounds, maximized) = self.calculate_window_bounds();

        if self.is_fullscreen() {
            WindowBounds::Fullscreen(self.fullscreen_restore_bounds.get())
        } else if maximized {
            WindowBounds::Maximized(bounds)
        } else {
            WindowBounds::Windowed(bounds)
        }
    }

    /// get the logical size of the app's drawable area.
    ///
    /// Currently, GPUI uses the logical size of the app to handle mouse interactions (such as
    /// whether the mouse collides with other elements of GPUI).
    fn content_size(&self) -> Size<Pixels> {
        self.logical_size.get()
    }
}

impl WindowsWindowInner {
    fn new(context: &mut WindowCreateContext, hwnd: HWND, cs: &CREATESTRUCTW) -> Result<Rc<Self>> {
        let state = WindowsWindowState::new(
            hwnd,
            &context.directx_devices,
            cs,
            context.current_cursor,
            context.display,
            context.min_size,
            context.appearance,
            context.disable_direct_composition,
            context.invalidate_devices.clone(),
        )?;

        Ok(Rc::new(Self {
            hwnd,
            drop_target_helper: context.drop_target_helper.clone(),
            state,
            handle: context.handle,
            hide_title_bar: context.hide_title_bar,
            is_movable: context.is_movable,
            executor: context.executor.clone(),
            validation_number: context.validation_number,
            main_receiver: context.main_receiver.clone(),
            platform_window_handle: context.platform_window_handle,
            system_settings: WindowsSystemSettings::new(),
            parent_hwnd: context.parent_hwnd,
        }))
    }

    fn toggle_fullscreen(self: &Rc<Self>) {
        let this = self.clone();
        self.executor
            .spawn(async move {
                let StyleAndBounds {
                    style,
                    x,
                    y,
                    cx,
                    cy,
                } = match this.state.fullscreen.take() {
                    Some(state) => state,
                    None => {
                        let (window_bounds, _) = this.state.calculate_window_bounds();
                        this.state.fullscreen_restore_bounds.set(window_bounds);

                        let style =
                            WINDOW_STYLE(unsafe { get_window_long(this.hwnd, GWL_STYLE) } as _);
                        let mut rc = RECT::default();
                        unsafe { GetWindowRect(this.hwnd, &mut rc) }
                            .context("failed to get window rect")
                            .log_err();
                        let _ = this.state.fullscreen.set(Some(StyleAndBounds {
                            style,
                            x: rc.left,
                            y: rc.top,
                            cx: rc.right - rc.left,
                            cy: rc.bottom - rc.top,
                        }));
                        let style = style
                            & !(WS_THICKFRAME
                                | WS_SYSMENU
                                | WS_MAXIMIZEBOX
                                | WS_MINIMIZEBOX
                                | WS_CAPTION);
                        let physical_bounds = this.state.display.get().physical_bounds();
                        StyleAndBounds {
                            style,
                            x: physical_bounds.left().0,
                            y: physical_bounds.top().0,
                            cx: physical_bounds.size.width.0,
                            cy: physical_bounds.size.height.0,
                        }
                    }
                };
                set_non_rude_hwnd(this.hwnd, !this.state.is_fullscreen());
                unsafe { set_window_long(this.hwnd, GWL_STYLE, style.0 as isize) };
                unsafe {
                    SetWindowPos(
                        this.hwnd,
                        None,
                        x,
                        y,
                        cx,
                        cy,
                        SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOZORDER,
                    )
                }
                .log_err();
            })
            .detach();
    }

    fn set_window_placement(self: &Rc<Self>) -> Result<()> {
        let Some(open_status) = self.state.initial_placement.take() else {
            return Ok(());
        };
        match open_status.state {
            WindowOpenState::Maximized => unsafe {
                SetWindowPlacement(self.hwnd, &open_status.placement)
                    .context("failed to set window placement")?;
                ShowWindowAsync(self.hwnd, SW_MAXIMIZE).ok()?;
            },
            WindowOpenState::Fullscreen => {
                unsafe {
                    SetWindowPlacement(self.hwnd, &open_status.placement)
                        .context("failed to set window placement")?
                };
                self.toggle_fullscreen();
            }
            WindowOpenState::Windowed => unsafe {
                SetWindowPlacement(self.hwnd, &open_status.placement)
                    .context("failed to set window placement")?;
            },
        }
        Ok(())
    }

    pub(crate) fn system_settings(&self) -> &WindowsSystemSettings {
        &self.system_settings
    }
}

#[derive(Default)]
pub(crate) struct Callbacks {
    pub(crate) request_frame: Cell<Option<Box<dyn FnMut(RequestFrameOptions)>>>,
    pub(crate) input: Cell<Option<Box<dyn FnMut(PlatformInput) -> DispatchEventResult>>>,
    pub(crate) active_status_change: Cell<Option<Box<dyn FnMut(bool)>>>,
    pub(crate) hovered_status_change: Cell<Option<Box<dyn FnMut(bool)>>>,
    pub(crate) resize: Cell<Option<Box<dyn FnMut(Size<Pixels>, f32)>>>,
    pub(crate) moved: Cell<Option<Box<dyn FnMut()>>>,
    pub(crate) should_close: Cell<Option<Box<dyn FnMut() -> bool>>>,
    pub(crate) close: Cell<Option<Box<dyn FnOnce()>>>,
    pub(crate) hit_test_window_control: Cell<Option<Box<dyn FnMut() -> Option<WindowControlArea>>>>,
    pub(crate) appearance_changed: Cell<Option<Box<dyn FnMut()>>>,
}

pub(super) struct WindowCreateContext {
    pub(super) inner: Option<Result<Rc<WindowsWindowInner>>>,
    pub(super) handle: AnyWindowHandle,
    pub(super) hide_title_bar: bool,
    pub(super) display: WindowsDisplay,
    pub(super) is_movable: bool,
    pub(super) min_size: Option<Size<Pixels>>,
    pub(super) executor: ForegroundExecutor,
    pub(super) current_cursor: Option<HCURSOR>,
    pub(super) drop_target_helper: IDropTargetHelper,
    pub(super) validation_number: usize,
    pub(super) main_receiver: PriorityQueueReceiver<RunnableVariant>,
    pub(super) platform_window_handle: HWND,
    pub(super) appearance: WindowAppearance,
    pub(super) disable_direct_composition: bool,
    pub(super) directx_devices: DirectXDevices,
    pub(super) invalidate_devices: Arc<AtomicBool>,
    pub(super) parent_hwnd: Option<HWND>,
}

