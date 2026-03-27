use std::io::Write;
use std::sync::Arc;
use std::thread;

use raijin_term::grid::Dimensions as GridDimensions;
use raijin_term::term::{Config, Term};
use raijin_term::vte::ansi;
use anyhow::Result;
use parking_lot::FairMutex;

use std::path::Path;

use crate::event::{RaijinEventListener, TerminalEvent};
use crate::osc_parser::OscScanner;
use crate::pty;

/// Default scrollback history, can be overridden via config.
const DEFAULT_SCROLLBACK_HISTORY: usize = 10_000;

/// Terminal dimensions for raijin-term.
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
            history: DEFAULT_SCROLLBACK_HISTORY,
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

/// A terminal emulator backed by raijin-term and a PTY.
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
    pub fn new(
        rows: u16,
        cols: u16,
        cwd: &Path,
        input_mode: pty::InputMode,
        scrollback_history: usize,
    ) -> Result<Self> {
        let (event_tx, event_rx) = flume::unbounded();

        let (master, reader, pty_writer) =
            pty::spawn_pty(rows, cols, input_mode, cwd)?;

        let shared_writer: Arc<std::sync::Mutex<Box<dyn Write + Send>>> =
            Arc::new(std::sync::Mutex::new(pty_writer));
        let listener = RaijinEventListener::new(event_tx.clone(), Arc::clone(&shared_writer));

        let config = Config {
            scrolling_history: scrollback_history,
            ..Config::default()
        };

        let dims = TermDimensions {
            cols: cols as usize,
            rows: rows as usize,
            history: scrollback_history,
        };

        let term = Term::new(config, &dims, listener);
        let term = Arc::new(FairMutex::new(term));

        Self::spawn_pty_reader(Arc::clone(&term), event_tx, reader);

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
    ///
    /// The thread scans incoming bytes for OSC 133 shell integration markers
    /// and emits ShellMarker events. The bytes are then passed unmodified to
    /// raijin-term's VTE parser for grid processing.
    fn spawn_pty_reader(
        term: Arc<FairMutex<Term<RaijinEventListener>>>,
        event_tx: flume::Sender<TerminalEvent>,
        mut reader: Box<dyn std::io::Read + Send>,
    ) {
        thread::Builder::new()
            .name("raijin-pty-reader".into())
            .spawn(move || {
                let mut buf = [0u8; 8192];
                let mut parser = ansi::Processor::<ansi::StdSyncHandler>::new();
                let mut osc_scanner = OscScanner::new();

                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let chunk = &buf[..n];

                            // Scan for OSC 133 shell integration markers
                            let markers = osc_scanner.scan(chunk);

                            // Route block grid BEFORE feeding bytes to parser
                            {
                                let mut term = term.lock();
                                for marker in &markers {
                                    match marker {
                                        crate::osc_parser::ShellMarker::PromptStart => {
                                            term.route_to_prompt();
                                        }
                                        crate::osc_parser::ShellMarker::CommandStart => {
                                            term.route_to_new_block(String::new());
                                        }
                                        crate::osc_parser::ShellMarker::CommandEnd { exit_code } => {
                                            term.route_finalize_block(*exit_code);
                                        }
                                        _ => {}
                                    }
                                }
                            }

                            // Send markers as events for UI processing
                            for marker in markers {
                                let _ = event_tx.send(TerminalEvent::ShellMarker(marker));
                            }

                            // Feed bytes to raijin-term
                            let mut term = term.lock();
                            parser.advance(&mut *term, chunk);
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
