# Phase 35: Dead Code Cleanup — Terminal-first Window-Modell

> ## Window-Root-Entscheidung: Pattern B bestätigt (April 2026)
>
> **Entscheidung:** AppShell bleibt Window-Root, Workspace wird Child.
>
> **Raijin Window-Hierarchie nach Cleanup:**
> ```
> Inazuma (PlatformWindow) → AppShell (Window-Root) → Workspace (Content-Root)
> ```
>
> **Begründung (Web-Recherche verifiziert):**
> - Ghostty validiert das 3-Layer-Modell: Platform → Runtime-Shell → Core. Raijin's Äquivalent: Inazuma → AppShell → Workspace.
> - Zed hat keinen AppShell und bezahlt mit 2+ Jahre alten Window-Decoration-Issues auf Linux (#14120, #14131, #14165, #40075, #40657).
> - WezTerm's `TermWindow` ist Shell+Renderer in einem — funktioniert weil sie alles custom rendern. Raijin will platform-sensitives Chrome → braucht Separation.
> - Tauri separiert `tao` (Window) von `wry` (Content) als Architektur-Prinzip. Gleiche Logik.
>
> **AppShell Verantwortlichkeiten (Window-Level Concerns):**
> - Sheet-Layer, Dialog-Layer, NotificationList (Overlay-Koordination)
> - Window-Border mit Shadow (cross-platform)
> - Tab/Shift-Tab Focus-Navigation zwischen Overlays
> - Theme/Background/Font Application auf Window-Level
> - Zukünftig: macOS Sheets, Windows Mica, Linux CSD — alles `cfg!(target_os)` gehört hierher
>
> **Workspace Verantwortlichkeiten (Content-Level Concerns):**
> - Panes, Docks, Panels, Items
> - `modal_layer` (Command Palette, File Finder, Tab Switcher) — das sind Pane-Modals, KEIN Window-Level
> - `toast_layer` — Workspace-Level Feedback
> - Persistence (WorkspaceDb)
>
> **Regel: Workspace darf KEINEN `cfg!(target_os)` haben.** Alles platform-spezifische geht in AppShell oder in Inazuma's PlatformWindow.
>
> **API-Vorgabe für Subphase 1+4:**
> ```rust
> impl AppShell {
>     /// Zugriff auf den Workspace im Window. Ersetzt alle MultiWorkspace-Downcasts.
>     pub fn workspace(window: &Window, cx: &App) -> Option<Entity<Workspace>> {
>         window.root::<AppShell>()
>             .flatten()
>             .map(|shell| shell.read(cx).view().downcast::<Workspace>())
>             .flatten()
>     }
> }
>
> /// Convenience: Workspace aus dem aktiven Window holen.
> pub fn with_active_workspace<F, R>(cx: &mut App, f: F) -> Option<R>
> where
>     F: FnOnce(Entity<Workspace>, &mut Window, &mut App) -> R,
> {
>     cx.active_window().and_then(|window_handle| {
>         window_handle.update(cx, |_, window, cx| {
>             AppShell::workspace(window, cx).map(|ws| f(ws, window, cx))
>         }).ok().flatten()
>     })
> }
> ```
>
> **Konsequenz für Subphase 4:** Alle `window.downcast::<MultiWorkspace>()` werden zu `AppShell::workspace(window, cx)`. Kein Caller muss wissen dass AppShell existiert — die Helper-Funktion abstrahiert das.
>
> **Was NICHT Plan-35-Scope ist:** Die Duplikation zwischen Workspace's `notifications.rs` und AppShell's `NotificationList` wird in einem Folge-Plan konsolidiert, nicht hier.

> **STATUS: Vor-Phase für Plan 25, verifiziert April 2026.**
>
> **Verifiziert gegen den Code:**
> - `crates/raijin-workspace/src/multi_workspace.rs` existiert (36 KB) ✓
> - `crates/raijin-workspace/src/workspace.rs` exportiert `MultiWorkspace`, `MultiWorkspaceEvent`, `Sidebar` etc. ✓
> - `WorkspaceStore` Struct ist in workspace.rs definiert: `pub struct WorkspaceStore { workspaces: HashSet<...>, client: Arc<Client>, _subscriptions: Vec<...> }` ✓
> - `AppState::workspace_store: Entity<WorkspaceStore>` ist real ✓
>
> **Was ich NICHT verifiziert habe (würde mehr Code-Lesen brauchen):**
> - Die exakten Zeilennummern (1079, 8335, 1068) — workspace.rs ist 552 KB groß. Behauptungen sind plausibel aber Zeilen können sich verschoben haben
> - Ob `with_active_or_new_workspace` wirklich schon defanged ist
> - Die genaue Zahl der Caller in den 6 genannten Dateien
>
> **Verhältnis zu Plan 25:** Plan 35 ist **Vorbedingung** für Plan 25. Erst aufräumen, dann Crate für Crate verdrahten. Sonst kämpft jede neue Crate-Verdrahtung gegen `MultiWorkspace`-Downcasts die bei uns immer fehlschlagen. Plan 25 nennt Plan 35 explizit als Pre-Work.
>
> **Vor Implementierung:** Subphase 1 starten und gleich beim ersten `cargo check` schauen ob die Zeilennummern noch stimmen. Wenn nicht: Datei auf der Suche nach den Funktionsnamen anpassen, nicht auf die Zeilennummern verlassen.

---

## Ziel

Geerbte Editor-Semantik entfernen die für Raijin's Terminal-first Modell irrelevant ist. MultiWorkspace, WorkspaceStore, und die "Projekt → Fenster"-Logik raus. Das Window-Modell wird: **ein Fenster, ein Workspace, Tabs als Kontext-Container.**

## Warum

Referenz: MultiWorkspace → Workspace₁ + Workspace₂ (ein Projekt pro Workspace, neues Fenster pro Projekt)
Raijin: AppShell → Workspace → Tabs (ein Fenster, Projekt-Kontext pro Tab via ProjectRegistry)

Die geerbte MultiWorkspace-Infrastruktur verursacht:
- Fenster-Explosion (`with_active_or_new_workspace` öffnet Fenster wenn kein MultiWorkspace-Root)
- Toten Code der die Codebase aufbläht (~1000 Zeilen in multi_workspace.rs allein)
- Verwirrende Downcasts (`window.downcast::<MultiWorkspace>()`) die bei uns immer fehlschlagen

## Voraussetzungen

- ✅ Phase 20 (Workspace Integration) — fertig
- ✅ ProjectRegistry (Phase 2 aus Reaktive Projekt-Detection Plan) — fertig
- ✅ `with_active_or_new_workspace` defanged — fertig
- ✅ `--printenv` Early-Return — fertig

## Umfang

50+ Dateien, ~100+ Änderungen. Aufgeteilt in 5 Subphasen die jeweils einzeln kompilieren.

---

## Subphase 1: `with_active_or_new_workspace` ersetzen und entfernen

**Aufwand:** Klein (6 Dateien, ~30 Zeilen)

Die Funktion ist bereits defanged (loggt nur Warning). Alle Caller müssen auf einen direkten Workspace-Zugriff umgestellt werden.

**Replacement-Pattern:** Statt `with_active_or_new_workspace(cx, |ws, window, cx| { ... })` nutzen Caller:
```rust
if let Some(window) = cx.active_window() {
    window.update(cx, |_, window, cx| {
        if let Some(workspace) = window.root::<AppShell>()
            .and_then(|shell| /* get workspace from shell */)
        {
            workspace.update(cx, |ws, cx| { ... });
        }
    }).ok();
}
```

Oder noch besser: eine neue Funktion `with_active_workspace(cx, f)` die direkt auf unsere AppShell→Workspace Kette zugreift.

**Dateien:**
- `raijin-workspace/src/workspace.rs` — Funktion entfernen, `with_active_workspace` als Replacement
- `raijin-settings-profile-selector/src/settings_profile_selector.rs`
- `raijin-onboarding/src/onboarding.rs`
- `raijin-keymap-editor/src/keymap_editor.rs`
- `raijin-dev-container/src/lib.rs`
- `raijin-recent-projects/src/recent_projects.rs`

---

## Subphase 2: `MoveWorkspaceToNewWindow` Action entfernen

**Aufwand:** Minimal (3 Dateien, ~10 Zeilen)

**Dateien:**
- `raijin-actions/src/lib.rs` — Action-Definition entfernen
- `raijin-sidebar/src/sidebar.rs` — Context-Menu-Eintrag entfernen
- `raijin-workspace/src/multi_workspace.rs` — Import + Handler entfernen

---

## Subphase 3: `WorkspaceStore` entfernen

**Aufwand:** Mittel (5 Dateien, ~150 Zeilen)

WorkspaceStore trackt Workspaces über Fenster für Collaboration. Raijin hat ein Fenster → kein Tracking nötig.

**Was passiert mit den Methoden:**
- `workspaces()` / `workspaces_with_windows()` — nicht mehr nötig (ein Fenster)
- `update_followers()` / `handle_follow()` / `handle_update_followers()` — Collaboration-Protokoll, tot für uns
- Message-Handler-Registrierung — Collaboration, tot

**Dateien:**
- `raijin-workspace/src/workspace.rs` — `pub struct WorkspaceStore { ... }` Definition entfernen
- `raijin-workspace/src/workspace.rs` — `impl WorkspaceStore` Block entfernen
- `raijin-workspace/src/workspace.rs` — `workspace_store: Entity<WorkspaceStore>` Feld aus AppState entfernen
- `raijin-app/src/app_bootstrap.rs` — WorkspaceStore-Erstellung entfernen
- `raijin-settings-ui/src/settings_ui.rs` — `.workspaces()` Aufrufe ersetzen
- `raijin-edit-prediction/src/edit_prediction.rs` — `.workspaces()` Aufruf ersetzen

**AppState-Änderung:**
```rust
// Vorher:
pub struct AppState {
    pub workspace_store: Entity<WorkspaceStore>,  // ENTFERNEN
    ...
}

// Nachher:
pub struct AppState {
    ...  // workspace_store Feld weg
}
```

---

## Subphase 4: MultiWorkspace-Downcasts neutralisieren

**Aufwand:** Groß (40+ Dateien, ~80 Änderungen)

Überall wo `window.downcast::<MultiWorkspace>()` steht, wird es durch einen sicheren Pfad ersetzt. Drei Patterns:

**Pattern A — Downcast entfernen (kein Replacement nötig):**
Für Stellen die nur `MultiWorkspace` casten um an den `Workspace` zu kommen. Da wir nur einen Workspace haben, kann direkt auf den Workspace zugegriffen werden.

**Pattern B — Optional machen:**
```rust
// Vorher:
let mw = window.downcast::<MultiWorkspace>().unwrap();

// Nachher:
// Entfernen — Raijin hat kein MultiWorkspace
```

**Pattern C — Workspace direkt nutzen:**
Für Stellen die `.workspace()` auf dem MultiWorkspace aufrufen.

**Betroffene Crates (Production-Code):**
- `raijin-workspace/src/workspace.rs` — 15+ Downcasts
- `raijin-sidebar/src/sidebar.rs` — MultiWorkspaceEvent Subscription + Rendering
- `raijin-title-bar/src/title_bar.rs` — `is_multi_workspace_enabled` Check
- `raijin-workspace/src/status_bar.rs` — `multi_workspace_enabled` Conditional
- `raijin-settings-ui/src/settings_ui.rs` — Window-Handle-Casting
- `raijin-vim/src/state.rs` — Window-Iteration
- `raijin-git-ui/src/worktree_picker.rs` — Downcast
- `raijin-collab-ui/src/collab_panel.rs` — Downcast
- `raijin-rules-library/src/rules_library.rs` — Downcast
- `raijin-agent-ui/src/conversation_view.rs` — Downcast
- `raijin-recent-projects/src/*.rs` — Multiple Downcasts
- `raijin-workspace/src/notifications.rs` — Downcast

**Hinweis zu den Crates oben:** Viele dieser Crates sind heute **nicht** in `raijin-app/Cargo.toml` (siehe Plan 25 Code-Inventur). Subphase 4 muss nur die Downcasts in Crates fixen die wirklich kompiliert werden — der Rest fixt sich automatisch wenn die Crates später in Plan 25 verdrahtet werden. Reihenfolge: erst Plan 35 für aktuelle App-Crates, dann Plan 25 verdrahtet neue Crates und repariert deren Downcasts gleich beim Verdrahten.

**Test-Code (~50 Dateien):**
Alle Tests die `MultiWorkspace::test_new()` nutzen müssen auf ein Test-Helper umgestellt werden der direkt einen `Workspace` erstellt (ohne MultiWorkspace-Wrapper). Das ist der größte Teil der Arbeit.

---

## Subphase 5: `multi_workspace.rs` entfernen

**Aufwand:** Mittel (nach Subphase 4 ist das nur noch Aufräumen)

**Erst wenn alle Downcasts neutralisiert sind:**
- `raijin-workspace/src/multi_workspace.rs` — gesamte Datei entfernen
- `raijin-workspace/src/lib.rs` — Module-Deklaration entfernen
- Re-Exports in `raijin-workspace` anpassen

**AgentV2FeatureFlag:**
- Flag BEHALTEN (wird von Agent-UI für andere Features genutzt)
- `multi_workspace_enabled()` Check entfällt mit der Datei
- `cx.has_flag::<AgentV2FeatureFlag>()` in `sidebar.rs` — nur den Multi-Workspace-Guard entfernen, andere Agent-Guards behalten

---

## Was NICHT angefasst wird

- `Workspace` selbst (Panes, Docks, Items, Modals, Persistence)
- `Project` (Worktrees, LSP, Git, Buffers)
- `SerializableItem` / Session-Restore
- `AppState` Kern (LanguageRegistry, Fs, NodeRuntime, Session)
- `open_new()` — bleibt als Workspace-Factory
- `find_existing_workspace()` — bleibt für Window-Reuse
- `open_workspace_by_id()` — bleibt für Session-Restore
- `AgentV2FeatureFlag` — bleibt für Agent-UI

## Reihenfolge & Abhängigkeiten

```
Subphase 1 (with_active_or_new_workspace ersetzen)
    ↓
Subphase 2 (MoveWorkspaceToNewWindow entfernen)
    ↓
Subphase 3 (WorkspaceStore entfernen)
    ↓
Subphase 4 (MultiWorkspace-Downcasts neutralisieren)  ← größter Block
    ↓
Subphase 5 (multi_workspace.rs entfernen)
```

Jede Subphase muss einzeln kompilieren und darf keine bestehende Funktionalität brechen.

## Verification

Nach jeder Subphase:
1. `cargo check --workspace` — keine Compile-Errors
2. `cargo run -p raijin-app` — Ein Fenster, kein zweites
3. `cd` in Git-Repo — Worktree wird erstellt, kein zweites Fenster
4. Tab-Wechsel — funktioniert normal
5. Command Palette — öffnet sich, keine Fenster-Explosion
