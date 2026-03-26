//! Auto-closing bracket/quote pairs for the input editor.
//!
//! When the user types an opening character like `(`, the matching
//! closing character `)` is automatically inserted and the cursor
//! is placed between them. Paste operations bypass auto-closing
//! to avoid doubling already-matched pairs.

/// Configuration for auto-closing bracket and quote pairs.
#[derive(Clone, Debug, Default)]
pub struct AutoPairConfig {
    pub enabled: bool,
    pub pairs: Vec<AutoPair>,
}

/// A single auto-closing pair definition.
#[derive(Clone, Debug)]
pub struct AutoPair {
    pub open: char,
    pub close: char,
    /// Only auto-close when the character after the cursor is one of these (or EOL).
    pub close_before: Vec<char>,
}

impl AutoPairConfig {
    /// Default pairs for shell editing: `()`, `[]`, `{}`, `""`, `''`, `` `` ``.
    pub fn shell_defaults() -> Self {
        let close_before = vec![' ', ')', ']', '}', '\'', '"', '`', '\n', ';', '|', '&', '>'];
        Self {
            enabled: true,
            pairs: vec![
                AutoPair {
                    open: '(',
                    close: ')',
                    close_before: close_before.clone(),
                },
                AutoPair {
                    open: '[',
                    close: ']',
                    close_before: close_before.clone(),
                },
                AutoPair {
                    open: '{',
                    close: '}',
                    close_before: close_before.clone(),
                },
                AutoPair {
                    open: '"',
                    close: '"',
                    close_before: vec![' ', ')', ']', '}', '\n', ';', '|', '&', '>'],
                },
                AutoPair {
                    open: '\'',
                    close: '\'',
                    close_before: vec![' ', ')', ']', '}', '\n', ';', '|', '&', '>'],
                },
                AutoPair {
                    open: '`',
                    close: '`',
                    close_before: vec![' ', ')', ']', '}', '\n', ';', '|', '&', '>'],
                },
            ],
        }
    }

    /// Check if the typed character should trigger auto-closing.
    /// Returns `Some(closing_char)` if auto-close should happen, `None` otherwise.
    pub fn should_auto_close(
        &self,
        open: char,
        next_char: Option<char>,
        is_pasting: bool,
    ) -> Option<char> {
        if !self.enabled || is_pasting {
            return None;
        }
        self.pairs
            .iter()
            .find(|p| p.open == open)
            .and_then(|pair| match next_char {
                None => Some(pair.close),
                Some(c) if pair.close_before.contains(&c) => Some(pair.close),
                _ => None,
            })
    }

    /// Check if typing the closing character should skip over it instead of inserting.
    /// E.g., typing `)` when the cursor is right before `)` should just move the cursor.
    pub fn should_skip_over(&self, typed: char, char_at_cursor: Option<char>) -> bool {
        self.enabled
            && self
                .pairs
                .iter()
                .any(|p| p.close == typed && char_at_cursor == Some(p.close))
    }

    /// Check if backspace should delete both characters of a pair.
    /// E.g., pressing backspace when cursor is between `(|)` should delete both.
    pub fn should_delete_pair(
        &self,
        char_before: Option<char>,
        char_after: Option<char>,
    ) -> bool {
        self.enabled
            && self
                .pairs
                .iter()
                .any(|p| char_before == Some(p.open) && char_after == Some(p.close))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_close_paren() {
        let config = AutoPairConfig::shell_defaults();
        // Opening paren before space → auto-close
        assert_eq!(config.should_auto_close('(', Some(' '), false), Some(')'));
        // Opening paren before EOL → auto-close
        assert_eq!(config.should_auto_close('(', None, false), Some(')'));
        // Opening paren before letter → no auto-close
        assert_eq!(config.should_auto_close('(', Some('a'), false), None);
        // Opening paren while pasting → no auto-close
        assert_eq!(config.should_auto_close('(', Some(' '), true), None);
    }

    #[test]
    fn test_skip_over() {
        let config = AutoPairConfig::shell_defaults();
        assert!(config.should_skip_over(')', Some(')')));
        assert!(config.should_skip_over('"', Some('"')));
        assert!(!config.should_skip_over(')', Some(' ')));
        assert!(!config.should_skip_over(')', None));
    }

    #[test]
    fn test_delete_pair() {
        let config = AutoPairConfig::shell_defaults();
        assert!(config.should_delete_pair(Some('('), Some(')')));
        assert!(config.should_delete_pair(Some('"'), Some('"')));
        assert!(!config.should_delete_pair(Some('('), Some(']')));
        assert!(!config.should_delete_pair(None, Some(')')));
    }

    #[test]
    fn test_disabled() {
        let config = AutoPairConfig::default();
        assert_eq!(config.should_auto_close('(', Some(' '), false), None);
        assert!(!config.should_skip_over(')', Some(')')));
        assert!(!config.should_delete_pair(Some('('), Some(')')));
    }
}
