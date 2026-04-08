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

Requires **Rust nightly** (edition 2024, resolver 3). No `rust-toolchain.toml` — install nightly manually. macOS is the primary platform (Metal rendering).

`.cargo/config.toml` sets: `symbol-mangling-version=v0` rustflag, `MACOSX_DEPLOYMENT_TARGET=10.15.7`, and the `cargo raijin` alias.

## Architecture

**Raijin (雷神)** — GPU-accelerated terminal emulator built on a vendored fork of Zed's GPUI framework.

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

**Inazuma (稲妻)** — The UI framework. A vendored fork of Zed's GPUI, rebranded. ~90 modules covering app lifecycle, element system, Metal/wgpu rendering, text shaping, layout (taffy), and platform abstraction. Modify inazuma directly when it's cleaner than working around it in raijin-app.

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

Inazuma/GPUI ist ein **Editor-Framework**. Terminal-Rendering hat fundamental andere Anforderungen (festes Grid, per-cell Positionierung, Unicode-Width, Emoji, Box-Drawing). Zed-Patterns nicht auf Terminal-Rendering anwenden.

## Conventions

- **Rust edition 2024** (nightly) with `resolver = "3"`
- **No `mod.rs`** — use `module_name.rs` (modern Rust convention)
- **No stubs or placeholders** — every feature must be production-complete, no `todo!()`, no `unimplemented!()`, no silent error swallowing
- **Clippy lints**: `dbg_macro` and `todo` are denied; `style`, `type_complexity`, `too_many_arguments`, `large_enum_variant` are allowed
- **macOS platform code** uses `objc2` + `objc2-app-kit` + `objc2-foundation` — NOT the old `cocoa`/`objc` crates. Never add `cocoa` or `objc` as dependency.
- Naming: the framework is called **Inazuma**, not GPUI — all imports use `inazuma::`

## Crate Architecture Rules

**Every piece of logic must live in its own dedicated crate, not in `raijin-app`.** `raijin-app` is ONLY the entry point (`main.rs`) and app bootstrap. It imports and wires up crates — it contains no business logic, no rendering code, no UI components.

This mirrors Zed's architecture where the `zed` crate is thin and everything lives in dedicated crates.

### Where code lives:

| Code Type | Crate | NOT in |
|---|---|---|
| Terminal backend (PTY, events, state) | `raijin-terminal` | raijin-app |
| Terminal rendering (grid, blocks, colors) | `raijin-terminal-view` | raijin-app |
| Terminal panel (Item trait, Workspace integration) | `raijin-terminal-view` | raijin-app |
| Shell completions | `raijin-completions` | raijin-app |
| Command history, session state | `raijin-session` | raijin-app |
| Settings UI | `raijin-settings-ui` | raijin-app |
| Theme system | `raijin-theme` + `raijin-theme-settings` | raijin-app |
| UI components | `inazuma-component` or `raijin-ui` | raijin-app |
| Workspace layout | `raijin-workspace` | raijin-app |
| Project model (buffers, LSP, git) | `raijin-project` | raijin-app |

### View crates are thin wrappers:

A `*-view` crate (like `raijin-terminal-view`) does NOT contain business logic. It only:
1. Imports types from backend crates (`raijin-terminal`, `raijin-completions`, etc.)
2. Implements Workspace traits (`Item`, `Panel`, `Render`)
3. Wires up event handling between crates

If you find yourself writing logic in a view crate, it belongs in the backend crate instead.

### Inazuma vs Raijin naming:

- **`inazuma-*`** = Framework-level, reusable without Raijin (collections, text, rope, fuzzy, settings, UI primitives)
- **`raijin-*`** = Application-level, Raijin-specific (terminal, theme, workspace, editor, agent)

### Dependencies flow downward:

```
raijin-app (entry point only)
  → raijin-terminal-view (thin view)
    → raijin-terminal (backend)
    → raijin-completions (backend)
    → raijin-workspace (workspace framework)
    → raijin-ui (UI components)
      → inazuma (GPU framework)
```

Never create circular dependencies. Backend crates must not depend on view crates.

## Project Phases

Roadmap lives in `plan/` (00–18). Completed: Phase 0 (foundation), Phase 1 (minimal terminal), Phase 2A/2B (shell integration + block system), Phase 4 (input editor). In progress: Phase 2C (block interaction), Phase 3 (design system). Completed plans are in `plan/done/`.
