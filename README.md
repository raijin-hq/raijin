<p align="center">
  <img src=".github/assets/header.png" alt="Raijin" width="100%" />
</p>

<h1 align="center">雷神 Raijin</h1>

<p align="center">
  <strong>GPU-accelerated terminal reimagined as a developer workstation.</strong>
</p>

<p align="center">
  Block-based commands · Native Nushell structured output · 69 context chips · AI agent toolbar · OKLCH color system · 715+ CLI completions
</p>

<p align="center">
  <a href="#features">Features</a> ·
  <a href="#installation">Installation</a> ·
  <a href="#building-from-source">Build</a> ·
  <a href="#configuration">Configuration</a> ·
  <a href="#contributing">Contributing</a>
</p>

---

## What is Raijin?

Raijin is a terminal emulator built from scratch in Rust. Every command runs in its own block with a header, timing, and exit status — no more scrolling through walls of text. The shell prompt is replaced by native context chips that show git status, language versions, and environment info without Starship or any external tool.

The rendering engine is [Inazuma](crates/inazuma/) (稲妻), forked from GPUI, delivering 120fps Metal/wgpu rendering with sub-millisecond input latency.

## Features

### Block-Based Command Output
Every command gets its own collapsible block with a sticky header showing the command, execution time, and exit status. Error blocks are visually distinct. Navigate between blocks with keyboard shortcuts.

### Native Context Chips
69 built-in context providers replace Starship entirely — git branch & status, language versions (Node, Python, Rust, Go, and 21 more), DevOps contexts (Kubernetes, Docker, AWS, Terraform), environment info, and more. Zero config, works from first launch.

### First-Class Nushell Support
Raijin is the first terminal with native Nushell structured output rendering. Tables, lists, and records are GPU-rendered with type badges in block headers. Nu's native OSC 133 support means no hook injection is needed.

### Intelligent Completions
Built-in completion specs for 715+ CLIs including git, cargo, npm, docker, kubectl, and hundreds more. Fuzzy matching, descriptions, file paths, git branches — all without external plugins.

### AI Agent Toolbar
Auto-detects running AI agents (Claude, Codex, Gemini, Aider) and surfaces a native toolbar with file explorer, diff viewer, and MCP integration. Natural language to command translation with `#` prefix.

### OKLCH Color System
The only terminal using perceptually uniform OKLCH color space. 120+ semantic tokens, 12-step auto-generated color scales, Display P3 wide gamut support on Retina displays. Ships with carefully crafted themes.

### Native Directory Jumping
Built-in frecency-based directory jumping (like zoxide, but native). Records your navigation patterns from shell integration and offers instant fuzzy directory switching — no external tools needed.

### Cross-Platform
macOS (Metal), Linux (Wayland/X11 via wgpu), Windows (DirectX), and Web — all from the same Inazuma rendering engine.

## Installation

> Raijin is in active development. Pre-built binaries are coming soon.

### Building from Source

**Requirements:**
- Rust nightly toolchain (edition 2024)
- macOS 10.15.7+ (primary platform), Linux, or Windows

```bash
# Clone the repository
git clone https://github.com/raijin-hq/raijin.git
cd raijin

# Build
cargo build --release

# Run
cargo run -p raijin-app

# Or use the dev tooling for hot-reload
cargo raijin dev
```

### Dev Tooling

```bash
cargo raijin dev              # Hot-reload dev mode (watches src, rebuilds + relaunches)
cargo raijin dev --release    # Hot-reload in release mode
cargo raijin build            # Release build + .app bundle (macOS)
cargo raijin icon             # Compile .icon → Assets.car via actool
```

## Configuration

Raijin is configured via `~/.config/raijin/config.toml`:

```toml
[general]
working_directory = "~"
input_mode = "raijin"          # "raijin" (context chips) or "shell" (raw PS1)

[appearance]
theme = "raijin-dark"
font_family = "Berkeley Mono"
font_size = 14.0

[appearance.symbol_map]
"0xe5fa-0xe6b5" = "Symbols Nerd Font Mono"   # Nerd Font ranges

[terminal]
scrollback_history = 10000
cursor_style = "bar"           # "bar", "block", "underline"
```

## Architecture

```
raijin-app          → Binary: workspace layout, terminal rendering, block UI
├── inazuma         → GPU UI framework (forked from GPUI, Metal/wgpu)
├── inazuma-component → 70+ UI components (input, tabs, chips, title bar, …)
├── raijin-terminal → PTY wrapper, OSC 133 parser, BlockManager
├── raijin-term     → Terminal emulation core (Grid-per-Block architecture)
├── raijin-shell    → Shell context: CWD, git branch, user info
├── raijin-settings → User config (TOML) with live reload
├── raijin-completions → 715+ CLI completion specs
└── cargo-raijin    → Dev tooling (cargo raijin dev/build/icon)
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## Security

See [SECURITY.md](SECURITY.md) for reporting vulnerabilities.

## License

License is being determined. This repository is currently private.
