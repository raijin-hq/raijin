use std::io::{Read, Write};

use anyhow::Result;
use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};

/// Spawn a PTY with the user's default shell.
///
/// Returns the master PTY handle, a reader for output, and a writer for input.
pub fn spawn_pty(
    rows: u16,
    cols: u16,
    suppress_prompt: bool,
) -> Result<(
    Box<dyn MasterPty + Send>,
    Box<dyn Read + Send>,
    Box<dyn Write + Send>,
)> {
    let pty_system = NativePtySystem::default();

    let size = PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = pty_system.openpty(size)?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());

    let mut cmd = CommandBuilder::new(&shell);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "raijin");

    // TODO: Shell integration hooks (precmd/preexec) for Raijin Mode
    // This will enable: prompt suppression, block boundaries, exit codes
    // For now, the shell runs with its normal prompt (PS1 mode)
    let _ = suppress_prompt;

    pair.slave.spawn_command(cmd)?;

    let reader = pair.master.try_clone_reader()?;
    let writer = pair.master.take_writer()?;

    Ok((pair.master, reader, writer))
}

/// Resize the PTY to new dimensions.
pub fn resize_pty(master: &dyn MasterPty, rows: u16, cols: u16) -> Result<()> {
    master.resize(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;
    Ok(())
}
