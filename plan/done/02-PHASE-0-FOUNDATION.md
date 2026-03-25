# Phase 0: Foundation Setup (Woche 1)

> **Ziel:** Repo aufsetzen, gpui-ce vendoren, alles kompiliert

---

## 0.1 — Repository & Toolchain

- [ ] Fork `gpui-ce/gpui-ce` → eigenes Repo `nyxb/raijin`
- [ ] Rust Toolchain: latest stable (`rustup update stable`)
- [ ] Xcode Command Line Tools installieren (Metal-Rendering auf macOS)
- [ ] gpui-ce als lokales Package vendoren (NICHT als git dependency!)
- [ ] gpui-component ebenfalls vendoren
- [ ] Projekt-Struktur als Cargo Workspace anlegen (siehe 00-OVERVIEW.md)

**Warum vendored statt git dependency:**
gpui-ce und gpui-component werden direkt ins Monorepo kopiert als lokale crates.
So können wir Shader, Rendering-Primitives, Styles und Widgets direkt editieren
ohne auf upstream angewiesen zu sein. Das Design divergiert sofort — Upstream-Syncs
passieren nur noch als gezielte Cherry-Picks für Bugfixes.

```toml
# Cargo.toml (workspace root)
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.dependencies]
gpui = { path = "crates/gpui" }
gpui-component = { path = "crates/gpui-component" }
raijin-ui = { path = "crates/raijin-ui" }
raijin-terminal = { path = "crates/raijin-terminal" }
raijin-shell = { path = "crates/raijin-shell" }
raijin-editor = { path = "crates/raijin-editor" }
raijin-agent = { path = "crates/raijin-agent" }
raijin-drive = { path = "crates/raijin-drive" }
```

---

## 0.2 — gpui-ce verifizieren

- [ ] gpui-ce clonen und Example-App kompilieren (`cargo run --example`)
- [ ] Sicherstellen dass Metal-Rendering auf deinem Mac läuft
- [ ] gpui-component Gallery starten und alle Widgets durchklicken
- [ ] Verstehen wie `div()`, `.bg()`, `.rounded()`, `.shadow()` etc. funktionieren

---

## 0.3 — Dependencies festlegen

```toml
# Cargo.toml — zusätzliche externe Dependencies
[workspace.dependencies]
alacritty_terminal = "0.24"    # Terminal Grid, VTE Parser, PTY
vte = "0.15"                   # ANSI Escape Sequence Parser
cosmic-text = "0.12"           # Text Shaping & Font Fallback
glyphon = "0.6"                # GPU Text Rendering (wgpu)
portable-pty = "0.8"           # Cross-platform PTY
tree-sitter = "0.24"           # Syntax Highlighting
tree-sitter-bash = "0.23"      # Shell Syntax
serde = { version = "1", features = ["derive"] }
toml = "0.8"                   # Config/Theme Dateien
notify = "7"                   # Filesystem Watcher (für File Explorer)
fuzzy-matcher = "0.3"          # Fuzzy Search (Command Palette, History)
```

---

## Milestone

✅ `cargo build` kompiliert den gesamten Workspace ohne Fehler
✅ gpui-ce Example-App rendert ein Fenster mit Metal
✅ Alle Crate-Stubs existieren und sind verlinkt
