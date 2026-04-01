use super::*;

impl Dispatch<zwp_pointer_gestures_v1::ZwpPointerGesturesV1, ()> for WaylandClientStatePtr {
    fn event(
        _this: &mut Self,
        _: &zwp_pointer_gestures_v1::ZwpPointerGesturesV1,
        _: <zwp_pointer_gestures_v1::ZwpPointerGesturesV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // The gesture manager doesn't generate events
    }
}

impl Dispatch<zwp_pointer_gesture_pinch_v1::ZwpPointerGesturePinchV1, ()>
    for WaylandClientStatePtr
{
    fn event(
        this: &mut Self,
        _: &zwp_pointer_gesture_pinch_v1::ZwpPointerGesturePinchV1,
        event: <zwp_pointer_gesture_pinch_v1::ZwpPointerGesturePinchV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use inazuma::PinchEvent;

        let client = this.get_client();
        let mut state = client.borrow_mut();

        let Some(window) = state.mouse_focused_window.clone() else {
            return;
        };

        match event {
            zwp_pointer_gesture_pinch_v1::Event::Begin {
                serial: _,
                time: _,
                surface: _,
                fingers: _,
            } => {
                state.pinch_scale = 1.0;
                let input = PlatformInput::Pinch(PinchEvent {
                    position: state.mouse_location.unwrap_or(point(px(0.0), px(0.0))),
                    delta: 0.0,
                    modifiers: state.modifiers,
                    phase: TouchPhase::Started,
                });
                drop(state);
                window.handle_input(input);
            }
            zwp_pointer_gesture_pinch_v1::Event::Update { time: _, scale, .. } => {
                let new_absolute_scale = scale as f32;
                let previous_scale = state.pinch_scale;
                let zoom_delta = new_absolute_scale - previous_scale;
                state.pinch_scale = new_absolute_scale;

                let input = PlatformInput::Pinch(PinchEvent {
                    position: state.mouse_location.unwrap_or(point(px(0.0), px(0.0))),
                    delta: zoom_delta,
                    modifiers: state.modifiers,
                    phase: TouchPhase::Moved,
                });
                drop(state);
                window.handle_input(input);
            }
            zwp_pointer_gesture_pinch_v1::Event::End {
                serial: _,
                time: _,
                cancelled: _,
            } => {
                state.pinch_scale = 1.0;
                let input = PlatformInput::Pinch(PinchEvent {
                    position: state.mouse_location.unwrap_or(point(px(0.0), px(0.0))),
                    delta: 0.0,
                    modifiers: state.modifiers,
                    phase: TouchPhase::Ended,
                });
                drop(state);
                window.handle_input(input);
            }
            _ => {}
        }
    }
}

impl Dispatch<wp_fractional_scale_v1::WpFractionalScaleV1, ObjectId> for WaylandClientStatePtr {
    fn event(
        this: &mut Self,
        _: &wp_fractional_scale_v1::WpFractionalScaleV1,
        event: <wp_fractional_scale_v1::WpFractionalScaleV1 as Proxy>::Event,
        surface_id: &ObjectId,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let client = this.get_client();
        let mut state = client.borrow_mut();

        let Some(window) = get_window(&mut state, surface_id) else {
            return;
        };

        drop(state);
        window.handle_fractional_scale_event(event);
    }
}

impl Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, ObjectId>
    for WaylandClientStatePtr
{
    fn event(
        this: &mut Self,
        _: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        event: zxdg_toplevel_decoration_v1::Event,
        surface_id: &ObjectId,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let client = this.get_client();
        let mut state = client.borrow_mut();
        let Some(window) = get_window(&mut state, surface_id) else {
            return;
        };

        drop(state);
        window.handle_toplevel_decoration_event(event);
    }
}

impl Dispatch<wl_data_device::WlDataDevice, ()> for WaylandClientStatePtr {
    fn event(
        this: &mut Self,
        _: &wl_data_device::WlDataDevice,
        event: wl_data_device::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let client = this.get_client();
        let mut state = client.borrow_mut();

        match event {
            // Clipboard
            wl_data_device::Event::DataOffer { id: data_offer } => {
                state.data_offers.push(DataOffer::new(data_offer));
                if state.data_offers.len() > 2 {
                    // At most we store a clipboard offer and a drag and drop offer.
                    state.data_offers.remove(0).inner.destroy();
                }
            }
            wl_data_device::Event::Selection { id: data_offer } => {
                if let Some(offer) = data_offer {
                    let offer = state
                        .data_offers
                        .iter()
                        .find(|wrapper| wrapper.inner.id() == offer.id());
                    let offer = offer.cloned();
                    state.clipboard.set_offer(offer);
                } else {
                    state.clipboard.set_offer(None);
                }
            }

            // Drag and drop
            wl_data_device::Event::Enter {
                serial,
                surface,
                x,
                y,
                id: data_offer,
            } => {
                state.serial_tracker.update(SerialKind::DataDevice, serial);
                if let Some(data_offer) = data_offer {
                    let Some(drag_window) = get_window(&mut state, &surface.id()) else {
                        return;
                    };

                    const ACTIONS: DndAction = DndAction::Copy;
                    data_offer.set_actions(ACTIONS, ACTIONS);

                    let pipe = Pipe::new().unwrap();
                    data_offer.receive(FILE_LIST_MIME_TYPE.to_string(), unsafe {
                        BorrowedFd::borrow_raw(pipe.write.as_raw_fd())
                    });
                    let fd = pipe.read;
                    drop(pipe.write);

                    let read_task = state.common.background_executor.spawn(async {
                        let buffer = unsafe { read_fd(fd)? };
                        let text = String::from_utf8(buffer)?;
                        anyhow::Ok(text)
                    });

                    let this = this.clone();
                    state
                        .common
                        .foreground_executor
                        .spawn(async move {
                            let file_list = match read_task.await {
                                Ok(list) => list,
                                Err(err) => {
                                    log::error!("error reading drag and drop pipe: {err:?}");
                                    return;
                                }
                            };

                            let paths: SmallVec<[_; 2]> = file_list
                                .lines()
                                .filter_map(|path| Url::parse(path).log_err())
                                .filter_map(|url| url.to_file_path().log_err())
                                .collect();
                            let position = Point::new(x.into(), y.into());

                            // Prevent dropping text from other programs.
                            if paths.is_empty() {
                                data_offer.destroy();
                                return;
                            }

                            let input = PlatformInput::FileDrop(FileDropEvent::Entered {
                                position,
                                paths: inazuma::ExternalPaths(paths),
                            });

                            let client = this.get_client();
                            let mut state = client.borrow_mut();
                            state.drag.data_offer = Some(data_offer);
                            state.drag.window = Some(drag_window.clone());
                            state.drag.position = position;

                            drop(state);
                            drag_window.handle_input(input);
                        })
                        .detach();
                }
            }
            wl_data_device::Event::Motion { x, y, .. } => {
                let Some(drag_window) = state.drag.window.clone() else {
                    return;
                };
                let position = Point::new(x.into(), y.into());
                state.drag.position = position;

                let input = PlatformInput::FileDrop(FileDropEvent::Pending { position });
                drop(state);
                drag_window.handle_input(input);
            }
            wl_data_device::Event::Leave => {
                let Some(drag_window) = state.drag.window.clone() else {
                    return;
                };
                let data_offer = state.drag.data_offer.clone().unwrap();
                data_offer.destroy();

                state.drag.data_offer = None;
                state.drag.window = None;

                let input = PlatformInput::FileDrop(FileDropEvent::Exited {});
                drop(state);
                drag_window.handle_input(input);
            }
            wl_data_device::Event::Drop => {
                let Some(drag_window) = state.drag.window.clone() else {
                    return;
                };
                let data_offer = state.drag.data_offer.clone().unwrap();
                data_offer.finish();
                data_offer.destroy();

                state.drag.data_offer = None;
                state.drag.window = None;

                let input = PlatformInput::FileDrop(FileDropEvent::Submit {
                    position: state.drag.position,
                });
                drop(state);
                drag_window.handle_input(input);
            }
            _ => {}
        }
    }

    event_created_child!(WaylandClientStatePtr, wl_data_device::WlDataDevice, [
        wl_data_device::EVT_DATA_OFFER_OPCODE => (wl_data_offer::WlDataOffer, ()),
    ]);
}

impl Dispatch<wl_data_offer::WlDataOffer, ()> for WaylandClientStatePtr {
    fn event(
        this: &mut Self,
        data_offer: &wl_data_offer::WlDataOffer,
        event: wl_data_offer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let client = this.get_client();
        let mut state = client.borrow_mut();

        if let wl_data_offer::Event::Offer { mime_type } = event {
            // Drag and drop
            if mime_type == FILE_LIST_MIME_TYPE {
                let serial = state.serial_tracker.get(SerialKind::DataDevice);
                let mime_type = mime_type.clone();
                data_offer.accept(serial, Some(mime_type));
            }

            // Clipboard
            if let Some(offer) = state
                .data_offers
                .iter_mut()
                .find(|wrapper| wrapper.inner.id() == data_offer.id())
            {
                offer.add_mime_type(mime_type);
            }
        }
    }
}

impl Dispatch<wl_data_source::WlDataSource, ()> for WaylandClientStatePtr {
    fn event(
        this: &mut Self,
        data_source: &wl_data_source::WlDataSource,
        event: wl_data_source::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let client = this.get_client();
        let state = client.borrow_mut();

        match event {
            wl_data_source::Event::Send { mime_type, fd } => {
                state.clipboard.send(mime_type, fd);
            }
            wl_data_source::Event::Cancelled => {
                data_source.destroy();
            }
            _ => {}
        }
    }
}

impl Dispatch<zwp_primary_selection_device_v1::ZwpPrimarySelectionDeviceV1, ()>
    for WaylandClientStatePtr
{
    fn event(
        this: &mut Self,
        _: &zwp_primary_selection_device_v1::ZwpPrimarySelectionDeviceV1,
        event: zwp_primary_selection_device_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let client = this.get_client();
        let mut state = client.borrow_mut();

        match event {
            zwp_primary_selection_device_v1::Event::DataOffer { offer } => {
                let old_offer = state.primary_data_offer.replace(DataOffer::new(offer));
                if let Some(old_offer) = old_offer {
                    old_offer.inner.destroy();
                }
            }
            zwp_primary_selection_device_v1::Event::Selection { id: data_offer } => {
                if data_offer.is_some() {
                    let offer = state.primary_data_offer.clone();
                    state.clipboard.set_primary_offer(offer);
                } else {
                    state.clipboard.set_primary_offer(None);
                }
            }
            _ => {}
        }
    }

    event_created_child!(WaylandClientStatePtr, zwp_primary_selection_device_v1::ZwpPrimarySelectionDeviceV1, [
        zwp_primary_selection_device_v1::EVT_DATA_OFFER_OPCODE => (zwp_primary_selection_offer_v1::ZwpPrimarySelectionOfferV1, ()),
    ]);
}

impl Dispatch<zwp_primary_selection_offer_v1::ZwpPrimarySelectionOfferV1, ()>
    for WaylandClientStatePtr
{
    fn event(
        this: &mut Self,
        _data_offer: &zwp_primary_selection_offer_v1::ZwpPrimarySelectionOfferV1,
        event: zwp_primary_selection_offer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let client = this.get_client();
        let mut state = client.borrow_mut();

        if let zwp_primary_selection_offer_v1::Event::Offer { mime_type } = event
            && let Some(offer) = state.primary_data_offer.as_mut()
        {
            offer.add_mime_type(mime_type);
        }
    }
}

impl Dispatch<zwp_primary_selection_source_v1::ZwpPrimarySelectionSourceV1, ()>
    for WaylandClientStatePtr
{
    fn event(
        this: &mut Self,
        selection_source: &zwp_primary_selection_source_v1::ZwpPrimarySelectionSourceV1,
        event: zwp_primary_selection_source_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let client = this.get_client();
        let state = client.borrow_mut();

        match event {
            zwp_primary_selection_source_v1::Event::Send { mime_type, fd } => {
                state.clipboard.send_primary(mime_type, fd);
            }
            zwp_primary_selection_source_v1::Event::Cancelled => {
                selection_source.destroy();
            }
            _ => {}
        }
    }
}

impl Dispatch<XdgWmDialogV1, ()> for WaylandClientStatePtr {
    fn event(
        _: &mut Self,
        _: &XdgWmDialogV1,
        _: <XdgWmDialogV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<XdgDialogV1, ()> for WaylandClientStatePtr {
    fn event(
        _state: &mut Self,
        _proxy: &XdgDialogV1,
        _event: <XdgDialogV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}
