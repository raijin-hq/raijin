# Phase 20: Workspace Integration — Workspace als Raijin-Fundament

## Ziel

Das Workspace-System (`raijin-workspace`) als Fundament übernehmen, unser Terminal-Design als primäre Oberfläche drüberlegen. Editor/Panels bleiben angeschlossen aber initial nicht sichtbar. Das Ergebnis sieht aus wie Raijin (Warp-Style Terminal), nutzt aber unter der Haube die Workspace-Architektur.

## Warum

Das Workspace-System gibt uns gratis:

- **Pane-System** — Split-Panes, Tab-Navigation, Keyboard-Shortcuts
- **Dock-System** — Left/Right/Bottom Panels, drag-to-resize
- **Item Trait** — Jedes "Tab" ist ein Item (Terminal, Editor, Settings, etc.)
- **Modal Layer** — Command Palette, Picker, Dialoge
- **Toast/Notification Layer** — Non-blocking Notifications
- **Persistence** — Workspace-State speichern/wiederherstellen (via `SerializableItem` Trait + SQLite)
- **Keyboard-Navigation** — Focus-Management zwischen Panes
- **Zoom** — Jedes Panel kann fullscreen gezoomt werden

All das selber bauen wäre Monate Arbeit und würde schlechter sein.

## Architektur-Konzept

### Raijin's Ziel-Layout (Warp-Style)

```
┌──●●●──┬──[Tab1]──┬──[*Tab2]──┬──[Tab3]──┬──+──┬─────────[Actions]──┐
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Terminal Output (Block-System)                                     │
│  ┌─ Block 1: $ git status ──────────────────────── 0.2s ── ✓ ────┐ │
│  │ On branch main                                                 │ │
│  │ nothing to commit                                              │ │
│  └────────────────────────────────────────────────────────────────┘ │
│  ┌─ Block 2: $ cargo build ─────────────────────── 12.3s ── ✓ ──┐ │
│  │ Compiling raijin v0.1.0                                       │ │
│  │ ...                                                           │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│ 🔌 Context Chips │ Input Bar                                 [⏎] │
└─────────────────────────────────────────────────────────────────────┘
```

**TitleBar = Tab Bar** — Eine einzige Zeile wie bei Warp: Traffic Lights links, Terminal-Tabs in der Mitte, Aktions-Buttons rechts. Kein separates Element. Die Pane-interne Tab-Bar wird per `TabBarSettings { show: false }` unterdrückt.

**StatusBar** — Initial ausgeblendet per `StatusBarSettings { show: false }`. Die StatusBar wird für Editor-spezifische Infos genutzt (Cursor-Position, Sprache, Encoding, LSP-Status, Diagnostics, Vim-Mode, Zeilenenden) — alles davon initial irrelevant für ein Terminal. Kann später aktiviert werden wenn Editor-Features kommen.

### Wie das zusammenpasst

**Raijin's Terminal-View = ein Workspace Item.** Genau wie der Editor ein Item ist, wird unser Terminal-View (`RaijinTerminalPane`) ein Item im Workspace-Pane-System.

```
Workspace (Architektur)
├── TitleBar → RaijinTitleBar (View, wraps PlatformTitleBar Entity — Traffic Lights + Tabs + Actions)
├── Left Dock → hidden (initial), später: File Tree
├── Center Pane
│   └── Item: RaijinTerminalPane ← UNSER TERMINAL
│       ├── Terminal Output (Block-System mit Block-Headers)
│       ├── Correction Banner (optional)
│       ├── History Panel (optional)
│       └── Input Area (Context Chips + Input)
├── Right Dock → hidden (initial), später: AI Chat
├── Bottom Dock → hidden (initial), später: Completions/Debug
├── StatusBar → hidden (initial), später: Editor-Infos
├── Modal Layer → Command Palette, Settings, Shell Install (kommt von Workspace automatisch)
└── Toast Layer → Notifications (kommt von Workspace automatisch)
```

## Voraussetzungen

### Phase 19 (Settings System) muss zuerst fertig sein

`raijin-workspace` erfordert `WorkspaceSettings::get_global(cx)` via `inazuma-settings-framework`. Die aktuelle App nutzt ein einfacheres `RaijinSettings` + `impl Global` Pattern. Phase 19 migriert auf den `SettingsStore` mit TOML-Schemas — ohne das kompiliert `raijin-workspace` nicht.

**Zusätzlich muss Phase 19 diese Defaults in der TOML-Default-Datei setzen:**

```toml
[tab_bar]
show = false

[status_bar]
show = false
```

Das sorgt dafür, dass Raijin's Default-Konfiguration die Pane-interne Tab-Bar und die StatusBar automatisch ausblendet. Die Settings werden über `SettingsContent` → `from_settings()` geladen — die Defaults müssen dort korrekt hinterlegt sein.

### Entity\<Project\> + Arc\<AppState\> bereitstellen

`Workspace::new()` hat diese Signatur:

```rust
pub fn new(
    workspace_id: Option<WorkspaceId>,
    project: Entity<Project>,
    app_state: Arc<AppState>,
    window: &mut Window,
    cx: &mut Context<Self>,
) -> Self
```

Wir behalten die volle Architektur — kein Abspecken, keine Feature-Gates. Stattdessen instanziieren wir alles normal, aber ohne aktiven Collaboration-Server und ohne Node Runtime. Das ist genau das Pattern das die Referenz in ihren Tests nutzt.

**Konkrete Init-Kette:**

1. **Client erstellen** (ohne Server-Verbindung):
   ```rust
   let clock = Arc::new(inazuma_clock::RealSystemClock);
   let http = Arc::new(HttpClientWithUrl::new_url(
       cx.http_client(),
       &ClientSettings::get_global(cx).server_url,
       cx.http_client().proxy().cloned(),
   ));
   let client = Client::new(clock, http, cx);
   ```

2. **UserStore + WorkspaceStore erstellen** (brauchen Client):
   ```rust
   let user_store = cx.new(|cx| UserStore::new(client.clone(), cx));
   let workspace_store = cx.new(|cx| WorkspaceStore::new(client.clone(), cx));
   ```

3. **Project erstellen** (volle Initialisierung, aber ohne aktive Verbindungen):
   ```rust
   let languages = Arc::new(LanguageRegistry::new(/* ... */));
   let fs = Arc::new(RealFs::new(/* ... */));
   let project = Project::local(
       client.clone(),
       NodeRuntime::unavailable(),  // Kein Node nötig für Terminal
       user_store.clone(),
       languages.clone(),
       fs.clone(),
       None,                        // env
       LocalProjectFlags::default(),
       cx,
   );
   ```

4. **AppState zusammenbauen** (wie Referenz-Test-Setup, `workspace.rs:7139`):
   ```rust
   let app_state = Arc::new(AppState {
       languages: project.read(cx).languages().clone(),
       workspace_store,
       client,
       user_store,
       fs: project.read(cx).fs().clone(),
       build_window_options: |_, _| Default::default(),
       node_runtime: NodeRuntime::unavailable(),
       session,
   });
   ```

`Project::local()` erstellt intern alle Subsysteme (WorktreeStore, BufferStore, LspStore, DapStore, GitStore, etc.) — die existieren alle, sind aber ohne aktive Verbindungen harmlos. Später wenn Editor-Features dazukommen, sind sie bereits da.

## Phasen

### Phase 1: Terminal als Workspace Item

**Ziel:** `RaijinTerminalPane` implementiert den `Item` Trait aus `raijin-workspace`.

**Aufgaben:**

1. `RaijinTerminalPane` in `raijin-app` erstellen mit **allen** aktuell benötigten Feldern:

   ```rust
   pub struct RaijinTerminalPane {
       // Terminal-Kern
       terminal: Terminal,                    // Direkt, KEIN Entity — wie aktuell
       terminal_title: String,
       focus_handle: FocusHandle,

       // Input-System
       input_state: Entity<InputState>,
       shell_completion: Rc<ShellCompletionProvider>,
       command_history: Arc<RwLock<CommandHistory>>,
       history_panel: HistoryPanel,
       correction_suggestion: Option<CorrectionResult>,

       // Shell-Kontext
       shell_context: ShellContext,           // Direkt, KEIN Entity — wie aktuell
       shell_name: String,
       available_shells: Vec<ShellOption>,
       pending_shell_install: Option<&'static ShellInstallInfo>,

       // Rendering
       block_list: Entity<BlockListView>,     // NICHT BlockManager — BlockListView ist die UI-Komponente
       cached_bg_image: Option<(PathBuf, Arc<RenderImage>)>,

       // UI-State
       interactive_mode: bool,
       show_terminal: bool,
       last_terminal_rows: u16,
       last_terminal_cols: u16,

       // Workspace-Referenz (für Modal-Zugriff)
       workspace: Option<WeakEntity<Workspace>>,
   }
   ```

   **Hinweis:** `view_mode: ViewMode` entfällt — Settings wird ein eigenes Workspace-Item, nicht ein ViewMode-Switch innerhalb des Terminals. `modal_layer` entfällt ebenfalls — der Workspace hat einen eigenen Modal-Layer mit public API (`workspace.toggle_modal()`, `workspace.hide_modal()`, etc.). Items öffnen Modals über den Workspace, nicht über einen eigenen Layer.

2. `impl Item for RaijinTerminalPane`:

   Der Item-Trait erfordert als Supertraits: `Focusable + EventEmitter<Self::Event> + Render + Sized` und einen Associated Type `type Event`.

   **Eigenen Event-Typ definieren** (NICHT direkt ItemEvent verwenden):

   ```rust
   #[derive(Clone, Debug)]
   pub enum TerminalPaneEvent {
       TitleChanged,
       CloseRequested,
       BellRang,
   }

   impl EventEmitter<TerminalPaneEvent> for RaijinTerminalPane {}
   ```

   **Item-Impl mit Associated Type:**

   ```rust
   impl Item for RaijinTerminalPane {
       type Event = TerminalPaneEvent;

       // ... alle Methoden
   }
   ```

   **Event-Mapping via `to_item_events()`:**

   ```rust
   fn to_item_events(event: &TerminalPaneEvent, f: &mut dyn FnMut(ItemEvent)) {
       match event {
           TerminalPaneEvent::TitleChanged => f(ItemEvent::UpdateTab),
           TerminalPaneEvent::CloseRequested => f(ItemEvent::CloseItem),
           TerminalPaneEvent::BellRang => {} // kein ItemEvent nötig
       }
   }
   ```

   **Pflichtmethode** (einzige ohne Default):

   ```rust
   fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
       // z.B. "zsh — ~/Projects/raijin"
       format!("{} — {}", self.shell_name, self.shell_context.cwd_short()).into()
   }
   ```

   **Relevante Overrides:**

   ```rust
   fn tab_content(&self, params: TabContentParams, _window: &Window, cx: &App) -> AnyElement {
       // Terminal-Tab mit Shell-Icon + Name + CWD
   }

   fn tab_tooltip_text(&self, _cx: &App) -> Option<SharedString> {
       Some(self.shell_context.cwd.clone().into()) // Voller Pfad
   }

   fn can_split(&self) -> bool {
       true // Multi-Terminal erlauben
   }

   fn clone_on_split(
       &self,
       _workspace_id: Option<WorkspaceId>,
       window: &mut Window,
       cx: &mut Context<Self>,
   ) -> Task<Option<Entity<Self>>> {
       // Async PTY spawnen:
       // 1. Neue PTY mit gleicher Shell + gleichem CWD
       // 2. Shell-Hooks injizieren (ZDOTDIR für Zsh, --rcfile für Bash)
       // 3. OSC-Parser aufsetzen
       // 4. Neues RaijinTerminalPane konstruieren
       // 5. Bei Fehler: None zurückgeben (Pane-Split wird abgebrochen)
       let shell = self.shell_name.clone();
       let cwd = self.shell_context.cwd.clone();
       // cx.spawn_in closure bekommt WeakEntity<Self> als ersten Parameter:
       cx.spawn_in(window, async move |_this: WeakEntity<Self>, mut cx| {
           // PTY spawnen...
           // Auf Fehler: return None
           todo!("PTY spawn + Terminal-Konstruktion")
       })
   }

   fn is_dirty(&self, _cx: &App) -> bool {
       // true wenn ein Prozess läuft (nicht nur Shell-Prompt)
       let term = self.terminal.handle().lock();
       term.block_router().has_active_block()
   }

   fn can_save(&self, _cx: &App) -> bool {
       false // Terminal-State wird nicht als File gespeichert
   }

   // Workspace-Referenz merken für Modal-Zugriff:
   fn added_to_workspace(&mut self, workspace: &mut Workspace, _window: &mut Window, cx: &mut Context<Self>) {
       self.workspace = Some(cx.entity().downgrade());
   }
   ```

   **Alle anderen 39 Methoden** (von insgesamt 40) haben sinnvolle Defaults (None, false, empty) und brauchen kein Override.

3. `impl Render for RaijinTerminalPane`:

   Der bestehende Render-Code aus `workspace.rs` wandert hierhin. Jeder Terminal-Tab rendert seinen eigenen kompletten Stack:

   ```rust
   fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
       // PTY-Resize-Logik (wie aktuell Zeile 1109-1137)
       // ...

       let mut container = div().flex().flex_col().size_full().relative();

       // Background Image Layer (wie aktuell Zeile 1065-1096)
       // ...

       // Terminal Output
       if self.show_terminal || self.interactive_mode {
           container = container.child(self.block_list.clone());
       } else {
           container = container.child(div().flex_1().min_h_0());
       }

       if !self.interactive_mode {
           let command_running = {
               let term = self.terminal.handle().lock();
               term.block_router().has_active_block()
           };

           // Correction Banner
           if let Some(ref correction) = self.correction_suggestion {
               container = container.child(self.render_correction_banner(correction));
           }

           // Input Area (nur wenn kein Command läuft)
           if !command_running {
               if self.history_panel.is_visible() {
                   container = container.child(self.history_panel.render());
               }
               container = container.child(self.render_input_area());
           }
       }

       // KEIN TitleBar hier — der kommt vom Workspace
       // KEIN Modal-Layer hier — der kommt vom Workspace
       // Shell-Install-Modals nutzen den Workspace-Modal-Layer

       container
   }
   ```

   **Wichtig:** TitleBar und ModalLayer werden NICHT mehr innerhalb des Items gerendert. Der Workspace rendert TitleBar oberhalb und ModalLayer als Overlay über allem.

   **Modal-Zugriff aus dem Item:** Shell-Install-Modals werden über Action-Dispatch geöffnet:

   ```rust
   // Im Terminal-Item bei Shell-Install:
   cx.dispatch_action(Box::new(ShowShellInstallModal { shell: self.shell_name.clone() }));

   // Im Workspace registriert:
   workspace.register_action(|workspace, action: &ShowShellInstallModal, window, cx| {
       workspace.toggle_modal(window, cx, |window, cx| {
           ShellInstallModal::new(action.shell.clone(), window, cx)
       });
   });
   ```

4. `impl Focusable for RaijinTerminalPane`:

   ```rust
   fn focus_handle(&self, _cx: &App) -> FocusHandle {
       self.focus_handle.clone()
   }
   ```

**Referenz:** Terminal-Item in `.reference/zed/crates/terminal_view/src/terminal_panel.rs`

### Phase 2: TitleBar als kombinierte Tab Bar

**Ziel:** Die TitleBar wird zu unserer Warp-Style TitleBar/TabBar.

**Architektur:** Exakt wie die Referenz es macht — die TitleBar ist ein **View** (`impl Render`), kein Element. Sie wraps `PlatformTitleBar` (aus dem `platform_title_bar` Crate) per Komposition. `PlatformTitleBar` bringt Traffic Lights, Window-Drag, und Platform-Abstraction mit. RaijinTitleBar packt ihren Content (Tabs, Buttons) als Children rein.

**Referenz:** TitleBar in `.reference/zed/crates/title_bar/src/title_bar.rs` — gleiche Architektur, nur mit Project/Branch statt Terminal-Tabs.

**Aufgaben:**

1. `RaijinTitleBar` als View erstellen:

   ```rust
   pub struct RaijinTitleBar {
       platform_titlebar: Entity<PlatformTitleBar>,
       workspace: WeakEntity<Workspace>,
       _subscriptions: Vec<Subscription>,
   }
   ```

2. **Subscription-Pattern** (wie die Referenz, Pull-basiert):

   ```rust
   impl RaijinTitleBar {
       pub fn new(
           id: impl Into<ElementId>,
           workspace: &Workspace,
           window: &mut Window,
           cx: &mut Context<Self>,
       ) -> Self {
           let workspace_handle = workspace.weak_handle();

           // PlatformTitleBar erstellen (bringt Traffic Lights + Drag mit):
           let platform_titlebar = cx.new(|cx| PlatformTitleBar::new(id, cx));

           let subscriptions = vec![
               // Workspace observieren — re-rendert bei JEDER Workspace-Änderung:
               cx.observe(&workspace_handle.upgrade().unwrap(), |_this, _workspace, cx| {
                   cx.notify();
               }),
           ];

           Self {
               platform_titlebar,
               workspace: workspace_handle,
               _subscriptions: subscriptions,
           }
       }
   }
   ```

3. **Render — Pull-basiertes Modell** (TitleBar liest aktuellen State in render()):

   ```rust
   impl Render for RaijinTitleBar {
       fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
           let workspace = self.workspace.upgrade().unwrap();
           let workspace = workspace.read(cx);
           let panes = workspace.panes();
           let active_pane = workspace.active_pane();

           // PlatformTitleBar wrappen und Content als Children setzen:
           self.platform_titlebar.update(cx, |titlebar, _cx| {
               titlebar.set_children(vec![
                   self.render_tabs(panes, active_pane, cx).into_any_element(),
                   self.render_actions(cx).into_any_element(),
               ]);
           });

           // Eine Zeile:
           // Links: Traffic Lights (macOS, von PlatformTitleBar) + Window-Drag-Region
           // Mitte: Terminal-Tabs (jeder Tab = ein Terminal-Pane/Item)
           // Rechts: Aktions-Buttons (New Tab +, Settings ⚙, etc.)
           self.platform_titlebar.clone()
       }
   }
   ```

   Workspace emittiert relevante Events: `PaneAdded`, `PaneRemoved`, `ActiveItemChanged`, `ItemAdded`, `ItemRemoved`. Pane emittiert: `ActivateItem`, `ChangeItemTitle`, `Remove`, `Focus`, `Split`. Das `cx.observe()` auf den Workspace fängt all das ab und triggert Re-Render.

4. **Registrierung** via `cx.observe_new()` in einer `init()` Funktion (wie die Referenz):

   ```rust
   pub fn init(cx: &mut App) {
       PlatformTitleBar::init(cx);

       cx.observe_new(|workspace: &mut Workspace, window, cx| {
           let Some(window) = window else { return };
           let item = cx.new(|cx| RaijinTitleBar::new("raijin-title-bar", workspace, window, cx));
           workspace.set_titlebar_item(item.into(), window, cx);
       });
   }
   ```

   `set_titlebar_item()` nimmt `AnyView` — daher `.into()` für die Konversion.

5. **Pane-interne Tab-Bar unterdrücken** — Passiert automatisch über die Default-Settings aus Phase 19:

   ```toml
   [tab_bar]
   show = false
   ```

   `TabBarSettings` hat ein `show: bool` Feld (`workspace_settings.rs:61`). Die `RaijinTitleBar` übernimmt die Tab-Darstellung komplett.

### Phase 3: Docks und StatusBar initial verstecken

**Ziel:** Docks und StatusBar sind da aber nicht sichtbar. Können später aktiviert werden.

**Aufgaben:**

1. Alle Docks schließen (korrekte Signatur — `window` Parameter ist Pflicht bei `set_open`):

   ```rust
   workspace.left_dock().update(cx, |dock, cx| dock.set_open(false, window, cx));
   workspace.right_dock().update(cx, |dock, cx| dock.set_open(false, window, cx));
   workspace.bottom_dock().update(cx, |dock, cx| dock.set_open(false, window, cx));
   ```

2. **StatusBar ausblenden** — Passiert automatisch über die Default-Settings aus Phase 19:

   ```toml
   [status_bar]
   show = false
   ```

   `StatusBarSettings` hat ein `show: bool` Feld (`workspace_settings.rs:134`, Default `true`). Raijin's Default-Konfiguration setzt es auf `false`.

3. Keyboard-Shortcuts registrieren zum Toggling:
   - `Cmd+B` → Left Dock toggle (File Tree, wenn implementiert)
   - `Cmd+J` → Bottom Dock toggle (Completions/Debug)

4. Dock-Panels als leere Placeholder registrieren, die später gefüllt werden.

### Phase 4: raijin-app/src/workspace.rs migrieren

**Ziel:** Unseren bestehenden Workspace-Code in das Item-System überführen.

**Was migriert wird:**

| Aktuell in `workspace.rs` | Ziel in Phase 20 |
|---|---|
| `Workspace` struct (20+ Felder) | `RaijinTerminalPane` (alle Felder übernommen) |
| `Workspace::render()` (Terminal-Branch) | `RaijinTerminalPane::render()` |
| `render_title_bar()` | → `RaijinTitleBar` (eigenes View, wraps PlatformTitleBar) |
| `render_input_area()` | → `RaijinTerminalPane::render_input_area()` (bleibt im Item) |
| `render_correction_banner()` | → `RaijinTerminalPane::render_correction_banner()` (bleibt im Item) |
| `render_shell_selector_chip()` | → `RaijinTerminalPane` oder `RaijinTitleBar` |
| Input-Bar Logik | → Teil von `RaijinTerminalPane::render()` — jeder Tab hat eigene Input-Bar |
| Block-System | → bleibt in `raijin-terminal`, `BlockListView` bleibt als Entity im Item |
| Shell-Detection | → bleibt in `raijin-shell`, wird beim PTY-Spawn aufgerufen |
| `ViewMode::Settings` | → Eigenes Workspace-Item (`RaijinSettingsPane`), kein ViewMode mehr |
| Shell Install Modal | → Workspace's Modal-Layer nutzen via Action-Dispatch |
| Completion-System | → Bleibt als Popup relativ zum Input-Cursor im Item (KEIN Dock-Panel) |
| `handle_terminal_event()` | → `RaijinTerminalPane::handle_terminal_event()` |
| `on_input_event()` | → `RaijinTerminalPane::on_input_event()` |
| PTY Resize Logik | → `RaijinTerminalPane::render()` (wie aktuell) |
| Background Image Cache | → `RaijinTerminalPane` Feld (pro Tab eigenes Hintergrundbild) |

**Was wegfällt:**

- Eigenes Window-Management (Workspace macht das)
- Eigenes Focus-Management (Workspace macht das)
- Eigenes Tab-Management / `ViewMode` enum (Pane-System macht Tabs, Settings wird eigenes Item)
- `PendingShellSwitch` / `PendingShellInstallName` Globals (können lokaler gelöst werden)
- Eigener `modal_layer` (Workspace hat einen mit public API)

**Was NICHT wegfällt:**

- `interactive_mode` / `show_terminal` — Terminal-spezifischer State pro Tab
- `last_terminal_rows` / `last_terminal_cols` — PTY-Resize-Tracking pro Tab
- `command_history` — Shared via `Arc<RwLock<>>` über alle Terminal-Tabs
- `history_panel` — Pro Tab eigene History-Ansicht

### Phase 5: Multi-Terminal Support

**Ziel:** Mehrere Terminals als Tabs und Split-Panes.

**Aufgaben:**

1. `Cmd+T` → Neues Terminal-Item im aktiven Pane:

   ```rust
   // Im Workspace-Action-Handler:
   workspace.register_action(|workspace, _: &NewTerminal, window, cx| {
       let terminal_pane = cx.new(|cx| RaijinTerminalPane::new(window, cx));
       workspace.add_item_to_active_pane(
           Box::new(terminal_pane),
           None,       // destination_index: am Ende
           true,       // focus_item
           window,
           cx,
       );
   });
   ```

   `add_item_to_active_pane()` ist die korrekte API — nimmt `Box<dyn ItemHandle>`, optional `destination_index`, und `focus_item` Flag.

2. `Cmd+D` → Split horizontal (neuer Pane rechts mit Terminal):

   Nutzt die **public Workspace-API** `split_and_clone()` — diese ruft intern `clone_on_split()` auf unserem Item auf und macht den Pane-Split automatisch:

   ```rust
   workspace.register_action(|workspace, _: &SplitTerminalRight, window, cx| {
       let pane = workspace.active_pane().clone();
       workspace.split_and_clone(pane, SplitDirection::Right, window, cx).detach();
   });
   ```

   **Verfügbare Split-APIs auf Workspace** (alle public):
   - `split_pane(pane, direction, window, cx) -> Entity<Pane>` — Neuen leeren Pane abspalten
   - `split_and_clone(pane, direction, window, cx) -> Task<Option<Entity<Pane>>>` — Pane splitten + aktives Item klonen (async, nutzt `clone_on_split()`)
   - `split_and_move(pane, direction, window, cx)` — Pane splitten + aktives Item in neuen Pane moven
   - `split_item(direction, item, window, cx)` — Item direkt in neuen Split-Pane packen

   `SplitDirection` hat `Up`, `Down`, `Left`, `Right` Varianten.

3. `Cmd+Shift+D` → Split vertikal (neuer Pane unten mit Terminal):

   ```rust
   workspace.register_action(|workspace, _: &SplitTerminalDown, window, cx| {
       let pane = workspace.active_pane().clone();
       workspace.split_and_clone(pane, SplitDirection::Down, window, cx).detach();
   });
   ```

4. `Cmd+W` → Terminal-Tab schließen:
   - `is_dirty()` gibt `true` zurück wenn Prozess läuft → Workspace zeigt automatisch "Unsaved changes" Dialog
   - Alternativ: eigene Bestätigungs-Logik über den Event-Lifecycle

5. Tab-Rearrangement per Drag & Drop — kommt vom Pane-System gratis.

6. `clone_on_split()` Implementierung ist der kritische Teil:
   - Async PTY-Spawn mit gleicher Shell + gleichem CWD
   - Shell-Hook-Injection (ZDOTDIR für Zsh, --rcfile für Bash, etc.)
   - OSC-Parser aufsetzen für den neuen PTY-Stream
   - Neuen `BlockListView` Entity erstellen
   - Neue `InputState` Entity erstellen
   - `ShellCompletionProvider` kann geshared oder neu erstellt werden
   - Bei Fehler: `None` zurückgeben, Split wird abgebrochen

### Phase 6: Persistence (optional, kann nachgelagert werden)

**Ziel:** Terminal-Layout überlebt Neustarts.

**Aufgaben:**

1. `impl SerializableItem for RaijinTerminalPane`:

   Alle 5 Methoden sind Pflicht (keine Defaults). Vollständige Signaturen:

   ```rust
   impl SerializableItem for RaijinTerminalPane {
       fn serialized_item_kind() -> &'static str {
           "RaijinTerminal"
       }

       fn serialize(
           &mut self,
           workspace: &mut Workspace,
           item_id: ItemId,
           closing: bool,
           window: &mut Window,
           cx: &mut Context<Self>,
       ) -> Option<Task<Result<()>>> {
           // Speichern: Shell-Name, CWD, Pane-Position
           // item_id und closing für DB-Logik nutzen
       }

       fn deserialize(
           project: Entity<Project>,
           workspace: WeakEntity<Workspace>,
           workspace_id: WorkspaceId,
           item_id: ItemId,
           window: &mut Window,
           cx: &mut App,
       ) -> Task<Result<Entity<Self>>> {
           // Wiederherstellen: PTY mit gespeicherter Shell + CWD spawnen
       }

       fn should_serialize(&self, event: &TerminalPaneEvent) -> bool {
           // Bei TitleChanged (CWD-Wechsel) serialisieren
           matches!(event, TerminalPaneEvent::TitleChanged)
       }

       fn cleanup(
           workspace_id: WorkspaceId,
           alive_items: Vec<ItemId>,
           window: &mut Window,
           cx: &mut App,
       ) -> Task<Result<()>> {
           // Alte Einträge aus DB entfernen die nicht mehr in alive_items sind
       }
   }
   ```

2. Persistence nutzt `raijin-db` / `raijin-sqlez` (SQLite) — Schema für Terminal-Items definieren.

## Was unser Terminal-Item rendert

```
RaijinTerminalPane::render()
├── Background Image Layer (optional, absolute, opacity-gesteuert)
├── Terminal Output Area (flex-1, scrollbar)
│   ├── BlockListView (Entity)
│   │   ├── Block Headers (Command + Duration + Exit Badge)
│   │   ├── Terminal Grid (per-cell rendering wie bisher)
│   │   └── Fold Indicators (collapsed blocks)
├── Correction Banner (optional, wenn Korrektur-Vorschlag vorhanden)
├── History Panel Overlay (optional, wenn History-Suche aktiv)
└── Input Area (fixed bottom, flex_shrink_0)
    ├── Context Chips (User, Hostname, CWD, Time, Shell, Git Branch, Git Stats)
    ├── Input Field (InputState Entity, mit Completion-Popup als Overlay)
    └── Submit-Hint

NICHT im Item (kommt vom Workspace):
├── TitleBar (RaijinTitleBar View, wraps PlatformTitleBar)
├── Modal Layer (globale Modals — Command Palette, Shell Install via Action-Dispatch)
├── Toast Layer (Notifications)
├── StatusBar (initial hidden via Settings)
└── Docks (initial hidden)
```

## Abhängigkeiten

- **Phase 19 (Settings)** muss zuerst fertig sein — `WorkspaceSettings`, `TabBarSettings`, `StatusBarSettings` nutzen alle `inazuma-settings-framework` und `get_global(cx)`. Phase 19 muss auch die Raijin-Defaults für `tab_bar.show = false` und `status_bar.show = false` in der Default-TOML-Datei setzen.
- **Entity\<Project\> + Arc\<AppState\>** — Volle Initialisierung wie die Referenz es macht, mit `Client::new()`, `NodeRuntime::unavailable()`, und `Project::local()`. Kein Abspecken.
- **Item Trait** erfordert `Focusable + EventEmitter<Self::Event> + Render + Sized` plus Associated Type `type Event`

## Fehlende Crates — Porting von der Referenz

Diese Referenz-Crates existieren in `.reference/zed/crates/` und müssen nach Raijin geportet werden. Alle sind workspace-relevant. Porting-Prinzip: vollständig übernehmen, `gpui` → `inazuma`, `ui` → `raijin-ui`, `zed_actions` → `raijin-actions`, Inhalt auf Terminal-Kontext anpassen wo nötig.

### Tier 1 — Workspace-Infrastruktur (MUSS vor Phase 1-2)

| Neues Crate | Referenz-Quelle | Zeilen | Was es tut |
|---|---|---|---|
| `raijin-panel` | `panel` | ~75 | `PanelHeader` + `PanelTabs` Traits (erweitern `workspace::Panel`), Helper-Buttons. Deps: `inazuma`, `raijin-ui`, `raijin-workspace` |
| `raijin-platform-title-bar` | `platform_title_bar` | ~1.186 | Cross-platform TitleBar **View** (`impl Render`). `SystemWindowTabs` (Multi-Window Tab Drag/Drop), `PlatformStyle` (Mac/Linux/Windows), Linux/Windows Window-Controls. Deps: `inazuma`, `raijin-ui`, `raijin-workspace`, `raijin-settings-framework`, `raijin-feature-flags` |
| `raijin-title-bar` | `title_bar` | ~1.267 | App-Level TitleBar View der `PlatformTitleBar` Entity wraps. Zeigt Project-Info, Git-Branch, User-Menu, Update-Notifications. Raijin-Anpassung: statt Collab/Branch → Shell-Context (CWD, Git-Branch, User@Host). Deps: `raijin-platform-title-bar`, `raijin-workspace`, `raijin-project`, `raijin-client`, `raijin-settings-framework`, `raijin-theme` |

### Tier 2 — Terminal als Workspace-Item (KERN)

| Neues Crate | Referenz-Quelle | Zeilen | Was es tut |
|---|---|---|---|
| `raijin-terminal-view` | `terminal_view` | ~7.034 | `TerminalView` (`impl Item`, `impl SerializableItem`, `impl SearchableItem`), `TerminalElement` (GPUI Element für Grid-Rendering), `TerminalPanel` (`impl Panel` für Dock), `TerminalScrollbar` (ScrollableHandle), Persistence (SQLite Schema), `terminal_path_like_target` (Hover/Click auf Dateipfade). **Raijin hat bereits** `raijin-app/src/terminal/` mit eigenem Grid/Block-Rendering — dieses muss in die Item-Architektur eingebettet werden, kein blindes Kopieren. Deps: `raijin-workspace`, `raijin-terminal`, `raijin-project`, `raijin-editor`, `raijin-task`, `raijin-db`, `raijin-ui` |

### Tier 3 — Essenzielle Modals & Navigation

| Neues Crate | Referenz-Quelle | Zeilen | Was es tut |
|---|---|---|---|
| `raijin-command-palette-hooks` | `command_palette_hooks` | ~153 | Global Filtering/Interception für Command Palette. `CommandPaletteFilter`, `GlobalCommandPaletteInterceptor`. Muss VOR `raijin-command-palette` existieren. Deps: `inazuma`, `raijin-workspace`, `inazuma-collections` |
| `raijin-command-palette` | `command_palette` | ~1.223 | Fuzzy-Searchable Action-Palette Modal. `CommandPalette` (`impl ModalView`), `CommandPaletteDelegate` (`impl PickerDelegate`). Deps: `raijin-command-palette-hooks`, `inazuma-picker`, `inazuma-fuzzy`, `raijin-workspace`, `raijin-settings-framework` |
| `raijin-tab-switcher` | `tab_switcher` | ~883 | Ctrl+Tab Modal zum schnellen Tab-Wechsel. `TabSwitcher` (`impl ModalView`), fuzzy search, tab closing. Deps: `inazuma-picker`, `raijin-workspace`, `raijin-project`, `raijin-editor`, `inazuma-fuzzy` |
| `raijin-file-finder` | `file_finder` | ~2.039 | Cmd+P File-Finder Modal. Fuzzy-Suche über Workspace-Files, Recent History, Split-Direction. Deps: `inazuma-picker`, `raijin-project`, `raijin-workspace`, `raijin-editor`, `inazuma-fuzzy`, `raijin-file-icons` |
| `raijin-recent-projects` | `recent_projects` | ~2.179 | Recent Projects Modal/Popover. Liest Workspace-History aus DB. Deps: `inazuma-picker`, `raijin-workspace`, `raijin-project`, `inazuma-fuzzy`, `raijin-remote` |

### Tier 4 — Panels & Sidebar

| Neues Crate | Referenz-Quelle | Zeilen | Was es tut |
|---|---|---|---|
| `raijin-sidebar` | `sidebar` | ~7.151 | Sidebar-Architektur mit Thread/Session-Management. **Stark Agent-spezifisch** — Architektur porten, Inhalt komplett auf Raijin anpassen (statt Agent-Threads → Terminal-Sessions). Deps: `raijin-workspace`, `raijin-project`, `raijin-git`, `raijin-editor`, `raijin-theme`, `raijin-settings-framework` |
| `raijin-project-panel` | `project_panel` | groß | File-Tree Panel (linke Sidebar). `impl Panel`. Deps: `raijin-workspace`, `raijin-project`, `raijin-editor`, `raijin-git`, `raijin-file-icons`, `raijin-settings-framework` |
| `raijin-outline` | `outline` | ~1.110 | Code-Outline Modal. Symbol-Navigation im Editor-Buffer. Deps: `inazuma-picker`, `raijin-editor`, `raijin-language`, `raijin-workspace`, `inazuma-fuzzy` |
| `raijin-outline-panel` | `outline_panel` | groß | Persistent Outline Panel als Dock-Item. Deps: `raijin-workspace`, `raijin-outline`, `raijin-editor`, `raijin-settings-framework` |
| `raijin-search` | `search` | mehrteilig | Buffer-Search + Project-Search mit Replace. `SearchBar`, `ProjectSearch`. Deps: `raijin-workspace`, `raijin-editor`, `raijin-project`, `inazuma-fuzzy` |
| `raijin-diagnostics` | `diagnostics` | mehrteilig | Error/Warning Panel. `impl Item` für Diagnostic-Ansicht. Deps: `raijin-workspace`, `raijin-editor`, `raijin-language`, `raijin-project` |

### Tier 5 — Kleinere Features

| Neues Crate | Referenz-Quelle | Zeilen | Was es tut |
|---|---|---|---|
| `raijin-which-key` | `which_key` | ~94 | Vi-style Key-Hint Modal. Zeigt Pending-Keystrokes nach Delay. Trivial — fast 1:1 kopieren. Deps: `raijin-workspace`, `raijin-settings-framework`, `inazuma` |

### Bereits vorhanden (kein Porting nötig)

| Raijin Crate | Referenz-Äquivalent | Status |
|---|---|---|
| `raijin-workspace/notifications.rs` + `toast_layer.rs` | `notifications` | ✅ Voll implementiert |
| `inazuma-picker` | `picker` | ✅ Voll implementiert |
| `raijin-workspace/dock.rs` (Panel Trait) | `workspace::Panel` | ✅ Basis-Trait vorhanden |
| `raijin-actions` (command_palette::Toggle) | `command_palette_hooks` (partial) | ⚠️ Action definiert, Implementierung fehlt |
| `inazuma-settings-content` (FileFinderSettingsContent) | `file_finder` Settings | ⚠️ Settings vorhanden, UI fehlt |

### Porting-Reihenfolge

```
Tier 1 (Infrastruktur):
  raijin-panel → raijin-platform-title-bar → raijin-title-bar

Tier 2 (Terminal-Item):
  raijin-terminal-view (parallel zu Phase 1-6 des Implementierungsplans)

Tier 3 (Modals, kann nach Phase 6):
  raijin-command-palette-hooks → raijin-command-palette
  raijin-tab-switcher
  raijin-file-finder
  raijin-recent-projects

Tier 4 (Panels, kann nach Tier 3):
  raijin-sidebar
  raijin-project-panel
  raijin-outline → raijin-outline-panel
  raijin-search
  raijin-diagnostics

Tier 5 (jederzeit):
  raijin-which-key
```

## Was die Referenz hat das wir später aktivieren

Diese Workspace-Features sind da, initial versteckt, können schrittweise aktiviert werden:

| Feature | Referenz Component | Raijin Nutzung | Wann |
|---|---|---|---|
| File Tree | Left Dock + Project Panel | Dateibrowser für CWD | Phase 21+ |
| Editor | Center Pane Item | Code-Editing direkt im Terminal | Phase 22+ |
| AI Chat | Right Dock | Agent Integration | Phase 23+ |
| Search | Bottom Dock | Grep/Find in Output | Phase 21+ |
| Breadcrumbs | Toolbar | CWD Path mit Klick-Navigation | Phase 21+ |
| Git Panel | Left Dock | Git Status/Diff Viewer | Phase 22+ |
| Debug Console | Bottom Dock | Command Output Inspection | Phase 22+ |
| StatusBar | Bottom Bar | Cursor-Position, Sprache, LSP | Phase 22+ (mit Editor) |

## Risiken

1. **Workspace.rs (`raijin-workspace`) ist ~9.500 Zeilen** — Vorsichtig anpassen, nicht alles auf einmal
2. **Item Trait hat 40 Methoden** — Aber nur 1 ist Pflicht (`tab_content_text`), die anderen 39 haben sinnvolle Defaults
3. **Focus-Chain** — Terminal braucht spezielles Keyboard-Handling (Raw-Mode vs. UI-Mode), muss sauber mit Workspace-Focus interagieren. `interactive_mode` steuert ob Keystrokes ans PTY oder an die UI gehen.
4. **Performance** — Terminal-Rendering darf nicht durch Workspace-Layout-Passes verlangsamt werden. `block_list` und Grid-Rendering sind bereits optimiert (Viewport-Culling, GPU-Primitives).
5. **Persistence** — `SerializableItem` ist optional aber nötig für Layout-Restore. Alle 5 Methoden sind Pflicht (keine Defaults). Braucht Schema: Shell-Name, CWD, Pane-Position.
6. **Entity\<Project\> Init-Kette** — `Project::local()` braucht `Client`, `NodeRuntime`, `UserStore`, `LanguageRegistry`, `Fs`. Wir instanziieren alles normal (volle Architektur), aber ohne aktive Server-Verbindung und mit `NodeRuntime::unavailable()`. Die internen Subsysteme (LspStore, DapStore, GitStore, etc.) existieren alle, sind aber ohne Verbindungen harmlos.
7. **clone_on_split() ist async** — PTY-Spawn kann fehlschlagen (Shell nicht gefunden, Permissions, etc.). Fehlerfall muss sauber gehandhabt werden. `cx.spawn_in()` Closure bekommt `WeakEntity<Self>` als ersten Parameter.
8. **Pane-Tab-Bar Dopplung** — Ohne `TabBarSettings { show: false }` zeigt jeder Pane seine eigene Tab-Leiste zusätzlich zur TitleBar. Muss in Phase 19 Default-Settings gesetzt werden.

## Erfolgs-Kriterien

Phase 20 ist fertig wenn:

- [ ] Raijin startet und zeigt ein Terminal im Workspace-Pane
- [ ] TitleBar zeigt Terminal-Tabs (Warp-Style, RaijinTitleBar wraps PlatformTitleBar)
- [ ] Pane-interne Tab-Bar ist ausgeblendet (`TabBarSettings.show = false`)
- [ ] StatusBar ist ausgeblendet (`StatusBarSettings.show = false`)
- [ ] `Cmd+T` öffnet neues Terminal (neuer Tab via `add_item_to_active_pane`)
- [ ] `Cmd+D` splittet Pane (zwei Terminals nebeneinander via `split_and_clone`)
- [ ] Docks sind da aber versteckt
- [ ] Unser Block-System, Input Bar, Context Chips, History Panel, Correction Banner funktionieren wie vorher
- [ ] Completion-Popup funktioniert als Overlay im Item (kein Dock-Panel)
- [ ] Shell-Detection und Shell-Install-Modal funktionieren über Workspace-Modal-Layer (via Action-Dispatch)
- [ ] Settings ist ein eigenes Workspace-Item (kein ViewMode-Switch mehr)
- [ ] Kein visueller Unterschied zum jetzigen Raijin (außer Multi-Tab/Split)
- [ ] `clone_on_split()` handhabt Fehler graceful (kein Crash bei PTY-Spawn-Failure)
