use super::*;

fn serve_requests(context: Arc<Inner>) -> Result<(), Box<dyn std::error::Error>> {
    fn handover_finished(clip: &Arc<Inner>, mut handover_state: MutexGuard<ManagerHandoverState>) {
        log::trace!("Finishing clipboard manager handover.");
        *handover_state = ManagerHandoverState::Finished;

        // Not sure if unlocking the mutex is necessary here but better safe than sorry.
        drop(handover_state);

        clip.handover_cv.notify_all();
    }

    log::trace!("Started serve requests thread.");

    let _guard = inazuma_util::defer(|| {
        context.serve_stopped.store(true, Ordering::Relaxed);
    });

    let mut written = false;
    let mut notified = false;

    loop {
        match context.server.conn.wait_for_event().map_err(into_unknown)? {
            Event::DestroyNotify(_) => {
                // This window is being destroyed.
                log::trace!("Clipboard server window is being destroyed x_x");
                return Ok(());
            }
            Event::SelectionClear(event) => {
                // TODO: check if this works
                // Someone else has new content in the clipboard, so it is
                // notifying us that we should delete our data now.
                log::trace!("Somebody else owns the clipboard now");

                if let Some(selection) = context.kind_of(event.selection) {
                    let selection = context.selection_of(selection);
                    let mut data_guard = selection.data.write();
                    *data_guard = None;

                    // It is important that this mutex is locked at the time of calling
                    // `notify_all` to prevent notifications getting lost in case the sleeping
                    // thread has unlocked its `data_guard` and is just about to sleep.
                    // It is also important that the RwLock is kept write-locked for the same
                    // reason.
                    let _guard = selection.mutex.lock();
                    selection.data_changed.notify_all();
                }
            }
            Event::SelectionRequest(event) => {
                log::trace!(
                    "SelectionRequest - selection is: {}, target is {}",
                    context.atom_name(event.selection),
                    context.atom_name(event.target),
                );
                // Someone is requesting the clipboard content from us.
                context
                    .handle_selection_request(event)
                    .map_err(into_unknown)?;

                // if we are in the progress of saving to the clipboard manager
                // make sure we save that we have finished writing
                let handover_state = context.handover_state.lock();
                if *handover_state == ManagerHandoverState::InProgress {
                    // Only set written, when the actual contents were written,
                    // not just a response to what TARGETS we have.
                    if event.target != context.atoms.TARGETS {
                        log::trace!("The contents were written to the clipboard manager.");
                        written = true;
                        // if we have written and notified, make sure to notify that we are done
                        if notified {
                            handover_finished(&context, handover_state);
                        }
                    }
                }
            }
            Event::SelectionNotify(event) => {
                // We've requested the clipboard content and this is the answer.
                // Considering that this thread is not responsible for reading
                // clipboard contents, this must come from the clipboard manager
                // signaling that the data was handed over successfully.
                if event.selection != context.atoms.CLIPBOARD_MANAGER {
                    log::error!(
                        "Received a `SelectionNotify` from a selection other than the CLIPBOARD_MANAGER. This is unexpected in this thread."
                    );
                    continue;
                }
                let handover_state = context.handover_state.lock();
                if *handover_state == ManagerHandoverState::InProgress {
                    // Note that some clipboard managers send a selection notify
                    // before even sending a request for the actual contents.
                    // (That's why we use the "notified" & "written" flags)
                    log::trace!(
                        "The clipboard manager indicated that it's done requesting the contents from us."
                    );
                    notified = true;

                    // One would think that we could also finish if the property
                    // here is set 0, because that indicates failure. However
                    // this is not the case; for example on KDE plasma 5.18, we
                    // immediately get a SelectionNotify with property set to 0,
                    // but following that, we also get a valid SelectionRequest
                    // from the clipboard manager.
                    if written {
                        handover_finished(&context, handover_state);
                    }
                }
            }
            _event => {
                // May be useful for debugging but nothing else really.
                //log::trace!("Received unwanted event: {:?}", event);
            }
        }
    }
}

pub(crate) struct Clipboard {
    inner: Arc<Inner>,
}

impl Clipboard {
    pub(crate) fn new() -> Result<Self> {
        let mut global_cb = CLIPBOARD.lock();
        if let Some(global_cb) = &*global_cb {
            return Ok(Self {
                inner: Arc::clone(&global_cb.inner),
            });
        }
        // At this point we know that the clipboard does not exist.
        let ctx = Arc::new(Inner::new()?);
        let join_handle = std::thread::Builder::new()
            .name("Clipboard".to_owned())
            .spawn({
                let ctx = Arc::clone(&ctx);
                move || {
                    if let Err(error) = serve_requests(ctx) {
                        log::error!("Worker thread errored with: {}", error);
                    }
                }
            })
            .unwrap();
        *global_cb = Some(GlobalClipboard {
            inner: Arc::clone(&ctx),
            server_handle: join_handle,
        });
        Ok(Self { inner: ctx })
    }

    pub(crate) fn set_text(
        &self,
        message: Cow<'_, str>,
        selection: ClipboardKind,
        wait: WaitConfig,
    ) -> Result<()> {
        let data = vec![ClipboardData {
            bytes: message.into_owned().into_bytes(),
            format: self.inner.atoms.UTF8_STRING,
        }];
        self.inner.write(data, selection, wait)
    }

    #[allow(unused)]
    pub(crate) fn set_image(
        &self,
        image: Image,
        selection: ClipboardKind,
        wait: WaitConfig,
    ) -> Result<()> {
        let format = match image.format {
            ImageFormat::Png => self.inner.atoms.PNG__MIME,
            ImageFormat::Jpeg => self.inner.atoms.JPEG_MIME,
            ImageFormat::Webp => self.inner.atoms.WEBP_MIME,
            ImageFormat::Gif => self.inner.atoms.GIF__MIME,
            ImageFormat::Svg => self.inner.atoms.SVG__MIME,
            ImageFormat::Bmp => self.inner.atoms.BMP__MIME,
            ImageFormat::Tiff => self.inner.atoms.TIFF_MIME,
            ImageFormat::Ico => self.inner.atoms.ICO__MIME,
        };
        let data = vec![ClipboardData {
            bytes: image.bytes,
            format: self.inner.atoms.PNG__MIME,
        }];
        self.inner.write(data, selection, wait)
    }

    pub(crate) fn get_any(&self, selection: ClipboardKind) -> Result<ClipboardItem> {
        const IMAGE_FORMAT_COUNT: usize = 7;
        let image_format_atoms: [Atom; IMAGE_FORMAT_COUNT] = [
            self.inner.atoms.PNG__MIME,
            self.inner.atoms.JPEG_MIME,
            self.inner.atoms.WEBP_MIME,
            self.inner.atoms.GIF__MIME,
            self.inner.atoms.SVG__MIME,
            self.inner.atoms.BMP__MIME,
            self.inner.atoms.TIFF_MIME,
        ];
        let image_formats: [ImageFormat; IMAGE_FORMAT_COUNT] = [
            ImageFormat::Png,
            ImageFormat::Jpeg,
            ImageFormat::Webp,
            ImageFormat::Gif,
            ImageFormat::Svg,
            ImageFormat::Bmp,
            ImageFormat::Tiff,
        ];

        const TEXT_FORMAT_COUNT: usize = 6;
        let text_format_atoms: [Atom; TEXT_FORMAT_COUNT] = [
            self.inner.atoms.UTF8_STRING,
            self.inner.atoms.UTF8_MIME_0,
            self.inner.atoms.UTF8_MIME_1,
            self.inner.atoms.STRING,
            self.inner.atoms.TEXT,
            self.inner.atoms.TEXT_MIME_UNKNOWN,
        ];

        let atom_none: Atom = AtomEnum::NONE.into();

        const FORMAT_ATOM_COUNT: usize = TEXT_FORMAT_COUNT + IMAGE_FORMAT_COUNT;

        let mut format_atoms: [Atom; FORMAT_ATOM_COUNT] = [atom_none; FORMAT_ATOM_COUNT];

        // image formats first, as they are more specific, and read will return the first
        // format that the contents can be converted to
        format_atoms[0..IMAGE_FORMAT_COUNT].copy_from_slice(&image_format_atoms);
        format_atoms[IMAGE_FORMAT_COUNT..].copy_from_slice(&text_format_atoms);
        debug_assert!(!format_atoms.contains(&atom_none));

        let result = self.inner.read(&format_atoms, selection)?;

        log::trace!(
            "read clipboard as format {:?}",
            self.inner.atom_name(result.format)
        );

        for (format_atom, image_format) in image_format_atoms.into_iter().zip(image_formats) {
            if result.format == format_atom {
                let bytes = result.bytes;
                let id = hash(&bytes);
                return Ok(ClipboardItem::new_image(&Image {
                    id,
                    format: image_format,
                    bytes,
                }));
            }
        }

        let text = if result.format == self.inner.atoms.STRING {
            // ISO Latin-1
            // See: https://stackoverflow.com/questions/28169745/what-are-the-options-to-convert-iso-8859-1-latin-1-to-a-string-utf-8
            result.bytes.into_iter().map(|c| c as char).collect()
        } else {
            String::from_utf8(result.bytes).map_err(|_| Error::ConversionFailure)?
        };
        Ok(ClipboardItem::new_string(text))
    }

    pub fn is_owner(&self, selection: ClipboardKind) -> bool {
        self.inner.is_owner(selection).unwrap_or(false)
    }
}

impl Drop for Clipboard {
    fn drop(&mut self) {
        // There are always at least 3 owners:
        // the global, the server thread, and one `Clipboard::inner`
        const MIN_OWNERS: usize = 3;

        // We start with locking the global guard to prevent race
        // conditions below.
        let mut global_cb = CLIPBOARD.lock();
        if Arc::strong_count(&self.inner) == MIN_OWNERS {
            // If the are the only owners of the clipboard are ourselves and
            // the global object, then we should destroy the global object,
            // and send the data to the clipboard manager

            if let Err(e) = self.inner.ask_clipboard_manager_to_request_our_data() {
                log::error!(
                    "Could not hand the clipboard data over to the clipboard manager: {}",
                    e
                );
            }
            let global_cb = global_cb.take();
            if let Err(e) = self
                .inner
                .server
                .conn
                .destroy_window(self.inner.server.win_id)
            {
                log::error!("Failed to destroy the clipboard window. Error: {}", e);
                return;
            }
            if let Err(e) = self.inner.server.conn.flush() {
                log::error!("Failed to flush the clipboard window. Error: {}", e);
                return;
            }
            if let Some(global_cb) = global_cb
                && let Err(e) = global_cb.server_handle.join()
            {
                // Let's try extracting the error message
                let message;
                if let Some(msg) = e.downcast_ref::<&'static str>() {
                    message = Some((*msg).to_string());
                } else if let Some(msg) = e.downcast_ref::<String>() {
                    message = Some(msg.clone());
                } else {
                    message = None;
                }
                if let Some(message) = message {
                    log::error!(
                        "The clipboard server thread panicked. Panic message: '{}'",
                        message,
                    );
                } else {
                    log::error!("The clipboard server thread panicked.");
                }
            }
        }
    }
}

fn into_unknown<E: std::fmt::Display>(error: E) -> Error {
    Error::Unknown {
        description: error.to_string(),
    }
}

/// Clipboard selection
///
/// Linux has a concept of clipboard "selections" which tend to be used in different contexts. This
/// enum provides a way to get/set to a specific clipboard
///
/// See <https://specifications.freedesktop.org/clipboards-spec/clipboards-0.1.txt> for a better
/// description of the different clipboards.
#[derive(Copy, Clone, Debug)]
pub enum ClipboardKind {
    /// Typically used selection for explicit cut/copy/paste actions (ie. windows/macos like
    /// clipboard behavior)
    Clipboard,

    /// Typically used for mouse selections and/or currently selected text. Accessible via middle
    /// mouse click.
    Primary,

    /// The secondary clipboard is rarely used but theoretically available on X11.
    Secondary,
}

/// Configuration on how long to wait for a new X11 copy event is emitted.
#[derive(Default)]
pub(crate) enum WaitConfig {
    /// Waits until the given [`Instant`] has reached.
    #[allow(
        unused,
        reason = "Right now we don't wait for clipboard contents to sync on app close, but we may in the future"
    )]
    Until(Instant),

    /// Waits forever until a new event is reached.
    #[allow(unused)]
    #[allow(
        unused,
        reason = "Right now we don't wait for clipboard contents to sync on app close, but we may in the future"
    )]
    Forever,

    /// It shouldn't wait.
    #[default]
    None,
}

#[non_exhaustive]
pub enum Error {
    /// The clipboard contents were not available in the requested format.
    /// This could either be due to the clipboard being empty or the clipboard contents having
    /// an incompatible format to the requested one (eg when calling `get_image` on text)
    ContentNotAvailable,

    /// The native clipboard is not accessible due to being held by an other party.
    ///
    /// This "other party" could be a different process or it could be within
    /// the same program. So for example you may get this error when trying
    /// to interact with the clipboard from multiple threads at once.
    ///
    /// Note that it's OK to have multiple `Clipboard` instances. The underlying
    /// implementation will make sure that the native clipboard is only
    /// opened for transferring data and then closed as soon as possible.
    ClipboardOccupied,

    /// The image or the text that was about the be transferred to/from the clipboard could not be
    /// converted to the appropriate format.
    ConversionFailure,

    /// Any error that doesn't fit the other error types.
    ///
    /// The `description` field is only meant to help the developer and should not be relied on as a
    /// means to identify an error case during runtime.
    Unknown { description: String },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
			Error::ContentNotAvailable => f.write_str("The clipboard contents were not available in the requested format or the clipboard is empty."),
			Error::ClipboardOccupied => f.write_str("The native clipboard is not accessible due to being held by an other party."),
			Error::ConversionFailure => f.write_str("The image or the text that was about the be transferred to/from the clipboard could not be converted to the appropriate format."),
			Error::Unknown { description } => f.write_fmt(format_args!("Unknown error while interacting with the clipboard: {description}")),
		}
    }
}

impl std::error::Error for Error {}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        macro_rules! kind_to_str {
			($( $e: pat ),*) => {
				match self {
					$(
						$e => stringify!($e),
					)*
				}
			}
		}
        let name = kind_to_str!(
            ContentNotAvailable,
            ClipboardOccupied,
            ConversionFailure,
            Unknown { .. }
        );
        f.write_fmt(format_args!("{name} - \"{self}\""))
    }
}

impl Error {
    pub(crate) fn unknown<M: Into<String>>(message: M) -> Self {
        Error::Unknown {
            description: message.into(),
        }
    }
}
