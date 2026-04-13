use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::collections::HashMap;

use raijin_shell::ShellContext;

use crate::command::{self, CommandOutput};

/// Default CWD scan timeout — 30ms.
/// Protects against slow filesystems (NFS, SSHFS).
pub const DEFAULT_SCAN_TIMEOUT: Duration = Duration::from_millis(30);

// ---------------------------------------------------------------------------
// DirContents — threaded CWD scan with timeout
// ---------------------------------------------------------------------------

/// Cached CWD directory contents for chip detection.
///
/// Scanned once per CWD change via a worker thread with timeout.
/// Stores filenames, folder names, and file extensions separately
/// for fast O(1) lookups during provider detection.
///
#[derive(Debug, Clone)]
pub struct DirContents {
    /// Filenames (without path prefix).
    pub files: HashSet<String>,
    /// Directory names.
    pub folders: HashSet<String>,
    /// File extensions (without leading dot). Includes compound extensions
    /// like `tar.gz` in addition to `gz`.
    pub extensions: HashSet<String>,
}

impl DirContents {
    /// Create empty DirContents.
    pub fn empty() -> Self {
        Self {
            files: HashSet::new(),
            folders: HashSet::new(),
            extensions: HashSet::new(),
        }
    }

    /// Scan a directory with timeout protection.
    ///
    /// Spawns a worker thread that reads directory entries and sends them
    /// via mpsc channel. The main thread collects entries until timeout.
    /// Returns partial results if the scan exceeds the timeout.
    pub fn scan(path: &Path, timeout: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        let path = path.to_path_buf();

        std::thread::spawn(move || {
            let entries = match std::fs::read_dir(&path) {
                Ok(entries) => entries,
                Err(_) => return,
            };

            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry
                    .file_type()
                    .map(|ft| ft.is_dir())
                    .unwrap_or(false);

                if tx.send((file_name, is_dir)).is_err() {
                    break; // receiver dropped
                }
            }
        });

        let mut files = HashSet::new();
        let mut folders = HashSet::new();
        let mut extensions = HashSet::new();
        let start = Instant::now();

        loop {
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                log::trace!("DirContents scan timed out after {:?}", timeout);
                break;
            }

            match rx.recv_timeout(remaining) {
                Ok((name, is_dir)) => {
                    if is_dir {
                        folders.insert(name);
                    } else {
                        // Extract extensions: foo.tar.gz → ["tar.gz", "gz"]
                        if !name.starts_with('.') {
                            if let Some(dot_pos) = name.find('.') {
                                let ext = &name[dot_pos + 1..];
                                if !ext.is_empty() {
                                    extensions.insert(ext.to_string());
                                    // Also add the last extension for compound types
                                    if let Some(last_dot) = ext.rfind('.') {
                                        let last_ext = &ext[last_dot + 1..];
                                        if !last_ext.is_empty() {
                                            extensions.insert(last_ext.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        files.insert(name);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    log::trace!("DirContents scan timed out after {:?}", timeout);
                    break;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break; // scan complete
                }
            }
        }

        Self {
            files,
            folders,
            extensions,
        }
    }

    /// Check if a file with this name exists in the scanned directory.
    pub fn has_file(&self, name: &str) -> bool {
        self.files.contains(name)
    }

    /// Check if a folder with this name exists in the scanned directory.
    pub fn has_folder(&self, name: &str) -> bool {
        self.folders.contains(name)
    }

    /// Check if any file with this extension exists.
    pub fn has_extension(&self, ext: &str) -> bool {
        self.extensions.contains(ext)
    }

    /// 3-level detection check.
    ///
    /// Returns `true` if ANY of the specified files, folders, or extensions
    /// are found in the scanned directory.
    pub fn matches(
        &self,
        detect_files: &[&str],
        detect_folders: &[&str],
        detect_extensions: &[&str],
    ) -> bool {
        detect_files.iter().any(|f| self.has_file(f))
            || detect_folders.iter().any(|f| self.has_folder(f))
            || detect_extensions.iter().any(|e| self.has_extension(e))
    }
}

// ---------------------------------------------------------------------------
// DetectionCache — re-scans only on CWD change
// ---------------------------------------------------------------------------

/// Caches DirContents and re-scans only when CWD changes.
pub struct DetectionCache {
    cached_cwd: Option<PathBuf>,
    contents: DirContents,
}

impl DetectionCache {
    pub fn new() -> Self {
        Self {
            cached_cwd: None,
            contents: DirContents::empty(),
        }
    }

    /// Returns DirContents for the given CWD, re-scanning if CWD changed.
    pub fn get_or_scan(&mut self, cwd: &Path, timeout: Duration) -> &DirContents {
        if self.cached_cwd.as_deref() != Some(cwd) {
            self.contents = DirContents::scan(cwd, timeout);
            self.cached_cwd = Some(cwd.to_path_buf());
        }
        &self.contents
    }

    /// Get a reference to the current contents (without re-scanning).
    pub fn contents_ref(&self) -> &DirContents {
        &self.contents
    }

    /// Force re-scan on next access (e.g., after a command completes).
    pub fn invalidate(&mut self) {
        self.cached_cwd = None;
    }
}

// ---------------------------------------------------------------------------
// ChipContext — data available to all chip providers
// ---------------------------------------------------------------------------

/// Aggregated context passed to chip providers during gathering.
///
/// Constructed once per render cycle by the terminal pane. Providers
/// use this to check availability and gather their output without
/// performing their own I/O.
pub struct ChipContext {
    /// Shell context (cwd, username, hostname, git info).
    pub shell_context: ShellContext,
    /// Current shell name (e.g., "zsh", "bash", "nu").
    pub shell_name: String,
    /// Current working directory as PathBuf.
    pub cwd: PathBuf,
    /// Formatted current time string (HH:MM).
    pub time_str: String,
    /// Pre-scanned directory contents (threaded, timeout-protected).
    pub dir_contents: DirContents,
    /// Snapshot of relevant environment variables.
    pub env: HashMap<String, String>,
    /// Last command exit code (from OSC 7777 metadata).
    pub last_exit_code: Option<i32>,
    /// Last command duration in milliseconds.
    pub last_duration_ms: Option<u64>,
    /// Command execution timeout (Default: 500ms).
    pub command_timeout: Duration,
    /// Battery info provider (cross-platform battery status).
    pub battery_info_provider: std::sync::Arc<dyn crate::providers::battery::BatteryInfoProvider + Send + Sync>,
}

impl ChipContext {
    /// Execute a command with the configured timeout.
    pub fn exec_cmd(&self, cmd: &str, args: &[&str]) -> Option<CommandOutput> {
        command::exec_cmd(cmd, args, self.command_timeout)
    }

    /// Check if an environment variable is set.
    pub fn has_env(&self, key: &str) -> bool {
        self.env.contains_key(key)
    }

    /// Get an environment variable value.
    pub fn get_env(&self, key: &str) -> Option<String> {
        self.env.get(key).cloned()
    }
}

/// Collect relevant environment variables for chip detection.
///
/// Only captures env vars that chip providers actually check,
/// avoiding a full env snapshot.
pub fn collect_chip_env_vars() -> HashMap<String, String> {
    let keys = [
        // DevOps & Cloud
        "KUBECONFIG",
        "DOCKER_CONTEXT",
        "DOCKER_HOST",
        "DOCKER_MACHINE_NAME",
        "DOCKER_CONFIG",
        "AWS_VAULT",
        "AWSU_PROFILE",
        "AWSUME_PROFILE",
        "AWS_SSO_PROFILE",
        "AWS_PROFILE",
        "AWS_REGION",
        "AWS_DEFAULT_REGION",
        "AWS_CONFIG_FILE",
        "CLOUDSDK_CONFIG",
        "CLOUDSDK_ACTIVE_CONFIG_NAME",
        "OS_CLOUD",
        "TF_WORKSPACE",
        "TF_DATA_DIR",
        "AZURE_CONFIG_DIR",
        // Environment Managers
        "CONDA_DEFAULT_ENV",
        "IN_NIX_SHELL",
        "GUIX_ENVIRONMENT",
        "DIRENV_DIR",
        "DIRENV_FILE",
        "container",
        "name",
        "SINGULARITY_NAME",
        "SINGULARITY_CONTAINER",
        "APPTAINER_NAME",
        "APPTAINER_CONTAINER",
        "NIX_SHELL_PACKAGES",
        "SPACK_ENV",
        // Haskell
        "GHC_VERSION",
        // Rust Toolchain
        "RUSTUP_TOOLCHAIN",
        "RUSTUP_HOME",
        // System
        "SHLVL",
        "RAIJIN_JOBS_COUNT",
        "JOBS_COUNT",
        "VIRTUAL_ENV",
        "PYENV_VERSION",
        "JAVA_HOME",
        "RBENV_VERSION",
        "RUBY_VERSION",
        // Shell
        "SHELL",
    ];

    let mut env = HashMap::new();
    for key in keys {
        if let Ok(val) = std::env::var(key) {
            env.insert(key.to_string(), val);
        }
    }
    env
}
