<p align="center">
  <img src=".github/assets/header.png" alt="Raijin" width="100%" />
</p>

<h1 align="center">雷神 Raijin</h1>

<p align="center">
  <strong>The terminal-first agentic development environment.</strong><br />
  Built from scratch in Rust on a custom GPU UI framework.
</p>

<p align="center">
  <em>Blocks for every command. Native Nushell. TUI-aware. 715 CLI completions built-in.<br />
  Zero prompt plugins, zero shell plugins, zero external dependencies — it just works.</em>
</p>

<p align="center">
  <a href="#why-raijin">Why</a> ·
  <a href="#what-makes-it-different">What makes it different</a> ·
  <a href="#feature-matrix">Feature matrix</a> ·
  <a href="#install">Install</a> ·
  <a href="#configure">Configure</a> ·
  <a href="#architecture">Architecture</a>
</p>

---

## Why Raijin

Modern terminals force a choice. Either you run an emulator that is fast and faithful but looks like 1995, or you run something with a block UI and lose raw speed, scrollback integrity, and the ability to run the tools you actually use. Raijin refuses that trade.

Raijin is a **terminal-first Agentic Development Environment (ADE)**. Every session starts as a real terminal with a real PTY. The block system, structured-output rendering, context chips, completions, and agent integrations all sit on top of a conformant VT state machine — not around it. Panels (file tree, agent, git, debug, outline, collab, notifications) are registered but hidden by default; you open what you need when you need it.

The rendering engine is **[Inazuma](crates/inazuma/) (稲妻)** — our own GPU UI framework with native Metal on macOS, Vulkan/DX12 via WGPU on Linux and Windows. Sub-millisecond input latency, 120 fps compositing, OKLCH color throughout, Display P3 wide gamut on capable displays.

## What makes it different

### Per-block terminal grids

Every command gets its own `BlockGrid` with its own cursor, scroll region, and state. No other terminal on the market does this. It is why Raijin can give you:

- command-accurate scrollback per block, not one flat buffer
- finalized blocks that stay intact when later commands repaint
- search that targets one block or all blocks with the same primitive
- folding, exit-status badges, timing, and structured-output rendering without heuristics layered over a flat stream

### TUI-aware block system (in development — world first)

Interactive TUIs that run in non-alt-screen mode — Ink-based CLIs, log-update pipelines, modern AI agent UIs — routinely stack their banners on top of each other inside other terminals because scrollback grows underneath the redraw. Raijin fixes this at three levels at once:

1. **DEC Private Mode 2026 (Synchronized Output)** — real BSU/ESU handling, damage batching, scrollback suppression for atomic redraws.
2. **Shell-level TUI hints** — the first terminal emulator to ship a shell preexec hook that tells the terminal "this is a TUI" before the first byte of output arrives. Zero heuristic lag.
3. **Cursor-up heuristic** — safety net for legacy TUIs that neither emit DEC 2026 nor get detected by the known-TUI list.

When a TUI finalizes, the **last frame is preserved** in scrollback as a single snapshot — not five hundred intermediate repaints. Every block stays scrollable, searchable, and clean.

### First-class Nushell

Raijin is the **first terminal with full native Nushell integration**. Tables, lists, and records render with type badges directly in the block header. Nushell's native OSC 133 support means **zero hook injection** — no ZDOTDIR trickery, no trap wrappers, no rc-file fragility. The `pre_execution` hook ships OSC 7777 metadata on every prompt: CWD, git branch and dirtiness, username, hostname, command duration. Structured output stays structured all the way to the GPU.

### Built-in context chips — goodbye, prompt plugins

**97 native context providers** ship in the box. Git branch, git status, git commit, git state, seven version-control systems. Language runtime detection for Node, Python, Rust, Go, Java, Kotlin, Scala, Ruby, PHP, Haskell, Elixir, Erlang, Nim, Zig, OCaml, Dart, Crystal, Deno, Bun, Swift, C, C++, and more. Cloud contexts for AWS, Azure, GCloud, Kubernetes, Docker, Terraform, Pulumi, Helm, Nix, OpenStack. System chips for battery, memory, jobs, shell level, hostname, localip, OS. All parallelized across Rayon, per-provider timeout-protected, none of them need an external binary to be installed.

No Starship. No Oh-My-Zsh. No Powerlevel10k. Ships empty, boots useful.

### 715 CLI completions — embedded, not downloaded

**715 JSON completion specs** compiled into the binary. Git, cargo, npm, pnpm, yarn, docker, kubectl, helm, aws, gcloud, terraform, ansible, brew, apt, systemctl, ssh, rsync, curl, ffmpeg — and 700 more. Fuzzy matching, descriptions, file paths, git branches and tags and remotes, env vars, process IDs. Zero plugins, zero setup.

### AI agent toolbar

Raijin auto-detects running AI agents — Claude Code, Codex, Gemini, Aider, Cline — and surfaces a native toolbar with file explorer, inline diff viewer, and MCP integration. Natural-language-to-command translation via `#` prefix (`# find all rust files modified in the last week`). The agent panel is first-class, not a plugin.

### Native directory jumping

Frecency-based directory jumping built into the terminal, driven by the shell integration's own history. Fuzzy directory switching without installing zoxide or autojump or z.lua.

### OKLCH color system

The only terminal using perceptually uniform OKLCH color space throughout. 120+ semantic tokens, 12-step auto-generated color scales, Display P3 wide-gamut support on Retina displays. Ships with seven themes — Raijin Dark, Raijin Light, Catppuccin Mocha, Dracula, Gruvbox Dark, Nord, One Dark — all OKLCH-derived.

### Single-item panes, session tabs

Each tab in the title bar is a **session**, not a pane. Each pane holds exactly one item — terminal or editor. Splits create new panes inside the session. File drops land in an editor pane without replacing your terminal. The last pane closed closes the session; the last session closed closes the window. No more twelve-pane accidents from a stray `Cmd+D`.

### Cross-platform by construction

One rendering engine, three backends. Metal on macOS, Vulkan/DX12 on Linux and Windows, WGPU under the hood. Platform code uses the modern `objc2` toolkit on macOS — no legacy Cocoa bindings anywhere.

## Feature matrix

| Capability | Raijin | Classic emulators | Block-style terminals |
|---|---|---|---|
| Per-command block grids | **yes, per-command GPU grid** | no | approximated via parsing |
| Native Nushell structured output | **yes, GPU-rendered** | no | no |
| TUI-aware rendering (DEC 2026 + shell hint + heuristic) | **yes** | partial or none | no |
| Context chips without Starship | **97 built-in** | no | no |
| CLI completions without plugins | **715 specs embedded** | external | limited |
| AI agent integration | **first-class panel + toolbar** | no | partial |
| Directory jumping without zoxide | **built-in, frecency** | no | no |
| OKLCH color pipeline | **yes, end-to-end** | no | no |
| Own GPU UI framework | **yes (Inazuma)** | no | varies |

## Install

> Raijin is in active development. Pre-built binaries ship soon.

```bash
git clone https://github.com/raijin-hq/raijin.git
cd raijin
cargo run -p raijin-app           # run it
cargo raijin dev                  # hot-reload dev loop
cargo raijin build                # release .app bundle (macOS)
cargo raijin build --debug        # debug .app bundle
cargo raijin icon                 # compile .icon → Assets.car
```

Requires Rust stable 1.94+ (pinned via `rust-toolchain.toml`, edition 2024, resolver 3). macOS 10.15.7 and later, Linux (Wayland or X11), Windows 10 and later.

## Configure

Config lives at `~/.config/raijin/config.toml`. TOML, not JSON. Hot-reloaded.

```toml
[general]
working_directory = "~"
input_mode = "raijin"          # "raijin" (native chips) or "shell" (raw PS1)

[appearance]
theme = "raijin-dark"          # raijin-dark, raijin-light, catppuccin-mocha,
                               # dracula, gruvbox-dark, nord, one-dark
font_family = "JetBrainsMono Nerd Font"
font_size = 14.0
window_opacity = 100           # 1–100, enables OS-level transparency < 100

[appearance.symbol_map]
"0xe5fa-0xe6b5" = "Symbols Nerd Font Mono"   # Nerd Font glyph ranges

[terminal]
scrollback_history = 10000
cursor_style = "bar"           # "bar", "block", "underline"
tui_awareness = "full"         # "full", "strict_protocol", "off"
```

## Architecture

```
raijin-app           → Binary: main.rs + bootstrap only, zero logic
├── inazuma          → GPU UI framework (Metal / WGPU), 90+ modules
├── inazuma-component → 70+ UI primitives: input, tabs, chips, title bar, toolbar
├── raijin-workspace → Sessions (tabs), panes (single-item), docks, toolbars
├── raijin-shell     → Window lifecycle (AppShell), open / close / reload
├── raijin-terminal  → PTY, OSC 133 + OSC 7777 parser, block manager
├── raijin-term      → Terminal emulation core, BlockGrid-per-command
├── raijin-terminal-view → Block rendering, grid element, search, folds
├── raijin-chips     → 97 native context providers (parallel, Rayon-backed)
├── raijin-completions → 715 CLI specs, fuzzy matcher, path / git resolvers
├── raijin-shell-integration → Shell context, known-TUI list, OSC 7777 payloads
├── raijin-settings  → TOML config, live reload, global registry
├── raijin-theme     → OKLCH pipeline, 7 themes, wide-gamut
└── cargo-raijin     → Dev tooling (cargo raijin dev/build/icon)
```

235 crates total. 213 `raijin-*` feature and infrastructure crates, 22 `inazuma-*` framework primitives. Strict 4-layer architecture (framework → UI → workspace → shell → bootstrap); dependencies flow downward only.

Shell integration ships in-tree: `shell/raijin.zsh`, `shell/raijin.bash`, `shell/raijin.fish`, and `shell/nushell/vendor/autoload/raijin.nu`. Zsh injects via `ZDOTDIR`, bash via `--rcfile`, fish via `conf.d`. Nushell plugs into its `pre_prompt` hook natively because Nu already emits OSC 133 — no hooks to inject.

## Shell integration protocol

Raijin speaks a small, documented protocol. Any shell can implement it.

- **OSC 133 (FTCS)** — `A` = PromptStart, `B` = InputStart, `C` = CommandStart, `D;<exit>` = CommandEnd. Used for block boundary detection.
- **OSC 7777 (Raijin metadata)** — hex-encoded JSON payloads. Prefixes:
  - `raijin-precmd` — CWD, git, user, host, duration, shell
  - `raijin-tui-hint` — `{"tui_mode": true}` before a TUI command runs
  - agent lifecycle events (Claude, Codex, Gemini, Aider)

Hex encoding prevents `0x9C` bytes (the ST terminator, which appears in emoji sequences) from breaking the escape. The payload is `#[serde(default)]`, so new fields can be added without breaking shells that do not yet emit them.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Every feature must be production-complete — no `todo!()`, no stubs, no silent swallowing. Comments are English, chat with maintainers is German or English.

## Security

See [SECURITY.md](SECURITY.md) for responsible disclosure.

## License

License is being determined. This repository is currently private.

---

<p align="center">
  <em>Raijin · 雷神 · the thunder god. Because your terminal should strike.</em>
</p>
