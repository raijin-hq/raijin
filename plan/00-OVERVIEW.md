# Raijin (雷神) — Projekt-Überblick

> **Name:** Raijin (雷神) — Der Donnergott unter den Terminals
> **CLI Command:** `raijin`
> **Repo:** `nyxb/raijin`
> **Basis:** gpui-ce (GPUI Community Edition, gevendored ins Monorepo)
> **Ziel:** GPU-beschleunigter Terminal-Emulator mit Warp-Level UX & Design
> **Stack:** Rust + gpui-ce + alacritty_terminal + cosmic-text

---

## Tech-Stack

| Komponente | Technologie |
|---|---|
| UI Framework | gpui-ce (gevendored in crates/gpui) |
| UI Components | gpui-component (gevendored in crates/gpui-component) |
| GPU Rendering | Metal (macOS), wgpu (Linux) |
| Terminal Emulation | alacritty_terminal |
| VTE Parser | vte |
| PTY | portable-pty |
| Text Shaping | cosmic-text |
| GPU Text Rendering | glyphon |
| Syntax Highlighting | tree-sitter + grammars |
| Layout Engine | Taffy (in GPUI) |
| Fuzzy Search | fuzzy-matcher |
| File Watching | notify |
| AI | Multi-Provider BYOK (Anthropic, OpenAI, Google) |
| MCP | Model Context Protocol |
| Config | TOML |

---

## Projekt-Struktur

```
raijin/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── gpui/               # gpui-ce gevendored — UNSER Fork, direkt editierbar
│   ├── gpui-component/     # gpui-component gevendored — Widgets anpassbar
│   ├── raijin-ui/          # Eigenes Design-System (Farben, Tokens, Themes)
│   ├── raijin-terminal/    # Terminal-Emulation (alacritty_terminal wrapper)
│   ├── raijin-shell/       # Shell-Integration (precmd/preexec hooks, agent detection)
│   ├── raijin-editor/      # Lightweight Code Editor (Warp Code equivalent)
│   ├── raijin-agent/       # AI/Agent Integration + Agent Toolbar
│   ├── raijin-drive/       # Workflows, Notebooks, Knowledge Store
│   └── raijin-app/         # Haupt-Binary, Window-Management, CLI entry point
├── assets/
│   ├── fonts/              # Input Mono, JetBrains Mono
│   ├── icons/              # Lucide oder custom SVG icons (⚡ Raijin thunder)
│   └── themes/             # Theme-Definitionen (TOML)
├── shell/
│   ├── raijin.zsh          # Shell-Hooks für zsh
│   ├── raijin.bash         # Shell-Hooks für bash
│   └── raijin.fish         # Shell-Hooks für fish
└── docs/
    └── ARCHITECTURE.md
```

---

## Zeitplan

| Woche | Phase | Ergebnis |
|---|---|---|
| 1 | Phase 0: Setup | Repo, Toolchain, gpui-ce verified |
| 2–3 | Phase 1: Minimal Terminal | Shell im GPUI-Window |
| 4–6 | Phase 2: Block-UX | Visuelle Blöcke |
| 6–9 | Phase 3: Design System | Warp-Level Polish |
| 9–11 | Phase 4: Input + Completions | Rich Editing |
| 11–14 | Phase 5: Explorer + Editor | File-Tree, Code Editor, Splits |
| 14–17 | Phase 6: AI + Agent Bar | Agent Detection, AI Features |
| 17–19 | Phase 7: Drive + Workflows | Notebooks, Launch Configs |
| 19–22 | Phase 8: Polish + Launch | Performance, Distribution |
| 22+ | Phase 9: Future | Windows, Cloud, Enterprise |

---

## Wo wir Warp SCHLAGEN können

| Bereich | Warp's Schwäche | Unsere Chance |
|---|---|---|
| Login-Pflicht | Account nötig | Kein Login, offline-first |
| Telemetry | Sammelt Daten (opt-out) | Default: keine Telemetry |
| Closed Source | Proprietär | Open Source |
| Agent Detection | Nur offizielle CLI-Names | Konfigurierbar |
| Pricing | $18-180/Monat | Freemium oder einmalig |
| Vendor Lock-in (AI) | Pusht eigene Models | BYOK: jedes Modell |
| Privacy | Cloud-abhängig | Local-first |

---

## Referenz-Projekte

| Projekt | Relevanz |
|---|---|
| termy | GPUI Terminal |
| Zed Terminal Panel | alacritty_terminal in GPUI |
| COSMIC Terminal | iced + alacritty_terminal |
| Warp Blog | Architektur |
| Rio Terminal | wgpu Rust Terminal |
| gpui-component | GPUI Widgets |
| nohrs | GPUI File Explorer |
| claude-code-warp | Agent Integration via OSC |

---

## Plan-Dateien

| Datei | Inhalt |
|---|---|
| `00-OVERVIEW.md` | Dieses Dokument — Überblick, Stack, Zeitplan |
| `01-WARP-FEATURE-ANALYSE.md` | Vollständige Warp Feature-Analyse (Referenz) |
| `02-PHASE-0-FOUNDATION.md` | Repository Setup, Toolchain, Dependencies |
| `03-PHASE-1-MINIMAL-TERMINAL.md` | Erstes funktionierendes Terminal im GPUI-Window |
| `04-PHASE-2-BLOCK-UX.md` | Block-System — Warp's Killer Feature |
| `05-PHASE-3-DESIGN-SYSTEM.md` | Theming, Farben, Typographie, Animationen |
| `06-PHASE-4-INPUT-EDITOR.md` | IDE-Style Input Editor & Smart Completions |
| `07-PHASE-5-EXPLORER-EDITOR.md` | File Explorer, Code Editor, Split Panes |
| `08-PHASE-6-AI-AGENTS.md` | AI Integration & Agent Toolbar |
| `09-PHASE-7-9-FUTURE.md` | Drive, Workflows, Polish, Distribution, Future |

---

*Erstellt: 24. März 2026*
*Projekt: Raijin (雷神) — nyxb/raijin*
*Bestätigt: Zed CEO hat GPUI-Nutzung freigegeben*
