# Block-Architektur Rewrite — raijin-term + Grid pro Block

## Context

Warp hat `alacritty_terminal` geforkt und drei echte Grids pro Block (Prompt, Command, Output). Der PTY-Output wird zur Laufzeit auf das aktive Grid geroutet. Keine Snapshots, keine nachträgliche Extraktion.

Wir machen das gleiche. `alacritty_terminal` wird zu `raijin-term` gevendort und rebranded — komplett, keine Spuren des Originals. Das ist unser Pattern (wie Inazuma ex-GPUI).

---

## raijin-term — Gevendorter Alacritty-Fork

`alacritty_terminal` Sourcecode in unseren Workspace vendorn, Remote entfernen, Crate umbenennen zu `raijin-term`. Grid (Circular-Buffer, Scrollback, Resize/Reflow, Cell-Storage) bleibt weitgehend unverändert. VTE-Parser (`vte` Crate) bleibt externe Dependency.

**Rebranding-Regeln:**
- Keine Alacritty-Referenzen in Kommentaren, Variablennamen, Dokumentation, Cargo-Metadaten
- Interne Typen bekommen unsere Naming-Conventions
- Code wird so refactored dass er sich wie von uns geschrieben anfühlt

---

## BlockGridRouter — Multi-Grid-Architektur

```rust
pub struct BlockGridRouter {
    /// Alle abgeschlossenen + aktiven Block-Grids
    blocks: Vec<BlockGrid>,
    /// ID des aktiven Blocks (oder None wenn kein Command läuft)
    active_block_id: Option<BlockId>,
    /// Prompt-Grid: Fängt Prompt-Bytes auf (Starship etc.), wird nie gerendert.
    /// Wird bei jedem PromptStart resetted. Hält den VTE-Parser-State konsistent.
    prompt_grid: BlockGrid,
    /// Alt-Screen-Grid (vim, htop) — lebt innerhalb des aktiven Blocks
    alt_grid: Option<Grid<Cell>>,
}

pub struct BlockGrid {
    pub id: BlockId,
    pub grid: Grid<Cell>,
    /// Cursor + saved_cursor — lebt PRO Grid, nicht global
    pub cursor: Cursor,
    /// Terminal modes für dieses Grid (insert mode, origin mode, autowrap, etc.)
    pub mode: TermMode,
    pub command: String,
    pub exit_code: Option<i32>,
    pub metadata: Option<BlockMetadata>,
    pub started_at: Instant,
    pub finished_at: Option<Instant>,
}
```

### Warum prompt_grid existieren muss

Der VTE-Parser ist eine State Machine. Jedes Byte beeinflusst den State — Farben, Cursor-Position, Modes. Starship setzt ANSI-Codes die bis zum nächsten Reset gelten. Wenn Prompt-Bytes gedroppt werden ("ins Nichts"), ist der Parser-State für den folgenden Command-Block inkonsistent.

Lösung: `prompt_grid` fängt alle Prompt-Bytes sauber auf. Bei PromptStart wird es resetted. Beim Rendern wird es ignoriert.

### Cursor-State pro Grid

In `alacritty_terminal` lebt der Cursor IN `Grid.cursor`, aber Terminal-Modes (`insert_mode`, `origin_mode`, `autowrap`, `tab_stops`, `charset`, `saved_cursor`) leben in `Term`. Beim Grid-Switch muss der Cursor+Mode mitswitchen.

Jedes `BlockGrid` speichert seinen eigenen `cursor: Cursor` + `mode: TermMode`. Beim Grid-Switch wird der aktuelle State gespeichert und der neue geladen — analog zu wie Alacritty zwischen primary/alt_grid switcht.

### Grid-Sizing

- Neue Block-Grids starten mit `cols = terminal_cols`, `rows = 24` (initial chunk)
- Rows wachsen dynamisch wenn Output reinkommt
- Window-Resize: Alle Block-Grids bekommen Column-Reflow (Zeilenumbrüche ändern sich)
- Lazy: Nur das aktive Block-Grid wird sofort resized, fertige Blocks erst beim Rendern
- Alt-Screen Apps: Alt-Grid innerhalb des aktiven Blocks, nutzt volle Viewport-Höhe

### Memory-Management

Von Anfang an eingebaut (Warp hatte nachträglich Memory-Issues von 3.6GB bis 113GB):

- **Max-Block-Count** — älteste Blocks werden ab Limit (default: 200) gedroppt
- **Max-Rows-per-Block** — Scrollback-Limit pro Block-Grid (default: 10.000)
- Beides konfigurierbar über `~/.config/raijin/config.toml`

---

## PTY-Output-Routing — Vollständiger Zyklus

```
PromptStart
  → prompt_grid wird resetted und als aktives Grid gesetzt
  → Starship/P10k-Bytes landen hier, werden nie gerendert
  → VTE-Parser-State bleibt konsistent

InputStart
  → prompt_grid bleibt aktiv
  → User-Input geht über unser Input-Feld (nicht über PTY)

CommandStart
  → Neues BlockGrid wird erstellt (cols = terminal_cols, rows = 24)
  → Cursor + Mode werden initialisiert
  → Neues Grid wird als aktiv gesetzt
  → Output fließt live in dieses Grid

CommandEnd { exit_code }
  → BlockGrid wird finalisiert (exit_code, finished_at)
  → Kein aktives Block-Grid mehr
  → Bytes bis zum nächsten PromptStart gehen ins prompt_grid

PromptStart
  → prompt_grid resetted, Zyklus beginnt neu

Zwischenzustand (nach CommandEnd, vor PromptStart):
  → Bytes gehen ins prompt_grid
```

### Handler-Routing im Term

```rust
impl Handler for Term {
    fn input(&mut self, c: char) {
        self.block_router.active_grid_mut().write(c);
    }

    fn goto(&mut self, line: Line, col: Column) {
        self.block_router.active_cursor_mut().goto(line, col);
    }

    // Alle Handler-Methoden routen zum aktiven Grid + Cursor
}
```

---

## Input-Feld-Lifecycle

**Aktuell (FALSCH):** Input-Feld klebt statisch unten, immer sichtbar.

**Richtig (Warp-Modell):**

1. Input-Feld ist da (mit Context Chips), User tippt Command
2. User drückt Enter
3. Input-Feld **VERSCHWINDET** komplett
4. An seiner Stelle: neuer Block — Header + Command-Text + Live-Output
5. Während Command läuft: **KEIN Input-Feld** — nur aktiver Block mit streamendem Output
6. Erst bei CommandEnd: neues Input-Feld spawnt **UNTER** dem fertigen Block

### Layout-Modell

```
Ruhezustand (kein Command):
┌──────────────────────────────┐
│  Fertige Blocks (scrollbar)  │
│  ...                         │
│  Block N                     │
├──────────────────────────────┤
│  [Chips] nyxb Mac ~ 15:30   │
│  Input-Editor mit Cursor     │
└──────────────────────────────┘

Command läuft:
┌──────────────────────────────┐
│  Fertige Blocks (scrollbar)  │
│  ...                         │
│  Block N                     │
│  Block N+1 (aktiv, live)     │
│    Header: nyxb Mac ~ 15:31  │
│    Command: cargo build      │
│    Output: streaming...      │
│                              │
│  (KEIN Input-Feld!)          │
└──────────────────────────────┘
```

---

## Sticky Command Header

### Verhalten

a) **Viewport-basiert:** Ist der Block-Header (Metadata + Command) im Viewport sichtbar? Nein → Sticky Header anzeigen. Gilt für **aktive UND fertige Blocks**. Auch während Live-Output streamt und nach unten wächst — sobald der Header oben rausscrollt, erscheint das Overlay sofort.

b) Bei langen mehrzeiligen Commands → **einklappbares Overlay**. Eingeklappt: nur Header-Zeile. Ausgeklappt: voller Command-Text.

c) Click auf Sticky Header → **Jump to top of block**.

d) **"Jump to bottom"** Button unten wenn man hochgescrollt hat und der aktuelle Output-End nicht sichtbar ist.

### Implementation

`block_list.rs` braucht Viewport-Tracking: Für jeden teilweise sichtbaren Block prüfen ob sein Header-Bereich im Viewport liegt. Wenn nicht → Sticky Overlay oben rendern.

---

## Datei-Struktur

```
crates/raijin-term/                   # NEU: Gevendorter alacritty_terminal Fork
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── grid.rs                       # Grid (weitgehend unverändert)
│   ├── term.rs                       # Term mit Multi-Grid-Support
│   ├── block_grid.rs                 # BlockGridRouter + BlockGrid
│   ├── vte_handler.rs                # Handler Trait Impl (routet zu aktivem Grid)
│   ├── cell.rs                       # Cell, CellFlags
│   ├── color.rs                      # Color types
│   ├── event.rs                      # EventListener trait
│   ├── selection.rs                  # Text-Selection (Phase 2)
│   └── ...

crates/raijin-terminal/               # Wrapper-Crate
├── src/
│   ├── lib.rs
│   ├── terminal.rs                   # Wrapped raijin-term
│   ├── pty.rs                        # Shell-Hooks, PTY-Spawning
│   ├── osc_parser.rs                 # OSC 133/7777 Scanning
│   ├── block.rs                      # BlockManager (vereinfacht)
│   └── event.rs

crates/raijin-app/src/
├── main.rs
├── workspace.rs                      # Schlank — nur Orchestrierung
│
├── terminal.rs                       # Re-exports
├── terminal/
│   ├── block_element.rs              # Block als Inazuma-Element
│   ├── block_list.rs                 # Scrollbare Block-Liste + Sticky Header
│   ├── live_block.rs                 # Aktiver Block aus Live-Grid
│   ├── colors.rs                     # Raijin Dark Theme, ANSI → Hsla
│   ├── text_rendering.rs             # Grid → ShapedLine
│   └── constants.rs                  # Layout-Konstanten
│
├── input.rs                          # Re-exports
├── input/
│   ├── input_area.rs                 # Context Chips + ShellEditor
│   ├── history_panel.rs              # History-Panel Overlay
│   └── shell_selector.rs             # Shell-Dropdown
│
├── completions.rs                    # Re-exports
├── completions/
│   ├── shell_completion.rs           # ShellCompletionProvider
│   └── command_correction.rs         # Typo-Korrektur + Banner-UI
│
├── command_history.rs
└── settings_view.rs
```

---

## PR-Aufteilung

### PR 1: raijin-term Crate
- `alacritty_terminal` vendorn, umbenennen, komplett rebranden
- Multi-Grid: `BlockGridRouter`, `BlockGrid` mit eigenem Cursor+Mode
- `prompt_grid` für VTE-State-Konsistenz
- Handler-Routing zum aktiven Grid
- Memory-Limits (max blocks, max rows per block)
- `raijin-terminal` wrapper auf `raijin-term` umstellen
- Keine UI-Änderungen

### PR 2: Rendering Switch + Block-UX
- `terminal/` Module die aus Block-Grids rendern
- `terminal_element.rs` wird gelöscht
- BlockElement (div-basiert, klickbar, selektierbar)
- BlockListView (scrollbar, Sticky Headers)
- LiveBlock (aktiver Command)
- Input-Feld-Lifecycle (verschwindet bei Command-Run)
- Block Dividers
- Block Click/Selection/Copy (Cmd+C)

### PR 3: Modularisierung
- `input/`, `completions/` Submodule
- `workspace.rs` verschlanken
- History-Panel, Shell-Selector, Corrections → eigene Module

---

## Verifikation

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
cargo run -p raijin-app
```

| Test | Erwartung |
|------|-----------|
| Block visuell | Command + Output ein Container, gleiches BG + Padding |
| Block klicken | Gesamter Block grün hervorgehoben |
| Block kopieren | Cmd+C → Command + Output reiner Text |
| Escape | Block-Selektion aufheben |
| Error-Block | Roter Left-Border, gesamte Block-Höhe |
| Starship-Prompt | Nicht sichtbar (prompt_grid nicht gerendert) |
| Mehrzeiliger Command | Im Header mehrzeilig mit Einrückung |
| Laufender Command | Live-Output, kein Input-Feld sichtbar |
| Command fertig | Input-Feld spawnt unter fertigem Block |
| Scrollen | Über alle Blocks vertikal scrollen |
| Sticky Header | Header fixiert wenn aus Viewport gescrollt |
| Jump to top | Click auf Sticky Header springt zum Block-Anfang |
| `clear` / Ctrl+L | Blocks bleiben, werden hochgescrollt |
| Langer Output >10k | Block-Grid eigenes Scrollback, kein Datenverlust |
| Command ohne Output | Block mit Header, leerer Output |
| Alt-Screen (vim) | Alt-Grid im aktiven Block |
| Ctrl+C | Block mit exit_code 130 geschlossen |
| 200+ Commands | Älteste Blocks werden gedroppt (Memory-Limit) |
| Window-Resize | Column-Reflow für alle Blocks |
| Parser-State | Prompt-Grid hält VTE-State konsistent |
