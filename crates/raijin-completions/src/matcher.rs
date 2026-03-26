/// Match a parsed command context against CLI specs to produce completion candidates.
use crate::parser::{CommandContext, TokenPosition};
use crate::spec::{ArgTemplate, CliSpec};

/// A single completion candidate.
#[derive(Debug, Clone)]
pub struct CompletionCandidate {
    /// The text to insert.
    pub text: String,
    /// Display text (may include formatting info).
    pub display: String,
    /// Description shown next to the completion.
    pub description: Option<String>,
    /// Kind of completion (for icon/sorting).
    pub kind: CompletionKind,
    /// Sort priority (lower = higher priority).
    pub sort_priority: u32,
}

/// Category of a completion item.
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionKind {
    Command,
    Subcommand,
    Option,
    Argument,
    FilePath,
    Folder,
    GitBranch,
    EnvVar,
    HistoryEntry,
}

/// Match a `CommandContext` against a `CliSpec` and return completion candidates.
pub fn complete(ctx: &CommandContext, spec: &CliSpec) -> Vec<CompletionCandidate> {
    // Resolve the deepest spec based on subcommands
    let resolved = resolve_spec(spec, &ctx.subcommands);

    match &ctx.token_position {
        TokenPosition::Command => {
            // Should not reach here — command completion is handled by ShellCompletionProvider
            vec![]
        }
        TokenPosition::Subcommand => {
            complete_subcommands(resolved, &ctx.current_token)
        }
        TokenPosition::OptionName => {
            complete_options(resolved, &ctx.current_token, &ctx.preceding_options)
        }
        TokenPosition::OptionValue(opt_name) => {
            complete_option_value(resolved, opt_name)
        }
        TokenPosition::Argument(idx) => {
            complete_argument(resolved, *idx)
        }
    }
}

/// Walk down the spec tree following the subcommand chain.
fn resolve_spec<'a>(spec: &'a CliSpec, subcommands: &[String]) -> &'a CliSpec {
    let mut current = spec;
    for sub in subcommands {
        if let Some(found) = current.find_subcommand(sub) {
            current = found;
        } else {
            break;
        }
    }
    current
}

fn complete_subcommands(spec: &CliSpec, prefix: &str) -> Vec<CompletionCandidate> {
    let mut candidates: Vec<CompletionCandidate> = spec
        .subcommands
        .iter()
        .filter(|s| s.name.starts_with(prefix))
        .map(|s| CompletionCandidate {
            text: s.name.clone(),
            display: s.name.clone(),
            description: s.description.clone(),
            kind: CompletionKind::Subcommand,
            sort_priority: 10,
        })
        .collect();

    // Also include options if user might be typing an option
    if prefix.is_empty() {
        // Show subcommands first, then common options
        candidates.extend(
            spec.options.iter().take(5).map(|o| CompletionCandidate {
                text: o.names.first().cloned().unwrap_or_default(),
                display: o.names.join(", "),
                description: o.description.clone(),
                kind: CompletionKind::Option,
                sort_priority: 50,
            }),
        );
    }

    candidates.sort_by_key(|c| c.sort_priority);
    candidates
}

fn complete_options(
    spec: &CliSpec,
    prefix: &str,
    preceding: &[String],
) -> Vec<CompletionCandidate> {
    spec.options
        .iter()
        .filter(|o| {
            // Filter out already-used non-repeatable options
            if !o.is_repeatable {
                if o.names.iter().any(|n| preceding.contains(n)) {
                    return false;
                }
            }
            // Match against prefix
            o.names.iter().any(|n| n.starts_with(prefix))
        })
        .map(|o| {
            let primary_name = o.names.iter()
                .find(|n| n.starts_with("--"))
                .or(o.names.first())
                .cloned()
                .unwrap_or_default();
            let display = if o.names.len() > 1 {
                o.names.join(", ")
            } else {
                primary_name.clone()
            };
            CompletionCandidate {
                text: primary_name,
                display,
                description: o.description.clone(),
                kind: CompletionKind::Option,
                sort_priority: 20,
            }
        })
        .collect()
}

fn complete_option_value(spec: &CliSpec, opt_name: &str) -> Vec<CompletionCandidate> {
    let Some(opt) = spec.find_option(opt_name) else {
        return vec![];
    };

    match &opt.arg_template {
        Some(ArgTemplate::Custom(values)) => {
            values
                .iter()
                .map(|v| CompletionCandidate {
                    text: v.clone(),
                    display: v.clone(),
                    description: None,
                    kind: CompletionKind::Argument,
                    sort_priority: 10,
                })
                .collect()
        }
        // Other templates (Filepaths, GitBranches, etc.) are handled by ShellCompletionProvider
        _ => vec![],
    }
}

fn complete_argument(spec: &CliSpec, arg_index: usize) -> Vec<CompletionCandidate> {
    let Some(arg) = spec.args.get(arg_index) else {
        // Check for variadic last arg
        if let Some(last) = spec.args.last() {
            if last.is_variadic {
                return match &last.template {
                    Some(ArgTemplate::Custom(values)) => {
                        values
                            .iter()
                            .map(|v| CompletionCandidate {
                                text: v.clone(),
                                display: v.clone(),
                                description: None,
                                kind: CompletionKind::Argument,
                                sort_priority: 10,
                            })
                            .collect()
                    }
                    _ => vec![],
                };
            }
        }
        return vec![];
    };

    match &arg.template {
        Some(ArgTemplate::Custom(values)) => {
            values
                .iter()
                .map(|v| CompletionCandidate {
                    text: v.clone(),
                    display: v.clone(),
                    description: None,
                    kind: CompletionKind::Argument,
                    sort_priority: 10,
                })
                .collect()
        }
        // Other templates delegated to ShellCompletionProvider
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_input;

    fn git_spec() -> CliSpec {
        serde_json::from_str(include_str!("../specs/git.json")).unwrap()
    }

    #[test]
    fn test_git_subcommands() {
        let spec = git_spec();
        let ctx = parse_input("git ", 4);
        let results = complete(&ctx, &spec);
        let names: Vec<&str> = results.iter().map(|r| r.text.as_str()).collect();
        assert!(names.contains(&"commit"));
        assert!(names.contains(&"push"));
        assert!(names.contains(&"pull"));
    }

    #[test]
    fn test_git_subcommand_prefix() {
        let spec = git_spec();
        let ctx = parse_input("git co", 6);
        let results = complete(&ctx, &spec);
        let names: Vec<&str> = results.iter().map(|r| r.text.as_str()).collect();
        assert!(names.contains(&"commit"));
        assert!(!names.contains(&"push"));

        // "ch" matches "checkout" and "cherry-pick"
        let ctx2 = parse_input("git ch", 6);
        let results2 = complete(&ctx2, &spec);
        let names2: Vec<&str> = results2.iter().map(|r| r.text.as_str()).collect();
        assert!(names2.contains(&"checkout"));
        assert!(names2.contains(&"cherry-pick"));
    }

    #[test]
    fn test_git_commit_options() {
        let spec = git_spec();
        let ctx = parse_input("git commit --", 13);
        let results = complete(&ctx, &spec);
        let names: Vec<&str> = results.iter().map(|r| r.text.as_str()).collect();
        assert!(names.contains(&"--message"));
        assert!(names.contains(&"--amend"));
    }
}
