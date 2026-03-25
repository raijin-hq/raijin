use std::io::Write;
use std::sync::Arc;
use std::thread;

use alacritty_terminal::grid::Dimensions as GridDimensions;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi;
use anyhow::Result;
use parking_lot::FairMutex;

use crate::event::{RaijinEventListener, TerminalEvent};
use crate::pty;

const SCROLLBACK_HISTORY: usize = 10_000;

/// Terminal dimensions for alacritty_terminal.
struct TermDimensions {
    cols: usize,
    rows: usize,
    history: usize,
}

impl GridDimensions for TermDimensions {
    fn total_lines(&self) -> usize {
        self.rows + self.history
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

/// A cloneable, thread-safe handle for terminal operations.
///
/// This handle can be passed to UI elements (like TerminalElement) so they
/// can read grid state for rendering and trigger resizes when bounds change.
#[derive(Clone)]
pub struct TerminalHandle {
    term: Arc<FairMutex<Term<RaijinEventListener>>>,
    pty_master: Arc<dyn portable_pty::MasterPty + Send>,
    pty_writer: Arc<std::sync::Mutex<Box<dyn Write + Send>>>,
}

impl TerminalHandle {
    /// Lock the terminal grid for reading (rendering).
    pub fn lock(&self) -> parking_lot::FairMutexGuard<'_, Term<RaijinEventListener>> {
        self.term.lock()
    }

    /// Write bytes to the PTY (user keyboard input).
    pub fn write(&self, bytes: &[u8]) {
        if let Ok(mut writer) = self.pty_writer.lock() {
            let _ = writer.write_all(bytes);
        }
    }

    /// Resize the terminal grid and PTY to new dimensions.
    ///
    /// Compares with current dimensions and only resizes if changed.
    /// Safe to call every frame from prepaint — no-ops when size unchanged.
    pub fn set_size(&self, rows: u16, cols: u16) {
        if rows == 0 || cols == 0 {
            return;
        }

        let mut term = self.term.lock();
        let current_rows = term.screen_lines() as u16;
        let current_cols = term.columns() as u16;

        if rows == current_rows && cols == current_cols {
            return;
        }

        let dims = TermDimensions {
            cols: cols as usize,
            rows: rows as usize,
            history: SCROLLBACK_HISTORY,
        };

        term.resize(dims);
        drop(term);

        let _ = pty::resize_pty(self.pty_master.as_ref(), rows, cols);
    }

    /// Get the raw Arc for advanced usage.
    pub fn term_arc(&self) -> &Arc<FairMutex<Term<RaijinEventListener>>> {
        &self.term
    }
}

/// A terminal emulator backed by alacritty_terminal and a PTY.
///
/// The terminal runs a background thread that reads PTY output and feeds it
/// through the VTE parser into the terminal grid. The UI thread uses the
/// `TerminalHandle` to read grid state and trigger resizes.
pub struct Terminal {
    handle: TerminalHandle,
    events_rx: flume::Receiver<TerminalEvent>,
}

impl Terminal {
    /// Create a new terminal with the given grid dimensions.
    ///
    /// Spawns the user's default shell in a PTY and starts a background
    /// thread to process PTY output.
    pub fn new(rows: u16, cols: u16) -> Result<Self> {
        let (event_tx, event_rx) = flume::unbounded();

        let (master, reader, pty_writer) = pty::spawn_pty(rows, cols, true)?;

        let shared_writer: Arc<std::sync::Mutex<Box<dyn Write + Send>>> =
            Arc::new(std::sync::Mutex::new(pty_writer));
        let listener = RaijinEventListener::new(event_tx, Arc::clone(&shared_writer));

        let config = Config {
            scrolling_history: SCROLLBACK_HISTORY,
            ..Config::default()
        };

        let dims = TermDimensions {
            cols: cols as usize,
            rows: rows as usize,
            history: SCROLLBACK_HISTORY,
        };

        let term = Term::new(config, &dims, listener);
        let term = Arc::new(FairMutex::new(term));

        Self::spawn_pty_reader(Arc::clone(&term), reader);

        let handle = TerminalHandle {
            term,
            pty_master: Arc::from(master),
            pty_writer: shared_writer,
        };

        Ok(Self {
            handle,
            events_rx: event_rx,
        })
    }

    /// Spawn the background thread that reads PTY output and feeds it to the terminal.
    fn spawn_pty_reader(
        term: Arc<FairMutex<Term<RaijinEventListener>>>,
        mut reader: Box<dyn std::io::Read + Send>,
    ) {
        thread::Builder::new()
            .name("raijin-pty-reader".into())
            .spawn(move || {
                let mut buf = [0u8; 8192];
                let mut parser = ansi::Processor::<ansi::StdSyncHandler>::new();

                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let mut term = term.lock();
                            parser.advance(&mut *term, &buf[..n]);
                        }
                        Err(e) => {
                            log::error!("PTY read error: {}", e);
                            break;
                        }
                    }
                }
            })
            .expect("failed to spawn PTY reader thread");
    }

    /// Get a cloneable handle for terminal operations.
    ///
    /// Pass this to UI elements for rendering and resize.
    pub fn handle(&self) -> TerminalHandle {
        self.handle.clone()
    }

    /// Write bytes to the PTY (user keyboard input).
    pub fn write(&self, bytes: &[u8]) {
        self.handle.write(bytes);
    }

    /// Get the event receiver for async polling.
    pub fn event_receiver(&self) -> &flume::Receiver<TerminalEvent> {
        &self.events_rx
    }
}
