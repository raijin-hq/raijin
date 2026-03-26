//! Parse shell input text into a structured command context for completion matching.

/// Describes where the cursor is within the command structure.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenPosition {
    /// First word — completing the command name.
    Command,
    /// After a known command — completing a subcommand.
    Subcommand,
    /// Token starts with "-" — completing an option name.
    OptionName,
    /// After an option that takes a value — completing the option's value.
    OptionValue(String),
    /// Positional argument at given index.
    Argument(usize),
}

/// Parsed command context for completion matching.
#[derive(Debug, Clone)]
pub struct CommandContext {
    /// The base command name (e.g., "git").
    pub command: String,
    /// Resolved subcommand chain (e.g., ["commit"] for "git commit").
    pub subcommands: Vec<String>,
    /// The current token being typed (may be partial).
    pub current_token: String,
    /// Position of cursor within the current token.
    pub cursor_in_token: usize,
    /// What kind of token the cursor is on.
    pub token_position: TokenPosition,
    /// Options already typed in this command invocation.
    pub preceding_options: Vec<String>,
}

/// Parse raw input text at the given cursor offset into a `CommandContext`.
pub fn parse_input(text: &str, cursor: usize) -> CommandContext {
    let text_to_cursor = &text[..cursor.min(text.len())];

    // Split into tokens (respecting quotes)
    let tokens = shell_tokenize(text_to_cursor);

    if tokens.is_empty() {
        return CommandContext {
            command: String::new(),
            subcommands: Vec::new(),
            current_token: String::new(),
            cursor_in_token: 0,
            token_position: TokenPosition::Command,
            preceding_options: Vec::new(),
        };
    }

    let command = tokens[0].clone();
    let mut subcommands = Vec::new();
    let mut preceding_options = Vec::new();
    let mut arg_index: usize = 0;

    // Check if cursor is right after a space (new token being started)
    let ends_with_space = text_to_cursor.ends_with(' ');

    // Current token is the last one (unless text ends with space)
    let current_token = if ends_with_space {
        String::new()
    } else if tokens.len() > 1 {
        tokens.last().unwrap().clone()
    } else {
        command.clone()
    };

    // If we only have the command token
    if tokens.len() == 1 && !ends_with_space {
        let len = current_token.len();
        return CommandContext {
            command: current_token.clone(),
            subcommands,
            cursor_in_token: len,
            current_token,
            token_position: TokenPosition::Command,
            preceding_options,
        };
    }

    // Parse the middle tokens (between command and current token)
    let middle_end = if ends_with_space {
        tokens.len()
    } else {
        tokens.len() - 1
    };

    for token in &tokens[1..middle_end] {
        if token.starts_with('-') {
            preceding_options.push(token.clone());
        } else {
            // Could be a subcommand or positional arg
            // We treat the first non-option token as a subcommand candidate
            if subcommands.is_empty() && !token.is_empty() {
                subcommands.push(token.clone());
            } else {
                arg_index += 1;
            }
        }
    }

    // Determine token position
    let token_position = if tokens.len() == 1 && ends_with_space {
        // After command with space — could be subcommand or argument
        TokenPosition::Subcommand
    } else if current_token.starts_with('-') {
        TokenPosition::OptionName
    } else if !ends_with_space && tokens.len() > 1 {
        // Check if previous token was an option that takes a value
        let prev = &tokens[tokens.len() - 2];
        if prev.starts_with('-') && !prev.starts_with("--no-") {
            TokenPosition::OptionValue(prev.clone())
        } else if subcommands.is_empty() {
            TokenPosition::Subcommand
        } else {
            TokenPosition::Argument(arg_index)
        }
    } else if ends_with_space {
        // Check if previous token was an option that takes a value
        let prev = tokens.last().unwrap();
        if prev.starts_with('-') && !prev.starts_with("--no-") {
            TokenPosition::OptionValue(prev.clone())
        } else {
            TokenPosition::Argument(arg_index)
        }
    } else {
        TokenPosition::Argument(arg_index)
    };

    CommandContext {
        command,
        subcommands,
        current_token: current_token.clone(),
        cursor_in_token: current_token.len(),
        token_position,
        preceding_options,
    }
}

/// Simple shell tokenizer that respects quotes.
fn shell_tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' if !in_single_quote => {
                escaped = true;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let ctx = parse_input("", 0);
        assert_eq!(ctx.token_position, TokenPosition::Command);
        assert_eq!(ctx.current_token, "");
    }

    #[test]
    fn test_command_only() {
        let ctx = parse_input("gi", 2);
        assert_eq!(ctx.command, "gi");
        assert_eq!(ctx.token_position, TokenPosition::Command);
        assert_eq!(ctx.current_token, "gi");
    }

    #[test]
    fn test_command_with_space() {
        let ctx = parse_input("git ", 4);
        assert_eq!(ctx.command, "git");
        assert_eq!(ctx.token_position, TokenPosition::Subcommand);
        assert_eq!(ctx.current_token, "");
    }

    #[test]
    fn test_subcommand_partial() {
        let ctx = parse_input("git com", 7);
        assert_eq!(ctx.command, "git");
        assert_eq!(ctx.token_position, TokenPosition::Subcommand);
        assert_eq!(ctx.current_token, "com");
    }

    #[test]
    fn test_option() {
        let ctx = parse_input("git commit --mess", 17);
        assert_eq!(ctx.command, "git");
        assert_eq!(ctx.subcommands, vec!["commit"]);
        assert_eq!(ctx.token_position, TokenPosition::OptionName);
        assert_eq!(ctx.current_token, "--mess");
    }

    #[test]
    fn test_option_value() {
        let ctx = parse_input("git commit -m ", 14);
        assert_eq!(ctx.command, "git");
        assert_eq!(ctx.token_position, TokenPosition::OptionValue("-m".into()));
    }

    #[test]
    fn test_quoted_string() {
        let tokens = shell_tokenize(r#"echo "hello world" foo"#);
        assert_eq!(tokens, vec!["echo", "hello world", "foo"]);
    }

    #[test]
    fn test_single_quotes() {
        // In single quotes, backslash is literal (no escaping)
        let tokens = shell_tokenize("echo 'hello world'");
        assert_eq!(tokens, vec!["echo", "hello world"]);
    }
}
