Hier ist der vollständige Inhalt der Datei :

***

# Terminal Features – Konsolidierte Liste
> Zusammengefasst aus Warp, WezTerm und Wave Terminal. Keine Duplikate, keine Zuordnung.

***

## KI & Automatisierung

- KI-gestützte Befehlsgenerierung aus natürlichsprachlichen Beschreibungen
- KI-Fehleranalyse: Automatische Erklärung und Korrekturvorschlag bei fehlgeschlagenen Befehlen
- KI-Agent-Modus: Mehrstufige Ziele beschreiben, KI plant und führt Befehlssequenz aus
- KI-Kontextfenster: Zugriff auf aktuelles Verzeichnis, Befehlshistorie, Fehlerausgaben und Umgebungsvariablen
- Unterstützung für lokale Modelle via Ollama / LM Studio / vLLM (BYOK)
- Unterstützung für OpenAI-kompatible API-Endpunkte (`/v1/chat/completions`)
- Unterstützung für Google Gemini, Anthropic Claude, Azure AI, OpenRouter, Groq, NanoGPT
- Mehrere KI-Profile / Modi – einfacher Wechsel zwischen Modellen und Providern
- BYOK (Bring Your Own Key) ohne Telemetrie-Pflicht
- KI-Vision / Bildverarbeitung (Multimodal: Bilder, PDFs, Textdateien als Anhang)
- KI kann Dateien erstellen und bearbeiten (mit Diff-Vorschau und Rollback)
- Drag & Drop von Dateien in den KI-Chat
- KI-Zugriff auf Terminal-Scrollback, Dateisystem und Widgets
- Eingebaute KI-Tools: Websuche, lokale Dateioperationen, Widget-Screenshots, Webnavigation
- Befehlsausgabe direkt per CLI an KI senden (`wsh ai`)
- KI Thinking Mode (Quick / Balanced / Deep)
- Stop-Generierung mitten in der KI-Antwort
- Feedback-Buttons (Daumen hoch/runter) für KI-Antworten

***

## Editor & Dateimanagement

- Eingebauter Code-Editor (Monaco) für lokale und Remote-Dateien
- Syntax-Highlighting, JSON/YAML-Fehleranzeige (Squiggly Lines) im Editor
- Visueller Diff-Viewer vor dem Bestätigen von KI-Dateiänderungen
- Datei-Rollback nach KI-Edits
- Datei-Vorschau: Bilder, Markdown, Audio/Video, PDFs direkt im Terminal
- Verzeichnis-Browser mit Erstellung, Umbenennung und Löschung von Dateien/Ordnern
- Drag & Drop von Dateien zwischen lokalen und Remote-Verzeichnissen
- Speichern von Terminal-Scrollback in eine Datei
- Bildeinfügen per Paste direkt ins Terminal (wird als Temp-Datei gespeichert)
- Quick Look-Integration für Dateien (macOS)
- Dateien im externen Datei-Explorer öffnen
- Web Bookmarks (editierbar in `bookmarks.json`, öffenbar im Web-Widget)

***

## Web & Inline-Widgets

- Eingebetteter Webbrowser direkt im Terminal (Web-Widget)
- Suche innerhalb von Web-Widgets (Cmd-F)
- Einstellbarer Zoom-Level für Web-Widgets
- Audio stumm schalten in Web-Widgets
- Bilder per Rechtsklick speichern im Web-Widget
- Mermaid-Diagramme in Markdown-Blöcken rendern
- System-Informations-Widget: CPU-Graphen (pro Kern), Speicher-Graphen, Netzwerk
- Eingebettete Dokumentations-Seite (Help-View) direkt im Terminal

***

## Terminal-Kernfunktionen

- GPU-beschleunigtes Rendering
- Vollständige xterm-Kompatibilität
- Tabs mit Drag & Drop, Umbenennung, Kontextmenü
- Vertikale und horizontale Tab-Leiste (umschaltbar)
- Splits / Panes (horizontal & vertikal)
- Vim-artige Navigation zwischen Panes (Ctrl+Shift+H/J/K/L)
- Mehrere Fenster
- Scrollback mit konfigurierbarer Zeilenzahl (bis 50.000)
- Suchbare Scrollback-Funktion (Cmd/Ctrl+F)
- Semantische Prompt-Navigation (springe per Keybinding zu vorherigen Befehlen)
- Command Blocks: Ausgaben werden als isolierte, navigierbare Einheiten dargestellt
- Block-Badges mit Icon, Farbe und Priorität (rollen in Tab-Leiste auf)
- Bell-Indikator als Badge (konfigurierbar)
- Multiline-Eingabe (Shift+Enter für neue Zeile ohne `\`-Fortsetzungszeichen)
- Multi-Input-Modus: Gleichzeitige Eingabe in alle Terminals eines Tabs
- Cursor-Style und Blink einstellbar (Block / Bar / Underline), auch per Block
- Fokus-folgt-Cursor (Hover-basierter Fokus, konfigurierbar)
- Copy-on-Select (bei Textmarkierung automatisch in Clipboard kopieren)
- OSC 52 Clipboard-Support (Terminal-Apps können direkt ins Clipboard schreiben)
- Bracketed Paste Mode
- Hyperlinks im Terminal (klickbar)
- Underline, Double-Underline, Italic, Bold, Strikethrough als Render-Attribute
- SGR-Maus-Reporting (kompatibel mit vim, tmux)
- Inline-Bilder via iTerm2 Image Protocol
- Inline-Bilder via Kitty Graphics Protocol
- Sixel-Grafiken
- Ligatures, Color Emoji und Font-Fallback
- True Color / 24-Bit-Farbunterstützung
- Dynamische Farbschemata / Themes (inkl. Dark/Light-Mode-Erkennung)
- Hintergrundbilder und Transparenz (Fenster-Hintergrund-Opacity)
- Tab-Hintergrund-Presets (konfigurierbar per `backgrounds.json`)
- Anpassbares Padding
- Font-Size individuell pro Block einstellbar
- Konfigurierbare FPS (z. B. 120 FPS)
- Audible Bell deaktivierbar
- IME-Unterstützung (CJK: Chinesisch, Japanisch, Koreanisch)
- Vollbild-Modus

***

## SSH & Remote-Verbindungen

- SSH-Verbindungsmanagement mit Profilen / Verbindungskonfiguration
- Durable SSH Sessions: Sessions überleben Netzwechsel, Sleep und Neustart
- Automatische Wiederverbindung nach Unterbrechung
- Shell-Zustand, laufende Programme und Terminal-Historie bleiben bei Disconnect erhalten
- Visuelle Statusanzeigen für Session-Zustand (Attached / Detached / Awaiting)
- Connection Keepalives und Stalled-Connection-Erkennung
- SSH Identity / Key Management und Agent Forwarding
- SSH-Passwörter im Secret Store speichern (kein Re-Typing)
- WSL2-native Unterstützung (Windows Subsystem for Linux)
- Git Bash Auto-Erkennung (Windows)
- Remote-Dateien direkt im eingebauten Editor öffnen und bearbeiten
- Drag & Drop von Dateien zwischen lokalen und Remote-Maschinen
- Per-Connection-Themes und Konfigurationsoverrides
- SSH-Verbindungen ohne wsh als Fallback
- Serielle Port-Verbindungen (für Embedded / Arduino)
- Verbindung zu lokalem Multiplexer über Unix Domain Sockets
- Verbindung zu Remote-Multiplexer via SSH oder TLS/TCP

***

## Multiplexer & Workspace-Organisation

- Eingebauter Terminal-Multiplexer (Tabs, Splits, Sessions nativ)
- Workspaces: Separate Umgebungen mit eigenen Tabs, Layouts und Einstellungen
- Tab-Close-Bestätigung (konfigurierbar)
- Bestätigung beim Beenden mit aktiven Sessions
- Workspace-spezifische Widgets
- Session-Wiederherstellung nach Neustart (Tab-Cache, Scrollposition, Editor-State)
- Zoom / Magnify einzelner Blöcke

***

## Shell-Integration & Produktivität

- Shell-Integration für bash, zsh, fish, pwsh
- OSC 7 Support: Automatisches Tracking und Wiederherstellen des aktuellen Verzeichnisses
- Shell Context Tracking: Erkennung von Bereitschaftsstatus, letztem Befehl und Exit-Code
- Environment-Variablen und Init-Scripts konfigurierbar (pro Block und pro Connection)
- History-basierte Autovervollständigung
- Subcommand- und Flag-Completions mit Beschreibungen (z. B. für `git`, `docker`)
- Fuzzy-Matching für Dateipfade
- Tab-Navigator (Überblick über alle Tabs)
- `wsh`-CLI für Terminal-Steuerung von außen (Blöcke erstellen, Dateien senden, Badges setzen etc.)
- `wsh run`: Befehle in dedizierten Blöcken starten (mit Magnification, Auto-Close, Execution Control)
- `wsh notify`: Desktop-Benachrichtigungen aus dem Terminal senden
- Secret Store: Sichere Speicherung und Verwaltung von Credentials per CLI (`wsh secret`)
- Variablen setzen und abrufen über Sessions hinweg (`wsh setvar/getvar`)
- Globale Hotkey-Unterstützung

***

## Konfiguration & Anpassung

- Hot-Reload der Konfigurationsdatei (Änderungen sofort wirksam ohne Neustart)
- Lua-Skripting für vollständige Konfiguration und Automatisierung
- JSON-Schema-Unterstützung für Konfigurationsdateien (Autocomplete im Editor)
- Einheitliches Konfigurations-Widget mit GUI
- Mehrere Farbschemata / Themes inkl. bekannter Presets (z. B. One Dark Pro)
- Kitty Keyboard Protocol (ermöglicht Tastenkürzel, die sonst unmöglich sind)
- Vollständig anpassbare Keybindings
- Benutzerdefinierte Widgets (definierbar in `widgets.json`)
- Hintergrundbilder per `wsh setbg` setzen
- Option Key als Meta Key (macOS, konfigurierbar)
- Ctrl-V als Paste auf Windows (konfigurierbar)
- Anpassbare Tab-Farben, Icons und Titel per Metadaten

***

## Plattform & Integration

- Läuft auf macOS, Linux, Windows und FreeBSD
- Snap-Paket für Linux (Snap Store)
- Windows Package Manager (`winget install`)
- Automatische Updates (mit Beta-Kanal)
- Microphone / Camera / Location-Zugriff für CLI-Apps (macOS Security Sandbox)
- Perplexity API Integration
- Azure AI Integration
- Anthropic API Integration (Claude-Modelle)
