# Plan 35 — Korrekturen und neue Subphase 0

## Kontext

Der aktuelle Plan 35 geht davon aus, dass nach dem Cleanup **Workspace direkt als Window-Root** dient und AppShell entfernt/absorbiert wird. Das ist falsch. Nach Web-Recherche (Ghostty, WezTerm, Zed, Tauri) und Code-Analyse haben wir uns für **Pattern B** entschieden:

**AppShell bleibt Window-Root. Workspace bleibt Content-Child.**

```
Inazuma (PlatformWindow) → AppShell (Window-Root) → Workspace (Content-Root)
```

Das ist Ghostty's 3-Layer-Modell (Platform → Runtime-Shell → Core) auf Raijin übertragen. Zed hat keinen AppShell und bezahlt dafür mit 2+ Jahre alten Window-Decoration-Bugs auf Linux (Issues #14120, #14131, #14165, #40075).

Zusätzlich: AppShell wird aus `raijin-ui` in eine eigene Crate **`raijin-shell`** extrahiert. Die bisherige `raijin-shell` Crate (ShellContext, ShellMetadataPayload, shell_install) wird zu **`raijin-shell-integration`** umbenannt. Begründung: AppShell ist konzeptionell kein UI-Component wie Button oder Dialog — es ist eine architektonische Schicht (Window-Root). Und heute gibt es eine zirkuläre Abhängigkeit zwischen AppShell und raijin-ui (Dialog/Input/window_ext rufen `AppShell::update()` direkt auf), die als Teil dieses Cleanups sauber aufgelöst wird.

---

## Was am bestehenden Plan geändert werden muss

### 1. Context-Abschnitt — falscher Satz

**Raus:**
> "main.rs ändert sich zu `Workspace` als Root, AppShell-Wrapping wird in Workspace::render() integriert oder direkt entfernt wenn nicht aktiv genutzt."

**Rein:**
> "AppShell bleibt Window-Root. main.rs:210 ist bereits korrekt (`AppShell::new(workspace, window, cx)`). Die Open-Funktionen in workspace.rs die noch `MultiWorkspace::new()` nutzen werden auf `AppShell::new()` umgestellt."

### 2. Subphase 4.B — Window-Erstellung

**Falsch:** `MultiWorkspace::new(workspace, window, cx)` → `workspace` direkt zurückgeben

**Richtig:** `MultiWorkspace::new(workspace, window, cx)` → `AppShell::new(workspace, window, cx)`

Das ist was main.rs heute schon macht. Die 15 Open-Stellen werden auf das main.rs-Pattern angeglichen.

### 3. Subphase 4.C — Typ-Signaturen

**Falsch:** `WindowHandle<MultiWorkspace>` → `WindowHandle<Workspace>`

**Richtig:** `WindowHandle<MultiWorkspace>` → `WindowHandle<AppShell>`

AppShell ist der Window-Root. `window.root::<T>()` gibt AppShell zurück, nicht Workspace.

### 4. Subphase 4.D — Downcasts

**Falsch:** `downcast::<MultiWorkspace>()` → `downcast::<Workspace>()`

**Richtig:** Alle 46 Stellen nutzen den neuen Helper:

```rust
// In raijin-shell (neue Crate):
impl AppShell {
    pub fn workspace(window: &Window, cx: &App) -> Option<Entity<Workspace>> {
        window.root::<AppShell>()
            .flatten()
            .and_then(|shell| shell.read(cx).view().downcast::<Workspace>())
    }
}
```

Die Downcast-Stellen werden zu `AppShell::workspace(window, cx)` — ein Aufruf statt zwei Schritte.

### 5. Subphase 4.G — Komplett streichen

Der ganze Abschnitt "main.rs anpassen" muss raus. main.rs ist bereits korrekt. Der Abschnitt sagt "AppShell entfernen oder in Workspace integrieren" — genau das Gegenteil unserer Entscheidung.

### 6. Subphase 4.H — `with_active_workspace` Signatur

**Falsch:**
```rust
window.downcast::<Workspace>()
```

**Richtig:**
```rust
pub fn with_active_workspace(
    cx: &mut App,
    f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) + Send + 'static,
) {
    if let Some(window) = cx.active_window() {
        cx.defer(move |cx| {
            window.update(cx, |_, window, cx| {
                if let Some(workspace) = AppShell::workspace(window, cx) {
                    workspace.update(cx, |ws, cx| f(ws, window, cx));
                }
            }).log_err();
        });
    }
}
```

Diese Funktion lebt in `raijin-workspace`, importiert `AppShell` aus `raijin-shell`.

---

## Neue Subphase 0: Crate-Restrukturierung (VOR allem anderen)

Muss als erstes laufen weil ab Subphase 1 schon `AppShell::workspace()` gebraucht wird.

### 0.A: `raijin-shell` → `raijin-shell-integration` umbenennen

Das bestehende `raijin-shell` (ShellContext, GitStats, ShellMetadataPayload, shell_install) wird umbenannt.

**Blast-Radius:** Klein — 4 Consumer-Crates, ~8 Import-Zeilen.

| Was | Änderung |
|-----|----------|
| Ordner | `crates/raijin-shell/` → `crates/raijin-shell-integration/` |
| `crates/raijin-shell-integration/Cargo.toml` | `name = "raijin-shell-integration"` |
| Root `Cargo.toml` Members | Pfad anpassen |
| Root `Cargo.toml` Workspace-Dep | `raijin-shell-integration = { path = "crates/raijin-shell-integration" }` |
| `raijin-app/Cargo.toml` | `raijin-shell-integration = { workspace = true }` |
| `raijin-terminal-view/Cargo.toml` | gleich |
| `raijin-completions/Cargo.toml` | gleich |
| `raijin-chips/Cargo.toml` | gleich |
| `raijin-terminal-view/src/terminal_pane.rs:14,25,347` | `use raijin_shell_integration::` |
| `raijin-completions/src/shell_completion.rs:21` | `use raijin_shell_integration::` |
| `raijin-chips/src/context.rs:7` | `use raijin_shell_integration::` |
| `raijin-app/src/main.rs` | `use raijin_shell_integration::` (falls main.rs direkt importiert) |

**Verify:** `cargo check --workspace`

### 0.B: Zirkuläre Abhängigkeit in raijin-ui auflösen

Bevor AppShell rausgezogen werden kann, müssen die 17 Stellen in raijin-ui die direkt `AppShell::update()` / `AppShell::read()` aufrufen auf saubere Patterns umgestellt werden. Das sind Anti-Patterns — Inazuma ist ein Action-basiertes Framework, Children sollten nie direkt in den Root greifen.

**Drei Gruppen von Abhängigkeiten:**

#### Gruppe 1: Dialog → AppShell (2 Stellen)

**Stelle 1** — `dialog.rs:384-388` — Dialog close:
```rust
// VORHER (Anti-Pattern: direkter Root-Zugriff):
fn defer_close_dialog(window: &mut Window, cx: &mut App) {
    AppShell::update(window, cx, |root, window, cx| {
        root.defer_close_dialog(window, cx);
    });
}

// NACHHER (Action-Pattern: bubbled up through element tree):
fn defer_close_dialog(window: &mut Window, cx: &mut App) {
    window.dispatch_action(&DeferCloseDialog, cx);
}
```

Dafür neue Actions in raijin-ui definieren (diese kennen AppShell nicht):
```rust
actions!(window_shell, [
    CloseDialog,
    DeferCloseDialog,
    CloseAllDialogs,
    CloseSheet,
    ClearNotifications,
]);
```

AppShell registriert Handler für diese Actions in seiner `render()` Methode:
```rust
.on_action(cx.listener(|this, _: &CloseDialog, window, cx| this.close_dialog(window, cx)))
.on_action(cx.listener(|this, _: &DeferCloseDialog, window, cx| this.defer_close_dialog(window, cx)))
// etc.
```

**Stelle 2** — `dialog.rs:498` — `active_dialog_count()` Check:
```rust
// VORHER:
if (self.layer_ix + 1) != AppShell::read(window, cx).active_dialog_count() {

// NACHHER: Dialog bekommt ein `is_topmost: bool` Prop statt selbst zu lesen.
// AppShell setzt dieses Prop schon in build_dialog_element() (dort wird layer_ix
// bereits gesetzt — einfach is_topmost dazu):
if !self.is_topmost {
```

In `AppShell::build_dialog_element()` (dialog_layer.rs Zeile 114-124):
```rust
// Bestehendes:
dialog.layer_ix = i;

// Neues Feld dazu:
dialog.is_topmost = (i + 1) == self.active_dialogs.len();
```

#### Gruppe 2: Input → AppShell (5 Stellen)

Input nutzt AppShell als globalen Tracker für "welches Input hat gerade Focus". Das gehört in einen eigenständigen Global, nicht in AppShell.

Neuer Typ in raijin-ui (z.B. `src/components/input/focus_tracker.rs`):
```rust
use inazuma::Global;

#[derive(Default)]
pub struct FocusedInputTracker {
    pub focused: Option<Entity<InputState>>,
}

impl Global for FocusedInputTracker {}
```

Initialisierung in raijin-ui's `init()`:
```rust
cx.set_global(FocusedInputTracker::default());
```

**input/element.rs:555-570** (4 Stellen):
```rust
// VORHER:
if AppShell::read(window, cx).focused_input() != Some(&state) {
    AppShell::update(window, cx, |root, _, cx| {
        root.set_focused_input(Some(state));

// NACHHER:
if cx.global::<FocusedInputTracker>().focused.as_ref() != Some(&state) {
    cx.global_mut::<FocusedInputTracker>().focused = Some(state);
    cx.notify();
}
```

**input/state.rs:1115-1117** (1 Stelle — Blur):
```rust
// VORHER:
AppShell::update(window, cx, |root, _, _| {
    root.set_focused_input(None);
});

// NACHHER:
cx.global_mut::<FocusedInputTracker>().focused = None;
```

AppShell's `focused_input` Feld und Getter/Setter entfallen — AppShell liest bei Bedarf aus dem Global.

#### Gruppe 3: window_ext.rs → AppShell (8 Stellen)

`WindowExt` Trait ist die Convenience-Bridge zwischen Window und AppShell. Bleibt als Trait in raijin-ui definiert, **Implementation wandert aber nach raijin-shell** — dort kennt man AppShell.

In raijin-ui:
```rust
// src/utils/window_ext.rs — NUR der Trait, keine Impl:
pub trait WindowExt: Sized {
    fn open_sheet<F>(&mut self, cx: &mut App, build: F) where F: ...;
    fn open_sheet_at<F>(&mut self, placement: Placement, cx: &mut App, build: F) where F: ...;
    fn close_sheet(&mut self, cx: &mut App);
    fn open_dialog<F>(&mut self, cx: &mut App, build: F) where F: ...;
    fn close_dialog(&mut self, cx: &mut App);
    fn close_all_dialogs(&mut self, cx: &mut App);
    fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App);
    fn remove_notification<T: Sized + 'static>(&mut self, cx: &mut App);
    fn clear_notifications(&mut self, cx: &mut App);
}
```

In raijin-shell:
```rust
// src/window_ext_impl.rs — Impl mit AppShell-Zugriff:
impl WindowExt for Window {
    fn open_dialog<F>(&mut self, cx: &mut App, build: F) where F: ... {
        AppShell::update(self, cx, |shell, window, cx| {
            shell.open_dialog(build, window, cx);
        });
    }
    // ... alle 8 Methoden
}
```

**Alternativ** (falls die Trait-Impl-in-anderer-Crate orphan-rule-Probleme macht): `WindowExt` komplett nach raijin-shell verschieben. Consumer die `window.open_dialog()` brauchen importieren dann `raijin_shell::WindowExt` statt `raijin_ui::WindowExt`. Das ist sauberer — raijin-ui Primitives sollten keine Window-Level-Operationen als Extension anbieten.

#### Gruppe 4: Test-Code (2 Stellen)

**input/state.rs:1468,1485** — Test-Helper nutzt `WindowHandle<AppShell>` und `AppShell::new()`:
```rust
// Tests importieren AppShell aus raijin-shell:
#[cfg(test)]
use raijin_shell::AppShell;
```

Das funktioniert weil Test-Dependencies anders aufgelöst werden — `raijin-ui` kann `raijin-shell` als Dev-Dependency haben:
```toml
[dev-dependencies]
raijin-shell.workspace = true
```

### 0.C: AppShell in neue Crate `raijin-shell` extrahieren

Nachdem die zirkulären Deps aufgelöst sind:

**Neue Crate erstellen:** `crates/raijin-shell/`

```toml
# crates/raijin-shell/Cargo.toml
[package]
name = "raijin-shell"
version = "0.1.0"
edition.workspace = true
publish.workspace = true

[dependencies]
inazuma.workspace = true
raijin-ui.workspace = true
raijin-theme.workspace = true
```

**Dateien verschieben:**
| Von | Nach |
|-----|------|
| `raijin-ui/src/components/app_shell/shell.rs` | `raijin-shell/src/shell.rs` |
| `raijin-ui/src/components/app_shell/dialog_layer.rs` | `raijin-shell/src/dialog_layer.rs` |
| `raijin-ui/src/components/app_shell/sheet_layer.rs` | `raijin-shell/src/sheet_layer.rs` |
| `raijin-ui/src/components/app_shell/notification_layer.rs` | `raijin-shell/src/notification_layer.rs` |
| `raijin-ui/src/components/app_shell/focus_navigation.rs` | `raijin-shell/src/focus_navigation.rs` |
| (WindowExt impl) | `raijin-shell/src/window_ext_impl.rs` |

**raijin-shell/src/lib.rs:**
```rust
mod shell;
mod dialog_layer;
mod sheet_layer;
mod notification_layer;
mod focus_navigation;
mod window_ext_impl;

pub use shell::AppShell;
pub use raijin_ui::WindowExt; // Re-Export den Trait
```

**raijin-ui aufräumen:**
- `src/components/app_shell/` Ordner löschen
- `src/components/app_shell.rs` löschen
- `src/components.rs` — `pub mod app_shell;` und `pub use app_shell::*;` entfernen
- `src/utils/window_ext.rs` — nur noch Trait-Definition, keine Impl

**Imports anpassen:**
| Datei | Vorher | Nachher |
|-------|--------|---------|
| `raijin-app/Cargo.toml` | — | `raijin-shell = { workspace = true }` |
| `raijin-app/src/main.rs:10` | `use raijin_ui::AppShell;` | `use raijin_shell::AppShell;` |

**Neuen Helper hinzufügen** (in `raijin-shell/src/shell.rs`):
```rust
impl AppShell {
    /// Zugriff auf den Workspace im Window. Ersetzt alle MultiWorkspace-Downcasts.
    pub fn workspace(window: &Window, cx: &App) -> Option<Entity<Workspace>> {
        window.root::<AppShell>()
            .flatten()
            .and_then(|shell| shell.read(cx).view().downcast::<Workspace>())
    }
}
```

Dependency-Kette verifiziert — keine Zirkularität:
```
raijin-shell → raijin-workspace  ✅ (für Entity<Workspace> Typ + AppShell::workspace() Helper)
raijin-shell → raijin-ui         ✅ (für Dialog, Sheet, Notification Typen)
raijin-workspace → raijin-ui     ✅ (besteht heute schon, Zeile 67 in Cargo.toml)
raijin-workspace → raijin-shell  ❌ NICHT NÖTIG — kein Import
```

Wichtig: `with_active_workspace()` lebt in **raijin-shell** (nicht in raijin-workspace), weil die Funktion über AppShell an den Workspace geht. Würde sie in raijin-workspace leben, bräuchte raijin-workspace eine Dep auf raijin-shell → Zirkularität. Consumer die `with_active_workspace()` brauchen importieren aus `raijin_shell`.

**Verify:** `cargo check --workspace`

### 0.D: Action-Handler in AppShell registrieren

In `raijin-shell/src/shell.rs`, in `AppShell::render()`:

```rust
// Bestehender Code:
.on_action(cx.listener(Self::on_action_tab))
.on_action(cx.listener(Self::on_action_tab_prev))

// Neue Handler für die Actions aus 0.B:
.on_action(cx.listener(|this, _: &CloseDialog, window, cx| this.close_dialog(window, cx)))
.on_action(cx.listener(|this, _: &DeferCloseDialog, window, cx| this.defer_close_dialog(window, cx)))
.on_action(cx.listener(|this, _: &CloseAllDialogs, window, cx| this.close_all_dialogs(window, cx)))
.on_action(cx.listener(|this, _: &CloseSheet, window, cx| this.close_sheet(window, cx)))
.on_action(cx.listener(|this, _: &ClearNotifications, window, cx| this.clear_notifications(window, cx)))
```

**Verify:** `cargo check --workspace` + `cargo test -p raijin-ui`

---

## Zusammenfassung der neuen Reihenfolge

```
Subphase 0A: raijin-shell → raijin-shell-integration (Umbenennung)
    ↓
Subphase 0B: Zirkuläre Deps in raijin-ui auflösen (Actions + FocusedInputTracker)
    ↓
Subphase 0C: AppShell → neue Crate raijin-shell (5 Files verschieben)
    ↓
Subphase 0D: Action-Handler in AppShell registrieren
    ↓
Subphase 1: with_active_or_new_workspace ersetzen (unverändert)
    ↓
Subphase 2: MoveWorkspaceToNewWindow entfernen (unverändert)
    ↓
Subphase 3: WorkspaceStore entfernen (unverändert)
    ↓
Subphase 4: MultiWorkspace-Downcasts → AppShell::workspace() (KORRIGIERT)
    ↓
Subphase 5: multi_workspace.rs entfernen (unverändert)
```

## Regeln nach dem Cleanup

1. **Window-Hierarchie:** `Inazuma (PlatformWindow) → AppShell (raijin-shell) → Workspace (raijin-workspace)`
2. **Workspace darf KEINEN `cfg!(target_os)` haben.** Alles platform-spezifische geht in AppShell oder Inazuma.
3. **raijin-ui Komponenten dürfen AppShell NICHT direkt aufrufen.** Actions dispatchen oder Globals nutzen.
4. **WindowExt Trait** ist in raijin-ui definiert, **Implementation** ist in raijin-shell.
5. **FocusedInputTracker** ist ein eigenständiger Global in raijin-ui, nicht Teil von AppShell.
