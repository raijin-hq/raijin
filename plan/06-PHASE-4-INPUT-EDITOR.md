# Phase 4: IDE-Style Input Editor + Completions (Woche 9–11)

> **Ziel:** Input-Editor fühlt sich wie ein Mini-IDE an

---

## 4.1 — Rich Input Editor

- [ ] Multi-Line Input mit Syntax-Highlighting
- [ ] Mouse-Cursor mit Click-to-Position
- [ ] Multi-Cursor Support (Cmd+D, Alt+Click)
- [ ] Vim Keybindings (togglebar via Setting)
- [ ] Custom Prompt mit Context-Chips (CWD, Git Branch, User)
- [ ] PS1/Starship/P10k Support (bestehende Prompt-Configs respektieren)

---

## 4.2 — Smart Completions

- [ ] File/Path Completion (Tab-Trigger)
- [ ] Command Completion (aus $PATH)
- [ ] Git-Branch Completion (bei git checkout, git switch etc.)
- [ ] History Completion (Frecency-basiert)
- [ ] Ghost-Text (inline Vorschlag, grau, Tab zum Akzeptieren)
- [ ] CLI-Specs für 400+ populäre Tools (Subcommands, Flags, Args)

---

## 4.3 — Command Corrections + Shell Selector

- [ ] Typo-Erkennung bei Exit Code != 0 (z.B. `gti` → `git`)
- [ ] Missing Parameter Vorschläge
- [ ] Shell-Dropdown für schnellen Wechsel (zsh ↔ bash ↔ fish)

---

## Milestone

✅ Multi-Line Input mit Syntax-Highlighting funktioniert
✅ Tab-Completion zeigt kontextuelle Vorschläge
✅ Ghost-Text erscheint basierend auf History
