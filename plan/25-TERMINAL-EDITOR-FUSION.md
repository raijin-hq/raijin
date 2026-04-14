# Phase 25: Terminal-Editor Fusion — Crate-by-Crate Wiring & Fusion

## Vision

Raijin = Warp (Terminal-first) + Editor-power. Das **Terminal ist das Herz**, der Editor ist die Niere — beide essentiell, aber das Terminal pumpt. Jedes Feature das im Editor funktioniert, muss auch mit dem Terminal funktionieren. Wenn ein Feature Editor-only bleibt, ist das ein **Bug**, kein "fehlt halt noch".

**End-State Definition:** Wenn der User Cmd+F drückt, funktioniert die Suche. Wenn der User Cmd+P drückt, findet er Files. Wenn der User Outline öffnet, sieht er was logisch ist. Wenn der User Diagnostics öffnet, sieht er Probleme. Es ist dem User egal ob er gerade in Editor oder Terminal ist — die Tools verhalten sich passend zum Kontext, nicht zur Komponente.

## Strategie: Crate für Crate, eines nach dem anderen

Dies ist kein Big-Bang. Es ist eine **lineare Liste von Stationen**. Jede Station ist ein Crate (oder eine eng zusammengehörende Crate-Gruppe). Pro Station passiert alles auf einmal:

1. Crate in `raijin-app/Cargo.toml` hinzufügen
2. `init(cx)` in `main.rs` aufrufen (falls nötig)
3. Falls das Crate Editor-coupled war: `Item`-Trait-Methoden auf `TerminalPane` implementieren oder neues Trait einführen
4. Falls das Crate ein neues Trait braucht: Editor- und Terminal-Implementation gleichzeitig schreiben
5. Verifizieren dass das Feature im Editor **und** im Terminal funktioniert
6. Commit, nächste Station

Nicht anders. Keine parallele Trait-Schicht die später verdrahtet wird. Keine "wir machen erst alle Traits, dann alle Implementations". Crate für Crate, Schritt für Schritt, jeder Commit hat einen sichtbaren User-Wert.

**Reihenfolge: nach Aufwand.** Einfache Wins zuerst, schwere Brocken später. Begründung: nach jeder Woche soll es etwas Sichtbares geben. Wenn die Reihenfolge "wertvollstes zuerst" wäre, würden die ersten 4 Wochen nur an `raijin-search` und `raijin-diagnostics` brennen ohne sichtbaren Fortschritt.

## Update-Loop nach jeder Station

Wenn eine Station fertig ist, müssen **drei Dinge parallel** passieren — sonst driftet die Plan-Dokumentation vom realen Code-Stand weg und die nächste Station startet auf falscher Grundlage:

1. **Plan 28 aktualisieren** (`plan/28-FEATURES-AND-EXTENSIONS.md`) — die entsprechende Feature-Zeile in den Audit-Tabellen ändern:
   - **Status-Spalte:** `Code da, nicht verdrahtet` → `funktioniert`
   - **Audit-Spalte:** Verdrahtungs-Detail eintragen (z.B. "raijin-search verdrahtet, SearchableItem auf TerminalPane mit BlockMatch, Cmd+F öffnet Toolbar")
   - **Phase-Spalte:** Stationsnummer → `vorhanden`

   Plan 28 ist die **Audit-Wahrheitsquelle** des Projekts. Wenn Plan 28 sagt "funktioniert", muss es im Code wirklich funktionieren. Wenn Plan 28 sagt "Code da, nicht verdrahtet", darf der nächste Dev sich darauf verlassen.

2. **Plan 25 aktualisieren** (diese Datei) — die fertige Station mit `✓ DONE` in der Überschrift markieren. Beispiel:

       ### Station 5 — raijin-search ✓ DONE

   Damit ist beim Reinschauen sofort klar wo der nächste Dev anfängt.

3. **Vision-Dokumente verschieben** (falls die Station eines hat) — nach `plan/done/`, mit Stub im `plan/` der auf den done-Pfad verweist:
   - **Station 11 (raijin-agent) fertig** → `plan/26-AGENTIC-DEVELOPMENT-ENVIRONMENT.md` nach `plan/done/`
   - **Station 13 (raijin-repl) fertig** → `plan/29-HYBRID-TERMINAL-REPL.md` nach `plan/done/`

   Stub-Format (siehe Plan 27 als Vorlage): kurze Datei im plan/ die nur sagt "STATUS: ERLEDIGT, siehe done/26-AGENTIC-DEVELOPMENT-ENVIRONMENT.md".

**Git-Commit pro Station:** Eine Station = ein Commit (oder eine Commit-Reihe mit klarem Prefix). Commit-Message-Format:

    Station N (crate-name): Kurzbeschreibung was jetzt funktioniert

Beispiel: `Station 5 (raijin-search): Cmd+F öffnet Search-Toolbar im Terminal, Block-Matches highlighted`

Macht später Bisect bei Bugs trivial — wenn "Search ist seit gestern kaputt" gemeldet wird, weißt du sofort welcher Station-Commit es war.

**Was passiert wenn du den Update-Loop vergisst:** Plan 28 und Code drift apart. In drei Stationen weiß keiner mehr ob `raijin-search` jetzt verdrahtet ist oder nicht, weil Plan 28 noch "neu bauen" sagt aber Code es schon kann. Dann musst du wieder die ganze Code-Inventur machen die du heute schon einmal gemacht hast. Nicht nochmal.

## Code-Inventur (verifiziert April 2026)

### Was `raijin-app` heute tatsächlich lädt

`crates/raijin-app/Cargo.toml` enthält folgende Crates:

```
inazuma, inazuma-clock, inazuma-util, inazuma-settings-framework,
raijin-actions, raijin-assets, raijin-call, raijin-client,
raijin-command-palette, raijin-completions, raijin-credentials-provider,
raijin-db, raijin-fs, raijin-http-client, raijin-paths, raijin-language,
raijin-node-runtime, raijin-platform-title-bar, raijin-project,
raijin-project-registry, raijin-release-channel, raijin-session,
raijin-settings, raijin-settings-ui, raijin-shell (Window-Shell/AppShell),
raijin-shell-integration (ehemals raijin-shell — ShellContext/Hooks),
raijin-tab-switcher, raijin-term, raijin-terminal, raijin-terminal-view,
raijin-ui, raijin-theme, raijin-theme-settings, raijin-title-bar,
raijin-workspace
```

Das ist alles. Insbesondere **nicht** geladen: `raijin-editor`, `raijin-search`, `raijin-diagnostics`, `raijin-outline`, `raijin-outline-panel`, `raijin-file-finder`, `raijin-project-panel`, `raijin-go-to-line`, `raijin-vim`, `raijin-copilot`, `raijin-edit-prediction`, `raijin-agent`, `raijin-agent-ui`, `raijin-language-model`, `raijin-language-models`, alle 13+ AI-Provider-Crates, `raijin-repl`, `raijin-image-viewer`, `raijin-markdown-preview`, `raijin-language-tools`, `raijin-breadcrumbs`, `raijin-multi-buffer`, `raijin-streaming-diff`.

**Konsequenz:** Die meisten Stationen haben "Crate in Cargo.toml hinzufügen" als ersten Schritt. Das ist die eigentliche Hauptarbeit, nicht das Trait-Design.

### Was im `Item`-Trait schon abstract ist (raijin-workspace/src/item.rs verifiziert)

Folgende Methoden existieren bereits und brauchen nur Implementation auf `TerminalPane`:

- `breadcrumbs() -> Option<(Vec<HighlightedText>, Option<Font>)>`
- `breadcrumb_prefix() -> Option<AnyElement>`
- `breadcrumb_location() -> ToolbarItemLocation`
- `tab_icon() -> Option<Icon>`
- `tab_content() -> AnyElement`
- `tab_tooltip_text() -> Option<SharedString>`
- `for_each_project_item(f)` (liefert ProjectPath via ProjectItem-Trait)
- `as_searchable() -> Option<Box<dyn SearchableItemHandle>>`
- `navigate(Arc<dyn Any + Send>) -> bool`
- `act_as_type(type_id) -> Option<AnyEntity>`

`Item::navigate()` ist generisch (`Arc<dyn Any + Send>`) — Editor packt seine `NavigationData` rein, Terminal kann seine `BlockNavigationData` reinpacken. **Kein neues `Navigable`-Trait nötig.**

`SearchableItem` (raijin-workspace/src/searchable.rs) hat `type Match: Any` — das `Match` ist generic, nicht Editor-spezifisch. Wenn `TerminalPane::as_searchable()` ein `SearchableItem` mit `Match = BlockMatch` zurückgibt, funktioniert die Search-Bar automatisch.

### Was `TerminalPane` heute schon implementiert (terminal_pane.rs verifiziert)

`impl Item for TerminalPane` hat aktuell:
- `tab_content_text` ✓
- `tab_tooltip_text` ✓
- `to_item_events` ✓
- `can_split` + `clone_on_split` ✓
- `is_dirty` ✓
- `can_save` ✓ (false)
- `added_to_workspace` ✓

Was **fehlt** und in den Stationen 1-2 hinzukommt:
- `breadcrumbs`, `breadcrumb_prefix`, `breadcrumb_location`
- `tab_icon`
- `for_each_project_item`
- `as_searchable`
- `navigate`

`TerminalPane` hat schon `current_git_root: Option<PathBuf>` und registriert das mit `ProjectRegistry` — das Plumbing fürs Workspace-Project-System existiert bereits.

## Pre-Work: Plan 35 (Dead Code Cleanup)

Vor Station 1 läuft **Plan 35** — Removal von `MultiWorkspace`, `WorkspaceStore`, MultiWorkspace-Downcasts, plus Crate-Restrukturierung (AppShell-Extraktion in neue Crate `raijin-shell`, Umbenennung `raijin-shell` → `raijin-shell-integration`). Begründung:

- Beim Verdrahten neuer Crates kämpfen wir sonst gegen `window.downcast::<MultiWorkspace>()` Pfade die bei uns immer fehlschlagen
- Plan 35 ist mechanisch und in 5 Subphasen gut strukturiert
- Jede Subphase kompiliert einzeln — kein Risiko die Codebase zu brechen
- Geschätzt 1-2 Wochen, danach ist die Codebase sauber für die Verdrahtung

**Plan 35 ist Vorbedingung für Phase 25. Nicht parallel.** Erst aufräumen, dann erweitern.

## Pre-Work nach Plan 35: Basis-Methoden auf TerminalPane

Bevor Station 1 anfängt, kriegt `TerminalPane` die fünf Basis-Item-Methoden die mehrere Stationen gleichzeitig nutzen. Das ist keine eigene Station, das ist Setup für Stationen 1-3.

```rust
impl Item for TerminalPane {
    // ... bestehende Methoden ...

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<Icon> {
        // Shell-spezifisches Icon: zsh → IconName::Terminal,
        // bash → IconName::Terminal, fish → IconName::Fish, nu → IconName::Nu
        // Falls IconName-Varianten fehlen: erstmal IconName::Terminal für alle
    }

    fn breadcrumbs(&self, _cx: &App) -> Option<(Vec<HighlightedText>, Option<Font>)> {
        // CWD-Segmente (klickbar) + Shell + optional Git Branch
        // Beispiel: ~ / Projects / raijin · zsh · main
    }

    fn breadcrumb_prefix(&self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<AnyElement> {
        // Shell-Icon als prefix
    }

    fn breadcrumb_location(&self, _cx: &App) -> ToolbarItemLocation {
        ToolbarItemLocation::PrimaryLeft
    }

    fn for_each_project_item(
        &self,
        _cx: &App,
        f: &mut dyn FnMut(EntityId, &dyn raijin_project::ProjectItem),
    ) {
        // Synthetisches ProjectItem mit current_git_root.unwrap_or_else(|| cwd) als ProjectPath.
        // Damit liefert ItemHandle::project_path() korrekt das Terminal-CWD.
    }

    fn navigate(
        &mut self,
        data: Arc<dyn Any + Send>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        // Empfängt BlockNavigationTarget, scrollt zum Block via block_list
        if let Some(target) = data.downcast_ref::<BlockNavigationTarget>() {
            self.block_list.update(cx, |list, cx| {
                list.scroll_to_block(target.block_id, cx)
            });
            return true;
        }
        false
    }
}
```

`BlockNavigationTarget` ist ein neuer Typ in `raijin-terminal-view`:

```rust
pub struct BlockNavigationTarget {
    pub block_id: BlockId,  // BlockId aus raijin_term::block_grid
}
```

**Nach Pre-Work funktionieren automatisch:** Tab-Switcher zeigt Shell-Icon, Title-Bar zeigt Tab-Title korrekt, Workspace-History trackt Terminal-CWDs. Das ist Station 1 und 2 gleichzeitig.

---

## Stationen-Liste

### Station 1: raijin-tab-switcher — Tab-Icon (Stunden)

**Status:** raijin-tab-switcher ist bereits in raijin-app/Cargo.toml geladen.

**Was tun:** Nichts in Cargo.toml ändern. Das Pre-Work `tab_icon()` reicht.

**Verifikation:**
- Terminal-Tab in der Tab-Bar zeigt Shell-Icon (zsh/bash/fish/nu)
- Tab-Switcher-Modal (`Cmd+B` oder ähnlich) zeigt das Icon neben dem Tab-Titel

**Risiken:** Keine. raijin-tab-switcher importiert `entry_diagnostic_aware_icon_decoration_and_color` und `entry_git_aware_label_color` aus `raijin_editor::items` — das sind reine Helper-Funktionen ohne Editor-State, ziehen aber `raijin-editor` als transitive Dep. Falls das Cargo-Compile-Errors auslöst (raijin-editor nicht in der App), müssen die Helper in ein neues `raijin-item-icons` Util-Crate ausgelagert werden. **Vor Station 1 prüfen: kompiliert raijin-tab-switcher heute überhaupt ohne raijin-editor in der App?**

---

### Station 2: raijin-breadcrumbs — Terminal-Breadcrumbs (Stunden bis Tag)

**Status:** raijin-breadcrumbs ist **nicht** in raijin-app/Cargo.toml. Crate existiert aber.

**Cargo.toml:**
```toml
raijin-breadcrumbs = { workspace = true }
```

**main.rs:**
```rust
raijin_breadcrumbs::init(cx);
// Breadcrumbs als Toolbar-Item registrieren — siehe wie raijin-tab-switcher in main.rs registriert wird
```

**Item-Methoden:** Pre-Work `breadcrumbs()`, `breadcrumb_prefix()`, `breadcrumb_location()` reichen. raijin-breadcrumbs operiert auf `Box<dyn ItemHandle>` und ruft `active_item.breadcrumbs(cx)` — ist schon abstract.

**Verifikation:**
- Terminal aktiv → Breadcrumbs zeigen `~ / Projects / raijin · zsh · main`
- CWD-Wechsel im Terminal → Breadcrumbs aktualisieren sich
- Klick auf CWD-Segment → öffnet Verzeichnis (oder noop in Phase 25.2)

**Risiken:** raijin-breadcrumbs könnte eigene Crate-Deps mitbringen die wiederum nicht in der App sind. Vor Station 2 die Cargo.toml von raijin-breadcrumbs anschauen. Wenn raijin-editor als Dep auftaucht: ab dieser Station ist raijin-editor in der App. Damit musst du leben — oder den Editor-Import in raijin-breadcrumbs eliminieren.

---

### Station 3: raijin-file-finder — Cmd+P im Terminal-CWD (Tage)

**Status:** Nicht in der App.

**Cargo.toml:**
```toml
raijin-file-finder = { workspace = true }
```

**main.rs:**
```rust
raijin_file_finder::init(cx);
```

**Item-Methoden:** Pre-Work `for_each_project_item()` reicht. raijin-file-finder holt `currently_opened_path` via `workspace.active_item(cx).project_path(cx)` — funktioniert mit jedem Item das `project_path` liefert.

**Was du ignorieren musst:** Im `confirm()` Handler von raijin-file-finder steht `item.downcast::<Editor>()` für Row/Col-Jump nach dem Öffnen. Das ist ein Editor-spezifischer Pfad — für Terminal wird er einfach übersprungen (downcast schlägt fehl, Jump entfällt). Harmlos.

**Verifikation:**
- Terminal aktiv → Cmd+P startet im Terminal-CWD
- File-Finder zeigt Files relativ zum CWD
- Datei öffnen → öffnet im Editor-Pane (sobald Editor-Pane via Station 4+ existiert) ODER fehlerhaft falls noch kein Editor da

**Risiken:**
1. raijin-file-finder zieht `raijin-editor` als Dep → ab dieser Station ist raijin-editor in der App
2. raijin-file-finder zieht möglicherweise weitere Crates die wiederum eigene Deps haben — Cascade-Effekt. Vor Station 3 die volle Dep-Kette von raijin-file-finder mit `cargo tree -p raijin-file-finder` durchgehen
3. Ohne Editor-Pane (Station 4+ noch nicht da) hat "Datei öffnen" keinen Zielort. **Workaround:** Datei in einer simplen Read-Only-Vorschau anzeigen, oder Cmd+P erstmal nur File-Liste rendern ohne Open-Action

---

### Station 4: raijin-project-panel — Tree mit CWD-Highlight (Tage)

**Status:** Nicht in der App.

**Cargo.toml:**
```toml
raijin-project-panel = { workspace = true }
```

**main.rs:**
```rust
raijin_project_panel::init(cx);
// Panel registrieren analog zu wie andere Panels in der Workspace-Sidebar leben
```

**Item-Methoden:** Pre-Work `for_each_project_item()` reicht für CWD-Highlighting.

**Verifikation:**
- Project-Panel ist als Sidebar-Panel sichtbar
- Terminal aktiv → Project-Panel highlightet das Verzeichnis das dem Terminal-CWD entspricht
- CWD-Wechsel im Terminal → Highlight wandert mit
- File-Klick im Project-Panel → öffnet Datei (im Editor wenn vorhanden, sonst Fehler)

**Risiken:**
1. raijin-project-panel hat `filename_editor: Entity<Editor>` — single-line Input-Widget, OK aber zieht raijin-editor als Dep. Ab Station 4 ist raijin-editor definitiv in der App
2. Mehrere `active_item.downcast::<Editor>()` Stellen für Editor-spezifische Aktionen — die bleiben Editor-only, harmlos
3. Project-Panel braucht ein "Worktree"-Konzept aus raijin-project — das ist schon in der App, sollte funktionieren

---

### Station 5: raijin-search — Cmd+F im Terminal (1 Woche)

**Status:** Nicht in der App. Erstes Crate in der Liste das echte neue Logik braucht.

**Cargo.toml:**
```toml
raijin-search = { workspace = true }
```

**main.rs:**
```rust
raijin_search::init(cx);
```

**Item-Methoden:** `as_searchable()` muss neu implementiert werden:

```rust
impl Item for TerminalPane {
    fn as_searchable(
        &self,
        handle: &Entity<Self>,
        _cx: &App,
    ) -> Option<Box<dyn SearchableItemHandle>> {
        Some(Box::new(handle.clone()))
    }
}

impl SearchableItem for TerminalPane {
    type Match = BlockMatch;
    // find_matches, update_matches, activate_match, select_matches, query_suggestion
}
```

**Neues Modul:** `crates/raijin-terminal-view/src/block_search.rs`:

```rust
pub struct BlockMatch {
    pub block_id: BlockId,
    pub byte_range: Range<usize>,
    pub line: u32,
}

pub fn strip_ansi(text: &str) -> String { /* ANSI-CSI/OSC-Stripping */ }

pub fn find_matches_in_block(
    block: &TerminalBlock,
    query: &SearchQuery,
) -> Vec<BlockMatch> { /* Iteration über strip_ansi(block.output) */ }
```

**raijin-search anpassen (minimal):** Im `BufferSearchBar` werden Splittable-Diff-Buttons aktuell unconditional gerendert — die müssen conditional werden:

```rust
let has_splittable_editor = self.active_searchable_item
    .as_ref()
    .and_then(|item| item.act_as::<Editor>(cx))
    .is_some();
```

Wenn `false`, Splittable-Diff-Buttons weglassen. Sonst nichts an raijin-search ändern.

**Verifikation:**
- Cmd+F im Terminal öffnet Search-Bar
- Tippen filtert/highlightet Matches in Block-Output
- Enter/Shift+Enter springt zwischen Matches
- ANSI-Codes werden korrekt gestrippt vor Match-Suche
- Performance-Test: Terminal mit 10k Blocks à 100 Zeilen, Search bleibt responsive (<200ms first match)

**Risiken:**
1. raijin-search zieht `raijin-multi-buffer` und `raijin-editor` als Deps (Cascade von Station 3-4 schon da)
2. Performance bei großen Block-Outputs — falls Suche linear über 100 MB Text geht, ist das langsam. Lösung: erst über Block-Headers (commands), dann optional in expandiertem Output
3. Match-Highlighting im `block_list` braucht neue API in `BlockListView` (`set_search_highlights(matches)`)

---

### Station 6: raijin-go-to-line — Go to Block im Terminal (Tage)

**Status:** Nicht in der App.

**Cargo.toml:**
```toml
raijin-go-to-line = { workspace = true }
```

**main.rs:**
```rust
raijin_go_to_line::init(cx);
```

**Item-Methoden:** Pre-Work `navigate()` reicht für die Mechanik. Die UI-Modal-Logik muss umgebaut werden.

**raijin-go-to-line umbauen:**
- `GoToLine` Modal nimmt `Box<dyn ItemHandle>` statt `Entity<Editor>`
- Beim Öffnen: `item.act_as::<Editor>(cx)` für Editor-Modus, sonst `item.act_as::<TerminalPane>(cx)` für Block-Modus
- UI-Label und Validation unterschiedlich: "Line N" vs "Block N"
- Bei Confirm: 
  - Editor: `item.navigate(Arc::new(EditorNavigationData { row, col }))`
  - Terminal: `item.navigate(Arc::new(BlockNavigationTarget { block_id }))`

**Verifikation:**
- Cmd+G im Editor → "Go to Line" Modal wie bisher
- Cmd+G im Terminal → "Go to Block" Modal mit Block-Index-Input
- Block-Index gültig → scrollt im Terminal zum Block

**Risiken:**
1. raijin-go-to-line registriert sich heute via `editor.register_action` — diese Registrierung muss zu workspace-level wechseln damit es auch im Terminal triggert
2. `act_as::<TerminalPane>` braucht eine `act_as_type` Implementation auf `TerminalPane` (der default macht das bereits richtig wenn `Self == TerminalPane`)

---

### Station 7: raijin-outline + raijin-outline-panel — Outline mit Command-Blocks (1-2 Wochen)

**Status:** Nicht in der App. Erstes Crate-Pair das ein neues Trait braucht.

**Cargo.toml:**
```toml
raijin-outline = { workspace = true }
raijin-outline-panel = { workspace = true }
```

**main.rs:**
```rust
raijin_outline::init(cx);
raijin_outline_panel::init(cx);
```

**Neues Trait:** `crates/raijin-workspace/src/outlineable.rs`

```rust
pub trait Outlineable: Item {
    fn outline_snapshot(&self, cx: &App) -> OutlineSnapshot;

    fn subscribe_to_outline_events(
        &self,
        window: &mut Window,
        cx: &mut App,
        handler: Box<dyn Fn(OutlineEvent, &mut Window, &mut App) + Send>,
    ) -> Subscription;
}

pub struct OutlineSnapshot {
    pub entries: Vec<OutlineEntry>,
    pub item_kind: OutlineItemKind,
}

pub enum OutlineItemKind {
    CodeSymbols,    // Editor: functions, classes, modules
    CommandBlocks,  // Terminal: command history with status
}

pub struct OutlineEntry {
    pub stable_id: u64,
    pub label: SharedString,
    pub detail: Option<SharedString>,
    pub depth: usize,
    pub icon: Option<IconName>,
    pub badge: Option<OutlineBadge>,
    pub navigation_data: Arc<dyn Any + Send + Sync>,
}

pub enum OutlineBadge {
    Success,
    Error(i32),
    Running,
    Warning,
    Duration(Duration),
}

pub enum OutlineEvent {
    Invalidated,
}
```

`Item`-Trait erweitern um `as_outlineable() -> Option<Box<dyn OutlineableHandle>>`, default `None`.

**Editor migrieren:** `impl Outlineable for Editor` — die Logik aus `outline_for_editor()` in raijin-outline wandert in die Trait-Implementation. **Kritisch:** alle existierenden Editor-Outline-Tests müssen weiter grün sein.

**raijin-outline umstellen:** Statt `editor.buffer_outline_items()` nutze `outlineable.outline_snapshot(cx)`. `OutlineView` operiert auf `Box<dyn OutlineableHandle>` statt `Entity<Editor>`.

**raijin-outline-panel umstellen:** Größerer Brocken weil das Crate viele Editor-Internals (`MultiBufferSnapshot`, `ExcerptId`, `display_map::ToDisplayPoint`) direkt nutzt. Diese Stellen entweder hinter conditional `act_as::<Editor>` Pfade oder über das Trait gehen.

**`impl Outlineable for TerminalPane`:**
- `outline_snapshot()` baut `Vec<OutlineEntry>` aus `terminal.block_router().blocks()`. Stable ID = BlockId aus `raijin_term::block_grid`. Badge = Exit Code / Running / Duration
- `subscribe_to_outline_events()` hört auf `TerminalPaneEvent` und emittiert `Invalidated` bei `CommandStart`/`CommandEnd`

**Performance-Constraints (Pflicht, nicht optional):**
- Snapshot wird gecacht, invalidated nur bei Block-Add/-Remove (nicht bei jedem Render)
- Block-IDs als stable IDs (NICHT Array-Indizes — Blocks können gelöscht werden)
- Outline-Panel nutzt `uniform_list` (existiert schon im Codebase)

**Verifikation:**
- Editor-Outline funktioniert exakt wie vorher (alle Tests grün)
- Terminal aktiv → Outline-Panel zeigt Command-Liste mit Status-Badges (✓/✗/⟳)
- Klick auf Command → scrollt im Terminal zum Block via `Item::navigate`
- Performance-Test: 10k Blocks, Panel scrollt ohne Stutter
- raijin-outline und raijin-outline-panel enthalten kein direktes `Entity<Editor>` mehr (außer hinter `act_as`-Pfaden für editor-spezifische Sub-Features)

**Risiken:**
1. raijin-outline-panel hat sehr viel Editor-Internal-Code. Migration ist nicht mechanisch, sondern Teil-Rewrite
2. Snapshot-Invalidierung bei Editor-Outlines kommt aus LSP-Events — die Subscription muss umgebogen werden ohne dass Editor-Outline-Tests brechen
3. Editor-Tests dürfen nicht brechen. Vor Station 7 alle Editor-Outline-Tests identifizieren und als Regression-Suite einfrieren

---

### Station 8: raijin-diagnostics → raijin-issues-panel (2-3 Wochen, größter Brocken)

**Status:** Nicht in der App.

**Vorab:** Das aktuelle `ProjectDiagnosticsEditor` IST ein Editor — `editor: Entity<Editor>`, `MultiBuffer`, `CustomBlockId`, `BlockProperties`, `BlockStyle`. Es ist nicht "Trait-aware machen", es ist "neu denken".

**Entscheidung:** Option A — neues Issues-Panel, ProjectDiagnosticsEditor bleibt unverändert.

Begründung: Der Editor-Diagnostic-View ist ein gewachsenes Feature mit eigenem Wert (Multi-Buffer mit anchored Custom Blocks, das ist nicht trivial). Nicht anfassen.

**Cargo.toml:**
```toml
raijin-diagnostics = { workspace = true }    # Bleibt für LSP-Diagnostics-Editor-View
# Neues Crate:
raijin-issues-panel = { workspace = true }    # Aggregator
```

**main.rs:**
```rust
raijin_diagnostics::init(cx);
raijin_issues_panel::init(cx);
```

**Neues Trait:** `crates/raijin-workspace/src/diagnosable.rs`

```rust
pub trait Diagnosable: Item {
    fn diagnostics_snapshot(&self, cx: &App) -> DiagnosticsSnapshot;

    fn subscribe_to_diagnostics_events(
        &self,
        window: &mut Window,
        cx: &mut App,
        handler: Box<dyn Fn(DiagnosticsEvent, &mut Window, &mut App) + Send>,
    ) -> Subscription;
}

pub struct DiagnosticsSnapshot {
    pub items: Vec<ItemDiagnostic>,
    pub source_kind: DiagnosticsSourceKind,
}

pub enum DiagnosticsSourceKind {
    LanguageServer,    // Editor
    CommandFailures,   // Terminal
}

pub struct ItemDiagnostic {
    pub stable_id: u64,
    pub severity: DiagnosticSeverity,
    pub message: SharedString,
    pub source: Option<SharedString>,
    pub navigation_data: Arc<dyn Any + Send + Sync>,
}

pub enum DiagnosticSeverity {
    Error, Warning, Info, Hint,
}

pub enum DiagnosticsEvent {
    Invalidated,
}
```

**Editor:** `impl Diagnosable for Editor` — ruft existierende LSP-Diagnostic-API auf, wrappt in `ItemDiagnostic`s.

**Terminal:** `impl Diagnosable for TerminalPane` — sammelt failed commands aus `block_router().blocks()`, exit code ≠ 0 → Error, Stderr-Zeilen → Warning.

**Neues Crate raijin-issues-panel:**
- Listet alle `Diagnosable` Items im Workspace (workspace.items.iter().filter_map(|i| i.as_diagnosable()))
- Aggregiert Snapshots, sortiert nach Severity dann Zeit
- Rendert via `uniform_list`
- Klick → `item.navigate(navigation_data)` — bei Editor öffnet das volle ProjectDiagnosticsEditor, bei Terminal scrollt zum Block

**Verifikation:**
- Editor-Diagnostics funktionieren unverändert (alle Tests grün)
- Issues-Panel zeigt Editor-Errors UND Terminal-Failures gemischt
- Terminal-Failure-Klick scrollt Terminal zum Block
- Editor-Error-Klick öffnet das volle Diagnostic-View
- Performance-Test: 1000 Diagnostics gemischt, Panel scrollt smooth

**Risiken:**
1. **Größter Risiko-Punkt der gesamten Phase 25.** raijin-diagnostics zu verdrahten ohne dass es die App bricht ist nicht trivial — das Crate hat tiefe Editor-Hooks
2. raijin-issues-panel ist ein **neues Crate** das geschrieben werden muss — kein Copy-Paste von raijin-diagnostics
3. Performance bei 1000+ Diagnostics aus 10k Blocks gleichzeitig

---

### Station 9: raijin-vim — Vim-Mode auch im Terminal-Input (2-4 Wochen)

**Status:** Nicht in der App. 43k LOC Editor-Adapter.

**Cargo.toml:**
```toml
raijin-vim = { workspace = true }
raijin-vim-mode-setting = { workspace = true }
```

**main.rs:**
```rust
raijin_vim::init(cx);
```

**Strategie:** raijin-vim **nicht** umstellen. Stattdessen ein **paralleler Adapter** für die Terminal-Input-Bar.

**Neues Sub-Modul:** `crates/raijin-vim/src/terminal_adapter.rs`
- Vim-State pro `InputState` (Normal/Insert/Visual)
- Keybinding-Routing über das raijin-vim Action-System wiederverwenden
- Ex-Commands: `:!cmd` schickt an Terminal, `:q` schließt Pane, `gf` auf Pfad im Output öffnet Datei

**Shared Registers (Workspace-globaler Store):**
- Yank in Editor → Paste in Terminal Input Bar
- Yank in Terminal-Output → Paste im Editor
- Konflikt-Resolution: last-write-wins (zwei Terminals yanken gleichzeitig → der zuletzt aktive Pane gewinnt)
- Muss explizit dokumentiert sein

**Verifikation:**
- `vim_mode = true` in Settings → Terminal Input Bar hat Normal/Insert/Visual Modes
- hjkl, w, b funktionieren im Input
- `:!cmd` schickt Command an Terminal
- Yank/Paste zwischen Editor und Terminal funktioniert
- Editor-Vim funktioniert exakt wie vorher (alle Tests grün)

**Risiken:**
1. raijin-vim ist 43k LOC — die Registry und das Action-System zu verstehen kostet Zeit
2. Shared Registers sind ein Sync-Problem das bei Konflikten leise daneben gehen kann
3. `vim_mode_setting` muss als eigene Cargo-Dep und Setting integriert werden

---

### Station 10: raijin-language-model + alle AI-Provider (1-2 Wochen)

**Status:** Nicht in der App. 15+ Crates, alle voneinander abhängig.

**Cargo.toml:**
```toml
raijin-language-model = { workspace = true }
raijin-language-models = { workspace = true }
raijin-anthropic = { workspace = true }
raijin-open-ai = { workspace = true }
raijin-ollama = { workspace = true }
raijin-mistral = { workspace = true }
raijin-deepseek = { workspace = true }
raijin-google-ai = { workspace = true }
raijin-bedrock = { workspace = true }
raijin-vercel = { workspace = true }
raijin-x-ai = { workspace = true }
raijin-open-router = { workspace = true }
raijin-codestral = { workspace = true }
raijin-lmstudio = { workspace = true }
raijin-copilot-chat = { workspace = true }
raijin-cloud-llm-client = { workspace = true }
raijin-cloud-api-types = { workspace = true }
raijin-cloud-api-client = { workspace = true }
```

**main.rs:**
```rust
raijin_language_model::init(cx);
raijin_language_models::init(cx);
// Provider-Crates registrieren sich vermutlich selbst über raijin_language_models::init
```

**Settings:** BYOK in raijin-settings-content erweitern für API-Keys pro Provider. raijin-settings-ui zeigt Provider-Liste mit Eingabefeldern.

**Verifikation:**
- Settings-UI zeigt alle Provider als Liste
- API-Keys können pro Provider gesetzt werden
- Test-Call gegen einen Provider (z.B. Anthropic) funktioniert
- Modelle erscheinen im Model-Selector

**Risiken:**
1. Die 15 Provider-Crates haben jeweils eigene HTTP-Client-Logik und können verschiedene Versions-Probleme haben
2. raijin-language-models hat möglicherweise Editor-Hooks (für Inline-Completions) — die müssen abstrahiert oder gekippt werden
3. BYOK-Settings müssen verschlüsselt im Credentials-Store landen, nicht im plain-text TOML — raijin-credentials-provider ist schon in der App

---

### Station 11: raijin-agent + raijin-agent-ui + agent-settings + agent-servers (4-6 Wochen)

**Das ist Plan 26 in einer Station.**

**Status:** Nicht in der App. 4 Crates, ~165k LOC kombiniert.

**Cargo.toml:**
```toml
raijin-agent = { workspace = true }
raijin-agent-ui = { workspace = true }
raijin-agent-settings = { workspace = true }
raijin-agent-servers = { workspace = true }
raijin-acp-thread = { workspace = true }
raijin-acp-tools = { workspace = true }
raijin-prompt-store = { workspace = true }
raijin-rules-library = { workspace = true }
raijin-streaming-diff = { workspace = true }
raijin-assistant-slash-command = { workspace = true }
raijin-assistant-slash-commands = { workspace = true }
raijin-assistant-text-thread = { workspace = true }
raijin-context-server = { workspace = true }
```

**main.rs:** `init()` für jedes Crate, Reihenfolge wichtig (acp vor agent, prompt-store vor agent, etc).

**Item-Methoden:** TerminalPane bekommt `context_for_ai() -> Option<String>` als neue Bonus-Methode am Item-Trait. Liefert relevant blocks als Context für Agent-Conversations.

**Neue Tools:**
- `TerminalExecutionTool` — Agent kann Commands im aktiven Terminal ausführen, mit User-Confirmation-Dialog (kein silent execution)
- `BlockContextTool` — Agent kann Block-Outputs als Context ziehen

**Settings:**
- Agent-Permissions in raijin-agent-settings (Auto-execute safe commands, Ask for approval, Blocklist)

**Slash Commands:** `/agent`, `/plan`, `/fork`, `/compact`, `/explain`, `/fix`

**Verifikation:**
- Cmd+Enter im Terminal öffnet Agent Conversation View
- Agent kann Terminal-Output lesen
- Agent kann Commands ausführen (mit User-Confirmation)
- `/fix` auf failed Block analysiert Error und schlägt Korrektur vor
- Plan Mode mit `/plan` erstellt editierbaren Plan
- Conversations können geforkt und kompaktiert werden

**Risiken:**
1. **Größte Station überhaupt.** Plan 26 schätzt 4-6 Wochen, das ist optimistisch
2. raijin-agent-ui ist 61k LOC und wird teilweise umgebaut werden müssen (von Editor-Panel zu Terminal-integriertem Panel)
3. `raijin-streaming-diff` ist nur 1k LOC (kein 36k wie Plan 28 fälschlich behauptet) — die Code Review Feature muss ggf. erweitert werden
4. Cloud Agents (Phase 26G im alten Plan) sind **nicht in dieser Station**. Wenn überhaupt, dann sehr viel später als eigene Station

---

### Station 12: raijin-copilot + raijin-edit-prediction + raijin-edit-prediction-ui (2-3 Wochen)

**Status:** Nicht in der App.

**Cargo.toml:**
```toml
raijin-copilot = { workspace = true }
raijin-copilot-ui = { workspace = true }
raijin-edit-prediction = { workspace = true }
raijin-edit-prediction-ui = { workspace = true }
raijin-edit-prediction-context = { workspace = true }
raijin-edit-prediction-types = { workspace = true }
```

**main.rs:** init() für jedes Crate.

**Terminal-Provider:**
- `TerminalCopilotProvider` — sammelt Shell-Context (CWD, recent commands, git status), schickt an Copilot-API, returnt Command-Vorschlag als Ghost-Text in `InputState`
- `TerminalCommandPredictor` — lokal, History-Pattern-basiert, kein API-Call

**Verifikation:**
- Tippen in Terminal-Input zeigt Ghost-Text-Vorschlag
- Tab akzeptiert Vorschlag
- Esc verwirft
- Funktioniert mit Copilot UND mit lokalem History-Predictor (User wählt in Settings)

**Risiken:**
1. Copilot-API-Auth ist ein eigenes Thema — raijin-copilot hat dafür schon Logik aber muss verdrahtet werden
2. Edit-Prediction ist heute Editor-only — Provider-System muss erweitert werden um Terminal-Input-Provider aufzunehmen

---

### Station 13: raijin-repl — Hybrid Terminal-Notebook (3-4 Wochen)

**Das ist Plan 29 in einer Station.**

**Status:** Nicht in der App. Kompiliert eigenständig laut Plan 29, aber `outputs/plain.rs` ist gebrochen.

**Pre-Work (vor Cargo-Verdrahtung):**
- `outputs/plain.rs` reparieren: `raijin_terminal_view::terminal_element::TerminalElement` Import entfernen, `to_grid_snapshot()` Methode auf `TerminalOutput` schreiben, durch `TerminalGridElement` rendern
- `raijin-repl/Cargo.toml` reviewen: raijin-terminal-view Dep entfernen oder fixen

**Cargo.toml:**
```toml
raijin-repl = { workspace = true }
```

**main.rs:**
```rust
raijin_repl::init(fs.clone(), cx);
```

**Neue Module:**
- `raijin-terminal/src/repl_detection.rs` — Foreground Process Inspection, Known REPL Registry (python, node, irb, ghci, julia, R, etc.)
- `raijin-terminal/src/repl_session.rs` — Sidecar Kernel Lifecycle, ReplOutput, Dual-Route Input
- `raijin-terminal-view/src/repl_block.rs` — ReplBlockView Rendering
- `raijin-terminal-view/src/repl_output.rs` — Rich Output GPU Renderer (Image, Table, Markdown, JSON via Inazuma)
- `raijin-completions/src/kernel_completion.rs` — Jupyter complete_request/inspect_request Provider

**Item-Methoden:** TerminalPane bekommt `repl_session() -> Option<&ReplSession>` Getter.

**Verifikation:**
- `python` im Terminal eingeben → REPL detected via Foreground Process
- ReplStore findet python3 Kernel via `jupyter kernelspec list`
- Sidecar Kernel startet via NativeRunningKernel
- `np.random.rand(3,3)` zeigt strukturierte Tabelle inline im Block
- `plt.plot(...)` zeigt PNG inline im Block
- `exit()` → REPL beendet, zurück zu Shell
- Input Bar wechselt Completions zwischen Shell und Kernel automatisch

**Risiken:**
1. raijin-repl ist 16k LOC eigenständige Logik plus mehrere neue Module — viel Glue-Code
2. Foreground Process Detection ist plattformspezifisch (sysctl macOS, /proc Linux)
3. Jupyter-Kernel-Discovery ist fragil bei verschiedenen Python-Installationen (venv, conda, system)
4. Rich Output Rendering braucht GPU-Image-API die in Inazuma existiert — muss aber mit Block-Layout interagieren

---

### Station 14: raijin-language-tools — Hover-Info, Manpages (1-2 Wochen)

**Status:** Nicht in der App.

**Cargo.toml:**
```toml
raijin-language-tools = { workspace = true }
```

**main.rs:**
```rust
raijin_language_tools::init(cx);
```

**Terminal-Erweiterung:**
- Hover über Command-Wort in Block-Output → Manpage-Snippet als Tooltip
- Hover über Pfad → File-Info (size, permissions, git status)
- Hover über Exit-Code in Block-Header → Bedeutung

**Verifikation:**
- Hover in Editor zeigt LSP-Hover wie bisher
- Hover über `git` in Terminal-Output zeigt Manpage-Snippet
- Hover über `/Users/nyxb/file.rs` zeigt File-Stats

**Risiken:** raijin-language-tools ist heute Editor-Hover-System mit LSP-Integration — Terminal-Hover ist eine separate Mechanik und muss neu gebaut werden.

---

### Stationen 15+: Restliche Crates aus Plan 28

In willkürlicher Reihenfolge nach Bedarf:

- **raijin-image-viewer** — Bilder im File-Finder klickbar
- **raijin-markdown-preview** — Markdown-Files im Editor mit Preview
- **raijin-svg-preview** — SVG-Preview
- **raijin-csv-preview** — CSV-Tabellen
- **raijin-remote** + **raijin-remote-server** + **raijin-remote-connection** — SSH/WSL/Docker Transport (großer Brocken, eigene Phase wert)
- **raijin-debugger-ui** + **raijin-dap** — Debug Adapter Protocol
- **raijin-task** + **raijin-tasks-ui** — Task System (siehe auch Plan 30 für TOML-Migration)
- **raijin-git-ui** + **raijin-git-graph** — Git UI
- **raijin-collab** + **raijin-collab-ui** — Multiplayer Editing (sehr spät, vermutlich nie in MVP)
- **raijin-extension-host** + **raijin-extensions-ui** — Extension System Runtime (Plan 27 ist erledigt, aber Verdrahtung in der App fehlt)
- **raijin-debug-adapter-extension**, **raijin-language-extension**, **raijin-theme-extension** — Extension-Subsysteme
- **raijin-channel**, **raijin-feedback**, **raijin-notifications**, **raijin-feature-flags** — Sozial- und Telemetrie-Features
- **raijin-onboarding**, **raijin-ai-onboarding**, **raijin-language-onboarding** — Onboarding-Flows
- **raijin-livekit-api**, **raijin-livekit-client**, **raijin-call**, **raijin-audio**, **raijin-media** — Voice/Video für Collab
- **raijin-snippet**, **raijin-snippet-provider**, **raijin-snippets-ui** — Snippets
- **raijin-toolchain-selector**, **raijin-language-selector**, **raijin-line-ending-selector**, **raijin-encoding-selector** — Selectors
- **raijin-jupyter** Connector falls separat von raijin-repl
- **raijin-journal** — Daily-Journal-Feature
- **raijin-recent-projects** — Recent Projects Picker
- **raijin-keymap-editor** — Visual Keymap Editor
- **raijin-component-preview**, **raijin-storybook**, **raijin-inspector-ui** — Dev-Tools

**Jede dieser Stationen ist eigenständig priorisierbar.** Reihenfolge nach Bedarf entscheiden, nicht hier festschreiben.

---

## Was nicht in Phase 25 ist

- **Plan 30 (Task TOML Migration)** — separates Projekt, kein Fusion-Thema. Verdrahtung von raijin-task ist eine eigene Station, aber die TOML-Migration ist orthogonal
- **Plan 35 (Dead Code Cleanup)** — **Vor-Phase**, läuft vor Station 1
- **Plan 27 (Extension System Rewrite)** — erledigt, im `done/` Verzeichnis
- **Plan 28 (Features and Extensions)** — bleibt als Audit-Wahrheitsquelle, wird nicht angefasst. Die "Code da, nicht verdrahtet"-Spalten dort sind die Realität, an der wir uns orientieren
- **Cloud Agents / Oz-Equivalent** — wenn überhaupt, dann sehr spät nach Station 11 als eigene Station
- **Multi-Cursor im Terminal** — Terminal-Input ist single-line/single-cursor, kein Mehrwert
- **Terminal-Recording / Replay** — eigenes Feature
- **Block-basierte Diff-View** ("vor Command vs nach Command")
- **Linux/Windows Platform-Layer** — Plan 28 listet das als "neu bauen", ist eigenes Phasen-Projekt

## Globale Architektur-Risiken

Übergreifend für alle Stationen:

1. **Editor-Tests dürfen nie brechen.** Bei jeder Station die ein neues Trait einführt (Outlineable in 7, Diagnosable in 8) muss die Editor-Implementation 1:1 gleich funktionieren. Vor Stationen 7 und 8 alle relevanten Editor-Tests identifizieren und als Regression-Suite einfrieren.

2. **Performance bei großen Terminals.** 10k+ Blocks ist real bei Power-Usern. Stable IDs, Snapshot-Caching, virtuelle Listen sind Pflicht ab Station 5 (Search). Performance-Tests pro Station, nicht erst am Ende.

3. **Cargo-Dep-Cascade.** Jede Station kann transitiv weitere Crates in die App ziehen die wir nicht erwartet haben. Vor jeder Station: `cargo tree -p <crate>` schauen und entscheiden ob die transitiven Deps OK sind oder ob Station vorgezogen werden muss.

4. **Editor-spezifische Helper aus raijin-editor.** Crates wie raijin-tab-switcher importieren `raijin_editor::items::entry_diagnostic_aware_icon_decoration_and_color` etc. Das sind reine Helper ohne Editor-State. Sollte langfristig in ein `raijin-item-icons` Util-Crate verschoben werden — kann aber bis Station 4 warten (wenn raijin-editor sowieso in der App ist, ist's egal).

5. **Vim Shared Registers (Station 9).** Workspace-globaler Register-Store ist ein Sync-Problem. Last-write-wins muss explizit dokumentiert sein, sonst wundert sich der nächste Dev.

6. **AI-Provider-Versions-Drift (Station 10).** 15 Provider-Crates mit eigenen HTTP-Client-Versionen können dependency-conflicts produzieren. Vor Station 10 alle Provider-Crates auf gemeinsame `reqwest`/`http`-Version bringen.

## Erfolgs-Kriterien — User-Statements als Test

Phase 25 ist "fertig" wenn der User folgende Sätze als wahr empfindet:

- [ ] "Mein Terminal-Tab hat ein Shell-Icon." (Station 1)
- [ ] "Wenn ich im Terminal bin, sehe ich oben den CWD und kann ihn anklicken." (Station 2)
- [ ] "Cmd+P startet im Verzeichnis wo mein Terminal gerade ist." (Station 3)
- [ ] "Project-Panel highlightet das Verzeichnis wo mein Terminal ist." (Station 4)
- [ ] "Cmd+F sucht in dem was ich gerade ansehe — egal ob Code oder Terminal-Output." (Station 5)
- [ ] "Cmd+G springt zur Zeile im Editor und zum Block im Terminal." (Station 6)
- [ ] "Outline zeigt mir was Sinn macht — Funktionen im Code, Commands im Terminal." (Station 7)
- [ ] "Wenn was schiefgeht, sehe ich's im Issues-Panel — egal ob LSP-Error oder failed Command." (Station 8)
- [ ] "Mein Vim-Setup funktioniert auch im Terminal-Input." (Station 9)
- [ ] "Ich kann meinen Lieblings-AI-Provider in den Settings einstellen." (Station 10)
- [ ] "Ich kann den Agent fragen mein Build-Problem zu lösen, er sieht meinen Terminal-Output." (Station 11)
- [ ] "Copilot schlägt mir Shell-Commands vor während ich tippe." (Station 12)
- [ ] "Wenn ich `python` starte, kann ich Plots direkt im Terminal sehen." (Station 13)
- [ ] "Hover über einen Command im Terminal-Output zeigt mir die Manpage." (Station 14)
- [ ] "Es gibt keinen Editor-only-Knopf mehr — alles funktioniert auch mit Terminal aktiv." (Konsequenz aus allem)

## Stations-Dependencies untereinander

```
Plan 35 (Dead Code Cleanup + AppShell-Crate-Extraktion) ──→ Pre-Work (Item-Basis-Methoden)
                                          ↓
                              Station 1 (tab-switcher)
                                          ↓
                              Station 2 (breadcrumbs)
                                          ↓
                              Station 3 (file-finder)        ─┐
                                          ↓                    │
                              Station 4 (project-panel)        │ → ab hier
                                          ↓                    │   ist raijin-editor
                              Station 5 (search)              ─┘   in der App
                                          ↓
                              Station 6 (go-to-line)
                                          ↓
                              Station 7 (outline)  ← braucht Outlineable Trait
                                          ↓
                              Station 8 (diagnostics) ← größter Brocken, neues Trait + neues Crate
                                          ↓
                              Station 9 (vim) ── parallel-fähig ab hier
                                          ↓
                              Station 10 (language-models + provider)
                                          ↓
                              Station 11 (agent) ← braucht Station 10
                                          ↓
                              Station 12 (copilot) ── parallel zu 11 möglich
                                          ↓
                              Station 13 (repl) ── unabhängig, kann nach Station 5 laufen
                                          ↓
                              Station 14 (language-tools) ── unabhängig
                                          ↓
                              Stationen 15+
```

Stationen 9, 12, 13, 14 sind **parallel-fähig** sobald die jeweiligen Voraussetzungen erfüllt sind. Stationen 1-8 sind **strikt linear**, weil jede die nächste vorbereitet (Cargo-Deps cascadieren, Item-Methoden bauen aufeinander).

## Erste Schritte (heute machbar)

1. **Plan 35 starten** (Subphase 1: `with_active_or_new_workspace` ersetzen). Eine Woche.
2. Wenn Plan 35 fertig: **Pre-Work auf TerminalPane** in `terminal_pane.rs`. Tag.
3. Station 1 prüfen: kompiliert raijin-tab-switcher heute ohne raijin-editor in der App? `cargo build -p raijin-tab-switcher`. Falls ja: Station 1 fertig nach Pre-Work-Commit.
4. Station 2: raijin-breadcrumbs in Cargo.toml + `init()` + visueller Test. Tag.
5. Station 3: raijin-file-finder Cargo-Tree analysieren, dann verdrahten. 2-3 Tage.

Wenn die ersten 5 Tage vier sichtbare Wins produzieren (Tab-Icon, Breadcrumbs, File-Finder im CWD, Project-Panel-Highlight), ist der Beweis erbracht dass die Strategie trägt. Dann weiter.

## Phase 26, 29 als historische Referenz

Plan 26 (Agentic Development Environment) und Plan 29 (Hybrid Terminal-REPL) bleiben als **Vision-Dokumente** im Plan-Ordner, werden aber **nicht** als eigenständige Phasen ausgeführt. Ihr Inhalt wird durch Stationen 11 und 13 dieser Master-Liste umgesetzt. Sobald Station 11 fertig ist, wandert Plan 26 ins `done/`. Sobald Station 13 fertig ist, wandert Plan 29 ins `done/`.

Die Plan-Files dienen als detaillierte Spec für die jeweilige Station. Wer Station 11 implementiert, liest Plan 26 als Anforderungsdokument. Wer Station 13 implementiert, liest Plan 29.

Plan 28 (Features and Extensions) bleibt als Audit-Wahrheitsquelle und wird parallel zur Master-Liste gepflegt — die "Code da / nicht verdrahtet" Spalten werden Station für Station aktualisiert wenn Crates verdrahtet werden.

Plan 30 (Task System TOML Migration) bleibt eigenständiges Projekt, läuft parallel oder nach Phase 25 — keine Abhängigkeit.
