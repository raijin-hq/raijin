# Phase 29: Hybrid Terminal-Notebook — REPL Revolution

## Vision

Das erste Terminal der Welt das nahtlos zwischen Shell-Mode und Kernel-Mode wechselt. Shell-Commands laufen als normale Blocks (OSC 133). Sobald ein REPL erkannt wird (`python`, `node`, `irb`, `ghci`, etc.), wechselt Raijin in den Kernel-Mode: Jupyter-Kernel als Sidecar, strukturierter Input/Output über das Wire Protocol, Rich Output (Bilder, Tabellen, Charts) GPU-gerendert direkt in Blocks. Kein anderes Terminal kann das — Warp hat nur Text, VS Code braucht eine separate Notebook-UI, andere Editoren können es nur im Editor.

## Ausgangslage

### Was bereits existiert

**raijin-repl** (16k Zeilen, `inazuma::` imports, kompiliert):
- Vollständiger Jupyter Wire Protocol Client (ZMQ 5-Channel: Shell, IOPub, Control, Stdin, Heartbeat)
- Kernel Management: Local (NativeRunningKernel), Remote (WebSocket), SSH, WSL
- Session Lifecycle: start → execute → interrupt → restart → shutdown
- Output Types: Plain (ANSI), Image (PNG/JPEG), Table, Markdown, JSON, HTML, Plotly
- MIME-Type Ranking: DataTable(7) > HTML(6) > JSON(5) > PNG(4) > JPEG(3) > Markdown(2) > Plain(1)
- Notebook UI: CodeCell, MarkdownCell, RawCell, nbformat v4 Serialization
- KernelSession Trait: `route()` dispatcht JupyterMessages zu ExecutionView
- ReplStore: Global Kernel-Spec Discovery (`jupyter kernelspec list`, Python venv Detection)
- Editor-Integration: EditorBlock mit BlockPlacement::Below für Inline-Output

**raijin-terminal-view** (Block-Rendering Pipeline):
- `TerminalGridElement`: Per-Cell GPU Rendering (Backgrounds → Selections → Glyphs → Builtins)
- `BlockListView`: Scrollbare Block-Liste mit Single-Lock-Snapshots, Fold-System, Selection
- `BlockSnapshot` / `BlockGridSnapshot` / `SnapshotCell`: Immutable Render-Daten
- `GridOriginStore`: Shared Pixel-Position für exaktes Hit-Testing
- `render_block()` / `render_fold_line()`: Block-Header mit Metadata + Duration + Badges

**raijin-term** (Terminal Core):
- `BlockGridRouter`: Routet VTE Output zu per-Command BlockGrids
- `BlockGrid`: Eigenständige Grid + Cursor + Selection pro Command
- `Term<VoidListener>`: Virtueller Term OHNE PTY — perfekt für ANSI-Parsing von Kernel-Output
- `RenderableContent`: Iterator über Cells + Cursor + Colors + Selection

### Was gebrochen ist

`outputs/plain.rs:29` importiert `raijin_terminal_view::terminal_element::TerminalElement` — Dependency kommentiert in Cargo.toml. Der ANSI-Output-Renderer ist nicht funktional.

## Architektur

### Zwei Modi in einem Terminal

```
┌─────────────────────────────────────────────────────┐
│ Raijin Terminal                                     │
├─────────────────────────────────────────────────────┤
│                                                     │
│  [Shell Block] $ cargo build                        │  ← PTY Mode (OSC 133)
│  > Compiling raijin-app v0.1.0                      │
│  > Finished in 12.4s                            ✓   │
│                                                     │
│  [Shell Block] $ python                             │  ← PTY Mode startet REPL
│                                                     │
│  [REPL Block] >>> import numpy as np                │  ← Kernel Mode (Jupyter)
│  [REPL Block] >>> np.random.rand(3,3)               │
│  │ array([[0.42, 0.18, 0.91],                       │  ← Structured Output
│  │        [0.73, 0.55, 0.29],                       │
│  │        [0.64, 0.82, 0.47]])                      │
│                                                     │
│  [REPL Block] >>> plt.plot([1,2,3], [4,5,6])        │
│  │ ┌──────────────────────┐                         │  ← GPU-gerendertes Bild
│  │ │   📈 Matplotlib Plot │                         │     direkt im Block
│  │ │   (640×480)          │                         │
│  │ └──────────────────────┘                         │
│                                                     │
│  [REPL Block] >>> df.describe()                     │
│  │ ┌──────┬──────┬──────┐                           │  ← Native Tabelle
│  │ │ col  │ mean │ std  │                           │
│  │ ├──────┼──────┼──────┤                           │
│  │ │ age  │ 32.4 │ 12.1 │                           │
│  │ └──────┴──────┴──────┘                           │
│                                                     │
│  [Shell Block] $ echo "back to shell"               │  ← PTY Mode (nach exit())
│  > back to shell                                ✓   │
│                                                     │
├─────────────────────────────────────────────────────┤
│ >>> _                                               │  ← Input Bar (Kernel Completions)
│ [Python 3.11] [numpy] [pandas]                      │    Context Chips zeigen Kernel-Info
└─────────────────────────────────────────────────────┘
```

### Crate-Architektur

```
raijin-app
├── raijin-terminal-view (Feature Crate — Block Rendering + REPL Integration)
│   ├── block_list.rs         (bestehend — erweitert um ReplBlock Rendering)
│   ├── grid_element.rs       (bestehend — unverändert)
│   ├── block_element.rs      (bestehend — erweitert um REPL-Block-Header)
│   ├── repl_block.rs         (NEU — ReplBlockView: Rich Output in Blocks)
│   ├── repl_output.rs        (NEU — MIME Renderer: Image/Table/Markdown/JSON inline)
│   └── repl_detection.rs     (NEU — Foreground Process Detection)
│
├── raijin-terminal (Shared Infrastructure — erweitert)
│   ├── terminal.rs           (bestehend — erweitert um REPL Session Management)
│   └── repl_session.rs       (NEU — Sidecar Kernel Lifecycle, KernelSession impl)
│
├── raijin-repl (Shared Infrastructure — Jupyter Protocol, Kernel Management)
│   ├── kernels/              (bestehend — unverändert)
│   ├── outputs/              (bestehend — plain.rs GEFIXT)
│   ├── session.rs            (bestehend — wird von repl_session.rs genutzt)
│   └── repl_store.rs         (bestehend — Kernel Discovery)
│
└── raijin-completions (Shared Infrastructure — erweitert)
    └── kernel_completion.rs  (NEU — Jupyter complete_request Provider)
```

### Entscheidung: Wo lebt was?

| Code | Crate | Begründung |
|------|-------|------------|
| REPL Detection (Foreground Process) | raijin-terminal | Shared Infrastructure — wird von terminal-view UND Completions gebraucht |
| Sidecar Kernel Session | raijin-terminal | Shared Infrastructure — Kernel-Lifecycle ist Backend-Logik |
| Rich Output Rendering | raijin-terminal-view | Feature Crate — UI-spezifisch |
| REPL Block Layout | raijin-terminal-view | Feature Crate — UI-spezifisch |
| Kernel Completions | raijin-completions | Shared Infrastructure — wird von Input Bar benutzt |
| Jupyter Protocol/Messages | raijin-repl | Bestehend — unverändert |
| Kernel Specs/Discovery | raijin-repl | Bestehend — unverändert |

## Implementierung

### 1. raijin-repl fixen und verdrahten

**`outputs/plain.rs` reparieren:**

Der Import `raijin_terminal_view::terminal_element::TerminalElement` muss ersetzt werden. `TerminalOutput` nutzt `raijin_term::Term<VoidListener>` intern für ANSI-Parsing — das funktioniert. Das Rendering muss auf unseren `TerminalGridElement` umgebaut werden.

```rust
// outputs/plain.rs — VORHER (gebrochen):
use raijin_terminal_view::terminal_element::TerminalElement;
// In render(): TerminalElement::layout_grid(...)

// NACHHER:
// TerminalOutput.render() erzeugt einen TerminalGridElement aus dem internen Term
// 1. term.renderable_content() → Cells iterieren
// 2. BlockGridSnapshot aus Cells bauen
// 3. TerminalGridElement rendern
```

Konkreter Ansatz: `TerminalOutput` bekommt eine `to_grid_snapshot()` Methode die den internen `Term<VoidListener>` in ein `BlockGridSnapshot` konvertiert. Das `BlockGridSnapshot` wird dann von `TerminalGridElement` gerendert — exakt wie normale Terminal-Blocks.

**Cargo.toml fixen:**
- `raijin-terminal-view` Dependency entkommentieren ODER
- `TerminalOutput` braucht nur `raijin-term` + eigene Snapshot-Konversion (kein terminal-view Import nötig)

**raijin-repl in raijin-app verdrahten:**
- `raijin-repl` zu `raijin-app/Cargo.toml` hinzufügen
- `raijin_repl::init(fs, cx)` in `main.rs` aufrufen
- `ReplStore` wird als Global registriert

### 2. REPL Detection

**In `raijin-terminal`** — Foreground Process Inspection:

```rust
// raijin-terminal/src/repl_detection.rs

pub struct ReplDetector {
    known_repls: HashMap<&'static str, ReplInfo>,
}

pub struct ReplInfo {
    pub language: &'static str,
    pub jupyter_kernel: &'static str,  // z.B. "python3", "javascript", "ir"
    pub prompt_pattern: &'static str,  // z.B. ">>> ", "In \\[\\d+\\]: "
}

pub enum ForegroundProcess {
    Shell(String),           // bash, zsh, fish
    Repl(ReplInfo),          // python, node, irb
    FullscreenApp(String),   // vim, htop (ALT_SCREEN)
    Unknown(String),
}

impl ReplDetector {
    pub fn detect_foreground(&self, pty_fd: RawFd) -> ForegroundProcess {
        // 1. tcgetpgrp(pty_fd) → foreground process group
        // 2. sysctl KERN_PROCARGS2 (macOS) / /proc/pid/exe (Linux) → binary name
        // 3. Match gegen known_repls
    }
}
```

**Known REPLs:**
| Binary | Language | Jupyter Kernel | Prompt Pattern |
|--------|----------|---------------|----------------|
| python, python3, ipython | Python | python3 | `>>> `, `In [N]: ` |
| node | JavaScript | javascript | `> ` |
| irb, pry | Ruby | ruby | `irb(main):NNN:0> ` |
| ghci | Haskell | haskell | `ghci> `, `Prelude> ` |
| iex | Elixir | elixir | `iex(N)> ` |
| erl | Erlang | erlang | `N> ` |
| lua, luajit | Lua | lua | `> ` |
| R, Rscript | R | ir | `> ` |
| julia | Julia | julia | `julia> ` |
| psql | SQL | - (kein Kernel) | `=> ` |
| sqlite3 | SQL | - | `sqlite> ` |
| mongosh | MongoDB | - | `> ` |

REPLs ohne Jupyter-Kernel (psql, sqlite3, mongosh) bekommen trotzdem per-Expression Blocks via Prompt-Detection, aber ohne Rich Output.

### 3. Sidecar Kernel Session

**In `raijin-terminal`** — Kernel als Sidecar neben PTY:

```rust
// raijin-terminal/src/repl_session.rs

pub struct ReplSession {
    /// Der Jupyter Kernel der parallel zum PTY läuft
    kernel: Kernel,
    kernel_spec: KernelSpecification,

    /// Aktuelle Execution-Blöcke (msg_id → Block)
    executions: HashMap<String, ReplExecution>,

    /// Ob Input über Kernel oder PTY geroutet wird
    input_mode: ReplInputMode,

    /// Kernel Completions Cache
    completion_cache: Option<Vec<CompletionCandidate>>,
}

pub enum ReplInputMode {
    /// Input geht an PTY UND Kernel (Dual-Route)
    /// PTY für Terminal-Rendering, Kernel für strukturierten Output
    DualRoute,
    /// Input geht NUR an Kernel (Pure Kernel Mode)
    /// Für Notebooks und wenn PTY-Echo stört
    KernelOnly,
}

pub struct ReplExecution {
    pub msg_id: String,
    pub code: String,
    pub status: ExecutionStatus,
    pub outputs: Vec<ReplOutput>,
    pub started_at: Instant,
    pub finished_at: Option<Instant>,
}

pub enum ReplOutput {
    /// ANSI Text — gerendert als TerminalGridElement via Term<VoidListener>
    Terminal(TerminalOutputData),
    /// Bild — GPU-gerendert inline
    Image { data: Arc<RenderImage>, width: u32, height: u32 },
    /// Tabelle — Native Table Rendering
    Table(TabularDataResource),
    /// Markdown — Rendered Markdown
    Markdown(String),
    /// JSON — Faltbarer Tree
    Json(serde_json::Value),
    /// Error — Traceback mit ANSI Colors
    Error { ename: String, evalue: String, traceback: Vec<String> },
}
```

**Lifecycle:**

```
REPL detected (python3 als Foreground Process)
    │
    ├── ReplDetector meldet: ForegroundProcess::Repl(python3)
    │
    ├── Terminal schaut in ReplStore nach passender KernelSpecification
    │   └── KernelSpecification::Jupyter("python3") gefunden
    │
    ├── ReplSession::start(kernel_spec)
    │   ├── NativeRunningKernel::new() → TCP Ports, connection.json
    │   ├── start_kernel_tasks() → IOPub/Shell/Control/Stdin Tasks
    │   └── Kernel Status: Starting → Idle
    │
    ├── Input Bar wechselt zu Kernel Completions
    │   └── complete_request statt raijin-completions Specs
    │
    ├── User tippt: "np.random.rand(3,3)"
    │   ├── Input → PTY (für Terminal-Echo)
    │   └── Input → Kernel (execute_request für strukturierten Output)
    │
    ├── Kernel antwortet via IOPub:
    │   ├── status: busy
    │   ├── execute_result: { "text/plain": "array(...)", "text/html": "<table>..." }
    │   └── status: idle
    │
    ├── ReplSession.route() dispatcht Output:
    │   ├── MIME Ranking → Beste Darstellung wählen
    │   └── ReplOutput::Table/Image/Terminal erstellen
    │
    ├── BlockListView rendert ReplBlock:
    │   ├── Block-Header: ">>> np.random.rand(3,3)" [Python 3.11] [0.02s] ✓
    │   └── Block-Content: Native Tabelle ODER GPU-Bild ODER ANSI Grid
    │
    └── User tippt "exit()" → REPL beendet
        ├── ReplSession::shutdown()
        ├── Kernel Process beendet
        ├── Input Bar wechselt zurück zu Shell Completions
        └── Nächster Block ist wieder Shell-Mode
```

### 4. Block-System erweitern

**`raijin-term/src/block_grid.rs`** — BlockGrid bekommt REPL-Awareness:

```rust
// Bestehender BlockGrid erweitert:
pub struct BlockGrid {
    // ... bestehende Felder ...

    /// REPL-spezifische Daten (None für Shell-Blocks)
    pub repl_data: Option<ReplBlockData>,
}

pub struct ReplBlockData {
    /// Jupyter Execution Count (In [N]:)
    pub execution_count: Option<i32>,
    /// Rich Outputs (Bilder, Tabellen, etc.)
    pub rich_outputs: Vec<ReplOutput>,
    /// Kernel Language
    pub language: String,
    /// Execution Duration (Kernel-gemessen, präziser als Wall-Clock)
    pub kernel_duration_ms: Option<u64>,
}
```

**`raijin-terminal-view/src/block_element.rs`** — REPL Block Header:

```rust
// Bestehende render_block() erweitert:
pub fn render_block(snapshot: BlockSnapshot, ...) -> impl IntoElement {
    match &snapshot.repl_data {
        Some(repl) => render_repl_block(snapshot, repl, ...),
        None => render_shell_block(snapshot, ...),  // Bestehende Logik
    }
}

fn render_repl_block(snapshot: BlockSnapshot, repl: &ReplBlockData, ...) -> impl IntoElement {
    // Header: "In [42]: expression" [Python] [0.02s] ✓
    // Content: Erst ANSI-Grid (wenn vorhanden), dann Rich Outputs
    div()
        .child(render_repl_header(snapshot, repl))
        .child(TerminalGridElement::new(...))  // ANSI Text Output
        .children(repl.rich_outputs.iter().map(|output| {
            render_rich_output(output)
        }))
}
```

### 5. Rich Output Rendering

**`raijin-terminal-view/src/repl_output.rs`** — GPU-native MIME Renderer:

```rust
pub fn render_rich_output(output: &ReplOutput, window: &mut Window, cx: &mut App) -> AnyElement {
    match output {
        ReplOutput::Image { data, width, height } => {
            // Inazuma img() Element — GPU-gerendert, skaliert auf Block-Breite
            let (scaled_w, scaled_h) = scale_to_fit(*width, *height, max_block_width);
            img(data.clone())
                .size(size(px(scaled_w), px(scaled_h)))
                .object_fit(ObjectFit::Contain)
                .into_any_element()
        }

        ReplOutput::Table(resource) => {
            // Native Tabelle mit Column-Alignment und Zebra-Striping
            // Reuse TableView aus raijin-repl/outputs/table.rs
            let table_view = cx.new(|cx| TableView::new(resource, window, cx));
            table_view.into_any_element()
        }

        ReplOutput::Markdown(text) => {
            // Rendered Markdown mit Syntax Highlighting
            let markdown = cx.new(|cx| MarkdownView::new(text.clone(), cx));
            markdown.into_any_element()
        }

        ReplOutput::Json(value) => {
            // Faltbarer JSON Tree
            let json_view = cx.new(|cx| JsonView::new(value.clone(), cx));
            json_view.into_any_element()
        }

        ReplOutput::Error { ename, evalue, traceback } => {
            // Error mit ANSI-colored Traceback
            // Jede Traceback-Line durch Term<VoidListener> für ANSI Colors
            render_error_output(ename, evalue, traceback, window, cx)
        }

        ReplOutput::Terminal(data) => {
            // Standard ANSI Output via TerminalGridElement
            let snapshot = data.to_grid_snapshot();
            TerminalGridElement::new(Arc::new(snapshot), ...)
                .into_any_element()
        }
    }
}
```

### 6. Kernel Completions

**`raijin-completions/src/kernel_completion.rs`:**

```rust
pub struct KernelCompletionProvider {
    session: WeakEntity<ReplSession>,
}

impl KernelCompletionProvider {
    pub async fn complete(&self, code: &str, cursor_pos: usize) -> Vec<CompletionCandidate> {
        // Jupyter complete_request:
        // { "code": "import nump", "cursor_pos": 11 }
        // → { "matches": ["numpy"], "cursor_start": 7, "cursor_end": 11 }
        let request = CompleteRequest {
            code: code.to_string(),
            cursor_pos: cursor_pos as u32,
        };
        let reply = self.session.send_complete(request).await;
        reply.matches.iter().map(|m| CompletionCandidate {
            label: m.clone(),
            kind: CompletionKind::KernelCompletion,
            ..
        }).collect()
    }

    pub async fn inspect(&self, code: &str, cursor_pos: usize) -> Option<String> {
        // Jupyter inspect_request → Docstring/Signature
        let request = InspectRequest {
            code: code.to_string(),
            cursor_pos: cursor_pos as u32,
            detail_level: 0,
        };
        let reply = self.session.send_inspect(request).await;
        reply.data.get("text/plain").cloned()
    }
}
```

**Completion Switching in Input Bar:**

```rust
// raijin-terminal-view/src/terminal_pane.rs
impl TerminalPane {
    fn update_completion_provider(&mut self, cx: &mut Context<Self>) {
        match self.terminal.repl_session() {
            Some(session) => {
                // REPL aktiv → Kernel Completions
                self.shell_completion.set_provider(
                    CompletionProvider::Kernel(KernelCompletionProvider::new(session))
                );
                // Context Chips aktualisieren
                self.update_repl_context_chips(session, cx);
            }
            None => {
                // Shell → Standard Completions
                self.shell_completion.set_provider(
                    CompletionProvider::Shell(ShellCompletionProvider::default())
                );
            }
        }
    }
}
```

### 7. Context Chips für REPL

Wenn ein REPL aktiv ist, zeigt die Input Bar REPL-spezifische Context Chips:

```
┌─────────────────────────────────────────────────┐
│ >>> _                                           │
│ [Python 3.11] [ipykernel] [numpy 1.24] [●Idle] │
└─────────────────────────────────────────────────┘
```

- **Language + Version**: Aus `kernel_info_reply.language_info`
- **Kernel Name**: Aus KernelSpecification
- **Imported Modules**: Aus Kernel State (via custom inspect)
- **Kernel Status**: Idle/Busy/Error — Live-Update via IOPub `status` Messages

### 8. TerminalOutput Snapshot-Konversion

Die Brücke zwischen `raijin-repl`'s `TerminalOutput` (Term\<VoidListener\>) und unserem Block-Rendering:

```rust
// raijin-terminal-view/src/repl_output.rs oder raijin-repl/src/outputs/plain.rs

impl TerminalOutput {
    /// Konvertiert den internen Term<VoidListener> in ein BlockGridSnapshot
    /// das von TerminalGridElement direkt gerendert werden kann.
    pub fn to_grid_snapshot(&self, theme: &Theme) -> BlockGridSnapshot {
        let content = self.handler.renderable_content();
        let mut lines = Vec::new();

        for indexed in content.display_iter {
            let row = indexed.point.line.0 as usize;
            while lines.len() <= row {
                lines.push(SnapshotLine { cells: Vec::new() });
            }

            let cell = indexed.cell;
            let (fg, bg) = resolve_cell_colors(cell, &content.colors, theme);

            lines[row].cells.push(SnapshotCell {
                c: cell.c,
                zerowidth: cell.zerowidth().to_vec(),
                fg,
                bg,
                bold: cell.flags.contains(Flags::BOLD),
                italic: cell.flags.contains(Flags::ITALIC),
                underline: cell.flags.contains(Flags::UNDERLINE),
                strikeout: cell.flags.contains(Flags::STRIKEOUT),
                wide: cell.flags.contains(Flags::WIDE_CHAR),
                font_family_override: None,
            });
        }

        BlockGridSnapshot {
            content_rows: lines.len(),
            command_row_count: 0,
            grid_history_size: 0,
            grid_cols: self.handler.columns(),
            lines,
        }
    }
}
```

## Dateien die geändert/erstellt werden

### Neue Dateien

| Datei | Crate | Inhalt |
|-------|-------|--------|
| `repl_detection.rs` | raijin-terminal | Foreground Process Detection, Known REPL Registry |
| `repl_session.rs` | raijin-terminal | Sidecar Kernel Lifecycle, KernelSession impl, ReplOutput |
| `repl_block.rs` | raijin-terminal-view | ReplBlockView Rendering, REPL Block Header |
| `repl_output.rs` | raijin-terminal-view | Rich Output GPU Renderer (Image, Table, Markdown, JSON) |
| `kernel_completion.rs` | raijin-completions | Jupyter complete_request/inspect_request Provider |

### Geänderte Dateien

| Datei | Änderung |
|-------|----------|
| `raijin-repl/src/outputs/plain.rs` | TerminalElement Import → to_grid_snapshot() + TerminalGridElement |
| `raijin-repl/Cargo.toml` | raijin-terminal-view Dependency entfernen, nur raijin-term behalten |
| `raijin-terminal/src/terminal.rs` | ReplSession Feld + REPL Detection Polling + repl_session() Getter |
| `raijin-terminal-view/src/block_element.rs` | render_repl_block() für REPL-spezifische Block-Header |
| `raijin-terminal-view/src/block_list.rs` | ReplBlock Integration in Block-Liste |
| `raijin-terminal-view/src/terminal_pane.rs` | Completion Provider Switching + REPL Context Chips |
| `raijin-term/src/block_grid.rs` | `repl_data: Option<ReplBlockData>` Feld in BlockGrid |
| `raijin-completions/src/shell_completion.rs` | CompletionProvider Enum (Shell vs Kernel) |
| `raijin-app/Cargo.toml` | raijin-repl Dependency hinzufügen |
| `raijin-app/src/main.rs` | raijin_repl::init(fs, cx) Aufruf |

## Nicht-Ziele (explizit ausgeschlossen)

- **Notebook-Editor UI** (raijin-repl/notebook/) — Das ist ein separates Feature (Editor-basierte Notebooks). Wir bauen die Terminal-REPL-Integration, nicht einen Notebook-Editor.
- **Remote Kernel Management UI** — Kernel Discovery passiert automatisch. Kein UI zum Verwalten von Kernel-Servern.
- **REPL für nicht-Jupyter REPLs** (psql, sqlite3) — Per-Expression Blocks via Prompt-Detection kommen in einer späteren Phase. Hier nur Jupyter-fähige REPLs.
- **DAP/Debugger Integration** — Debug Adapter Protocol kommt separat.

## Abhängigkeiten

- raijin-term BlockGrid System (vorhanden ✓)
- raijin-terminal-view Block Rendering (vorhanden ✓)
- raijin-repl Jupyter Protocol (vorhanden ✓)
- raijin-completions Engine (vorhanden ✓)
- `runtimelib` Crate für Jupyter Messages (vorhanden ✓)
- `jupyter-protocol` Crate für KernelSpecs (vorhanden ✓)
