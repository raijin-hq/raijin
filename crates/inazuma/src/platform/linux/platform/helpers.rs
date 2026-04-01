use super::*;

pub(super) fn open_uri_internal(
    executor: BackgroundExecutor,
    uri: &str,
    activation_token: Option<String>,
) {
    if let Some(uri) = ashpd::Uri::parse(uri).log_err() {
        executor
            .spawn(async move {
                let mut xdg_open_failed = false;
                for mut command in open::commands(uri.to_string()) {
                    if let Some(token) = activation_token.as_ref() {
                        command.env("XDG_ACTIVATION_TOKEN", token);
                    }
                    let program = format!("{:?}", command.get_program());
                    match smol::process::Command::from(command).spawn() {
                        Ok(mut cmd) => match cmd.status().await {
                            Ok(status) if status.success() => return,
                            Ok(status) => {
                                log::error!("Command {} exited with status: {}", program, status);
                                xdg_open_failed = true;
                            }
                            Err(e) => {
                                log::error!("Failed to get status from {}: {}", program, e);
                                xdg_open_failed = true;
                            }
                        },
                        Err(e) => {
                            log::error!("Failed to open with {}: {}", program, e);
                            xdg_open_failed = true;
                        }
                    }
                }

                if xdg_open_failed {
                    match ashpd::desktop::open_uri::OpenFileRequest::default()
                        .activation_token(activation_token.map(ashpd::ActivationToken::from))
                        .send_uri(&uri)
                        .await
                        .and_then(|e| e.response())
                    {
                        Ok(()) => {}
                        Err(ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled)) => {}
                        Err(e) => {
                            log::error!("Failed to open with dbus: {}", e);
                        }
                    }
                }
            })
            .detach();
    }
}

#[cfg(any(feature = "x11", feature = "wayland"))]
pub(super) fn reveal_path_internal(
    executor: BackgroundExecutor,
    path: PathBuf,
    activation_token: Option<String>,
) {
    executor
        .spawn(async move {
            if let Some(dir) = File::open(path.clone()).log_err() {
                match ashpd::desktop::open_uri::OpenDirectoryRequest::default()
                    .activation_token(activation_token.map(ashpd::ActivationToken::from))
                    .send(&dir.as_fd())
                    .await
                {
                    Ok(_) => return,
                    Err(e) => log::error!("Failed to open with dbus: {}", e),
                }
                if path.is_dir() {
                    open::that_detached(path).log_err();
                } else {
                    open::that_detached(path.parent().unwrap_or(Path::new(""))).log_err();
                }
            }
        })
        .detach();
}

#[cfg(any(feature = "wayland", feature = "x11"))]
pub(super) fn is_within_click_distance(a: Point<Pixels>, b: Point<Pixels>) -> bool {
    let diff = a - b;
    diff.x.abs() <= DOUBLE_CLICK_DISTANCE && diff.y.abs() <= DOUBLE_CLICK_DISTANCE
}

#[cfg(any(feature = "wayland", feature = "x11"))]
pub(super) fn get_xkb_compose_state(cx: &xkb::Context) -> Option<xkb::compose::State> {
    let mut locales = Vec::default();
    if let Some(locale) = env::var_os("LC_CTYPE") {
        locales.push(locale);
    }
    locales.push(OsString::from("C"));
    let mut state: Option<xkb::compose::State> = None;
    for locale in locales {
        if let Ok(table) =
            xkb::compose::Table::new_from_locale(cx, &locale, xkb::compose::COMPILE_NO_FLAGS)
        {
            state = Some(xkb::compose::State::new(
                &table,
                xkb::compose::STATE_NO_FLAGS,
            ));
            break;
        }
    }
    state
}

#[cfg(any(feature = "wayland", feature = "x11"))]
pub(super) unsafe fn read_fd(fd: filedescriptor::FileDescriptor) -> Result<Vec<u8>> {
    let mut file = unsafe { File::from_raw_fd(fd.into_raw_fd()) };
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

#[cfg(any(feature = "wayland", feature = "x11"))]
pub(super) const DEFAULT_CURSOR_ICON_NAME: &str = "left_ptr";

#[cfg(any(feature = "wayland", feature = "x11"))]
pub(super) fn cursor_style_to_icon_names(style: CursorStyle) -> &'static [&'static str] {
    // Based on cursor names from chromium:
    // https://github.com/chromium/chromium/blob/d3069cf9c973dc3627fa75f64085c6a86c8f41bf/ui/base/cursor/cursor_factory.cc#L113
    match style {
        CursorStyle::Arrow => &[DEFAULT_CURSOR_ICON_NAME],
        CursorStyle::IBeam => &["text", "xterm"],
        CursorStyle::Crosshair => &["crosshair", "cross"],
        CursorStyle::ClosedHand => &["closedhand", "grabbing", "hand2"],
        CursorStyle::OpenHand => &["openhand", "grab", "hand1"],
        CursorStyle::PointingHand => &["pointer", "hand", "hand2"],
        CursorStyle::ResizeLeft => &["w-resize", "left_side"],
        CursorStyle::ResizeRight => &["e-resize", "right_side"],
        CursorStyle::ResizeLeftRight => &["ew-resize", "sb_h_double_arrow"],
        CursorStyle::ResizeUp => &["n-resize", "top_side"],
        CursorStyle::ResizeDown => &["s-resize", "bottom_side"],
        CursorStyle::ResizeUpDown => &["sb_v_double_arrow", "ns-resize"],
        CursorStyle::ResizeUpLeftDownRight => &["size_fdiag", "bd_double_arrow", "nwse-resize"],
        CursorStyle::ResizeUpRightDownLeft => &["size_bdiag", "nesw-resize", "fd_double_arrow"],
        CursorStyle::ResizeColumn => &["col-resize", "sb_h_double_arrow"],
        CursorStyle::ResizeRow => &["row-resize", "sb_v_double_arrow"],
        CursorStyle::IBeamCursorForVerticalLayout => &["vertical-text"],
        CursorStyle::OperationNotAllowed => &["not-allowed", "crossed_circle"],
        CursorStyle::DragLink => &["alias"],
        CursorStyle::DragCopy => &["copy"],
        CursorStyle::ContextualMenu => &["context-menu"],
        CursorStyle::None => {
            #[cfg(debug_assertions)]
            panic!("CursorStyle::None should be handled separately in the client");
            #[cfg(not(debug_assertions))]
            &[DEFAULT_CURSOR_ICON_NAME]
        }
    }
}

#[cfg(any(feature = "wayland", feature = "x11"))]
pub(super) fn log_cursor_icon_warning(message: impl std::fmt::Display) {
    if let Ok(xcursor_path) = env::var("XCURSOR_PATH") {
        log::warn!(
            "{:#}\ncursor icon loading may be failing if XCURSOR_PATH environment variable is invalid. \
                    XCURSOR_PATH overrides the default icon search. Its current value is '{}'",
            message,
            xcursor_path
        );
    } else {
        log::warn!("{:#}", message);
    }
}

#[cfg(any(feature = "wayland", feature = "x11"))]
pub(super) fn guess_ascii(keycode: Keycode, shift: bool) -> Option<char> {
    let c = match (keycode.raw(), shift) {
        (24, _) => 'q',
        (25, _) => 'w',
        (26, _) => 'e',
        (27, _) => 'r',
        (28, _) => 't',
        (29, _) => 'y',
        (30, _) => 'u',
        (31, _) => 'i',
        (32, _) => 'o',
        (33, _) => 'p',
        (34, false) => '[',
        (34, true) => '{',
        (35, false) => ']',
        (35, true) => '}',
        (38, _) => 'a',
        (39, _) => 's',
        (40, _) => 'd',
        (41, _) => 'f',
        (42, _) => 'g',
        (43, _) => 'h',
        (44, _) => 'j',
        (45, _) => 'k',
        (46, _) => 'l',
        (47, false) => ';',
        (47, true) => ':',
        (48, false) => '\'',
        (48, true) => '"',
        (49, false) => '`',
        (49, true) => '~',
        (51, false) => '\\',
        (51, true) => '|',
        (52, _) => 'z',
        (53, _) => 'x',
        (54, _) => 'c',
        (55, _) => 'v',
        (56, _) => 'b',
        (57, _) => 'n',
        (58, _) => 'm',
        (59, false) => ',',
        (59, true) => '>',
        (60, false) => '.',
        (60, true) => '<',
        (61, false) => '/',
        (61, true) => '?',

        _ => return None,
    };

    Some(c)
}

#[cfg(any(feature = "wayland", feature = "x11"))]
pub(super) fn keystroke_from_xkb(
    state: &State,
    mut modifiers: inazuma::Modifiers,
    keycode: Keycode,
) -> inazuma::Keystroke {
    let key_utf32 = state.key_get_utf32(keycode);
    let key_utf8 = state.key_get_utf8(keycode);
    let key_sym = state.key_get_one_sym(keycode);

    let key = match key_sym {
        Keysym::Return => "enter".to_owned(),
        Keysym::Prior => "pageup".to_owned(),
        Keysym::Next => "pagedown".to_owned(),
        Keysym::ISO_Left_Tab => "tab".to_owned(),
        Keysym::KP_Prior => "pageup".to_owned(),
        Keysym::KP_Next => "pagedown".to_owned(),
        Keysym::XF86_Back => "back".to_owned(),
        Keysym::XF86_Forward => "forward".to_owned(),
        Keysym::XF86_Cut => "cut".to_owned(),
        Keysym::XF86_Copy => "copy".to_owned(),
        Keysym::XF86_Paste => "paste".to_owned(),
        Keysym::XF86_New => "new".to_owned(),
        Keysym::XF86_Open => "open".to_owned(),
        Keysym::XF86_Save => "save".to_owned(),

        Keysym::comma => ",".to_owned(),
        Keysym::period => ".".to_owned(),
        Keysym::less => "<".to_owned(),
        Keysym::greater => ">".to_owned(),
        Keysym::slash => "/".to_owned(),
        Keysym::question => "?".to_owned(),

        Keysym::semicolon => ";".to_owned(),
        Keysym::colon => ":".to_owned(),
        Keysym::apostrophe => "'".to_owned(),
        Keysym::quotedbl => "\"".to_owned(),

        Keysym::bracketleft => "[".to_owned(),
        Keysym::braceleft => "{".to_owned(),
        Keysym::bracketright => "]".to_owned(),
        Keysym::braceright => "}".to_owned(),
        Keysym::backslash => "\\".to_owned(),
        Keysym::bar => "|".to_owned(),

        Keysym::grave => "`".to_owned(),
        Keysym::asciitilde => "~".to_owned(),
        Keysym::exclam => "!".to_owned(),
        Keysym::at => "@".to_owned(),
        Keysym::numbersign => "#".to_owned(),
        Keysym::dollar => "$".to_owned(),
        Keysym::percent => "%".to_owned(),
        Keysym::asciicircum => "^".to_owned(),
        Keysym::ampersand => "&".to_owned(),
        Keysym::asterisk => "*".to_owned(),
        Keysym::parenleft => "(".to_owned(),
        Keysym::parenright => ")".to_owned(),
        Keysym::minus => "-".to_owned(),
        Keysym::underscore => "_".to_owned(),
        Keysym::equal => "=".to_owned(),
        Keysym::plus => "+".to_owned(),
        Keysym::space => "space".to_owned(),
        Keysym::BackSpace => "backspace".to_owned(),
        Keysym::Tab => "tab".to_owned(),
        Keysym::Delete => "delete".to_owned(),
        Keysym::Escape => "escape".to_owned(),

        Keysym::Left => "left".to_owned(),
        Keysym::Right => "right".to_owned(),
        Keysym::Up => "up".to_owned(),
        Keysym::Down => "down".to_owned(),
        Keysym::Home => "home".to_owned(),
        Keysym::End => "end".to_owned(),
        Keysym::Insert => "insert".to_owned(),

        _ => {
            let name = xkb::keysym_get_name(key_sym).to_lowercase();
            if key_sym.is_keypad_key() {
                name.replace("kp_", "")
            } else if let Some(key) = key_utf8.chars().next()
                && key_utf8.len() == 1
                && key.is_ascii()
            {
                if key.is_ascii_graphic() {
                    key_utf8.to_lowercase()
                // map ctrl-a to `a`
                // ctrl-0..9 may emit control codes like ctrl-[, but
                // we don't want to map them to `[`
                } else if key_utf32 <= 0x1f
                    && !name.chars().next().is_some_and(|c| c.is_ascii_digit())
                {
                    ((key_utf32 as u8 + 0x40) as char)
                        .to_ascii_lowercase()
                        .to_string()
                } else {
                    name
                }
            } else if let Some(key_en) = guess_ascii(keycode, modifiers.shift) {
                String::from(key_en)
            } else {
                name
            }
        }
    };

    if modifiers.shift {
        // we only include the shift for upper-case letters by convention,
        // so don't include for numbers and symbols, but do include for
        // tab/enter, etc.
        if key.chars().count() == 1 && key.to_lowercase() == key.to_uppercase() {
            modifiers.shift = false;
        }
    }

    // Ignore control characters (and DEL) for the purposes of key_char
    let key_char =
        (key_utf32 >= 32 && key_utf32 != 127 && !key_utf8.is_empty()).then_some(key_utf8);

    inazuma::Keystroke {
        modifiers,
        key,
        key_char,
    }
}

/**
 * Returns which symbol the dead key represents
 * <https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_key_values#dead_keycodes_for_linux>
 */
#[cfg(any(feature = "wayland", feature = "x11"))]
pub fn keystroke_underlying_dead_key(keysym: Keysym) -> Option<String> {
    match keysym {
        Keysym::dead_grave => Some("`".to_owned()),
        Keysym::dead_acute => Some("´".to_owned()),
        Keysym::dead_circumflex => Some("^".to_owned()),
        Keysym::dead_tilde => Some("~".to_owned()),
        Keysym::dead_macron => Some("¯".to_owned()),
        Keysym::dead_breve => Some("˘".to_owned()),
        Keysym::dead_abovedot => Some("˙".to_owned()),
        Keysym::dead_diaeresis => Some("¨".to_owned()),
        Keysym::dead_abovering => Some("˚".to_owned()),
        Keysym::dead_doubleacute => Some("˝".to_owned()),
        Keysym::dead_caron => Some("ˇ".to_owned()),
        Keysym::dead_cedilla => Some("¸".to_owned()),
        Keysym::dead_ogonek => Some("˛".to_owned()),
        Keysym::dead_iota => Some("ͅ".to_owned()),
        Keysym::dead_voiced_sound => Some("゙".to_owned()),
        Keysym::dead_semivoiced_sound => Some("゚".to_owned()),
        Keysym::dead_belowdot => Some("̣̣".to_owned()),
        Keysym::dead_hook => Some("̡".to_owned()),
        Keysym::dead_horn => Some("̛".to_owned()),
        Keysym::dead_stroke => Some("̶̶".to_owned()),
        Keysym::dead_abovecomma => Some("̓̓".to_owned()),
        Keysym::dead_abovereversedcomma => Some("ʽ".to_owned()),
        Keysym::dead_doublegrave => Some("̏".to_owned()),
        Keysym::dead_belowring => Some("˳".to_owned()),
        Keysym::dead_belowmacron => Some("̱".to_owned()),
        Keysym::dead_belowcircumflex => Some("ꞈ".to_owned()),
        Keysym::dead_belowtilde => Some("̰".to_owned()),
        Keysym::dead_belowbreve => Some("̮".to_owned()),
        Keysym::dead_belowdiaeresis => Some("̤".to_owned()),
        Keysym::dead_invertedbreve => Some("̯".to_owned()),
        Keysym::dead_belowcomma => Some("̦".to_owned()),
        Keysym::dead_currency => None,
        Keysym::dead_lowline => None,
        Keysym::dead_aboveverticalline => None,
        Keysym::dead_belowverticalline => None,
        Keysym::dead_longsolidusoverlay => None,
        Keysym::dead_a => None,
        Keysym::dead_A => None,
        Keysym::dead_e => None,
        Keysym::dead_E => None,
        Keysym::dead_i => None,
        Keysym::dead_I => None,
        Keysym::dead_o => None,
        Keysym::dead_O => None,
        Keysym::dead_u => None,
        Keysym::dead_U => None,
        Keysym::dead_small_schwa => Some("ə".to_owned()),
        Keysym::dead_capital_schwa => Some("Ə".to_owned()),
        Keysym::dead_greek => None,
        _ => None,
    }
}
#[cfg(any(feature = "wayland", feature = "x11"))]
pub(super) fn modifiers_from_xkb(keymap_state: &State) -> inazuma::Modifiers {
    let shift = keymap_state.mod_name_is_active(xkb::MOD_NAME_SHIFT, xkb::STATE_MODS_EFFECTIVE);
    let alt = keymap_state.mod_name_is_active(xkb::MOD_NAME_ALT, xkb::STATE_MODS_EFFECTIVE);
    let control = keymap_state.mod_name_is_active(xkb::MOD_NAME_CTRL, xkb::STATE_MODS_EFFECTIVE);
    let platform = keymap_state.mod_name_is_active(xkb::MOD_NAME_LOGO, xkb::STATE_MODS_EFFECTIVE);
    inazuma::Modifiers {
        shift,
        alt,
        control,
        platform,
        function: false,
    }
}

#[cfg(any(feature = "wayland", feature = "x11"))]
pub(super) fn capslock_from_xkb(keymap_state: &State) -> inazuma::Capslock {
    let on = keymap_state.mod_name_is_active(xkb::MOD_NAME_CAPS, xkb::STATE_MODS_EFFECTIVE);
    inazuma::Capslock { on }
}

/// Resolve a Linux `dev_t` to PCI vendor/device IDs via sysfs, returning a
/// [`CompositorGpuHint`] that the GPU adapter selection code can use to
/// prioritize the compositor's rendering device.
#[cfg(any(feature = "wayland", feature = "x11"))]
pub(super) fn compositor_gpu_hint_from_dev_t(
    dev: u64,
) -> Option<crate::platform::wgpu::CompositorGpuHint> {
    fn dev_major(dev: u64) -> u32 {
        ((dev >> 8) & 0xfff) as u32 | (((dev >> 32) & !0xfff) as u32)
    }

    fn dev_minor(dev: u64) -> u32 {
        (dev & 0xff) as u32 | (((dev >> 12) & !0xff) as u32)
    }

    fn read_sysfs_hex_id(path: &str) -> Option<u32> {
        let content = std::fs::read_to_string(path).ok()?;
        let trimmed = content.trim().strip_prefix("0x").unwrap_or(content.trim());
        u32::from_str_radix(trimmed, 16).ok()
    }

    let major = dev_major(dev);
    let minor = dev_minor(dev);

    let vendor_path = format!("/sys/dev/char/{major}:{minor}/device/vendor");
    let device_path = format!("/sys/dev/char/{major}:{minor}/device/device");

    let vendor_id = read_sysfs_hex_id(&vendor_path)?;
    let device_id = read_sysfs_hex_id(&device_path)?;

    log::info!(
        "Compositor GPU hint: vendor={:#06x}, device={:#06x} (from dev {major}:{minor})",
        vendor_id,
        device_id,
    );

    Some(crate::platform::wgpu::CompositorGpuHint {
        vendor_id,
        device_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use inazuma::{Point, px};

    #[test]
    fn test_is_within_click_distance() {
        let zero = Point::new(px(0.0), px(0.0));
        assert!(is_within_click_distance(zero, Point::new(px(5.0), px(5.0))));
        assert!(is_within_click_distance(
            zero,
            Point::new(px(-4.9), px(5.0))
        ));
        assert!(is_within_click_distance(
            Point::new(px(3.0), px(2.0)),
            Point::new(px(-2.0), px(-2.0))
        ));
        assert!(!is_within_click_distance(
            zero,
            Point::new(px(5.0), px(5.1))
        ),);
    }
}
