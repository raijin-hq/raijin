# Phase 28: Feature-Roadmap & Extension-Capabilities

## Übersicht

Vollständige Feature-Roadmap basierend auf `plan/features.md` (konsolidiert aus Warp, WezTerm, Wave Terminal). Jedes Feature ist als **Core** (wir bauen es) oder **Extension** (Drittanbieter bauen es via WASM Extensions) klassifiziert.

## Core Features

### KI & Automatisierung (→ Phase 25, 26)

Alles Core — KI ist das Herz von Raijin's ADE.

| Feature | Status | Audit (2026-04-12) | Phase |
|---|---|---|---|
| KI-gestützte Befehlsgenerierung aus natürlicher Sprache | Code da, nicht verdrahtet | raijin-agent (60+ Dateien, inazuma imports) existiert, NICHT in raijin-app/Cargo.toml | 26 |
| KI-Fehleranalyse bei fehlgeschlagenen Befehlen | neu bauen | Kein Code | 25E |
| KI-Agent-Modus (mehrstufige Ziele, planen + ausführen) | Code da, nicht verdrahtet | raijin-agent hat thread.rs, tools.rs, edit_agent.rs — nicht in App geladen | 26 |
| KI-Kontextfenster (CWD, History, Errors, Env) | neu bauen | Kein Code | 26B |
| Lokale Modelle (Ollama, LM Studio, vLLM) | Code da, nicht verdrahtet | Provider-Crates existieren, nicht in App geladen | - |
| OpenAI-kompatible Endpoints | Code da, nicht verdrahtet | raijin-open-ai (3 Dateien), nicht in raijin-app | - |
| Gemini, Claude, Azure, OpenRouter | Code da, nicht verdrahtet | Provider-Crates existieren, nicht in App geladen | - |
| Mehrere KI-Profile / Modi | neu bauen | Kein Code | 26A |
| BYOK (Bring Your Own Key) | neu bauen | Kein Setting vorhanden | 19 |
| KI-Vision / Multimodal (Bilder, PDFs) | neu bauen | Kein Code | 26 |
| KI kann Dateien erstellen/bearbeiten (Diff + Rollback) | Code da, nicht verdrahtet | raijin-agent hat edit_file_tool.rs, streaming_edit_file_tool.rs — nicht verdrahtet | 26E |
| Drag & Drop Dateien in KI-Chat | neu bauen | Kein Code | 26 |
| KI-Zugriff auf Scrollback, Dateisystem | Code da, nicht verdrahtet | raijin-agent hat read_file_tool.rs, terminal_tool.rs — nicht verdrahtet | 26C |
| KI-Tools: Websuche, Dateioperationen | Code da, nicht verdrahtet | raijin-agent hat web_search_tool.rs, fetch_tool.rs — nicht verdrahtet | - |
| CLI-Output an KI senden | neu bauen | Kein Code | 26B |
| KI Thinking Mode (Quick / Balanced / Deep) | neu bauen | Kein Code | 26 |
| Stop-Generierung | neu bauen | Kein Code | 26 |
| Feedback-Buttons (Daumen hoch/runter) | neu bauen | Kein Code | 26 |

### Terminal-Kernfunktionen (→ Phase 20, raijin-term)

Alles Core — das ist was Raijin als Terminal SEIN muss.

| Feature | Status | Audit (2026-04-12) | Phase |
|---|---|---|---|
| GPU-beschleunigtes Rendering | funktioniert | Metal: metal_renderer.rs, WGPU: wgpu_renderer.rs, Shader vorhanden | vorhanden |
| Vollständige xterm-Kompatibilität | funktioniert | raijin-term mit VTE Parser, Term struct, Handler — vollständig | vorhanden |
| Tabs mit Drag & Drop, Umbenennung, Kontextmenü | neu bauen | Workspace Pane System existiert nicht | 20 |
| Vertikale und horizontale Tab-Leiste | neu bauen | Kein Code | 24 |
| Splits / Panes (horizontal & vertikal) | neu bauen | Kein Code | 20 |
| Vim-artige Pane-Navigation (Ctrl+Shift+H/J/K/L) | neu bauen | Kein Code | 20 |
| Mehrere Fenster | neu bauen | Kein Code | 20 |
| Scrollback mit konfigurierbarer Zeilenzahl | funktioniert | Setting gelesen (10k Default), terminal_pane.rs:134→Terminal::new() | vorhanden |
| Suchbare Scrollback-Funktion (Cmd+F) | neu bauen | Kein Code | 25B |
| Semantische Prompt-Navigation (OSC 133) | funktioniert | osc_parser.rs: A/B/C/D/P Marker, State-Machine Scanner | vorhanden |
| Command Blocks als navigierbare Einheiten | funktioniert | block.rs: BlockManager, TerminalBlock mit ID/Command/Exit/Duration | vorhanden |
| Block-Badges (Icon, Farbe, Priorität) | funktioniert | block_element.rs: Running●/Error✗/Success✓ Badges + Metadata-Line | vorhanden |
| Multiline-Eingabe (Shift+Enter) | funktioniert | state_editing.rs:155 "Enter=submit, Shift+Enter=newline", shell_editor(1,10) | vorhanden |
| Multi-Input-Modus (gleichzeitig in alle Terminals) | neu bauen | Kein Code | 28 |
| Cursor-Style und Blink einstellbar | teilweise | Setting gelesen (cursor_style, cursor_blink), aber _cursor_shape IGNORIERT im TerminalBuilder:56 | vorhanden |
| Copy-on-Select | neu bauen | Kein Code | 28 |
| OSC 52 Clipboard-Support | neu bauen | Kein Code | 28 |
| Bracketed Paste Mode | funktioniert | mode.rs: BRACKETED_PASTE Flag, handler.rs: CSI ?2004h/l | vorhanden |
| Hyperlinks im Terminal (klickbar) | teilweise | cell.rs: Hyperlink struct in CellExtra, aber kein Click-Handler in UI | 28 |
| Text-Attribute (Underline, Italic, Bold, Strikethrough) | funktioniert | cell.rs: BOLD/ITALIC/UNDERLINE/STRIKEOUT/UNDERCURL/DOTTED/DASHED Flags | vorhanden |
| SGR-Maus-Reporting | funktioniert | mode.rs: SGR_MOUSE Flag, handler.rs: CSI ?1006h/l | vorhanden |
| Inline-Bilder (iTerm2 Image Protocol) | neu bauen | Kein Code | 28 |
| Inline-Bilder (Kitty Graphics Protocol) | neu bauen | Kein Code | 28 |
| Sixel-Grafiken | neu bauen | Kein Code | 28 |
| Ligatures, Color Emoji, Font-Fallback | funktioniert | font_features.rs: CALT Control, grid_element.rs: paint_emoji, font_fallbacks.rs | vorhanden |
| True Color / 24-Bit | funktioniert | color.rs: 269 Colors mit Rgb, SGR 38;2/48;2 via VTE | vorhanden |
| Themes (Dark/Light, Custom) | funktioniert | ThemeRegistry, TOML Loader, OKLCH Pipeline, GlobalTheme Sync | vorhanden |
| Hintergrundbilder und Transparenz | funktioniert | ThemeBackgroundImage, TOML Config, terminal_pane.rs:991 Rendering mit Opacity | vorhanden |
| Anpassbares Padding | neu bauen | Kein Code | 28 |
| Font-Size pro Block einstellbar | neu bauen | Kein Code | 28 |
| Konfigurierbare FPS (120 FPS) | neu bauen | Kein Code | 28 |
| Audible Bell deaktivierbar | neu bauen | Kein Code | 28 |
| IME-Unterstützung (CJK) | funktioniert | NSTextInputClient Protocol, markedText, Composition — nur macOS | vorhanden |
| Vollbild-Modus | neu bauen | Kein Code | 28 |
| Bell-Indikator als Badge | neu bauen | Kein Code | 28 |
| Fokus-folgt-Cursor | neu bauen | Kein Code | 28 |

### Editor & Dateimanagement (→ Phase 25, raijin-editor)

| Feature | Status | Audit (2026-04-12) | Phase |
|---|---|---|---|
| Eingebauter Code-Editor | Code da, nicht verdrahtet | raijin-editor (116k Zeilen, inazuma imports), nur transitiv via settings-ui geladen — nicht als eigenständiger Editor nutzbar | 25 |
| Syntax-Highlighting, Fehleranzeige | Code da, nicht verdrahtet | raijin-editor + raijin-lsp + raijin-language (Tree-sitter, 6 Grammars) — nur intern in Settings-UI aktiv | 25 |
| Visueller Diff-Viewer | Code da, nicht verdrahtet | raijin-streaming-diff: Myers-Diff Algo, 36k — NICHT in raijin-app/Cargo.toml | 25 |
| Datei-Rollback nach KI-Edits | neu bauen | Kein Code | 26E |
| Datei-Vorschau: Bilder, Markdown | Code da, nicht verdrahtet | raijin-image-viewer (Zoom/Pan/Actions), raijin-markdown-preview — NICHT in raijin-app | 25 |
| Datei-Vorschau: Audio/Video, PDFs | neu bauen | Kein Code | 28 |
| Verzeichnis-Browser | Code da, nicht verdrahtet | raijin-project-panel (18k Zeilen, Git-Integration, Undo/Redo) — NICHT in raijin-app | 25 |
| Drag & Drop Dateien | neu bauen | Kein Code | 20 |
| Scrollback in Datei speichern | neu bauen | Kein Code | 28 |
| Bildeinfügen per Paste | neu bauen | Kein Code | 28 |
| Quick Look (macOS) | neu bauen | Kein Code | 28 |
| Dateien im externen Explorer öffnen | neu bauen | Kein Code | 28 |

### SSH & Remote-Verbindungen (→ raijin-remote)

| Feature | Status | Audit (2026-04-12) | Phase |
|---|---|---|---|
| SSH-Verbindungsmanagement mit Profilen | Code da, nicht verdrahtet | raijin-remote: SSH/WSL/Docker Transport, ConnectionState — NICHT in raijin-app/Cargo.toml | 20 |
| Durable SSH Sessions | neu bauen | Kein Code | 28 |
| Auto-Reconnect | neu bauen | Kein Code | 28 |
| Shell-State bei Disconnect erhalten | neu bauen | Kein Code | 28 |
| Status-Anzeigen (Attached/Detached) | neu bauen | Kein Code | 28 |
| SSH Key Management / Agent Forwarding | Code da, nicht verdrahtet | raijin-askpass: AskPassDelegate/Session/PasswordProxy (393 Zeilen) — nur Dep von raijin-remote | 20 |
| SSH-Passwörter im Secret Store | Code da, nicht verdrahtet | raijin-credentials-provider: Keychain+Dev Provider — in Cargo.toml aber init() nie aufgerufen | 20 |
| WSL2 Support | Code da, nicht verdrahtet | raijin-remote/transport/wsl.rs existiert — nicht geladen | 28 |
| Remote-Dateien im Editor | neu bauen | Kein Code | 28 |
| Drag & Drop lokal ↔ remote | neu bauen | Kein Code | 28 |
| Per-Connection Themes | neu bauen | Kein Code | 28 |

### Multiplexer & Workspace (→ Phase 20)

| Feature | Status | Audit (2026-04-12) | Phase |
|---|---|---|---|
| Eingebauter Multiplexer (Tabs, Splits, Sessions) | neu bauen | raijin-workspace existiert als Framework, aber Tabs/Splits/Sessions nicht implementiert | 20 |
| Workspaces mit eigenen Layouts/Settings | neu bauen | Kein Code | 20 |
| Tab-Close-Bestätigung | neu bauen | Kein Code | 20 |
| Bestätigung beim Beenden mit aktiven Sessions | neu bauen | Kein Code | 20 |
| Session-Wiederherstellung nach Neustart | neu bauen | Kein Code | 20 |
| Zoom einzelner Blöcke | neu bauen | Kein Code | 28 |

### Shell-Integration & Produktivität (→ raijin-shell, raijin-completions)

| Feature | Status | Audit (2026-04-12) | Phase |
|---|---|---|---|
| Shell-Integration bash, zsh, fish, pwsh | teilweise | Zsh ✓ (ZDOTDIR), Bash ✓ (--rcfile), Fish ✓ (--init-command), Nushell ✓ (autoload) — **pwsh fehlt** | vorhanden |
| OSC 7 CWD Tracking | teilweise | Kein OSC 7 Parser — CWD wird über OSC 7777 JSON Metadata statt Standard OSC 7 aktualisiert | vorhanden |
| Shell Context Tracking (Status, Exit-Code) | funktioniert | osc_parser.rs: OSC 133;D;N → exit_code, block.rs: Duration + Metadata | vorhanden |
| Env-Variablen und Init-Scripts konfigurierbar | neu bauen | Kein Code | 28 |
| History-basierte Autovervollständigung | teilweise | command_history.rs: frecency_search() → nur Ghost-Text (Top 1), keine Menu-Items | vorhanden |
| Subcommand-Completions für Standard-CLIs (git, docker, cargo) | funktioniert | 72 JSON Specs embedded (git, cargo, docker, npm, kubectl...), Lazy-Loading | vorhanden |
| Fuzzy-Matching für Dateipfade | teilweise | inazuma-fuzzy existiert (CharBag, Distance-Penalty) aber Completions nutzt nur prefix-match (starts_with) | vorhanden |
| Tab-Navigator | funktioniert | raijin-tab-switcher: init(cx) aufgerufen in main.rs:130, Fuzzy-Picker | vorhanden |
| CLI-Steuerung von außen (wie wsh) | neu bauen | Kein Code | 28 |
| Desktop-Benachrichtigungen | neu bauen | Kein Code | 28 |
| Secret Store | Code da, nicht verdrahtet | raijin-credentials-provider in Cargo.toml, aber init() nie aufgerufen, keine Nutzung | vorhanden |
| Variablen über Sessions hinweg | neu bauen | Kein Code | 28 |
| Globale Hotkey-Unterstützung | neu bauen | Kein Code | 28 |

### Konfiguration & Anpassung (→ Phase 19, Settings)

| Feature | Status | Audit (2026-04-12) | Phase |
|---|---|---|---|
| Hot-Reload der Konfiguration | funktioniert | main.rs: Fs::watch auf settings.toml + keymap.toml, cx.refresh_windows() | vorhanden |
| Settings-GUI | funktioniert | raijin-settings-ui (5.4k Zeilen): SettingFieldRenderer, Switch, Dropdown, NumberField — init(cx) aufgerufen | vorhanden |
| Themes inkl. Presets | funktioniert | 7 bundled Themes (Raijin Dark/Light, Dracula, Nord, Gruvbox, One Dark, Catppuccin) + Fallbacks | vorhanden |
| Kitty Keyboard Protocol | neu bauen | Kein Code | 28 |
| Anpassbare Keybindings | funktioniert | KeymapFile TOML Parser, default-macos.toml, Hot-Reload via watch_config_file | vorhanden |
| Option Key als Meta Key (macOS) | neu bauen | Kein Code | 28 |
| Anpassbare Tab-Farben/Icons/Titel | neu bauen | Kein Code | 20 |

### Plattform & Integration

| Feature | Status | Audit (2026-04-12) | Phase |
|---|---|---|---|
| macOS | funktioniert | Vollständiger Platform Layer: NSApplication, NSWindow, Metal, CoreText, objc2 | vorhanden |
| Linux, Windows | neu bauen | Kein Platform Code | 28 |
| Auto-Updates | neu bauen | Kein Code | 28 |

---

## Extension-Capabilities

### Neue WIT Interfaces für `raijin:extension`

Diese Interfaces erweitern Phase 27 (Extension System Rewrite) um Terminal-spezifische Capabilities die Drittanbieter nutzen können.

#### 1. Rich Block Renderer

```wit
interface block-renderer {
    /// Extension registriert einen Block-Renderer für bestimmte Command-Patterns
    record block-renderer-registration {
        /// Regex auf Command-String (z.B. "^docker ps", "^kubectl get")
        command-pattern: string,
        /// Eindeutiger Name des Renderers
        name: string,
    }

    /// Host ruft Extension auf wenn ein matchender Block fertig ist
    render-block: func(command: string, output: string, exit-code: s32) -> rendered-block;

    record rendered-block {
        /// Inazuma UI-Element als serialisierte Beschreibung
        elements: list<ui-element>,
        /// Interaktive Actions die der User triggern kann
        actions: list<block-action>,
    }
}
```

**Use Cases:**
- Docker Extension → `docker ps` als Container-Karten mit Start/Stop/Logs Buttons
- Kubernetes Extension → `kubectl get pods` als Live-Status mit Sparklines
- CI/CD Extension → GitHub Actions Status als Pipeline-Visualisierung
- SQL Extension → Query-Results als sortierbare/filterbare Tabelle
- Terraform Extension → `terraform plan` als interaktiver Diff mit Apply-Button

#### 2. CLI Completion Provider

```wit
interface completion-provider {
    /// Extension registriert sich für bestimmte Commands
    record completion-registration {
        /// Command-Prefix (z.B. "kubectl", "terraform", "aws")
        command: string,
    }

    /// Host fragt Completions ab
    complete: func(line: string, cursor-position: u32) -> list<completion-item>;

    record completion-item {
        label: string,
        detail: option<string>,
        documentation: option<string>,
        insert-text: string,
    }
}
```

**Use Cases:**
- `kubectl` Extension → Alle Kubernetes Commands, Resources, Namespaces, Pod-Namen
- `terraform` Extension → HCL Completions, Provider-Resources, State-Referenzen
- `aws` Extension → Alle AWS CLI Commands mit Parametern und Docs
- `gcloud` Extension → Google Cloud Completions
- Firmeninterne CLI Extensions → Eigene Tools mit Completions ausstatten

#### 3. Output Transformer

```wit
interface output-transformer {
    /// Extension registriert sich für Output-Patterns
    record transformer-registration {
        /// Regex auf Command ODER Output
        pattern: string,
        /// Matcht auf Command-String oder Output-Content
        match-target: match-target,
    }

    enum match-target {
        command,
        output,
    }

    /// Host ruft Transformer auf
    transform: func(command: string, output: string) -> transformed-output;

    record transformed-output {
        /// Ersetze den Output komplett oder füge Rich-UI hinzu
        mode: transform-mode,
        elements: list<ui-element>,
    }

    enum transform-mode {
        replace,     // Ersetze gesamten Output
        append,      // Füge unter dem Output hinzu
        overlay,     // Zeige als Overlay über dem Output
    }
}
```

**Use Cases:**
- JSON Formatter → `curl` Output als faltbarer, syntax-highlighted Tree
- Test Runner → pytest/jest/cargo-test als interaktive Checklist mit Rerun-Buttons
- Build Error Formatter → Compiler-Errors als klickbare Diagnostics die im Editor öffnen
- CSV Viewer → CSV-Output als sortierbare Tabelle
- Diff Enhancer → Git-Diff als Side-by-Side mit Syntax Highlighting
- Log Viewer → Structured Logs (JSON) als filterbare/durchsuchbare Tabelle

#### 4. Terminal Overlay App

```wit
interface overlay-app {
    /// Extension registriert eine Overlay-App
    record app-registration {
        name: string,
        /// Slash-Command zum Starten (z.B. "/files", "/monitor")
        slash-command: string,
        /// Keyboard Shortcut
        shortcut: option<string>,
    }

    /// Host startet die App — Extension bekommt ein Canvas
    start: func(width: u32, height: u32) -> app-handle;

    /// Extension rendert ein Frame
    render: func(handle: app-handle) -> list<ui-element>;

    /// Host sendet Input-Events an die App
    handle-input: func(handle: app-handle, event: input-event);

    /// Extension beendet die App
    stop: func(handle: app-handle);
}
```

**Use Cases:**
- File Manager Extension → GPU-gerenderter `ranger` mit Previews und Icons
- Database Browser → SQL Editor + Results als native Tabellen-UI
- API Client → HTTP Requests bauen und Results inspizieren (wie Postman im Terminal)
- System Monitor → CPU/RAM/Network als interaktive Charts
- Git UI Extension → Stage/Commit/Push als visuelles Overlay
- Log Analyzer → Log-Files als filterbare, durchsuchbare Timeline

#### 5. Protocol Handler

```wit
interface protocol-handler {
    /// Extension registriert URL-Schemas
    record protocol-registration {
        /// URL Schema (z.B. "jira", "figma", "slack", "linear")
        scheme: string,
    }

    /// Host ruft Extension auf wenn User auf Link klickt
    handle-url: func(url: string) -> protocol-action;

    variant protocol-action {
        /// Öffne externe URL im Browser
        open-external(string),
        /// Zeige Inline-Preview im Terminal
        show-preview(list<ui-element>),
        /// Führe Command aus
        run-command(string),
    }
}
```

**Use Cases:**
- Jira Extension → `PROJ-123` in Terminal-Output wird klickbar, zeigt Issue-Preview
- Figma Extension → `figma://file/xxx` öffnet Figma oder zeigt Thumbnail
- Linear Extension → Issue-Links werden zu Inline-Previews
- Sentry Extension → Error-Links zeigen Stack-Trace als Preview
- Confluence Extension → Wiki-Links als Markdown-Preview

#### 6. AI Tool Provider

```wit
interface ai-tool-provider {
    /// Extension stellt Tools für den AI Agent bereit
    record tool-registration {
        name: string,
        description: string,
        /// JSON Schema für die Parameter
        parameters-schema: string,
    }

    /// Agent ruft das Tool auf
    invoke-tool: func(name: string, parameters: string) -> tool-result;

    record tool-result {
        success: bool,
        output: string,
        /// Optional: Rich-UI als Ergebnis
        elements: option<list<ui-element>>,
    }
}
```

**Use Cases:**
- Vercel Extension → "Deploy to Vercel" als Agent-Tool
- Datadog Extension → "Query Metrics" als Agent-Tool
- PagerDuty Extension → "Acknowledge Incident" als Agent-Tool
- Internal API Extension → Firmeninterne APIs als Agent-Tools
- Stripe Extension → "Check Payment Status" als Agent-Tool

#### 7. Notification Integration

```wit
interface notification-provider {
    /// Extension registriert sich als Notification-Kanal
    record notification-registration {
        name: string,
        /// z.B. "slack", "teams", "discord", "pagerduty"
        channel-type: string,
    }

    /// Host routet Terminal-Events an Extension
    notify: func(event: terminal-event) -> notify-result;

    record terminal-event {
        event-type: event-type,
        command: option<string>,
        exit-code: option<s32>,
        duration-ms: option<u64>,
        output-summary: option<string>,
    }

    enum event-type {
        command-completed,
        command-failed,
        long-running-finished,
        custom,
    }
}
```

**Use Cases:**
- Slack Extension → "Build fertig" als Slack-Message wenn Terminal im Hintergrund
- Teams Extension → Failed Commands an Teams Channel
- Discord Extension → Deploy-Status an Discord Webhook
- PagerDuty Extension → Critical Errors triggern PagerDuty Alert
- Email Extension → Tägliche Zusammenfassung der Terminal-Aktivität

#### 8. Hardware / Serial Connection

```wit
interface serial-connection {
    /// Extension stellt Serial-Port-Verbindung her
    connect: func(port: string, baud-rate: u32, config: serial-config) -> connection-handle;

    record serial-config {
        data-bits: u8,
        stop-bits: u8,
        parity: parity,
        flow-control: flow-control,
    }

    /// Daten senden/empfangen
    write: func(handle: connection-handle, data: list<u8>);
    read: func(handle: connection-handle) -> list<u8>;

    /// Verbindung beenden
    disconnect: func(handle: connection-handle);
}
```

**Use Cases:**
- Arduino Extension → Serial Monitor im Terminal
- Embedded Extension → UART/SPI Debugging
- IoT Extension → Sensor-Daten empfangen und visualisieren
- 3D Printer Extension → G-Code senden und Status überwachen

#### 9. Theme Generator

```wit
interface theme-generator {
    /// Extension generiert ein Theme
    generate: func(input: theme-input) -> theme-output;

    variant theme-input {
        /// Generiere aus einem Wallpaper
        from-image(list<u8>),
        /// Generiere aus Brand Colors
        from-colors(list<string>),
        /// Generiere basierend auf Uhrzeit/Saison
        from-context(context-info),
    }

    record context-info {
        hour: u8,
        month: u8,
        latitude: option<f64>,
        longitude: option<f64>,
    }

    record theme-output {
        name: string,
        /// TOML-String des generierten Themes
        theme-toml: string,
    }
}
```

**Use Cases:**
- Wallpaper Theme Extension → Extrahiert Farben aus Desktop-Wallpaper, generiert passendes Theme
- Brand Theme Extension → Firmenfarben rein → vollständiges OKLCH Theme raus
- Seasonal Extension → Theme ändert sich mit Tageszeit/Jahreszeit
- Album Art Extension → Theme basierend auf aktuell spielendem Song

#### 10. Custom Widget

```wit
interface custom-widget {
    /// Extension registriert ein Widget für die Sidebar/StatusBar
    record widget-registration {
        name: string,
        /// Wo das Widget angezeigt wird
        position: widget-position,
        /// Wie oft das Widget aktualisiert wird (ms)
        refresh-interval-ms: u32,
    }

    enum widget-position {
        status-bar,
        sidebar,
        context-chip,
    }

    /// Host fragt Widget-Content ab
    render: func() -> list<ui-element>;

    /// User interagiert mit Widget
    handle-action: func(action-id: string);
}
```

**Use Cases:**
- Pomodoro Extension → Timer-Widget in StatusBar
- Spotify Extension → Now Playing als Context Chip
- Weather Extension → Wetter-Widget in Sidebar
- Stock Ticker Extension → Aktien/Crypto Preise in StatusBar
- Calendar Extension → Nächster Termin als Context Chip

---

## Shared UI Element Type

Alle Extension-Interfaces nutzen einen gemeinsamen `ui-element` Typ für Rich-Rendering:

```wit
record ui-element {
    element-type: element-type,
    children: list<ui-element>,
    text: option<string>,
    style: option<element-style>,
    on-click: option<string>,  // Action-ID
}

enum element-type {
    div, text, button, icon, badge, progress-bar, table, table-row, table-cell,
    chart, image, code-block, link, separator, spinner, toggle,
}

record element-style {
    color: option<string>,      // OKLCH
    background: option<string>, // OKLCH
    padding: option<u32>,
    margin: option<u32>,
    border: option<string>,
    font-weight: option<string>,
    font-size: option<u32>,
    width: option<string>,
    height: option<string>,
    flex-direction: option<string>,
    gap: option<u32>,
}
```

---

## Priorisierung

### Sofort (mit Phase 27 Extension Rewrite)
- CLI Completion Provider — höchster Drittanbieter-Wert
- Output Transformer — macht Terminal sofort mächtiger
- AI Tool Provider — erweitert Agent-Capabilities

### Danach
- Rich Block Renderer — braucht UI Element Serialisierung
- Protocol Handler — relativ einfach
- Custom Widget — erweitert UI

### Langfristig
- Terminal Overlay App — komplex, braucht Canvas-API
- Hardware/Serial Connection — Niche
- Theme Generator — Nice-to-have
- Notification Integration — Nice-to-have

## Abhängigkeiten

- Phase 27 (Extension System Rewrite) muss fertig sein
- Phase 20 (Workspace) für Widget-Positionen
- Phase 25 (Terminal-Editor Fusion) für Block Renderer Integration
- Phase 26 (ADE) für AI Tool Provider
