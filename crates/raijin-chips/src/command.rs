use std::ffi::OsStr;
use std::process::Command;
use std::time::Duration;

use process_control::{ChildExt, Control};

/// Default command timeout — 500ms.
pub const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_millis(500);

/// Output from a successfully executed command.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
}

/// Execute a command with timeout protection.
///
/// Returns `None` on:
/// - Binary not found in PATH
/// - Spawn failure
/// - Timeout exceeded (process is killed)
/// - Non-zero exit code
///
/// Uses the `process_control` crate for timeout-safe process management,
/// 
pub fn exec_cmd(cmd: &str, args: &[&str], timeout: Duration) -> Option<CommandOutput> {
    let binary = which::which(cmd).ok()?;

    let mut command = Command::new(binary);
    command
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let child = command.spawn().ok()?;

    let output = child
        .controlled_with_output()
        .time_limit(timeout)
        .terminate_for_timeout()
        .wait()
        .ok()??;

    if !output.status.success() {
        return None;
    }

    Some(CommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

/// Execute a command with OsStr arguments and timeout.
pub fn exec_cmd_os<T, U>(cmd: T, args: &[U], timeout: Duration) -> Option<CommandOutput>
where
    T: AsRef<OsStr>,
    U: AsRef<OsStr>,
{
    let binary = which::which(cmd.as_ref()).ok()?;

    let mut command = Command::new(binary);
    command
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let child = command.spawn().ok()?;

    let output = child
        .controlled_with_output()
        .time_limit(timeout)
        .terminate_for_timeout()
        .wait()
        .ok()??;

    if !output.status.success() {
        return None;
    }

    Some(CommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}
