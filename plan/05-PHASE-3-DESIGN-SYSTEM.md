# Phase 3: Design System — Von "funktional" zu "Warp-Level"

> **Ziel:** Das Ding muss GEIL aussehen

---

## Architektur-Übersicht

### Crate: `raijin-ui`

Das Design-Token-System lebt in `crates/raijin-ui/`. Es baut auf Inazuma (unserem GPUI-Fork) auf und stellt typisierte Farb-Tokens, Theme-Loading und semantische Styles bereit.

```
raijin-ui/src/
├── lib.rs            # Re-exports
├── color.rs          # Oklch Struct + Multi-Format Parsing + Konvertierungen
├── theme.rs          # Theme, ThemeFamily, Appearance, GlobalTheme
├── colors.rs         # ThemeColors — semantische Tokens (100+ Felder)
├── status.rs         # StatusColors (error, warning, success, info, conflict...)
├── syntax.rs         # SyntaxTheme (Terminal ANSI-Farben + Highlight-Styles)
├── scale.rs          # ColorScale (12-Step, OKLCH-basiert, perceptually uniform)
├── players.rs        # PlayerColors (für zukünftiges Multiplayer/Pair-Programming)
└── registry.rs       # ThemeRegistry (TOML laden, cachen, Hot-Reload)
```

### Abhängigkeit auf Inazuma

`Oklch` als Struct wird **in Inazuma's `color.rs`** definiert (neben `Rgba` und `Hsla`), weil es ein Framework-Primitiv ist. `raijin-ui` baut darauf auf mit semantischen Tokens und Theme-Logik.

---

## Farbraum: OKLCH statt HSLA

### Warum OKLCH

Zed nutzt intern `Hsla` — wir migrieren komplett auf **OKLCH** (Oklab Lightness Chroma Hue).

| Problem mit HSLA | Lösung durch OKLCH |
|---|---|
| `hsl(60°,100%,50%)` (Gelb) wirkt deutlich heller als `hsl(240°,100%,50%)` (Blau) bei gleichem L-Wert | Gleiche L-Werte = tatsächlich gleiche wahrgenommene Helligkeit |
| Gradienten/Interpolation erzeugt Matsch-Farben (z.B. Grau-Bereich bei Blau→Gelb) | Perceptually uniforme Interpolation, saubere Übergänge |
| Palette-Generierung erfordert manuelle Korrekturen pro Hue | Lightness-Stufen sind trivial: L von 0.15 → 0.95 in N Schritten |
| Kein Gamut-Mapping für Wide-Gamut (P3) Displays | OKLCH trennt Gamut (Chroma) von Helligkeit — P3-Mapping trivial |
| Contrast-Checks sind ungenau | L-Differenz ≈ wahrgenommener Kontrast |

### Referenzen

- [OKLCH Color Picker](https://oklch.com) — Evil Martians
- [CSS Color Level 4 Spec](https://www.w3.org/TR/css-color-4/#ok-lab)
- [Oklab Paper (Björn Ottosson)](https://bottosson.github.io/posts/oklab/)

---

## Schritt 1: `Oklch` in Inazuma (`color.rs`)

### Neuer Struct in `crates/inazuma/src/color.rs`

```rust
/// An OKLCH color (Oklab Lightness Chroma Hue)
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C)]
pub struct Oklch {
    /// Perceived lightness, range 0.0 (black) to 1.0 (white)
    pub l: f32,
    /// Chroma (colorfulness), range 0.0 (gray) to ~0.4 (most vivid)
    pub c: f32,
    /// Hue angle in degrees, range 0.0 to 360.0
    pub h: f32,
    /// Alpha, range 0.0 (transparent) to 1.0 (opaque)
    pub a: f32,
}
```

### Konvertierungs-Pipeline

```
                    ┌─────────────────────────────────┐
User Input          │        Inazuma Intern            │      GPU
──────────────      │  ┌───────┐     ┌──────────────┐ │   ┌────────────┐
"#14F195"       ──┐ │  │       │     │              │ │   │            │
"rgb(20,241,149)"─┼─┼─►│ Oklch │────►│ Linear sRGB  │─┼──►│   Metal    │
"oklch(0.88       │ │  │       │     │   (Rgba)     │ │   │   /wgpu    │
  0.2 160)"    ───┘ │  └───────┘     └──────────────┘ │   └────────────┘
                    └─────────────────────────────────┘
```

**Konvertierungskette:**
1. `Oklch → Oklab → Linear sRGB → Rgba` (für GPU-Rendering)
2. `Rgba → Linear sRGB → Oklab → Oklch` (für Import von Hex/RGB)
3. `Hsla ↔ Oklch` (über Rgba als Zwischenschritt, für Compat)

### Multi-Format Parsing

Der Color-Parser in Inazuma akzeptiert alle gängigen Formate und konvertiert intern zu `Oklch`:

```rust
// Alle diese Formate werden unterstützt:
"#14F195"                    // Hex (RGB)
"#14F195ff"                  // Hex (RGBA)
"rgb(20, 241, 149)"          // CSS rgb()
"rgba(20, 241, 149, 0.8)"   // CSS rgba()
"oklch(0.88 0.2 160)"       // OKLCH
"oklch(0.88 0.2 160 / 0.5)" // OKLCH mit Alpha
"hsl(152, 92%, 51%)"        // HSL (Legacy-Compat)
```

**Wichtig:** Hex bleibt das primäre User-facing Format. Die meisten User kennen und nutzen Hex — OKLCH ist optional für Power-User die perceptually uniforme Paletten bauen wollen. Wie der [oklch-picker](https://oklch.com) zeigt: Input in jedem Format, intern OKLCH.

### Blending & Interpolation in OKLCH

Zed blendet Farben in HSLA (über Rgba-Konvertierung) — das erzeugt perceptual Artefakte. Wir machen Blending direkt in Oklch:

```rust
impl Oklch {
    pub fn blend(self, other: Oklch) -> Oklch {
        // Interpolation in Oklch-Space = perceptually korrekt
        // Kein Umweg über Rgba nötig
    }
}
```

### Was von Zed bleibt (Compat)

- `Rgba` — bleibt als GPU-Output-Format und für Hex-Serialisierung
- `Hsla` — bleibt für Legacy-Compat, wird aber intern nicht mehr für Blending/Interpolation genutzt
- `hsla()` Constructor — bleibt verfügbar, konvertiert intern sofort zu Oklch
- Hex-Parsing auf `Rgba` — bleibt, wird um Oklch-Konvertierung erweitert

---

## Schritt 2: Semantische Farb-Tokens (`raijin-ui/colors.rs`)

### `ThemeColors` Struct

Inspiriert von Zeds `ThemeColors`, aber alle Felder sind `Oklch` statt `Hsla`:

```rust
pub struct ThemeColors {
    // Backgrounds
    pub background: Oklch,
    pub surface: Oklch,
    pub elevated_surface: Oklch,
    pub element_background: Oklch,
    pub element_hover: Oklch,
    pub element_active: Oklch,
    pub element_selected: Oklch,
    pub ghost_element_hover: Oklch,
    pub ghost_element_active: Oklch,
    pub ghost_element_selected: Oklch,
    pub drop_target: Oklch,

    // Borders
    pub border: Oklch,
    pub border_variant: Oklch,
    pub border_focused: Oklch,
    pub border_selected: Oklch,
    pub border_transparent: Oklch,
    pub border_disabled: Oklch,

    // Text
    pub text: Oklch,
    pub text_muted: Oklch,
    pub text_placeholder: Oklch,
    pub text_disabled: Oklch,
    pub text_accent: Oklch,

    // Icons
    pub icon: Oklch,
    pub icon_muted: Oklch,
    pub icon_disabled: Oklch,
    pub icon_accent: Oklch,

    // Terminal-spezifisch
    pub terminal_background: Oklch,
    pub terminal_foreground: Oklch,
    pub terminal_ansi_black: Oklch,
    pub terminal_ansi_red: Oklch,
    pub terminal_ansi_green: Oklch,
    pub terminal_ansi_yellow: Oklch,
    pub terminal_ansi_blue: Oklch,
    pub terminal_ansi_magenta: Oklch,
    pub terminal_ansi_cyan: Oklch,
    pub terminal_ansi_white: Oklch,
    pub terminal_ansi_bright_black: Oklch,
    pub terminal_ansi_bright_red: Oklch,
    pub terminal_ansi_bright_green: Oklch,
    pub terminal_ansi_bright_yellow: Oklch,
    pub terminal_ansi_bright_blue: Oklch,
    pub terminal_ansi_bright_magenta: Oklch,
    pub terminal_ansi_bright_cyan: Oklch,
    pub terminal_ansi_bright_white: Oklch,

    // Workspace
    pub title_bar_background: Oklch,
    pub status_bar_background: Oklch,
    pub tab_bar_background: Oklch,
    pub tab_active_background: Oklch,
    pub tab_inactive_background: Oklch,
    pub input_background: Oklch,
    pub input_border: Oklch,

    // Block-System (Raijin-spezifisch)
    pub block_header_background: Oklch,
    pub block_header_hover: Oklch,
    pub block_success_badge: Oklch,
    pub block_error_badge: Oklch,
    pub block_running_badge: Oklch,

    // Scrollbar
    pub scrollbar_track: Oklch,
    pub scrollbar_thumb: Oklch,
    pub scrollbar_thumb_hover: Oklch,
}
```

### `StatusColors`

```rust
pub struct StatusColors {
    pub conflict: Oklch,
    pub conflict_background: Oklch,
    pub conflict_border: Oklch,
    pub created: Oklch,
    pub created_background: Oklch,
    pub created_border: Oklch,
    pub deleted: Oklch,
    pub deleted_background: Oklch,
    pub deleted_border: Oklch,
    pub error: Oklch,
    pub error_background: Oklch,
    pub error_border: Oklch,
    pub info: Oklch,
    pub info_background: Oklch,
    pub info_border: Oklch,
    pub modified: Oklch,
    pub modified_background: Oklch,
    pub modified_border: Oklch,
    pub success: Oklch,
    pub success_background: Oklch,
    pub success_border: Oklch,
    pub warning: Oklch,
    pub warning_background: Oklch,
    pub warning_border: Oklch,
}
```

---

## Schritt 3: 12-Step Color Scales (`raijin-ui/scale.rs`)

OKLCH-basierte Farbskalen — perceptually uniforme Stufen statt Zeds HSLA-Scales:

```rust
pub struct ColorScale {
    colors: [Oklch; 12],
}

pub enum ColorScaleStep {
    ONE = 0,    // Hintergrund (dunkelste Stufe in Dark-Themes)
    TWO,        // Subtiler Hintergrund
    THREE,      // UI Element Background
    FOUR,       // Hover State
    FIVE,       // Active/Selected State
    SIX,        // Subtile Borders
    SEVEN,      // UI Element Border
    EIGHT,      // Hover Border
    NINE,       // Solid Background (z.B. Buttons)
    TEN,        // Solid Hover
    ELEVEN,     // Low-Contrast Text
    TWELVE,     // High-Contrast Text
}
```

**Vorteil gegenüber Zed:** Bei OKLCH reicht es, L linear von ~0.15 bis ~0.95 zu skalieren bei konstantem C und H — die Stufen sehen dann tatsächlich gleichmäßig verteilt aus. Bei HSLA müsste man pro Hue manuell korrigieren.

```rust
impl ColorScale {
    /// Generiert eine 12-Step Scale aus einer Basis-Farbe
    pub fn from_base(base: Oklch) -> Self {
        // L gleichmäßig verteilen, C leicht variieren
        // Step 1: L=0.13, Step 12: L=0.95
        // Hue bleibt konstant
    }
}
```

---

## Schritt 4: Theme-System (`raijin-ui/theme.rs`)

### Strukturen

```rust
pub enum Appearance {
    Light,
    Dark,
}

pub struct Theme {
    pub id: String,
    pub name: SharedString,
    pub appearance: Appearance,
    pub styles: ThemeStyles,
}

pub struct ThemeStyles {
    pub colors: ThemeColors,
    pub status: StatusColors,
    pub syntax: SyntaxTheme,
    pub accents: AccentColors,
    pub window_background_appearance: WindowBackgroundAppearance,
}

pub struct ThemeFamily {
    pub id: String,
    pub name: SharedString,
    pub author: String,
    pub themes: Vec<Theme>,  // Typisch: [Dark, Light]
    pub scales: ColorScales,
}
```

### Globaler Zugriff via Inazuma

```rust
// GlobalTheme implementiert inazuma::Global
pub struct GlobalTheme(pub Arc<Theme>);

// Zugriff in render():
fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div()
        .bg(cx.theme().colors.background)
        .text_color(cx.theme().colors.text)
}

// ActiveTheme Trait auf App/Context
pub trait ActiveTheme {
    fn theme(&self) -> &Arc<Theme>;
}
```

### Refinement Pattern (von Zed übernommen)

Themes können partiell überschrieben werden — alle Felder `Option<Oklch>`:

```rust
pub struct ThemeColorsRefinement {
    pub background: Option<Oklch>,
    pub surface: Option<Oklch>,
    pub text: Option<Oklch>,
    // ... alle Felder optional
}
```

Nützlich für: User-Overrides in der Config, Accent-Color Anpassung, partielle Theme-Varianten.

---

## Schritt 5: Theme-Dateien (TOML)

### Format

```toml
[theme]
name = "Raijin Dark"
author = "nyxb"
appearance = "dark"

[colors]
# Alle Formate erlaubt — Hex, RGB, OKLCH, HSL
background = "#121212"
surface = "#1a1a1a"
elevated_surface = "#222222"
text = "#f1f1f1"
text_muted = "#888888"
accent = "#14F195"
border = "#2a2a2a"

# Power-User können direkt OKLCH nutzen:
# accent = "oklch(0.88 0.22 160)"

[colors.terminal]
background = "#121212"
foreground = "#f1f1f1"
ansi_black = "#282828"
ansi_red = "#f7768e"
ansi_green = "#14F195"
ansi_yellow = "#e0af68"
ansi_blue = "#7aa2f7"
ansi_magenta = "#bb9af7"
ansi_cyan = "#7dcfff"
ansi_white = "#c0caf5"

[status]
error = "#f7768e"
warning = "#e0af68"
success = "#14F195"
info = "#7dcfff"

[syntax]
keyword = { color = "#bb9af7", font_weight = "bold" }
string = "#14F195"
comment = "#565f89"
function = "#7aa2f7"
variable = "#c0caf5"
type = "#2ac3de"
number = "#ff9e64"
operator = "#89ddff"
```

### Lade-Pipeline

```
~/.config/raijin/themes/*.toml     # User-Themes
crates/raijin-ui/themes/*.toml     # Bundled Themes
        │
        ▼
    TOML Parser
        │
        ▼
    Multi-Format Color Parser
    (#hex / rgb() / oklch() / hsl() → Oklch)
        │
        ▼
    ThemeFamily / Theme Structs
        │
        ▼
    ThemeRegistry (HashMap<name, Arc<Theme>>)
        │
        ▼
    GlobalTheme (inazuma::Global)
        │
        ▼
    cx.theme().colors.X  (in jedem render())
```

---

## 3.1 — Farb-System & Theming (Tasks)

- [ ] `Oklch` Struct in `crates/inazuma/src/color.rs` definieren
- [ ] Konvertierungen: `Oklch ↔ Oklab ↔ Linear sRGB ↔ Rgba`
- [ ] Konvertierungen: `Oklch ↔ Hsla` (über Rgba)
- [ ] Multi-Format Color Parser: `#hex`, `rgb()`, `oklch()`, `hsl()`
- [ ] `oklch()` und `oklcha()` Constructor-Funktionen
- [ ] Blending & Interpolation direkt in Oklch
- [ ] `ThemeColors` Struct mit allen semantischen Tokens
- [ ] `StatusColors`, `AccentColors`, `SyntaxTheme`
- [ ] `ColorScale` (12-Step, OKLCH-basiert)
- [ ] `Theme`, `ThemeFamily`, `ThemeStyles` Structs
- [ ] `GlobalTheme` via `inazuma::Global` + `ActiveTheme` Trait
- [ ] `ThemeColorsRefinement` für partielle Overrides
- [ ] `ThemeRegistry` mit TOML-Loader
- [ ] Theme-Lade-Pipeline: TOML → Parse → Registry → Global
- [ ] Raijin Dark als Default-Theme (aktuell hardcoded: `#121212` bg, `#14F195` accent, `#f1f1f1` fg)
- [ ] Alle hardcodierten Farben in `terminal_element.rs` durch `cx.theme()` ersetzen
- [ ] Theme Library: Min. 5 Themes (Dark, Light, Dracula, Nord, Gruvbox)
- [ ] Accent-Color konfigurierbar in `~/.config/raijin/config.toml`
- [ ] Tab-Farben pro Tab konfigurierbar (6 Farben wie Warp)
- [ ] Transparenter Background mit Opacity-Slider
- [ ] Hot-Reload: Theme-Dateien überwachen, bei Änderung neu laden

---

## 3.2 — Typographie-Hierarchie

- [ ] Font-Stack: Terminal (monospace) + UI (proportional)
- [ ] Größen-Skala: 11px → 12px → 13px → 14px → 16px
- [ ] Font konfigurierbar in Settings (Type + Size)

---

## 3.3 — Visual Layering

- [ ] 3+ Hintergrund-Ebenen mit subtilen Borders
- [ ] Komponenten-Design: Tabs, Sidebar, Blocks, Scrollbar

---

## 3.4 — Animationen

- [ ] Hover: 150ms ease-out
- [ ] Tab-Switch: Accent slide
- [ ] Block-Expand/Collapse
- [ ] Cursor-Blink: Smooth opacity

---

## Vergleich: Zed vs. Raijin Architektur

| Aspekt | Zed | Raijin |
|---|---|---|
| **Interner Farbraum** | `Hsla` | `Oklch` |
| **User-Input Format** | nur `#hex` | `#hex`, `rgb()`, `oklch()`, `hsl()` |
| **Theme-Format** | JSON | TOML |
| **Color Blending** | Hsla (perceptual Artefakte) | Oklch (perceptually uniform) |
| **Color Scales** | 12-Step HSLA (manuell kalibriert) | 12-Step OKLCH (automatisch uniform) |
| **Palette-Generierung** | Manuelle Korrekturen pro Hue nötig | L linear skalieren bei konstantem C/H |
| **Gamut-Mapping (P3)** | Nicht unterstützt | Oklch Chroma-Clipping für sRGB/P3 |
| **Contrast-Checks** | Ungenau (HSLA L ≠ wahrgenommene Helligkeit) | L-Differenz ≈ tatsächlicher Kontrast |
| **Framework-Primitiv** | `Hsla` in GPUI | `Oklch` in Inazuma (+ Hsla Compat) |
| **Struct Felder** | ~100 ThemeColors Felder | ~100 ThemeColors + Raijin-spezifische (Blocks, Input) |
| **Global Access** | `cx.theme()` via Global Trait | Identisch: `cx.theme()` via `inazuma::Global` |
| **Refinement** | `ThemeColorsRefinement` (alle Optional) | Identisch: partielle Overrides |
| **Theme Registry** | `ThemeRegistry` mit RwLock | Identisch, aber TOML statt JSON |

---

## Abhängigkeiten auf andere Pläne

- **Plan 10 — OKLCH-Migration:** `Oklch` Struct in Inazuma's `color.rs` ist die Grundlage für das gesamte Token-System
- **Plan 10 — Wide Gamut P3:** Die Inazuma Metal Renderer Änderungen (CAMetalLayer, Shader, Pixel-Format) leben dort. `raijin-ui` nutzt P3 für Gamut-Mapping: Theme-Autoren können OKLCH-Farben mit hohem Chroma definieren die den P3-Gamut ausnutzen, mit automatischem Fallback auf sRGB
- **Plan 10 — objc2 Migration:** P3 Support braucht ggf. objc2 Bindings für `CGColorSpace`. Kann parallel laufen

---

## Milestone

✅ OKLCH als interner Farbraum in Inazuma
✅ Semantische Design-Tokens statt hardcodierter Hex-Werte
✅ Multi-Format Color Input (Hex, RGB, OKLCH, HSL)
✅ Wide Gamut Display P3 via OKLCH Gamut-Mapping (Implementierung in Plan 10)
✅ App sieht visuell auf Warp-Niveau aus
✅ Mindestens 5 Themes verfügbar
✅ Animationen fühlen sich smooth und polished an
✅ Perceptually korrekte Farb-Interpolation überall
