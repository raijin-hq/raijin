# Phase 22: Alle mod.rs eliminieren — moderne Rust Convention

## Ziel

Jede `mod.rs` im gesamten Repo durch `{modul}.rs` ersetzen. Moderne Rust Convention seit Edition 2018 — kein `mod.rs` mehr, nirgendwo.

## Regel (aus CLAUDE.md)

> **No `mod.rs`** — use `module_name.rs` (modern Rust convention)

## Scope

Das **gesamte Repo** wird gescannt, nicht nur `crates/`:

```bash
find /Users/nyxb/Projects/raijin/ -name "mod.rs" \
  -not -path "*/.reference/*" \
  -not -path "*/target/*" \
  -not -path "*/.git/*"
```

## Aktuell bekannte mod.rs (Stand 2026-04-08)

| Datei | Crate |
|---|---|
| `crates/raijin-call/src/call_impl/mod.rs` | raijin-call |
| `crates/raijin-eval/src/examples/mod.rs` | raijin-eval |
| `crates/raijin-keymap-editor/src/ui_components/mod.rs` | raijin-keymap-editor |
| `crates/raijin-agent/src/tests/mod.rs` | raijin-agent |
| `crates/raijin-repl/src/kernels/mod.rs` | raijin-repl |

## Prozess pro Datei

Für jede `src/foo/mod.rs`:

1. Wenn `src/foo/` weitere Dateien enthält (Submodule):
   - `mv src/foo/mod.rs src/foo.rs`
   - Die Submodule in `src/foo/` bleiben wo sie sind
   - Rust findet sie automatisch über `src/foo.rs` + `src/foo/`

2. Wenn `src/foo/mod.rs` das einzige File in `src/foo/` ist:
   - `mv src/foo/mod.rs src/foo.rs`
   - `rmdir src/foo/`

3. `cargo check -p CRATE_NAME` muss weiterhin kompilieren

## Abschluss-Check

Nach allen Fixes nochmal den Scan laufen lassen und sicherstellen: **0 Ergebnisse**.
