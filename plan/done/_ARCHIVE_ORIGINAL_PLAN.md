# Raijin (雷神) — Project Plan

> **Name:** Raijin (雷神) — Der Donnergott unter den Terminals
> **CLI Command:** `raijin`
> **Repo:** `nyxb/raijin`
> **Basis:** gpui-ce (GPUI Community Edition, gevendored ins Monorepo)
> **Ziel:** GPU-beschleunigter Terminal-Emulator mit Warp-Level UX & Design
> **Stack:** Rust + gpui-ce + alacritty_terminal + cosmic-text

---

## Warp Feature-Analyse (Vollständig)

> Alle Features die Warp hat und die wir nachbauen oder verbessern müssen.
> Quelle: warp.dev/all-features, Changelog, Docs, GitHub Issues

### Kategorie 1: Terminal-Grundlagen

| Feature | Warp | Unser Ziel |
|---|---|---|
| GPU-beschleunigtes Rendering (Metal/Vulkan) | ✅ Custom Rust + Metal/wgpu | ✅ gpui-ce (Metal + wgpu) |
| Shell-Kompatibilität (zsh, bash, fish, pwsh) | ✅ | ✅ |
| Platform Support | macOS, Linux, Windows | macOS + Linux first, Windows später |
| Blocks (Command + Output gruppiert) | ✅ Killer Feature | ✅ Prio 1 |
| IDE-like Input Editor (Cursor, Selections, Multi-line) | ✅ | ✅ |
| Vim Keybindings im Input | ✅ | ✅ |
| Smart Completions (400+ CLI Tools) | ✅ | ✅ |
| Command Corrections (Typo-Fix Vorschläge) | ✅ | ✅ Phase 4 |
| Split Panes (horizontal/vertikal) | ✅ | ✅ |
| Tabs (mit farbigen Indikatoren) | ✅ | ✅ |
| Scrollback Buffer | ✅ | ✅ |
| Text Selection (Maus + Keyboard) | ✅ | ✅ |
| Copy/Paste | ✅ | ✅ |
| Backwards compatible (bestehende Shell-Configs) | ✅ | ✅ |

### Kategorie 2: Appearance & Customization

| Feature | Warp | Unser Ziel |
|---|---|---|
| Custom Themes (Library + GUI Builder) | ✅ Themes + Background Images | ✅ |
| Theme aus Background-Image generieren (Farbpalette) | ✅ | ✅ Nice-to-have |
| Accent Color konfigurierbar | ✅ | ✅ |
| Custom Prompt (Chips) + PS1 Support (Starship, P10k) | ✅ | ✅ |
| Input Position (Top/Bottom pinnable) | ✅ | ✅ |
| Transparenter Background (Opacity) | ✅ | ✅ |
| App-Zoom (CMD+/CMD-) | ✅ | ✅ |
| Tab-Bar Visibility (Always/Hover/Windowed) | ✅ | ✅ |
| Tab Close-Button Position (links/rechts) | ✅ | ✅ |
| Tab-Farben (pro Tab konfigurierbar) | ✅ | ✅ |
| Font konfigurierbar (Type + Size) | ✅ | ✅ |
| Syntax Highlighting im Terminal-Output | ✅ | ✅ |

### Kategorie 3: Agent Utility Bar (das Feature aus dem Screenshot!)

| Feature | Warp | Unser Ziel |
|---|---|---|
| Automatische Agent-Erkennung (claude, codex, gemini) | ✅ Erkennt CLI-Agent automatisch | ✅ Prio 1 |
| Kontextuelle Footer-Toolbar für Third-Party CLI Agents | ✅ Zeigt File Explorer, View Changes etc. | ✅ Prio 1 |
| File Explorer in Agent-Modus | ✅ Project Explorer Sidebar | ✅ |
| View Changes (Diff-View) in Agent-Modus | ✅ Inline Diff-Viewer | ✅ |
| Voice Input Icon | ✅ (Wispr Flow Integration) | ✅ Optional |
| Image Attachment Icon | ✅ Bilder an Agents senden | ✅ |
| Drag File-Paths aus Explorer in Agent-Commands | ✅ | ✅ |
| Agent Status Indikator (blocked, working, done) | ✅ | ✅ |
| Custom Wrapper-Commands als Agent erkennen | ❌ (Feature Request #8579) | ✅ Von Anfang an konfigurierbar |
| Desktop Notifications wenn Agent fertig | ✅ (via OSC escape sequences) | ✅ |

### Kategorie 4: Code Editor (Warp Code)

| Feature | Warp | Unser Ziel |
|---|---|---|
| Nativer File Editor (Tabs, Syntax Highlighting) | ✅ Warp Code | ✅ Phase 5 |
| Real-time Diff Tracking | ✅ | ✅ |
| Code Review Panel | ✅ Accept/Reject/Edit Diffs inline | ✅ |
| File Tree / Project Explorer | ✅ mit .gitignore awareness | ✅ |
| Go to Line (CTRL-G) | ✅ | ✅ |
| Open Files from Explorer in Editor | ✅ | ✅ |
| External Editor Integration (VS Code, Zed, Cursor) | ✅ Konfigurierbar | ✅ |
| Lightweight — kein vollständiger IDE-Ersatz | ✅ | ✅ |

### Kategorie 5: AI / Agent Mode

| Feature | Warp | Unser Ziel |
|---|---|---|
| AI Command Suggestions (Natural Language → Command) | ✅ # Prefix | ✅ |
| Chat mit AI (Seitenpanel) | ✅ Agent Mode Panel | ✅ |
| Pair Mode (AI assistiert neben dir) | ✅ | ✅ |
| Dispatch Mode (AI arbeitet autonom) | ✅ | ✅ Phase 6+ |
| /plan Command (AI erstellt Ausführungsplan) | ✅ | ✅ |
| Multi-Model Support (OpenAI, Anthropic, Google) | ✅ 20+ Modelle | ✅ BYOK (Bring Your Own Key) |
| @-Context (Files, Images, URLs, Conversations) | ✅ Universal Input | ✅ |
| Prompt Suggestions (Active AI) | ✅ Kontextuelle Vorschläge | ✅ |
| Next Command (Ghost-Command basierend auf History) | ✅ | ✅ |
| Error Explanation (Exit Code != 0 → AI erklärt) | ✅ | ✅ |
| AI Block-Inhalte kopierbar | ✅ | ✅ |
| Agent Thinking expanded lassen (Setting) | ✅ | ✅ |
| Secret Redaction (API Keys in AI-Context obscuren) | ✅ | ✅ |
| MCP Server Integration | ✅ Figma, Linear, Slack, Sentry etc. | ✅ |
| Auto-detect MCP Servers (claude/codex config files) | ✅ | ✅ |
| WARP.md / agents.md / claude.md Support | ✅ Rules für Agents | ✅ |
| Voice Input (Wispr Flow) | ✅ | ⬜ Nice-to-have |

### Kategorie 6: Warp Drive (Cloud Knowledge)

| Feature | Warp | Unser Ziel |
|---|---|---|
| Workflows (parametrisierte Commands speichern) | ✅ | ✅ Phase 7 |
| Notebooks (interaktive Runbooks) | ✅ | ✅ Phase 7 |
| Personal Drive (Cloud-basierte Wissensbibliothek) | ✅ | ✅ Phase 8+ |
| Team Drive (geteilte Workflows/Notebooks) | ✅ | ✅ Phase 8+ |
| Environment Variables (sync across sessions) | ✅ | ✅ |
| Rules (Agent-Verhaltens-Konfiguration) | ✅ | ✅ |
| MCP Server Configs (teilbar im Team) | ✅ | ✅ |
| Warp Drive on Web (Browser-Zugang) | ✅ | ⬜ Später |

### Kategorie 7: Oz Cloud Agents (Warp's Neueste Plattform)

| Feature | Warp | Unser Ziel |
|---|---|---|
| Cloud Agent Orchestration | ✅ Oz Platform | ⬜ Phase 9+ (Differenzierung) |
| Parallel Cloud Agents | ✅ Unlimited parallel | ⬜ |
| Triggers (Slack, Linear, GitHub, Cron, Webhooks) | ✅ | ⬜ |
| Agent Audit Trail | ✅ | ⬜ |
| CLI + API/SDK | ✅ oz CLI | ⬜ |
| Self-hosted Environments | ✅ | ⬜ |
| Full Terminal Use (PTY attach) | ✅ | ⬜ |
| Computer Use (GUI Sandbox) | ✅ | ⬜ |

### Kategorie 8: Collaboration

| Feature | Warp | Unser Ziel |
|---|---|---|
| Session Sharing (Real-time terminal sharing) | ✅ Beta | ⬜ Phase 8+ |
| Block Sharing (Permalink für Command+Output) | ✅ | ✅ Phase 7 |
| Shared Agent Sessions | ✅ | ⬜ Phase 8+ |

### Kategorie 9: Usability

| Feature | Warp | Unser Ziel |
|---|---|---|
| Command Palette (CMD+P) | ✅ | ✅ |
| Command Search (History + Drive) | ✅ | ✅ |
| Rich History (Exit Codes, Directory, Branch, Timestamps) | ✅ | ✅ |
| Markdown Viewer (mit ausführbaren Commands) | ✅ | ✅ |
| Launch Configurations (Window/Pane/Command Presets) | ✅ | ✅ Phase 7 |
| Quake Mode (Dedicated Hotkey Window) | ✅ | ✅ |
| Shell Selector (Dropdown für Shell-Wechsel) | ✅ | ✅ |
| Sticky Headers (Block-Header beim Scrollen) | ✅ | ✅ |
| Global Search (über Code, Terminal, Notebooks) | ✅ | ✅ Phase 6 |

### Kategorie 10: Privacy & Security

| Feature | Warp | Unser Ziel |
|---|---|---|
| Secret Redaction (API Keys obscuren) | ✅ | ✅ |
| Disable Telemetry | ✅ | ✅ Default: aus |
| Zero Data Retention (Enterprise) | ✅ | ✅ |
| Disable Active AI | ✅ | ✅ |
| SSO / SAML (Enterprise) | ✅ | ⬜ Phase 8+ |

### Kategorie 11: Integrations

| Feature | Warp | Unser Ziel |
|---|---|---|
| Raycast / Alfred Integration | ✅ | ✅ |
| External Editor öffnen (VS Code, Zed, Cursor) | ✅ | ✅ |
| Docker Extension | ✅ | ✅ Phase 7 |
| Figma MCP (Auto-Detect Figma References) | ✅ | ✅ via MCP |
| GitHub Actions Integration (Oz) | ✅ | ⬜ Phase 9+ |

---

## Phase 0: Foundation Setup (Woche 1)

### 0.1 — Repository & Toolchain

- [ ] Fork `gpui-ce/gpui-ce` → eigenes Repo `nyxb/raijin`
- [ ] Rust Toolchain: latest stable (`rustup update stable`)
- [ ] Xcode Command Line Tools installieren (Metal-Rendering auf macOS)
- [ ] gpui-ce als lokales Package vendoren (NICHT als git dependency!)
- [ ] gpui-component ebenfalls vendoren
- [ ] Projekt-Struktur als Cargo Workspace anlegen:

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

**Warum vendored statt git dependency:**
gpui-ce und gpui-component werden direkt ins Monorepo kopiert als lokale crates.
So können wir Shader, Rendering-Primitives, Styles und Widgets direkt editieren
ohne auf upstream angewiesen zu sein. Das Design divergiert sofort — Upstream-Syncs
passieren nur noch als gezielte Cherry-Picks für Bugfixes.

```toml
# Cargo.toml (workspace root)
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.dependencies]
gpui = { path = "crates/gpui" }
gpui-component = { path = "crates/gpui-component" }
raijin-ui = { path = "crates/raijin-ui" }
raijin-terminal = { path = "crates/raijin-terminal" }
raijin-shell = { path = "crates/raijin-shell" }
raijin-editor = { path = "crates/raijin-editor" }
raijin-agent = { path = "crates/raijin-agent" }
raijin-drive = { path = "crates/raijin-drive" }
```

### 0.2 — gpui-ce verifizieren

- [ ] gpui-ce clonen und Example-App kompilieren (`cargo run --example`)
- [ ] Sicherstellen dass Metal-Rendering auf deinem Mac läuft
- [ ] gpui-component Gallery starten und alle Widgets durchklicken
- [ ] Verstehen wie `div()`, `.bg()`, `.rounded()`, `.shadow()` etc. funktionieren

### 0.3 — Dependencies festlegen

```toml
# Cargo.toml — zusätzliche externe Dependencies
[workspace.dependencies]
alacritty_terminal = "0.24"    # Terminal Grid, VTE Parser, PTY
vte = "0.15"                   # ANSI Escape Sequence Parser
cosmic-text = "0.12"           # Text Shaping & Font Fallback
glyphon = "0.6"                # GPU Text Rendering (wgpu)
portable-pty = "0.8"           # Cross-platform PTY
tree-sitter = "0.24"           # Syntax Highlighting
tree-sitter-bash = "0.23"      # Shell Syntax
serde = { version = "1", features = ["derive"] }
toml = "0.8"                   # Config/Theme Dateien
notify = "7"                   # Filesystem Watcher (für File Explorer)
fuzzy-matcher = "0.3"          # Fuzzy Search (Command Palette, History)
```

---

## Phase 1: Minimal Terminal (Woche 2–3)

### Ziel: Ein Fenster das eine Shell rendert — ugly but functional

### 1.1 — GPUI Window + Basic Layout

- [ ] `raijin-app` Binary erstellen mit `Application::new()` + `open_window()`
- [ ] Root-View mit Sidebar (links) + Terminal-Area (rechts) als Grundlayout
- [ ] Sidebar erstmal leer, nur als farbiger Rect mit fester Breite
- [ ] Terminal-Area als scrollbarer Bereich
- [ ] Tab-Bar (oben) mit einem Tab

### 1.2 — alacritty_terminal Integration

- [ ] `alacritty_terminal::Term` initialisieren mit Standard-Config
- [ ] PTY spawnen (`portable-pty`) mit Default-Shell (`$SHELL` oder `/bin/zsh`)
- [ ] Event-Loop: PTY-Output → `Term::advance()` → Grid-State updaten
- [ ] Keyboard-Input von GPUI-Window an PTY forwarden

### 1.3 — Terminal Grid Rendering

- [ ] `Term.renderable_content()` auslesen → Cell-Grid mit Chars + Farben
- [ ] Custom GPUI Element bauen (`TerminalElement`) das das Grid rendert
- [ ] Monospace-Font laden (Input Mono oder JetBrains Mono)
- [ ] Jede Cell als positioned Text-Glyph rendern
- [ ] ANSI-Farben (16 + 256 + TrueColor) korrekt mappen
- [ ] Cursor rendern (Block, Beam, Underline)
- [ ] Scrollback-Buffer implementieren (Mouse-Scroll, Shift+PageUp)

### 1.4 — Basis-Interaktion

- [ ] Text-Selection mit Maus (Click + Drag)
- [ ] Copy/Paste (Cmd+C / Cmd+V)
- [ ] Resize: Window-Resize → PTY-Resize → Grid-Resize
- [ ] Shell-Kompatibilität testen: zsh, bash, fish

### Milestone: `cargo run -p raijin-app` startet ein Fenster mit funktionierender Shell

---

## Phase 2: Block-UX — Warp's Killer Feature (Woche 4–6)

### Ziel: Commands und Output als visuelle Blöcke statt Textstrom

### 2.1 — Shell-Integration (precmd/preexec Hooks)

- [ ] Shell-Hook-Scripts erstellen für zsh, bash, fish:
  - `precmd`: Sendet Marker vor jedem Prompt (z.B. `\x1b]133;A\x07`)
  - `preexec`: Sendet Marker vor Command-Execution (`\x1b]133;C\x07`)
- [ ] VTE-Parser erweitern um diese OSC-Markers zu erkennen
- [ ] Daraus Command-Boundaries ableiten: wo fängt ein Command an, wo endet sein Output

### 2.2 — Block-Datenmodell

- [ ] `Block` struct definieren:

```rust
struct TerminalBlock {
    id: BlockId,
    command: String,              // Der eingegebene Command
    output_grid: Grid<Cell>,      // Separates Grid für den Output
    start_time: Instant,
    end_time: Option<Instant>,
    exit_code: Option<i32>,
    working_directory: PathBuf,
    git_branch: Option<String>,
    is_collapsed: bool,
    is_selected: bool,
}
```

- [ ] `BlockManager` der eine `Vec<TerminalBlock>` maintained
- [ ] Jeder Block bekommt sein eigenes `alacritty_terminal::Grid`

### 2.3 — Block-Rendering

- [ ] Jeden Block als eigene GPUI-View rendern mit:
  - Command-Zeile (oben, leicht hervorgehoben)
  - Output-Bereich (darunter, scrollbar bei langem Output)
  - Exit-Code Badge (Pill: grün = 0, rot = non-zero)
  - Timestamp + Duration
  - Subtiler Separator zwischen Blöcken
  - Hover-State: Block-Hintergrund leicht aufhellen
  - Selected-State: Akzent-Border links
- [ ] Block-Navigation: ↑/↓ Pfeile springen zwischen Blöcken
- [ ] Block kopieren (Cmd+C auf selektierten Block)
- [ ] Block collapsible (Chevron oder Shortcut)
- [ ] Block Sharing: Permalink generieren (Block → URL)
- [ ] Sticky Block-Header beim Scrollen durch langen Output
- [ ] Rich History pro Block: Exit Code, Directory, Branch, Timestamp

### 2.4 — Input Position (Warp Feature)

- [ ] Input-Editor pinnable: Top oder Bottom
- [ ] Setting: "Pin to top" (Blocks fließen nach unten)
- [ ] Setting: "Pin to bottom" (Standard, Blocks fließen nach oben)
- [ ] CTRL-L: Blocks außer Sichtweite scrollen (Clean View)
- [ ] CTRL-SHIFT-K: Alle Blocks löschen

### Milestone: Terminal zeigt Commands als separate visuelle Einheiten

---

## Phase 3: Design System — Von "funktional" zu "Warp-Level" (Woche 6–9)

### Ziel: Das Ding muss GEIL aussehen

### 3.1 — Farb-System & Theming

- [ ] Design-Token-System definieren (TOML/YAML):

```toml
[colors]
bg_primary = "#1a1b26"
bg_secondary = "#1e1f2b"
bg_tertiary = "#252736"
bg_block = "#1c1d28"
bg_block_hover = "#22233a"
accent = "#7dcfff"
accent_secondary = "#bb9af7"
text_primary = "#c0caf5"
text_secondary = "#565f89"
text_tertiary = "#3b4261"
border = "#292e42"
success = "#9ece6a"
warning = "#e0af68"
error = "#f7768e"
```

- [ ] Theme-Loader der TOML-Files liest und in GPUI-Styles übersetzt
- [ ] Theme Library: Min. 5 Themes (Dark, Light, Dracula, Nord, Gruvbox)
- [ ] GUI Theme Builder: Accent Color + Background Image → Palette generieren
- [ ] Accent-Color konfigurierbar
- [ ] Tab-Farben pro Tab konfigurierbar (6 Farben wie Warp)
- [ ] Transparenter Background mit Opacity-Slider

### 3.2 — Typographie-Hierarchie

- [ ] Font-Stack: Terminal (monospace) + UI (proportional)
- [ ] Größen-Skala: 11px → 12px → 13px → 14px → 16px
- [ ] Font konfigurierbar in Settings (Type + Size)

### 3.3 — Visual Layering

- [ ] 3+ Hintergrund-Ebenen mit subtilen Borders
- [ ] Komponenten-Design: Tabs, Sidebar, Blocks, Scrollbar

### 3.4 — Animationen

- [ ] Hover: 150ms ease-out
- [ ] Tab-Switch: Accent slide
- [ ] Block-Expand/Collapse
- [ ] Cursor-Blink: Smooth opacity

### Milestone: App sieht visuell auf Warp-Niveau aus

---

## Phase 4: IDE-Style Input Editor + Completions (Woche 9–11)

### 4.1 — Rich Input Editor

- [ ] Multi-Line, Syntax-Highlighting, Mouse-Cursor, Multi-Cursor
- [ ] Vim Keybindings (togglebar)
- [ ] Custom Prompt mit Context-Chips + PS1/Starship/P10k Support

### 4.2 — Smart Completions

- [ ] File/Path, Command, Git-Branch, History Completion
- [ ] Ghost-Text (inline Vorschlag)
- [ ] CLI-Specs für 400+ populäre Tools

### 4.3 — Command Corrections + Shell Selector

- [ ] Typo-Erkennung, Missing Parameter Vorschläge
- [ ] Shell-Dropdown für schnellen Wechsel

### Milestone: Input-Editor fühlt sich wie ein Mini-IDE an

---

## Phase 5: File Explorer + Code Editor + Panels (Woche 11–14)

### 5.1 — File Explorer / Project Explorer

- [ ] Tree mit lazy loading, Drag & Drop, Rename, Context-Menu
- [ ] .gitignore awareness, Hidden Files oben, File-Icons
- [ ] Drag Paths in Terminal-Commands

### 5.2 — Code Editor (Warp Code Equivalent)

- [ ] Nativer Editor mit Tabs, Syntax Highlighting, Go to Line
- [ ] Diff View + Code Review Panel (Accept/Reject/Edit)

### 5.3 — Split Panes + Command Palette + Quake Mode + Markdown Viewer

- [ ] Splits, Palette (Cmd+P), Quake Mode (Hotkey Window)
- [ ] Markdown Viewer mit ausführbaren Commands

### Milestone: Vollständige Desktop-App

---

## Phase 6: AI Integration + Agent Toolbar (Woche 14–17)

### 6.1 — Agent Detection & Utility Bar

- [ ] Prozess-Erkennung: `claude`, `codex`, `gemini` → Toolbar einblenden
- [ ] Footer-Bar: File Explorer, View Changes, Image Attach, Voice, Status
- [ ] Konfigurierbar: Custom Commands als Agent erkennen (Pattern-Matching)
- [ ] Desktop Notifications via OSC wenn Agent fertig
- [ ] Task-Name in Tab-Title

### 6.2 — AI Features

- [ ] `#` Prefix → Natural Language → Command (Multi-Provider BYOK)
- [ ] Agent Mode Panel: Pair + Dispatch Mode, /plan, @-Context
- [ ] Error Explanation, Next Command Suggestions
- [ ] MCP Server Integration (Auto-Detect + konfigurierbar)
- [ ] Secret Redaction + Rules/agents.md Support

### Milestone: AI-unterstütztes Terminal mit Agent Toolbar

---

## Phase 7: Drive, Workflows, Notebooks (Woche 17–19)

- [ ] Workflows (parametrisierte Commands, AI Autofill)
- [ ] Notebooks (Markdown + ausführbare Commands)
- [ ] Launch Configurations (Presets speichern/laden)
- [ ] Environment Variables (sync across sessions)
- [ ] Block Sharing (Permalinks)
- [ ] Docker Extension

---

## Phase 8: Polish, Performance & Distribution (Woche 19–22)

- [ ] Performance: < 8ms Frame-Budget, Startup < 200ms
- [ ] Settings: Vollständiges GUI Settings-Panel
- [ ] Keybindings: Context-abhängig, Warp-kompatibel, Custom
- [ ] Integrations: Raycast/Alfred, External Editors
- [ ] Distribution: macOS DMG + Homebrew, Linux AppImage/deb/rpm
- [ ] Branding: Name, Logo, Icon, Landing Page, Auto-Updater

---

## Phase 9+: Future / Differenzierung (Post-Launch)

- [ ] Windows Support (via wgpu)
- [ ] Session Sharing (Real-time)
- [ ] Team Drive
- [ ] Cloud Agent Orchestration
- [ ] SSO / SAML (Enterprise)
- [ ] Web-Version (WASM)

---

## Tech-Stack Zusammenfassung

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

*Erstellt: 24. März 2026*
*Projekt: Raijin (雷神) — nyxb/raijin*
*Stack: gpui-ce (gevendored) + alacritty_terminal + cosmic-text*
*Bestätigt: Zed CEO hat GPUI-Nutzung freigegeben*
*Warp Feature-Analyse: Vollständig (warp.dev/all-features + Changelog + Docs + GitHub)*
