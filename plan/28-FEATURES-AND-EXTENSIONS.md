# Phase 28: Feature-Roadmap & Extension-Capabilities

## Übersicht

Vollständige Feature-Roadmap basierend auf `plan/features.md` (konsolidiert aus Warp, WezTerm, Wave Terminal). Jedes Feature ist als **Core** (wir bauen es) oder **Extension** (Drittanbieter bauen es via WASM Extensions) klassifiziert.

## Core Features

### KI & Automatisierung (→ Phase 25, 26)

Alles Core — KI ist das Herz von Raijin's ADE.

| Feature | Status | Phase |
|---|---|---|
| KI-gestützte Befehlsgenerierung aus natürlicher Sprache | Infrastruktur da (raijin-agent) | 26 |
| KI-Fehleranalyse bei fehlgeschlagenen Befehlen | Smart Block Diagnostics | 25E |
| KI-Agent-Modus (mehrstufige Ziele, planen + ausführen) | Phase 26 ADE | 26 |
| KI-Kontextfenster (CWD, History, Errors, Env) | Block Context System | 26B |
| Lokale Modelle (Ollama, LM Studio, vLLM) | Crates vorhanden | - |
| OpenAI-kompatible Endpoints | raijin-open-ai | - |
| Gemini, Claude, Azure, OpenRouter | Provider-Crates vorhanden | - |
| Mehrere KI-Profile / Modi | Agent Profiles | 26A |
| BYOK (Bring Your Own Key) | Settings | 19 |
| KI-Vision / Multimodal (Bilder, PDFs) | Agent Feature | 26 |
| KI kann Dateien erstellen/bearbeiten (Diff + Rollback) | Interactive Code Review | 26E |
| Drag & Drop Dateien in KI-Chat | Agent UI | 26 |
| KI-Zugriff auf Scrollback, Dateisystem | Full Terminal Use | 26C |
| KI-Tools: Websuche, Dateioperationen | raijin-web-search, raijin-acp-tools | - |
| CLI-Output an KI senden | Block Context Attachment | 26B |
| KI Thinking Mode (Quick / Balanced / Deep) | Model Selection | 26 |
| Stop-Generierung | Agent UI Control | 26 |
| Feedback-Buttons (Daumen hoch/runter) | Agent UI | 26 |

### Terminal-Kernfunktionen (→ Phase 20, raijin-term)

Alles Core — das ist was Raijin als Terminal SEIN muss.

| Feature | Status | Phase |
|---|---|---|
| GPU-beschleunigtes Rendering | Inazuma Metal/WGPU | vorhanden |
| Vollständige xterm-Kompatibilität | raijin-term | vorhanden |
| Tabs mit Drag & Drop, Umbenennung, Kontextmenü | Workspace Pane System | 20 |
| Vertikale und horizontale Tab-Leiste | Tab Variants (IC merge) | 24 |
| Splits / Panes (horizontal & vertikal) | Workspace Pane Splits | 20 |
| Vim-artige Pane-Navigation (Ctrl+Shift+H/J/K/L) | Workspace Keybindings | 20 |
| Mehrere Fenster | Multi-Window Support | 20 |
| Scrollback mit konfigurierbarer Zeilenzahl | raijin-settings | vorhanden |
| Suchbare Scrollback-Funktion (Cmd+F) | Terminal Search | 25B |
| Semantische Prompt-Navigation (OSC 133) | Block System | vorhanden |
| Command Blocks als navigierbare Einheiten | Block System | vorhanden |
| Block-Badges (Icon, Farbe, Priorität) | Block Headers | vorhanden |
| Multiline-Eingabe (Shift+Enter) | Input Bar | vorhanden |
| Multi-Input-Modus (gleichzeitig in alle Terminals) | Broadcast Input | 28 |
| Cursor-Style und Blink einstellbar | raijin-settings | vorhanden |
| Copy-on-Select | Settings | 28 |
| OSC 52 Clipboard-Support | raijin-term | 28 |
| Bracketed Paste Mode | raijin-term | vorhanden |
| Hyperlinks im Terminal (klickbar) | Link Detection | 28 |
| Text-Attribute (Underline, Italic, Bold, Strikethrough) | raijin-term | vorhanden |
| SGR-Maus-Reporting | raijin-term | vorhanden |
| Inline-Bilder (iTerm2 Image Protocol) | raijin-term | 28 |
| Inline-Bilder (Kitty Graphics Protocol) | raijin-term | 28 |
| Sixel-Grafiken | raijin-term | 28 |
| Ligatures, Color Emoji, Font-Fallback | Inazuma Text System | vorhanden |
| True Color / 24-Bit | raijin-term | vorhanden |
| Themes (Dark/Light, Custom) | Theme System OKLCH | vorhanden |
| Hintergrundbilder und Transparenz | Theme Background Image | vorhanden |
| Anpassbares Padding | Settings | 28 |
| Font-Size pro Block einstellbar | Block Rendering | 28 |
| Konfigurierbare FPS (120 FPS) | Inazuma Render Loop | 28 |
| Audible Bell deaktivierbar | Settings | 28 |
| IME-Unterstützung (CJK) | Inazuma IME | vorhanden |
| Vollbild-Modus | Window Management | 28 |
| Bell-Indikator als Badge | Tab Badge | 28 |
| Fokus-folgt-Cursor | Settings | 28 |

### Editor & Dateimanagement (→ Phase 25, raijin-editor)

| Feature | Status | Phase |
|---|---|---|
| Eingebauter Code-Editor | raijin-editor (von Zed) | vorhanden |
| Syntax-Highlighting, Fehleranzeige | raijin-editor + LSP | vorhanden |
| Visueller Diff-Viewer | raijin-streaming-diff | vorhanden |
| Datei-Rollback nach KI-Edits | Agent Code Review | 26E |
| Datei-Vorschau: Bilder, Markdown | raijin-image-viewer, raijin-markdown-preview | vorhanden |
| Datei-Vorschau: Audio/Video, PDFs | Media Preview | 28 |
| Verzeichnis-Browser | raijin-project-panel | vorhanden |
| Drag & Drop Dateien | Workspace DnD | 20 |
| Scrollback in Datei speichern | Block Export | 28 |
| Bildeinfügen per Paste | Image Handling | 28 |
| Quick Look (macOS) | Platform Integration | 28 |
| Dateien im externen Explorer öffnen | Platform Integration | 28 |

### SSH & Remote-Verbindungen (→ raijin-remote)

| Feature | Status | Phase |
|---|---|---|
| SSH-Verbindungsmanagement mit Profilen | raijin-remote | vorhanden |
| Durable SSH Sessions | raijin-remote Transport | 28 |
| Auto-Reconnect | raijin-remote | 28 |
| Shell-State bei Disconnect erhalten | raijin-remote | 28 |
| Status-Anzeigen (Attached/Detached) | Remote UI | 28 |
| SSH Key Management / Agent Forwarding | raijin-askpass | vorhanden |
| SSH-Passwörter im Secret Store | raijin-credentials-provider | vorhanden |
| WSL2 Support | raijin-remote Transport | 28 |
| Remote-Dateien im Editor | raijin-remote + raijin-editor | 28 |
| Drag & Drop lokal ↔ remote | Remote DnD | 28 |
| Per-Connection Themes | Settings Override | 28 |

### Multiplexer & Workspace (→ Phase 20)

| Feature | Status | Phase |
|---|---|---|
| Eingebauter Multiplexer (Tabs, Splits, Sessions) | raijin-workspace | 20 |
| Workspaces mit eigenen Layouts/Settings | Workspace Persistence | 20 |
| Tab-Close-Bestätigung | Workspace Settings | 20 |
| Bestätigung beim Beenden mit aktiven Sessions | Workspace | 20 |
| Session-Wiederherstellung nach Neustart | Workspace Persistence | 20 |
| Zoom einzelner Blöcke | Block Zoom | 28 |

### Shell-Integration & Produktivität (→ raijin-shell, raijin-completions)

| Feature | Status | Phase |
|---|---|---|
| Shell-Integration bash, zsh, fish, pwsh | Shell Hooks | vorhanden |
| OSC 7 CWD Tracking | raijin-terminal | vorhanden |
| Shell Context Tracking (Status, Exit-Code) | OSC 133 Block System | vorhanden |
| Env-Variablen und Init-Scripts konfigurierbar | Settings | 28 |
| History-basierte Autovervollständigung | raijin-completions | vorhanden |
| Subcommand-Completions für Standard-CLIs (git, docker, cargo) | raijin-completions Specs | vorhanden |
| Fuzzy-Matching für Dateipfade | inazuma-fuzzy | vorhanden |
| Tab-Navigator | raijin-tab-switcher | vorhanden |
| CLI-Steuerung von außen (wie wsh) | raijin-cli | 28 |
| Desktop-Benachrichtigungen | raijin-notifications | 28 |
| Secret Store | raijin-credentials-provider | vorhanden |
| Variablen über Sessions hinweg | raijin-session | 28 |
| Globale Hotkey-Unterstützung | Platform Integration | 28 |

### Konfiguration & Anpassung (→ Phase 19, Settings)

| Feature | Status | Phase |
|---|---|---|
| Hot-Reload der Konfiguration | Settings Watcher | vorhanden |
| Settings-GUI | raijin-settings-ui | vorhanden |
| Themes inkl. Presets | Theme System + Assets | vorhanden |
| Kitty Keyboard Protocol | raijin-term | 28 |
| Anpassbare Keybindings | Keymap TOML | vorhanden |
| Option Key als Meta Key (macOS) | Settings | 28 |
| Anpassbare Tab-Farben/Icons/Titel | Tab API | 20 |

### Plattform & Integration

| Feature | Status | Phase |
|---|---|---|
| macOS, Linux, Windows | Inazuma Platform | vorhanden (macOS), 28 (Linux/Windows) |
| Auto-Updates | raijin-auto-update | 28 |

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
