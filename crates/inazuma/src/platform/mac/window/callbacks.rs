use super::*;
use objc2::ClassType;

pub(super) fn handle_key_equivalent(ivars: &WindowIvars, native_view: &AnyObject, native_event: &objc2_app_kit::NSEvent) -> bool {
    handle_key_event(ivars, native_view, native_event, true)
}

pub(super) fn handle_key_down(ivars: &WindowIvars, native_view: &AnyObject, native_event: &objc2_app_kit::NSEvent) {
    handle_key_event(ivars, native_view, native_event, false);
}

pub(super) fn handle_key_up(ivars: &WindowIvars, native_view: &AnyObject, native_event: &objc2_app_kit::NSEvent) {
    handle_key_event(ivars, native_view, native_event, false);
}

// Things to test if you're modifying this method:
//  U.S. layout:
//   - The IME consumes characters like 'j' and 'k', which makes paging through `less` in
//     the terminal behave incorrectly by default. This behavior should be patched by our
//     IME integration
//   - `alt-t` should open the tasks menu
//   - In vim mode, this keybinding should work:
//     ```
//        {
//          "context": "Editor && vim_mode == insert",
//          "bindings": {"j j": "vim::NormalBefore"}
//        }
//     ```
//     and typing 'j k' in insert mode with this keybinding should insert the two characters
//  Brazilian layout:
//   - `" space` should create an unmarked quote
//   - `" backspace` should delete the marked quote
//   - `" "`should create an unmarked quote and a second marked quote
//   - `" up` should insert a quote, unmark it, and move up one line
//   - `" cmd-down` should insert a quote, unmark it, and move to the end of the file
//   - `cmd-ctrl-space` and clicking on an emoji should type it
//  Czech (QWERTY) layout:
//   - in vim mode `option-4`  should go to end of line (same as $)
//  Japanese (Romaji) layout:
//   - type `a i left down up enter enter` should create an unmarked text "愛"
fn handle_key_event(
    ivars: &WindowIvars,
    native_view: &AnyObject,
    native_event: &objc2_app_kit::NSEvent,
    key_equivalent: bool,
) -> bool {
    let window_state = ivars.get_state();
    let mut lock = window_state.as_ref().lock();

    let window_height = lock.content_size().height;
    let event = platform_input_from_native(native_event, Some(window_height));

    let Some(event) = event else {
        return false;
    };

    let run_callback = |event: PlatformInput| -> bool {
        let mut callback = window_state.as_ref().lock().event_callback.take();
        let handled: bool = if let Some(callback) = callback.as_mut() {
            !callback(event).propagate
        } else {
            false
        };
        window_state.as_ref().lock().event_callback = callback;
        handled
    };

    match event {
        PlatformInput::KeyDown(key_down_event) => {
            // For certain keystrokes, macOS will first dispatch a "key equivalent" event.
            // If that event isn't handled, it will then dispatch a "key down" event. GPUI
            // makes no distinction between these two types of events, so we need to ignore
            // the "key down" event if we've already just processed its "key equivalent" version.
            if key_equivalent {
                lock.last_key_equivalent = Some(key_down_event.clone());
            } else if lock.last_key_equivalent.take().as_ref() == Some(&key_down_event) {
                return false;
            }

            drop(lock);

            let is_composing =
                with_input_handler(ivars, |input_handler| input_handler.marked_text_range())
                    .flatten()
                    .is_some();

            // If we're composing, send the key to the input handler first;
            // otherwise we only send to the input handler if we don't have a matching binding.
            // The input handler may call `do_command_by_selector` if it doesn't know how to handle
            // a key. If it does so, it will return true so we won't send the key twice.
            // We also do this for non-printing keys (like arrow keys and escape) as the IME menu
            // may need them even if there is no marked text;
            // however we skip keys with control or the input handler adds control-characters to the buffer.
            // and keys with function, as the input handler swallows them.
            // and keys with platform (Cmd), so that Cmd+key events (e.g. Cmd+`) are not
            // consumed by the IME on non-QWERTY / dead-key layouts.
            if is_composing
                || (key_down_event.keystroke.key_char.is_none()
                    && !key_down_event.keystroke.modifiers.control
                    && !key_down_event.keystroke.modifiers.function
                    && !key_down_event.keystroke.modifiers.platform)
            {
                {
                    let mut lock = window_state.as_ref().lock();
                    lock.keystroke_for_do_command = Some(key_down_event.keystroke.clone());
                    lock.do_command_handled.take();
                    drop(lock);
                }

                let handled: bool = unsafe {
                    let input_context: *mut AnyObject = msg_send![native_view, inputContext];
                    if input_context.is_null() {
                        false
                    } else {
                        msg_send![&*input_context, handleEvent: native_event]
                    }
                };
                window_state.as_ref().lock().keystroke_for_do_command.take();
                if let Some(handled) = window_state.as_ref().lock().do_command_handled.take() {
                    return handled;
                } else if handled {
                    return true;
                }

                let handled = run_callback(PlatformInput::KeyDown(key_down_event));
                return handled;
            }

            let handled = run_callback(PlatformInput::KeyDown(key_down_event.clone()));
            if handled {
                return true;
            }

            if key_down_event.is_held
                && let Some(key_char) = key_down_event.keystroke.key_char.as_ref()
            {
                let handled = with_input_handler(ivars, |input_handler| {
                    if !input_handler.apple_press_and_hold_enabled() {
                        input_handler.replace_text_in_range(None, key_char);
                        return true;
                    }
                    false
                });
                if handled == Some(true) {
                    return true;
                }
            }

            // Don't send key equivalents to the input handler if there are key modifiers other
            // than Function key, or macOS shortcuts like cmd-` will stop working.
            if key_equivalent && key_down_event.keystroke.modifiers != Modifiers::function() {
                return false;
            }

            unsafe {
                let input_context: *mut AnyObject = msg_send![native_view, inputContext];
                msg_send![input_context, handleEvent: native_event]
            }
        }

        PlatformInput::KeyUp(_) => {
            drop(lock);
            run_callback(event)
        }

        _ => false,
    }
}

pub(super) fn handle_view_event(ivars: &WindowIvars, native_view: &AnyObject, native_event: &objc2_app_kit::NSEvent) {
    let window_state = ivars.get_state();
    let weak_window_state = Arc::downgrade(&window_state);
    let mut lock = window_state.as_ref().lock();
    let window_height = lock.content_size().height;
    let event = platform_input_from_native(native_event, Some(window_height));

    if let Some(mut event) = event {
        match &mut event {
            PlatformInput::MouseDown(
                event @ MouseDownEvent {
                    button: MouseButton::Left,
                    modifiers: Modifiers { control: true, .. },
                    ..
                },
            ) => {
                // On mac, a ctrl-left click should be handled as a right click.
                *event = MouseDownEvent {
                    button: MouseButton::Right,
                    modifiers: Modifiers {
                        control: false,
                        ..event.modifiers
                    },
                    click_count: 1,
                    ..*event
                };
            }

            // Handles focusing click.
            PlatformInput::MouseDown(
                event @ MouseDownEvent {
                    button: MouseButton::Left,
                    ..
                },
            ) if (lock.first_mouse) => {
                *event = MouseDownEvent {
                    first_mouse: true,
                    ..*event
                };
                lock.first_mouse = false;
            }

            // Because we map a ctrl-left_down to a right_down -> right_up let's ignore
            // the ctrl-left_up to avoid having a mismatch in button down/up events if the
            // user is still holding ctrl when releasing the left mouse button
            PlatformInput::MouseUp(
                event @ MouseUpEvent {
                    button: MouseButton::Left,
                    modifiers: Modifiers { control: true, .. },
                    ..
                },
            ) => {
                *event = MouseUpEvent {
                    button: MouseButton::Right,
                    modifiers: Modifiers {
                        control: false,
                        ..event.modifiers
                    },
                    click_count: 1,
                    ..*event
                };
            }

            _ => {}
        };

        match &event {
            PlatformInput::MouseDown(_) => {
                drop(lock);
                unsafe {
                    let input_context: *mut AnyObject = msg_send![native_view, inputContext];
                    let _: bool = msg_send![input_context, handleEvent: native_event];
                }
                lock = window_state.as_ref().lock();
            }
            PlatformInput::MouseMove(
                event @ MouseMoveEvent {
                    pressed_button: Some(_),
                    ..
                },
            ) => {
                // Synthetic drag is used for selecting long buffer contents while buffer is being scrolled.
                // External file drag and drop is able to emit its own synthetic mouse events which will conflict
                // with these ones.
                if !lock.external_files_dragged {
                    lock.synthetic_drag_counter += 1;
                    let executor = lock.foreground_executor.clone();
                    executor
                        .spawn(synthetic_drag(
                            weak_window_state,
                            lock.synthetic_drag_counter,
                            event.clone(),
                            lock.background_executor.clone(),
                        ))
                        .detach();
                }
            }

            PlatformInput::MouseUp(MouseUpEvent { .. }) => {
                lock.synthetic_drag_counter += 1;
            }

            PlatformInput::ModifiersChanged(ModifiersChangedEvent {
                modifiers,
                capslock,
            }) => {
                // Only raise modifiers changed event when they have actually changed
                if let Some(PlatformInput::ModifiersChanged(ModifiersChangedEvent {
                    modifiers: prev_modifiers,
                    capslock: prev_capslock,
                })) = &lock.previous_modifiers_changed_event
                    && prev_modifiers == modifiers
                    && prev_capslock == capslock
                {
                    return;
                }

                lock.previous_modifiers_changed_event = Some(event.clone());
            }

            _ => {}
        }

        if let Some(mut callback) = lock.event_callback.take() {
            drop(lock);
            callback(event);
            window_state.lock().event_callback = Some(callback);
        }
    }
}

pub(super) fn window_did_change_occlusion_state(ivars: &WindowIvars) {
    let window_state = ivars.get_state();
    let lock = &mut *window_state.lock();
    unsafe {
        let occlusion_state: objc2_app_kit::NSWindowOcclusionState =
            msg_send![lock.native_window, occlusionState];
        if occlusion_state
            .contains(objc2_app_kit::NSWindowOcclusionState::Visible)
        {
            lock.move_traffic_light();
            lock.start_display_link();
        } else {
            lock.stop_display_link();
        }
    }
}

pub(super) fn window_did_resize(ivars: &WindowIvars) {
    let window_state = ivars.get_state();
    window_state.as_ref().lock().move_traffic_light();
}

pub(super) fn window_will_enter_fullscreen(ivars: &WindowIvars) {
    let window_state = ivars.get_state();
    let mut lock = window_state.as_ref().lock();
    lock.fullscreen_restore_bounds = lock.bounds();

    let min_version = objc2_foundation::NSOperatingSystemVersion { majorVersion: 15, minorVersion: 3, patchVersion: 0 };

    if is_macos_version_at_least(min_version) {
        unsafe {
            let _: () = msg_send![lock.native_window, setTitlebarAppearsTransparent: false];
        }
    }
}

pub(super) fn window_will_exit_fullscreen(ivars: &WindowIvars) {
    let window_state = ivars.get_state();
    let lock = window_state.as_ref().lock();

    let min_version = objc2_foundation::NSOperatingSystemVersion { majorVersion: 15, minorVersion: 3, patchVersion: 0 };

    if is_macos_version_at_least(min_version) && lock.transparent_titlebar {
        unsafe {
            let _: () = msg_send![lock.native_window, setTitlebarAppearsTransparent: true];
        }
    }
}

pub(crate) fn is_macos_version_at_least(
    version: objc2_foundation::NSOperatingSystemVersion,
) -> bool {
    let process_info = objc2_foundation::NSProcessInfo::processInfo();
    process_info.isOperatingSystemAtLeastVersion(version)
}

pub(super) fn window_did_move(ivars: &WindowIvars) {
    let window_state = ivars.get_state();
    let mut lock = window_state.as_ref().lock();
    if let Some(mut callback) = lock.moved_callback.take() {
        drop(lock);
        callback();
        window_state.lock().moved_callback = Some(callback);
    }
}

// Update the window scale factor and drawable size, and call the resize callback if any.
fn update_window_scale_factor(window_state: &Arc<Mutex<MacWindowState>>) {
    let mut lock = window_state.as_ref().lock();
    let scale_factor = lock.scale_factor();
    let size = lock.content_size();
    let drawable_size = size.to_device_pixels(scale_factor);
    if let Some(layer) = lock.renderer.layer() {
        unsafe {
            let _: () = msg_send![
                layer,
                setContentsScale: scale_factor as f64
            ];
        }
    }

    lock.renderer.update_drawable_size(drawable_size);

    if let Some(mut callback) = lock.resize_callback.take() {
        let content_size = lock.content_size();
        let scale_factor = lock.scale_factor();
        drop(lock);
        callback(content_size, scale_factor);
        window_state.as_ref().lock().resize_callback = Some(callback);
    };
}

pub(super) fn window_did_change_screen(ivars: &WindowIvars) {
    let window_state = ivars.get_state();
    let mut lock = window_state.as_ref().lock();
    lock.start_display_link();
    drop(lock);
    update_window_scale_factor(&window_state);
}

pub(super) fn window_did_change_key_status(ivars: &WindowIvars, selector: objc2::runtime::Sel) {
    let window_state = ivars.get_state();
    let lock = window_state.lock();
    let is_active: bool = unsafe { msg_send![lock.native_window, isKeyWindow] };

    // When opening a pop-up while the application isn't active, Cocoa sends a spurious
    // `windowDidBecomeKey` message to the previous key window even though that window
    // isn't actually key. This causes a bug if the application is later activated while
    // the pop-up is still open, making it impossible to activate the previous key window
    // even if the pop-up gets closed. The only way to activate it again is to de-activate
    // the app and re-activate it, which is a pretty bad UX.
    // The following code detects the spurious event and invokes `resignKeyWindow`:
    // in theory, we're not supposed to invoke this method manually but it balances out
    // the spurious `becomeKeyWindow` event and helps us work around that bug.
    if selector == sel!(windowDidBecomeKey:) && !is_active {
        let native_window = lock.native_window;
        drop(lock);
        unsafe {
            let _: () = msg_send![native_window, resignKeyWindow];
        }
        return;
    }

    let executor = lock.foreground_executor.clone();
    drop(lock);

    // When a window becomes active, trigger an immediate synchronous frame request to prevent
    // tab flicker when switching between windows in native tabs mode.
    //
    // This is only done on subsequent activations (not the first) to ensure the initial focus
    // path is properly established. Without this guard, the focus state would remain unset until
    // the first mouse click, causing keybindings to be non-functional.
    if selector == sel!(windowDidBecomeKey:) && is_active {
        let window_state = ivars.get_state();
        let mut lock = window_state.lock();

        if lock.activated_least_once {
            if let Some(mut callback) = lock.request_frame_callback.take() {
                lock.renderer.set_presents_with_transaction(true);
                lock.stop_display_link();
                drop(lock);
                callback(Default::default());

                let mut lock = window_state.lock();
                lock.request_frame_callback = Some(callback);
                lock.renderer.set_presents_with_transaction(false);
                lock.start_display_link();
            }
        } else {
            lock.activated_least_once = true;
        }
    }

    executor
        .spawn(async move {
            let mut lock = window_state.as_ref().lock();
            if is_active {
                lock.move_traffic_light();
            }

            if let Some(mut callback) = lock.activate_callback.take() {
                drop(lock);
                callback(is_active);
                window_state.lock().activate_callback = Some(callback);
            };
        })
        .detach();
}

pub(super) fn window_should_close(ivars: &WindowIvars) -> bool {
    let window_state = ivars.get_state();
    let mut lock = window_state.as_ref().lock();
    if let Some(mut callback) = lock.should_close_callback.take() {
        drop(lock);
        let should_close = callback();
        window_state.lock().should_close_callback = Some(callback);
        should_close
    } else {
        true
    }
}

pub(super) fn close_window(ivars: &WindowIvars, this: &AnyObject) {
    unsafe {
        let close_callback = {
            let window_state = ivars.get_state();
            let mut lock = window_state.as_ref().lock();
            lock.close_callback.take()
        };

        if let Some(callback) = close_callback {
            callback();
        }

        let _: () = msg_send![super(this, objc2_app_kit::NSWindow::class()), close];
    }
}

pub(super) fn make_backing_layer(ivars: &WindowIvars) -> *mut AnyObject {
    let window_state = ivars.get_state();
    let window_state = window_state.as_ref().lock();
    window_state.renderer.layer_ptr() as *mut AnyObject
}

pub(super) fn view_did_change_backing_properties(ivars: &WindowIvars) {
    let window_state = ivars.get_state();
    update_window_scale_factor(&window_state);
}

pub(super) fn set_frame_size(ivars: &WindowIvars, native_view: &AnyObject, size: NSSize) {
    fn convert(value: NSSize) -> Size<Pixels> {
        Size {
            width: px(value.width as f32),
            height: px(value.height as f32),
        }
    }

    let window_state = ivars.get_state();
    let mut lock = window_state.as_ref().lock();

    let new_size = convert(size);
    let old_size = unsafe {
        let old_frame: NSRect = msg_send![native_view, frame];
        convert(old_frame.size)
    };

    if old_size == new_size {
        return;
    }

    unsafe {
        let _: () = msg_send![super(native_view, objc2_app_kit::NSView::class()), setFrameSize: size];
    }

    let scale_factor = lock.scale_factor();
    let drawable_size = new_size.to_device_pixels(scale_factor);
    lock.renderer.update_drawable_size(drawable_size);

    if let Some(mut callback) = lock.resize_callback.take() {
        let content_size = lock.content_size();
        let scale_factor = lock.scale_factor();
        drop(lock);
        callback(content_size, scale_factor);
        window_state.lock().resize_callback = Some(callback);
    };
}

pub(super) fn display_layer(ivars: &WindowIvars) {
    let window_state = ivars.get_state();
    let mut lock = window_state.lock();
    if let Some(mut callback) = lock.request_frame_callback.take() {
        lock.renderer.set_presents_with_transaction(true);
        lock.stop_display_link();
        drop(lock);
        callback(Default::default());

        let mut lock = window_state.lock();
        lock.request_frame_callback = Some(callback);
        lock.renderer.set_presents_with_transaction(false);
        lock.start_display_link();
    }
}

pub(super) extern "C" fn step(view: *mut c_void) {
    use objc2::DefinedClass;
    let view = view as *mut class_registration::GPUIView;
    let window_state = unsafe { (&*view).ivars().get_state() };
    let mut lock = window_state.lock();

    if let Some(mut callback) = lock.request_frame_callback.take() {
        drop(lock);
        callback(Default::default());
        window_state.lock().request_frame_callback = Some(callback);
    }
}

pub(super) fn valid_attributes_for_marked_text() -> *mut AnyObject {
    unsafe { msg_send![objc2_foundation::NSArray::<AnyObject>::class(), array] }
}

pub(super) fn has_marked_text(ivars: &WindowIvars) -> bool {
    let has_marked_text_result =
        with_input_handler(ivars, |input_handler| input_handler.marked_text_range()).flatten();

    has_marked_text_result.is_some()
}

pub(super) fn marked_range(ivars: &WindowIvars) -> NSRange {
    let marked_range_result =
        with_input_handler(ivars, |input_handler| input_handler.marked_text_range()).flatten();

    marked_range_result.map_or(ns_range_invalid(), |range| range.into())
}

pub(super) fn selected_range(ivars: &WindowIvars) -> NSRange {
    let selected_range_result = with_input_handler(ivars, |input_handler| {
        input_handler.selected_text_range(false)
    })
    .flatten();

    selected_range_result.map_or(ns_range_invalid(), |selection| selection.range.into())
}

pub(super) fn first_rect_for_character_range(
    ivars: &WindowIvars,
    range: NSRange,
    _actual_range: *mut c_void,
) -> NSRect {
    let frame = get_frame(ivars);
    with_input_handler(ivars, |input_handler| {
        input_handler.bounds_for_range(ns_range_to_range(range)?)
    })
    .flatten()
    .map_or(
        NSRect::new(NSPoint::new(0., 0.), NSSize::new(0., 0.)),
        |bounds| {
            NSRect::new(
                NSPoint::new(
                    frame.origin.x + bounds.origin.x.as_f32() as f64,
                    frame.origin.y + frame.size.height
                        - bounds.origin.y.as_f32() as f64
                        - bounds.size.height.as_f32() as f64,
                ),
                NSSize::new(
                    bounds.size.width.as_f32() as f64,
                    bounds.size.height.as_f32() as f64,
                ),
            )
        },
    )
}

pub(super) fn get_frame(ivars: &WindowIvars) -> NSRect {
    unsafe {
        let state = ivars.get_state();
        let lock = state.lock();
        let mut frame: NSRect = msg_send![lock.native_window, frame];
        let content_layout_rect: NSRect = msg_send![lock.native_window, contentLayoutRect];
        let style_mask: objc2_app_kit::NSWindowStyleMask =
            msg_send![lock.native_window, styleMask];
        if !style_mask.contains(objc2_app_kit::NSWindowStyleMask::FullSizeContentView) {
            frame.origin.y -= frame.size.height - content_layout_rect.size.height;
        }
        frame
    }
}

pub(super) fn insert_text(ivars: &WindowIvars, text: &AnyObject, replacement_range: NSRange) {
    unsafe {
        let is_attributed_string: bool =
            msg_send![text, isKindOfClass: objc2_foundation::NSAttributedString::class()];
        let text: &AnyObject = if is_attributed_string {
            msg_send![text, string]
        } else {
            text
        };

        let ns_string: &objc2_foundation::NSString = &*(text as *const AnyObject as *const objc2_foundation::NSString);
        let text_str = ns_string.to_string();
        let replacement_range = ns_range_to_range(replacement_range);
        with_input_handler(ivars, |input_handler| {
            input_handler.replace_text_in_range(replacement_range, &text_str)
        });
    }
}

pub(super) fn set_marked_text(
    ivars: &WindowIvars,
    text: &AnyObject,
    selected_range: NSRange,
    replacement_range: NSRange,
) {
    unsafe {
        let is_attributed_string: bool =
            msg_send![text, isKindOfClass: objc2_foundation::NSAttributedString::class()];
        let text: &AnyObject = if is_attributed_string {
            msg_send![text, string]
        } else {
            text
        };
        let selected_range = ns_range_to_range(selected_range);
        let replacement_range = ns_range_to_range(replacement_range);
        let ns_string: &objc2_foundation::NSString = &*(text as *const AnyObject as *const objc2_foundation::NSString);
        let text_str = ns_string.to_string();
        with_input_handler(ivars, |input_handler| {
            input_handler.replace_and_mark_text_in_range(replacement_range, &text_str, selected_range)
        });
    }
}

pub(super) fn unmark_text(ivars: &WindowIvars) {
    with_input_handler(ivars, |input_handler| input_handler.unmark_text());
}

pub(super) fn attributed_substring_for_proposed_range(
    ivars: &WindowIvars,
    range: NSRange,
    actual_range: *mut c_void,
) -> *mut AnyObject {
    with_input_handler(ivars, |input_handler| {
        let range = ns_range_to_range(range)?;
        if range.is_empty() {
            return None;
        }
        let mut adjusted: Option<Range<usize>> = None;

        let selected_text = input_handler.text_for_range(range.clone(), &mut adjusted)?;
        if let Some(adjusted) = adjusted
            && adjusted != range
        {
            unsafe { (actual_range as *mut NSRange).write(NSRange::from(adjusted)) };
        }
        unsafe {
            let ns_str = objc2_foundation::NSString::from_str(&selected_text);
            let string: *mut AnyObject = msg_send![
                objc2_foundation::NSAttributedString::class(),
                alloc
            ];
            let string: *mut AnyObject = msg_send![string, initWithString: &*ns_str];
            Some(string)
        }
    })
    .flatten()
    .unwrap_or(ptr::null_mut())
}

// We ignore which selector it asks us to do because the user may have
// bound the shortcut to something else.
pub(super) fn do_command_by_selector(ivars: &WindowIvars) {
    let state = ivars.get_state();
    let mut lock = state.as_ref().lock();
    let keystroke = lock.keystroke_for_do_command.take();
    let mut event_callback = lock.event_callback.take();
    drop(lock);

    if let Some((keystroke, callback)) = keystroke.zip(event_callback.as_mut()) {
        let handled = (callback)(PlatformInput::KeyDown(KeyDownEvent {
            keystroke,
            is_held: false,
            prefer_character_input: false,
        }));
        state.as_ref().lock().do_command_handled = Some(!handled.propagate);
    }

    state.as_ref().lock().event_callback = event_callback;
}

pub(super) fn view_did_change_effective_appearance(ivars: &WindowIvars) {
    let state = ivars.get_state();
    let mut lock = state.as_ref().lock();
    if let Some(mut callback) = lock.appearance_changed_callback.take() {
        drop(lock);
        callback();
        state.lock().appearance_changed_callback = Some(callback);
    }
}

async fn synthetic_drag(
    window_state: Weak<Mutex<MacWindowState>>,
    drag_id: usize,
    event: MouseMoveEvent,
    executor: BackgroundExecutor,
) {
    loop {
        executor.timer(Duration::from_millis(16)).await;
        if let Some(window_state) = window_state.upgrade() {
            let mut lock = window_state.lock();
            if lock.synthetic_drag_counter == drag_id {
                if let Some(mut callback) = lock.event_callback.take() {
                    drop(lock);
                    callback(PlatformInput::MouseMove(event.clone()));
                    window_state.lock().event_callback = Some(callback);
                }
            } else {
                break;
            }
        }
    }
}

pub(super) fn with_input_handler<F, R>(ivars: &WindowIvars, f: F) -> Option<R>
where
    F: FnOnce(&mut PlatformInputHandler) -> R,
{
    let window_state = ivars.get_state();
    let mut lock = window_state.as_ref().lock();
    if let Some(mut input_handler) = lock.input_handler.take() {
        drop(lock);
        let result = f(&mut input_handler);
        window_state.lock().input_handler = Some(input_handler);
        Some(result)
    } else {
        None
    }
}

pub(super) fn add_titlebar_accessory_view_controller(this: &AnyObject, view_controller: &AnyObject) {
    unsafe {
        let _: () = msg_send![
            super(this, objc2_app_kit::NSWindow::class()),
            addTitlebarAccessoryViewController: view_controller
        ];

        // Hide the native tab bar and set its height to 0, since we render our own.
        let accessory_view: *mut AnyObject = msg_send![view_controller, view];
        let _: () = msg_send![accessory_view, setHidden: true];
        let mut frame: NSRect = msg_send![accessory_view, frame];
        frame.size.height = 0.0;
        let _: () = msg_send![accessory_view, setFrame: frame];
    }
}

pub(super) fn move_tab_to_new_window(ivars: &WindowIvars, this: &AnyObject) {
    unsafe {
        let _: () = msg_send![
            super(this, objc2_app_kit::NSWindow::class()),
            moveTabToNewWindow: ptr::null_mut::<AnyObject>()
        ];

        let window_state = ivars.get_state();
        let mut lock = window_state.as_ref().lock();
        if let Some(mut callback) = lock.move_tab_to_new_window_callback.take() {
            drop(lock);
            callback();
            window_state.lock().move_tab_to_new_window_callback = Some(callback);
        }
    }
}

pub(super) fn merge_all_windows(ivars: &WindowIvars, this: &AnyObject) {
    unsafe {
        let _: () = msg_send![
            super(this, objc2_app_kit::NSWindow::class()),
            mergeAllWindows: ptr::null_mut::<AnyObject>()
        ];

        let window_state = ivars.get_state();
        let mut lock = window_state.as_ref().lock();
        if let Some(mut callback) = lock.merge_all_windows_callback.take() {
            drop(lock);
            callback();
            window_state.lock().merge_all_windows_callback = Some(callback);
        }
    }
}

pub(super) fn select_next_tab(ivars: &WindowIvars) {
    let window_state = ivars.get_state();
    let mut lock = window_state.as_ref().lock();
    if let Some(mut callback) = lock.select_next_tab_callback.take() {
        drop(lock);
        callback();
        window_state.lock().select_next_tab_callback = Some(callback);
    }
}

pub(super) fn select_previous_tab(ivars: &WindowIvars) {
    let window_state = ivars.get_state();
    let mut lock = window_state.as_ref().lock();
    if let Some(mut callback) = lock.select_previous_tab_callback.take() {
        drop(lock);
        callback();
        window_state.lock().select_previous_tab_callback = Some(callback);
    }
}

pub(super) fn toggle_tab_bar(ivars: &WindowIvars, this: &AnyObject) {
    unsafe {
        let _: () = msg_send![
            super(this, objc2_app_kit::NSWindow::class()),
            toggleTabBar: ptr::null_mut::<AnyObject>()
        ];

        let window_state = ivars.get_state();
        let mut lock = window_state.as_ref().lock();
        lock.move_traffic_light();

        if let Some(mut callback) = lock.toggle_tab_bar_callback.take() {
            drop(lock);
            callback();
            window_state.lock().toggle_tab_bar_callback = Some(callback);
        }
    }
}
