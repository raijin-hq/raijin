use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};

/// The input mode determines whether the shell's prompt is suppressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    /// Raijin Mode: Shell prompt suppressed, replaced by context chips.
    Raijin,
    /// PS1 Mode: Shell prompt visible (Starship, P10k, etc.).
    ShellPs1,
}

/// Spawn a PTY with the user's default shell and Raijin shell integration hooks.
///
/// The hooks inject OSC 133 markers (precmd/preexec) for block boundary detection.
/// In Raijin mode, the shell's prompt is also suppressed.
/// `cwd` sets the initial working directory for the shell.
pub fn spawn_pty(
    rows: u16,
    cols: u16,
    mode: InputMode,
    cwd: &Path,
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
    let shell_name = shell.rsplit('/').next().unwrap_or("zsh");

    let mut cmd = CommandBuilder::new(&shell);
    cmd.cwd(cwd);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "raijin");

    // Set input mode env var for shell hooks
    match mode {
        InputMode::Raijin => cmd.env("RAIJIN_MODE", "raijin"),
        InputMode::ShellPs1 => cmd.env("RAIJIN_MODE", "ps1"),
    };

    // Inject shell integration hooks via shell-specific mechanisms
    let hooks_dir = find_shell_hooks_dir();
    inject_shell_hooks(&mut cmd, shell_name, &hooks_dir, mode);

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

/// Find the directory containing our shell hook scripts.
///
/// Looks for `shell/raijin.zsh` relative to the executable, then falls back
/// to the source directory for development builds.
fn find_shell_hooks_dir() -> PathBuf {
    // Development: relative to crate root
    let dev_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("shell"))
        .unwrap_or_default();

    if dev_dir.join("raijin.zsh").exists() {
        return dev_dir;
    }

    // Installed: next to executable
    if let Ok(exe) = std::env::current_exe() {
        let installed_dir = exe.parent().unwrap_or(&exe).join("shell");
        if installed_dir.join("raijin.zsh").exists() {
            return installed_dir;
        }
    }

    dev_dir
}

/// Inject shell integration hooks into the command environment.
///
/// For zsh: Uses ZDOTDIR to inject a .zshenv that sources our hooks
/// before the user's own shell configuration.
///
/// For bash: Uses --rcfile to source our hooks alongside ~/.bashrc.
///
/// For fish: Uses --init-command to source our hooks.
fn inject_shell_hooks(
    cmd: &mut CommandBuilder,
    shell_name: &str,
    hooks_dir: &PathBuf,
    mode: InputMode,
) {
    match shell_name {
        "zsh" => {
            let hook_script = hooks_dir.join("raijin.zsh");
            if !hook_script.exists() {
                log::warn!("Zsh hook script not found at {:?}", hook_script);
                return;
            }

            // Create temp ZDOTDIR with .zshenv that loads our hooks first
            let raijin_zdotdir = std::env::temp_dir().join("raijin-zsh");
            if std::fs::create_dir_all(&raijin_zdotdir).is_err() {
                log::error!("Failed to create ZDOTDIR at {:?}", raijin_zdotdir);
                return;
            }

            // Preserve original ZDOTDIR (defaults to HOME)
            let original_zdotdir = std::env::var("ZDOTDIR")
                .unwrap_or_else(|_| {
                    std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
                });

            // Write .zshenv that sources our hooks then restores ZDOTDIR
            let zshenv_content = format!(
                r#"# Raijin Shell Integration Loader
# Restore original ZDOTDIR so user's .zshrc/.zprofile etc. load normally
export ZDOTDIR="{original_zdotdir}"

# Source Raijin hooks (OSC 133 markers)
source "{hook_script}"

# Source user's .zshenv if it exists
[[ -f "$ZDOTDIR/.zshenv" ]] && source "$ZDOTDIR/.zshenv"
"#,
                original_zdotdir = original_zdotdir,
                hook_script = hook_script.to_string_lossy(),
            );

            let zshenv_path = raijin_zdotdir.join(".zshenv");
            if let Err(e) = std::fs::write(&zshenv_path, zshenv_content) {
                log::error!("Failed to write .zshenv: {}", e);
                return;
            }

            // Redirect zsh to our ZDOTDIR
            cmd.env("_RAIJIN_ORIG_ZDOTDIR", &original_zdotdir);
            cmd.env("ZDOTDIR", raijin_zdotdir.to_string_lossy().as_ref());
        }

        "bash" => {
            let hook_script = hooks_dir.join("raijin.bash");
            if hook_script.exists() {
                cmd.args(["--rcfile", hook_script.to_string_lossy().as_ref()]);
            }
        }

        "fish" => {
            let hook_script = hooks_dir.join("raijin.fish");
            if hook_script.exists() {
                cmd.args([
                    "--init-command",
                    &format!("source {}", hook_script.to_string_lossy()),
                ]);
            }
        }

        "nu" => {
            // Nushell emits OSC 133 markers natively (reedline) — no marker injection needed.
            // We only inject our OSC 7777 metadata hooks via XDG_DATA_DIRS autoload.
            let nu_hooks_dir = hooks_dir.join("nushell");
            if nu_hooks_dir.join("vendor/autoload/raijin.nu").exists() {
                let existing_xdg = std::env::var("XDG_DATA_DIRS").unwrap_or_default();
                let raijin_xdg = if existing_xdg.is_empty() {
                    nu_hooks_dir.to_string_lossy().to_string()
                } else {
                    format!("{}:{}", nu_hooks_dir.to_string_lossy(), existing_xdg)
                };
                cmd.env("XDG_DATA_DIRS", &raijin_xdg);
            }
            cmd.env("RAIJIN_SHELL_FEATURES", "metadata,sudo");
        }

        other => {
            log::warn!("No shell integration hooks for shell: {}", other);
        }
    }

    let _ = mode; // Mode is set via RAIJIN_MODE env var above
}
