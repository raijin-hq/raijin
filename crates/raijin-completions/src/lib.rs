/// CLI completion specs engine for Raijin terminal.
///
/// Provides context-aware command completions using static spec definitions
/// for 400+ popular CLI tools. Specs define subcommands, options, and argument
/// templates (filepaths, git branches, env vars, etc.).
mod matcher;
mod parser;
mod spec;
mod specs;

pub use matcher::{CompletionCandidate, CompletionKind, complete};
pub use parser::{CommandContext, TokenPosition, parse_input};
pub use spec::{ArgTemplate, CliArg, CliOption, CliSpec};
pub use specs::load_all_specs;
