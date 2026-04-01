use super::*;

pub(super) struct ImeContext {
    pub(super) hwnd: HWND,
    pub(super) himc: HIMC,
}

impl ImeContext {
    fn get(hwnd: HWND) -> Option<Self> {
        let himc = unsafe { ImmGetContext(hwnd) };
        if himc.is_invalid() {
            return None;
        }
        Some(Self { hwnd, himc })
    }
}

impl std::ops::Deref for ImeContext {
    type Target = HIMC;
    fn deref(&self) -> &HIMC {
        &self.himc
    }
}

impl Drop for ImeContext {
    fn drop(&mut self) {
        unsafe {
            ImmReleaseContext(self.hwnd, self.himc).ok().log_err();
        }
    }
}

pub(super) fn handle_key_event<F>(
    wparam: WPARAM,
    lparam: LPARAM,
    state: &WindowsWindowState,
    f: F,
) -> Option<PlatformInput>
where
    F: FnOnce(Keystroke, bool) -> PlatformInput,
{
    let virtual_key = VIRTUAL_KEY(wparam.loword());
    let modifiers = current_modifiers();

    match virtual_key {
        VK_SHIFT | VK_CONTROL | VK_MENU | VK_LMENU | VK_RMENU | VK_LWIN | VK_RWIN => {
            if state
                .last_reported_modifiers
                .get()
                .is_some_and(|prev_modifiers| prev_modifiers == modifiers)
            {
                return None;
            }
            state.last_reported_modifiers.set(Some(modifiers));
            Some(PlatformInput::ModifiersChanged(ModifiersChangedEvent {
                modifiers,
                capslock: current_capslock(),
            }))
        }
        VK_PACKET => None,
        VK_CAPITAL => {
            let capslock = current_capslock();
            if state
                .last_reported_capslock
                .get()
                .is_some_and(|prev_capslock| prev_capslock == capslock)
            {
                return None;
            }
            state.last_reported_capslock.set(Some(capslock));
            Some(PlatformInput::ModifiersChanged(ModifiersChangedEvent {
                modifiers,
                capslock,
            }))
        }
        vkey => {
            let keystroke = parse_normal_key(vkey, lparam, modifiers)?;
            Some(f(keystroke.0, keystroke.1))
        }
    }
}

pub(super) fn parse_immutable(vkey: VIRTUAL_KEY) -> Option<String> {
    Some(
        match vkey {
            VK_SPACE => "space",
            VK_BACK => "backspace",
            VK_RETURN => "enter",
            VK_TAB => "tab",
            VK_UP => "up",
            VK_DOWN => "down",
            VK_RIGHT => "right",
            VK_LEFT => "left",
            VK_HOME => "home",
            VK_END => "end",
            VK_PRIOR => "pageup",
            VK_NEXT => "pagedown",
            VK_BROWSER_BACK => "back",
            VK_BROWSER_FORWARD => "forward",
            VK_ESCAPE => "escape",
            VK_INSERT => "insert",
            VK_DELETE => "delete",
            VK_APPS => "menu",
            VK_F1 => "f1",
            VK_F2 => "f2",
            VK_F3 => "f3",
            VK_F4 => "f4",
            VK_F5 => "f5",
            VK_F6 => "f6",
            VK_F7 => "f7",
            VK_F8 => "f8",
            VK_F9 => "f9",
            VK_F10 => "f10",
            VK_F11 => "f11",
            VK_F12 => "f12",
            VK_F13 => "f13",
            VK_F14 => "f14",
            VK_F15 => "f15",
            VK_F16 => "f16",
            VK_F17 => "f17",
            VK_F18 => "f18",
            VK_F19 => "f19",
            VK_F20 => "f20",
            VK_F21 => "f21",
            VK_F22 => "f22",
            VK_F23 => "f23",
            VK_F24 => "f24",
            _ => return None,
        }
        .to_string(),
    )
}

pub(super) fn parse_normal_key(
    vkey: VIRTUAL_KEY,
    lparam: LPARAM,
    mut modifiers: Modifiers,
) -> Option<(Keystroke, bool)> {
    let (key_char, prefer_character_input) = process_key(vkey, lparam.hiword());

    let key = parse_immutable(vkey).or_else(|| {
        let scan_code = lparam.hiword() & 0xFF;
        get_keystroke_key(vkey, scan_code as u32, &mut modifiers)
    })?;

    Some((
        Keystroke {
            modifiers,
            key,
            key_char,
        },
        prefer_character_input,
    ))
}

pub(super) fn process_key(vkey: VIRTUAL_KEY, scan_code: u16) -> (Option<String>, bool) {
    let mut keyboard_state = [0u8; 256];
    unsafe {
        if GetKeyboardState(&mut keyboard_state).is_err() {
            return (None, false);
        }
    }

    let mut buffer_c = [0u16; 8];
    let result_c = unsafe {
        ToUnicode(
            vkey.0 as u32,
            scan_code as u32,
            Some(&keyboard_state),
            &mut buffer_c,
            0x4,
        )
    };

    if result_c == 0 {
        return (None, false);
    }

    let c = &buffer_c[..result_c.unsigned_abs() as usize];
    let key_char = String::from_utf16(c)
        .ok()
        .filter(|s| !s.is_empty() && !s.chars().next().unwrap().is_control());

    if result_c < 0 {
        return (key_char, true);
    }

    if key_char.is_none() {
        return (None, false);
    }

    // Workaround for some bug that makes the compiler think keyboard_state is still zeroed out
    let keyboard_state = std::hint::black_box(keyboard_state);
    let ctrl_down = (keyboard_state[VK_CONTROL.0 as usize] & 0x80) != 0;
    let alt_down = (keyboard_state[VK_MENU.0 as usize] & 0x80) != 0;
    let win_down = (keyboard_state[VK_LWIN.0 as usize] & 0x80) != 0
        || (keyboard_state[VK_RWIN.0 as usize] & 0x80) != 0;

    let has_modifiers = ctrl_down || alt_down || win_down;
    if !has_modifiers {
        return (key_char, false);
    }

    let mut state_no_modifiers = keyboard_state;
    state_no_modifiers[VK_CONTROL.0 as usize] = 0;
    state_no_modifiers[VK_LCONTROL.0 as usize] = 0;
    state_no_modifiers[VK_RCONTROL.0 as usize] = 0;
    state_no_modifiers[VK_MENU.0 as usize] = 0;
    state_no_modifiers[VK_LMENU.0 as usize] = 0;
    state_no_modifiers[VK_RMENU.0 as usize] = 0;
    state_no_modifiers[VK_LWIN.0 as usize] = 0;
    state_no_modifiers[VK_RWIN.0 as usize] = 0;

    let mut buffer_c_no_modifiers = [0u16; 8];
    let result_c_no_modifiers = unsafe {
        ToUnicode(
            vkey.0 as u32,
            scan_code as u32,
            Some(&state_no_modifiers),
            &mut buffer_c_no_modifiers,
            0x4,
        )
    };

    let c_no_modifiers = &buffer_c_no_modifiers[..result_c_no_modifiers.unsigned_abs() as usize];
    (
        key_char,
        result_c != result_c_no_modifiers || c != c_no_modifiers,
    )
}

pub(super) fn parse_ime_composition_string(ctx: HIMC, comp_type: IME_COMPOSITION_STRING) -> Option<Vec<u16>> {
    unsafe {
        let string_len = ImmGetCompositionStringW(ctx, comp_type, None, 0);
        if string_len >= 0 {
            let mut buffer = vec![0u8; string_len as usize + 2];
            ImmGetCompositionStringW(
                ctx,
                comp_type,
                Some(buffer.as_mut_ptr() as _),
                string_len as _,
            );
            let wstring = std::slice::from_raw_parts::<u16>(
                buffer.as_mut_ptr().cast::<u16>(),
                string_len as usize / 2,
            );
            Some(wstring.to_vec())
        } else {
            None
        }
    }
}

#[inline]
pub(super) fn retrieve_composition_cursor_position(ctx: HIMC) -> usize {
    unsafe { ImmGetCompositionStringW(ctx, GCS_CURSORPOS, None, 0) as usize }
}

pub(super) fn should_use_ime_cursor_position(ctx: HIMC, cursor_pos: usize) -> bool {
    let attrs_size = unsafe { ImmGetCompositionStringW(ctx, GCS_COMPATTR, None, 0) } as usize;
    if attrs_size == 0 {
        return false;
    }

    let mut attrs = vec![0u8; attrs_size];
    let result = unsafe {
        ImmGetCompositionStringW(
            ctx,
            GCS_COMPATTR,
            Some(attrs.as_mut_ptr() as *mut _),
            attrs_size as u32,
        )
    };
    if result <= 0 {
        return false;
    }

    // Keep the cursor adjacent to the inserted text by only using the suggested position
    // if it's adjacent to unconverted text.
    let at_cursor_is_input = cursor_pos < attrs.len() && attrs[cursor_pos] == (ATTR_INPUT as u8);
    let before_cursor_is_input = cursor_pos > 0
        && (cursor_pos - 1) < attrs.len()
        && attrs[cursor_pos - 1] == (ATTR_INPUT as u8);

    at_cursor_is_input || before_cursor_is_input
}

#[inline]
pub(super) fn is_virtual_key_pressed(vkey: VIRTUAL_KEY) -> bool {
    unsafe { GetKeyState(vkey.0 as i32) < 0 }
}

#[inline]
pub(crate) fn current_modifiers() -> Modifiers {
    Modifiers {
        control: is_virtual_key_pressed(VK_CONTROL),
        alt: is_virtual_key_pressed(VK_MENU),
        shift: is_virtual_key_pressed(VK_SHIFT),
        platform: is_virtual_key_pressed(VK_LWIN) || is_virtual_key_pressed(VK_RWIN),
        function: false,
    }
}

#[inline]
pub(crate) fn current_capslock() -> Capslock {
    let on = unsafe { GetKeyState(VK_CAPITAL.0 as i32) & 1 } > 0;
    Capslock { on }
}

// there is some additional non-visible space when talking about window
// borders on Windows:
// - SM_CXSIZEFRAME: The resize handle.
// - SM_CXPADDEDBORDER: Additional border space that isn't part of the resize handle.
pub(super) fn get_frame_thicknessx(dpi: u32) -> i32 {
    let resize_frame_thickness = unsafe { GetSystemMetricsForDpi(SM_CXSIZEFRAME, dpi) };
    let padding_thickness = unsafe { GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi) };
    resize_frame_thickness + padding_thickness
}

pub(super) fn get_frame_thicknessy(dpi: u32) -> i32 {
    let resize_frame_thickness = unsafe { GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi) };
    let padding_thickness = unsafe { GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi) };
    resize_frame_thickness + padding_thickness
}

pub(super) fn notify_frame_changed(handle: HWND) {
    unsafe {
        SetWindowPos(
            handle,
            None,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED
                | SWP_NOACTIVATE
                | SWP_NOCOPYBITS
                | SWP_NOMOVE
                | SWP_NOOWNERZORDER
                | SWP_NOREPOSITION
                | SWP_NOSENDCHANGING
                | SWP_NOSIZE
                | SWP_NOZORDER,
        )
        .log_err();
    }
}
