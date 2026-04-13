# Raijin (雷神) — Projekt-Überblick

> **Name:** Raijin (雷神) — Der Donnergott unter den Terminals
> **CLI Command:** `raijin`
> **Repo:** `nyxb/raijin`
> **Framework:** Inazuma (稲妻) — geforkt von Zed's GPUI
> **Ziel:** GPU-beschleunigter Terminal-Emulator mit Warp-Level UX & Design
> **Stack:** Rust + Inazuma + alacritty_terminal + Metal

---

## Tech-Stack

| Komponente | Technologie |
|---|---|
| UI Framework | Inazuma (稲妻) — GPU-UI-Framework (geforkt von Zed's GPUI) |
| UI Components | inazuma-component (70+ Widgets) |
| GPU Rendering | Metal (macOS) |
| Terminal Emulation | alacritty_terminal + raijin-term |
| VTE Parser | vte (in alacritty_terminal) |
| PTY | portable-pty |
| Text Shaping | Inazuma text system (cosmic-text based) |
| Layout Engine | Taffy (in Inazuma) |
| Config | TOML (raijin-settings) |
| Shell Integration | OSC 133 (blocks) + OSC 7777 (metadata) |
| Completions | raijin-completions (CLI specs) + ShellCompletionProvider |

---

## Crate-Struktur

```
raijin/
├── Cargo.toml                  # Workspace root (Rust nightly, edition 2024)
├── crates/
│   ├── inazuma/                # GPU UI Framework (forked from Zed's GPUI)
│   │   └── tooling/macros/     # Proc-Macros
│   ├── inazuma-component/      # 70+ UI Components (Input, Chips, TitleBar, Tabs)
│   │   ├── ui/
│   │   ├── macros/
│   │   └── assets/             # Bundled Fonts/Icons
│   ├── raijin-app/             # Binary — Workspace, Terminal Rendering
│   ├── raijin-terminal/        # PTY + alacritty_terminal + OSC Parser + Blocks
│   ├── raijin-term/            # Low-level terminal emulation core (BlockGrid)
│   ├── raijin-shell/           # Shell Context, Metadata Payload
│   ├── raijin-settings/        # Config System (TOML)
│   ├── raijin-completions/     # CLI spec-based completion engine
│   ├── raijin-ui/              # Design Token System (WIP)
│   └── cargo-raijin/           # Dev CLI (dev/build/icon commands)
├── shell/
│   ├── raijin.zsh              # Shell Hooks (OSC 133 + OSC 7777 metadata)
│   ├── raijin.bash
│   ├── raijin.fish
│   └── nushell/                # Nushell integration
├── .reference/zed/             # Reference codebase (shallow clone, gitignored)
└── plan/                       # Roadmap & Architecture Plans
```

---

## Phase-Status

| Phase | Status | Beschreibung |
|---|---|---|
| Phase 0: Foundation | ✅ Done | Repo, Toolchain, Inazuma vendored |
| Phase 1: Minimal Terminal | ✅ Done | PTY, Grid-Rendering, Input, ANSI Colors |
| Phase 2A: Shell Integration | ✅ Done | OSC 133 Hooks, BlockManager, OSC 7777 Metadata |
| Phase 2B: Block Rendering | ✅ Done | Warp-style Headers, Prompt Suppression, Error Styling |
| Phase 2C: Block Interaction | ⬜ Planned | Copy, Collapse, Navigation, Sticky Headers → Plan 04 |
| Phase 3: Design System | ⬜ Planned | Theme Tokens, Animations → Plan 05 |
| Phase 4: Input Editor | ✅ Done | Completions, History, Shell Selector, Nushell, 715 CLI Specs → Plan 06 |
| Phase 5: Explorer + Editor | ⬜ Planned | File Tree, Code Editor → Plan 07 |
| Phase 6: AI + Agents | ⬜ Planned | Agent Detection, AI Features → Plan 08 |
| Phase 7–9: Future | ⬜ Planned | Drive, Workflows, Distribution → Plan 09 |
| Multi-Tab Sessions | ⬜ Planned | Tab-Management, Session-Persistence → Plan 11 |

---

## Nächste Schritte

1. **Plan 04: Block Interaction** — Copy, Collapse, Navigation, Search
2. **Plan 15: Inazuma-Component Parity** — Fehlende Referenz UI-Komponenten portieren
3. **Plan 11: Multi-Tab Session Management** — Tab-System, Session-Lifecycle

---

## Plan-Dateien

| Datei | Inhalt | Status |
|---|---|---|
| `00-OVERVIEW.md` | Dieses Dokument | Aktuell |
| `01-WARP-FEATURE-ANALYSE.md` | Warp Feature-Analyse (Referenz) | Referenz |
| `04-PHASE-2-BLOCK-UX.md` | Block-System Architektur | 🔄 Teilweise done |
| `05-PHASE-3-DESIGN-SYSTEM.md` | Theming, Farben, Typographie | ⬜ Planned |
| `15-INAZUMA-COMPONENT-PARITY.md` | Fehlende Referenz UI-Komponenten portieren | ⬜ Planned |
| `07-PHASE-5-EXPLORER-EDITOR.md` | File Explorer, Code Editor | ⬜ Planned |
| `08-PHASE-6-AI-AGENTS.md` | AI Integration & Agent Toolbar | ⬜ Planned |
| `09-PHASE-7-9-FUTURE.md` | Drive, Workflows, Distribution | ⬜ Planned |
| `10-INAZUMA-OBJC2-MIGRATION.md` | objc2 + OKLCH + mod.rs Migration | ⬜ Planned |
| `11-MULTI-TAB-SESSION-MANAGEMENT.md` | Tab-System, Sessions | ⬜ Planned |
| `12-NUSHELL-FIRST-CLASS.md` | Nushell als First-Class Shell | ⬜ Planned |
| `13-INLINE-IMAGES.md` | Kitty/Sixel/iTerm2 Image Protocols | ⬜ Planned |
| `14-BLOCK-ARCHITECTURE-REWRITE.md` | raijin-term Grid pro Block | ⬜ Planned |
| `15-GHOSTTY-INSPIRATIONS.md` | Ghostty-Pattern für raijin-term | Referenz |
| `16-RENDERING-OPTIMIZATION.md` | GPU Rendering Performance | ⬜ Planned |
| `17-NATIVE-DIRECTORY-JUMPING.md` | Zoxide-Equivalent nativ | ⬜ Planned |
| `features.md` | Konsolidierte Feature-Liste (Referenz) | Referenz |
| **done/** | | |
| `done/02-PHASE-0-FOUNDATION.md` | Repository Setup | ✅ Done |
| `done/03-PHASE-1-MINIMAL-TERMINAL.md` | Erstes Terminal im Window | ✅ Done |
| `done/06-PHASE-4-INPUT-EDITOR.md` | IDE-Style Input, Completions, Shell Selector | ✅ Done |
| `done/13-PRECMD-JSON-METADATA-ARCHITECTURE.md` | OSC 7777 Metadata System | ✅ Done |

---

*Erstellt: 24. März 2026*
*Aktualisiert: 29. März 2026*
*Projekt: Raijin (雷神) — nyxb/raijin*
