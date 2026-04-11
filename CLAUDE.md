# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run Commands

```bash
cargo build                           # Build raijin-app (default member)
cargo run -p raijin-app               # Run the terminal app
cargo raijin dev                      # Hot-reload dev mode (watches src, rebuilds + relaunches)
cargo raijin dev --release            # Hot-reload in release mode
cargo raijin build                    # Release build + .app bundle
cargo raijin build --debug            # Debug build + .app bundle
cargo raijin icon                     # Compile .icon → Assets.car via actool
cargo test -p raijin-terminal         # Run terminal tests (OSC parser, blocks)
cargo test -p inazuma-macros          # Run framework macro tests
cargo test --workspace                # All tests
cargo clippy --workspace              # Lint (dbg! and todo! are denied)
```

Requires **Rust stable 1.94+** (edition 2024, resolver 3). Pinned via `rust-toolchain.toml`. macOS is the primary platform (Metal rendering).

`.cargo/config.toml` sets: `symbol-mangling-version=v0` rustflag, `MACOSX_DEPLOYMENT_TARGET=10.15.7`, and the `cargo raijin` alias.

## Architecture

**Raijin (雷神)** — GPU-accelerated terminal emulator built on the Inazuma (稲妻) UI framework.

### Crate Dependency Graph

```
raijin-app (binary — entry point, workspace layout, terminal rendering)
├── inazuma (GPU UI framework, forked from gpui-ce)
│   └── inazuma-macros (proc-macros: derive Actions, elements, etc.)
├── inazuma-component (70+ UI components: input, chips, title_bar, tabs, etc.)
│   ├── inazuma-component-macros (proc-macros: icon_named!, IntoPlot derive)
│   └── inazuma-component-assets (bundled fonts/icons/SVGs)
├── raijin-terminal (PTY + alacritty_terminal wrapper + OSC 133 parser + block system)
├── raijin-term (low-level terminal emulation core — standalone fork of alacritty_terminal with BlockGrid)
├── raijin-shell (shell context: CWD, git branch, user info)
├── raijin-settings (user config at ~/.config/raijin/config.toml — theme, font, cursor, scrollback, symbol_map)
├── raijin-completions (spec-based CLI completion engine — JSON specs for git, cargo, etc.)
├── raijin-assets (compile-time asset bundling via rust-embed — themes, fonts, keymaps; falls back to inazuma-component-assets)
├── raijin-theme (theme system — ThemeRegistry, ThemeColors, ThemeStyles, OKLCH color pipeline)
│   ├── raijin-theme-settings (connects themes to raijin-settings — ThemeSelection, reload_theme)
│   ├── raijin-theme-extension (dynamic theme loading — ExtensionThemeProxy)
│   ├── raijin-theme-selector (UI picker for browsing/switching themes)
│   └── raijin-theme-importer (imports Zed and VS Code themes)
├── inazuma-fuzzy (fuzzy matching engine — CharBag, match_strings)
├── inazuma-util (general utilities — fs, paths, shell, markdown, etc.)
├── inazuma-collections (FxHashMap/FxHashSet aliases, VecMap)
├── inazuma-gpui-util (GPUI helpers — post_inc, measure, ArcCow)
├── inazuma-util-macros (proc-macros — path! for cross-platform paths)
├── inazuma-perf (perf profiler data types)
└── cargo-raijin (dev tooling binary: cargo raijin dev/build/icon — not a library)
```

### Key Subsystems

**Inazuma (稲妻)** — The GPU UI framework. ~90 modules covering app lifecycle, element system, Metal/wgpu rendering, text shaping, layout (taffy), and platform abstraction. Modify inazuma directly when it's cleaner than working around it in raijin-app.

**Terminal Backend** (`raijin-terminal`) — Wraps `alacritty_terminal::Term` for grid state. PTY spawning in `pty.rs` injects shell hooks via `ZDOTDIR` manipulation. The `osc_parser.rs` scans PTY byte streams for OSC 133 (FTCS) shell integration markers. `block.rs` provides `BlockManager` which tracks command blocks (prompt→input→output→exit code).

**Terminal Core** (`raijin-term`) — Lower-level terminal emulation: VT state machine, grid storage, `BlockGrid` (per-command grids with independent cursors/scroll regions), PTY abstraction via `rustix-openpty`. Being developed as a more complete replacement for the alacritty_terminal dependency.

**Shell Hooks** (`shell/raijin.{zsh,bash,fish}`, `shell/nushell/`) — Injected into the spawned shell to emit OSC 133 markers (PromptStart, InputStart, CommandStart, CommandEnd) and OSC 7777 JSON metadata (hex-encoded). Zsh uses `ZDOTDIR` injection, Bash uses `--rcfile`. Nushell has dedicated integration in `shell/nushell/`.

**Workspace** (`raijin-app/src/workspace.rs`) — Warp-style 3-zone layout: tab bar (top), terminal output with block headers (middle), input bar with context chips (bottom). Two input modes: Raijin Mode (custom input + context chips) and Shell PS1 Mode (raw shell prompt).

**Terminal Element** (`raijin-app/src/terminal_element.rs`) — Custom Inazuma element that renders the alacritty grid cell-by-cell with ANSI color mapping, block headers (command + duration + exit badge), cursor, and content masking.

**Theme System** (`raijin-theme`) — Full theme pipeline: TOML theme definitions in `assets/themes/`, loaded into `ThemeRegistry` at startup. `GlobalTheme` provides app-wide access. All colors are token-based via `ThemeColors`/`ThemeStyles` — no hardcoded colors. Supports importing Zed and VS Code themes.

**Settings** (`raijin-settings`) — `RaijinConfig` implements `inazuma::Global` for app-wide access. Config sections: `GeneralConfig` (working_directory, input_mode), `AppearanceConfig` (theme, font_family, font_size, symbol_map for Nerd Font ranges), `TerminalConfig` (scrollback_history, cursor_style).

**Completions** (`raijin-completions`) — Parses the user's current input line into `CommandContext` + `TokenPosition`, matches against embedded JSON specs (`specs/git.json`, `specs/cargo.json`), returns `CompletionCandidate`s. Supports file paths, git branches/tags/remotes, env vars, process IDs.

### Theme

Raijin Dark: `#121212` background, `#00BFFF` accent (Cyan), `#f1f1f1` foreground. Theme definitions live in `assets/themes/` as TOML files, loaded through `raijin-theme::ThemeRegistry`.

## Terminal vs Editor Rendering

Strikte Trennung zwischen Terminal-Code und Editor/UI-Code:

- **Terminal Output** (grid rendering, PTY, cells): Immer wie echte Terminals bauen — **Rio, Alacritty, Kitty, Ghostty** als Referenz. Per-cell Rendering, Grid-Positionierung auf `col * cell_width`, kein per-line Text-Shaping, kein `force_width`. Box-Drawing via `builtin_font.rs` (GPU-Primitive), Emoji via `paint_emoji` mit CoreText Font-Fallback. Bei Unsicherheit: Rio-Code in `.reference/rio` prüfen.
- **Editor/UI Features** (Code-Editor, Text-Input, Completions, Panels, Settings): Inazuma's Text-System (`ShapedLine`, `shape_line`, `TextRun`). Das ist wofür das Framework gebaut wurde.

Inazuma ist ein **Editor-Framework**. Terminal-Rendering hat fundamental andere Anforderungen (festes Grid, per-cell Positionierung, Unicode-Width, Emoji, Box-Drawing). Editor-Patterns nicht auf Terminal-Rendering anwenden.

## Conventions

- **Rust edition 2024** (stable 1.94+) with `resolver = "3"`
- **No `mod.rs`** — use `module_name.rs` (modern Rust convention)
- **No stubs or placeholders** — every feature must be production-complete, no `todo!()`, no `unimplemented!()`, no silent error swallowing
- **Clippy lints**: `dbg_macro` and `todo` are denied; `style`, `type_complexity`, `too_many_arguments`, `large_enum_variant` are allowed
- **macOS platform code** uses `objc2` + `objc2-app-kit` + `objc2-foundation` — NOT the old `cocoa`/`objc` crates. Never add `cocoa` or `objc` as dependency.
- Naming: the framework is called **Inazuma**, not GPUI — all imports use `inazuma::`

## Crate Architecture Rules

These rules apply to **every crate in the entire repo**.

### The 3 Crate Types

Every crate in the repo falls into exactly one of these categories:

**1. Shared Infrastructure** — Backend logic used by multiple feature crates.
- Contains: state management, events, traits, data models, protocols, algorithms
- Does NOT contain: UI rendering, Workspace integration (`impl Item`, `impl Panel`)
- Rule: If >1 feature crate imports this logic, it belongs here
- Examples: `raijin-terminal`, `raijin-project`, `raijin-editor`, `raijin-git`, `raijin-lsp`, `raijin-completions`, `raijin-session`, `raijin-shell`, `raijin-task`

**2. Feature Crates** — Self-contained features with logic + UI together.
- Contains: feature-specific logic + settings + UI rendering + Workspace traits (`impl Item`, `impl Panel`, `impl Render`)
- Feature-specific logic that ONLY this feature needs lives HERE — not in a separate backend crate
- These are NOT "thin wrappers" — they can be large (`raijin-project-panel` is 18k lines, `raijin-search` is 9.6k, `raijin-terminal-view` is 9.5k)
- Imports from Shared Infrastructure + Framework
- Examples: `raijin-terminal-view`, `raijin-project-panel`, `raijin-search`, `raijin-file-finder`, `raijin-diagnostics`, `raijin-debugger-ui`, `raijin-git-ui`, `raijin-agent-ui`, `raijin-settings-ui`

**3. Framework Primitives** — Reusable building blocks with zero app knowledge.
- Contains: generic data structures, UI primitives, rendering engine, settings framework
- Knows NOTHING about Raijin, terminals, editors, or any app features
- Examples: `inazuma`, `inazuma-collections`, `inazuma-text`, `inazuma-rope`, `raijin-ui`, `inazuma-picker`, `inazuma-menu`, `inazuma-settings-framework`

**Entry Point:** `raijin-app` is ONLY `main.rs` + bootstrap. It imports and initializes crates. Zero logic, zero rendering, zero UI.

### Decision Flowchart: "Where does this code go?"

```
Is it a generic library with no app knowledge?
  YES → Framework Primitive (inazuma-*)
  NO  ↓

Is this logic needed by >1 feature crate?
  YES → Shared Infrastructure (raijin-terminal, raijin-project, etc.)
  NO  ↓

→ Feature Crate (raijin-terminal-view, raijin-search, etc.)
```

### Before writing code

Before adding code anywhere:
1. We have 220+ crates — check `crates/` directory first
2. Check how the reference codebase organizes the same thing in `.reference/`
3. **Never create a backend crate that only one feature uses** — put that logic in the feature crate instead
4. **Never put code in `raijin-app`** — find or create the proper crate

### Naming convention

- **`inazuma-*`** = Framework-level, reusable independent of Raijin (collections, text, rope, fuzzy, settings framework, UI primitives, GPU rendering)
- **`raijin-*`** = Application-level, Raijin-specific features (terminal, theme, workspace, editor, agent, collab, project)

### Dependency rules

```
raijin-app (entry point — imports everything, contains nothing)
  ├── Feature Crates (raijin-terminal-view, raijin-search, etc.)
  │     ├── Shared Infrastructure (raijin-terminal, raijin-project, etc.)
  │     ├── raijin-workspace (workspace framework)
  │     └── raijin-ui / inazuma-component (UI components)
  └── Framework Primitives (inazuma, inazuma-collections, etc.)
```

- Shared Infrastructure NEVER depends on Feature Crates
- Feature Crates NEVER depend on `raijin-app`
- Framework Primitives (`inazuma-*`) NEVER depend on application crates (`raijin-*`)
- No circular dependencies — extract shared parts into a third crate

### Reference architecture

When in doubt, check `.reference/` for how the reference codebase organizes the same functionality. Our architecture follows the same patterns with these key differences:
- Naming (framework = `inazuma`, app = `raijin`)
- Colors (OKLCH instead of HSLA)
- Settings format (TOML instead of JSON)
- Terminal (our own Block system with Warp-style UX)
- Platform code (`objc2` instead of `cocoa`/`objc`)

## Project Phases

Roadmap lives in `plan/` (00–18). Completed: Phase 0 (foundation), Phase 1 (minimal terminal), Phase 2A/2B (shell integration + block system), Phase 4 (input editor). In progress: Phase 2C (block interaction), Phase 3 (design system). Completed plans are in `plan/done/`.
