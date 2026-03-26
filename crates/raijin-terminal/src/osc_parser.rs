/// Shell integration markers from OSC sequences.
///
/// OSC 133 markers are sent by the shell hooks (raijin.zsh/bash/fish) to
/// indicate command block boundaries. OSC 7777 carries JSON metadata from
/// the shell's precmd hook (CWD, git branch, username, etc.).
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
    /// OSC 133;P;k=<kind> — Prompt kind (Nushell-specific).
    /// i=initial, c=continuation, s=secondary, r=right.
    PromptKind { kind: PromptKindType },
    /// OSC 7777;raijin-precmd;<hex> — Shell metadata (JSON, hex-decoded).
    Metadata(String),
}

/// Nushell prompt kind types from OSC 133;P.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKindType {
    /// Initial prompt (default).
    Initial,
    /// Continuation prompt (multi-line).
    Continuation,
    /// Secondary prompt.
    Secondary,
    /// Right-side prompt.
    Right,
}

/// Scans a byte stream for OSC 133 and OSC 7777 shell integration markers.
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
            param_buf: Vec::with_capacity(512),
        }
    }

    /// Scan a chunk of bytes for OSC 133 and OSC 7777 markers.
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
                        if let Some(marker) = self.try_parse_osc() {
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
                        if let Some(marker) = self.try_parse_osc() {
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

    /// Try parsing the accumulated OSC as either 133 (block markers) or 7777 (metadata).
    fn try_parse_osc(&self) -> Option<ShellMarker> {
        self.parse_osc_133().or_else(|| self.parse_osc_7777())
    }

    /// Parse OSC 7777;raijin-precmd;<hex> into ShellMarker::Metadata.
    ///
    /// The payload after the prefix is hex-encoded JSON (two hex chars per byte).
    /// This encoding prevents bytes like 0x9C (ST terminator) in emoji/special
    /// chars from breaking the escape sequence — same strategy as Warp.
    fn parse_osc_7777(&self) -> Option<ShellMarker> {
        let params = &self.param_buf;
        let prefix = b"7777;raijin-precmd;";
        if params.len() <= prefix.len() || params[..prefix.len()] != *prefix {
            return None;
        }
        let hex_bytes = &params[prefix.len()..];
        let decoded = hex_decode(hex_bytes)?;
        let json = std::str::from_utf8(&decoded).ok()?;
        Some(ShellMarker::Metadata(json.to_string()))
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
            b'P' => {
                // OSC 133;P;k=<kind> — Nushell prompt kind
                let kind = if rest.len() > 4 && &rest[1..4] == b";k=" {
                    match rest[4] {
                        b'i' => PromptKindType::Initial,
                        b'c' => PromptKindType::Continuation,
                        b's' => PromptKindType::Secondary,
                        b'r' => PromptKindType::Right,
                        _ => PromptKindType::Initial,
                    }
                } else {
                    PromptKindType::Initial
                };
                Some(ShellMarker::PromptKind { kind })
            }
            _ => None,
        }
    }
}

/// Decode a hex-encoded byte slice (e.g., b"48656c6c6f" → b"Hello").
/// Returns None if the input has odd length or contains non-hex characters.
fn hex_decode(input: &[u8]) -> Option<Vec<u8>> {
    if !input.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(input.len() / 2);
    for pair in input.chunks_exact(2) {
        let hi = hex_nibble(pair[0])?;
        let lo = hex_nibble(pair[1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
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

    // --- OSC 7777 (raijin-precmd metadata) tests ---

    fn hex_encode(input: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(input.len() * 2);
        for &b in input {
            out.push(b"0123456789abcdef"[(b >> 4) as usize]);
            out.push(b"0123456789abcdef"[(b & 0x0f) as usize]);
        }
        out
    }

    #[test]
    fn test_scan_osc_7777_basic() {
        let json = br#"{"cwd":"/tmp","username":"nyxb"}"#;
        let hex = hex_encode(json);
        let mut seq = b"\x1b]7777;raijin-precmd;".to_vec();
        seq.extend_from_slice(&hex);
        seq.push(0x07);

        let mut scanner = OscScanner::new();
        let markers = scanner.scan(&seq);
        assert_eq!(
            markers,
            vec![ShellMarker::Metadata(
                r#"{"cwd":"/tmp","username":"nyxb"}"#.to_string()
            )]
        );
    }

    #[test]
    fn test_scan_osc_7777_st_terminator() {
        let json = br#"{"cwd":"/"}"#;
        let hex = hex_encode(json);
        let mut seq = b"\x1b]7777;raijin-precmd;".to_vec();
        seq.extend_from_slice(&hex);
        seq.extend_from_slice(b"\x1b\\");

        let mut scanner = OscScanner::new();
        let markers = scanner.scan(&seq);
        assert_eq!(
            markers,
            vec![ShellMarker::Metadata(r#"{"cwd":"/"}"#.to_string())]
        );
    }

    #[test]
    fn test_scan_osc_7777_split_across_chunks() {
        let json = br#"{"cwd":"/home"}"#;
        let hex = hex_encode(json);
        let mut full = b"\x1b]7777;raijin-precmd;".to_vec();
        full.extend_from_slice(&hex);
        full.push(0x07);

        // Split at an arbitrary midpoint
        let mid = full.len() / 2;
        let mut scanner = OscScanner::new();
        let m1 = scanner.scan(&full[..mid]);
        assert!(m1.is_empty());
        let m2 = scanner.scan(&full[mid..]);
        assert_eq!(
            m2,
            vec![ShellMarker::Metadata(r#"{"cwd":"/home"}"#.to_string())]
        );
    }

    #[test]
    fn test_scan_osc_7777_interleaved_with_133() {
        let json = br#"{"cwd":"/"}"#;
        let hex = hex_encode(json);
        let mut seq = b"\x1b]7777;raijin-precmd;".to_vec();
        seq.extend_from_slice(&hex);
        seq.push(0x07);
        seq.extend_from_slice(b"\x1b]133;A\x07");

        let mut scanner = OscScanner::new();
        let markers = scanner.scan(&seq);
        assert_eq!(
            markers,
            vec![
                ShellMarker::Metadata(r#"{"cwd":"/"}"#.to_string()),
                ShellMarker::PromptStart,
            ]
        );
    }

    #[test]
    fn test_scan_osc_7777_invalid_hex() {
        // Odd-length hex should be ignored
        let mut seq = b"\x1b]7777;raijin-precmd;abc\x07".to_vec();
        let mut scanner = OscScanner::new();
        let markers = scanner.scan(&seq);
        assert!(markers.is_empty());

        // Non-hex characters
        seq = b"\x1b]7777;raijin-precmd;ZZZZ\x07".to_vec();
        let markers = scanner.scan(&seq);
        assert!(markers.is_empty());
    }

    #[test]
    fn test_hex_decode_roundtrip() {
        let original = b"Hello, World!";
        let encoded = hex_encode(original);
        let decoded = super::hex_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }
}
