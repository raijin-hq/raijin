/// CLI completion specs engine for Raijin terminal.
///
/// Provides context-aware command completions using static spec definitions
/// for 400+ popular CLI tools. Specs define subcommands, options, and argument
/// templates (filepaths, git branches, env vars, etc.).
pub mod command_correction;
mod matcher;
pub mod nu_lsp_client;
mod parser;
pub mod shell_completion;
mod spec;
mod specs;

pub use matcher::{CompletionCandidate, CompletionKind, complete};
pub use parser::{CommandContext, TokenPosition, parse_input};
pub use spec::{ArgTemplate, CliArg, CliOption, CliSpec};
pub use specs::load_all_specs;
