# Phase 35: Zed Dead Code Cleanup — Terminal-first Window-Modell

## Ziel

Zed-Editor-Semantik entfernen die für Raijin's Terminal-first Modell irrelevant ist. MultiWorkspace, WorkspaceStore, und die "Projekt → Fenster"-Logik raus. Das Window-Modell wird: **ein Fenster, ein Workspace, Tabs als Kontext-Container.**

## Warum

Zed: MultiWorkspace → Workspace₁ + Workspace₂ (ein Projekt pro Workspace, neues Fenster pro Projekt)
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
- `raijin-settings-profile-selector/src/settings_profile_selector.rs:12`
- `raijin-onboarding/src/onboarding.rs:56,83,340`
- `raijin-keymap-editor/src/keymap_editor.rs:134`
- `raijin-dev-container/src/lib.rs:100`
- `raijin-recent-projects/src/recent_projects.rs:224,308,320,389,415,431`

---

## Subphase 2: `MoveWorkspaceToNewWindow` Action entfernen

**Aufwand:** Minimal (3 Dateien, ~10 Zeilen)

**Dateien:**
- `raijin-actions/src/lib.rs:790` — Action-Definition entfernen
- `raijin-sidebar/src/sidebar.rs:1445` — Context-Menu-Eintrag entfernen
- `raijin-workspace/src/multi_workspace.rs:18,780` — Import + Handler entfernen

---

## Subphase 3: `WorkspaceStore` entfernen

**Aufwand:** Mittel (5 Dateien, ~150 Zeilen)

WorkspaceStore trackt Workspaces über Fenster für Zed-Collaboration. Raijin hat ein Fenster → kein Tracking nötig.

**Was passiert mit den Methoden:**
- `workspaces()` / `workspaces_with_windows()` — nicht mehr nötig (ein Fenster)
- `update_followers()` / `handle_follow()` / `handle_update_followers()` — Collaboration-Protokoll, tot für uns
- Message-Handler-Registrierung — Collaboration, tot

**Dateien:**
- `raijin-workspace/src/workspace.rs:1079-1083` — Struct-Definition entfernen
- `raijin-workspace/src/workspace.rs:8335-8443` — Impl-Block entfernen
- `raijin-workspace/src/workspace.rs:1068` — Feld aus AppState entfernen
- `raijin-app/src/app_bootstrap.rs:39` — WorkspaceStore-Erstellung entfernen
- `raijin-settings-ui/src/settings_ui.rs:1544,3373,3755` — `.workspaces()` Aufrufe ersetzen
- `raijin-edit-prediction/src/edit_prediction.rs:2000` — `.workspaces()` Aufruf ersetzen

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
- `multi_workspace_enabled()` Check in `multi_workspace.rs:267` entfällt mit der Datei
- `cx.has_flag::<AgentV2FeatureFlag>()` in `sidebar.rs:322` — nur den Multi-Workspace-Guard entfernen, andere Agent-Guards behalten

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
