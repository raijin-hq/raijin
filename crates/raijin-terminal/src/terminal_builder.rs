use anyhow::Result;
use inazuma_collections::HashMap;
use inazuma_util::paths::PathStyle;
use raijin_task::Shell;
use smol::channel::Sender;
use std::{path::PathBuf, process::ExitStatus};

use crate::task_state::TaskState;
use crate::terminal_settings::{AlternateScroll, CursorShape};
use crate::Terminal;

/// Builder for creating terminals, either for shell sessions or task execution.
pub struct TerminalBuilder {
    pub working_directory: Option<PathBuf>,
    pub task: Option<TaskState>,
    pub shell: Shell,
    pub env: HashMap<String, String>,
}

impl TerminalBuilder {
    /// Create a new terminal builder with the full set of configuration options.
    ///
    /// Returns an async task that resolves to the builder once environment
    /// setup is complete.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        working_directory: Option<PathBuf>,
        task: Option<TaskState>,
        shell: Shell,
        env: HashMap<String, String>,
        _cursor_shape: CursorShape,
        _alternate_scroll: AlternateScroll,
        _max_scroll_history_lines: Option<usize>,
        _path_hyperlink_regexes: Vec<String>,
        _path_hyperlink_timeout_ms: u64,
        _is_remote_terminal: bool,
        _window_id: u64,
        _completion_tx: Option<Sender<Option<ExitStatus>>>,
        _cx: &inazuma::App,
        _activation_script: Vec<String>,
        _path_style: PathStyle,
    ) -> inazuma::Task<Result<TerminalBuilder>> {
        inazuma::Task::ready(Ok(TerminalBuilder {
            working_directory,
            task,
            shell,
            env,
        }))
    }

    /// Create a display-only terminal builder (no PTY, just a rendering buffer).
    ///
    /// Used for agent terminals where output is written programmatically.
    #[allow(clippy::too_many_arguments)]
    pub fn new_display_only(
        _cursor_shape: CursorShape,
        _alternate_scroll: AlternateScroll,
        _max_scroll_history_lines: Option<usize>,
        _window_id: u64,
        _executor: &inazuma::BackgroundExecutor,
        _path_style: PathStyle,
    ) -> Result<TerminalBuilder> {
        Ok(TerminalBuilder {
            working_directory: None,
            task: None,
            shell: Shell::System,
            env: Default::default(),
        })
    }

    /// Consume the builder and create a Terminal, subscribing to its events.
    pub fn subscribe(self, _cx: &inazuma::Context<Terminal>) -> Terminal {
        let cwd = self
            .working_directory
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let shell_path = match &self.shell {
            Shell::System => None,
            Shell::Program(p) => Some(p.as_str()),
            Shell::WithArguments { program, .. } => Some(program.as_str()),
        };

        let mut terminal = Terminal::with_shell(
            24,
            80,
            &cwd,
            crate::InputMode::Raijin,
            10_000,
            shell_path,
        )
        .expect("failed to create terminal");

        terminal.task = self.task;
        terminal
    }
}

/// Inserts environment variables identifying the terminal as running inside Raijin.
pub fn insert_raijin_terminal_env(
    env: &mut HashMap<String, String>,
    version: &impl std::fmt::Display,
) {
    env.insert("RAIJIN_TERM".to_string(), "true".to_string());
    env.insert("TERM_PROGRAM".to_string(), "raijin".to_string());
    env.insert("TERM".to_string(), "xterm-256color".to_string());
    env.insert("COLORTERM".to_string(), "truecolor".to_string());
    env.insert("TERM_PROGRAM_VERSION".to_string(), version.to_string());
}
