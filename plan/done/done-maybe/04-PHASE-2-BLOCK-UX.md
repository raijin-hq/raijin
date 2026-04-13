# Phase 2: Block-UX + Warp-Style Layout

> **Ziel:** Warp-Level Layout mit Bottom-Input, Block-System, Shell-Integration

---

## Architektur-Entscheidung: Warp-Style Layout

Warp trennt den Input komplett von der Shell. Der Input-Editor lebt im Terminal, nicht im PTY. Das ermöglicht:
- Input unten gepinnt (fixiert), Output scrollt oben
- Volle IDE-Features im Input (Syntax-Highlighting, Multi-Cursor, Completions)
- Jeder Command+Output ist ein isolierter "Block" mit eigenem Grid

### Window-Layout:

```
┌─────────────────────────────────────────┐
│ Tab Bar (Tabs mit Farb-Indikatoren)     │  ← Fixed top
├─────────────────────────────────────────┤
│  Scrollable Block Area                  │  ← Blocks fließen nach oben
│  ┌─ Block ────────────────────────┐     │
│  │ nyxb host ~ 17:33 (0.032s)    │     │  ← Metadata (dimmed)
│  │ git status                     │     │  ← Command (bright)
│  │ On branch main                 │     │  ← Block Output
│  └────────────────────────────────┘     │
│  ┌─ Block (error) ───────────────┐     │
│  │▌nyxb host ~ 17:34 (12.100s)  │     │  ← Red bg + left border
│  │▌cargo build                   │     │
│  │▌error[E0308]: mismatched types│     │
│  └────────────────────────────────┘     │
├─────────────────────────────────────────┤
│ Context Chips: nyxb │ host │ 📁~ │ 🕐  │  ← Dynamic via OSC 7777
│ ▶ _                                    │  ← Input Editor (fixed bottom)
└─────────────────────────────────────────┘
```

---

## Erledigt ✅

### 2.1 — Warp-Style Window Layout
- [x] Drei-Zonen Layout: Tab Bar (top) → Block Area (middle) → Input Bar (bottom)
- [x] Input-Editor als Inazuma-Component am unteren Rand
- [x] Block Area als bottom-grow Container (flex-1)
- [x] Context Chips dynamisch via OSC 7777 Metadata
- [x] Layout responsive: Block Area füllt verfügbaren Platz

### 2.2 — Input Editor (Basics)
- [x] Eigener Text-Editor am unteren Rand (nicht Shell's readline)
- [x] Bei Enter: Buffer an PTY senden
- [x] Context via Chips (Username, Hostname, CWD, Git, Time)

### 2.3 — Shell-Integration
- [x] Shell-Hooks für zsh, bash, fish (OSC 133 + OSC 7777)
- [x] OscScanner Statemachine parst beide OSC-Typen
- [x] JSON Metadata hex-encoded (Warp-Strategie gegen ST-Terminator)
- [x] Shell-gemessene Duration (EPOCHREALTIME/date +%s%3N)
- [x] Graceful Degradation: PS1 Mode als Fallback

### 2.4 — Block-Datenmodell
- [x] TerminalBlock mit id, command, exit_code, rows, timing, metadata_json
- [x] BlockManager: Block-Lifecycle via OSC 133 Markers
- [x] hidden_prompt_regions: Renderer-side Prompt Suppression
- [x] Metadata-Snapshot pro Block für Header-Rendering

### 2.5 — Block-Rendering (Core)
- [x] Warp-style 2-Zeilen Header: Metadata + Command
- [x] Error-Styling: transparenter roter BG + linker Border (4px)
- [x] Prompt Suppression shell-agnostisch (Starship, P10k, anything)
- [x] No-Output-Blocks: nur Header, keine Gap
- [x] Tight stacking: Block → Input-Area ohne Lücke

### 2.6 — Input Position
- [x] Pin to bottom (Default, Warp-Style)

---

## Offen — Block Interaction

### Block-Selection & Navigation
- [ ] Klick auf Block → Highlight-Tint über gesamten Block
- [ ] Markierter Block zeigt Action-Icons rechts oben
- [ ] Cmd+↑/↓ — Navigation zwischen Blöcken
- [ ] Selected-State: Akzent-Border links (grün)

### Block-Aktionen
- [ ] Copy (Cmd+C) — gesamten Block
- [ ] Copy command (Shift+Cmd+C)
- [ ] Copy output (Alt+Shift+Cmd+C)
- [ ] Find within block (Cmd+F)
- [ ] Toggle bookmark (Cmd+B)

### Block Collapse
- [ ] Chevron oder Cmd+Click toggled Block-Body
- [ ] Collapsed: nur Header sichtbar

### Sticky Block-Header
- [ ] Bei langem Output: Header pinnt an oberen Viewport-Rand
- [ ] Klick scrollt zum Block-Start

### Action-Icons (rechts im Header, on hover)
- [ ] 📎 Attach as agent context
- [ ] ⬇ Download/Save output
- [ ] 🔍 Filter block output
- [ ] ⋮ More (Context-Menu)

### Visual States
- [ ] Hover: Block-Hintergrund leicht aufhellen
- [ ] Running: Puls-Animation im Header

### Keyboard Shortcuts
- [ ] CTRL-L: Blocks aus Sichtweite scrollen
- [ ] CTRL-SHIFT-K: Alle Blocks löschen

---

## Offen — Input Editor (Advanced)

- [ ] Multi-Line Support (Shift+Enter)
- [ ] Syntax-Highlighting für Shell-Commands
- [ ] History Navigation (Pfeil hoch/runter)
- [ ] Setting: Pin to top / Classic Mode
