use super::*;

pub(super) struct WindowsDragDropHandler(pub Rc<WindowsWindowInner>);

impl WindowsDragDropHandler {
    pub(super) fn handle_drag_drop(&self, input: PlatformInput) {
        if let Some(mut func) = self.0.state.callbacks.input.take() {
            func(input);
            self.0.state.callbacks.input.set(Some(func));
        }
    }
}

#[allow(non_snake_case)]
impl IDropTarget_Impl for WindowsDragDropHandler_Impl {
    fn DragEnter(
        &self,
        pdataobj: windows::core::Ref<IDataObject>,
        _grfkeystate: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        unsafe {
            let idata_obj = pdataobj.ok()?;
            let config = FORMATETC {
                cfFormat: CF_HDROP.0,
                ptd: std::ptr::null_mut() as _,
                dwAspect: DVASPECT_CONTENT.0,
                lindex: -1,
                tymed: TYMED_HGLOBAL.0 as _,
            };
            let cursor_position = POINT { x: pt.x, y: pt.y };
            if idata_obj.QueryGetData(&config as _) == S_OK {
                *pdweffect = DROPEFFECT_COPY;
                let Some(mut idata) = idata_obj.GetData(&config as _).log_err() else {
                    return Ok(());
                };
                if idata.u.hGlobal.is_invalid() {
                    return Ok(());
                }
                let hdrop = HDROP(idata.u.hGlobal.0);
                let mut paths = SmallVec::<[PathBuf; 2]>::new();
                with_file_names(hdrop, |file_name| {
                    if let Some(path) = PathBuf::from_str(&file_name).log_err() {
                        paths.push(path);
                    }
                });
                ReleaseStgMedium(&mut idata);
                let mut cursor_position = cursor_position;
                ScreenToClient(self.0.hwnd, &mut cursor_position)
                    .ok()
                    .log_err();
                let scale_factor = self.0.state.scale_factor.get();
                let input = PlatformInput::FileDrop(FileDropEvent::Entered {
                    position: logical_point(
                        cursor_position.x as f32,
                        cursor_position.y as f32,
                        scale_factor,
                    ),
                    paths: ExternalPaths(paths),
                });
                self.handle_drag_drop(input);
            } else {
                *pdweffect = DROPEFFECT_NONE;
            }
            self.0
                .drop_target_helper
                .DragEnter(self.0.hwnd, idata_obj, &cursor_position, *pdweffect)
                .log_err();
        }
        Ok(())
    }

    fn DragOver(
        &self,
        _grfkeystate: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let mut cursor_position = POINT { x: pt.x, y: pt.y };
        unsafe {
            *pdweffect = DROPEFFECT_COPY;
            self.0
                .drop_target_helper
                .DragOver(&cursor_position, *pdweffect)
                .log_err();
            ScreenToClient(self.0.hwnd, &mut cursor_position)
                .ok()
                .log_err();
        }
        let scale_factor = self.0.state.scale_factor.get();
        let input = PlatformInput::FileDrop(FileDropEvent::Pending {
            position: logical_point(
                cursor_position.x as f32,
                cursor_position.y as f32,
                scale_factor,
            ),
        });
        self.handle_drag_drop(input);

        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        unsafe {
            self.0.drop_target_helper.DragLeave().log_err();
        }
        let input = PlatformInput::FileDrop(FileDropEvent::Exited);
        self.handle_drag_drop(input);

        Ok(())
    }

    fn Drop(
        &self,
        pdataobj: windows::core::Ref<IDataObject>,
        _grfkeystate: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let idata_obj = pdataobj.ok()?;
        let mut cursor_position = POINT { x: pt.x, y: pt.y };
        unsafe {
            *pdweffect = DROPEFFECT_COPY;
            self.0
                .drop_target_helper
                .Drop(idata_obj, &cursor_position, *pdweffect)
                .log_err();
            ScreenToClient(self.0.hwnd, &mut cursor_position)
                .ok()
                .log_err();
        }
        let scale_factor = self.0.state.scale_factor.get();
        let input = PlatformInput::FileDrop(FileDropEvent::Submit {
            position: logical_point(
                cursor_position.x as f32,
                cursor_position.y as f32,
                scale_factor,
            ),
        });
        self.handle_drag_drop(input);

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ClickState {
    pub(super) button: Cell<MouseButton>,
    pub(super) last_click: Cell<Instant>,
    pub(super) last_position: Cell<Point<DevicePixels>>,
    pub(super) double_click_spatial_tolerance_width: Cell<i32>,
    pub(super) double_click_spatial_tolerance_height: Cell<i32>,
    pub(super) double_click_interval: Cell<Duration>,
    pub(crate) current_count: Cell<usize>,
}

impl ClickState {
    pub fn new() -> Self {
        let double_click_spatial_tolerance_width = unsafe { GetSystemMetrics(SM_CXDOUBLECLK) };
        let double_click_spatial_tolerance_height = unsafe { GetSystemMetrics(SM_CYDOUBLECLK) };
        let double_click_interval = Duration::from_millis(unsafe { GetDoubleClickTime() } as u64);

        ClickState {
            button: Cell::new(MouseButton::Left),
            last_click: Cell::new(Instant::now()),
            last_position: Cell::new(Point::default()),
            double_click_spatial_tolerance_width: Cell::new(double_click_spatial_tolerance_width),
            double_click_spatial_tolerance_height: Cell::new(double_click_spatial_tolerance_height),
            double_click_interval: Cell::new(double_click_interval),
            current_count: Cell::new(0),
        }
    }

    /// update self and return the needed click count
    pub fn update(&self, button: MouseButton, new_position: Point<DevicePixels>) -> usize {
        if self.button.get() == button && self.is_double_click(new_position) {
            self.current_count.update(|it| it + 1);
        } else {
            self.current_count.set(1);
        }
        self.last_click.set(Instant::now());
        self.last_position.set(new_position);
        self.button.set(button);

        self.current_count.get()
    }

    pub fn system_update(&self, wparam: usize) {
        match wparam {
            // SPI_SETDOUBLECLKWIDTH
            29 => self
                .double_click_spatial_tolerance_width
                .set(unsafe { GetSystemMetrics(SM_CXDOUBLECLK) }),
            // SPI_SETDOUBLECLKHEIGHT
            30 => self
                .double_click_spatial_tolerance_height
                .set(unsafe { GetSystemMetrics(SM_CYDOUBLECLK) }),
            // SPI_SETDOUBLECLICKTIME
            32 => self
                .double_click_interval
                .set(Duration::from_millis(unsafe { GetDoubleClickTime() } as u64)),
            _ => {}
        }
    }

    #[inline]
    fn is_double_click(&self, new_position: Point<DevicePixels>) -> bool {
        let diff = self.last_position.get() - new_position;

        self.last_click.get().elapsed() < self.double_click_interval.get()
            && diff.x.0.abs() <= self.double_click_spatial_tolerance_width.get()
            && diff.y.0.abs() <= self.double_click_spatial_tolerance_height.get()
    }
}

#[derive(Copy, Clone)]
pub(super) struct StyleAndBounds {
    pub(super) style: WINDOW_STYLE,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) cx: i32,
    pub(super) cy: i32,
}

#[repr(C)]
pub(super) struct WINDOWCOMPOSITIONATTRIBDATA {
    pub(super) attrib: u32,
    pub(super) pv_data: *mut std::ffi::c_void,
    pub(super) cb_data: usize,
}

#[repr(C)]
pub(super) struct AccentPolicy {
    pub(super) accent_state: u32,
    pub(super) accent_flags: u32,
    pub(super) gradient_color: u32,
    pub(super) animation_id: u32,
}

pub(super) type Color = (u8, u8, u8, u8);

#[derive(Debug, Default, Clone)]
pub(crate) struct WindowBorderOffset {
    pub(crate) width_offset: Cell<i32>,
    pub(crate) height_offset: Cell<i32>,
}

impl WindowBorderOffset {
    pub(crate) fn update(&self, hwnd: HWND) -> anyhow::Result<()> {
        let window_rect = unsafe {
            let mut rect = std::mem::zeroed();
            GetWindowRect(hwnd, &mut rect)?;
            rect
        };
        let client_rect = unsafe {
            let mut rect = std::mem::zeroed();
            GetClientRect(hwnd, &mut rect)?;
            rect
        };
        self.width_offset
            .set((window_rect.right - window_rect.left) - (client_rect.right - client_rect.left));
        self.height_offset
            .set((window_rect.bottom - window_rect.top) - (client_rect.bottom - client_rect.top));
        Ok(())
    }
}

#[derive(Clone)]
pub(super) struct WindowOpenStatus {
    pub(super) placement: WINDOWPLACEMENT,
    pub(super) state: WindowOpenState,
}

#[derive(Clone, Copy)]
pub(super) enum WindowOpenState {
    Maximized,
    Fullscreen,
    Windowed,
}

pub(super) const WINDOW_CLASS_NAME: PCWSTR = w!("Raijin::Window");

pub(super) fn register_window_class(icon_handle: HICON) {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let wc = WNDCLASSW {
            lpfnWndProc: Some(window_procedure),
            hIcon: icon_handle,
            lpszClassName: PCWSTR(WINDOW_CLASS_NAME.as_ptr()),
            style: CS_HREDRAW | CS_VREDRAW,
            hInstance: get_module_handle().into(),
            hbrBackground: unsafe { CreateSolidBrush(COLORREF(0x00000000)) },
            ..Default::default()
        };
        unsafe { RegisterClassW(&wc) };
    });
}

unsafe extern "system" fn window_procedure(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        let window_params = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
        let window_creation_context = window_params.lpCreateParams as *mut WindowCreateContext;
        let window_creation_context = unsafe { &mut *window_creation_context };
        return match WindowsWindowInner::new(window_creation_context, hwnd, window_params) {
            Ok(window_state) => {
                let weak = Box::new(Rc::downgrade(&window_state));
                unsafe { set_window_long(hwnd, GWLP_USERDATA, Box::into_raw(weak) as isize) };
                window_creation_context.inner = Some(Ok(window_state));
                unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
            }
            Err(error) => {
                window_creation_context.inner = Some(Err(error));
                LRESULT(0)
            }
        };
    }

    let ptr = unsafe { get_window_long(hwnd, GWLP_USERDATA) } as *mut Weak<WindowsWindowInner>;
    if ptr.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let inner = unsafe { &*ptr };
    let result = if let Some(inner) = inner.upgrade() {
        inner.handle_msg(hwnd, msg, wparam, lparam)
    } else {
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    };

    if msg == WM_NCDESTROY {
        unsafe { set_window_long(hwnd, GWLP_USERDATA, 0) };
        unsafe { drop(Box::from_raw(ptr)) };
    }

    result
}

pub(crate) fn window_from_hwnd(hwnd: HWND) -> Option<Rc<WindowsWindowInner>> {
    if hwnd.is_invalid() {
        return None;
    }

    let ptr = unsafe { get_window_long(hwnd, GWLP_USERDATA) } as *mut Weak<WindowsWindowInner>;
    if !ptr.is_null() {
        let inner = unsafe { &*ptr };
        inner.upgrade()
    } else {
        None
    }
}

pub(super) fn get_module_handle() -> HMODULE {
    unsafe {
        let mut h_module = std::mem::zeroed();
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            windows::core::w!("RaijinModule"),
            &mut h_module,
        )
        .expect("Unable to get module handle"); // this should never fail

        h_module
    }
}

pub(super) fn register_drag_drop(window: &Rc<WindowsWindowInner>) -> Result<()> {
    let window_handle = window.hwnd;
    let handler = WindowsDragDropHandler(window.clone());
    // The lifetime of `IDropTarget` is handled by Windows, it won't release until
    // we call `RevokeDragDrop`.
    // So, it's safe to drop it here.
    let drag_drop_handler: IDropTarget = handler.into();
    unsafe {
        RegisterDragDrop(window_handle, &drag_drop_handler)
            .context("unable to register drag-drop event")?;
    }
    Ok(())
}

pub(super) fn calculate_window_rect(bounds: Bounds<DevicePixels>, border_offset: &WindowBorderOffset) -> RECT {
    // NOTE:
    // The reason we're not using `AdjustWindowRectEx()` here is
    // that the size reported by this function is incorrect.
    // You can test it, and there are similar discussions online.
    // See: https://stackoverflow.com/questions/12423584/how-to-set-exact-client-size-for-overlapped-window-winapi
    //
    // So we manually calculate these values here.
    let mut rect = RECT {
        left: bounds.left().0,
        top: bounds.top().0,
        right: bounds.right().0,
        bottom: bounds.bottom().0,
    };
    let left_offset = border_offset.width_offset.get() / 2;
    let top_offset = border_offset.height_offset.get() / 2;
    let right_offset = border_offset.width_offset.get() - left_offset;
    let bottom_offset = border_offset.height_offset.get() - top_offset;
    rect.left -= left_offset;
    rect.top -= top_offset;
    rect.right += right_offset;
    rect.bottom += bottom_offset;
    rect
}

pub(super) fn calculate_client_rect(
    rect: RECT,
    border_offset: &WindowBorderOffset,
    scale_factor: f32,
) -> Bounds<Pixels> {
    let left_offset = border_offset.width_offset.get() / 2;
    let top_offset = border_offset.height_offset.get() / 2;
    let right_offset = border_offset.width_offset.get() - left_offset;
    let bottom_offset = border_offset.height_offset.get() - top_offset;
    let left = rect.left + left_offset;
    let top = rect.top + top_offset;
    let right = rect.right - right_offset;
    let bottom = rect.bottom - bottom_offset;
    let physical_size = size(DevicePixels(right - left), DevicePixels(bottom - top));
    Bounds {
        origin: logical_point(left as f32, top as f32, scale_factor),
        size: physical_size.to_pixels(scale_factor),
    }
}

pub(super) fn retrieve_window_placement(
    hwnd: HWND,
    display: WindowsDisplay,
    initial_bounds: Bounds<Pixels>,
    scale_factor: f32,
    border_offset: &WindowBorderOffset,
) -> Result<WINDOWPLACEMENT> {
    let mut placement = WINDOWPLACEMENT {
        length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    unsafe { GetWindowPlacement(hwnd, &mut placement)? };
    // the bounds may be not inside the display
    let bounds = if display.check_given_bounds(initial_bounds) {
        initial_bounds
    } else {
        display.default_bounds()
    };
    let bounds = bounds.to_device_pixels(scale_factor);
    placement.rcNormalPosition = calculate_window_rect(bounds, border_offset);
    Ok(placement)
}

pub(super) fn dwm_set_window_composition_attribute(hwnd: HWND, backdrop_type: u32) {
    let mut version = unsafe { std::mem::zeroed() };
    let status = unsafe { windows::Wdk::System::SystemServices::RtlGetVersion(&mut version) };

    // DWMWA_SYSTEMBACKDROP_TYPE is available only on version 22621 or later
    // using SetWindowCompositionAttributeType as a fallback
    if !status.is_ok() || version.dwBuildNumber < 22621 {
        return;
    }

    unsafe {
        let result = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop_type as *const _ as *const _,
            std::mem::size_of_val(&backdrop_type) as u32,
        );

        if !result.is_ok() {
            return;
        }
    }
}

pub(super) fn set_window_composition_attribute(hwnd: HWND, color: Option<Color>, state: u32) {
    let mut version = unsafe { std::mem::zeroed() };
    let status = unsafe { windows::Wdk::System::SystemServices::RtlGetVersion(&mut version) };

    if !status.is_ok() || version.dwBuildNumber < 17763 {
        return;
    }

    unsafe {
        type SetWindowCompositionAttributeType =
            unsafe extern "system" fn(HWND, *mut WINDOWCOMPOSITIONATTRIBDATA) -> BOOL;
        let module_name = PCSTR::from_raw(c"user32.dll".as_ptr() as *const u8);
        if let Some(user32) = GetModuleHandleA(module_name)
            .context("Unable to get user32.dll handle")
            .log_err()
        {
            let func_name = PCSTR::from_raw(c"SetWindowCompositionAttribute".as_ptr() as *const u8);
            let set_window_composition_attribute: SetWindowCompositionAttributeType =
                std::mem::transmute(GetProcAddress(user32, func_name));
            let mut color = color.unwrap_or_default();
            let is_acrylic = state == 4;
            if is_acrylic && color.3 == 0 {
                color.3 = 1;
            }
            let accent = AccentPolicy {
                accent_state: state,
                accent_flags: if is_acrylic { 0 } else { 2 },
                gradient_color: (color.0 as u32)
                    | ((color.1 as u32) << 8)
                    | ((color.2 as u32) << 16)
                    | ((color.3 as u32) << 24),
                animation_id: 0,
            };
            let mut data = WINDOWCOMPOSITIONATTRIBDATA {
                attrib: 0x13,
                pv_data: &accent as *const _ as *mut _,
                cb_data: std::mem::size_of::<AccentPolicy>(),
            };
            let _ = set_window_composition_attribute(hwnd, &mut data as *mut _ as _);
        }
    }
}

// When the platform title bar is hidden, Windows may think that our application is meant to appear 'fullscreen'
// and will stop the taskbar from appearing on top of our window. Prevent this.
// https://devblogs.microsoft.com/oldnewthing/20250522-00/?p=111211
pub(super) fn set_non_rude_hwnd(hwnd: HWND, non_rude: bool) {
    if non_rude {
        unsafe { SetPropW(hwnd, w!("NonRudeHWND"), Some(HANDLE(1 as _))) }.log_err();
    } else {
        unsafe { RemovePropW(hwnd, w!("NonRudeHWND")) }.log_err();
    }
}

#[cfg(test)]
mod tests {
    use super::ClickState;
    use inazuma::{DevicePixels, MouseButton, point};
    use std::time::Duration;

    #[test]
    fn test_double_click_interval() {
        let state = ClickState::new();
        assert_eq!(
            state.update(MouseButton::Left, point(DevicePixels(0), DevicePixels(0))),
            1
        );
        assert_eq!(
            state.update(MouseButton::Right, point(DevicePixels(0), DevicePixels(0))),
            1
        );
        assert_eq!(
            state.update(MouseButton::Left, point(DevicePixels(0), DevicePixels(0))),
            1
        );
        assert_eq!(
            state.update(MouseButton::Left, point(DevicePixels(0), DevicePixels(0))),
            2
        );
        state
            .last_click
            .update(|it| it - Duration::from_millis(700));
        assert_eq!(
            state.update(MouseButton::Left, point(DevicePixels(0), DevicePixels(0))),
            1
        );
    }

    #[test]
    fn test_double_click_spatial_tolerance() {
        let state = ClickState::new();
        assert_eq!(
            state.update(MouseButton::Left, point(DevicePixels(-3), DevicePixels(0))),
            1
        );
        assert_eq!(
            state.update(MouseButton::Left, point(DevicePixels(0), DevicePixels(3))),
            2
        );
        assert_eq!(
            state.update(MouseButton::Right, point(DevicePixels(3), DevicePixels(2))),
            1
        );
        assert_eq!(
            state.update(MouseButton::Right, point(DevicePixels(10), DevicePixels(0))),
            1
        );
    }
}
