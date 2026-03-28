# Phase 4: IDE-Style Input Editor + Completions — Full Implementation Plan

## Status-Übersicht (Stand: 28. März 2026)

| Feature | Status | Dateien |
|---------|--------|---------|
| ShellEditor Mode (AutoGrow + Syntax-Highlight) | ✅ Done | `input/mode.rs`, `input/state.rs` |
| Auto-Closing Brackets (AutoPairConfig) | ✅ Done | `input/auto_pairs.rs` |
| raijin-completions Crate (Specs + Parser + Matcher) | ✅ Done | `crates/raijin-completions/` |
| CLI Specs: git, cargo | ✅ Done | `raijin-completions/specs/` |
| CLI Specs: 715 Tools (71 embedded + 644 external) | ✅ Done | `raijin-completions/specs/`, `tools/fig-to-raijin/`, `build.rs` |
| ShellCompletionProvider (Command, File, Git, Env, Spec) | ✅ Done | `raijin-app/src/completions/shell_completion.rs` |
| Completion Menu (Warp-style, Icons, Doc Panel, Filtering) | ✅ Done | `input/popovers/completion_menu.rs` |
| Completion Menu: Frozen Position (word-start anchor) | ✅ Done | `completion_menu.rs` (`frozen_origin`) |
| Completion Menu: Live Preview (Pfeil → Input update) | ✅ Done | `completion_menu.rs` (`apply_selected_to_editor`) |
| Completion Menu: Tab Confirm | ✅ Done | `input/indent.rs` |
| Completion Menu: No initial selection | ✅ Done | `completion_menu.rs` |
| Inline Completion (Ghost Text, Frecency History) | ✅ Done | `input/lsp/completions.rs` |
| Inline Completion: Tab Accept | ✅ Done | `input/lsp/completions.rs` |
| Case-insensitive File Matching | ✅ Done | `shell_completion.rs` |
| Shell-Escape Sonderzeichen bei Insert | ✅ Done | `shell_completion.rs` (`shell_escape_path`) |
| Command History (Zsh, Bash, Fish, Nu-Plaintext Parser) | ✅ Done | `raijin-app/src/command_history.rs` |
| Nu SQLite History Parser | ✅ Done | `command_history.rs` (hinter Feature-Flag) |
| History Panel (Open/Close/Select/Filter/Render) | ✅ Done | `raijin-app/src/input/history_panel.rs` |
| Command Correction (Typo-Map + Damerau-Levenshtein) | ✅ Done | `raijin-app/src/completions/command_correction.rs` |
| Overlay Highlight System | ✅ Done | `input/state.rs` (`overlay_highlights`) |
| Command Validation Highlighting (valid = brand color) | ✅ Done | `workspace.rs` (`update_input_highlights`) |
| Completion-Inserted Text Coloring | ✅ Done | `completion_inserted_range` tracking |
| Real-time Filtering (Menu bleibt offen beim Tippen) | ✅ Done | `input/lsp/completions.rs` |
| Nu LSP Client (`nu --lsp`, async) | ✅ Done | `completions/nu_lsp_client.rs`, `shell_completion.rs` |
| Shell-Install-Modal (generisch, plattform-agnostisch) | ✅ Done | `shell_install.rs`, `workspace.rs` |
| External Spec Loader (Tier 2, cached) | ✅ Done | `shell_completion.rs` (`get_spec`, `load_external_spec`) |
| Shift+Enter Multi-line (ohne `\`) | ✅ Done | `input/state.rs` (keybinding + enter handler) |

---

## Context

Raijin's Input-Editor nutzt aktuell `PlainText` single-line ohne Syntax-Highlighting, ohne Command-History, ohne Completions. Das Inazuma-Component `InputState` bietet ein extrem reiches Fundament (Rope, Undo/Redo, CompletionProvider-Trait, InlineCompletion/Ghost-Text, DisplayMap/Soft-Wrap, Mouse-Selection, 40+ Keybinding-Actions, Tree-sitter Highlighting inkl. Bash).

**Ziel:** In einem einzigen, vollständigen Implementierungs-Durchgang den gesamten Input-Editor production-ready fertigstellen. Kein Feature-Flagging, keine Platzhalter, keine "wird später gemacht"-Kommentare. Nach dieser Phase ist der Input-Editor komplett.

**Besonderheit:** Raijin wird das **erste Terminal mit nativem Nushell-Support** — inkl. Syntax-Highlighting, `nu --ide-complete`, SQLite History, und Shell-Integration-Hooks.

---

## Architektur-Überblick

```
┌─────────────────────────────────────────────────────────────────┐
│                        Workspace                                 │
│                                                                   │
│  ┌─────────────────────────┐  ┌────────────────────────────────┐│
│  │     ShellEditorInput     │  │        Terminal Output          ││
│  │                          │  │        (Block Rendering)        ││
│  │  InputState              │  │                                 ││
│  │  ├─ ShellEditor Mode     │  │  BlockManager                  ││
│  │  │  (AutoGrow+Highlight) │  │  ├─ OSC 133 Markers            ││
│  │  ├─ AutoPairs            │  │  ├─ Command Blocks             ││
│  │  ├─ CompletionProvider   │  │  └─ Correction Banner          ││
│  │  │  └─ShellCompletion    │  │                                 ││
│  │  │    ├─ PathCompleter   │  └────────────────────────────────┘│
│  │  │    ├─ CommandCompleter│                                     │
│  │  │    ├─ GitCompleter    │  ┌────────────────────────────────┐│
│  │  │    ├─ EnvCompleter    │  │     Shell Selector Dropdown     ││
│  │  │    ├─ SpecCompleter   │  │     Context Chips               ││
│  │  │    ├─ HistoryCompleter│  ┌────────────────────────────────┐
│  │  │    │                 │  │     HistoryPanel (Overlay)      │
│  │  │    │                 │  │     ├─ Scrollable Entry List    │
│  │  │    │                 │  │     ├─ Selected Highlight       │
│  │  │    │                 │  │     ├─ Fuzzy Filter             │
│  │  │    │                 │  │     └─ Navigation Hints         │
│  │  │    │                 │  └────────────────────────────────┘  └────────────────────────────────┘│
│  │  │    └─ NuIdeCompleter  │                                     │
│  │  └─ InlineCompletion     │  ┌────────────────────────────────┐│
│  │     └─ FrecencyGhost     │  │     CommandHistory              ││
│  │                          │  │     ├─ Zsh Histfile Parser      ││
│  │  CommandHistory ◄────────│──│     ├─ Bash Histfile Parser     ││
│  └─────────────────────────┘  │     ├─ Fish Histfile Parser     ││
│                                │     ├─ Nu SQLite Parser         ││
│  ┌─────────────────────────┐  │     └─ Frecency Scoring         ││
│  │   raijin-completions     │  └────────────────────────────────┘│
│  │   ├─ CliSpec Structs     │                                     │
│  │   ├─ Input Parser        │  ┌────────────────────────────────┐│
│  │   ├─ Spec Matcher        │  │     CommandCorrection           ││
│  │   └─ 400+ Tool Specs     │  │     └─ Levenshtein + Typo Map  ││
│  └─────────────────────────┘  └────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

---

## 1. Shell Editor Mode (`InputMode::ShellEditor`)

### Problem
`CodeEditor` Mode hat Line Numbers + Folding + Indent Guides — falsch für Shell-Input. `AutoGrow` Mode hat keine Syntax-Highlighting.

### Lösung
Neuer `InputMode::ShellEditor` Variant — AutoGrow + Syntax-Highlighting, ohne CodeEditor-Extras. Shell-Language dynamisch basierend auf aktiver Shell.

### Dateien

**`crates/inazuma-component/ui/src/input/mode.rs`**
```rust
ShellEditor {
    min_rows: usize,
    max_rows: usize,
    rows: usize,
    language: SharedString,
    highlighter: Rc<RefCell<Option<SyntaxHighlighter>>>,
    parse_task: Rc<RefCell<Option<Task<()>>>>,
}
```
Alle match-Arms erweitern: `is_multi_line` → true, `rows`/`set_rows` → wie AutoGrow, `update_auto_grow` → aktiv, `update_highlighter` → wie CodeEditor, `highlighter()` → Rc-Ref. `is_code_editor()` → false (keine Line Numbers), `show_indent_guides()` → false, `show_folding()` → false.

**`crates/inazuma-component/ui/src/input/state.rs`**
```rust
pub fn shell_editor(mut self, language: impl Into<SharedString>, min_rows: usize, max_rows: usize) -> Self {
    self.mode = InputMode::ShellEditor {
        min_rows, max_rows, rows: min_rows,
        language: language.into(),
        highlighter: Rc::new(RefCell::new(None)),
        parse_task: Rc::new(RefCell::new(None)),
    };
    self.soft_wrap = true;
    self
}
```

**`crates/raijin-app/src/workspace.rs`**
```rust
// Shell-Language basierend auf erkannter Shell
let lang = match shell_name.as_str() {
    "nu" => "nu",
    "fish" => "fish",   // Fallback zu bash wenn kein tree-sitter-fish
    _ => "bash",        // zsh, bash, sh → bash highlighting
};
let input_state = cx.new(|cx| {
    InputState::new(window, cx)
        .shell_editor(lang, 1, 10)
        .auto_pairs(AutoPairConfig::shell_defaults())
});
```

### Shell-Language Mapping

| Shell | Tree-sitter Language | Status |
|-------|---------------------|--------|
| bash, zsh, sh | `Language::Bash` | ✅ Bereits in Registry |
| fish | `Language::Bash` (Fallback) | Kein tree-sitter-fish in Registry, Bash ist nah genug |
| nu | `Language::Nu` | Neu hinzufügen via `tree-sitter-nu` Dependency |

**`crates/inazuma-component/ui/src/highlighter/languages.rs`** — `Language::Nu` Variant hinzufügen:
```rust
Nu,
// In name(): "nu" => "nu",
// In from_str(): "nu" | "nushell" => Some(Language::Nu),
```

**`crates/inazuma-component/ui/Cargo.toml`** — tree-sitter-nu Dependency hinzufügen.

---

## 2. Auto-Closing Brackets + Paste Detection

### Dateien

**`crates/inazuma-component/ui/src/input/auto_pairs.rs`** (NEU)

```rust
pub struct AutoPairConfig {
    pub enabled: bool,
    pub pairs: Vec<AutoPair>,
}

pub struct AutoPair {
    pub open: char,
    pub close: char,
    /// Nur auto-closen wenn nächstes Zeichen in dieser Liste ist (oder EOL)
    pub close_before: Vec<char>,
}

impl AutoPairConfig {
    pub fn shell_defaults() -> Self {
        Self {
            enabled: true,
            pairs: vec![
                AutoPair { open: '(', close: ')', close_before: vec![' ', ')', ']', '}', '\'', '"', '`', '\n'] },
                AutoPair { open: '[', close: ']', close_before: vec![' ', ')', ']', '}', '\'', '"', '`', '\n'] },
                AutoPair { open: '{', close: '}', close_before: vec![' ', ')', ']', '}', '\'', '"', '`', '\n'] },
                AutoPair { open: '"', close: '"', close_before: vec![' ', ')', ']', '}', '\n'] },
                AutoPair { open: '\'', close: '\'', close_before: vec![' ', ')', ']', '}', '\n'] },
                AutoPair { open: '`', close: '`', close_before: vec![' ', ')', ']', '}', '\n'] },
            ],
        }
    }

    /// Prüft ob auto-close aktiviert werden soll
    pub fn should_auto_close(&self, open: char, next_char: Option<char>, is_pasting: bool) -> Option<char> {
        if !self.enabled || is_pasting { return None; }
        self.pairs.iter().find(|p| p.open == open).and_then(|pair| {
            match next_char {
                None => Some(pair.close),  // EOL
                Some(c) if pair.close_before.contains(&c) => Some(pair.close),
                _ => None,
            }
        })
    }

    /// Prüft ob Closing-Char übersprungen werden soll (Skip-Over)
    pub fn should_skip_over(&self, typed: char, char_at_cursor: Option<char>) -> bool {
        self.enabled && self.pairs.iter().any(|p| p.close == typed && char_at_cursor == Some(p.close))
    }

    /// Prüft ob bei Backspace das Pair gelöscht werden soll
    pub fn should_delete_pair(&self, char_before: Option<char>, char_after: Option<char>) -> bool {
        self.enabled && self.pairs.iter().any(|p| char_before == Some(p.open) && char_after == Some(p.close))
    }
}
```

**`crates/inazuma-component/ui/src/input/state.rs`** — Erweiterungen:

```rust
// Neue Felder in InputState:
auto_pairs: AutoPairConfig,
is_pasting: bool,

// Builder:
pub fn auto_pairs(mut self, config: AutoPairConfig) -> Self {
    self.auto_pairs = config;
    self
}

// In replace_text_in_range_silent() — Auto-Close Hook:
// Wenn new_text.len() == 1 (single char typed):
//   let typed = new_text.chars().next().unwrap();
//   let next_char = self.text.char_at(cursor_offset);
//   if let Some(close) = self.auto_pairs.should_auto_close(typed, next_char, self.is_pasting) {
//       // Insert open+close, position cursor between them
//   }
//   if self.auto_pairs.should_skip_over(typed, next_char) {
//       // Move cursor right instead of inserting
//   }

// In paste():
//   self.is_pasting = true;
//   self.replace_text_in_range_silent(...);
//   self.is_pasting = false;

// In backspace():
//   let char_before = self.text.char_at(cursor_offset - 1);
//   let char_after = self.text.char_at(cursor_offset);
//   if self.auto_pairs.should_delete_pair(char_before, char_after) {
//       // Delete both characters
//   }
```

**`crates/inazuma-component/ui/src/input/mod.rs`** — `mod auto_pairs; pub use auto_pairs::*;`

---

## 3. Command History + History Panel

### Design (Warp-Referenz)

Wenn der User Up/Down drückt, erscheint ein **History Panel** — ein visuelles Overlay zwischen Terminal-Output und Input-Area:

```
┌─────────────────────────────────────────────────────────────────┐
│  Terminal Output (scrollt nach oben, macht Platz)               │
│  ...                                                             │
│  ✅ Successfully installed curl                                  │
├─────────────────────────────────────────────────────────────────┤
│  HISTORY PANEL (Overlay, slide-in von unten)                     │
│                                                                   │
│  >_ cargo raijin dev                                   21h ago  │
│  >_ cd                                                 21h ago  │
│  >_ /bin/bash -c "$(curl -fsSL https://raw.gith...   18h ago  │
│  >_ brew install cairo pkg-config                      18h ago  │
│  >_ test                                               14h ago  │
│  >_ brew update                                        14h ago  │
│  >_ for pkg in node python git wget broken-pkg...     12h ago  │ ← highlight
│  >_ for pkg in node python git wget broken-pkg...     12h ago  │
│                                                                   │
│  ↑ ↓ to navigate   [esc] to dismiss                              │
├─────────────────────────────────────────────────────────────────┤
│  [nyxb] [MacBook-Pro.fritz.box] [📁 ~] [09:15]                  │  ← Context Chips
│                                                                   │
│  for pkg in node python git wget broken-pkg curl; do            │  ← Input Editor
│    echo "==> Fetching $pkg"                                      │  (zeigt den
│    sleep 0.2                                                      │   ausgewählten
│    if [ "$pkg" = "broken-pkg" ]; then                            │   Command mit
│      echo "⚠ Warning: checksum mismatch"                        │   voller Syntax-
│      echo "❌ Error: failed to install $pkg" >&2                 │   Highlighting)
│    else                                                           │
│      echo "✅ Successfully installed $pkg"                       │
│    fi                                                             │
│    sleep 0.1                                                      │
│  done█                                                            │
│                                                                   │
│  ↵ to execute                                                     │
└─────────────────────────────────────────────────────────────────┘
```

### Verhalten

| Aktion | Effekt |
|--------|--------|
| **Up** (bei Cursor Row 0 oder leerer Input) | History Panel öffnet sich, selektiert neuesten Entry |
| **Up/Down** (Panel offen) | Navigiert durch History-Entries |
| **Enter** (Panel offen) | Schließt Panel, führt ausgewählten Command aus |
| **Esc** (Panel offen) | Schließt Panel, stellt gespeicherten Input wieder her |
| **Tippen** (Panel offen) | Filtert History-Entries (Fuzzy-Suche) |
| **Click** auf Entry | Selektiert diesen Entry, zeigt im Input |
| **Scroll** | Scrollt durch History (Mouse-Wheel) |

### History Panel UI-Elemente

Jede Zeile im Panel:
- **`>_` Icon** (Shell-Prompt-Symbol, subtil grau)
- **Command-Text** — truncated auf eine Zeile, mit `...` bei Overflow
- **Relative Zeit** — right-aligned, `21h ago`, `3d ago`, `just now`
- **Highlight** — aktuell selektierter Entry hat accent-Background (`#14F195` @ 10% opacity)
- **Hinweis-Leiste unten:** `↑ ↓ to navigate` `esc to dismiss`

### Dateien

**`crates/raijin-app/src/command_history.rs`** (NEU)

```rust
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub command: String,
    pub timestamp: u64,         // Unix timestamp in seconds
    pub frequency: u32,         // How often this exact command was used
    pub exit_status: Option<i32>,
    pub cwd: Option<String>,
    pub shell: Option<String>,  // Which shell executed this
}

pub struct CommandHistory {
    entries: Vec<HistoryEntry>,          // All entries, newest last
    dedup_index: HashMap<String, usize>, // command → index in entries (for frecency)
}

#[derive(Debug, Clone, Copy)]
pub enum HistfileFormat {
    Zsh,           // `: timestamp:duration;command`
    Bash,          // One command per line
    Fish,          // YAML: `- cmd: command\n  when: timestamp`
    NuSqlite,      // SQLite DB: history table
    NuPlaintext,   // One command per line (Nu alternative)
}

impl CommandHistory {
    pub fn new() -> Self { ... }
    pub fn load_from_histfile(path: &Path, format: HistfileFormat) -> Result<Self> { ... }
    pub fn detect_and_load(shell: &str) -> Self { ... }
    pub fn push(&mut self, command: String) { ... }
    pub fn entries(&self) -> &[HistoryEntry] { ... }

    /// Frecency-scored search for prefix matches (for ghost-text + filtering)
    pub fn frecency_search(&self, prefix: &str, limit: usize) -> Vec<&HistoryEntry> { ... }

    /// Fuzzy filter (when user types while panel is open)
    pub fn fuzzy_filter(&self, query: &str) -> Vec<&HistoryEntry> { ... }
}

// --- Histfile Parsers (all 5 formats) ---
// Zsh:  `: 1234567890:0;command`
// Bash: one command per line, optionally with #timestamp
// Fish: YAML `- cmd: command\n  when: timestamp`
// Nu SQLite: SELECT command_line, start_timestamp, duration, exit_status, cwd FROM history
// Nu Plaintext: one command per line
```

**`crates/raijin-app/src/history_panel.rs`** (NEU)

```rust
pub struct HistoryPanel {
    visible: bool,
    entries: Vec<HistoryEntry>,       // Currently displayed (filtered) entries
    selected_index: usize,            // Currently highlighted entry
    saved_input: String,              // Input saved when panel opens
    filter_query: String,             // Current filter text (user typing while panel open)
    scroll_offset: usize,             // For scrolling through long history
    max_visible_rows: usize,          // How many rows fit in the panel
}

impl HistoryPanel {
    pub fn open(&mut self, history: &CommandHistory, current_input: &str) {
        self.visible = true;
        self.saved_input = current_input.to_string();
        self.entries = history.entries().to_vec();
        self.entries.reverse(); // Newest first
        self.selected_index = 0;
        self.filter_query.clear();
    }

    pub fn close(&mut self) -> &str {
        self.visible = false;
        &self.saved_input
    }

    pub fn select_previous(&mut self) {
        if self.selected_index + 1 < self.entries.len() {
            self.selected_index += 1;
        }
    }

    pub fn select_next(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn selected_command(&self) -> Option<&str> {
        self.entries.get(self.selected_index).map(|e| e.command.as_str())
    }

    pub fn filter(&mut self, query: &str, history: &CommandHistory) {
        self.filter_query = query.to_string();
        if query.is_empty() {
            self.entries = history.entries().to_vec();
            self.entries.reverse();
        } else {
            self.entries = history.fuzzy_filter(query).into_iter().cloned().collect();
        }
        self.selected_index = 0;
    }

    /// Relative time display: "just now", "5m ago", "3h ago", "2d ago"
    pub fn relative_time(timestamp: u64) -> String { ... }
}
```

### Rendering

Das History Panel wird in `workspace.rs` als **Inazuma Element** gerendert — ein `div()` mit:
- Fixed height (max 40% des Terminal-Bereichs)
- Scrollable list von History-Entries
- Jeder Entry: `flex().row()` mit Icon + Text (truncated) + relative Zeit
- Selected Entry: accent-colored Background
- Bottom bar: Navigation hints
- **Slide-in Animation** von unten (opacity + translate_y)

### InputEvent + Workspace Wiring

**`crates/inazuma-component/ui/src/input/state.rs`**
```rust
pub enum InputEvent {
    Change(SharedString),
    PressEnter,
    Focus,
    Blur,
    HistoryUp,    // NEU — emitted when Up on row 0
    HistoryDown,  // NEU — emitted when Down on last row
}
```

**`crates/inazuma-component/ui/src/input/movement.rs`**
```rust
// In move_up(): Wenn cursor auf Row 0 → emit HistoryUp
// In move_down(): Wenn cursor auf letzter Row → emit HistoryDown
```

**`crates/raijin-app/src/workspace.rs`**
```rust
// In on_input_event():
InputEvent::HistoryUp => {
    if !self.history_panel.visible {
        let current = input_state.read(cx).value();
        self.history_panel.open(&self.command_history, &current);
    } else {
        self.history_panel.select_previous();
    }
    // Update input to show selected command
    if let Some(cmd) = self.history_panel.selected_command() {
        input_state.update(cx, |s, cx| s.set_value(cmd, window, cx));
    }
}
InputEvent::HistoryDown => {
    if self.history_panel.visible {
        self.history_panel.select_next();
        if let Some(cmd) = self.history_panel.selected_command() {
            input_state.update(cx, |s, cx| s.set_value(cmd, window, cx));
        }
    }
}
// Escape handling:
// If history_panel.visible → close panel, restore saved input
// Enter handling:
// If history_panel.visible → close panel, execute selected command
```

---

## 4. Shell Completion Provider

### Neues Crate: `raijin-completions`

```
crates/raijin-completions/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── spec.rs          # CliSpec, Subcommand, CliOption, CliArg Structs
│   ├── parser.rs        # Parse shell input → CommandContext
│   ├── matcher.rs       # Match CommandContext gegen Specs → CompletionItems
│   └── specs.rs         # Eingebettete Specs via include_str!
└── specs/               # JSON-Specs (vereinfachtes Fig-Format)
    ├── git.json
    ├── cargo.json
    ├── docker.json
    ├── npm.json
    ├── ... (400+ Specs)
```

**`crates/raijin-completions/src/spec.rs`**
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct CliSpec {
    pub name: String,
    pub aliases: Vec<String>,
    pub description: Option<String>,
    pub subcommands: Vec<CliSpec>,  // Recursive — subcommands have same structure
    pub options: Vec<CliOption>,
    pub args: Vec<CliArg>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CliOption {
    pub names: Vec<String>,         // ["-v", "--verbose"]
    pub description: Option<String>,
    pub takes_arg: bool,
    pub arg_name: Option<String>,   // e.g., "FILE", "PATH"
    pub arg_template: Option<ArgTemplate>,
    pub is_repeatable: bool,
    pub is_required: bool,
    pub exclusive_of: Vec<String>,  // Mutually exclusive options
}

#[derive(Debug, Clone, Deserialize)]
pub struct CliArg {
    pub name: String,
    pub description: Option<String>,
    pub template: Option<ArgTemplate>,
    pub is_optional: bool,
    pub is_variadic: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub enum ArgTemplate {
    Filepaths,
    Folders,
    History,
    GitBranches,
    GitTags,
    GitRemotes,
    EnvVars,
    ProcessIds,
    Custom(Vec<String>),  // Static list of values
}
```

**`crates/raijin-completions/src/parser.rs`**
```rust
pub struct CommandContext {
    pub command: String,             // "git"
    pub subcommands: Vec<String>,    // ["commit"]
    pub current_token: String,       // "--mess"
    pub cursor_in_token: usize,     // Position within current token
    pub token_position: TokenPosition,
    pub preceding_options: Vec<String>, // Already typed options
}

pub enum TokenPosition {
    Command,           // First word — completing the command name
    Subcommand,        // After known command — completing subcommand
    OptionName,        // Starts with "-" — completing option name
    OptionValue(String), // After option that takes value — completing value
    Argument(usize),   // Positional argument index
}

/// Parse raw input text + cursor offset → CommandContext
pub fn parse_input(text: &str, cursor: usize) -> CommandContext { ... }
```

**`crates/raijin-completions/src/matcher.rs`**
```rust
/// Match CommandContext against a CliSpec, return completion items
pub fn complete(ctx: &CommandContext, spec: &CliSpec) -> Vec<CompletionCandidate> { ... }

pub struct CompletionCandidate {
    pub text: String,
    pub display: String,
    pub description: Option<String>,
    pub kind: CompletionKind,
    pub sort_priority: u32,
}

pub enum CompletionKind {
    Command,
    Subcommand,
    Option,
    Argument,
    FilePath,
    GitBranch,
    EnvVar,
    HistoryEntry,
}
```

### Shell Completion Provider

**`crates/raijin-app/src/shell_completion.rs`** (NEU)

```rust
use raijin_completions::{CliSpec, CommandContext, CompletionCandidate, ArgTemplate};
use inazuma_component::input::{CompletionProvider, InlineCompletionResponse};

pub struct ShellCompletionProvider {
    shell: String,                          // "bash", "zsh", "fish", "nu"
    cwd: Arc<RwLock<PathBuf>>,
    path_executables: Arc<RwLock<Vec<String>>>,
    shell_builtins: Vec<String>,
    cli_specs: HashMap<String, CliSpec>,     // command name → spec
    history: Arc<RwLock<CommandHistory>>,
    nu_binary: Option<PathBuf>,             // Path to `nu` binary (for --ide-complete)
}

impl ShellCompletionProvider {
    pub fn new(shell: &str, cwd: PathBuf, history: Arc<RwLock<CommandHistory>>) -> Self {
        let mut provider = Self {
            shell: shell.to_string(),
            cwd: Arc::new(RwLock::new(cwd)),
            path_executables: Arc::new(RwLock::new(Vec::new())),
            shell_builtins: Self::builtins_for_shell(shell),
            cli_specs: raijin_completions::load_all_specs(),
            history,
            nu_binary: which::which("nu").ok(),
        };
        provider.scan_path_executables_background();
        provider
    }

    /// Scan $PATH in background thread, populate path_executables
    fn scan_path_executables_background(&self) { ... }

    /// Update CWD (called when ShellContext updates)
    pub fn update_cwd(&self, new_cwd: PathBuf) {
        *self.cwd.write().unwrap() = new_cwd;
    }

    // --- Completion Sub-Systems ---

    fn complete_command(&self, prefix: &str) -> Vec<CompletionCandidate> {
        // Match against: $PATH executables + shell builtins + aliases
    }

    fn complete_file_path(&self, partial: &str) -> Vec<CompletionCandidate> {
        // List directory entries relative to CWD, handle ~/ expansion
    }

    fn complete_git(&self, ctx: &CommandContext) -> Vec<CompletionCandidate> {
        // git branch --list, git tag, git remote, git stash list
    }

    fn complete_env_var(&self, prefix: &str) -> Vec<CompletionCandidate> {
        // std::env::vars() filtered by prefix
    }

    fn complete_from_spec(&self, ctx: &CommandContext, spec: &CliSpec) -> Vec<CompletionCandidate> {
        // Delegate to raijin_completions::matcher::complete()
    }

    fn complete_via_nu_ide(&self, text: &str, offset: usize) -> Vec<CompletionCandidate> {
        // Spawn `nu --ide-complete <offset>` with text on stdin
        // Parse JSON response → CompletionCandidate
        // Only used when shell == "nu"
    }
}

impl CompletionProvider for ShellCompletionProvider {
    fn completions(&self, text: &Rope, offset: usize, ...) -> Task<Result<CompletionResponse>> {
        let text_str = text.to_string();
        let ctx = raijin_completions::parser::parse_input(&text_str, offset);

        // Priority chain:
        // 1. Nushell native completions (if shell == "nu")
        if self.shell == "nu" {
            return self.complete_via_nu_ide(&text_str, offset);
        }

        // 2. CLI Spec completions (if command has a spec)
        if let Some(spec) = self.cli_specs.get(&ctx.command) {
            return self.complete_from_spec(&ctx, spec);
        }

        // 3. Context-aware completions
        match ctx.token_position {
            TokenPosition::Command => self.complete_command(&ctx.current_token),
            TokenPosition::OptionName => vec![], // No spec available, can't complete
            TokenPosition::Argument(_) | TokenPosition::OptionValue(_) => {
                // Check for ArgTemplate hints from spec, otherwise file completion
                self.complete_file_path(&ctx.current_token)
            }
            _ => self.complete_file_path(&ctx.current_token),
        }

        // 4. Environment variables (after $)
        if ctx.current_token.starts_with('$') {
            return self.complete_env_var(&ctx.current_token[1..]);
        }
    }

    fn inline_completion(&self, text: &Rope, offset: usize, ...) -> Task<Result<InlineCompletionResponse>> {
        // Frecency-based ghost text from command history
        let text_str = text.to_string();
        if text_str.trim().is_empty() { return Task::ready(Ok(None)); }

        let history = self.history.read().unwrap();
        if let Some(entry) = history.frecency_search(&text_str, 1).first() {
            let suffix = &entry.command[text_str.len()..];
            return Task::ready(Ok(Some(InlineCompletionItem {
                text: suffix.to_string(),
                // ...
            })));
        }
        Task::ready(Ok(None))
    }

    fn is_completion_trigger(&self, offset: usize, new_text: &str, ...) -> bool {
        // Trigger on: Tab, /, ., $, -, space (after command)
        matches!(new_text, "/" | "." | "$" | "-")
    }
}
```

### Completion Menu Design (Warp-Referenz)

Das bestehende `CompletionMenu` in inazuma-component (`popovers/completion_menu.rs`) hat bereits das richtige UI-Pattern:
- **Popover ÜBER dem Input** (nicht darunter)
- **Highlight-Prefix** — getippter Text wird im Item-Label hervorgehoben
- **Detail/Description** — grauer Text rechts neben dem Command-Name
- **Selected-Highlight** — accent color Background
- **Scrollbare List** (max 240px Höhe)

**Erweiterungen für Shell-Completions:**

Jedes Completion-Item zeigt:
```
┌──────────────────────────────────────────────┐
│  >_  brew    Package manager for macOS       │  ← Selected (accent bg)
│  >_  _brew   Shell function                  │
│  >_  _x_borderwidth   Shell function         │
└──────────────────────────────────────────────┘
```

- **`>_` Icon** — kleines Shell-Prompt-Symbol (grau) links von jedem Entry
- **Command-Name** — bold, mit getipptem Prefix hervorgehoben
- **Description** — grau, truncated mit `...` bei Overflow
- **`tab ▾` Badge** — im Input rechts neben dem Cursor, zeigt an dass Tab akzeptiert

**Anpassungen in `CompletionMenuItem::render()`:**
- `>_` Icon Element vor dem Label hinzufügen
- `detail` Text als truncated Label rechts-aligned
- Prefix-Highlighting mit accent color statt nur blue

**`tab ▾` Badge im Input:**
- Rendern in `element.rs` wenn CompletionMenu offen oder Ghost-Text sichtbar
- Kleines Label mit Border rechts vom Cursor

### Tab-Key Handling

**`crates/inazuma-component/ui/src/input/state.rs`** — Tab-Logik anpassen:
- Wenn InlineCompletion sichtbar → Accept ghost text
- Wenn CompletionMenu offen → Accept selected item
- Wenn Wort vor Cursor UND CompletionProvider vorhanden → Trigger completion menu
- Sonst → Insert tab / indent

---

## 5. Nushell First-Class Support

**Referenz:** `plan/12-NUSHELL-FIRST-CLASS.md` für die vollständige Architektur.

### Kernprinzip
Nushell emittiert **OSC 133 nativ** seit reedline#1019 — Raijins BlockManager funktioniert ohne Änderung. Wir brauchen nur:
1. PTY-Injection für `raijin.nu` (OSC 7777 Metadata)
2. `nu --ide-complete` als Completion-Backend
3. SQLite History-Parsing
4. Tree-sitter-nu für Highlighting

### Neue Dateien

**`shell/nushell/vendor/autoload/raijin.nu`**
```nushell
# Raijin Terminal — Nushell Integration
# OSC 133 (block boundaries) is handled natively by Nushell/reedline.
# This script adds Raijin-specific features on top.

let features = ($env.RAIJIN_SHELL_FEATURES? | default "metadata,sudo" | split row ",")

if "metadata" in $features {
    $env.config.hooks.pre_prompt = (
        $env.config.hooks.pre_prompt | default [] | append {||
            let meta = {
                cwd: ($env.PWD),
                username: (whoami | str trim),
                shell: "nu",
                shell_version: (version | get version),
                last_duration_ms: ($env.CMD_DURATION_MS? | default 0),
            }
            # Git info
            let meta = if (do { git rev-parse --git-dir } | complete).exit_code == 0 {
                $meta | merge {
                    git_branch: (git rev-parse --abbrev-ref HEAD | str trim),
                    git_dirty: ((do { git diff --quiet HEAD } | complete).exit_code != 0),
                }
            } else { $meta }
            let hex = ($meta | to json -r | encode hex)
            print -n $"\e]7777;raijin-precmd;($hex)\a"
        }
    )
}

if "sudo" in $features {
    def --env raijin-sudo [...args: string] {
        if ("-e" in $args) or ("--edit" in $args) {
            ^sudo ...$args
        } else {
            ^sudo $"TERMINFO=($env.TERMINFO? | default '')" ...$args
        }
    }
    alias sudo = raijin-sudo
}
```

### Geänderte Dateien

**`crates/raijin-terminal/src/pty.rs`** — Nu-Detection + XDG_DATA_DIRS Injection:
```rust
"nu" => {
    // Nu emits OSC 133 natively — no marker injection needed
    let hooks_dir = hooks_dir.join("nushell");
    if hooks_dir.join("vendor/autoload/raijin.nu").exists() {
        let xdg = std::env::var("XDG_DATA_DIRS").unwrap_or_default();
        let raijin_xdg = format!("{}:{}", hooks_dir.display(), xdg);
        cmd.env("XDG_DATA_DIRS", &raijin_xdg);
    }
    cmd.env("RAIJIN_SHELL_FEATURES", "metadata,sudo");
}
```

**`crates/raijin-terminal/src/osc_parser.rs`** — OSC 133;P (PromptKind) Parser:
```rust
pub enum PromptKindType {
    Initial,       // k=i
    Continuation,  // k=c
    Secondary,     // k=s
    Right,         // k=r
}

// In ShellMarker:
PromptKind { kind: PromptKindType },
```

---

## 6. Command Corrections

**`crates/raijin-app/src/command_correction.rs`** (NEU)

```rust
use strsim::damerau_levenshtein;

/// Common typo map (instant corrections, no fuzzy matching needed)
const TYPO_MAP: &[(&str, &str)] = &[
    ("gti", "git"), ("sl", "ls"), ("dc", "cd"), ("grpe", "grep"),
    ("dokcer", "docker"), ("pytohn", "python"), ("ndoe", "node"),
    ("claer", "clear"), ("eixt", "exit"), ("whcih", "which"),
    ("suod", "sudo"), ("mkae", "make"), ("carg", "cargo"),
];

pub struct CorrectionResult {
    pub original: String,
    pub suggestion: String,
    pub confidence: f64,  // 0.0-1.0
}

/// Suggest correction for a failed command (exit code 127 = command not found)
pub fn suggest_correction(
    command_line: &str,
    exit_code: i32,
    known_commands: &[String],
) -> Option<CorrectionResult> {
    if exit_code != 127 { return None; }

    let first_word = command_line.split_whitespace().next()?;
    let rest = &command_line[first_word.len()..];

    // 1. Check typo map (instant, high confidence)
    if let Some(&(_, correct)) = TYPO_MAP.iter().find(|(typo, _)| *typo == first_word) {
        return Some(CorrectionResult {
            original: command_line.to_string(),
            suggestion: format!("{}{}", correct, rest),
            confidence: 1.0,
        });
    }

    // 2. Damerau-Levenshtein against known commands (max distance 2)
    let mut best: Option<(&str, usize)> = None;
    for cmd in known_commands {
        let dist = damerau_levenshtein(first_word, cmd);
        if dist <= 2 && dist < best.map_or(usize::MAX, |b| b.1) {
            best = Some((cmd, dist));
        }
    }

    best.map(|(cmd, dist)| CorrectionResult {
        original: command_line.to_string(),
        suggestion: format!("{}{}", cmd, rest),
        confidence: 1.0 - (dist as f64 / first_word.len() as f64),
    })
}
```

**`crates/raijin-app/src/workspace.rs`** — Correction-Banner:
```rust
// In handle_shell_marker() for CommandEnd:
if exit_code == 127 {
    let known = self.shell_completion.path_executables.read().unwrap().clone();
    if let Some(correction) = command_correction::suggest_correction(&last_command, 127, &known) {
        self.show_correction_banner(correction, window, cx);
    }
}
```

Banner-UI: Inline-Notification unter dem Block-Header mit "Did you mean `<suggestion>`? Press Enter to run" — akzeptieren setzt den korrigierten Command in Input + submitted.

---

## 7. Shell Selector

**`crates/raijin-app/src/workspace.rs`** — Shell-Dropdown neben Context-Chips:

```rust
struct ShellOption {
    name: String,      // "zsh", "bash", "fish", "nu"
    path: PathBuf,     // /bin/zsh, /usr/local/bin/nu
    version: String,   // "5.9", "0.111.0"
}

fn detect_available_shells() -> Vec<ShellOption> {
    let candidates = [
        ("zsh", &["/bin/zsh", "/usr/bin/zsh"]),
        ("bash", &["/bin/bash", "/usr/bin/bash"]),
        ("fish", &["/usr/local/bin/fish", "/opt/homebrew/bin/fish"]),
        ("nu", &["/usr/local/bin/nu", "/opt/homebrew/bin/nu"]),
    ];
    // Also check `which nu`, `~/.cargo/bin/nu`
    // Run `<shell> --version` to get version string
}

fn switch_shell(&mut self, shell: &ShellOption, window: &mut Window, cx: &mut Context<Self>) {
    // 1. Terminate current PTY
    self.terminal.kill();
    // 2. Update ShellEditor language (bash → nu syntax highlighting)
    let lang = match shell.name.as_str() { "nu" => "nu", "fish" => "bash", _ => "bash" };
    self.input_state.update(cx, |s, _| s.set_shell_language(lang));
    // 3. Load command history for new shell
    self.command_history = CommandHistory::detect_and_load(&shell.name);
    // 4. Update completion provider
    self.shell_completion.update_shell(&shell.name);
    // 5. Spawn new PTY with new shell
    self.terminal = Terminal::spawn(shell.path.clone(), ...);
}
```

---

## Neue Dependencies

| Crate | Cargo.toml | Zweck |
|-------|-----------|-------|
| `rusqlite` | `raijin-app/Cargo.toml` | Nushell SQLite History lesen. Features: `bundled` |
| `strsim` | `raijin-app/Cargo.toml` | Damerau-Levenshtein für Typo-Korrektur |
| `which` | `raijin-app/Cargo.toml` | Shell-Binary detection (`which nu`) |
| `dirs` | `raijin-app/Cargo.toml` | Home/Config dir detection (histfile paths) |
| `tree-sitter-nu` | `inazuma-component/ui/Cargo.toml` | Nushell Syntax-Highlighting |
| `serde_json` | `raijin-completions/Cargo.toml` | CLI Spec JSON Parsing |
| `serde` | `raijin-completions/Cargo.toml` | Spec Deserialization |

## Workspace-Level

| Datei | Änderung |
|-------|----------|
| `Cargo.toml` (workspace root) | Neues Member: `crates/raijin-completions` |

---

## Vollständige Datei-Übersicht

### Neue Dateien
| Datei | Beschreibung |
|-------|-------------|
| `crates/inazuma-component/ui/src/input/auto_pairs.rs` | AutoPairConfig, should_auto_close, skip-over, pair-delete |
| `crates/raijin-app/src/command_history.rs` | CommandHistory, alle 5 Histfile-Parser, Frecency-Scoring, Fuzzy-Filter |
| `crates/raijin-app/src/history_panel.rs` | HistoryPanel UI — Overlay mit scrollbarer Entry-Liste, Selection, relative Zeit, Navigation-Hints |
| `crates/raijin-app/src/shell_completion.rs` | ShellCompletionProvider (CompletionProvider impl) |
| `crates/raijin-app/src/command_correction.rs` | Typo-Korrektur (Levenshtein + Typo-Map) |
| `crates/raijin-completions/` | Neues Crate: CLI Specs, Parser, Matcher |
| `shell/nushell/vendor/autoload/raijin.nu` | Nushell Shell-Integration-Hooks |

### Geänderte Dateien
| Datei | Änderung |
|-------|----------|
| `crates/inazuma-component/ui/src/input/mode.rs` | `ShellEditor` Variant |
| `crates/inazuma-component/ui/src/input/state.rs` | `shell_editor()` Builder, `auto_pairs` + `is_pasting` Felder, `HistoryUp`/`HistoryDown` Events, Tab-Key Completion Logic |
| `crates/inazuma-component/ui/src/input/movement.rs` | Up/Down emit HistoryUp/HistoryDown bei Boundary |
| `crates/inazuma-component/ui/src/input/mod.rs` | `mod auto_pairs;` Export |
| `crates/inazuma-component/ui/src/highlighter/languages.rs` | `Language::Nu` Variant |
| `crates/inazuma-component/ui/Cargo.toml` | tree-sitter-nu Dependency |
| `crates/raijin-terminal/src/pty.rs` | Nu-Detection, XDG_DATA_DIRS Injection |
| `crates/raijin-terminal/src/osc_parser.rs` | OSC 133;P (PromptKind) Parser |
| `crates/raijin-app/src/workspace.rs` | Shell Editor Mode, History wiring, Completion Provider, Correction Banner, Shell Selector |
| `crates/raijin-app/src/main.rs` | Neue Module registrieren |
| `crates/raijin-app/Cargo.toml` | Neue Dependencies |
| `Cargo.toml` (workspace) | `raijin-completions` Member |

---

## Deferred (NICHT in Phase 4)

- **Multi-Cursor** (Cmd+D, Alt+Click): Erfordert `Selection` → `Vec<Selection>` Refactor im gesamten InputState. Eigene Phase.
- **Vim Keybindings**: Normal/Insert/Visual Mode State Machine. Eigene Phase.
- **Structured Output Rendering** (Plan 12 Phase 2): OSC 7778, interaktive Tabellen. Eigene Phase.
- **nu_plugin_raijin** (Plan 12 Phase 3): Deep Integration via Nu Plugin-Protokoll. Eigene Phase.
- **PS1/Starship**: Bereits gelöst via Shell PS1 Mode — kein Input-Editor-Feature.

---

## Verifikation

```bash
cargo build --workspace              # Alles kompiliert
cargo test --workspace               # Alle Tests grün
cargo clippy --workspace             # Kein dbg!, kein todo!
cargo run -p raijin-app              # Manuell testen:
```

| Test | Erwartung |
|------|-----------|
| Bash-Highlighting | `if [ -f foo ]; then echo "bar"; fi` → Syntax-Farben sichtbar |
| Nu-Highlighting | In Nushell: `ls \| where size > 10mb` → Nu-Syntax-Farben |
| Multi-Line | Shift+Enter → Editor wächst, Soft-Wrap bei langen Zeilen |
| Auto-Pairs | `(` → `()`, `)` Skip-Over, Backspace Pair-Delete, kein Auto-Close bei Paste |
| History Panel | Up → Panel öffnet sich mit scrollbarer Command-Liste, `>_` Icon + Command + relative Zeit |
| History Navigation | Up/Down navigiert durch Panel-Entries, selektierter Command erscheint im Input mit Syntax-Highlighting |
| History Dismiss | Esc → Panel schließt, gespeicherter Input wird wiederhergestellt |
| History Execute | Enter bei offenem Panel → Command ausführen, Panel schließt |
| History Filter | Tippen bei offenem Panel → Fuzzy-Filter der History-Entries |
| Histfile Import | Nach Start sind alte Commands via Panel erreichbar (zsh/bash/fish/nu inkl. SQLite) |
| Tab Completion | `gi` Tab → `git`, `cd ~/Pr` Tab → `~/Projects/` |
| Git Completion | `git checkout ` Tab → Branch-Liste |
| Env Var Completion | `$HO` Tab → `$HOME` |
| Spec Completion | `git commit -` Tab → `--message`, `--amend`, `--no-edit` |
| Ghost Text | `cargo b` → grauer Ghost-Text `uild`, Tab akzeptiert |
| Nu Completions | In Nushell: `ls \| wh` Tab → `where` (via `nu --ide-complete`) |
| Nu History | Up/Down zeigt Nushell-Commands (aus SQLite geladen) |
| Typo-Korrektur | `gti status` → Exit 127 → Banner "Did you mean `git status`?" |
| Shell-Selector | Dropdown → nu auswählen → PTY + Highlighting + History wechseln |
