use std::io::Write;
use std::sync::{Arc, Mutex};

use alacritty_terminal::event::Event as AlacrittyEvent;
use alacritty_terminal::event::EventListener;

use crate::osc_parser::ShellMarker;

/// Events emitted by the terminal to the UI layer.
pub enum TerminalEvent {
    /// Terminal content changed, UI should repaint.
    Wakeup,
    /// Terminal title changed (via OSC escape sequence).
    Title(String),
    /// BEL character received.
    Bell,
    /// Shell process exited.
    Exit,
    /// Shell integration marker detected (OSC 133).
    ShellMarker(ShellMarker),
}

/// Bridges alacritty_terminal events to our TerminalEvent channel.
///
/// Also handles PtyWrite events by writing directly to the shared PTY writer.
pub struct RaijinEventListener {
    sender: flume::Sender<TerminalEvent>,
    pty_writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl RaijinEventListener {
    pub fn new(
        sender: flume::Sender<TerminalEvent>,
        pty_writer: Arc<Mutex<Box<dyn Write + Send>>>,
    ) -> Self {
        Self { sender, pty_writer }
    }
}

impl EventListener for RaijinEventListener {
    fn send_event(&self, event: AlacrittyEvent) {
        match event {
            AlacrittyEvent::Wakeup => {
                let _ = self.sender.send(TerminalEvent::Wakeup);
            }
            AlacrittyEvent::Title(title) => {
                let _ = self.sender.send(TerminalEvent::Title(title));
            }
            AlacrittyEvent::Bell => {
                let _ = self.sender.send(TerminalEvent::Bell);
            }
            AlacrittyEvent::Exit => {
                let _ = self.sender.send(TerminalEvent::Exit);
            }
            AlacrittyEvent::PtyWrite(text) => {
                if let Ok(mut writer) = self.pty_writer.lock() {
                    let _ = writer.write_all(text.as_bytes());
                }
            }
            _ => {}
        }
    }
}
