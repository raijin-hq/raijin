# Phase 25: Terminal-Editor Fusion — Raijin's Dual-Heart Architecture

## Vision

Raijin = Warp (Terminal-first) + Zed (Editor-power). Beide Herzen schlagen gleichberechtigt. Jedes Feature das heute nur mit dem Editor funktioniert muss auch mit dem Terminal funktionieren. Nicht durch Duplizierung, sondern durch **abstrakte Traits und das Provider-Pattern**.

## Das Problem

47 Crates sind hart auf den Editor verdrahtet:
- **~18 Crates** nutzen Editor nur als Text-Input-Widget → kein Umbau nötig
- **~10 Crates** prüfen ob aktives Item ein Editor ist (`downcast::<Editor>()`) → müssen Terminal-aware werden
- **~6 Crates** haben Editor-spezifische Features → bekommen Terminal-Äquivalente als neue Provider
- **~13 Crates** sind rein Editor/Infra-spezifisch → kein Umbau nötig

## Architektur: Shared Traits + Provider Pattern

Statt `downcast::<Editor>()` programmieren Feature-Crates gegen abstrakte Traits. Editor und Terminal implementieren dieselben Traits mit unterschiedlichem Verhalten.

### Neue Traits (in raijin-workspace)

**`Searchable`** (existiert schon, Terminal muss es implementieren)
```rust
// Editor: Suche in Buffer-Text
// Terminal: Suche in Block-Output (ANSI-bereinigt), über Blocks hinweg
```

**`Navigable`** (NEU)
```rust
trait Navigable: Item {
    fn navigate_to(&self, target: NavigationTarget, window: &mut Window, cx: &mut Context<Self>);
    fn current_position(&self, cx: &App) -> Option<NavigationPosition>;
}

enum NavigationTarget {
    Line(u32),              // Editor: Zeilennummer
    Block(usize),           // Terminal: Block-Index
    Symbol(SharedString),   // Editor: Symbol-Name
    Command(SharedString),  // Terminal: Command-String in History
    Offset(usize),          // Beide: Byte-Offset
}

struct NavigationPosition {
    label: SharedString,      // "Line 42" oder "Block #3: git status"
    path: Option<PathBuf>,    // Editor: File Path, Terminal: CWD
    secondary: Option<SharedString>, // Editor: Symbol, Terminal: Command
}
```

**`Outlineable`** (NEU)
```rust
trait Outlineable: Item {
    fn outline_items(&self, cx: &App) -> Vec<OutlineEntry>;
}

struct OutlineEntry {
    label: SharedString,
    detail: Option<SharedString>,
    depth: usize,
    icon: Option<IconName>,
    badge: Option<OutlineBadge>,
    target: NavigationTarget,
}

enum OutlineBadge {
    Success,                // Terminal: Exit Code 0
    Error(i32),             // Terminal: Non-zero Exit Code
    Running,                // Terminal: Laufender Command
    Warning,                // Editor: LSP Warning
    Duration(Duration),     // Terminal: Command Duration
}
```
- **Editor:** Code-Symbole (functions, classes, modules) via LSP
- **Terminal:** Command-Blocks (command + exit code + duration), gruppiert nach Session

**`Diagnosable`** (NEU)
```rust
trait Diagnosable: Item {
    fn diagnostics(&self, cx: &App) -> Vec<ItemDiagnostic>;
}

struct ItemDiagnostic {
    severity: DiagnosticSeverity,
    message: SharedString,
    source: Option<SharedString>,
    target: NavigationTarget,
}

enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}
```
- **Editor:** LSP Diagnostics (errors, warnings, hints)
- **Terminal:** Exit Codes (non-zero = Error), Stderr als Warning, Duration-Anomalien als Info

**`ContextProvider`** (NEU)
```rust
trait ContextProvider: Item {
    fn breadcrumbs(&self, cx: &App) -> Vec<BreadcrumbEntry>;
    fn active_path(&self, cx: &App) -> Option<PathBuf>;
    fn context_for_ai(&self, cx: &App) -> Option<String>;
}

struct BreadcrumbEntry {
    label: SharedString,
    icon: Option<IconName>,
    target: Option<NavigationTarget>,
}
```
- **Editor:** File Path + Current Symbol + Language
- **Terminal:** CWD + Shell Name + Git Branch + Current/Last Command

**`Completable`** (NEU)
```rust
trait Completable: Item {
    fn completion_provider(&self) -> Option<Box<dyn CompletionProvider>>;
}

trait CompletionProvider {
    fn completions(&self, query: &str, cx: &App) -> Vec<CompletionItem>;
}
```
- **Editor:** LSP Completions, Copilot, Edit Prediction
- **Terminal:** Shell Completions (raijin-completions), AI Command Suggestions, History-based Frecency

## Crate-für-Crate Umbau

### Gruppe B: Terminal-aware machen (10 Crates)

**raijin-search**
- Aktuell: `query_editor: Entity<Editor>`, `results_editor: Entity<Editor>`, sucht nur in Editor-Buffers
- Umbau: Search-Provider-System. Editor-Provider (existiert) + Terminal-Provider (NEU)
- Terminal-Search: Suche in Block-Output, filtere Blocks nach Content, Regex in ANSI-bereinigtem Text
- UI: Search-Results zeigen Block-Matches mit Command-Header als Kontext

**raijin-diagnostics**
- Aktuell: `editor: Entity<Editor>`, zeigt LSP Diagnostics
- Umbau: Nutzt `Diagnosable` Trait statt direkt Editor
- Terminal-Diagnostics: Failed Commands (exit code ≠ 0) als Errors, Stderr als Warnings
- UI: Diagnostics-Panel zeigt Editor-Errors UND Terminal-Failures gemischt, sortiert nach Zeit

**raijin-outline / raijin-outline-panel**
- Aktuell: `downcast::<Editor>()`, zeigt Code-Symbole
- Umbau: Nutzt `Outlineable` Trait
- Terminal-Outline: Alle Command-Blocks als Items mit Exit-Status-Badges und Duration
- UI: Outline-Panel wechselt automatisch zwischen Symbole (Editor) und Commands (Terminal)

**raijin-go-to-line**
- Aktuell: `act_as::<Editor>()`, navigiert zu Zeile
- Umbau: Nutzt `Navigable` Trait
- Terminal-Navigation: "Go to Block #N" oder "Go to Command `git push`"
- UI: Input zeigt "Line" oder "Block" je nach aktivem Item

**raijin-breadcrumbs**
- Aktuell: Zeigt File Path vom Editor
- Umbau: Nutzt `ContextProvider` Trait
- Terminal-Breadcrumbs: `~ / Projects / raijin` (CWD) + `zsh` (Shell) + `main` (Git Branch)
- Klickbar: Jedes Segment öffnet den Ordner oder wechselt CWD

**raijin-file-finder**
- Aktuell: `downcast::<Editor>()` für Kontext
- Umbau: Nutzt `ContextProvider::active_path()` für Start-Verzeichnis
- Wenn Terminal aktiv: CWD als Root für Dateisuche
- "Open" Action: Öffnet Datei im Editor ODER sendet `cat`/`vim` ans Terminal (User-Preference)

**raijin-project-panel**
- Aktuell: Hebt aktive Editor-Datei hervor
- Umbau: Nutzt `ContextProvider::active_path()`
- Wenn Terminal aktiv: CWD im Baum hervorheben, Auto-Expand
- Drag & Drop auf Terminal: Generiert `mv`/`cp` Command

**raijin-tab-switcher**
- Aktuell: `act_as::<Editor>()`
- Umbau: Funktioniert schon über `Item` Trait — Terminal-Tabs werden korrekt gezeigt
- Erweiterung: Terminal-Tabs zeigen Shell + CWD statt Filename

**raijin-sidebar**
- Aktuell: `filter_editor: Entity<Editor>`
- Umbau: Filter-Input bleibt Editor-Widget (das ist OK — universelles Textfeld)
- Erweiterung: Sidebar zeigt Terminal-relevante Panels (Command History, Block Navigator)

**raijin-command-palette**
- Aktuell: Nutzt Editor als Suchfeld, zeigt nur Actions
- Umbau: Suchfeld bleibt Editor-Widget
- Erweiterung: Command Palette zeigt AUCH:
  - Letzte Shell Commands (aus raijin-session) mit frecency
  - "Run: `git push`" → führt im Terminal aus
  - "Open: `src/main.rs`" → öffnet im Editor
  - Terminal-spezifische Actions (New Terminal, Split Terminal, Clear Blocks)

### Gruppe C: Terminal-Äquivalente als Provider (6 Crates)

**raijin-vim**
- Editor-Vim bleibt unverändert (43k Zeilen)
- NEU: Terminal-Vim-Adapter — registriert Vim-Keybindings für Terminal Input Bar
  - Normal Mode: hjkl Navigation im Input, w/b Word-Movement
  - Insert Mode: normales Tippen
  - Visual Mode: Text-Selection im Input
  - `:!command` → führt im aktiven Terminal aus
  - `gf` auf Pfad im Terminal-Output → öffnet Datei im Editor

**raijin-copilot**
- Editor-Copilot bleibt unverändert
- NEU: Terminal-Copilot-Provider — Ghost-Text für Shell Commands
  - Sendet Shell-Kontext (CWD, History, Git Status) an Copilot API
  - Zeigt Command-Vorschläge als Ghost-Text in Input Bar
  - Tab zum Akzeptieren

**raijin-edit-prediction-ui**
- Editor-Prediction bleibt unverändert
- NEU: Terminal-Command-Prediction
  - Predicts nächsten Command basierend auf: History-Pattern, CWD, Git Status, Zeit
  - Zeigt als Ghost-Text in Input Bar

**raijin-language-tools**
- Editor-Language-Tools bleiben unverändert
- NEU: Shell-Language-Tools
  - Hover über Command im Terminal-Output → Manpage-Preview
  - Hover über Pfad → File-Info (size, permissions, git status)
  - Hover über Error-Code → Erklärung

**raijin-copilot-chat / raijin-agent-ui**
- Chat/Agent bekommt Terminal-Kontext
  - "Fix this" auf einem fehlgeschlagenen Block → schickt Command + Output + Error an Agent
  - Agent kann Terminal-Commands vorschlagen und ausführen
  - Terminal-Output als Kontext in Conversations

## Innovative Terminal-First Features

### 1. Block-basierte Suche
Nicht nur "Text suchen" sondern strukturierte Block-Suche:
- "Zeige alle fehlgeschlagenen Commands" → filtert nach Exit Code ≠ 0
- "Suche `error` in Output" → Regex über ANSI-bereinigtem Block-Output
- "Commands mit `git`" → filtert Command-Headers
- Suchergebnisse als Block-Highlights mit Kontext

### 2. Command History als First-Class-Feature
Command Palette (Cmd+K) zeigt:
- Actions (wie bisher)
- Recent Commands (frecency-sortiert)
- Suggested Commands (AI-basiert, basierend auf CWD/Git-Status)
- Re-run: Enter auf einem Command → führt im aktiven Terminal aus

### 3. Smart Block Diagnostics
Wenn ein Command fehlschlägt:
- Exit Code wird als Error-Diagnostic registriert
- Stderr wird als Warning-Diagnostic geparst
- Duration-Anomalien als Info ("3x langsamer als üblich")
- "Quick Fix" Action: AI analysiert Error, schlägt korrigierten Command vor
- Diagnostics-Panel zeigt Editor-Errors UND Terminal-Failures chronologisch gemischt

### 4. Terminal-Outline = Command Flow Visualisierung
Outline-Panel zeigt für Terminal:
- Chronologische Liste aller Commands
- Pro Command: Icon (✓/✗/⟳), Command-String, Duration
- Klick → scrollt zum Block
- Filter: nur Errors, nur lang laufende, nur git, etc.
- Nested: Script-Commands werden als Children erkannt

### 5. CWD-aware File Operations
Wenn Terminal aktiv:
- File Finder (Cmd+P) startet im Terminal-CWD
- Project Panel expandiert automatisch Terminal-CWD
- "New File" (Cmd+N) erstellt im Terminal-CWD
- Drag & Drop auf Terminal-Pane → generiert Shell-Command
- Breadcrumbs zeigen CWD-Path, klickbar für Navigation

### 6. Cross-Item Selection & Transfer
User selektiert Text im Terminal-Output:
- "Open in Editor" → erstellt temporären Buffer mit selektiertem Text
- "Search in Project" → öffnet Search mit selektiertem Text als Query
- "Add to Agent Context" → gibt AI-Agent den Terminal-Output als Kontext
- "Copy as Markdown" → formatiert mit Command-Header als Code-Block

### 7. Vim Terminal Fusion
Vim-Mode funktioniert in Terminal Input Bar:
- Normal/Insert/Visual Mode
- `:` → Shell-Command (nicht Vim-Command)
- `gf` auf Pfad im Output → öffnet Datei
- `yy` in Output → kopiert Zeile
- Registers werden zwischen Editor und Terminal geteilt

### 8. AI Terminal Integration
Agent hat vollen Terminal-Zugriff:
- Kann Commands vorschlagen und ausführen (mit User-Bestätigung)
- Liest Terminal-Output als Kontext
- "Fix this" auf Error-Block → analysiert und korrigiert
- Slash Commands: `/run git status`, `/explain` (erklärt letzten Output)
- Terminal-Blocks als Kontext in Agent-Conversations

## Phasen

### Phase 25A: Trait-Definitionen (Fundament)
1. Neue Traits in `raijin-workspace` definieren: `Navigable`, `Outlineable`, `Diagnosable`, `ContextProvider`, `Completable`
2. `raijin-editor` implementiert alle Traits (Verhalten identisch zu bisherigem Code, nur abstrahiert)
3. `raijin-terminal-view` implementiert alle Traits (Terminal-spezifisches Verhalten)
4. Tests: Beide Implementierungen funktionieren über die Trait-Interfaces

### Phase 25B: Feature-Crates Trait-basiert machen (10 Crates)
1. `raijin-search` → Search-Provider-System
2. `raijin-diagnostics` → Diagnosable Trait
3. `raijin-outline` + `raijin-outline-panel` → Outlineable Trait
4. `raijin-go-to-line` → Navigable Trait
5. `raijin-breadcrumbs` → ContextProvider Trait
6. `raijin-file-finder` → ContextProvider für active_path
7. `raijin-project-panel` → ContextProvider für CWD-Highlighting
8. `raijin-tab-switcher` → Terminal-Tab-Labels
9. `raijin-sidebar` → Terminal-Panel-Integration
10. `raijin-command-palette` → Command History + Terminal Actions

### Phase 25C: Terminal Feature-Provider (parallel zu 25B)
1. TerminalSearchProvider — Block-basierte Suche
2. TerminalOutlineProvider — Command History als Outline
3. TerminalDiagnosticsProvider — Exit Codes, Stderr, Duration
4. TerminalContextProvider — CWD, Shell, Git, Current Command
5. TerminalNavigator — Go-to-Block
6. TerminalCompletionProvider — Shell Completions über Trait

### Phase 25D: Terminal-Äquivalente für Editor-Features (6 Crates)
1. Vim Terminal Adapter (Vim-Bindings in Input Bar)
2. Copilot Terminal Provider (Shell Command Ghost Text)
3. Command Prediction (History + AI basiert)
4. Shell Language Tools (Manpage Hover, Path Info)
5. Agent Terminal Integration (Commands ausführen, Output lesen)
6. Cross-Item Selection (Terminal → Editor Transfer)

### Phase 25E: Innovative Terminal-First Features
1. Block-basierte strukturierte Suche
2. Command History in Command Palette
3. Smart Block Diagnostics mit AI Quick Fix
4. Terminal-Outline mit Command Flow
5. CWD-aware File Operations
6. Vim Terminal Fusion (Shared Registers)
7. AI Terminal Slash Commands

## Abhängigkeiten

- Phase 20 (Workspace Integration) muss fertig sein — TerminalPane als Item
- Phase 24 (Component Consolidation) sollte fertig sein — einheitliche UI
- Phase 19 (Settings) muss fertig sein — ThemeSettings::get_global()

## Risiken

1. **Trait-Design** — Falsches Trait-Design führt zu Pain. Lösung: Traits als unstable markieren, iterativ verbessern anhand realer Nutzung
2. **Performance** — Terminal mit 10k+ Blocks: Outline, Diagnostics, Search müssen lazy/virtual sein
3. **AI-Integration** — Copilot/Agent APIs ändern sich. Provider-Pattern erlaubt einfachen Austausch
4. **Scope Creep** — 25E ist groß. Jedes Feature ist eigenständig priorisierbar, kein Big-Bang nötig

## Erfolgs-Kriterien

Phase 25 ist fertig wenn:
- [ ] User kann im Terminal suchen (Cmd+F funktioniert auf Terminal-Blocks)
- [ ] Outline-Panel zeigt Commands wenn Terminal aktiv
- [ ] Diagnostics-Panel zeigt fehlgeschlagene Commands
- [ ] Breadcrumbs zeigen CWD + Shell wenn Terminal aktiv
- [ ] Go-to-Line wird zu "Go to Block" im Terminal
- [ ] File Finder startet im Terminal-CWD
- [ ] Command Palette zeigt recent Shell Commands
- [ ] Vim-Bindings funktionieren in Terminal Input Bar
- [ ] Agent kann Terminal-Output als Kontext nutzen
- [ ] Kein `downcast::<Editor>()` mehr in Feature-Crates (nur Trait-Abfragen)
