use inazuma::{
    Capslock, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, ModifiersChangedEvent, MouseButton,
    MouseDownEvent, MouseExitEvent, MouseMoveEvent, MousePressureEvent, MouseUpEvent,
    NavigationDirection, PinchEvent, Pixels, PlatformInput, PressureStage, ScrollDelta,
    ScrollWheelEvent, TouchPhase, point, px,
};

use super::{
    LMGetKbdType, TISCopyCurrentKeyboardLayoutInputSource, TISGetInputSourceProperty,
    UCKeyTranslate, kTISPropertyUnicodeKeyLayoutData,
};
use objc2_core_foundation::CFData;
use objc2_core_graphics::CGKeyCode;
use objc2_app_kit::{
    NSEvent, NSEventModifierFlags, NSEventPhase, NSEventType,
    NSUpArrowFunctionKey, NSDownArrowFunctionKey, NSLeftArrowFunctionKey, NSRightArrowFunctionKey,
    NSPageUpFunctionKey, NSPageDownFunctionKey, NSHomeFunctionKey, NSEndFunctionKey,
    NSDeleteFunctionKey, NSHelpFunctionKey, NSModeSwitchFunctionKey,
    NSF1FunctionKey, NSF2FunctionKey, NSF3FunctionKey, NSF4FunctionKey,
    NSF5FunctionKey, NSF6FunctionKey, NSF7FunctionKey, NSF8FunctionKey,
    NSF9FunctionKey, NSF10FunctionKey, NSF11FunctionKey, NSF12FunctionKey,
    NSF13FunctionKey, NSF14FunctionKey, NSF15FunctionKey, NSF16FunctionKey,
    NSF17FunctionKey, NSF18FunctionKey, NSF19FunctionKey, NSF20FunctionKey,
    NSF21FunctionKey, NSF22FunctionKey, NSF23FunctionKey, NSF24FunctionKey,
    NSF25FunctionKey, NSF26FunctionKey, NSF27FunctionKey, NSF28FunctionKey,
    NSF29FunctionKey, NSF30FunctionKey, NSF31FunctionKey, NSF32FunctionKey,
    NSF33FunctionKey, NSF34FunctionKey, NSF35FunctionKey,
};
use std::{borrow::Cow, ffi::c_void};

const BACKSPACE_KEY: u16 = 0x7f;
const SPACE_KEY: u16 = b' ' as u16;
const ENTER_KEY: u16 = 0x0d;
const NUMPAD_ENTER_KEY: u16 = 0x03;
pub(crate) const ESCAPE_KEY: u16 = 0x1b;
const TAB_KEY: u16 = 0x09;
const SHIFT_TAB_KEY: u16 = 0x19;

pub fn key_to_native(key: &str) -> Cow<'_, str> {
    let code: u16 = match key {
        "space" => SPACE_KEY,
        "backspace" => BACKSPACE_KEY,
        "escape" => ESCAPE_KEY,
        "up" => NSUpArrowFunctionKey as u16,
        "down" => NSDownArrowFunctionKey as u16,
        "left" => NSLeftArrowFunctionKey as u16,
        "right" => NSRightArrowFunctionKey as u16,
        "pageup" => NSPageUpFunctionKey as u16,
        "pagedown" => NSPageDownFunctionKey as u16,
        "home" => NSHomeFunctionKey as u16,
        "end" => NSEndFunctionKey as u16,
        "delete" => NSDeleteFunctionKey as u16,
        "insert" => NSHelpFunctionKey as u16,
        "f1" => NSF1FunctionKey as u16,
        "f2" => NSF2FunctionKey as u16,
        "f3" => NSF3FunctionKey as u16,
        "f4" => NSF4FunctionKey as u16,
        "f5" => NSF5FunctionKey as u16,
        "f6" => NSF6FunctionKey as u16,
        "f7" => NSF7FunctionKey as u16,
        "f8" => NSF8FunctionKey as u16,
        "f9" => NSF9FunctionKey as u16,
        "f10" => NSF10FunctionKey as u16,
        "f11" => NSF11FunctionKey as u16,
        "f12" => NSF12FunctionKey as u16,
        "f13" => NSF13FunctionKey as u16,
        "f14" => NSF14FunctionKey as u16,
        "f15" => NSF15FunctionKey as u16,
        "f16" => NSF16FunctionKey as u16,
        "f17" => NSF17FunctionKey as u16,
        "f18" => NSF18FunctionKey as u16,
        "f19" => NSF19FunctionKey as u16,
        "f20" => NSF20FunctionKey as u16,
        "f21" => NSF21FunctionKey as u16,
        "f22" => NSF22FunctionKey as u16,
        "f23" => NSF23FunctionKey as u16,
        "f24" => NSF24FunctionKey as u16,
        "f25" => NSF25FunctionKey as u16,
        "f26" => NSF26FunctionKey as u16,
        "f27" => NSF27FunctionKey as u16,
        "f28" => NSF28FunctionKey as u16,
        "f29" => NSF29FunctionKey as u16,
        "f30" => NSF30FunctionKey as u16,
        "f31" => NSF31FunctionKey as u16,
        "f32" => NSF32FunctionKey as u16,
        "f33" => NSF33FunctionKey as u16,
        "f34" => NSF34FunctionKey as u16,
        "f35" => NSF35FunctionKey as u16,
        _ => return Cow::Borrowed(key),
    };
    Cow::Owned(String::from_utf16(&[code]).unwrap())
}

fn read_modifiers(native_event: &NSEvent) -> Modifiers {
    let modifiers = native_event.modifierFlags();
    let control = modifiers.contains(NSEventModifierFlags::Control);
    let alt = modifiers.contains(NSEventModifierFlags::Option);
    let shift = modifiers.contains(NSEventModifierFlags::Shift);
    let command = modifiers.contains(NSEventModifierFlags::Command);
    let function = modifiers.contains(NSEventModifierFlags::Function);

    Modifiers {
        control,
        alt,
        shift,
        platform: command,
        function,
    }
}

pub(crate) fn platform_input_from_native(
    native_event: &NSEvent,
    window_height: Option<Pixels>,
) -> Option<PlatformInput> {
    let event_type = native_event.r#type();

    // Filter out event types that aren't in the NSEventType enum.
    // See https://github.com/servo/cocoa-rs/issues/155#issuecomment-323482792 for details.
    match event_type.0 as u64 {
        0 | 21 | 32 | 33 | 35 | 36 | 37 => {
            return None;
        }
        _ => {}
    }

    match event_type {
        NSEventType::FlagsChanged => {
            Some(PlatformInput::ModifiersChanged(ModifiersChangedEvent {
                modifiers: read_modifiers(native_event),
                capslock: Capslock {
                    on: native_event
                        .modifierFlags()
                        .contains(NSEventModifierFlags::CapsLock),
                },
            }))
        }
        NSEventType::KeyDown => Some(PlatformInput::KeyDown(KeyDownEvent {
            keystroke: parse_keystroke(native_event),
            is_held: native_event.isARepeat(),
            prefer_character_input: false,
        })),
        NSEventType::KeyUp => Some(PlatformInput::KeyUp(KeyUpEvent {
            keystroke: parse_keystroke(native_event),
        })),
        NSEventType::LeftMouseDown
        | NSEventType::RightMouseDown
        | NSEventType::OtherMouseDown => {
            let button = match native_event.buttonNumber() {
                0 => MouseButton::Left,
                1 => MouseButton::Right,
                2 => MouseButton::Middle,
                3 => MouseButton::Navigate(NavigationDirection::Back),
                4 => MouseButton::Navigate(NavigationDirection::Forward),
                _ => return None,
            };
            window_height.map(|window_height| {
                PlatformInput::MouseDown(MouseDownEvent {
                    button,
                    position: point(
                        px(native_event.locationInWindow().x as f32),
                        window_height - px(native_event.locationInWindow().y as f32),
                    ),
                    modifiers: read_modifiers(native_event),
                    click_count: native_event.clickCount() as usize,
                    first_mouse: false,
                })
            })
        }
        NSEventType::LeftMouseUp
        | NSEventType::RightMouseUp
        | NSEventType::OtherMouseUp => {
            let button = match native_event.buttonNumber() {
                0 => MouseButton::Left,
                1 => MouseButton::Right,
                2 => MouseButton::Middle,
                3 => MouseButton::Navigate(NavigationDirection::Back),
                4 => MouseButton::Navigate(NavigationDirection::Forward),
                _ => return None,
            };

            window_height.map(|window_height| {
                PlatformInput::MouseUp(MouseUpEvent {
                    button,
                    position: point(
                        px(native_event.locationInWindow().x as f32),
                        window_height - px(native_event.locationInWindow().y as f32),
                    ),
                    modifiers: read_modifiers(native_event),
                    click_count: native_event.clickCount() as usize,
                })
            })
        }
        NSEventType::Pressure => {
            let stage = native_event.stage();
            let pressure = native_event.pressure();

            window_height.map(|window_height| {
                PlatformInput::MousePressure(MousePressureEvent {
                    stage: match stage {
                        1 => PressureStage::Normal,
                        2 => PressureStage::Force,
                        _ => PressureStage::Zero,
                    },
                    pressure,
                    modifiers: read_modifiers(native_event),
                    position: point(
                        px(native_event.locationInWindow().x as f32),
                        window_height - px(native_event.locationInWindow().y as f32),
                    ),
                })
            })
        }
        // Some mice (like Logitech MX Master) send navigation buttons as swipe events
        NSEventType::Swipe => {
            let navigation_direction = match native_event.phase() {
                NSEventPhase::Ended => match native_event.deltaX() {
                    x if x > 0.0 => Some(NavigationDirection::Back),
                    x if x < 0.0 => Some(NavigationDirection::Forward),
                    _ => return None,
                },
                _ => return None,
            };

            match navigation_direction {
                Some(direction) => window_height.map(|window_height| {
                    PlatformInput::MouseDown(MouseDownEvent {
                        button: MouseButton::Navigate(direction),
                        position: point(
                            px(native_event.locationInWindow().x as f32),
                            window_height - px(native_event.locationInWindow().y as f32),
                        ),
                        modifiers: read_modifiers(native_event),
                        click_count: 1,
                        first_mouse: false,
                    })
                }),
                _ => None,
            }
        }
        NSEventType::Magnify => window_height.map(|window_height| {
            let phase = match native_event.phase() {
                NSEventPhase::MayBegin | NSEventPhase::Began => TouchPhase::Started,
                NSEventPhase::Ended => TouchPhase::Ended,
                _ => TouchPhase::Moved,
            };

            let magnification = native_event.magnification() as f32;

            PlatformInput::Pinch(PinchEvent {
                position: point(
                    px(native_event.locationInWindow().x as f32),
                    window_height - px(native_event.locationInWindow().y as f32),
                ),
                delta: magnification,
                modifiers: read_modifiers(native_event),
                phase,
            })
        }),
        NSEventType::ScrollWheel => window_height.map(|window_height| {
            let phase = match native_event.phase() {
                NSEventPhase::MayBegin | NSEventPhase::Began => TouchPhase::Started,
                NSEventPhase::Ended => TouchPhase::Ended,
                _ => TouchPhase::Moved,
            };

            let raw_data = point(
                native_event.scrollingDeltaX() as f32,
                native_event.scrollingDeltaY() as f32,
            );

            let delta = if native_event.hasPreciseScrollingDeltas() {
                ScrollDelta::Pixels(raw_data.map(px))
            } else {
                ScrollDelta::Lines(raw_data)
            };

            PlatformInput::ScrollWheel(ScrollWheelEvent {
                position: point(
                    px(native_event.locationInWindow().x as f32),
                    window_height - px(native_event.locationInWindow().y as f32),
                ),
                delta,
                touch_phase: phase,
                modifiers: read_modifiers(native_event),
            })
        }),
        NSEventType::LeftMouseDragged
        | NSEventType::RightMouseDragged
        | NSEventType::OtherMouseDragged => {
            let pressed_button = match native_event.buttonNumber() {
                0 => MouseButton::Left,
                1 => MouseButton::Right,
                2 => MouseButton::Middle,
                3 => MouseButton::Navigate(NavigationDirection::Back),
                4 => MouseButton::Navigate(NavigationDirection::Forward),
                _ => return None,
            };

            window_height.map(|window_height| {
                PlatformInput::MouseMove(MouseMoveEvent {
                    pressed_button: Some(pressed_button),
                    position: point(
                        px(native_event.locationInWindow().x as f32),
                        window_height - px(native_event.locationInWindow().y as f32),
                    ),
                    modifiers: read_modifiers(native_event),
                })
            })
        }
        NSEventType::MouseMoved => window_height.map(|window_height| {
            PlatformInput::MouseMove(MouseMoveEvent {
                position: point(
                    px(native_event.locationInWindow().x as f32),
                    window_height - px(native_event.locationInWindow().y as f32),
                ),
                pressed_button: None,
                modifiers: read_modifiers(native_event),
            })
        }),
        NSEventType::MouseExited => window_height.map(|window_height| {
            PlatformInput::MouseExited(MouseExitEvent {
                position: point(
                    px(native_event.locationInWindow().x as f32),
                    window_height - px(native_event.locationInWindow().y as f32),
                ),
                pressed_button: None,
                modifiers: read_modifiers(native_event),
            })
        }),
        _ => None,
    }
}

fn parse_keystroke(native_event: &NSEvent) -> Keystroke {
    let characters = native_event
        .charactersIgnoringModifiers()
        .map(|s| s.to_string())
        .unwrap_or_default();
    let mut key_char = None;
    let first_char = characters.chars().next().map(|ch| ch as u16);
    let modifiers = native_event.modifierFlags();

    let control = modifiers.contains(NSEventModifierFlags::Control);
    let alt = modifiers.contains(NSEventModifierFlags::Option);
    let mut shift = modifiers.contains(NSEventModifierFlags::Shift);
    let command = modifiers.contains(NSEventModifierFlags::Command);
    let function = modifiers.contains(NSEventModifierFlags::Function)
        && first_char
            .is_none_or(|ch| !(NSUpArrowFunctionKey as u16..=NSModeSwitchFunctionKey as u16).contains(&ch));

    #[allow(non_upper_case_globals)]
    let key = match first_char {
        Some(SPACE_KEY) => {
            key_char = Some(" ".to_string());
            "space".to_string()
        }
        Some(TAB_KEY) => {
            key_char = Some("\t".to_string());
            "tab".to_string()
        }
        Some(ENTER_KEY) | Some(NUMPAD_ENTER_KEY) => {
            key_char = Some("\n".to_string());
            "enter".to_string()
        }
        Some(BACKSPACE_KEY) => "backspace".to_string(),
        Some(ESCAPE_KEY) => "escape".to_string(),
        Some(SHIFT_TAB_KEY) => "tab".to_string(),
        Some(ch) if ch == NSUpArrowFunctionKey as u16 => "up".to_string(),
        Some(ch) if ch == NSDownArrowFunctionKey as u16 => "down".to_string(),
        Some(ch) if ch == NSLeftArrowFunctionKey as u16 => "left".to_string(),
        Some(ch) if ch == NSRightArrowFunctionKey as u16 => "right".to_string(),
        Some(ch) if ch == NSPageUpFunctionKey as u16 => "pageup".to_string(),
        Some(ch) if ch == NSPageDownFunctionKey as u16 => "pagedown".to_string(),
        Some(ch) if ch == NSHomeFunctionKey as u16 => "home".to_string(),
        Some(ch) if ch == NSEndFunctionKey as u16 => "end".to_string(),
        Some(ch) if ch == NSDeleteFunctionKey as u16 => "delete".to_string(),
        Some(ch) if ch == NSHelpFunctionKey as u16 => "insert".to_string(),
        Some(ch) if ch == NSF1FunctionKey as u16 => "f1".to_string(),
        Some(ch) if ch == NSF2FunctionKey as u16 => "f2".to_string(),
        Some(ch) if ch == NSF3FunctionKey as u16 => "f3".to_string(),
        Some(ch) if ch == NSF4FunctionKey as u16 => "f4".to_string(),
        Some(ch) if ch == NSF5FunctionKey as u16 => "f5".to_string(),
        Some(ch) if ch == NSF6FunctionKey as u16 => "f6".to_string(),
        Some(ch) if ch == NSF7FunctionKey as u16 => "f7".to_string(),
        Some(ch) if ch == NSF8FunctionKey as u16 => "f8".to_string(),
        Some(ch) if ch == NSF9FunctionKey as u16 => "f9".to_string(),
        Some(ch) if ch == NSF10FunctionKey as u16 => "f10".to_string(),
        Some(ch) if ch == NSF11FunctionKey as u16 => "f11".to_string(),
        Some(ch) if ch == NSF12FunctionKey as u16 => "f12".to_string(),
        Some(ch) if ch == NSF13FunctionKey as u16 => "f13".to_string(),
        Some(ch) if ch == NSF14FunctionKey as u16 => "f14".to_string(),
        Some(ch) if ch == NSF15FunctionKey as u16 => "f15".to_string(),
        Some(ch) if ch == NSF16FunctionKey as u16 => "f16".to_string(),
        Some(ch) if ch == NSF17FunctionKey as u16 => "f17".to_string(),
        Some(ch) if ch == NSF18FunctionKey as u16 => "f18".to_string(),
        Some(ch) if ch == NSF19FunctionKey as u16 => "f19".to_string(),
        Some(ch) if ch == NSF20FunctionKey as u16 => "f20".to_string(),
        Some(ch) if ch == NSF21FunctionKey as u16 => "f21".to_string(),
        Some(ch) if ch == NSF22FunctionKey as u16 => "f22".to_string(),
        Some(ch) if ch == NSF23FunctionKey as u16 => "f23".to_string(),
        Some(ch) if ch == NSF24FunctionKey as u16 => "f24".to_string(),
        Some(ch) if ch == NSF25FunctionKey as u16 => "f25".to_string(),
        Some(ch) if ch == NSF26FunctionKey as u16 => "f26".to_string(),
        Some(ch) if ch == NSF27FunctionKey as u16 => "f27".to_string(),
        Some(ch) if ch == NSF28FunctionKey as u16 => "f28".to_string(),
        Some(ch) if ch == NSF29FunctionKey as u16 => "f29".to_string(),
        Some(ch) if ch == NSF30FunctionKey as u16 => "f30".to_string(),
        Some(ch) if ch == NSF31FunctionKey as u16 => "f31".to_string(),
        Some(ch) if ch == NSF32FunctionKey as u16 => "f32".to_string(),
        Some(ch) if ch == NSF33FunctionKey as u16 => "f33".to_string(),
        Some(ch) if ch == NSF34FunctionKey as u16 => "f34".to_string(),
        Some(ch) if ch == NSF35FunctionKey as u16 => "f35".to_string(),
        _ => {
            // Cases to test when modifying this:
            //
            //           qwerty key | none | cmd   | cmd-shift
            // * Armenian         s | ս    | cmd-s | cmd-shift-s  (layout is non-ASCII, so we use cmd layout)
            // * Dvorak+QWERTY    s | o    | cmd-s | cmd-shift-s  (layout switches on cmd)
            // * Ukrainian+QWERTY s | с    | cmd-s | cmd-shift-s  (macOS reports cmd-s instead of cmd-S)
            // * Czech            7 | ý    | cmd-ý | cmd-7        (layout has shifted numbers)
            // * Norwegian        7 | 7    | cmd-7 | cmd-/        (macOS reports cmd-shift-7 instead of cmd-/)
            // * Russian          7 | 7    | cmd-7 | cmd-&        (shift-7 is . but when cmd is down, should use cmd layout)
            // * German QWERTZ    ; | ö    | cmd-ö | cmd-Ö        (Raijin's shift special case only applies to a-z)
            //
            let mut chars_ignoring_modifiers =
                chars_for_modified_key(native_event.keyCode(), NO_MOD);
            let mut chars_with_shift =
                chars_for_modified_key(native_event.keyCode(), SHIFT_MOD);
            let always_use_cmd_layout = always_use_command_layout();

            // Handle Dvorak+QWERTY / Russian / Armenian
            if command || always_use_cmd_layout {
                let chars_with_cmd = chars_for_modified_key(native_event.keyCode(), CMD_MOD);
                let chars_with_both =
                    chars_for_modified_key(native_event.keyCode(), CMD_MOD | SHIFT_MOD);

                // We don't do this in the case that the shifted command key generates
                // the same character as the unshifted command key (Norwegian, e.g.)
                if chars_with_both != chars_with_cmd {
                    chars_with_shift = chars_with_both;

                // Handle edge-case where cmd-shift-s reports cmd-s instead of
                // cmd-shift-s (Ukrainian, etc.)
                } else if chars_with_cmd.to_ascii_uppercase() != chars_with_cmd {
                    chars_with_shift = chars_with_cmd.to_ascii_uppercase();
                }
                chars_ignoring_modifiers = chars_with_cmd;
            }

            if !control && !command && !function {
                let mut mods = NO_MOD;
                if shift {
                    mods |= SHIFT_MOD;
                }
                if alt {
                    mods |= OPTION_MOD;
                }

                key_char = Some(chars_for_modified_key(native_event.keyCode(), mods));
            }

            if shift
                && chars_ignoring_modifiers
                    .chars()
                    .all(|c| c.is_ascii_lowercase())
            {
                chars_ignoring_modifiers
            } else if shift {
                shift = false;
                chars_with_shift
            } else {
                chars_ignoring_modifiers
            }
        }
    };

    Keystroke {
        modifiers: Modifiers {
            control,
            alt,
            shift,
            platform: command,
            function,
        },
        key,
        key_char,
    }
}

fn always_use_command_layout() -> bool {
    if chars_for_modified_key(0, NO_MOD).is_ascii() {
        return false;
    }

    chars_for_modified_key(0, CMD_MOD).is_ascii()
}

const NO_MOD: u32 = 0;
const CMD_MOD: u32 = 1;
const SHIFT_MOD: u32 = 2;
const OPTION_MOD: u32 = 8;

fn chars_for_modified_key(code: CGKeyCode, modifiers: u32) -> String {
    // Values from: https://github.com/phracker/MacOSX-SDKs/blob/master/MacOSX10.6.sdk/System/Library/Frameworks/Carbon.framework/Versions/A/Frameworks/HIToolbox.framework/Versions/A/Headers/Events.h#L126
    // shifted >> 8 for UCKeyTranslate
    const CG_SPACE_KEY: u16 = 49;
    // https://github.com/phracker/MacOSX-SDKs/blob/master/MacOSX10.6.sdk/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/CarbonCore.framework/Versions/A/Headers/UnicodeUtilities.h#L278
    #[allow(non_upper_case_globals)]
    const kUCKeyActionDown: u16 = 0;
    #[allow(non_upper_case_globals)]
    const kUCKeyTranslateNoDeadKeysMask: u32 = 0;

    let keyboard_type = unsafe { LMGetKbdType() as u32 };
    const BUFFER_SIZE: usize = 4;
    let mut dead_key_state = 0;
    let mut buffer: [u16; BUFFER_SIZE] = [0; BUFFER_SIZE];
    let mut buffer_size: usize = 0;

    let keyboard = unsafe { TISCopyCurrentKeyboardLayoutInputSource() };
    if keyboard.is_null() {
        return "".to_string();
    }
    let layout_data = unsafe {
        TISGetInputSourceProperty(keyboard, kTISPropertyUnicodeKeyLayoutData as *const c_void)
            as *const CFData
    };
    if layout_data.is_null() {
        unsafe { CFRelease(keyboard as *const c_void) };
        return "".to_string();
    }
    let keyboard_layout = unsafe { (*layout_data).byte_ptr() };

    unsafe {
        UCKeyTranslate(
            keyboard_layout as *const c_void,
            code,
            kUCKeyActionDown,
            modifiers,
            keyboard_type,
            kUCKeyTranslateNoDeadKeysMask,
            &mut dead_key_state,
            BUFFER_SIZE,
            &mut buffer_size as *mut usize,
            &mut buffer as *mut u16,
        );
        if dead_key_state != 0 {
            UCKeyTranslate(
                keyboard_layout as *const c_void,
                CG_SPACE_KEY,
                kUCKeyActionDown,
                modifiers,
                keyboard_type,
                kUCKeyTranslateNoDeadKeysMask,
                &mut dead_key_state,
                BUFFER_SIZE,
                &mut buffer_size as *mut usize,
                &mut buffer as *mut u16,
            );
        }
        CFRelease(keyboard as *const c_void);
    }
    String::from_utf16(&buffer[..buffer_size]).unwrap_or_default()
}

unsafe extern "C" {
    fn CFRelease(cf: *const c_void);
}
