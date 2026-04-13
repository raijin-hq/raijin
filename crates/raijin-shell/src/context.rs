use std::process::Command;

use crate::metadata::ShellMetadataPayload;

/// Git diff statistics (insertions, deletions, changed files).
#[derive(Clone)]
pub struct GitStats {
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
}

/// Shell context information gathered from the environment.
///
/// Provides CWD, git branch, hostname, username, and other metadata for display
/// in the terminal's context chips area. Updated dynamically via OSC 7777 metadata.
#[derive(Clone)]
pub struct ShellContext {
    pub cwd: String,
    pub cwd_short: String,
    pub hostname: String,
    pub username: String,
    pub git_branch: Option<String>,
    pub git_stats: Option<GitStats>,
}

impl ShellContext {
    /// Gather shell context for the given working directory.
    pub fn gather_for(cwd_path: &std::path::Path) -> Self {
        let cwd = cwd_path.to_string_lossy().to_string();

        let cwd_short = shorten_path(&cwd);
        let git_branch = detect_git_branch(cwd_path);
        let git_stats = if git_branch.is_some() {
            detect_git_stats(cwd_path)
        } else {
            None
        };
        let hostname = detect_hostname();
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "user".to_string());

        Self {
            cwd,
            cwd_short,
            hostname,
            username,
            git_branch,
            git_stats,
        }
    }

    /// Update context dynamically from shell metadata (OSC 7777).
    pub fn update_from_metadata(&mut self, payload: &ShellMetadataPayload) {
        self.cwd = payload.cwd.clone();
        self.cwd_short = shorten_path(&payload.cwd);
        if let Some(ref u) = payload.username {
            self.username = u.clone();
        }
        if let Some(ref h) = payload.hostname {
            self.hostname = h.clone();
        }
        self.git_branch = payload.git_branch.clone();
        self.git_stats = if payload.git_dirty == Some(true) {
            Some(GitStats {
                files_changed: 1,
                insertions: 0,
                deletions: 0,
            })
        } else {
            None
        };
    }
}

impl Default for ShellContext {
    fn default() -> Self {
        Self {
            cwd: "~".to_string(),
            cwd_short: "~".to_string(),
            hostname: "localhost".to_string(),
            username: "user".to_string(),
            git_branch: None,
            git_stats: None,
        }
    }
}

pub fn shorten_path(path: &str) -> String {
    if let Some(home) = std::env::var_os("HOME") {
        let home = home.to_string_lossy();
        if path.starts_with(home.as_ref()) {
            return format!("~{}", &path[home.len()..]);
        }
    }
    path.to_string()
}

fn detect_hostname() -> String {
    // Try gethostname first (no subprocess)
    if let Ok(name) = hostname::get() {
        return name.to_string_lossy().to_string();
    }
    "localhost".to_string()
}

fn detect_git_branch(cwd: &std::path::Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(cwd)
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

fn detect_git_stats(cwd: &std::path::Path) -> Option<GitStats> {
    let output = Command::new("git")
        .args(["diff", "--stat", "--shortstat", "HEAD"])
        .current_dir(cwd)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    // Parse: "3 files changed, 454 insertions(+), 110 deletions(-)"
    let mut files_changed = 0u32;
    let mut insertions = 0u32;
    let mut deletions = 0u32;

    for part in text.split(',') {
        let part = part.trim();
        if part.contains("file") {
            if let Some(n) = part.split_whitespace().next().and_then(|s| s.parse().ok()) {
                files_changed = n;
            }
        } else if part.contains("insertion") {
            if let Some(n) = part.split_whitespace().next().and_then(|s| s.parse().ok()) {
                insertions = n;
            }
        } else if part.contains("deletion") {
            if let Some(n) = part.split_whitespace().next().and_then(|s| s.parse().ok()) {
                deletions = n;
            }
        }
    }

    Some(GitStats {
        files_changed,
        insertions,
        deletions,
    })
}
