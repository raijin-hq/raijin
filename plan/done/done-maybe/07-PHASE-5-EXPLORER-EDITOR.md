# Phase 5: File Explorer + Code Editor + Panels (Woche 11–14)

> **Ziel:** Vollständige Desktop-App mit Explorer, Editor und Split Panes

---

## 5.1 — File Explorer / Project Explorer

- [ ] Tree-View mit lazy loading (nur sichtbare Nodes laden)
- [ ] Drag & Drop (Files verschieben)
- [ ] Rename inline (F2 oder slow double-click)
- [ ] Context-Menu (Rechtsklick: New File, New Folder, Delete, Rename, Copy Path)
- [ ] .gitignore awareness (ignorierte Files ausgegraut oder hidden)
- [ ] Hidden Files Toggle (Cmd+Shift+.)
- [ ] File-Icons (nach Extension: .rs, .ts, .py, .md etc.)
- [ ] Drag File-Paths in Terminal-Commands (Explorer → Input Editor)
- [ ] Filesystem Watcher (`notify` crate) für Live-Updates

---

## 5.2 — Code Editor (Warp Code Equivalent)

- [ ] Nativer Editor mit Tabs (mehrere Files gleichzeitig offen)
- [ ] Syntax Highlighting via tree-sitter (Rust, JS/TS, Python, Go, Shell, YAML, TOML, JSON, Markdown)
- [ ] Go to Line (CTRL-G)
- [ ] Real-time Diff Tracking (geänderte Zeilen markiert vs. Git HEAD)
- [ ] Code Review Panel: Accept/Reject/Edit Diffs inline
- [ ] Open Files from Explorer in Editor (Doppelklick oder Enter)
- [ ] External Editor Integration: "Open in VS Code/Zed/Cursor" Button
- [ ] Lightweight — bewusst kein vollständiger IDE-Ersatz

---

## 5.3 — Split Panes

- [ ] Horizontal Split (Cmd+D)
- [ ] Vertical Split (Cmd+Shift+D)
- [ ] Resize via Drag auf Separator
- [ ] Focus-Wechsel via Cmd+Alt+Arrow
- [ ] Pane schließen (Cmd+W)

---

## 5.4 — Command Palette + Quake Mode + Markdown Viewer

- [ ] Command Palette (Cmd+P): Fuzzy Search über alle Actions
- [ ] Quake Mode: Dedicated Hotkey (z.B. Ctrl+`) öffnet/schließt Raijin als Overlay
- [ ] Markdown Viewer: .md Files rendern mit ausführbaren Code-Blocks (Click to Run)

---

## Milestone

✅ File Explorer zeigt Projekt-Tree mit Live-Updates
✅ Code Editor öffnet Files mit Syntax Highlighting
✅ Split Panes funktionieren horizontal und vertikal
✅ Command Palette durchsucht alle verfügbaren Actions
