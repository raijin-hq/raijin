use super::*;

    fn event(
        this: &mut Self,
        text_input: &zwp_text_input_v3::ZwpTextInputV3,
        event: <zwp_text_input_v3::ZwpTextInputV3 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let client = this.get_client();
        let mut state = client.borrow_mut();
        match event {
            zwp_text_input_v3::Event::Enter { .. } => {
                drop(state);
                this.enable_ime();
            }
            zwp_text_input_v3::Event::Leave { .. } => {
                drop(state);
                this.disable_ime();
            }
            zwp_text_input_v3::Event::CommitString { text } => {
                state.composing = false;
                let Some(window) = state.keyboard_focused_window.clone() else {
                    return;
                };

                if let Some(commit_text) = text {
                    drop(state);
                    // IBus Intercepts keys like `a`, `b`, but those keys are needed for vim mode.
                    // We should only send ASCII characters to Zed, otherwise a user could remap a letter like `か` or `相`.
                    if commit_text.len() == 1 {
                        window.handle_input(PlatformInput::KeyDown(KeyDownEvent {
                            keystroke: Keystroke {
                                modifiers: Modifiers::default(),
                                key: commit_text.clone(),
                                key_char: Some(commit_text),
                            },
                            is_held: false,
                            prefer_character_input: false,
                        }));
                    } else {
                        window.handle_ime(ImeInput::InsertText(commit_text));
                    }
                }
            }
            zwp_text_input_v3::Event::PreeditString { text, .. } => {
                state.composing = true;
                state.ime_pre_edit = text;
            }
            zwp_text_input_v3::Event::Done { serial } => {
                let last_serial = state.serial_tracker.get(SerialKind::InputMethod);
                state.serial_tracker.update(SerialKind::InputMethod, serial);
                let Some(window) = state.keyboard_focused_window.clone() else {
                    return;
                };

                if let Some(text) = state.ime_pre_edit.take() {
                    drop(state);
                    window.handle_ime(ImeInput::SetMarkedText(text));
                    if let Some(area) = window.get_ime_area() {
                        text_input.set_cursor_rectangle(
                            f32::from(area.origin.x) as i32,
                            f32::from(area.origin.y) as i32,
                            f32::from(area.size.width) as i32,
                            f32::from(area.size.height) as i32,
                        );
                        if last_serial == serial {
                            text_input.commit();
                        }
                    }
                } else {
                    state.composing = false;
                    drop(state);
                    window.handle_ime(ImeInput::DeleteText);
                }
            }
            _ => {}
        }
    }
}

pub(super) fn linux_button_to_gpui(button: u32) -> Option<MouseButton> {
    // These values are coming from <linux/input-event-codes.h>.
    const BTN_LEFT: u32 = 0x110;
    const BTN_RIGHT: u32 = 0x111;
    const BTN_MIDDLE: u32 = 0x112;
    const BTN_SIDE: u32 = 0x113;
    const BTN_EXTRA: u32 = 0x114;
    const BTN_FORWARD: u32 = 0x115;
    const BTN_BACK: u32 = 0x116;

    Some(match button {
        BTN_LEFT => MouseButton::Left,
        BTN_RIGHT => MouseButton::Right,
        BTN_MIDDLE => MouseButton::Middle,
        BTN_BACK | BTN_SIDE => MouseButton::Navigate(NavigationDirection::Back),
        BTN_FORWARD | BTN_EXTRA => MouseButton::Navigate(NavigationDirection::Forward),
        _ => return None,
    })
}

impl Dispatch<wl_pointer::WlPointer, ()> for WaylandClientStatePtr {
    fn event(
        this: &mut Self,
        wl_pointer: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let client = this.get_client();
        let mut state = client.borrow_mut();

        match event {
            wl_pointer::Event::Enter {
                serial,
                surface,
                surface_x,
                surface_y,
                ..
            } => {
                state.serial_tracker.update(SerialKind::MouseEnter, serial);
                state.mouse_location = Some(point(px(surface_x as f32), px(surface_y as f32)));
                state.button_pressed = None;

                if let Some(window) = get_window(&mut state, &surface.id()) {
                    state.mouse_focused_window = Some(window.clone());

                    if state.enter_token.is_some() {
                        state.enter_token = None;
                    }
                    if let Some(style) = state.cursor_style {
                        if let CursorStyle::None = style {
                            let wl_pointer = state
                                .wl_pointer
                                .clone()
                                .expect("window is focused by pointer");
                            wl_pointer.set_cursor(serial, None, 0, 0);
                        } else if let Some(cursor_shape_device) = &state.cursor_shape_device {
                            cursor_shape_device.set_shape(serial, to_shape(style));
                        } else {
                            let scale = window.primary_output_scale();
                            state.cursor.set_icon(
                                wl_pointer,
                                serial,
                                cursor_style_to_icon_names(style),
                                scale,
                            );
                        }
                    }
                    drop(state);
                    window.set_hovered(true);
                }
            }
            wl_pointer::Event::Leave { .. } => {
                if let Some(focused_window) = state.mouse_focused_window.clone() {
                    let input = PlatformInput::MouseExited(MouseExitEvent {
                        position: state.mouse_location.unwrap(),
                        pressed_button: state.button_pressed,
                        modifiers: state.modifiers,
                    });
                    state.mouse_focused_window = None;
                    state.mouse_location = None;
                    state.button_pressed = None;

                    drop(state);
                    focused_window.handle_input(input);
                    focused_window.set_hovered(false);
                }
            }
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                if state.mouse_focused_window.is_none() {
                    return;
                }
                state.mouse_location = Some(point(px(surface_x as f32), px(surface_y as f32)));

                if let Some(window) = state.mouse_focused_window.clone() {
                    if window.is_blocked() {
                        let default_style = CursorStyle::Arrow;
                        if state.cursor_style != Some(default_style) {
                            let serial = state.serial_tracker.get(SerialKind::MouseEnter);
                            state.cursor_style = Some(default_style);

                            if let Some(cursor_shape_device) = &state.cursor_shape_device {
                                cursor_shape_device.set_shape(serial, to_shape(default_style));
                            } else {
                                // cursor-shape-v1 isn't supported, set the cursor using a surface.
                                let wl_pointer = state
                                    .wl_pointer
                                    .clone()
                                    .expect("window is focused by pointer");
                                let scale = window.primary_output_scale();
                                state.cursor.set_icon(
                                    &wl_pointer,
                                    serial,
                                    cursor_style_to_icon_names(default_style),
                                    scale,
                                );
                            }
                        }
                    }
                    if state
                        .keyboard_focused_window
                        .as_ref()
                        .is_some_and(|keyboard_window| window.ptr_eq(keyboard_window))
                    {
                        state.enter_token = None;
                    }
                    let input = PlatformInput::MouseMove(MouseMoveEvent {
                        position: state.mouse_location.unwrap(),
                        pressed_button: state.button_pressed,
                        modifiers: state.modifiers,
                    });
                    drop(state);
                    window.handle_input(input);
                }
            }
            wl_pointer::Event::Button {
                serial,
                button,
                state: WEnum::Value(button_state),
                ..
            } => {
                state.serial_tracker.update(SerialKind::MousePress, serial);
                let button = linux_button_to_gpui(button);
                let Some(button) = button else { return };
                if state.mouse_focused_window.is_none() {
                    return;
                }
                match button_state {
                    wl_pointer::ButtonState::Pressed => {
                        if let Some(window) = state.keyboard_focused_window.clone() {
                            if state.composing && state.text_input.is_some() {
                                drop(state);
                                // text_input_v3 don't have something like a reset function
                                this.disable_ime();
                                this.enable_ime();
                                window.handle_ime(ImeInput::UnmarkText);
                                state = client.borrow_mut();
                            } else if let (Some(text), Some(compose)) =
                                (state.pre_edit_text.take(), state.compose_state.as_mut())
                            {
                                compose.reset();
                                drop(state);
                                window.handle_ime(ImeInput::InsertText(text));
                                state = client.borrow_mut();
                            }
                        }
                        let click_elapsed = state.click.last_click.elapsed();

                        if click_elapsed < DOUBLE_CLICK_INTERVAL
                            && state
                                .click
                                .last_mouse_button
                                .is_some_and(|prev_button| prev_button == button)
                            && is_within_click_distance(
                                state.click.last_location,
                                state.mouse_location.unwrap(),
                            )
                        {
                            state.click.current_count += 1;
                        } else {
                            state.click.current_count = 1;
                        }

                        state.click.last_click = Instant::now();
                        state.click.last_mouse_button = Some(button);
                        state.click.last_location = state.mouse_location.unwrap();

                        state.button_pressed = Some(button);

                        if let Some(window) = state.mouse_focused_window.clone() {
                            let input = PlatformInput::MouseDown(MouseDownEvent {
                                button,
                                position: state.mouse_location.unwrap(),
                                modifiers: state.modifiers,
                                click_count: state.click.current_count,
                                first_mouse: state.enter_token.take().is_some(),
                            });
                            drop(state);
                            window.handle_input(input);
                        }
                    }
                    wl_pointer::ButtonState::Released => {
                        state.button_pressed = None;

                        if let Some(window) = state.mouse_focused_window.clone() {
                            let input = PlatformInput::MouseUp(MouseUpEvent {
                                button,
                                position: state.mouse_location.unwrap(),
                                modifiers: state.modifiers,
                                click_count: state.click.current_count,
                            });
                            drop(state);
                            window.handle_input(input);
                        }
                    }
                    _ => {}
                }
            }

            // Axis Events
            wl_pointer::Event::AxisSource {
                axis_source: WEnum::Value(axis_source),
            } => {
                state.axis_source = axis_source;
            }
            wl_pointer::Event::Axis {
                axis: WEnum::Value(axis),
                value,
                ..
            } => {
                if state.axis_source == AxisSource::Wheel {
                    return;
                }
                let axis = if state.modifiers.shift {
                    wl_pointer::Axis::HorizontalScroll
                } else {
                    axis
                };
                let axis_modifier = match axis {
                    wl_pointer::Axis::VerticalScroll => state.vertical_modifier,
                    wl_pointer::Axis::HorizontalScroll => state.horizontal_modifier,
                    _ => 1.0,
                };
                state.scroll_event_received = true;
                let scroll_delta = state
                    .continuous_scroll_delta
                    .get_or_insert(point(px(0.0), px(0.0)));
                let modifier = 3.0;
                match axis {
                    wl_pointer::Axis::VerticalScroll => {
                        scroll_delta.y += px(value as f32 * modifier * axis_modifier);
                    }
                    wl_pointer::Axis::HorizontalScroll => {
                        scroll_delta.x += px(value as f32 * modifier * axis_modifier);
                    }
                    _ => unreachable!(),
                }
            }
            wl_pointer::Event::AxisDiscrete {
                axis: WEnum::Value(axis),
                discrete,
            } => {
                state.scroll_event_received = true;
                let axis = if state.modifiers.shift {
                    wl_pointer::Axis::HorizontalScroll
                } else {
                    axis
                };
                let axis_modifier = match axis {
                    wl_pointer::Axis::VerticalScroll => state.vertical_modifier,
                    wl_pointer::Axis::HorizontalScroll => state.horizontal_modifier,
                    _ => 1.0,
                };

                let scroll_delta = state.discrete_scroll_delta.get_or_insert(point(0.0, 0.0));
                match axis {
                    wl_pointer::Axis::VerticalScroll => {
                        scroll_delta.y += discrete as f32 * axis_modifier * SCROLL_LINES;
                    }
                    wl_pointer::Axis::HorizontalScroll => {
                        scroll_delta.x += discrete as f32 * axis_modifier * SCROLL_LINES;
                    }
                    _ => unreachable!(),
                }
            }
            wl_pointer::Event::AxisValue120 {
                axis: WEnum::Value(axis),
                value120,
            } => {
                state.scroll_event_received = true;
                let axis = if state.modifiers.shift {
                    wl_pointer::Axis::HorizontalScroll
                } else {
                    axis
                };
                let axis_modifier = match axis {
                    wl_pointer::Axis::VerticalScroll => state.vertical_modifier,
                    wl_pointer::Axis::HorizontalScroll => state.horizontal_modifier,
                    _ => unreachable!(),
                };

                let scroll_delta = state.discrete_scroll_delta.get_or_insert(point(0.0, 0.0));
                let wheel_percent = value120 as f32 / 120.0;
                match axis {
                    wl_pointer::Axis::VerticalScroll => {
                        scroll_delta.y += wheel_percent * axis_modifier * SCROLL_LINES;
                    }
                    wl_pointer::Axis::HorizontalScroll => {
                        scroll_delta.x += wheel_percent * axis_modifier * SCROLL_LINES;
                    }
                    _ => unreachable!(),
                }
            }
            wl_pointer::Event::Frame => {
                if state.scroll_event_received {
                    state.scroll_event_received = false;
                    let continuous = state.continuous_scroll_delta.take();
                    let discrete = state.discrete_scroll_delta.take();
                    if let Some(continuous) = continuous {
                        if let Some(window) = state.mouse_focused_window.clone() {
                            let input = PlatformInput::ScrollWheel(ScrollWheelEvent {
                                position: state.mouse_location.unwrap(),
                                delta: ScrollDelta::Pixels(continuous),
                                modifiers: state.modifiers,
                                touch_phase: TouchPhase::Moved,
                            });
                            drop(state);
                            window.handle_input(input);
                        }
                    } else if let Some(discrete) = discrete
                        && let Some(window) = state.mouse_focused_window.clone()
                    {
                        let input = PlatformInput::ScrollWheel(ScrollWheelEvent {
                            position: state.mouse_location.unwrap(),
                            delta: ScrollDelta::Lines(discrete),
                            modifiers: state.modifiers,
                            touch_phase: TouchPhase::Moved,
                        });
                        drop(state);
                        window.handle_input(input);
                    }
                }
            }
            _ => {}
        }
    }
}

