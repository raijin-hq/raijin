/// Shell completion provider — implements CompletionProvider for terminal input.
///
/// Provides completions for: commands from $PATH, file paths, git branches,
/// environment variables, and CLI spec-based subcommands/options.
/// Also provides frecency-based inline (ghost-text) completions from command history.
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use inazuma::{Context, Task, Window};
use inazuma_component::input::{InputState, Rope};
use lsp_types::{
    CompletionContext, CompletionItem, CompletionItemKind, CompletionResponse,
    InlineCompletionContext, InlineCompletionItem, InlineCompletionResponse,
    InsertTextFormat,
};

use crate::command_history::CommandHistory;
use crate::completions::nu_lsp_client::NuLspClient;
use crate::shell_install;

/// Convert a byte offset into (line, character) for LSP position.
fn offset_to_position(text: &str, offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in text.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Escape special shell characters in a path (spaces, parens, brackets, etc.)
fn shell_escape_path(path: &str) -> String {
    let mut escaped = String::with_capacity(path.len());
    for ch in path.chars() {
        match ch {
            ' ' | '(' | ')' | '[' | ']' | '{' | '}' | '!' | '&' | '|' | ';' | '\''
            | '"' | '`' | '$' | '#' | '~' | '*' | '?' | '<' | '>' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

/// Shell completion provider with all completion sub-systems.
pub struct ShellCompletionProvider {
    /// Current shell name ("bash", "zsh", "fish", "nu").
    shell: String,
    /// Current working directory (updated on CWD change).
    cwd: Arc<RwLock<PathBuf>>,
    /// Cached executables from $PATH.
    pub path_executables: Arc<RwLock<Vec<String>>>,
    /// CLI specs for known commands (Tier 1: embedded at compile time).
    cli_specs: HashMap<String, raijin_completions::CliSpec>,
    /// Shared command history for frecency ghost-text.
    history: Arc<RwLock<CommandHistory>>,
    /// Nushell LSP client for native Nu completions (lazy-started, only when shell == "nu").
    nu_lsp: Option<NuLspClient>,
    /// Cache for external specs loaded from disk (Tier 2). None = tried and not found.
    external_spec_cache: RwLock<HashMap<String, Option<raijin_completions::CliSpec>>>,
}

impl ShellCompletionProvider {
    pub fn new(shell: &str, cwd: PathBuf, history: Arc<RwLock<CommandHistory>>) -> Self {
        // Start Nu LSP client if running Nushell
        let nu_lsp = if shell == "nu" {
            shell_install::resolve_shell_path("nu")
                .and_then(|path| NuLspClient::new(std::path::Path::new(&path)))
        } else {
            None
        };

        let provider = Self {
            shell: shell.to_string(),
            cwd: Arc::new(RwLock::new(cwd)),
            path_executables: Arc::new(RwLock::new(Vec::new())),
            cli_specs: raijin_completions::load_all_specs(),
            history,
            nu_lsp,
            external_spec_cache: RwLock::new(HashMap::new()),
        };
        provider.scan_path_executables();
        provider
    }

    /// Update CWD (called when ShellContext updates).
    pub fn update_cwd(&self, new_cwd: PathBuf) {
        if let Ok(mut cwd) = self.cwd.write() {
            *cwd = new_cwd;
        }
    }

    /// Scan $PATH for executables (synchronous, called once at startup).
    fn scan_path_executables(&self) {
        let path_var = std::env::var("PATH").unwrap_or_default();
        let mut executables: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for dir in path_var.split(':') {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        if seen.insert(name.clone()) {
                            executables.push(name);
                        }
                    }
                }
            }
        }

        executables.sort();
        if let Ok(mut cache) = self.path_executables.write() {
            *cache = executables;
        }
    }

    /// Complete command names (first word) from $PATH + builtins.
    fn complete_command(&self, prefix: &str) -> Vec<CompletionItem> {
        let cache = self.path_executables.read().unwrap();
        let mut items: Vec<CompletionItem> = cache
            .iter()
            .filter(|cmd| cmd.starts_with(prefix))
            .take(20)
            .map(|cmd| CompletionItem {
                label: cmd.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Command".to_string()),
                ..Default::default()
            })
            .collect();

        // Also include shell builtins
        let builtins = match self.shell.as_str() {
            "zsh" | "bash" | "sh" => vec![
                "cd", "echo", "export", "source", "alias", "unalias", "type",
                "which", "eval", "exec", "set", "unset", "readonly", "shift",
                "test", "true", "false", "pwd", "pushd", "popd", "dirs",
                "bg", "fg", "jobs", "kill", "wait", "trap", "umask",
            ],
            "fish" => vec![
                "cd", "echo", "set", "function", "end", "if", "else",
                "for", "while", "switch", "case", "begin", "return",
                "source", "alias", "type", "test", "true", "false",
            ],
            "nu" => vec![
                "cd", "ls", "mv", "cp", "rm", "mkdir", "open", "save",
                "where", "select", "get", "each", "par-each", "sort-by",
                "group-by", "flatten", "merge", "wrap", "unwrap",
                "to", "from", "into", "str", "math", "date",
            ],
            _ => vec![],
        };

        for b in builtins {
            if b.starts_with(prefix) && !items.iter().any(|i| i.label == b) {
                items.push(CompletionItem {
                    label: b.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some("Builtin".to_string()),
                    ..Default::default()
                });
            }
        }

        items.truncate(30);
        items
    }

    /// Complete file/directory paths relative to CWD.
    fn complete_file_path(&self, partial: &str) -> Vec<CompletionItem> {
        let cwd = self.cwd.read().unwrap().clone();

        let (dir, prefix) = if partial.contains('/') {
            let last_slash = partial.rfind('/').unwrap();
            let dir_part = &partial[..=last_slash];
            let file_prefix = &partial[last_slash + 1..];

            // Handle ~ expansion
            let resolved_dir = if let Some(rest) = dir_part.strip_prefix("~/") {
                dirs::home_dir()
                    .unwrap_or_default()
                    .join(rest)
            } else if dir_part.starts_with('/') {
                PathBuf::from(dir_part)
            } else {
                cwd.join(dir_part)
            };
            (resolved_dir, file_prefix.to_string())
        } else {
            (cwd, partial.to_string())
        };

        let mut items = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    let name_lower = name.to_lowercase();
                    let prefix_lower = prefix.to_lowercase();
                    if name_lower.starts_with(&prefix_lower) && !name.starts_with('.') {
                        let is_dir = entry.file_type().map_or(false, |t| t.is_dir());
                        let suffix = if is_dir { "/" } else { "" };
                        let display = if partial.contains('/') {
                            let dir_prefix = &partial[..partial.rfind('/').unwrap() + 1];
                            format!("{}{}{}", dir_prefix, name, suffix)
                        } else {
                            format!("{}{}", name, suffix)
                        };
                        // Shell-escaped version for actual insertion
                        let escaped_display = shell_escape_path(&display);
                        let type_label = if is_dir { "Directory" } else { "File" };
                        items.push(CompletionItem {
                            label: display.clone(),
                            insert_text: Some(escaped_display),
                            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                            kind: Some(if is_dir {
                                CompletionItemKind::FOLDER
                            } else {
                                CompletionItemKind::FILE
                            }),
                            detail: Some(type_label.to_string()),
                            documentation: Some(lsp_types::Documentation::MarkupContent(
                                lsp_types::MarkupContent {
                                    kind: lsp_types::MarkupKind::PlainText,
                                    value: format!("{}\n{}", display, type_label),
                                },
                            )),
                            ..Default::default()
                        });
                    }
                }
            }
        }
        items.sort_by(|a, b| a.label.cmp(&b.label));
        items.truncate(30);
        items
    }

    /// Complete git branches.
    fn complete_git_branches(&self) -> Vec<CompletionItem> {
        let cwd = self.cwd.read().unwrap().clone();
        let output = std::process::Command::new("git")
            .args(["branch", "--list", "--format=%(refname:short)"])
            .current_dir(&cwd)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .map(|branch| CompletionItem {
                        label: branch.trim().to_string(),
                        kind: Some(CompletionItemKind::REFERENCE),
                        detail: Some("Branch".to_string()),
                        ..Default::default()
                    })
                    .collect()
            }
            _ => vec![],
        }
    }

    /// Complete environment variables.
    fn complete_env_var(&self, prefix: &str) -> Vec<CompletionItem> {
        std::env::vars()
            .filter(|(key, _)| key.starts_with(prefix))
            .take(20)
            .map(|(key, val)| {
                let display_val = if val.len() > 40 {
                    format!("{}...", &val[..37])
                } else {
                    val
                };
                CompletionItem {
                    label: format!("${}", key),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some(display_val),
                    ..Default::default()
                }
            })
            .collect()
    }

    /// Complete using CLI specs.
    fn complete_from_spec(
        &self,
        ctx: &raijin_completions::CommandContext,
        spec: &raijin_completions::CliSpec,
    ) -> Vec<CompletionItem> {
        let candidates = raijin_completions::complete(ctx, spec);
        candidates
            .into_iter()
            .map(|c| {
                let kind = match c.kind {
                    raijin_completions::CompletionKind::Subcommand => CompletionItemKind::MODULE,
                    raijin_completions::CompletionKind::Option => CompletionItemKind::PROPERTY,
                    raijin_completions::CompletionKind::Command => CompletionItemKind::FUNCTION,
                    _ => CompletionItemKind::TEXT,
                };
                CompletionItem {
                    label: c.display,
                    kind: Some(kind),
                    detail: c.description,
                    insert_text: Some(c.text),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    ..Default::default()
                }
            })
            .collect()
    }

    /// Look up a CLI spec by command name, checking embedded (Tier 1) then external (Tier 2).
    fn get_spec(&self, command: &str) -> Option<raijin_completions::CliSpec> {
        // Tier 1: embedded specs (instant)
        if let Some(spec) = self.cli_specs.get(command) {
            return Some(spec.clone());
        }

        // Tier 2: cached external specs
        {
            let cache = self.external_spec_cache.read().unwrap();
            if let Some(cached) = cache.get(command) {
                return cached.clone();
            }
        }

        // Tier 2: load from disk, cache result (also cache None to avoid repeated disk reads)
        let spec = self.load_external_spec(command);
        self.external_spec_cache
            .write()
            .unwrap()
            .insert(command.to_string(), spec.clone());
        spec
    }

    /// Try to load an external spec from ~/.config/raijin/specs/ or app bundle.
    fn load_external_spec(&self, command: &str) -> Option<raijin_completions::CliSpec> {
        let spec_dirs = [
            dirs::config_dir().map(|c| c.join("raijin/specs")),
            // macOS .app bundle: Contents/Resources/specs/
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent()?.parent().map(|b| b.join("Resources/specs"))),
        ];
        for dir in spec_dirs.iter().flatten() {
            let path = dir.join(format!("{}.json", command));
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(spec) = serde_json::from_str::<raijin_completions::CliSpec>(&content) {
                    return Some(spec);
                }
            }
        }
        None
    }

    /// Build completion items for the current input context.
    fn build_completions(&self, text: &str, offset: usize) -> Vec<CompletionItem> {
        // Nushell: try nu --lsp first for builtins/custom commands
        if self.shell == "nu" {
            if let Some(ref nu_lsp) = self.nu_lsp {
                // Calculate line/character from offset
                let (line, character) = offset_to_position(text, offset);
                // Try synchronous receive with a short timeout
                let rx = nu_lsp.complete(text, line, character);
                if let Ok(items) = rx.recv_timeout(std::time::Duration::from_millis(200)) {
                    if !items.is_empty() {
                        return items;
                    }
                }
                // Fallthrough to spec-based completion for external commands
            }
        }

        let ctx = raijin_completions::parse_input(text, offset);

        // Environment variable completion
        if ctx.current_token.starts_with('$') {
            return self.complete_env_var(&ctx.current_token[1..]);
        }

        match &ctx.token_position {
            raijin_completions::TokenPosition::Command => {
                self.complete_command(&ctx.current_token)
            }
            raijin_completions::TokenPosition::Subcommand => {
                if let Some(spec) = self.get_spec(&ctx.command) {
                    let items = self.complete_from_spec(&ctx, &spec);
                    if !items.is_empty() {
                        return items;
                    }
                }
                self.complete_file_path(&ctx.current_token)
            }
            raijin_completions::TokenPosition::OptionName => {
                if let Some(spec) = self.get_spec(&ctx.command) {
                    self.complete_from_spec(&ctx, &spec)
                } else {
                    vec![]
                }
            }
            raijin_completions::TokenPosition::OptionValue(opt) => {
                if let Some(spec) = self.get_spec(&ctx.command) {
                    let resolved = self.resolve_arg_template(&ctx, &spec, opt);
                    if !resolved.is_empty() {
                        return resolved;
                    }
                }
                self.complete_file_path(&ctx.current_token)
            }
            raijin_completions::TokenPosition::Argument(_) => {
                if let Some(spec) = self.get_spec(&ctx.command) {
                    let spec_items = self.complete_from_spec(&ctx, &spec);
                    if !spec_items.is_empty() {
                        return spec_items;
                    }
                }
                if ctx.command == "git" {
                    let git_branch_cmds = ["checkout", "switch", "merge", "rebase", "branch", "diff"];
                    if ctx.subcommands.first().map_or(false, |s| git_branch_cmds.contains(&s.as_str())) {
                        return self.complete_git_branches();
                    }
                }
                self.complete_file_path(&ctx.current_token)
            }
        }
    }

    /// Resolve argument templates from CLI specs to actual completion items.
    fn resolve_arg_template(
        &self,
        ctx: &raijin_completions::CommandContext,
        spec: &raijin_completions::CliSpec,
        _opt_name: &str,
    ) -> Vec<CompletionItem> {
        // For now, fall back to spec-based completions which handle custom values
        self.complete_from_spec(ctx, spec)
    }
}

impl inazuma_component::input::CompletionProvider for ShellCompletionProvider {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        _trigger: CompletionContext,
        _window: &mut Window,
        _cx: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse>> {
        let text_str = text.to_string();
        let items = self.build_completions(&text_str, offset);

        if items.is_empty() {
            Task::ready(Ok(CompletionResponse::Array(vec![])))
        } else {
            Task::ready(Ok(CompletionResponse::Array(items)))
        }
    }

    fn inline_completion(
        &self,
        rope: &Rope,
        _offset: usize,
        _trigger: InlineCompletionContext,
        _window: &mut Window,
        _cx: &mut Context<InputState>,
    ) -> Task<Result<InlineCompletionResponse>> {
        let text = rope.to_string();
        if text.trim().is_empty() {
            return Task::ready(Ok(InlineCompletionResponse::Array(vec![])));
        }

        let history = self.history.read().unwrap();
        let results = history.frecency_search(&text, 1);

        if let Some(entry) = results.first() {
            if entry.command.len() > text.len() {
                let suffix = &entry.command[text.len()..];
                return Task::ready(Ok(InlineCompletionResponse::Array(vec![
                    InlineCompletionItem {
                        insert_text: suffix.to_string(),
                        filter_text: None,
                        range: None,
                        command: None,
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    },
                ])));
            }
        }

        Task::ready(Ok(InlineCompletionResponse::Array(vec![])))
    }

    fn is_completion_trigger(
        &self,
        _offset: usize,
        new_text: &str,
        _cx: &mut Context<InputState>,
    ) -> bool {
        matches!(new_text, "/" | "." | "$" | "-" | " ")
    }
}
