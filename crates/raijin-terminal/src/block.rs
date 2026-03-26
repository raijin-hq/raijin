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
    /// Command duration measured by the shell (milliseconds).
    /// More accurate than Instant-based timing since the shell measures
    /// the actual wall-clock time between preexec and precmd.
    pub shell_duration_ms: Option<u64>,
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

    /// Format the duration for display like Warp (e.g., "0.032s", "2.100s", "1m 30s").
    /// Prefers the shell-measured duration over Rust Instant-based timing.
    pub fn duration_display(&self) -> String {
        let secs = if let Some(ms) = self.shell_duration_ms {
            ms as f64 / 1000.0
        } else {
            self.duration().as_secs_f64()
        };
        if secs < 60.0 {
            format!("{:.3}s", secs)
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
    /// The grid row where the current prompt started (pending close).
    prompt_start_row: Option<usize>,
    /// Whether we're currently in a command execution phase.
    in_command: bool,
    /// The last command text entered by the user.
    pending_command: Option<String>,
    /// Latest raw JSON metadata from shell precmd (updated on each Metadata marker).
    latest_metadata_json: Option<String>,
    /// Prompt regions to hide: (start_row, end_row) inclusive.
    /// Each region spans from PromptStart to CommandStart — the rows where
    /// the shell prompt (Starship, P10k, etc.) renders. Like Warp, we simply
    /// don't render these rows, making prompt suppression shell-agnostic.
    hidden_prompt_regions: Vec<(usize, usize)>,
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
            hidden_prompt_regions: Vec::new(),
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
                // Close the prompt region: rows from PromptStart up to here are prompt
                // text (Starship, P10k, etc.) that we hide — like Warp's prompt grid.
                if let Some(prompt_row) = self.prompt_start_row.take() {
                    if cursor_row > prompt_row {
                        self.hidden_prompt_regions
                            .push((prompt_row, cursor_row.saturating_sub(1)));
                    }
                }

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
                    shell_duration_ms: None,
                });

                self.in_command = true;
            }

            ShellMarker::CommandEnd { exit_code } => {
                if let Some(block) = self.blocks.last_mut() {
                    if block.exit_code.is_none() {
                        block.exit_code = Some(exit_code);
                        // end_row is the last row of actual output, not the cursor
                        // position (which is one line AFTER the output). For commands
                        // with no output, clamp to start_row.
                        block.end_row = Some(
                            cursor_row.saturating_sub(1).max(block.start_row),
                        );
                        block.finished_at = Some(Instant::now());
                    }
                }
                self.in_command = false;
            }

            ShellMarker::PromptKind { .. } => {
                // Nushell-specific prompt kind — used for multi-line detection
            }

            ShellMarker::Metadata(json) => {
                self.latest_metadata_json = Some(json);
            }
        }
    }

    /// Set the shell-measured command duration on the last finished block.
    /// Called when metadata arrives with `last_duration_ms`.
    pub fn set_last_block_duration(&mut self, duration_ms: u64) {
        if let Some(block) = self.blocks.last_mut() {
            if block.is_finished() {
                block.shell_duration_ms = Some(duration_ms);
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

    /// Get the command text of the most recently finished block.
    pub fn last_command(&self) -> Option<String> {
        self.blocks
            .iter()
            .rev()
            .find(|b| b.is_finished())
            .map(|b| b.command.clone())
    }

    /// Get the currently active block (if any command is running).
    pub fn active_block(&self) -> Option<&TerminalBlock> {
        self.blocks.last().filter(|b| !b.is_finished())
    }

    /// Check if there are any blocks at all.
    pub fn has_blocks(&self) -> bool {
        !self.blocks.is_empty()
    }

    /// Get the grid row where the current (pending) prompt started.
    /// If set, all rows from this row onward are prompt text that should be hidden.
    pub fn prompt_start_row(&self) -> Option<usize> {
        self.prompt_start_row
    }

    /// Get all closed prompt regions to hide.
    /// Each region is (start_row, end_row) inclusive — these are rows where
    /// the shell prompt rendered (Starship, P10k, etc.) that Raijin replaces
    /// with its own context chips, like Warp does.
    pub fn hidden_prompt_regions(&self) -> &[(usize, usize)] {
        &self.hidden_prompt_regions
    }
}
