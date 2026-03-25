# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run Commands

```bash
cargo build                           # Build raijin-app (default member)
cargo build -p raijin-app             # Explicit app build
cargo run -p raijin-app               # Run the terminal app
cargo build -p raijin-terminal        # Build terminal crate only
cargo build -p inazuma                # Build framework only
cargo test -p raijin-terminal         # Run terminal tests (OSC parser, blocks)
cargo test -p inazuma-macros          # Run framework macro tests
cargo test --workspace                # All tests
cargo clippy --workspace              # Lint (dbg! and todo! are denied)
./crates/raijin-app/bundle.sh         # Create macOS .app bundle
```

Requires **Rust nightly** (edition 2024, resolver 3). macOS is the primary platform (Metal rendering).

## Architecture

**Raijin (雷神)** — GPU-accelerated terminal emulator built on a vendored fork of Zed's GPUI framework.

### Crate Dependency Graph

```
raijin-app (binary — entry point, workspace layout, terminal rendering)
├── inazuma (GPU UI framework, forked from gpui-ce)
│   └── inazuma-macros (proc-macros)
├── inazuma-component (70+ UI components: input, chips, title_bar, tabs, etc.)
│   └── inazuma-component-assets (bundled fonts/icons)
├── raijin-terminal (PTY + alacritty_terminal wrapper + OSC 133 parser + block system)
├── raijin-shell (shell context: CWD, git branch, user info)
└── raijin-ui (design token system — WIP, currently empty)
```

### Key Subsystems

**Inazuma (稲妻)** — The UI framework. A vendored fork of Zed's GPUI, rebranded. ~90 modules covering app lifecycle, element system, Metal/wgpu rendering, text shaping, layout (taffy), and platform abstraction. Modify inazuma directly when it's cleaner than working around it in raijin-app.

**Terminal Backend** (`raijin-terminal`) — Wraps `alacritty_terminal::Term` for grid state. PTY spawning in `pty.rs` injects shell hooks via `ZDOTDIR` manipulation. The `osc_parser.rs` scans PTY byte streams for OSC 133 (FTCS) shell integration markers. `block.rs` provides `BlockManager` which tracks command blocks (prompt→input→output→exit code).

**Shell Hooks** (`shell/raijin.{zsh,bash,fish}`) — Injected into the spawned shell to emit OSC 133 markers (PromptStart, InputStart, CommandStart, CommandEnd with exit code).

**Workspace** (`raijin-app/src/workspace.rs`) — Warp-style 3-zone layout: tab bar (top), terminal output with block headers (middle), input bar with context chips (bottom). Two input modes: Raijin Mode (custom input + context chips) and Shell PS1 Mode (raw shell prompt).

**Terminal Element** (`raijin-app/src/terminal_element.rs`) — Custom Inazuma element that renders the alacritty grid cell-by-cell with ANSI color mapping, block headers (command + duration + exit badge), cursor, and content masking.

### Theme

Raijin Dark: `#121212` background, `#14F195` accent (Solana green), `#f1f1f1` foreground. Colors are currently hardcoded in `terminal_element.rs` — the `raijin-ui` crate will house typed tokens once implemented.

## Conventions

- **Rust edition 2024** (nightly) with `resolver = "3"`
- **No `mod.rs`** — use `module_name.rs` (modern Rust convention)
- **No stubs or placeholders** — every feature must be production-complete, no `todo!()`, no `unimplemented!()`, no silent error swallowing
- **Clippy lints**: `dbg_macro` and `todo` are denied; `style`, `type_complexity`, `too_many_arguments`, `large_enum_variant` are allowed
- **macOS platform code** uses `cocoa 0.26` + `objc 0.2` (migration to `objc2` is planned, see `plan/10-INAZUMA-OBJC2-MIGRATION.md`)
- Naming: the framework is called **Inazuma**, not GPUI — all imports use `inazuma::`

## Project Phases

Roadmap lives in `plan/` (00–10). Current status: Phase 2A (Shell Integration + Block System) is in progress. Phases 0 (foundation) and 1 (minimal terminal) are complete.
