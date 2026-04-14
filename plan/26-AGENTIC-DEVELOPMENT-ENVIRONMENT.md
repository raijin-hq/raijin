# Phase 26: Agentic Development Environment — Warp-Style Agent System

> **STATUS: Vision-Dokument** — wird durch **Station 11** in `25-TERMINAL-EDITOR-FUSION.md` umgesetzt. Verschoben nach `done/` wenn Station 11 fertig ist.
>
> Wer Station 11 implementiert, liest dieses Dokument als Anforderungs-Spec. Die hier beschriebenen Features (Zwei-Modi-System, Block-Context, Full Terminal Use, /plan, Code Review, Conversation Management, Cloud Agents) sind das **Ziel** von Station 11 — die konkrete Reihenfolge und Vorbedingungen stehen aber in Plan 25.
>
> **Wichtige Korrektur zur Inventur unten:** Die LOC-Zahlen sind aus der ursprünglichen Schätzung. Verifiziert gegen den Code (April 2026):
> - `raijin-streaming-diff` ist tatsächlich nur ~1k LOC (eine Datei, 35 KB) — die Tabelle unten stimmt
> - Alle gelisteten Crates existieren als Source-Code, **keines** ist heute in `raijin-app/Cargo.toml` geladen
> - Die Verdrahtung in die App ist Hauptarbeit von Station 11, nicht das Trait-Design

---

## Vision

Raijin's Agent System vereint Warp's ADE-Konzept (Terminal-native, Full Terminal Use, Conversation Management) mit unserer existierenden AI-Infrastruktur (231k Zeilen AI/Agent-Code von der Referenz-Codebase). Das Ergebnis: Ein Terminal-first Agent das sowohl interaktive Conversations als auch autonome Cloud Agents unterstützt.

## Was wir haben (von der Referenz-Codebase)

| Crate | Zeilen | Was es macht |
|---|---|---|
| `raijin-agent` | 98.754 | Agent-Kern: Thread-Management, Tool-Calling, Context, Eval |
| `raijin-agent-ui` | 61.679 | Conversation View, Agent Panel, Configuration Modals |
| `raijin-agent-settings` | 1.236 | Agent Profile Settings |
| `raijin-agent-servers` | 2.993 | Agent Server Management |
| `raijin-acp-thread` | 7.639 | Agent Communication Protocol Thread |
| `raijin-acp-tools` | 615 | Tool-Definitionen für Agents |
| `raijin-language-model` | 4.015 | LLM Abstraction Layer |
| `raijin-language-models` | 18.193 | Multi-Provider LLM Registry |
| `raijin-assistant-slash-command` | 959 | Slash Command Framework |
| `raijin-assistant-slash-commands` | 2.672 | Built-in Slash Commands |
| `raijin-assistant-text-thread` | 6.086 | Text-basierte Conversations |
| `raijin-prompt-store` | 1.373 | Prompt Templates |
| `raijin-rules-library` | 1.444 | Agent Rules/Guidelines |
| `raijin-streaming-diff` | 1.104 | Streaming Code Diffs |
| `raijin-edit-prediction` | 13.410 | Code/Command Prediction |
| 13 AI Provider Crates | ~8.000 | Anthropic, OpenAI, Ollama, Mistral, etc. |
| **Gesamt** | **~231.000** | |

## Was Warp hat, wir noch nicht

### 1. Terminal Mode + Agent Mode (Zwei-Modi-System)
- **Terminal Mode:** Sauberes Shell-Interface, Agent-Controls versteckt, `Enter` = Shell Command, `Cmd+Enter` = an Agent senden
- **Agent Conversation View:** Dediziertes Panel mit Toolbelt (Model Selector, Voice, Image), eigene Block-Isolation
- **Message Bar:** Kontextuelle Hinweise unten ("Cmd+Enter for new agent", "Cmd+↑ attach output as context")

### 2. Block-Kontext-System
- Terminal-Blöcke vs Agent-Konversations-Blöcke — getrennte Kontexte
- Terminal-Block-Output als Kontext an Agent anhängen (Cmd+↑)
- Agent-Blöcke erscheinen NUR in ihrer Konversation, nicht in der Terminal-Blockliste
- Fehlgeschlagene Commands automatisch als Agent-Kontext anbieten

### 3. Full Terminal Use
- Agent steuert Terminal interaktiv: GDB, PostgreSQL REPL, `top`, Debugger
- Alles sichtbar im Vordergrund — kein Hintergrund-Magie
- User kann jederzeit übernehmen oder anleiten

### 4. /plan Mode
- Strukturierte Planung vor Code-Ausführung
- Kollaborativ editierbar (Agent + User)
- Versioniert, persistiert, teilbar via Links
- Referenzierbar mit `@plan` in späteren Conversations

### 5. Interactive Code Review
- Agent macht Änderungen → Live-Diffs im Code Review Panel
- User hinterlässt Inline-Kommentare
- Agent behebt alle Kommentare in einem Pass
- Loop bis User zufrieden ist

### 6. Conversation Management
- Conversation Panel: Browsen, Suchen, Filtern
- Forking: Konversation branchen
- Compacting: Kontext zusammenfassen
- History: Alle vergangenen Conversations

### 7. Cloud Agents (Oz)
- Autonome Agents in isolierter Cloud-Umgebung
- Getriggert von Slack, Linear, GitHub Actions
- Real-time Sharing-Link
- Agent Management View

## Architektur: Was umbauen, was behalten

### Behalten und erweitern:
- `raijin-agent` — Agent-Kern bleibt, bekommt Terminal-Tool-Support
- `raijin-language-model` + `raijin-language-models` — LLM Abstraction bleibt
- Alle 13 AI Provider Crates — bleiben unverändert
- `raijin-acp-thread` + `raijin-acp-tools` — Agent Communication Protocol bleibt
- `raijin-prompt-store` — Prompt Templates bleiben
- `raijin-rules-library` — Agent Rules bleiben
- `raijin-streaming-diff` — Streaming Diffs bleiben
- `raijin-assistant-slash-command` — Slash Command Framework bleibt, wird erweitert

### Umbauen:
- `raijin-agent-ui` — Komplett umbauen zu Warp's Zwei-Modi-System
- `raijin-agent-settings` — Agent Profiles mit Permissions (wie Warp)
- `raijin-edit-prediction` + `raijin-edit-prediction-ui` — Wird zu Command Prediction im Terminal

### Neu bauen:
- **Terminal-Agent-Bridge** — Agent steuert Terminal (Full Terminal Use)
- **Block-Context-System** — Terminal-Blöcke als Agent-Kontext
- **Plan Mode** — /plan Slash Command + Plan Panel
- **Code Review Panel** — Live-Diffs von Agent-Änderungen
- **Conversation Manager** — Fork, Compact, History
- **Message Bar** — Kontextuelle Hinweise im Terminal Mode
- **Cloud Agent Infrastructure** — Oz-äquivalent (langfristig)

## Phasen

### Phase 26A: Zwei-Modi-System (Terminal Mode + Agent Mode)

**Ziel:** Terminal Mode zeigt sauberes Shell-Interface. `Cmd+Enter` oder `/agent` wechselt zu Agent Conversation View.

1. **Terminal Mode erweitern:**
   - Message Bar am unteren Rand: kontextuelle Hinweise
   - `Cmd+Enter` auf leerer Eingabe → neue Agent Conversation
   - `Cmd+Enter` mit Text → sende als Agent Prompt
   - `Cmd+↑` nach fehlgeschlagenem Command → attach Output als Agent-Kontext
   - Agent-Controls standardmäßig versteckt

2. **Agent Conversation View:**
   - Eigenes Panel (nicht der gleiche Bereich wie Terminal)
   - Toolbelt: Model Selector, Voice Input, Image Attachment
   - `Enter` = an Agent senden
   - `!command` = Shell-Command erzwingen
   - `Esc` = zurück zu Terminal Mode
   - Eigene Block-Isolation (Agent-Blöcke ≠ Terminal-Blöcke)

3. **Autodetection:**
   - Lokaler Classifier: erkennt NL vs Shell-Command
   - Zwei separate Toggles in Settings
   - Visueller Indikator "(autodetected)"
   - `Cmd+I` als Override-Toggle

**Basis:** `raijin-agent-ui` umbauen. Die Conversation View existiert schon (61k Zeilen), muss von Editor-Panel zu Terminal-integriertem Panel umgebaut werden.

### Phase 26B: Block-Context-System

**Ziel:** Terminal-Blöcke können als Agent-Kontext angehängt werden.

1. **Context-Attachment-API:**
   - `raijin-terminal` Block-Output als Text extrahieren (ANSI-bereinigt)
   - Kontext-Format: Command + Output + Exit Code + Duration
   - Automatisch: fehlgeschlagene Commands als Kontext anbieten
   - Manuell: User wählt Blöcke aus und hängt sie an

2. **Block-Trennung:**
   - Terminal-Blöcke bleiben in Terminal-Blockliste
   - Agent-Konversations-Blöcke erscheinen nur in ihrer Conversation
   - Shared Context: Terminal-Blöcke können in beliebige Conversations angehängt werden

3. **Integration mit raijin-agent:**
   - `raijin-acp-tools` bekommt `TerminalContextTool`
   - Agent kann Terminal-History als Kontext lesen
   - Agent kann auf spezifische Blöcke referenzieren

**Basis:** Nutzt unser Block-System (`raijin-terminal` BlockManager) + `raijin-agent`'s Context-System.

### Phase 26C: Full Terminal Use

**Ziel:** Agent kann das Terminal steuern wie ein Mensch — interaktive Programme, REPLs, Debugger.

1. **Terminal-Execution-Tool:**
   - Agent sendet Commands ans Terminal PTY
   - Agent liest Terminal-Output (Block-basiert)
   - Agent reagiert auf Prompts (y/n, Password, etc.)
   - Agent kann interaktive Programme steuern (GDB, psql, python REPL)

2. **Sichtbarkeit:**
   - Alles läuft im sichtbaren Terminal — kein Hintergrund
   - User sieht was Agent tippt in Echtzeit
   - User kann jederzeit mit Cmd+C unterbrechen oder selbst tippen

3. **Permissions:**
   - Agent Profiles mit konfigurierbaren Permissions
   - "Auto-execute safe commands" vs "Ask for approval"
   - Blocklist für gefährliche Commands (`rm -rf`, `sudo`, etc.)
   - Settings > AI > Agent Permissions

**Basis:** `raijin-acp-tools` bekommt `TerminalExecutionTool`. `raijin-terminal` PTY wird von Agent angesteuert.

### Phase 26D: /plan Mode

**Ziel:** Strukturierte Planung vor Code-/Command-Ausführung.

1. **Slash Command:**
   - `/plan <prompt>` → Agent erstellt Implementierungsplan
   - Plan wird in eigenem Panel angezeigt (neben Conversation)
   - Kollaborativ: User kann Plan editieren, Agent reagiert

2. **Plan Panel:**
   - Markdown-basiert mit Checkboxen
   - Versionierung (Plan V1, V2, V3...)
   - Persistiert in `raijin-session`
   - Referenzierbar mit `@plan` in späteren Conversations

3. **Plan-to-Execution:**
   - Agent arbeitet Plan Schritt für Schritt ab
   - Jeder Schritt wird als eigener Agent-Block gezeigt
   - User kann Schritte überspringen oder umordnen

**Basis:** `raijin-assistant-slash-command` Framework + neues Plan-Panel in `raijin-agent-ui`.

### Phase 26E: Interactive Code Review

**Ziel:** Agent macht Code-Änderungen, User reviewed inline.

1. **Code Review Panel:**
   - Split-View: Links Original, Rechts Agent-Änderung
   - Inline-Diff mit Syntax Highlighting
   - User kann Inline-Kommentare hinterlassen
   - "Accept", "Reject", "Request Changes" Buttons

2. **Review Loop:**
   - Agent macht Änderungen → Diffs erscheinen
   - User kommentiert → Agent liest Kommentare
   - Agent behebt → neue Diffs erscheinen
   - Loop bis User "Accept All" klickt

3. **Integration:**
   - Nutzt `raijin-streaming-diff` für Live-Diff-Updates
   - Nutzt `raijin-editor` für Diff-Rendering
   - Nutzt `raijin-git` für File-Status

**Basis:** `raijin-streaming-diff` + `raijin-editor` Diff-View + neues Review-Panel.

### Phase 26F: Conversation Management

**Ziel:** Conversations browsen, forken, kompaktieren.

1. **Conversation Panel:**
   - `Cmd+Y` → Conversation Selector
   - `Cmd+Shift+H` → Conversation List Panel
   - Suche, Filter (nach Datum, Status, Tags)
   - Delete, Rename, Pin

2. **Fork:**
   - `/fork` → brancht aktuelle Conversation
   - `/fork from <message>` → brancht ab bestimmtem Punkt
   - Fork in aktuellem oder neuem Pane

3. **Compact:**
   - `/compact` → fasst Conversation zusammen (spart Token/Kontext)
   - `/fork-and-compact` → Fork + automatische Zusammenfassung

4. **Slash Commands erweitern:**
   - `/agent`, `/new` → neue Conversation
   - `/plan` → Plan Mode
   - `/fork`, `/compact`, `/fork-and-compact`
   - `/model` → Model wechseln
   - `/conversations` → Panel öffnen

**Basis:** `raijin-assistant-text-thread` + `raijin-agent` Thread-Management.

### Phase 26G: Cloud Agents (Langfristig)

**Ziel:** Autonome Agents in Cloud-Umgebung, getriggert von externen Events.

1. **Cloud Agent Infrastructure:**
   - `raijin-collab` Server erweitern für Agent-Execution
   - Sandboxed Environments (Docker/VM)
   - Agent Management View (Status, Logs, Control)

2. **Trigger-System:**
   - GitHub Webhook → Agent startet
   - Slack Message → Agent startet
   - Cron/Schedule → Agent startet
   - API Endpoint → Agent startet

3. **Sharing:**
   - Real-time Sharing-Link für laufende Agents
   - Session-Trace an PR/Issue anhängen
   - Team-Mitglieder können live steuern

**Basis:** `raijin-collab` + `raijin-agent-servers` + neue Cloud-Infrastruktur.

## Mapping: Warp-Feature → Unsere Crates

| Warp Feature | Unsere Basis | Was umbauen/erweitern |
|---|---|---|
| Universal Input / Two-Mode System | `raijin-agent-ui` (61k Zeilen) | Von Editor-Panel zu Terminal-integriert |
| Agent Conversation View | `raijin-agent-ui` ConversationView | Toolbelt, Block-Isolation hinzufügen |
| Model Selector | `raijin-language-models` (18k) | UI-Widget in Toolbelt |
| Slash Commands | `raijin-assistant-slash-command` | `/plan`, `/fork`, `/compact` hinzufügen |
| Full Terminal Use | `raijin-acp-tools` + `raijin-terminal` | TerminalExecutionTool bauen |
| /plan Mode | `raijin-agent` + `raijin-prompt-store` | Plan Panel + Persistence |
| Code Review | `raijin-streaming-diff` + `raijin-editor` | Review Panel bauen |
| Conversation Management | `raijin-assistant-text-thread` (6k) | Fork, Compact, History UI |
| Agent Profiles/Permissions | `raijin-agent-settings` (1.2k) | Erweitern mit Permissions |
| Block Context | `raijin-terminal` BlockManager | Context-Extraction-API |
| AI Providers | 13 Provider Crates (8k) | Keine Änderung |
| Cloud Agents | `raijin-collab` + `raijin-agent-servers` | Server-Side Agent Execution |
| Command Prediction | `raijin-edit-prediction` (13k) | Von Code → Shell Commands |

## Abhängigkeiten

- Phase 20 (Workspace Integration) — TerminalPane als Item
- Phase 25 (Terminal-Editor Fusion) — Traits für Terminal-aware Features
- Phase 24 (Component Consolidation) — Einheitliche UI

## Risiken

1. **Scope:** 231k Zeilen AI-Code umbauen ist riesig — Sub-Phasen unabhängig voneinander machbar
2. **UX:** Zwei-Modi-System kann User verwirren (Warp's Community-Kritik) — gute Defaults + konfigurierbar
3. **Full Terminal Use:** Agent muss Terminal-Output parsen — fragil bei verschiedenen Shells/Locales
4. **Cloud Agents:** Braucht Server-Infrastruktur — langfristiges Ziel, nicht MVP
5. **Performance:** Agent-Conversations mit vielen Blöcken können Context-Window sprengen — Compact-Feature hilft

## Erfolgs-Kriterien

Phase 26 ist fertig wenn:
- [ ] `Cmd+Enter` öffnet Agent Conversation View aus Terminal Mode
- [ ] Agent Conversation View hat Toolbelt (Model, Voice, Image)
- [ ] `!command` führt Shell-Command im Agent View aus
- [ ] Terminal-Block-Output kann als Agent-Kontext angehängt werden
- [ ] Agent kann Shell-Commands im Terminal ausführen (Full Terminal Use)
- [ ] `/plan` erstellt einen editierbaren Plan
- [ ] Agent-Änderungen erscheinen im Code Review Panel
- [ ] Conversations können geforkt und kompaktiert werden
- [ ] Fehlgeschlagene Commands werden automatisch als Agent-Kontext angeboten
- [ ] "Fix this" auf Error-Block → Agent analysiert und korrigiert
