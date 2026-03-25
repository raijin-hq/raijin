# Raijin (雷神) — Projekt-Überblick

> **Name:** Raijin (雷神) — Der Donnergott unter den Terminals
> **CLI Command:** `raijin`
> **Repo:** `nyxb/raijin`
> **Framework:** Inazuma (稲妻) — gevendorter Fork von Zed's GPUI
> **Ziel:** GPU-beschleunigter Terminal-Emulator mit Warp-Level UX & Design
> **Stack:** Rust + Inazuma + alacritty_terminal + Metal

---

## Tech-Stack

| Komponente | Technologie |
|---|---|
| UI Framework | Inazuma (稲妻) — gevendorter GPUI-Fork |
| UI Components | inazuma-component (70+ Widgets) |
| GPU Rendering | Metal (macOS) |
| Terminal Emulation | alacritty_terminal |
| VTE Parser | vte (in alacritty_terminal) |
| PTY | portable-pty |
| Text Shaping | Inazuma text system (cosmic-text based) |
| Layout Engine | Taffy (in Inazuma) |
| Config | TOML (raijin-settings) |
| Shell Integration | OSC 133 (blocks) + OSC 7777 (metadata) |

---

## Crate-Struktur

```
raijin/
├── Cargo.toml                  # Workspace root (Rust nightly, edition 2024)
├── crates/
│   ├── inazuma/                # GPU UI Framework (Zed GPUI Fork)
│   │   └── tooling/macros/     # Proc-Macros
│   ├── inazuma-component/      # 70+ UI Components (Input, Chips, TitleBar, Tabs)
│   │   ├── ui/
│   │   ├── macros/
│   │   └── assets/             # Bundled Fonts/Icons
│   ├── raijin-app/             # Binary — Workspace, Terminal Rendering
│   ├── raijin-terminal/        # PTY + alacritty_terminal + OSC Parser + Blocks
│   ├── raijin-shell/           # Shell Context, Metadata Payload
│   ├── raijin-settings/        # Config System (TOML)
│   ├── raijin-ui/              # Design Token System (WIP)
│   └── cargo-raijin/           # Dev CLI (dev/build/icon commands)
├── shell/
│   ├── raijin.zsh              # Shell Hooks (OSC 133 + OSC 7777 metadata)
│   ├── raijin.bash
│   └── raijin.fish
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
| Phase 2C: Block Interaction | 🔜 Next | Copy, Collapse, Navigation, Sticky Headers → Plan 12 |
| Multi-Tab Sessions | 🔜 Next | Tab-Management, Session-Persistence → Plan 11 |
| Phase 3: Design System | ⬜ Planned | Theme Tokens, Animations |
| Phase 4: Input Editor | ⬜ Planned | History, Completions, Multi-Line |
| Phase 5: Explorer + Editor | ⬜ Planned | File Tree, Code Editor |
| Phase 6: AI + Agents | ⬜ Planned | Agent Detection, AI Features |
| Phase 7–9: Future | ⬜ Planned | Drive, Workflows, Distribution |

---

## Nächste Schritte

1. **Plan 11: Multi-Tab Session Management** — Tab-System, Session-Lifecycle
2. **Plan 12: Block Interaction Design** — Copy, Collapse, Navigation, Search

---

## Plan-Dateien

| Datei | Inhalt | Status |
|---|---|---|
| `00-OVERVIEW.md` | Dieses Dokument | Aktuell |
| `01-WARP-FEATURE-ANALYSE.md` | Warp Feature-Analyse (Referenz) | Referenz |
| `04-PHASE-2-BLOCK-UX.md` | Block-System Architektur | 🔄 In Progress |
| `05-PHASE-3-DESIGN-SYSTEM.md` | Theming, Farben, Typographie | ⬜ Planned |
| `06-PHASE-4-INPUT-EDITOR.md` | IDE-Style Input & Completions | ⬜ Planned |
| `07-PHASE-5-EXPLORER-EDITOR.md` | File Explorer, Code Editor | ⬜ Planned |
| `08-PHASE-6-AI-AGENTS.md` | AI Integration & Agent Toolbar | ⬜ Planned |
| `09-PHASE-7-9-FUTURE.md` | Drive, Workflows, Distribution | ⬜ Planned |
| `10-INAZUMA-OBJC2-MIGRATION.md` | objc→objc2 Migration Plan | ⬜ Planned |
| `11-MULTI-TAB-SESSION-MANAGEMENT.md` | Tab-System, Sessions | 🔜 Next |
| `12-BLOCK-INTERACTION-DESIGN.md` | Block Copy/Collapse/Nav | 🔜 Next |
| **done/** | | |
| `done/02-PHASE-0-FOUNDATION.md` | Repository Setup | ✅ Done |
| `done/03-PHASE-1-MINIMAL-TERMINAL.md` | Erstes Terminal im Window | ✅ Done |
| `done/13-PRECMD-JSON-METADATA-ARCHITECTURE.md` | OSC 7777 Metadata System | ✅ Done |

---

*Erstellt: 24. März 2026*
*Aktualisiert: 25. März 2026*
*Projekt: Raijin (雷神) — nyxb/raijin*
