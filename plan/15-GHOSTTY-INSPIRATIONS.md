# Ghostty-Inspirationen für raijin-term

Verbesserungen die wir uns von Ghostty's Implementierung abgucken können,
nachdem der alacritty_terminal-Fork (raijin-term) steht.

---

## 1. SIMD-optimierter VTE-Scanner

**Was Ghostty macht:**
Ghostty's `Parser.zig` nutzt CPU-spezifische SIMD-Instruktionen (SSE2/AVX2 auf x86, NEON auf ARM)
um den Byte-Stream zu scannen. Statt Byte-für-Byte durch eine State-Machine zu laufen,
werden 16-32 Bytes gleichzeitig klassifiziert: "normaler Text" vs "Escape-Sequenz-Start" (0x1B).
Normaler Text wird direkt in Bulk ans Grid weitergereicht, nur bei Escape-Sequenzen wird der
volle VTE-Parser aktiviert.

**Was Alacritty aktuell macht:**
Die `vte` Crate verarbeitet jeden Byte einzeln durch einen tabellenbasierten DFA.
Funktioniert, ist aber nicht optimiert für den häufigsten Fall (>95% der Bytes sind druckbarer Text).

**Was wir tun sollten:**
Einen SIMD-beschleunigten Pre-Scanner vor den VTE-Parser schalten.
In Rust geht das mit `std::arch` (SSE2/AVX2) oder dem `std::simd` nightly API.

```
PTY-Bytes → SIMD-Scanner → Bulk-Text direkt ans Grid
                         → Escape-Bytes → VTE-Parser → Grid
```

**Wo im Code:**
`raijin-term/src/vte/` — neues Modul `simd_scanner.rs` das vor `parser.rs` sitzt.

**Erwarteter Gewinn:**
2-5x schnellerer Durchsatz bei text-lastigen Ausgaben (z.B. `cat large_file.txt`, Compiler-Output).
Bei interaktiver Nutzung kaum spürbar, aber messbar in Benchmarks.

**Priorität:** Mittel — nach Grid-pro-Block, vor v1.0

---

## 2. Threaded Architecture mit minimaler Lock-Contention

**Was Ghostty macht:**
Drei dedizierte Threads mit klar getrennten Zuständigkeiten:
- **Read-Thread:** Liest PTY-Output, parst VTE, schreibt in Terminal-State
- **Write-Thread:** Sendet User-Input an PTY via Mailbox-Pattern
- **Render-Thread:** Liest Terminal-State für GPU-Rendering

Zwischen Read- und Render-Thread gibt es einen optimierten Synchronisationspunkt.
Mitchell hat kürzlich die Lock-Hold-Time um 2.4x reduziert durch feinere Granularität
(Lock pro Block/Region statt globaler Lock auf den ganzen Terminal-State).

**Was Alacritty aktuell macht:**
`terminal.rs` hat einen `spawn_pty_reader` Thread der durch einen `Arc<FairMutex<Term>>`
synchronisiert wird. Der Render-Thread lockt denselben Mutex zum Lesen.
Ein globaler Lock auf den gesamten `Term`-State.

**Was wir tun sollten:**
Mit Grid-pro-Block haben wir eine natürliche Granularität für feinere Locks:
- Jeder `BlockGrid` bekommt seinen eigenen `RwLock`
- Der Read-Thread lockt nur den aktiven Block (write)
- Der Render-Thread lockt nur die sichtbaren Blocks (read)
- Fertige Blocks werden nie mehr geschrieben → Read-Lock reicht, zero contention

```rust
struct BlockGrid {
    grid: RwLock<Grid<Cell>>,
    cursor: Cursor,
    // ...
}
```

**Wo im Code:**
`raijin-term/src/block_grid.rs` — Lock-Design direkt beim Grid-pro-Block-Umbau einbauen.

**Erwarteter Gewinn:**
Render-Thread blockiert nie auf den aktiven Block wenn er gerade fertige Blocks rendert.
Besonders spürbar bei schnellem Output (Compiler, Logs) wo Read- und Render-Thread
gleichzeitig arbeiten müssen.

**Priorität:** Hoch — direkt beim Grid-pro-Block-Umbau berücksichtigen

---

## 3. Grapheme-Handling mit Inline-Storage

**Was Ghostty macht:**
Cells haben ein kompaktes Inline-Format für den häufigsten Fall (ein einzelnes Codepoint).
Multi-Codepoint-Grapheme (Emoji-Sequenzen wie 👨‍👩‍👧‍👦, Combining Characters)
werden in einem Page-lokalen Bitmap-Allocator gespeichert. Die Cell hält nur einen
Offset/Index in diesen Allocator. Kein Heap-Alloc pro Cell.

**Was Alacritty aktuell macht:**
Normale Cells speichern ein `char` inline. Sobald ein Grapheme mehrere Codepoints hat,
wird ein `CellExtra` auf dem Heap allokiert (`Box<CellExtra>`) das die zusätzlichen
Codepoints enthält. Jedes Emoji mit ZWJ-Sequenz = eine Heap-Allokation.

**Was wir tun sollten:**
Einen Block-lokalen Grapheme-Store einführen:

```rust
struct BlockGrid {
    grid: Grid<Cell>,
    grapheme_store: GraphemeStore,  // Arena-Allocator für Multi-Codepoint-Grapheme
}

struct Cell {
    codepoint: u32,                // Inline für Single-Codepoint (häufigster Fall)
    grapheme_idx: Option<u32>,     // Index in grapheme_store falls Multi-Codepoint
    fg: Color,
    bg: Color,
    flags: CellFlags,
}

struct GraphemeStore {
    data: Vec<u32>,                // Codepoints hintereinander
    offsets: Vec<(u32, u16)>,      // (start_offset, length) pro Grapheme
}
```

**Wo im Code:**
`raijin-term/src/grid/cell.rs` + neues `raijin-term/src/grid/grapheme_store.rs`

**Erwarteter Gewinn:**
Eliminiert Heap-Allokationen für Emoji-Heavy-Output. Bessere Cache-Locality
weil alle Grapheme-Daten eines Blocks zusammenliegen. Einfacheres Cleanup
(Block droppen = Store droppen, keine einzelnen Box-Deallocs).

**Priorität:** Mittel — nach Grid-pro-Block, gut kombinierbar mit Cell-Refactoring

---

## 4. Dedizierter Input-Thread mit Mailbox-Pattern

**Was Ghostty macht:**
User-Input wird nicht direkt auf den PTY-fd geschrieben, sondern in eine
Lock-Free-Mailbox gepostet. Ein dedizierter Write-Thread konsumiert die Mailbox
und schreibt gesammelt an den PTY. Das entkoppelt UI-Thread von PTY-Writes
und ermöglicht Batching (mehrere Keystrokes in einem write()-Call).

**Was Alacritty aktuell macht:**
`TerminalHandle::write()` schreibt direkt über `OwnedFd` an den PTY.
Das passiert vom UI-Thread aus, was bei langsamen PTY-Reads (z.B. SSH)
den UI-Thread blockieren kann.

**Was wir tun sollten:**
```rust
// Im UI-Thread:
terminal.send_input(b"ls -la\n");  // Non-blocking, postet in Channel

// Dedizierter Write-Thread:
fn write_loop(rx: Receiver<Vec<u8>>, pty_fd: OwnedFd) {
    let mut batch = Vec::new();
    while let Ok(input) = rx.recv() {
        batch.extend_from_slice(&input);
        // Drain alle pending Messages (Batching)
        while let Ok(more) = rx.try_recv() {
            batch.extend_from_slice(&more);
        }
        pty_fd.write_all(&batch).ok();
        batch.clear();
    }
}
```

**Wo im Code:**
`raijin-term/src/pty.rs` — Write-Thread neben dem bestehenden Read-Thread.

**Erwarteter Gewinn:**
UI bleibt responsive auch wenn PTY-Writes blockieren (SSH, langsame Pipes).
Input-Batching reduziert Syscalls bei schnellem Tippen.

**Priorität:** Niedrig — Nice-to-have, aktuelle Lösung funktioniert für lokale Shells

---

## 5. Synchronized Rendering (DCS Sequences)

**Was Ghostty macht:**
Implementiert das Synchronized-Rendering-Protokoll (BSU/ESU — Begin/End Synchronized Update,
DCS `?2026h` / `?2026l`). Zwischen BSU und ESU werden Änderungen am Grid gepuffert und
erst bei ESU in einem Batch ans Rendering übergeben. Verhindert Tearing bei schnellen
Full-Screen-Updates (TUIs wie htop, vim-Redraws).

**Was Alacritty aktuell macht:**
Alacritty unterstützt Synchronized Rendering seit v0.13, aber die Implementierung
ist ein simpler "damage flag" — kein echtes Buffering der Grid-Änderungen.

**Was wir tun sollten:**
Mit Grid-pro-Block haben wir einen natürlichen Punkt fürs Buffering:
- BSU empfangen → Render-Thread überspringt den aktiven Block
- Alle Änderungen landen normal im Grid (Write-Lock nur auf aktiven Block)
- ESU empfangen → Render-Thread darf den Block wieder lesen
- Ergebnis: Zero-Cost wenn kein BSU/ESU, sauberes Rendering wenn doch

**Wo im Code:**
`raijin-term/src/block_grid_router.rs` — Flag `sync_rendering_active: bool` pro Block.

**Erwarteter Gewinn:**
Tear-freies Rendering für TUI-Apps (vim, htop, btop, lazygit).
Besonders wichtig weil unser Grid-pro-Block-Ansatz sonst bei jedem
VTE-Parse-Schritt ein Render triggern könnte.

**Priorität:** Hoch — sollte beim Grid-pro-Block-Umbau direkt mit rein

---

## Zusammenfassung nach Priorität

| # | Verbesserung | Priorität | Wann |
|---|-------------|-----------|------|
| 2 | Feinere Locks (RwLock pro Block) | Hoch | Beim Grid-pro-Block-Umbau |
| 5 | Synchronized Rendering | Hoch | Beim Grid-pro-Block-Umbau |
| 1 | SIMD VTE-Scanner | Mittel | Nach Grid-pro-Block |
| 3 | Grapheme Inline-Storage | Mittel | Nach Grid-pro-Block |
| 4 | Input-Thread Mailbox | Niedrig | Vor v1.0 |

**Nicht übernehmen:**
- mmap-basierte PageList (löst ein Problem das Grid-pro-Block nicht hat)
- Zig-spezifische Optimierungen (wir bleiben bei Rust)
- Ghostty's Shell-Integration-Approach (wir haben eigene OSC 7777 Hooks die weiter gehen)
