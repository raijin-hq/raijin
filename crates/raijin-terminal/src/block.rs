use std::time::Instant;

use crate::osc_parser::ShellMarker;

/// A terminal block represents one command execution cycle:
/// the command the user typed, its output, and metadata.
#[derive(Debug, Clone)]
pub struct TerminalBlock {
    pub id: u64,
    /// The command text that was executed.
    pub command: String,
    /// Exit code (None if still running).
    pub exit_code: Option<i32>,
    /// First row in the terminal grid where this block's output starts.
    pub start_row: usize,
    /// Last row in the terminal grid (None if still active/running).
    pub end_row: Option<usize>,
    /// When the command started executing.
    pub started_at: Instant,
    /// When the command finished (None if still running).
    pub finished_at: Option<Instant>,
    /// Raw JSON metadata snapshot from the shell at block creation time.
    pub metadata_json: Option<String>,
}

impl TerminalBlock {
    /// Check if this block has finished executing.
    pub fn is_finished(&self) -> bool {
        self.exit_code.is_some()
    }

    /// Get the duration of the command execution.
    pub fn duration(&self) -> std::time::Duration {
        match self.finished_at {
            Some(finished) => finished.duration_since(self.started_at),
            None => Instant::now().duration_since(self.started_at),
        }
    }

    /// Format the duration for display (e.g., "0.3s", "2.1s", "1m 30s").
    pub fn duration_display(&self) -> String {
        let dur = self.duration();
        let secs = dur.as_secs_f64();
        if secs < 60.0 {
            format!("{:.1}s", secs)
        } else {
            let mins = secs as u64 / 60;
            let remaining = secs as u64 % 60;
            format!("{}m {}s", mins, remaining)
        }
    }
}

/// Tracks shell integration markers and builds terminal blocks.
///
/// The BlockManager listens to OSC 133 shell markers and uses them
/// to split the terminal output into logical blocks. Each block
/// corresponds to one command + its output.
pub struct BlockManager {
    blocks: Vec<TerminalBlock>,
    next_id: u64,
    /// The grid row where the current prompt started.
    prompt_start_row: Option<usize>,
    /// Whether we're currently in a command execution phase.
    in_command: bool,
    /// The last command text entered by the user.
    pending_command: Option<String>,
    /// Latest raw JSON metadata from shell precmd (updated on each Metadata marker).
    latest_metadata_json: Option<String>,
}

impl BlockManager {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            next_id: 0,
            prompt_start_row: None,
            in_command: false,
            pending_command: None,
            latest_metadata_json: None,
        }
    }

    /// Process a shell marker from the OSC 133 parser.
    ///
    /// `cursor_row` is the current cursor position in the terminal grid,
    /// used to track which rows belong to which block.
    pub fn process_marker(&mut self, marker: ShellMarker, cursor_row: usize) {
        match marker {
            ShellMarker::PromptStart => {
                // Close the previous block if one was running
                if self.in_command {
                    if let Some(block) = self.blocks.last_mut() {
                        if block.end_row.is_none() {
                            block.end_row = Some(cursor_row.saturating_sub(1));
                        }
                    }
                    self.in_command = false;
                }
                self.prompt_start_row = Some(cursor_row);
            }

            ShellMarker::InputStart => {
                // Prompt ended, input region starts — nothing special to track
            }

            ShellMarker::CommandStart => {
                // User hit Enter, command is being executed
                let id = self.next_id;
                self.next_id += 1;

                let command = self.pending_command.take().unwrap_or_default();

                self.blocks.push(TerminalBlock {
                    id,
                    command,
                    exit_code: None,
                    start_row: cursor_row,
                    end_row: None,
                    started_at: Instant::now(),
                    finished_at: None,
                    metadata_json: self.latest_metadata_json.clone(),
                });

                self.in_command = true;
            }

            ShellMarker::CommandEnd { exit_code } => {
                if let Some(block) = self.blocks.last_mut() {
                    if block.exit_code.is_none() {
                        block.exit_code = Some(exit_code);
                        block.end_row = Some(cursor_row);
                        block.finished_at = Some(Instant::now());
                    }
                }
                self.in_command = false;
            }

            ShellMarker::Metadata(json) => {
                self.latest_metadata_json = Some(json);
            }
        }
    }

    /// Set the command text for the next block (called when user presses Enter in input bar).
    pub fn set_pending_command(&mut self, command: String) {
        self.pending_command = Some(command);
    }

    /// Get all blocks.
    pub fn blocks(&self) -> &[TerminalBlock] {
        &self.blocks
    }

    /// Get only finished blocks (have exit code).
    pub fn finished_blocks(&self) -> impl Iterator<Item = &TerminalBlock> {
        self.blocks.iter().filter(|b| b.is_finished())
    }

    /// Get the currently active block (if any command is running).
    pub fn active_block(&self) -> Option<&TerminalBlock> {
        self.blocks.last().filter(|b| !b.is_finished())
    }

    /// Check if there are any blocks at all.
    pub fn has_blocks(&self) -> bool {
        !self.blocks.is_empty()
    }

    /// Get the grid row where the initial prompt started (before any commands).
    /// Used to hide the initial prompt in Raijin mode.
    pub fn prompt_start_row(&self) -> Option<usize> {
        self.prompt_start_row
    }
}
