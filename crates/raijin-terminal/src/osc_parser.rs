/// Shell integration markers from OSC 133 sequences (FTCS standard).
///
/// These markers are sent by the shell hooks (raijin.zsh/bash/fish) to
/// indicate command block boundaries. The terminal uses them to separate
/// output into visual blocks with metadata (command text, exit code, duration).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellMarker {
    /// OSC 133;A — Prompt region starts. A new potential block begins.
    PromptStart,
    /// OSC 133;B — Input region starts (prompt ended, user can type).
    InputStart,
    /// OSC 133;C — Command execution starts, output region begins.
    CommandStart,
    /// OSC 133;D;N — Command finished with the given exit code.
    CommandEnd { exit_code: i32 },
}

/// Scans a byte stream for OSC 133 shell integration markers.
///
/// The scanner is stateful — it handles OSC sequences that may be split
/// across multiple read() calls (e.g., `\x1b]133;` in one chunk, `A\x07`
/// in the next).
///
/// The scanned bytes are NOT modified. They should still be passed to
/// alacritty_terminal which will ignore unknown OSC sequences.
pub struct OscScanner {
    state: ScanState,
    param_buf: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanState {
    /// Normal byte processing, not inside an OSC sequence.
    Normal,
    /// Saw ESC (0x1B), waiting for ] to confirm OSC start.
    SawEsc,
    /// Inside an OSC sequence, accumulating parameter bytes.
    InOsc,
    /// Inside OSC, saw ESC — waiting for \ (ST terminator).
    InOscSawEsc,
}

impl OscScanner {
    pub fn new() -> Self {
        Self {
            state: ScanState::Normal,
            param_buf: Vec::with_capacity(64),
        }
    }

    /// Scan a chunk of bytes for OSC 133 markers.
    ///
    /// Returns all markers found in this chunk. May return empty vec
    /// if no markers are present or if a marker spans across chunks.
    pub fn scan(&mut self, bytes: &[u8]) -> Vec<ShellMarker> {
        let mut markers = Vec::new();

        for &byte in bytes {
            match self.state {
                ScanState::Normal => {
                    if byte == 0x1B {
                        self.state = ScanState::SawEsc;
                    }
                    // 0x9D is the 8-bit OSC introducer (C1 control)
                    else if byte == 0x9D {
                        self.param_buf.clear();
                        self.state = ScanState::InOsc;
                    }
                }

                ScanState::SawEsc => {
                    if byte == b']' {
                        // ESC ] = OSC start
                        self.param_buf.clear();
                        self.state = ScanState::InOsc;
                    } else {
                        // Not an OSC, back to normal
                        self.state = ScanState::Normal;
                    }
                }

                ScanState::InOsc => {
                    if byte == 0x07 {
                        // BEL terminates the OSC sequence
                        if let Some(marker) = self.parse_osc_133() {
                            markers.push(marker);
                        }
                        self.state = ScanState::Normal;
                    } else if byte == 0x1B {
                        // Possible ST (ESC \) terminator
                        self.state = ScanState::InOscSawEsc;
                    } else {
                        self.param_buf.push(byte);
                    }
                }

                ScanState::InOscSawEsc => {
                    if byte == b'\\' {
                        // ESC \ = ST (String Terminator)
                        if let Some(marker) = self.parse_osc_133() {
                            markers.push(marker);
                        }
                        self.state = ScanState::Normal;
                    } else {
                        // False alarm, ESC was part of something else
                        self.param_buf.push(0x1B);
                        self.param_buf.push(byte);
                        self.state = ScanState::InOsc;
                    }
                }
            }
        }

        markers
    }

    /// Parse accumulated OSC parameters to check for 133;X markers.
    fn parse_osc_133(&self) -> Option<ShellMarker> {
        let params = &self.param_buf;

        // Must start with "133;"
        if params.len() < 4 || &params[..4] != b"133;" {
            return None;
        }

        let rest = &params[4..];
        if rest.is_empty() {
            return None;
        }

        match rest[0] {
            b'A' => Some(ShellMarker::PromptStart),
            b'B' => Some(ShellMarker::InputStart),
            b'C' => Some(ShellMarker::CommandStart),
            b'D' => {
                // Parse exit code from "D;N" or "D" (default 0)
                let exit_code = if rest.len() > 2 && rest[1] == b';' {
                    std::str::from_utf8(&rest[2..])
                        .ok()
                        .and_then(|s| {
                            // Exit code may have additional params like ";aid=123"
                            s.split(';').next().and_then(|code| code.parse().ok())
                        })
                        .unwrap_or(0)
                } else {
                    0
                };
                Some(ShellMarker::CommandEnd { exit_code })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_prompt_start() {
        let mut scanner = OscScanner::new();
        let markers = scanner.scan(b"\x1b]133;A\x07");
        assert_eq!(markers, vec![ShellMarker::PromptStart]);
    }

    #[test]
    fn test_scan_command_end_with_exit_code() {
        let mut scanner = OscScanner::new();
        let markers = scanner.scan(b"\x1b]133;D;127\x07");
        assert_eq!(markers, vec![ShellMarker::CommandEnd { exit_code: 127 }]);
    }

    #[test]
    fn test_scan_multiple_markers() {
        let mut scanner = OscScanner::new();
        let input = b"hello\x1b]133;A\x07world\x1b]133;C\x07done";
        let markers = scanner.scan(input);
        assert_eq!(
            markers,
            vec![ShellMarker::PromptStart, ShellMarker::CommandStart]
        );
    }

    #[test]
    fn test_scan_split_across_chunks() {
        let mut scanner = OscScanner::new();
        // Split the sequence across two reads
        let m1 = scanner.scan(b"text\x1b]133;");
        assert!(m1.is_empty());
        let m2 = scanner.scan(b"D;42\x07more");
        assert_eq!(m2, vec![ShellMarker::CommandEnd { exit_code: 42 }]);
    }

    #[test]
    fn test_scan_st_terminator() {
        let mut scanner = OscScanner::new();
        let markers = scanner.scan(b"\x1b]133;B\x1b\\");
        assert_eq!(markers, vec![ShellMarker::InputStart]);
    }

    #[test]
    fn test_scan_ignores_other_osc() {
        let mut scanner = OscScanner::new();
        let markers = scanner.scan(b"\x1b]0;window title\x07");
        assert!(markers.is_empty());
    }
}
