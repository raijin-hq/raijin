# Phase 1: Minimal Terminal (Woche 2–3)

> **Ziel:** Ein Fenster das eine Shell rendert — ugly but functional

---

## 1.1 — GPUI Window + Basic Layout

- [ ] `raijin-app` Binary erstellen mit `Application::new()` + `open_window()`
- [ ] Root-View mit Sidebar (links) + Terminal-Area (rechts) als Grundlayout
- [ ] Sidebar erstmal leer, nur als farbiger Rect mit fester Breite
- [ ] Terminal-Area als scrollbarer Bereich
- [ ] Tab-Bar (oben) mit einem Tab

---

## 1.2 — alacritty_terminal Integration

- [ ] `alacritty_terminal::Term` initialisieren mit Standard-Config
- [ ] PTY spawnen (`portable-pty`) mit Default-Shell (`$SHELL` oder `/bin/zsh`)
- [ ] Event-Loop: PTY-Output → `Term::advance()` → Grid-State updaten
- [ ] Keyboard-Input von GPUI-Window an PTY forwarden

---

## 1.3 — Terminal Grid Rendering

- [ ] `Term.renderable_content()` auslesen → Cell-Grid mit Chars + Farben
- [ ] Custom GPUI Element bauen (`TerminalElement`) das das Grid rendert
- [ ] Monospace-Font laden (Input Mono oder JetBrains Mono)
- [ ] Jede Cell als positioned Text-Glyph rendern
- [ ] ANSI-Farben (16 + 256 + TrueColor) korrekt mappen
- [ ] Cursor rendern (Block, Beam, Underline)
- [ ] Scrollback-Buffer implementieren (Mouse-Scroll, Shift+PageUp)

---

## 1.4 — Basis-Interaktion

- [ ] Text-Selection mit Maus (Click + Drag)
- [ ] Copy/Paste (Cmd+C / Cmd+V)
- [ ] Resize: Window-Resize → PTY-Resize → Grid-Resize
- [ ] Shell-Kompatibilität testen: zsh, bash, fish

---

## Milestone

✅ `cargo run -p raijin-app` startet ein Fenster mit funktionierender Shell
✅ Man kann Commands eingeben und Output sehen
✅ Farben und Cursor werden korrekt dargestellt
