/// Command history management for terminal input.
///
/// Loads history from shell-specific histfiles on startup (zsh, bash, fish, nushell),
/// tracks commands during the session, and provides frecency-scored search
/// for ghost-text suggestions and history panel filtering.
use std::collections::HashMap;
use std::io::BufRead;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

/// A single history entry with metadata.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub command: String,
    pub timestamp: u64,
    pub frequency: u32,
    pub exit_status: Option<i32>,
    pub cwd: Option<String>,
}

/// Shell-specific history file format.
#[derive(Debug, Clone, Copy)]
pub enum HistfileFormat {
    /// zsh: `: timestamp:duration;command`
    Zsh,
    /// bash: one command per line (optionally with `#timestamp` lines)
    Bash,
    /// fish: YAML-like `- cmd: command\n  when: timestamp`
    Fish,
    /// nushell: plaintext, one command per line
    NuPlaintext,
}

/// Command history store with frecency search.
///
/// Navigation (Up/Down browsing) is handled by `HistoryPanel` which copies
/// entries and manages its own selection state.
pub struct CommandHistory {
    entries: Vec<HistoryEntry>,
    dedup_index: HashMap<String, usize>,
}

impl CommandHistory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            dedup_index: HashMap::new(),
        }
    }

    /// Auto-detect and load history for the given shell.
    pub fn detect_and_load(shell_name: &str) -> Self {
        let result = match shell_name {
            "zsh" => {
                let path = dirs::home_dir().map(|h| h.join(".zsh_history"));
                path.and_then(|p| Self::load_from_histfile(&p, HistfileFormat::Zsh).ok())
            }
            "bash" => {
                let path = dirs::home_dir().map(|h| h.join(".bash_history"));
                path.and_then(|p| Self::load_from_histfile(&p, HistfileFormat::Bash).ok())
            }
            "fish" => {
                let path = dirs::data_local_dir()
                    .or_else(dirs::config_dir)
                    .map(|c| c.join("fish/fish_history"));
                path.and_then(|p| Self::load_from_histfile(&p, HistfileFormat::Fish).ok())
            }
            "nu" => {
                // Try SQLite first, then plaintext
                let sqlite_path =
                    dirs::config_dir().map(|c| c.join("nushell/history.sqlite3"));
                if let Some(ref p) = sqlite_path {
                    if p.exists() {
                        return Self::load_nu_sqlite(p).unwrap_or_else(|_| Self::new());
                    }
                }
                let txt_path = dirs::config_dir().map(|c| c.join("nushell/history.txt"));
                txt_path.and_then(|p| {
                    Self::load_from_histfile(&p, HistfileFormat::NuPlaintext).ok()
                })
            }
            _ => None,
        };
        result.unwrap_or_else(Self::new)
    }

    /// Load history from a text-based histfile.
    pub fn load_from_histfile(path: &Path, format: HistfileFormat) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let content = std::fs::read(path)?;
        let mut history = Self::new();

        match format {
            HistfileFormat::Zsh => history.parse_zsh(&content),
            HistfileFormat::Bash => history.parse_bash(&content),
            HistfileFormat::Fish => history.parse_fish(&content),
            HistfileFormat::NuPlaintext => history.parse_plaintext(&content),
        }

        Ok(history)
    }

    /// Load nushell SQLite history.
    #[cfg(feature = "nushell-history")]
    fn load_nu_sqlite(path: &Path) -> Result<Self> {
        use rusqlite::Connection;
        let conn = Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let mut stmt = conn.prepare(
            "SELECT command_line, start_timestamp, exit_status, cwd
             FROM history ORDER BY start_timestamp ASC LIMIT 10000",
        )?;
        let mut history = Self::new();
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1).unwrap_or(0),
                row.get::<_, Option<i32>>(2).unwrap_or(None),
                row.get::<_, Option<String>>(3).unwrap_or(None),
            ))
        })?;
        for row in rows {
            if let Ok((cmd, ts, exit_status, cwd)) = row {
                let cmd = cmd.trim().to_string();
                if cmd.is_empty() {
                    continue;
                }
                history.insert_entry(HistoryEntry {
                    command: cmd,
                    timestamp: ts as u64,
                    frequency: 1,
                    exit_status,
                    cwd,
                });
            }
        }
        Ok(history)
    }

    #[cfg(not(feature = "nushell-history"))]
    fn load_nu_sqlite(_path: &Path) -> Result<Self> {
        Ok(Self::new())
    }

    // --- Parsers ---

    fn parse_zsh(&mut self, content: &[u8]) {
        // zsh extended history format: `: timestamp:duration;command`
        // Multi-line commands: lines ending with `\` are continuations.
        // Lines without `: ` prefix after a `\`-terminated line are continuations.
        let text = String::from_utf8_lossy(content);
        let mut pending_cmd: Option<String> = None;
        let mut pending_ts: u64 = 0;

        for line in text.lines() {
            if line.starts_with(": ") {
                // Flush previous pending multi-line command
                if let Some(cmd) = pending_cmd.take() {
                    if !cmd.is_empty() {
                        self.insert_entry(HistoryEntry {
                            command: cmd,
                            timestamp: pending_ts,
                            frequency: 1,
                            exit_status: None,
                            cwd: None,
                        });
                    }
                }

                // Parse new entry: `: timestamp:duration;command`
                if let Some(semi_pos) = line.find(';') {
                    let meta = &line[2..semi_pos];
                    let command_part = &line[semi_pos + 1..];
                    pending_ts = meta
                        .split(':')
                        .next()
                        .and_then(|s| s.trim().parse::<u64>().ok())
                        .unwrap_or(0);

                    if let Some(stripped) = command_part.strip_suffix('\\') {
                        // Multi-line command starts — strip trailing backslash
                        pending_cmd = Some(stripped.to_string());
                    } else if !command_part.is_empty() {
                        self.insert_entry(HistoryEntry {
                            command: command_part.to_string(),
                            timestamp: pending_ts,
                            frequency: 1,
                            exit_status: None,
                            cwd: None,
                        });
                    }
                }
            } else if let Some(ref mut cmd) = pending_cmd {
                // Continuation line of a multi-line command
                cmd.push('\n');
                if let Some(stripped) = line.strip_suffix('\\') {
                    cmd.push_str(stripped);
                } else {
                    cmd.push_str(line);
                    // Multi-line command complete
                    let finished = std::mem::take(cmd);
                    if !finished.is_empty() {
                        self.insert_entry(HistoryEntry {
                            command: finished,
                            timestamp: pending_ts,
                            frequency: 1,
                            exit_status: None,
                            cwd: None,
                        });
                    }
                    pending_cmd = None;
                }
            }
            // Lines without `: ` prefix and no pending continuation are ignored
        }

        // Flush last pending command
        if let Some(cmd) = pending_cmd {
            if !cmd.is_empty() {
                self.insert_entry(HistoryEntry {
                    command: cmd,
                    timestamp: pending_ts,
                    frequency: 1,
                    exit_status: None,
                    cwd: None,
                });
            }
        }
    }

    fn parse_bash(&mut self, content: &[u8]) {
        let reader = std::io::BufReader::new(content);
        let mut pending_timestamp: Option<u64> = None;
        for line in reader.lines().map_while(Result::ok) {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }
            if let Some(ts_str) = line.strip_prefix('#') {
                // Timestamp line: #1234567890
                pending_timestamp = ts_str.trim().parse::<u64>().ok();
                continue;
            }
            self.insert_entry(HistoryEntry {
                command: line,
                timestamp: pending_timestamp.take().unwrap_or(0),
                frequency: 1,
                exit_status: None,
                cwd: None,
            });
        }
    }

    fn parse_fish(&mut self, content: &[u8]) {
        // fish format:
        // - cmd: some command
        //   when: 1234567890
        //   paths:
        //     - /some/path
        let text = String::from_utf8_lossy(content);
        let mut current_cmd: Option<String> = None;
        let mut current_ts: u64 = 0;

        for line in text.lines() {
            if let Some(cmd_str) = line.strip_prefix("- cmd: ") {
                // Save previous entry
                if let Some(cmd) = current_cmd.take() {
                    if !cmd.is_empty() {
                        self.insert_entry(HistoryEntry {
                            command: cmd,
                            timestamp: current_ts,
                            frequency: 1,
                            exit_status: None,
                            cwd: None,
                        });
                    }
                }
                current_cmd = Some(cmd_str.to_string());
                current_ts = 0;
            } else if let Some(when_str) = line.trim_start().strip_prefix("when: ") {
                current_ts = when_str.trim().parse().unwrap_or(0);
            }
        }
        // Don't forget the last entry
        if let Some(cmd) = current_cmd {
            if !cmd.is_empty() {
                self.insert_entry(HistoryEntry {
                    command: cmd,
                    timestamp: current_ts,
                    frequency: 1,
                    exit_status: None,
                    cwd: None,
                });
            }
        }
    }

    fn parse_plaintext(&mut self, content: &[u8]) {
        let reader = std::io::BufReader::new(content);
        for line in reader.lines().map_while(Result::ok) {
            let line = line.trim().to_string();
            if !line.is_empty() {
                self.insert_entry(HistoryEntry {
                    command: line,
                    timestamp: 0,
                    frequency: 1,
                    exit_status: None,
                    cwd: None,
                });
            }
        }
    }

    fn insert_entry(&mut self, entry: HistoryEntry) {
        if let Some(&idx) = self.dedup_index.get(&entry.command) {
            // Update existing entry with newer timestamp and increment frequency
            let existing = &mut self.entries[idx];
            if entry.timestamp > existing.timestamp {
                existing.timestamp = entry.timestamp;
            }
            existing.frequency += 1;
            if entry.exit_status.is_some() {
                existing.exit_status = entry.exit_status;
            }
            if entry.cwd.is_some() {
                existing.cwd = entry.cwd;
            }
        } else {
            let idx = self.entries.len();
            self.dedup_index.insert(entry.command.clone(), idx);
            self.entries.push(entry);
        }
    }

    // --- Public API ---

    /// All history entries, oldest first.
    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    /// Total number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Push a new command to history.
    pub fn push(&mut self, command: String) {
        if command.trim().is_empty() {
            return;
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.insert_entry(HistoryEntry {
            command,
            timestamp: now,
            frequency: 1,
            exit_status: None,
            cwd: None,
        });
    }

    /// Frecency-scored prefix search. Returns entries sorted by score (highest first).
    pub fn frecency_search(&self, prefix: &str, limit: usize) -> Vec<&HistoryEntry> {
        if prefix.is_empty() {
            return Vec::new();
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut matches: Vec<(&HistoryEntry, f64)> = self
            .entries
            .iter()
            .filter(|e| e.command.starts_with(prefix) && e.command != prefix)
            .map(|e| {
                let age_secs = now.saturating_sub(e.timestamp);
                let recency = match age_secs {
                    0..=3600 => 4.0,
                    3601..=86400 => 2.0,
                    86401..=604800 => 1.0,
                    604801..=2592000 => 0.5,
                    _ => 0.25,
                };
                (e, e.frequency as f64 * recency)
            })
            .collect();

        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        matches.into_iter().take(limit).map(|(e, _)| e).collect()
    }

    /// Fuzzy filter for history panel. Returns matching entries, newest first.
    pub fn fuzzy_filter(&self, query: &str) -> Vec<&HistoryEntry> {
        if query.is_empty() {
            return self.entries.iter().rev().collect();
        }
        let query_lower = query.to_lowercase();
        let mut matches: Vec<&HistoryEntry> = self
            .entries
            .iter()
            .filter(|e| e.command.to_lowercase().contains(&query_lower))
            .collect();
        matches.reverse(); // newest first
        matches
    }
}

/// Format a timestamp as relative time: "just now", "5m ago", "3h ago", "2d ago".
pub fn relative_time(timestamp: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let age = now.saturating_sub(timestamp);

    if age < 60 {
        "just now".to_string()
    } else if age < 3600 {
        format!("{}m ago", age / 60)
    } else if age < 86400 {
        format!("{}h ago", age / 3600)
    } else if age < 604800 {
        format!("{}d ago", age / 86400)
    } else if age < 2592000 {
        format!("{}w ago", age / 604800)
    } else {
        format!("{}mo ago", age / 2592000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_entries() {
        let mut history = CommandHistory::new();
        history.push("ls".into());
        history.push("cd ..".into());
        history.push("git status".into());

        assert_eq!(history.len(), 3);
        assert_eq!(history.entries()[0].command, "ls");
        assert_eq!(history.entries()[1].command, "cd ..");
        assert_eq!(history.entries()[2].command, "git status");
    }

    #[test]
    fn test_deduplication() {
        let mut history = CommandHistory::new();
        history.push("ls".into());
        history.push("cd".into());
        history.push("ls".into());

        assert_eq!(history.len(), 2); // "ls" deduplicated
        assert_eq!(history.entries[0].frequency, 2); // "ls" frequency bumped
    }

    #[test]
    fn test_frecency_search() {
        let mut history = CommandHistory::new();
        history.push("cargo build".into());
        history.push("cargo test".into());
        history.push("cargo run".into());
        history.push("cargo build".into()); // Repeated, higher frequency

        let results = history.frecency_search("cargo ", 10);
        // "cargo build" should rank higher due to frequency=2
        assert!(!results.is_empty());
        assert_eq!(results[0].command, "cargo build");
    }

    #[test]
    fn test_fuzzy_filter() {
        let mut history = CommandHistory::new();
        history.push("cargo build".into());
        history.push("git status".into());
        history.push("cargo test".into());

        let results = history.fuzzy_filter("cargo");
        assert_eq!(results.len(), 2);
        // Newest first
        assert_eq!(results[0].command, "cargo test");
        assert_eq!(results[1].command, "cargo build");
    }

    #[test]
    fn test_relative_time() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        assert_eq!(relative_time(now), "just now");
        assert_eq!(relative_time(now - 300), "5m ago");
        assert_eq!(relative_time(now - 7200), "2h ago");
        assert_eq!(relative_time(now - 172800), "2d ago");
    }

    #[test]
    fn test_parse_zsh_history() {
        let content = b": 1234567890:0;ls -la\n: 1234567891:0;cd ..\n";
        let mut history = CommandHistory::new();
        history.parse_zsh(content);
        assert_eq!(history.len(), 2);
        assert_eq!(history.entries[0].command, "ls -la");
        assert_eq!(history.entries[0].timestamp, 1234567890);
        assert_eq!(history.entries[1].command, "cd ..");
    }

    #[test]
    fn test_parse_bash_history() {
        let content = b"#1234567890\nls -la\ncd ..\n";
        let mut history = CommandHistory::new();
        history.parse_bash(content);
        assert_eq!(history.len(), 2);
        assert_eq!(history.entries[0].command, "ls -la");
        assert_eq!(history.entries[0].timestamp, 1234567890);
    }

    #[test]
    fn test_parse_fish_history() {
        let content = b"- cmd: ls -la\n  when: 1234567890\n- cmd: cd ..\n  when: 1234567891\n";
        let mut history = CommandHistory::new();
        history.parse_fish(content);
        assert_eq!(history.len(), 2);
        assert_eq!(history.entries[0].command, "ls -la");
        assert_eq!(history.entries[0].timestamp, 1234567890);
    }

    #[test]
    fn test_empty_commands_ignored() {
        let mut history = CommandHistory::new();
        history.push("".into());
        history.push("   ".into());
        assert_eq!(history.len(), 0);
    }
}
