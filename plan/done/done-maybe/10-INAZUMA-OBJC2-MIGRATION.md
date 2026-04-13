# Inazuma: Modernization Migrations — ✅ ERLEDIGT (2026-04-05)

## Status: KOMPLETT

Erledigt auf Branch `refactor/inazuma-objc2-migration`:
- **objc2:** cocoa/objc komplett entfernt, 183/228 msg_send! → typed methods, 45 bleiben (super-init/private APIs)
- **OKLCH:** Hsla komplett eliminiert (814→0 Stellen), Oklch als einziger Farbtyp
- **P3 Wide Gamut:** sRGB auf CAMetalLayer, Gamut-Mapping, WindowColorspace Config
- **mod.rs:** 2 Dateien umbenannt

## Ziel (Original)

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

# Inazuma: Wide Gamut Display P3 Support

## Ziel

Metal Rendering Pipeline in Inazuma color-managed machen. Display P3 als konfigurierbaren Farbraum unterstützen — Raijin wäre damit neben Ghostty das einzige Terminal mit echtem Wide Gamut Support.

## Warum

- Jeder Mac seit 2016 hat ein **Display P3 Panel** (~25% mehr Farbraum als sRGB, besonders Rot/Grün/Orange)
- **Aktuell kein Color Management** in Inazuma: `CAMetalLayer` hat kein `colorspace` gesetzt → Raw-Werte gehen direkt zum Display → leichte Übersättigung auf P3 Displays
- Kein anderes Terminal außer Ghostty und iTerm2 hat P3 Support
- OKLCH macht Gamut-Mapping trivial (Chroma-Clipping)

## Stand der Konkurrenz

| Terminal | P3 Support | Wie |
|----------|-----------|-----|
| **Ghostty** | Ja | `window-colorspace` Config (p3/srgb/native) |
| **iTerm2** | Ja | Profil-Ebene, proprietäre Escape Codes (`p3:RRGGBB`) |
| **Kitty** | Nein | — |
| **Alacritty** | Nein | — |
| **WezTerm** | Nein | — |
| **Rio** | Nein | — |
| **Warp** | Nein | — |

Es gibt **keinen Standard** für P3-Farben in Terminal Escape Sequences. `ESC[38;2;r;g;b;m` hat keinen Colorspace-Parameter (ITU T.416 definiert einen, aber kein Terminal implementiert ihn). Die Lösung ist Window-Level Konfiguration.

## Aktueller Zustand in Inazuma

**`metal_renderer.rs` (Zeile ~147-172):**
```rust
layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);  // 8-bit, kein expliziter Colorspace
// KEIN layer.set_colorspace(...)
// KEIN window.set_color_space(...)
```

**`shaders.metal` (Zeile ~937-941):**
```metal
float3 srgb_to_linear(float3 color) { return pow(color, float3(2.2)); }
float3 linear_to_srgb(float3 color) { return pow(color, float3(1.0 / 2.2)); }
// Kein P3 Support
```

**`text_system.rs`:** Nutzt `CGColorSpace::create_device_rgb()` für Glyph-Rasterisierung — device-abhängig statt sRGB-getagged.

Zed hat **exakt den gleichen Zustand** — kein Color Management.

## Implementierung

### 1. `CAMetalLayer.colorspace` setzen (`metal_renderer.rs`)

```rust
// Aus Config lesen
match config.window_colorspace {
    WindowColorspace::P3 => {
        // CGColorSpace(name: kCGColorSpaceDisplayP3)
        let p3 = CGColorSpace::create_with_name(kCGColorSpaceDisplayP3);
        layer.set_colorspace(p3);
    }
    WindowColorspace::Srgb => {
        // Explizit sRGB — verhindert Übersättigung auf P3 Displays
        let srgb = CGColorSpace::create_with_name(kCGColorSpaceSRGB);
        layer.set_colorspace(srgb);
    }
    WindowColorspace::Native => {
        // OS-Default: P3 auf macOS, sRGB auf Linux
        // Kein expliziter Colorspace → OS entscheidet
    }
}
```

**Wichtig:** Allein das explizite Setzen von sRGB als Default fixt schon die aktuelle Übersättigung auf P3 Displays. Das ist ein Quick Win.

### 2. Pixel-Format evaluieren

| Format | Bits | Gamut | Anmerkung |
|--------|------|-------|-----------|
| `BGRA8Unorm` | 8-bit | sRGB | Aktuell, kein Gamma-Handling |
| `BGRA8Unorm_sRGB` | 8-bit | sRGB | Explizites sRGB Gamma in Hardware |
| `rgba16Float` | 16-bit float | Extended Range P3/HDR | Mehr VRAM, aber voller P3 Gamut + HDR |
| `bgra10_xr_srgb` | 10-bit | Extended Range sRGB | Apple XR Format, guter Kompromiss |

Empfehlung: `BGRA8Unorm_sRGB` als Default, `rgba16Float` optional für P3 Mode.

### 3. Shader P3 Output

Wenn P3 aktiv, müssen sRGB-Eingabewerte (von SGR Escape Sequences) in P3 konvertiert werden:

```metal
// sRGB Primaries → P3 Primaries (3x3 Matrix)
constant float3x3 srgb_to_p3_matrix = float3x3(
    float3(0.8225, 0.1774, 0.0000),
    float3(0.0332, 0.9669, 0.0000),
    float3(0.0171, 0.0724, 0.9108)
);

float3 srgb_to_display_p3(float3 srgb) {
    float3 linear = srgb_to_linear(srgb);
    float3 p3_linear = srgb_to_p3_matrix * linear;
    return linear_to_srgb(p3_linear);  // P3 transfer function ≈ sRGB
}
```

### 4. SGR Truecolor Handling

- `38;2;r;g;b` Werte = **immer sRGB** (de-facto Convention, kein Standard definiert Colorspace)
- Bei P3 Mode: sRGB → P3 Konvertierung im Shader
- Theme-Farben (aus OKLCH) können den vollen P3-Gamut nutzen — Chroma-Werte >0.25 erreichen Farben die in sRGB nicht darstellbar sind

### 5. OKLCH Gamut-Mapping (Verbindung zur OKLCH-Migration)

```rust
impl Oklch {
    /// Clamp Chroma auf den maximalen Wert im Ziel-Gamut
    pub fn clamp_to_srgb(self) -> Self { /* C reduzieren bis in sRGB Gamut */ }
    pub fn clamp_to_p3(self) -> Self { /* C reduzieren bis in P3 Gamut */ }
    pub fn in_srgb_gamut(&self) -> bool { /* Check ob darstellbar */ }
    pub fn in_p3_gamut(&self) -> bool { /* Check ob darstellbar */ }
}
```

Theme-Autoren können `oklch(0.7 0.35 150)` definieren — ein Grün das in sRGB nicht darstellbar ist, aber auf P3 Displays leuchtet. Auf sRGB-Displays wird automatisch auf den nächsten darstellbaren Wert geclippt.

## Config-Option (in `raijin-settings`)

```toml
[appearance]
# "p3" — Display P3 (voller Gamut, lebendigere Farben)
# "srgb" — Standard sRGB (kompatibel, keine Übersättigung)
# "native" — OS-Default (P3 auf macOS, sRGB auf Linux)
window_colorspace = "native"
```

## Tasks

- [ ] `CAMetalLayer.colorspace` in `metal_renderer.rs` explizit auf sRGB setzen (Quick Win — fixt Übersättigung)
- [ ] `WindowColorspace` Enum in Inazuma definieren (P3, Srgb, Native)
- [ ] P3 Mode: `CGColorSpace.displayP3` wenn konfiguriert
- [ ] Pixel-Format: `BGRA8Unorm_sRGB` als neuer Default evaluieren
- [ ] Pixel-Format: `rgba16Float` für P3 Mode evaluieren
- [ ] Shader: sRGB → P3 Konvertierungsmatrix
- [ ] `text_system.rs`: `CGColorSpace::create_device_rgb()` → explizit sRGB oder P3
- [ ] OKLCH `clamp_to_srgb()` / `clamp_to_p3()` Gamut-Mapping
- [ ] Config-Option `window_colorspace` in `raijin-settings`
- [ ] Testen: P3 Display (internes MacBook) vs. externer sRGB Monitor

## Referenzen

- [WWDC 2016 Session 712: Working with Wide Color](https://developer.apple.com/videos/play/wwdc2016/712/)
- [Apple CGColorSpace Documentation](https://developer.apple.com/documentation/coregraphics/cgcolorspace)
- [Ghostty window-colorspace Config](https://ghostty.org/docs/config/reference)
- [Ghostty P3 Discussion #2665](https://github.com/ghostty-org/ghostty/discussions/2665)
- [Ghostty PR #4913: Metal alpha blending + color handling](https://github.com/ghostty-org/ghostty/pull/4913)
- [Servo PR: Use sRGB colorspace on macOS](https://github.com/servo/servo/pull/35683)
- [GLFW Issue #2748: sRGB colour space on macOS](https://github.com/GLFW/glfw/issues/2748)

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
