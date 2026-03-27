# Inazuma: Modernization Migrations

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

---

# Inazuma: HSLA → OKLCH Color Migration

## Ziel

Gesamtes Farbsystem in Inazuma von HSLA auf OKLCH (Oklab Lightness Chroma Hue) umstellen.

## Warum

- OKLCH ist **perceptually uniform** — gleiche Lightness-Werte sehen für das menschliche Auge tatsächlich gleich hell aus (bei HSLA ist `hsl(60°,100%,50%)` deutlich heller als `hsl(240°,100%,50%)`)
- Bessere Farb-Interpolation für Gradienten, Animationen, Opacity-Blending
- Modernster CSS-Standard (CSS Color Level 4, ab 2023 in allen Browsern)
- Design-Token-System (`raijin-ui`) profitiert massiv — Palette-Generierung, Contrast-Checks, Theme-Varianten werden alle einfacher
- Gamut-Mapping für Wide-Gamut Displays (P3) ist mit OKLCH trivial

## Scope

| Bereich | Aufwand |
|---------|---------|
| `inazuma/src/color.rs` — `Oklch` Struct + Konvertierung | Mittel |
| `inazuma/src/styled.rs` — `oklch()` / `oklcha()` Helper | Klein |
| Alle `hsla()` Aufrufe in inazuma + inazuma-component | Groß (viele Stellen) |
| `raijin-app` hardcoded Farben | Klein |
| Theme-System (`ActiveTheme`) | Mittel |

## Vorgehen

1. `Oklch` Struct in `color.rs` definieren (L: 0–1, C: 0–0.4, H: 0–360, A: 0–1)
2. Konvertierung `Oklch ↔ Hsla ↔ Rgba` implementieren
3. `oklch()` und `oklcha()` Constructor-Funktionen
4. Interne Rendering-Pipeline: OKLCH → Linear sRGB → GPU (Metal/wgpu erwartet Linear sRGB)
5. Schrittweise Migration: neue Farben in OKLCH, bestehende Stück für Stück umstellen
6. `Hsla` als Fallback/Compat behalten bis alles migriert ist

## Referenzen

- [OKLCH Color Picker](https://oklch.com)
- [CSS Color Level 4 Spec](https://www.w3.org/TR/css-color-4/#ok-lab)
- [Oklab Paper (Björn Ottosson)](https://bottosson.github.io/posts/oklab/)

---

# Inazuma: mod.rs → {modul}.rs Migration

## Ziel

Alle `mod.rs` Dateien in `crates/inazuma/` durch benannte Modul-Dateien ersetzen (moderne Rust-Convention).

## Warum

- `mod.rs` macht Tabs im Editor ununterscheidbar (alle heißen `mod.rs`)
- Moderne Rust-Convention seit Edition 2018 — `foo.rs` statt `foo/mod.rs`
- Raijin-Projekt-Convention verbietet `mod.rs` bereits (siehe CLAUDE.md)
- Inazuma hat noch viele `mod.rs` aus dem GPUI-Fork

## Scope

```bash
# Alle mod.rs in Inazuma finden:
find crates/inazuma/src -name "mod.rs" | wc -l
```

## Vorgehen

1. Für jedes `src/foo/mod.rs` → umbenennen zu `src/foo.rs`
2. Interne `mod`-Deklarationen anpassen (von `mod foo { mod bar; }` zu `mod foo;`)
3. Sicherstellen dass `cargo test -p inazuma` und `cargo build --workspace` durchlaufen
4. Batch-weise machen (z.B. alle `platform/` auf einmal, dann `elements/`, etc.)

## Hinweis

Rein mechanische Umbenennung — kein Code ändert sich, nur Dateipfade. Git erkennt Renames automatisch.
