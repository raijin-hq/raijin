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

Die Referenz nutzt intern `Hsla` — wir migrieren komplett auf **OKLCH** (Oklab Lightness Chroma Hue).

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

Die Referenz blendet Farben in HSLA (über Rgba-Konvertierung) — das erzeugt perceptual Artefakte. Wir machen Blending direkt in Oklch:

```rust
impl Oklch {
    pub fn blend(self, other: Oklch) -> Oklch {
        // Interpolation in Oklch-Space = perceptually korrekt
        // Kein Umweg über Rgba nötig
    }
}
```

### Was von der Referenz bleibt (Compat)

- `Rgba` — bleibt als GPU-Output-Format und für Hex-Serialisierung
- `Hsla` — bleibt für Legacy-Compat, wird aber intern nicht mehr für Blending/Interpolation genutzt
- `hsla()` Constructor — bleibt verfügbar, konvertiert intern sofort zu Oklch
- Hex-Parsing auf `Rgba` — bleibt, wird um Oklch-Konvertierung erweitert

---

## Schritt 2: Semantische Farb-Tokens (`raijin-ui/colors.rs`)

### `ThemeColors` Struct

Inspiriert von der Referenz `ThemeColors`, aber alle Felder sind `Oklch` statt `Hsla`:

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

OKLCH-basierte Farbskalen — perceptually uniforme Stufen statt HSLA-Scales:

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

**Vorteil gegenüber der Referenz:** Bei OKLCH reicht es, L linear von ~0.15 bis ~0.95 zu skalieren bei konstantem C und H — die Stufen sehen dann tatsächlich gleichmäßig verteilt aus. Bei HSLA müsste man pro Hue manuell korrigieren.

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

### Refinement Pattern (von der Referenz übernommen)

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

### Warum TOML statt JSON

Raijin ist ein Hybrid aus Terminal (Warp) und Editor. Beide Welten brauchen ein mächtiges Theme-System mit ~120+ semantischen Tokens und Syntax-Highlighting. Die Formatwahl wurde bewusst getroffen:

**Die Referenz-Theme-Struktur ist flach, nicht tief verschachtelt.** Das `style`-Objekt in den Referenz-Themes ist ein flaches Key-Value-Mapping mit Dot-Notation (`"editor.background"`, `"terminal.ansi.red"`, `"border.focused"`). Es gibt keine tiefen Objekt-Hierarchien — nur ~120 flache Farbwerte + eine `syntax`-Map. TOML kann das identisch abbilden.

| Aspekt | JSON (Referenz/VS Code) | TOML (Raijin) |
|---|---|---|
| **Kommentare** | Nicht möglich (JSONC als Hack) | Nativ — Theme-Autoren können Farbgruppen dokumentieren |
| **Boilerplate** | `{}`, `""` um jeden Key, Trailing-Comma-Probleme | Minimal — saubere Key=Value Syntax |
| **Dot-Notation** | `"editor.background": "#282c33"` | `"editor.background" = "#282c33"` — identisch |
| **Syntax-Map** | Tief verschachtelte Objekte | `[style.syntax.keyword]` — übersichtlicher |
| **Ökosystem** | VS Code Theme-Community | Rust-Ökosystem (Alacritty, Rio, Helix, Cargo) |
| **Schema-Validierung** | JSON Schema (ausgereift) | `taplo` TOML LSP unterstützt JSON Schema seit 2025 |
| **Sharing** | `.json` Dateien | `.toml` Dateien — genauso teilbar |
| **Config-Konsistenz** | Raijin-Config ist TOML → Theme wäre anderes Format | Ein Format für alles: Config + Themes |

**Referenzen im Terminal/Editor-Ökosystem:**
- Alacritty (seit 0.13.0), Rio, Helix Editor — alle TOML für Themes
- Ghostty — TOML-ähnliches Flat-Format
- Warp — YAML (kein Vorbild)
- Referenz, VS Code — JSON (historisch bedingt, Atom/Electron-Erbe)

**Referenz-Kompatibilität:** Die semantischen Token-Namen sind 1:1 kompatibel mit dem Referenz-Schema. Ein Konverter kann Referenz-JSON-Themes automatisch nach Raijin-TOML importieren, da die Struktur identisch ist — nur das Serialisierungsformat unterscheidet sich.

### Format

Raijin-Themes nutzen **Referenz-kompatible semantische Token-Namen** in TOML. Die `style`-Sektion verwendet dieselbe Dot-Notation wie die Referenz, alle ~120+ Tokens werden unterstützt.

```toml
# Raijin Theme — Referenz-kompatibles Token-Schema in TOML
#
# Alle Farbformate erlaubt: #hex, rgb(), oklch(), hsl()
# Fehlende Tokens werden vom Theme-Resolver mit Defaults gefüllt.

[theme]
name = "Raijin Dark"
author = "nyxb"
appearance = "dark"  # "dark" | "light"

[style]
# === Backgrounds ===
background = "#121212"
"surface.background" = "#1a1a1a"
"elevated_surface.background" = "#222222"
"element.background" = "#1e1e1e"
"element.hover" = "#2a2a2a"
"element.active" = "#333333"
"element.selected" = "#333333"
"element.disabled" = "#1e1e1e"
"drop_target.background" = "#14F19540"
"ghost_element.background" = "#00000000"
"ghost_element.hover" = "#2a2a2a"
"ghost_element.active" = "#333333"
"ghost_element.selected" = "#333333"

# === Borders ===
border = "#2a2a2a"
"border.variant" = "#222222"
"border.focused" = "#14F195"
"border.selected" = "#14F19566"
"border.transparent" = "#00000000"
"border.disabled" = "#1e1e1e"

# === Text ===
text = "#f1f1f1"
"text.muted" = "#888888"
"text.placeholder" = "#555555"
"text.disabled" = "#555555"
"text.accent" = "#14F195"

# === Icons ===
icon = "#f1f1f1"
"icon.muted" = "#888888"
"icon.disabled" = "#555555"
"icon.accent" = "#14F195"

# === Workspace Chrome ===
"status_bar.background" = "#121212"
"title_bar.background" = "#121212"
"title_bar.inactive_background" = "#0e0e0e"
"toolbar.background" = "#1a1a1a"
"tab_bar.background" = "#0e0e0e"
"tab.active_background" = "#1a1a1a"
"tab.inactive_background" = "#0e0e0e"
"panel.background" = "#0e0e0e"

# === Editor ===
"editor.foreground" = "#f1f1f1"
"editor.background" = "#121212"
"editor.gutter.background" = "#121212"
"editor.active_line.background" = "#1a1a1abf"
"editor.line_number" = "#4e5a5f"
"editor.active_line_number" = "#f1f1f1"
"editor.invisible" = "#333333"
"editor.wrap_guide" = "#2a2a2a0d"

# === Search ===
"search.match_background" = "#14F19540"

# === Scrollbar ===
"scrollbar.thumb.background" = "#ffffff1a"
"scrollbar.thumb.hover_background" = "#ffffff33"
"scrollbar.track.background" = "#00000000"

# === Terminal (ANSI-16 + dim) ===
"terminal.background" = "#121212"
"terminal.foreground" = "#f1f1f1"
"terminal.bright_foreground" = "#ffffff"
"terminal.dim_foreground" = "#888888"
"terminal.ansi.black" = "#282828"
"terminal.ansi.red" = "#f7768e"
"terminal.ansi.green" = "#14F195"
"terminal.ansi.yellow" = "#e0af68"
"terminal.ansi.blue" = "#7aa2f7"
"terminal.ansi.magenta" = "#bb9af7"
"terminal.ansi.cyan" = "#7dcfff"
"terminal.ansi.white" = "#c0caf5"
"terminal.ansi.bright_black" = "#555555"
"terminal.ansi.bright_red" = "#ff9e9e"
"terminal.ansi.bright_green" = "#3dffc0"
"terminal.ansi.bright_yellow" = "#ffcf88"
"terminal.ansi.bright_blue" = "#9db8ff"
"terminal.ansi.bright_magenta" = "#d4b4ff"
"terminal.ansi.bright_cyan" = "#a8e4ff"
"terminal.ansi.bright_white" = "#ffffff"
"terminal.ansi.dim_black" = "#1a1a1a"
"terminal.ansi.dim_red" = "#a55060"
"terminal.ansi.dim_green" = "#0ea56a"
"terminal.ansi.dim_yellow" = "#a58050"
"terminal.ansi.dim_blue" = "#5575a5"
"terminal.ansi.dim_magenta" = "#8570a5"
"terminal.ansi.dim_cyan" = "#5590a5"
"terminal.ansi.dim_white" = "#888888"

# === Git / Version Control ===
"version_control.added" = "#14F195"
"version_control.modified" = "#e0af68"
"version_control.deleted" = "#f7768e"

# === Status (error, warning, success, info, etc.) ===
error = "#f7768e"
"error.background" = "#f7768e1a"
"error.border" = "#f7768e33"
warning = "#e0af68"
"warning.background" = "#e0af681a"
"warning.border" = "#e0af6833"
success = "#14F195"
"success.background" = "#14F1951a"
"success.border" = "#14F19533"
info = "#7dcfff"
"info.background" = "#7dcfff1a"
"info.border" = "#7dcfff33"

# === Link ===
"link_text.hover" = "#14F195"

# === Raijin-spezifisch (Block-System) ===
"block.header_background" = "#1a1a1a"
"block.header_hover" = "#222222"
"block.success_badge" = "#14F195"
"block.error_badge" = "#f7768e"
"block.running_badge" = "#e0af68"

# === Players (Cursor-Farben, zukünftig: Multiplayer) ===
[[style.players]]
cursor = "#14F195"
background = "#14F195"
selection = "#14F1953d"

[[style.players]]
cursor = "#7aa2f7"
background = "#7aa2f7"
selection = "#7aa2f73d"

[[style.players]]
cursor = "#bb9af7"
background = "#bb9af7"
selection = "#bb9af73d"

[[style.players]]
cursor = "#f7768e"
background = "#f7768e"
selection = "#f7768e3d"

# === Syntax Highlighting ===
# Jeder Scope kann als einfacher Farbstring oder als Objekt mit
# color, font_style ("normal"/"italic"/"oblique"), font_weight angegeben werden.

[style.syntax.attribute]
color = "#7aa2f7"

[style.syntax.boolean]
color = "#ff9e64"

[style.syntax.comment]
color = "#565f89"
font_style = "italic"

[style.syntax."comment.doc"]
color = "#6a7394"
font_style = "italic"

[style.syntax.constant]
color = "#ff9e64"

[style.syntax.constructor]
color = "#7aa2f7"

[style.syntax.function]
color = "#7aa2f7"

[style.syntax.keyword]
color = "#bb9af7"

[style.syntax.number]
color = "#ff9e64"

[style.syntax.operator]
color = "#89ddff"

[style.syntax.property]
color = "#73daca"

[style.syntax.string]
color = "#14F195"

[style.syntax."string.escape"]
color = "#89ddff"

[style.syntax."string.regex"]
color = "#b4f9f8"

[style.syntax.tag]
color = "#f7768e"

[style.syntax.type]
color = "#2ac3de"

[style.syntax.variable]
color = "#c0caf5"

[style.syntax."variable.special"]
color = "#ff9e64"

[style.syntax.punctuation]
color = "#c0caf5"

[style.syntax."punctuation.bracket"]
color = "#9aa5ce"

[style.syntax.title]
color = "#f7768e"
font_weight = 700

# Power-User können direkt OKLCH nutzen:
# [style.syntax.keyword]
# color = "oklch(0.75 0.15 300)"
# font_style = "italic"
```

### Referenz-Theme Import

Da Raijin dieselben semantischen Token-Namen wie die Referenz verwendet, können Referenz-JSON-Themes automatisch importiert werden:

```
zed-theme.json                    raijin-theme.toml
─────────────────                 ─────────────────
"editor.background": "#282c33"  → "editor.background" = "#282c33"
"syntax": { "keyword": {        → [style.syntax.keyword]
    "color": "#b477cf" }}           color = "#b477cf"
```

Der `ThemeRegistry` enthält einen `import_zed_theme(json: &str) -> Theme` Konverter.

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

### Erledigt (2026-04-05)

- [x] `Oklch` Struct in `crates/inazuma/src/color/types.rs` — Hsla komplett entfernt (814 Stellen, 96 Files)
- [x] Konvertierungen: Oklch ↔ Rgba (via Oklab intern), culori-Referenz-Koeffizienten
- [x] `oklch()` und `oklcha()` Constructor-Funktionen + `hsla()` gibt jetzt Oklch zurück
- [x] Blending & Interpolation direkt in Oklch (shortest-arc Hue)
- [x] `ThemeColors` Struct mit 278 semantischen Tokens (Referenz-kompatibel)
- [x] `StatusColors`, `AccentColors`, `SyntaxTheme`, `ColorScale`, `PlayerColors`
- [x] `Theme`, `ThemeFamily`, `ThemeStyles`, `ThemeColorsRefinement`
- [x] `GlobalTheme` via `inazuma::Global` + `ActiveTheme` Trait
- [x] `ThemeRegistry` (HashMap-basiert)
- [x] Raijin Dark Fallback-Theme (hardcoded: #121212 bg, #00BFFF accent, #f1f1f1 fg)
- [x] 6 Bundled Theme TOML-Dateien: Raijin Dark, Dracula, Nord, Gruvbox, One Dark, Catppuccin
- [x] Theme-Importer: `import_zed_theme(json)` (imports Zed-format themes) + VS Code Importer
- [x] Token-Kompatibilität: 278 Referenz-kompatible ThemeColors Tokens
- [x] GPU-Shaders (Metal/WGSL/HLSL): `oklch_to_rgba()` ersetzt `hsla_to_rgba()`
- [x] GPU-Primitive Structs: `Edges<Oklch>`, `color: Oklch`
- [x] Wide Gamut P3: sRGB auf CAMetalLayer, Gamut-Mapping (clamp_to_srgb/p3)
- [x] `WindowColorspace` Enum (Srgb/DisplayP3/Native) + Config
- [x] 5 Theme-Crates: raijin-theme, raijin-theme-settings, raijin-theme-importer, raijin-theme-extension, raijin-theme-selector
- [x] Alle hardcodierten Farben in raijin-app durch `cx.theme()` ersetzt

### KRITISCH — Altes Theme-System komplett entfernen

**Problem:** Zwei Theme-Systeme laufen parallel — das alte `RaijinTheme` (raijin-settings) und das neue `GlobalTheme` (raijin-theme). Die App liest Hintergrundbild aus dem alten, Farben aus dem neuen System. Das muss EIN System werden.

**Saubere Trennung:**

| Bereich | Zuständig für | Crate |
|---------|--------------|-------|
| **Theme** | ALLES Visuelle: Farben, ANSI-Palette, terminal_accent, background_image, Syntax | `raijin-theme` |
| **Theme-Settings** | Welches Theme aktiv, Overrides, Light/Dark Switch | `raijin-theme-settings` |
| **App-Settings** | Font, Scrollback, Cursor, Working-Dir, Input-Mode — NICHTS Visuelles | `raijin-settings` |

**Was aus `raijin-settings` raus muss:**
- [ ] `RaijinTheme` struct komplett löschen (accent, background, foreground, error, terminal_colors)
- [ ] `RaijinTheme::load()` löschen — Theme-Loading macht `raijin-theme-settings`
- [ ] `ThemeBackgroundImage` struct → nach `raijin-theme` verschieben
- [ ] `ThemeTerminalColors`, `ThemeAnsiColors` → ersetzt durch ThemeColors.terminal_ansi_*
- [ ] `background_opacity` aus AppearanceConfig → weg
- [ ] `block_opacity` → weg (Blocks sind transparent, wie Warp)
- [ ] `ResolvedTheme` → weg (bereits gelöscht)
- [ ] Was in raijin-settings BLEIBT: Font, Scrollback, Cursor, Working-Dir, Input-Mode, symbol_map

**Was in `raijin-theme` rein muss:**
- [ ] `background_image` (path + opacity 0-100) als Feld in `ThemeStyles`
- [ ] `fallback.rs` LÖSCHEN — kein hardcoded Fallback, Default-Theme ist TOML
- [ ] Das "Raijin" Theme ist eine gebundelte TOML-Datei mit `default = true` Markierung

**Das Default-Theme "Raijin":**
- [ ] Vollständiges TOML mit allen Tokens + ANSI-Palette + terminal_accent (#00BFFF) + background_image
- [ ] Markiert als Default-Theme
- [ ] Wird mit der App ausgeliefert (gebundelt in `crates/raijin-theme/themes/raijin.toml`)
- [ ] KEIN hardcoded Fallback in Rust — wenn Theme-Files fehlen, Error statt stilles Fallback

### Offen — TOML-Loader + Theme-Loading Pipeline

- [ ] **TOML-Parser** in `raijin-theme-settings::init()`: Liest `crates/raijin-theme/themes/*.toml` (Bundled) + `~/.raijin/themes/*.toml` (User) → ThemeRegistry
- [ ] **Theme-Format**: Neues Format mit 170+ Tokens. Altes Format (accent/background/foreground) mit Compat-Layer automatisch konvertieren
- [ ] **Theme-Persistenz**: Gewähltes Theme in `~/.raijin/config.toml` speichern + beim Start laden
- [ ] **Hot-Reload**: Theme-Dateien watchen, bei Änderung neu laden
- [ ] **workspace.rs aufräumen**: Hintergrundbild aus `GlobalTheme` lesen statt aus altem `RaijinTheme`

### Offen — Rendering-Modell (wie Warp)

- [ ] **Blocks transparent**: Kein eigener Background — nur Text, Linien, farbiger Rand
- [ ] **Hintergrundbild-Layer**: Fenster-Hintergrundfarbe (opak) → Bild darüber (mit `background_image.opacity`)
- [ ] **`terminal_accent`**: Selection (@8%), Hover (@15%), Cursor, aktive Ränder — Opacity hardcoded, nur Farbe einstellbar
- [ ] **Window Opacity**: Separates Feature (NSWindow), NICHT Theme — für später

### Offen — App Chrome

- [ ] **Command Palette**: `Shift+Cmd+P` → Modal mit allen Actions (Referenz: `.reference/zed/crates/command_palette/`)
- [ ] **Theme Picker**: Via Command Palette "theme" eingeben → ThemeSelector Modal
- [ ] **User Menu**: Dropdown oben rechts mit Settings, Themes..., Extensions
- [ ] **Extensions Page**: `Cmd+Shift+X` → Tab/Panel mit Theme-Browser
- [ ] Tab-Farben pro Tab (6 Farben wie Warp)

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

## Vergleich: Referenz vs. Raijin Architektur

| Aspekt | Referenz | Raijin |
|---|---|---|
| **Interner Farbraum** | `Hsla` | `Oklch` |
| **User-Input Format** | nur `#hex` | `#hex`, `rgb()`, `oklch()`, `hsl()` |
| **Theme-Format** | JSON (historisch, Atom/Electron-Erbe) | TOML (Rust-Ökosystem-Standard, Kommentare, Config-Konsistenz) |
| **Color Blending** | Hsla (perceptual Artefakte) | Oklch (perceptually uniform) |
| **Color Scales** | 12-Step HSLA (manuell kalibriert) | 12-Step OKLCH (automatisch uniform) |
| **Palette-Generierung** | Manuelle Korrekturen pro Hue nötig | L linear skalieren bei konstantem C/H |
| **Gamut-Mapping (P3)** | Nicht unterstützt | Oklch Chroma-Clipping für sRGB/P3 |
| **Contrast-Checks** | Ungenau (HSLA L ≠ wahrgenommene Helligkeit) | L-Differenz ≈ tatsächlicher Kontrast |
| **Framework-Primitiv** | `Hsla` in der Referenz | `Oklch` in Inazuma (Hsla komplett entfernt) |
| **Struct Felder** | ~100 ThemeColors Felder | ~100 ThemeColors + Raijin-spezifische (Blocks, Input) |
| **Global Access** | `cx.theme()` via Global Trait | Identisch: `cx.theme()` via `inazuma::Global` |
| **Refinement** | `ThemeColorsRefinement` (alle Optional) | Identisch: partielle Overrides |
| **Theme Registry** | `ThemeRegistry` mit RwLock | Identisch, aber TOML statt JSON |

---

## Abhängigkeiten auf andere Pläne

- ~~**Plan 10 — OKLCH-Migration:**~~ ✅ Erledigt (2026-04-05) — Oklch als einziger Farbtyp, Hsla eliminiert
- ~~**Plan 10 — Wide Gamut P3:**~~ ✅ Erledigt — sRGB auf CAMetalLayer, Gamut-Mapping
- ~~**Plan 10 — objc2 Migration:**~~ ✅ Erledigt — 183/228 msg_send! zu typed methods

---

## Milestone

✅ OKLCH als einziger interner Farbraum (Hsla komplett entfernt)
✅ 278 semantische Design-Tokens (ThemeColors)
✅ Wide Gamut Display P3 via OKLCH Gamut-Mapping
✅ 6 Bundled Themes verfügbar (TOML)
✅ Perceptually korrekte Farb-Interpolation (Oklch Blending)
✅ GPU-Shaders auf Oklch migriert (Metal + WGSL + HLSL)
✅ Zed-format + VS Code Theme-Importer
⬜ TOML-Themes tatsächlich laden (Loader nicht verdrahtet)
⬜ Theme-Switching zur Laufzeit (Command Palette + Modal)
⬜ User Menu mit Settings/Themes/Extensions
⬜ Extensions Page für Theme-Browser
⬜ App sieht visuell auf Warp-Niveau aus
⬜ Animationen fühlen sich smooth und polished an
