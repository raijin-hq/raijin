# Inazuma: cocoa/objc → objc2 Migration

## Ziel

Alle `cocoa` (0.26) und `objc` (0.2) Nutzung in `crates/inazuma/` durch `objc2`, `objc2-app-kit`, `objc2-foundation` ersetzen.

## Warum

- `objc` 0.2 produziert `unexpected_cfgs` Warnings (veraltete Macro-Patterns)
- `cocoa` crate ist unmaintained, kein Upstream mehr
- `objc2` ist der Community-Standard (winit, tauri, etc. haben migriert)
- Typsichere Bindings statt raw `msg_send!` — weniger UB-Risiko

## Scope

| Datei | Zeilen ca. | Aufwand |
|-------|-----------|---------|
| `crates/inazuma/src/platform/mac/window.rs` | ~2800 | Groß — Hauptdatei |
| Weitere `platform/mac/*.rs` | varies | Mittel |
| `raijin-app/src/main.rs` (`set_dock_icon`) | ~20 | Klein |
| Workspace `Cargo.toml` Dependencies | — | Klein |

## Crate-Mapping

| Alt | Neu |
|-----|-----|
| `cocoa` 0.26 | `objc2-app-kit` |
| `cocoa-foundation` 0.2 | `objc2-foundation` |
| `objc` 0.2 | `objc2` |
| `msg_send![obj, method]` | Typisierte Method-Calls |
| `class!(NSWindow)` | `NSWindow::class()` |

## Vorgehen

1. `objc2`, `objc2-app-kit`, `objc2-foundation` als Workspace-Dependencies hinzufügen
2. Datei für Datei in `crates/inazuma/src/platform/mac/` durchgehen
3. `cocoa`/`objc` Imports ersetzen, `msg_send!` durch typisierte Calls
4. Kompilieren, testen (Window-Rendering, Traffic Lights, Resize, Fullscreen)
5. `cocoa`/`objc`/`cocoa-foundation` Dependencies entfernen
6. `raijin-app` Dock-Icon Code auf `objc2` umstellen

## Referenzen

- [objc2 GitHub](https://github.com/madsmtm/objc2)
- [objc2-app-kit docs](https://docs.rs/objc2-app-kit)
- [Migration Tracking Issue #174](https://github.com/madsmtm/objc2/issues/174)
