use super::*;

pub(super) type Result<T, E = Error> = std::result::Result<T, E>;

pub(super) static CLIPBOARD: Mutex<Option<GlobalClipboard>> = parking_lot::const_mutex(None);

x11rb::atom_manager! {
    pub Atoms: AtomCookies {
        CLIPBOARD,
        PRIMARY,
        SECONDARY,

        CLIPBOARD_MANAGER,
        SAVE_TARGETS,
        TARGETS,
        ATOM,
        INCR,

        UTF8_STRING,
        UTF8_MIME_0: b"text/plain;charset=utf-8",
        UTF8_MIME_1: b"text/plain;charset=UTF-8",
        // Text in ISO Latin-1 encoding
        // See: https://tronche.com/gui/x/icccm/sec-2.html#s-2.6.2
        STRING,
        // Text in unknown encoding
        // See: https://tronche.com/gui/x/icccm/sec-2.html#s-2.6.2
        TEXT,
        TEXT_MIME_UNKNOWN: b"text/plain",

        // HTML: b"text/html",
        // URI_LIST: b"text/uri-list",

        PNG__MIME: ImageFormat::mime_type(ImageFormat::Png ).as_bytes(),
        JPEG_MIME: ImageFormat::mime_type(ImageFormat::Jpeg).as_bytes(),
        WEBP_MIME: ImageFormat::mime_type(ImageFormat::Webp).as_bytes(),
        GIF__MIME: ImageFormat::mime_type(ImageFormat::Gif ).as_bytes(),
        SVG__MIME: ImageFormat::mime_type(ImageFormat::Svg ).as_bytes(),
        BMP__MIME: ImageFormat::mime_type(ImageFormat::Bmp ).as_bytes(),
        TIFF_MIME: ImageFormat::mime_type(ImageFormat::Tiff).as_bytes(),
        ICO__MIME: ImageFormat::mime_type(ImageFormat::Ico ).as_bytes(),

        // This is just some random name for the property on our window, into which
        // the clipboard owner writes the data we requested.
        ARBOARD_CLIPBOARD,
    }
}

thread_local! {
    static ATOM_NAME_CACHE: RefCell<HashMap<Atom, &'static str>> = Default::default();
}

// Some clipboard items, like images, may take a very long time to produce a
// `SelectionNotify`. Multiple seconds long.
pub(super) const LONG_TIMEOUT_DUR: Duration = Duration::from_millis(4000);
pub(super) const SHORT_TIMEOUT_DUR: Duration = Duration::from_millis(10);

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ManagerHandoverState {
    Idle,
    InProgress,
    Finished,
}

pub(super) struct GlobalClipboard {
    pub(super) inner: Arc<Inner>,

    /// Join handle to the thread which serves selection requests.
    pub(super) server_handle: JoinHandle<()>,
}

pub(super) struct XContext {
    pub(super) conn: RustConnection,
    pub(super) win_id: u32,
}

pub(super) struct Inner {
    /// The context for the thread which serves clipboard read
    /// requests coming to us.
    pub(super) server: XContext,
    pub(super) atoms: Atoms,

    pub(super) clipboard: Selection,
    pub(super) primary: Selection,
    pub(super) secondary: Selection,

    pub(super) handover_state: Mutex<ManagerHandoverState>,
    pub(super) handover_cv: Condvar,

    pub(super) serve_stopped: AtomicBool,
}

impl XContext {
    fn new() -> Result<Self> {
        // create a new connection to an X11 server
        let (conn, screen_num): (RustConnection, _) =
            RustConnection::connect(None).map_err(|_| {
                Error::unknown("X11 server connection timed out because it was unreachable")
            })?;
        let screen = conn
            .setup()
            .roots
            .get(screen_num)
            .ok_or(Error::unknown("no screen found"))?;
        let win_id = conn.generate_id().map_err(into_unknown)?;

        let event_mask =
            // Just in case that some program reports SelectionNotify events
            // with XCB_EVENT_MASK_PROPERTY_CHANGE mask.
            EventMask::PROPERTY_CHANGE |
            // To receive DestroyNotify event and stop the message loop.
            EventMask::STRUCTURE_NOTIFY;
        // create the window
        conn.create_window(
            // copy as much as possible from the parent, because no other specific input is needed
            COPY_DEPTH_FROM_PARENT,
            win_id,
            screen.root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::COPY_FROM_PARENT,
            COPY_FROM_PARENT,
            // don't subscribe to any special events because we are requesting everything we need ourselves
            &CreateWindowAux::new().event_mask(event_mask),
        )
        .map_err(into_unknown)?;
        conn.flush().map_err(into_unknown)?;

        Ok(Self { conn, win_id })
    }
}

#[derive(Default)]
pub(super) struct Selection {
    pub(super) data: RwLock<Option<Vec<ClipboardData>>>,
    /// Mutex around nothing to use with the below condvar.
    pub(super) mutex: Mutex<()>,
    /// A condvar that is notified when the contents of this clipboard are changed.
    ///
    /// This is associated with `Self::mutex`.
    pub(super) data_changed: Condvar,
}

#[derive(Debug, Clone)]
pub(super) struct ClipboardData {
    pub(super) bytes: Vec<u8>,

    /// The atom representing the format in which the data is encoded.
    pub(super) format: Atom,
}

pub(super) enum ReadSelNotifyResult {
    GotData(ClipboardData),
    IncrStarted,
    EventNotRecognized,
}

impl Inner {
    fn new() -> Result<Self> {
        let server = XContext::new()?;
        let atoms = Atoms::new(&server.conn)
            .map_err(into_unknown)?
            .reply()
            .map_err(into_unknown)?;

        Ok(Self {
            server,
            atoms,
            clipboard: Selection::default(),
            primary: Selection::default(),
            secondary: Selection::default(),
            handover_state: Mutex::new(ManagerHandoverState::Idle),
            handover_cv: Condvar::new(),
            serve_stopped: AtomicBool::new(false),
        })
    }

    fn write(
        &self,
        data: Vec<ClipboardData>,
        selection: ClipboardKind,
        wait: WaitConfig,
    ) -> Result<()> {
        if self.serve_stopped.load(Ordering::Relaxed) {
            return Err(Error::unknown(
                "The clipboard handler thread seems to have stopped. Logging messages may reveal the cause. (See the `log` crate.)",
            ));
        }

        let server_win = self.server.win_id;

        // ICCCM version 2, section 2.6.1.3 states that we should re-assert ownership whenever data
        // changes.
        self.server
            .conn
            .set_selection_owner(server_win, self.atom_of(selection), Time::CURRENT_TIME)
            .map_err(|_| Error::ClipboardOccupied)?;

        self.server.conn.flush().map_err(into_unknown)?;

        // Just setting the data, and the `serve_requests` will take care of the rest.
        let selection = self.selection_of(selection);
        let mut data_guard = selection.data.write();
        *data_guard = Some(data);

        // Lock the mutex to both ensure that no wakers of `data_changed` can wake us between
        // dropping the `data_guard` and calling `wait[_for]` and that we don't we wake other
        // threads in that position.
        let mut guard = selection.mutex.lock();

        // Notify any existing waiting threads that we have changed the data in the selection.
        // It is important that the mutex is locked to prevent this notification getting lost.
        selection.data_changed.notify_all();

        match wait {
            WaitConfig::None => {}
            WaitConfig::Forever => {
                drop(data_guard);
                selection.data_changed.wait(&mut guard);
            }
            WaitConfig::Until(deadline) => {
                drop(data_guard);
                selection.data_changed.wait_until(&mut guard, deadline);
            }
        }

        Ok(())
    }

    /// `formats` must be a slice of atoms, where each atom represents a target format.
    /// The first format from `formats`, which the clipboard owner supports will be the
    /// format of the return value.
    fn read(&self, formats: &[Atom], selection: ClipboardKind) -> Result<ClipboardData> {
        // if we are the current owner, we can get the current clipboard ourselves
        if self.is_owner(selection)? {
            let data = self.selection_of(selection).data.read();
            if let Some(data_list) = &*data {
                for data in data_list {
                    for format in formats {
                        if *format == data.format {
                            return Ok(data.clone());
                        }
                    }
                }
            }
            return Err(Error::ContentNotAvailable);
        }
        let reader = XContext::new()?;

        let highest_precedence_format =
            match self.read_single(&reader, selection, self.atoms.TARGETS) {
                Err(err) => {
                    log::trace!("Clipboard TARGETS query failed with {err:?}");
                    None
                }
                Ok(ClipboardData { bytes, format }) => {
                    if format == self.atoms.ATOM {
                        let available_formats = Self::parse_formats(&bytes);
                        formats
                            .iter()
                            .find(|format| available_formats.contains(format))
                    } else {
                        log::trace!(
                            "Unexpected clipboard TARGETS format {}",
                            self.atom_name(format)
                        );
                        None
                    }
                }
            };

        if let Some(&format) = highest_precedence_format {
            let data = self.read_single(&reader, selection, format)?;
            if !formats.contains(&data.format) {
                // This shouldn't happen since the format is from the TARGETS list.
                log::trace!(
                    "Conversion to {} responded with {} which is not supported",
                    self.atom_name(format),
                    self.atom_name(data.format),
                );
                return Err(Error::ConversionFailure);
            }
            return Ok(data);
        }

        log::trace!("Falling back on attempting to convert clipboard to each format.");
        for format in formats {
            match self.read_single(&reader, selection, *format) {
                Ok(data) => {
                    if formats.contains(&data.format) {
                        return Ok(data);
                    } else {
                        log::trace!(
                            "Conversion to {} responded with {} which is not supported",
                            self.atom_name(*format),
                            self.atom_name(data.format),
                        );
                        continue;
                    }
                }
                Err(Error::ContentNotAvailable) => {
                    continue;
                }
                Err(e) => {
                    log::trace!("Conversion to {} failed: {}", self.atom_name(*format), e);
                    return Err(e);
                }
            }
        }
        log::trace!("All conversions to supported formats failed.");
        Err(Error::ContentNotAvailable)
    }

    fn parse_formats(bytes: &[u8]) -> Vec<Atom> {
        bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    }

    fn read_single(
        &self,
        reader: &XContext,
        selection: ClipboardKind,
        target_format: Atom,
    ) -> Result<ClipboardData> {
        // Delete the property so that we can detect (using property notify)
        // when the selection owner receives our request.
        reader
            .conn
            .delete_property(reader.win_id, self.atoms.ARBOARD_CLIPBOARD)
            .map_err(into_unknown)?;

        // request to convert the clipboard selection to our data type(s)
        reader
            .conn
            .convert_selection(
                reader.win_id,
                self.atom_of(selection),
                target_format,
                self.atoms.ARBOARD_CLIPBOARD,
                Time::CURRENT_TIME,
            )
            .map_err(into_unknown)?;
        reader.conn.sync().map_err(into_unknown)?;

        log::trace!("Finished `convert_selection`");

        let mut incr_data: Vec<u8> = Vec::new();
        let mut using_incr = false;

        let mut timeout_end = Instant::now() + LONG_TIMEOUT_DUR;

        while Instant::now() < timeout_end {
            let event = reader.conn.poll_for_event().map_err(into_unknown)?;
            let event = match event {
                Some(e) => e,
                None => {
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                }
            };
            match event {
                // The first response after requesting a selection.
                Event::SelectionNotify(event) => {
                    log::trace!("Read SelectionNotify");
                    let result = self.handle_read_selection_notify(
                        reader,
                        target_format,
                        &mut using_incr,
                        &mut incr_data,
                        event,
                    )?;
                    match result {
                        ReadSelNotifyResult::GotData(data) => return Ok(data),
                        ReadSelNotifyResult::IncrStarted => {
                            // This means we received an indication that an the
                            // data is going to be sent INCRementally. Let's
                            // reset our timeout.
                            timeout_end += SHORT_TIMEOUT_DUR;
                        }
                        ReadSelNotifyResult::EventNotRecognized => (),
                    }
                }
                // If the previous SelectionNotify event specified that the data
                // will be sent in INCR segments, each segment is transferred in
                // a PropertyNotify event.
                Event::PropertyNotify(event) => {
                    let result = self.handle_read_property_notify(
                        reader,
                        target_format,
                        using_incr,
                        &mut incr_data,
                        &mut timeout_end,
                        event,
                    )?;
                    if result {
                        return Ok(ClipboardData {
                            bytes: incr_data,
                            format: target_format,
                        });
                    }
                }
                _ => log::trace!(
                    "An unexpected event arrived while reading the clipboard: {:?}",
                    event
                ),
            }
        }
        log::info!("Time-out hit while reading the clipboard.");
        Err(Error::ContentNotAvailable)
    }

