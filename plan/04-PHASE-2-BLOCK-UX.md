# Phase 2: Block-UX + Warp-Style Layout (Woche 4–6)

> **Ziel:** Warp-Level Layout mit Bottom-Input, Block-System, Shell-Integration

---

## Architektur-Entscheidung: Warp-Style Layout

Warp trennt den Input komplett von der Shell. Der Input-Editor lebt im Terminal, nicht im PTY. Das ermöglicht:
- Input unten gepinnt (fixiert), Output scrollt oben
- Volle IDE-Features im Input (Syntax-Highlighting, Multi-Cursor, Completions)
- Jeder Command+Output ist ein isolierter "Block" mit eigenem Grid

### Window-Layout (von oben nach unten):

```
┌─────────────────────────────────────────┐
│ Tab Bar (Tabs mit Farb-Indikatoren)     │  ← Fixed top
├─────────────────────────────────────────┤
│                                         │
│  Scrollable Block Area                  │  ← Blocks fließen nach oben
│  ┌─ Block ────────────────────────┐     │
│  │ ⬤ $ git status          0.3s  │     │  ← Block Header (Command + Duration)
│  │ On branch main                 │     │  ← Block Output
│  │ nothing to commit              │     │
│  └────────────────────────────────┘     │
│  ┌─ Block ────────────────────────┐     │
│  │ ✖ $ cargo build          12s  │     │  ← Exit Code ✖ = Error
│  │ error[E0308]: mismatched types │     │
│  └────────────────────────────────┘     │
│                                         │
├─────────────────────────────────────────┤
│ Context Chips: 📁 ~/raijin  🌿 main    │  ← Prompt-Info (CWD, Git, etc.)
│ ▶ _                                    │  ← Input Editor (fixed bottom)
│ ┌ 📎  🎤  💬 Agent Status ──────┐      │  ← Agent Toolbar (wenn aktiv)
└─────────────────────────────────────────┘
```

### Wie Warp den Input vom PTY trennt:

1. Shell startet normal via PTY
2. Shell-Hooks (precmd/preexec) senden DCS-Marker an Terminal
3. User tippt in **Terminal's eigenen Editor** (nicht Shell's readline)
4. Bei Enter: Terminal sendet kompletten Command an PTY
5. Shell empfängt, führt aus, Output geht an PTY
6. Terminal parst DCS-Marker → erstellt neuen Block
7. Block speichert Command + Output in isoliertem Grid

### Ohne Shell-Integration (Fallback):
- Terminal funktioniert trotzdem als normales Terminal
- Blocks werden nicht getrennt, Output fließt als ein Strom
- Input-Editor unten funktioniert trotzdem (sendet Zeilen an PTY bei Enter)

---

## 2.1 — Warp-Style Window Layout

- [ ] Drei-Zonen Layout: Tab Bar (top) → Block Area (scrollbar, middle) → Input Bar (fixed, bottom)
- [ ] Input-Editor als eigenes Inazuma-Component am unteren Rand
- [ ] Block Area als scrollbarer Container der nach oben wächst
- [ ] Context Chips über dem Input (CWD, Git Branch, Rust Version etc.)
- [ ] Layout responsive: Block Area füllt verfügbaren Platz

---

## 2.2 — Input Editor (Terminal-Level, nicht Shell)

- [ ] Eigener Text-Editor am unteren Rand (nicht Shell's readline)
- [ ] Multi-Line Support (Shift+Enter für neue Zeile)
- [ ] Syntax-Highlighting für Shell-Commands
- [ ] Bei Enter: Gesamten Buffer an PTY senden
- [ ] History Navigation (Pfeil hoch/runter) durch vorherige Commands
- [ ] Prompt-Prefix zeigt aktuellen Context (User, Host, CWD)

---

## 2.3 — Shell-Integration (precmd/preexec Hooks)

- [ ] Shell-Hook-Scripts erstellen für zsh, bash, fish:
  - `precmd`: Sendet OSC-Marker vor jedem Prompt (`\x1b]133;A\x07`)
  - `preexec`: Sendet OSC-Marker vor Command-Execution (`\x1b]133;C\x07`)
  - Command-Ende Marker (`\x1b]133;D;$?\x07`) mit Exit-Code
- [ ] VTE-Parser erweitern um OSC-Markers zu erkennen
- [ ] DCS mit JSON-Metadata für erweiterte Info (CWD, Git, etc.)
- [ ] Daraus Command-Boundaries ableiten: wo fängt ein Command an, wo endet sein Output
- [ ] Graceful Degradation: Ohne Hooks → klassisches Terminal-Verhalten

---

## 2.4 — Block-Datenmodell

- [ ] `TerminalBlock` struct:

```rust
struct TerminalBlock {
    id: BlockId,
    command: String,              // Der eingegebene Command
    output: Vec<TerminalLine>,    // Output-Zeilen mit ANSI-Farben
    start_time: Instant,
    end_time: Option<Instant>,
    exit_code: Option<i32>,
    working_directory: PathBuf,
    git_branch: Option<String>,
    is_collapsed: bool,
    is_selected: bool,
}
```

- [ ] `BlockManager` der `Vec<TerminalBlock>` maintained
- [ ] Neuer Block bei jedem preexec-Marker
- [ ] Block abgeschlossen bei precmd-Marker (nächster Prompt)
- [ ] Exit-Code aus OSC `133;D` Marker extrahieren

---

## 2.5 — Block-Rendering

- [ ] Jeden Block als eigene Inazuma-View rendern:
  - Block Header: Command-Zeile + Exit-Code Badge + Duration
  - Block Body: Output-Zeilen (scrollbar bei langem Output)
  - Subtiler Separator zwischen Blöcken
  - Hover-State: Block-Hintergrund leicht aufhellen
  - Selected-State: Akzent-Border links
- [ ] Exit-Code Badge: Pill (grün ⬤ = 0, rot ✖ = non-zero)
- [ ] Block collapsible (Chevron oder Cmd+Click)
- [ ] Block-Navigation: Cmd+↑/↓ springt zwischen Blöcken
- [ ] Block kopieren: Cmd+C auf selektierten Block kopiert Output
- [ ] Sticky Block-Header beim Scrollen durch langen Output

---

## 2.6 — Input Position (konfigurierbar)

- [ ] Setting: "Pin to bottom" (Default, Warp-Style)
- [ ] Setting: "Pin to top" (Alternative)
- [ ] Setting: "Classic" (Prompt scrollt mit Output, traditionell)
- [ ] CTRL-L: Blocks außer Sichtweite scrollen (Clean View)
- [ ] CTRL-SHIFT-K: Alle Blocks löschen

---

## Milestone

✅ Warp-Style Layout: Output oben scrollbar, Input unten gepinnt
✅ Shell-Hooks senden Block-Boundaries für zsh, bash, fish
✅ Commands und Output als separate visuelle Blöcke
✅ Blöcke haben Header mit Command, Exit-Code, Duration
✅ Blöcke sind collapsible und navigierbar
✅ Input-Editor mit Syntax-Highlighting und Multi-Line
