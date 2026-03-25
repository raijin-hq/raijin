use std::path::PathBuf;
use std::process::Command;

/// Shell context information gathered from the environment.
///
/// Provides CWD, git branch, and other metadata for display
/// in the terminal's context chips area.
pub struct ShellContext {
    pub cwd: String,
    pub cwd_short: String,
    pub git_branch: Option<String>,
}

impl ShellContext {
    /// Gather shell context from the current environment.
    ///
    /// This reads the current working directory and runs git commands
    /// to determine the active branch. Should be called on a background
    /// thread to avoid blocking the UI.
    pub fn gather() -> Self {
        let cwd = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("~"))
            .to_string_lossy()
            .to_string();

        let cwd_short = shorten_path(&cwd);
        let git_branch = detect_git_branch();

        Self {
            cwd,
            cwd_short,
            git_branch,
        }
    }
}

impl Default for ShellContext {
    fn default() -> Self {
        Self {
            cwd: "~".to_string(),
            cwd_short: "~".to_string(),
            git_branch: None,
        }
    }
}

fn shorten_path(path: &str) -> String {
    if let Some(home) = std::env::var_os("HOME") {
        let home = home.to_string_lossy();
        if path.starts_with(home.as_ref()) {
            return format!("~{}", &path[home.len()..]);
        }
    }
    path.to_string()
}

fn detect_git_branch() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        None
    } else {
        Some(branch)
    }
}
