# Raijin (雷神) — Fortschritt

> GPU-beschleunigter Terminal-Emulator mit Warp-Level UX
> Stack: Rust + Inazuma (稲妻) + alacritty_terminal

---

## Phase 0: Foundation Setup ✅

**Status:** Abgeschlossen (25. März 2026)

- Inazuma (GPU UI-Framework) vendored + umbenannt, kompiliert sauber
- Inazuma-Component (70+ Widgets) vendored + API-Fixes, kompiliert sauber
- Cargo Workspace mit 9 Members, 0 Errors, 0 Warnings
- Metal Toolchain + Shader-Kompilierung funktioniert

---

## Phase 1: Terminal Backend ✅

**Status:** Abgeschlossen (25. März 2026)

- raijin-terminal: PTY + alacritty_terminal 0.26 + Background Read Thread
- TerminalHandle: Cloneable, thread-safe Handle für Resize + Rendering
- TerminalElement: Custom Inazuma Element für Grid-Rendering
- ANSI-Farben (16 + 256 + TrueColor) mit Tokyo Night Palette
- Keyboard Input → PTY Bytes Translation (Arrows, Ctrl, Alt, Special Keys)
- Terminal Grid Resize (Zed-Pattern: jeden Frame neu berechnen)

---

## Phase 2A: Warp-Style Layout 🔨

**Status:** In Arbeit (25. März 2026)

| Task | Status |
|------|--------|
| Root als Window-Infrastruktur (Overlays, Notifications) | ✅ |
| Workspace als App-Layout (3-Zonen: TabBar + Output + Input) | ✅ |
| Dark Theme (inazuma-component ThemeMode::Dark) | ✅ |
| TabBar oben mit Terminal-Titel | ✅ |
| Context Chips (User, CWD, Git Branch) als Warp-Style Pills | ✅ |
| Input Bar unten (inazuma-component Input Widget) | ✅ |
| Terminal Output bottom-grow (wächst von unten nach oben) | ✅ |
| Content Mask Clipping (verhindert Text-Bleeding) | ✅ |
| Beam Cursor (dünn, grün, wie Warp) | ✅ |
| Alt-Screen Detection (vim/htop → Input Bar versteckt) | ✅ |
| Enter → Command an PTY → Output erscheint | ✅ |
| Shell-Kontext (raijin-shell: CWD, Git Branch) | ✅ |
| Text-Bleeding Bug fixen (links rausragende Glyphen) | 🔲 |
| Nerd Font Support (Starship Icons) | 🔲 |
| Terminal Output nur nach erstem Command zeigen | 🔲 → Braucht Shell-Hooks |

---

## Phase 2B: Shell-Integration + Blocks 🔨

**Status:** In Arbeit (25. März 2026)

**Ziel:** Shell-Hooks für Block-Boundaries, Prompt-Unterdrückung, Exit-Codes

| Task | Status |
|------|--------|
| Shell-Hook Scripts (zsh, bash, fish) mit OSC 133 Markern | ✅ |
| OSC 133 Byte-Scanner (osc_parser.rs, 6 Tests) | ✅ |
| PTY Shell-Hook Injection (ZDOTDIR für zsh, --rcfile für bash) | ✅ |
| ShellMarker Events in TerminalEvent Pipeline | ✅ |
| Block-Datenmodell (TerminalBlock + BlockManager) | ✅ |
| Block-Rendering (Header + Body + Exit-Code Badge) | 🔲 ← NÄCHSTER SCHRITT |
| Prompt-Unterdrückung in Raijin Mode (via Block-Filter) | 🔲 |
| PS1 Mode (Shell-Prompt sichtbar) als Alternative | 🔲 |
| Raijin Mode / PS1 Mode Setting | 🔲 |
| Text-Bleeding Bug fixen | 🔲 |
| Block-Navigation (Cmd+↑/↓) | 🔲 |
| Block Collapse/Expand | 🔲 |

---

## Phase 3: Design System 🔲
## Phase 4: Input Editor + Completions 🔲
## Phase 5: File Explorer + Code Editor 🔲
## Phase 6: AI + Agent Toolbar 🔲
## Phase 7–9: Drive, Polish, Future 🔲
