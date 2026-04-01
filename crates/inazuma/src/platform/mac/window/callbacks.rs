use super::*;

pub(super) extern "C" fn yes(_: &Object, _: Sel) -> BOOL {
    YES
}

pub(super) extern "C" fn dealloc_window(this: &Object, _: Sel) {
    unsafe {
        drop_window_state(this);
        let _: () = msg_send![super(this, class!(NSWindow)), dealloc];
    }
}

pub(super) extern "C" fn dealloc_view(this: &Object, _: Sel) {
    unsafe {
        drop_window_state(this);
        let _: () = msg_send![super(this, class!(NSView)), dealloc];
    }
}

pub(super) extern "C" fn handle_key_equivalent(this: &Object, _: Sel, native_event: id) -> BOOL {
    handle_key_event(this, native_event, true)
}

pub(super) extern "C" fn handle_key_down(this: &Object, _: Sel, native_event: id) {
    handle_key_event(this, native_event, false);
}

pub(super) extern "C" fn handle_key_up(this: &Object, _: Sel, native_event: id) {
    handle_key_event(this, native_event, false);
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
extern "C" fn handle_key_event(this: &Object, native_event: id, key_equivalent: bool) -> BOOL {
    let window_state = unsafe { get_window_state(this) };
    let mut lock = window_state.as_ref().lock();

    let window_height = lock.content_size().height;
    let event = unsafe { platform_input_from_native(native_event, Some(window_height)) };

    let Some(event) = event else {
        return NO;
    };

    let run_callback = |event: PlatformInput| -> BOOL {
        let mut callback = window_state.as_ref().lock().event_callback.take();
        let handled: BOOL = if let Some(callback) = callback.as_mut() {
            !callback(event).propagate as BOOL
        } else {
            NO
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
                return NO;
            }

            drop(lock);

            let is_composing =
                with_input_handler(this, |input_handler| input_handler.marked_text_range())
                    .flatten()
                    .is_some();

            // If we're composing, send the key to the input handler first;
            // otherwise we only send to the input handler if we don't have a matching binding.
            // The input handler may call `do_command_by_selector` if it doesn't know how to handle
            // a key. If it does so, it will return YES so we won't send the key twice.
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

                let handled: BOOL = unsafe {
                    let input_context: id = msg_send![this, inputContext];
                    msg_send![input_context, handleEvent: native_event]
                };
                window_state.as_ref().lock().keystroke_for_do_command.take();
                if let Some(handled) = window_state.as_ref().lock().do_command_handled.take() {
                    return handled as BOOL;
                } else if handled == YES {
                    return YES;
                }

                let handled = run_callback(PlatformInput::KeyDown(key_down_event));
                return handled;
            }

            let handled = run_callback(PlatformInput::KeyDown(key_down_event.clone()));
            if handled == YES {
                return YES;
            }

            if key_down_event.is_held
                && let Some(key_char) = key_down_event.keystroke.key_char.as_ref()
            {
                let handled = with_input_handler(this, |input_handler| {
                    if !input_handler.apple_press_and_hold_enabled() {
                        input_handler.replace_text_in_range(None, key_char);
                        return YES;
                    }
                    NO
                });
                if handled == Some(YES) {
                    return YES;
                }
            }

            // Don't send key equivalents to the input handler if there are key modifiers other
            // than Function key, or macOS shortcuts like cmd-` will stop working.
            if key_equivalent && key_down_event.keystroke.modifiers != Modifiers::function() {
                return NO;
            }

            unsafe {
                let input_context: id = msg_send![this, inputContext];
                msg_send![input_context, handleEvent: native_event]
            }
        }

        PlatformInput::KeyUp(_) => {
            drop(lock);
            run_callback(event)
        }

        _ => NO,
    }
}

pub(super) extern "C" fn handle_view_event(this: &Object, _: Sel, native_event: id) {
    let window_state = unsafe { get_window_state(this) };
    let weak_window_state = Arc::downgrade(&window_state);
    let mut lock = window_state.as_ref().lock();
    let window_height = lock.content_size().height;
    let event = unsafe { platform_input_from_native(native_event, Some(window_height)) };

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
                    let input_context: id = msg_send![this, inputContext];
                    msg_send![input_context, handleEvent: native_event]
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

pub(super) extern "C" fn window_did_change_occlusion_state(this: &Object, _: Sel, _: id) {
    let window_state = unsafe { get_window_state(this) };
    let lock = &mut *window_state.lock();
    unsafe {
        if lock
            .native_window
            .occlusionState()
            .contains(NSWindowOcclusionState::NSWindowOcclusionStateVisible)
        {
            lock.move_traffic_light();
            lock.start_display_link();
        } else {
            lock.stop_display_link();
        }
    }
}

pub(super) extern "C" fn window_did_resize(this: &Object, _: Sel, _: id) {
    let window_state = unsafe { get_window_state(this) };
    window_state.as_ref().lock().move_traffic_light();
}

pub(super) extern "C" fn window_will_enter_fullscreen(this: &Object, _: Sel, _: id) {
    let window_state = unsafe { get_window_state(this) };
    let mut lock = window_state.as_ref().lock();
    lock.fullscreen_restore_bounds = lock.bounds();

    let min_version = NSOperatingSystemVersion::new(15, 3, 0);

    if is_macos_version_at_least(min_version) {
        unsafe {
            lock.native_window.setTitlebarAppearsTransparent_(NO);
        }
    }
}

pub(super) extern "C" fn window_will_exit_fullscreen(this: &Object, _: Sel, _: id) {
    let window_state = unsafe { get_window_state(this) };
    let lock = window_state.as_ref().lock();

    let min_version = NSOperatingSystemVersion::new(15, 3, 0);

    if is_macos_version_at_least(min_version) && lock.transparent_titlebar {
        unsafe {
            lock.native_window.setTitlebarAppearsTransparent_(YES);
        }
    }
}

pub(crate) fn is_macos_version_at_least(version: NSOperatingSystemVersion) -> bool {
    unsafe { NSProcessInfo::processInfo(nil).isOperatingSystemAtLeastVersion(version) }
}

pub(super) extern "C" fn window_did_move(this: &Object, _: Sel, _: id) {
    let window_state = unsafe { get_window_state(this) };
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

pub(super) extern "C" fn window_did_change_screen(this: &Object, _: Sel, _: id) {
    let window_state = unsafe { get_window_state(this) };
    let mut lock = window_state.as_ref().lock();
    lock.start_display_link();
    drop(lock);
    update_window_scale_factor(&window_state);
}

pub(super) extern "C" fn window_did_change_key_status(this: &Object, selector: Sel, _: id) {
    let window_state = unsafe { get_window_state(this) };
    let lock = window_state.lock();
    let is_active = unsafe { lock.native_window.isKeyWindow() == YES };

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
        let window_state = unsafe { get_window_state(this) };
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

pub(super) extern "C" fn window_should_close(this: &Object, _: Sel, _: id) -> BOOL {
    let window_state = unsafe { get_window_state(this) };
    let mut lock = window_state.as_ref().lock();
    if let Some(mut callback) = lock.should_close_callback.take() {
        drop(lock);
        let should_close = callback();
        window_state.lock().should_close_callback = Some(callback);
        should_close as BOOL
    } else {
        YES
    }
}

pub(super) extern "C" fn close_window(this: &Object, _: Sel) {
    unsafe {
        let close_callback = {
            let window_state = get_window_state(this);
            let mut lock = window_state.as_ref().lock();
            lock.close_callback.take()
        };

        if let Some(callback) = close_callback {
            callback();
        }

        let _: () = msg_send![super(this, class!(NSWindow)), close];
    }
}

pub(super) extern "C" fn make_backing_layer(this: &Object, _: Sel) -> id {
    let window_state = unsafe { get_window_state(this) };
    let window_state = window_state.as_ref().lock();
    window_state.renderer.layer_ptr() as id
}

pub(super) extern "C" fn view_did_change_backing_properties(this: &Object, _: Sel) {
    let window_state = unsafe { get_window_state(this) };
    update_window_scale_factor(&window_state);
}

pub(super) extern "C" fn set_frame_size(this: &Object, _: Sel, size: NSSize) {
    fn convert(value: NSSize) -> Size<Pixels> {
        Size {
            width: px(value.width as f32),
            height: px(value.height as f32),
        }
    }

    let window_state = unsafe { get_window_state(this) };
    let mut lock = window_state.as_ref().lock();

    let new_size = convert(size);
    let old_size = unsafe {
        let old_frame: NSRect = msg_send![this, frame];
        convert(old_frame.size)
    };

    if old_size == new_size {
        return;
    }

    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), setFrameSize: size];
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

pub(super) extern "C" fn display_layer(this: &Object, _: Sel, _: id) {
    let window_state = unsafe { get_window_state(this) };
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
    let view = view as id;
    let window_state = unsafe { get_window_state(&*view) };
    let mut lock = window_state.lock();

    if let Some(mut callback) = lock.request_frame_callback.take() {
        drop(lock);
        callback(Default::default());
        window_state.lock().request_frame_callback = Some(callback);
    }
}

pub(super) extern "C" fn valid_attributes_for_marked_text(_: &Object, _: Sel) -> id {
    unsafe { msg_send![class!(NSArray), array] }
}

pub(super) extern "C" fn has_marked_text(this: &Object, _: Sel) -> BOOL {
    let has_marked_text_result =
        with_input_handler(this, |input_handler| input_handler.marked_text_range()).flatten();

    has_marked_text_result.is_some() as BOOL
}

pub(super) extern "C" fn marked_range(this: &Object, _: Sel) -> NSRange {
    let marked_range_result =
        with_input_handler(this, |input_handler| input_handler.marked_text_range()).flatten();

    marked_range_result.map_or(NSRange::invalid(), |range| range.into())
}

pub(super) extern "C" fn selected_range(this: &Object, _: Sel) -> NSRange {
    let selected_range_result = with_input_handler(this, |input_handler| {
        input_handler.selected_text_range(false)
    })
    .flatten();

    selected_range_result.map_or(NSRange::invalid(), |selection| selection.range.into())
}

pub(super) extern "C" fn first_rect_for_character_range(
    this: &Object,
    _: Sel,
    range: NSRange,
    _: id,
) -> NSRect {
    let frame = get_frame(this);
    with_input_handler(this, |input_handler| {
        input_handler.bounds_for_range(range.to_range()?)
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

pub(super) fn get_frame(this: &Object) -> NSRect {
    unsafe {
        let state = get_window_state(this);
        let lock = state.lock();
        let mut frame = NSWindow::frame(lock.native_window);
        let content_layout_rect: CGRect = msg_send![lock.native_window, contentLayoutRect];
        let style_mask: NSWindowStyleMask = msg_send![lock.native_window, styleMask];
        if !style_mask.contains(NSWindowStyleMask::NSFullSizeContentViewWindowMask) {
            frame.origin.y -= frame.size.height - content_layout_rect.size.height;
        }
        frame
    }
}

pub(super) extern "C" fn insert_text(this: &Object, _: Sel, text: id, replacement_range: NSRange) {
    unsafe {
        let is_attributed_string: BOOL =
            msg_send![text, isKindOfClass: [class!(NSAttributedString)]];
        let text: id = if is_attributed_string == YES {
            msg_send![text, string]
        } else {
            text
        };

        let text = text.to_str();
        let replacement_range = replacement_range.to_range();
        with_input_handler(this, |input_handler| {
            input_handler.replace_text_in_range(replacement_range, text)
        });
    }
}

pub(super) extern "C" fn set_marked_text(
    this: &Object,
    _: Sel,
    text: id,
    selected_range: NSRange,
    replacement_range: NSRange,
) {
    unsafe {
        let is_attributed_string: BOOL =
            msg_send![text, isKindOfClass: [class!(NSAttributedString)]];
        let text: id = if is_attributed_string == YES {
            msg_send![text, string]
        } else {
            text
        };
        let selected_range = selected_range.to_range();
        let replacement_range = replacement_range.to_range();
        let text = text.to_str();
        with_input_handler(this, |input_handler| {
            input_handler.replace_and_mark_text_in_range(replacement_range, text, selected_range)
        });
    }
}
pub(super) extern "C" fn unmark_text(this: &Object, _: Sel) {
    with_input_handler(this, |input_handler| input_handler.unmark_text());
}

pub(super) extern "C" fn attributed_substring_for_proposed_range(
    this: &Object,
    _: Sel,
    range: NSRange,
    actual_range: *mut c_void,
) -> id {
    with_input_handler(this, |input_handler| {
        let range = range.to_range()?;
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
            let string: id = msg_send![class!(NSAttributedString), alloc];
            let string: id = msg_send![string, initWithString: ns_string(&selected_text)];
            Some(string)
        }
    })
    .flatten()
    .unwrap_or(nil)
}

// We ignore which selector it asks us to do because the user may have
// bound the shortcut to something else.
pub(super) extern "C" fn do_command_by_selector(this: &Object, _: Sel, _: Sel) {
    let state = unsafe { get_window_state(this) };
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

pub(super) extern "C" fn view_did_change_effective_appearance(this: &Object, _: Sel) {
    unsafe {
        let state = get_window_state(this);
        let mut lock = state.as_ref().lock();
        if let Some(mut callback) = lock.appearance_changed_callback.take() {
            drop(lock);
            callback();
            state.lock().appearance_changed_callback = Some(callback);
        }
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

pub(super) fn with_input_handler<F, R>(window: &Object, f: F) -> Option<R>
where
    F: FnOnce(&mut PlatformInputHandler) -> R,
{
    let window_state = unsafe { get_window_state(window) };
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
