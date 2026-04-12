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
    pub(crate) task: Option<crate::task_state::TaskState>,
    /// User-visible breadcrumb text (e.g., agent terminal label).
    pub breadcrumb_text: String,
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
        Self::with_shell(rows, cols, cwd, input_mode, scrollback_history, None)
    }

    /// Create a new terminal with an explicit shell path.
    pub fn with_shell(
        rows: u16,
        cols: u16,
        cwd: &Path,
        input_mode: pty::InputMode,
        scrollback_history: usize,
        shell: Option<&str>,
    ) -> Result<Self> {
        let (event_tx, event_rx) = flume::unbounded();

        let (master, reader, pty_writer) =
            pty::spawn_pty(rows, cols, input_mode, cwd, shell)?;

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
            task: None,
            breadcrumb_text: String::new(),
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
                            // Scan for OSC markers and process them INLINE with
                            // the byte stream. Each segment between markers is fed
                            // to the VTE parser before the next marker is handled.
                            // This ensures command output lands in the correct block.
                            let markers = osc_scanner.scan(chunk);

                            {
                                let mut term = term.lock();
                                let mut offset = 0;

                                for pm in &markers {
                                    // Feed bytes BEFORE this marker to the parser
                                    if pm.start > offset {
                                        parser.advance(&mut *term, &chunk[offset..pm.start]);
                                    }

                                    // Process the marker (route grid)
                                    match &pm.marker {
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

                                    offset = pm.end;
                                }

                                // Feed remaining bytes after the last marker
                                if offset < chunk.len() {
                                    parser.advance(&mut *term, &chunk[offset..]);
                                }
                            }

                            // Send markers as events for UI processing
                            for pm in markers {
                                let _ = event_tx.send(TerminalEvent::ShellMarker(pm.marker));
                            }

                            // Notify UI that new content is available
                            let _ = event_tx.send(TerminalEvent::Wakeup);
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

    /// Returns the task state if this terminal is running a task.
    pub fn task(&self) -> Option<&crate::task_state::TaskState> {
        self.task.as_ref()
    }

    /// Clone the builder configuration from this terminal for creating a copy.
    pub fn clone_builder(
        &self,
        _cx: &inazuma::App,
        cwd: Option<std::path::PathBuf>,
    ) -> inazuma::Task<anyhow::Result<crate::TerminalBuilder>> {
        let cwd = cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        inazuma::Task::ready(Ok(crate::TerminalBuilder {
            working_directory: Some(cwd),
            task: None,
            shell: raijin_task::Shell::System,
            env: Default::default(),
        }))
    }

    /// Returns the total number of lines in the terminal grid (screen + scrollback history).
    pub fn total_lines(&self) -> usize {
        use raijin_term::grid::Dimensions;
        let term = self.handle.lock();
        term.total_lines()
    }

    /// Returns the terminal grid content as a string.
    ///
    /// Reads all visible lines from the grid and concatenates them.
    pub fn get_content(&self) -> String {
        use raijin_term::grid::Dimensions;
        let term = self.handle.lock();
        let top = term.topmost_line();
        let bottom = term.bottommost_line();
        let last_col = term.last_column();
        term.bounds_to_string(
            raijin_term::index::Point::new(top, raijin_term::index::Column(0)),
            raijin_term::index::Point::new(bottom, last_col),
        )
    }

    /// Write output data to the terminal display (as if received from PTY).
    ///
    /// This feeds bytes through the VTE parser into the terminal grid.
    pub fn write_output(&self, data: &[u8]) {
        use raijin_term::vte::ansi;
        let mut term = self.handle.lock();
        let mut parser = ansi::Processor::<ansi::StdSyncHandler>::new();
        parser.advance(&mut *term, data);
    }

    /// Returns a shared future that resolves when the terminal's shell task completes.
    ///
    /// Currently returns a pre-resolved future with `None` as the exit status.
    /// Terminal exit tracking in Raijin happens via `TerminalEvent::Exit` events.
    pub fn wait_for_completed_task(
        &self,
        _cx: &inazuma::App,
    ) -> futures::future::Shared<inazuma::Task<Option<std::process::ExitStatus>>> {
        use futures::FutureExt;
        inazuma::Task::ready(None).shared()
    }

    /// Kill the active task running in this terminal (sends SIGHUP to the PTY).
    pub fn kill_active_task(&self) {
        // Drop the PTY master to send SIGHUP to the child process.
        // The TerminalHandle holds an Arc to the master; we can resize to 0
        // which will cause the child to receive SIGWINCH. For a proper kill,
        // we rely on the PTY master being dropped when all references are gone.
        // For now, close the writer to signal EOF to the child.
        if let Ok(mut writer) = self.handle.pty_writer.lock() {
            // Write EOF (Ctrl-D) to signal the shell to exit
            let _ = std::io::Write::write_all(&mut *writer, &[0x04]);
        }
    }
}
