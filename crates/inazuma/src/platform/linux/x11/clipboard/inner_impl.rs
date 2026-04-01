use super::*;

impl Inner {
    fn atom_of(&self, selection: ClipboardKind) -> Atom {
        match selection {
            ClipboardKind::Clipboard => self.atoms.CLIPBOARD,
            ClipboardKind::Primary => self.atoms.PRIMARY,
            ClipboardKind::Secondary => self.atoms.SECONDARY,
        }
    }

    fn selection_of(&self, selection: ClipboardKind) -> &Selection {
        match selection {
            ClipboardKind::Clipboard => &self.clipboard,
            ClipboardKind::Primary => &self.primary,
            ClipboardKind::Secondary => &self.secondary,
        }
    }

    fn kind_of(&self, atom: Atom) -> Option<ClipboardKind> {
        match atom {
            a if a == self.atoms.CLIPBOARD => Some(ClipboardKind::Clipboard),
            a if a == self.atoms.PRIMARY => Some(ClipboardKind::Primary),
            a if a == self.atoms.SECONDARY => Some(ClipboardKind::Secondary),
            _ => None,
        }
    }

    fn is_owner(&self, selection: ClipboardKind) -> Result<bool> {
        let current = self
            .server
            .conn
            .get_selection_owner(self.atom_of(selection))
            .map_err(into_unknown)?
            .reply()
            .map_err(into_unknown)?
            .owner;

        Ok(current == self.server.win_id)
    }

    fn query_atom_name(&self, atom: x11rb::protocol::xproto::Atom) -> Result<String> {
        String::from_utf8(
            self.server
                .conn
                .get_atom_name(atom)
                .map_err(into_unknown)?
                .reply()
                .map_err(into_unknown)?
                .name,
        )
        .map_err(into_unknown)
    }

    fn atom_name(&self, atom: x11rb::protocol::xproto::Atom) -> &'static str {
        ATOM_NAME_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            match cache.entry(atom) {
                Entry::Occupied(entry) => *entry.get(),
                Entry::Vacant(entry) => {
                    let s = self
                        .query_atom_name(atom)
                        .map(|s| Box::leak(s.into_boxed_str()) as &str)
                        .unwrap_or("FAILED-TO-GET-THE-ATOM-NAME");
                    entry.insert(s);
                    s
                }
            }
        })
    }

    fn handle_read_selection_notify(
        &self,
        reader: &XContext,
        target_format: u32,
        using_incr: &mut bool,
        incr_data: &mut Vec<u8>,
        event: SelectionNotifyEvent,
    ) -> Result<ReadSelNotifyResult> {
        // The property being set to NONE means that the `convert_selection`
        // failed.

        // According to: https://tronche.com/gui/x/icccm/sec-2.html#s-2.4
        // the target must be set to the same as what we requested.
        if event.property == NONE || event.target != target_format {
            return Err(Error::ContentNotAvailable);
        }
        if self.kind_of(event.selection).is_none() {
            log::info!(
                "Received a SelectionNotify for a selection other than CLIPBOARD, PRIMARY or SECONDARY. This is unexpected."
            );
            return Ok(ReadSelNotifyResult::EventNotRecognized);
        }
        if *using_incr {
            log::warn!("Received a SelectionNotify while already expecting INCR segments.");
            return Ok(ReadSelNotifyResult::EventNotRecognized);
        }
        // Accept any property type. The property type will typically match the format type except
        // when it is `TARGETS` in which case it is `ATOM`. `ANY` is provided to handle the case
        // where the clipboard is not convertible to the requested format. In this case
        // `reply.type_` will have format information, but `bytes` will only be non-empty if `ANY`
        // is provided.
        let property_type = AtomEnum::ANY;
        // request the selection
        let mut reply = reader
            .conn
            .get_property(
                true,
                event.requestor,
                event.property,
                property_type,
                0,
                u32::MAX / 4,
            )
            .map_err(into_unknown)?
            .reply()
            .map_err(into_unknown)?;

        // we found something
        if reply.type_ == self.atoms.INCR {
            // Note that we call the get_property again because we are
            // indicating that we are ready to receive the data by deleting the
            // property, however deleting only works if the type matches the
            // property type. But the type didn't match in the previous call.
            reply = reader
                .conn
                .get_property(
                    true,
                    event.requestor,
                    event.property,
                    self.atoms.INCR,
                    0,
                    u32::MAX / 4,
                )
                .map_err(into_unknown)?
                .reply()
                .map_err(into_unknown)?;
            log::trace!("Receiving INCR segments");
            *using_incr = true;
            if reply.value_len == 4 {
                let min_data_len = reply
                    .value32()
                    .and_then(|mut vals| vals.next())
                    .unwrap_or(0);
                incr_data.reserve(min_data_len as usize);
            }
            Ok(ReadSelNotifyResult::IncrStarted)
        } else {
            Ok(ReadSelNotifyResult::GotData(ClipboardData {
                bytes: reply.value,
                format: reply.type_,
            }))
        }
    }

    /// Returns Ok(true) when the incr_data is ready
    fn handle_read_property_notify(
        &self,
        reader: &XContext,
        target_format: u32,
        using_incr: bool,
        incr_data: &mut Vec<u8>,
        timeout_end: &mut Instant,
        event: PropertyNotifyEvent,
    ) -> Result<bool> {
        if event.atom != self.atoms.ARBOARD_CLIPBOARD || event.state != Property::NEW_VALUE {
            return Ok(false);
        }
        if !using_incr {
            // This must mean the selection owner received our request, and is
            // now preparing the data
            return Ok(false);
        }
        let reply = reader
            .conn
            .get_property(
                true,
                event.window,
                event.atom,
                if target_format == self.atoms.TARGETS {
                    self.atoms.ATOM
                } else {
                    target_format
                },
                0,
                u32::MAX / 4,
            )
            .map_err(into_unknown)?
            .reply()
            .map_err(into_unknown)?;

        // log::trace!("Received segment. value_len {}", reply.value_len,);
        if reply.value_len == 0 {
            // This indicates that all the data has been sent.
            return Ok(true);
        }
        incr_data.extend(reply.value);

        // Let's reset our timeout, since we received a valid chunk.
        *timeout_end = Instant::now() + SHORT_TIMEOUT_DUR;

        // Not yet complete
        Ok(false)
    }

    fn handle_selection_request(&self, event: SelectionRequestEvent) -> Result<()> {
        let selection = match self.kind_of(event.selection) {
            Some(kind) => kind,
            None => {
                log::warn!(
                    "Received a selection request to a selection other than the CLIPBOARD, PRIMARY or SECONDARY. This is unexpected."
                );
                return Ok(());
            }
        };

        let success;
        // we are asked for a list of supported conversion targets
        if event.target == self.atoms.TARGETS {
            log::trace!(
                "Handling TARGETS, dst property is {}",
                self.atom_name(event.property)
            );
            let mut targets = Vec::with_capacity(10);
            targets.push(self.atoms.TARGETS);
            targets.push(self.atoms.SAVE_TARGETS);
            let data = self.selection_of(selection).data.read();
            if let Some(data_list) = &*data {
                for data in data_list {
                    targets.push(data.format);
                    if data.format == self.atoms.UTF8_STRING {
                        // When we are storing a UTF8 string,
                        // add all equivalent formats to the supported targets
                        targets.push(self.atoms.UTF8_MIME_0);
                        targets.push(self.atoms.UTF8_MIME_1);
                    }
                }
            }
            self.server
                .conn
                .change_property32(
                    PropMode::REPLACE,
                    event.requestor,
                    event.property,
                    // TODO: change to `AtomEnum::ATOM`
                    self.atoms.ATOM,
                    &targets,
                )
                .map_err(into_unknown)?;
            self.server.conn.flush().map_err(into_unknown)?;
            success = true;
        } else {
            log::trace!("Handling request for (probably) the clipboard contents.");
            let data = self.selection_of(selection).data.read();
            if let Some(data_list) = &*data {
                success = match data_list.iter().find(|d| d.format == event.target) {
                    Some(data) => {
                        self.server
                            .conn
                            .change_property8(
                                PropMode::REPLACE,
                                event.requestor,
                                event.property,
                                event.target,
                                &data.bytes,
                            )
                            .map_err(into_unknown)?;
                        self.server.conn.flush().map_err(into_unknown)?;
                        true
                    }
                    None => false,
                };
            } else {
                // This must mean that we lost ownership of the data
                // since the other side requested the selection.
                // Let's respond with the property set to none.
                success = false;
            }
        }
        // on failure we notify the requester of it
        let property = if success {
            event.property
        } else {
            AtomEnum::NONE.into()
        };
        // tell the requestor that we finished sending data
        self.server
            .conn
            .send_event(
                false,
                event.requestor,
                EventMask::NO_EVENT,
                SelectionNotifyEvent {
                    response_type: SELECTION_NOTIFY_EVENT,
                    sequence: event.sequence,
                    time: event.time,
                    requestor: event.requestor,
                    selection: event.selection,
                    target: event.target,
                    property,
                },
            )
            .map_err(into_unknown)?;

        self.server.conn.flush().map_err(into_unknown)
    }

    fn ask_clipboard_manager_to_request_our_data(&self) -> Result<()> {
        if self.server.win_id == 0 {
            // This shouldn't really ever happen but let's just check.
            log::error!("The server's window id was 0. This is unexpected");
            return Ok(());
        }

        if !self.is_owner(ClipboardKind::Clipboard)? {
            // We are not owning the clipboard, nothing to do.
            return Ok(());
        }
        if self
            .selection_of(ClipboardKind::Clipboard)
            .data
            .read()
            .is_none()
        {
            // If we don't have any data, there's nothing to do.
            return Ok(());
        }

        // It's important that we lock the state before sending the request
        // because we don't want the request server thread to lock the state
        // after the request but before we can lock it here.
        let mut handover_state = self.handover_state.lock();

        log::trace!("Sending the data to the clipboard manager");
        self.server
            .conn
            .convert_selection(
                self.server.win_id,
                self.atoms.CLIPBOARD_MANAGER,
                self.atoms.SAVE_TARGETS,
                self.atoms.ARBOARD_CLIPBOARD,
                Time::CURRENT_TIME,
            )
            .map_err(into_unknown)?;
        self.server.conn.flush().map_err(into_unknown)?;

        *handover_state = ManagerHandoverState::InProgress;
        let max_handover_duration = Duration::from_millis(100);

        // Note that we are using a parking_lot condvar here, which doesn't wake up
        // spuriously
        let result = self
            .handover_cv
            .wait_for(&mut handover_state, max_handover_duration);

        if *handover_state == ManagerHandoverState::Finished {
            return Ok(());
        }
        if result.timed_out() {
            log::warn!(
                "Could not hand the clipboard contents over to the clipboard manager. The request timed out."
            );
            return Ok(());
        }

        Err(Error::unknown(
            "The handover was not finished and the condvar didn't time out, yet the condvar wait ended. This should be unreachable.",
        ))
    }
}

