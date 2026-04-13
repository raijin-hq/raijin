# Phase 19: Settings-System Migration — SettingsStore mit TOML

## Ziel

Das vollständige Referenz-Settings-System (`Settings` Trait, `SettingsStore`, `SettingsContent`, `from_settings()`, `get_global()`, `observe_global`, etc.) als unsere Architektur übernehmen. JSON wird durch TOML ersetzt, Pfade gehen nach `~/.raijin/`. Am Ende ist die Struktur 1:1 wie die Referenz, nur mit TOML und unseren eigenen Extra-Settings.

## Warum

- Der gesamte Referenz-Crate-Graph nutzt `ThemeSettings::get_global(cx)` — das kommt vom `Settings` Trait
- Unser `raijin-theme-settings` hat den Trait NICHT implementiert, nutzt `impl Global` direkt
- `inazuma-settings-framework` (= Referenz `settings` Crate) ist schon 1:1 kopiert, wird aber nicht genutzt
- Ohne diese Integration blocken dutzende Crates beim Kompilieren (raijin-markdown, raijin-editor, etc.)

## Ist-Zustand

### Was wir haben:

| Crate | Zeilen | Beschreibung |
|---|---|---|
| `inazuma-settings-framework` | 7295 | 1:1 Kopie vom Referenz `crates/settings` — `Settings` Trait, `SettingsStore`, `SettingsContent`, `RegisterSetting` derive, JSON-Schema-System. Wird aktuell NICHT genutzt. |
| `raijin-settings` | 672 | Unser eigenes System — `RaijinSettings` als monolithisches TOML-Struct, `~/.raijin/settings.toml`, `impl Global`. |
| `raijin-theme-settings` | 234 | Selbst geschrieben — `ThemeSettings` mit `impl Global`, KEIN `Settings` Trait, kein `get_global()`. |
| Referenz `theme_settings` | 1896 | NICHT kopiert — vollständige `ThemeSettings` mit `Settings` Trait, Font-Management, Theme-Overrides, Icon-Themes. |

### Was fehlt:
- `ThemeSettings` implementiert `Settings` Trait nicht → kein `get_global()`
- Kein `SettingsStore` in der App registriert
- `ThemeSettings.buffer_font`, `.ui_font`, `.buffer_font_size`, `.ui_font_size` fehlen komplett
- Font-Size-Management (`adjust_buffer_font_size`, `setup_ui_font`, etc.) fehlt
- `apply_theme_overrides()` fehlt
- `schema.rs` mit Konvertierungsfunktionen fehlt

## Architektur-Entscheidungen

### TOML statt JSON
- `SettingsContent` in `inazuma-settings-framework` wird von JSON auf TOML umgestellt
- `settings_file.rs` Lese-/Schreib-Logik: `toml::from_str` statt `serde_json::from_str`
- `default_settings()` gibt TOML-String statt JSON zurück
- JSON-Schema-System (`schemars`) bleibt erstmal (für zukünftige Editor-Completion), File-Format wird TOML

### Pfade
| Referenz | Raijin |
|---|---|
| `~/.config/zed/settings.json` | `~/.raijin/settings.toml` |
| `~/.config/zed/keymap.json` | `~/.raijin/keymap.toml` |
| `~/.config/zed/themes/` | `~/.raijin/themes/` |

Pfade sind bereits in `raijin-settings::RaijinSettings::home_dir()` definiert.

### Zusammenführung
`raijin-settings` wird IN das `inazuma-settings-framework` / `SettingsStore` System integriert:
- `RaijinSettings` Sektionen werden zu `Settings` Trait Implementoren
- `ThemeSettings` wird wie in der Referenz über den `SettingsStore` verwaltet
- Raijin-spezifische Settings als eigene Sektionen im `SettingsContent`

## Settings-Datei Format (Ziel)

```toml
# ~/.raijin/settings.toml

# === Theme ===
[theme]
theme = "Raijin Dark"                    # oder:
# theme = { mode = "system", light = "Raijin Light", dark = "Raijin Dark" }
icon_theme = "Raijin"
ui_font_family = "Raijin Plex Mono"
ui_font_size = 16
buffer_font_family = "Raijin Plex Mono"
buffer_font_size = 14
buffer_line_height = "comfortable"       # comfortable | standard | 1.8
ui_density = "default"                   # compact | default | comfortable
unnecessary_code_fade = 0.3

# === Theme Overrides (optional) ===
# [theme.overrides."Raijin Dark".colors]
# background = "oklch(0.15 0 none)"

# === General (Raijin-spezifisch) ===
[general]
working_directory = "home"               # home | previous | "/custom/path"
input_mode = "raijin"                    # raijin | shell_ps1

# === Terminal ===
[terminal]
scrollback_history = 10000
cursor_style = "beam"                    # beam | block | underline
# shell = "system"                      # oder: { program = "/bin/zsh", args = [] }

# === Appearance (Raijin-spezifisch) ===
[appearance]
window_colorspace = "srgb"              # srgb | display_p3 | native

[[appearance.symbol_map]]
start = "E0B0"
end = "E0D7"
font_family = "Symbols Nerd Font Mono"
```

## Phasen

### Phase 1: SettingsStore auf TOML umstellen

**Dateien:**
- `inazuma-settings-framework/src/settings_store.rs`
- `inazuma-settings-framework/src/settings_file.rs`
- `inazuma-settings-framework/src/settings.rs`

**Aufgaben:**
1. `settings_store.rs` — Alle `serde_json::from_str` / `serde_json::from_value` Aufrufe durch `toml::from_str` / `toml::Value` ersetzen
2. `settings_store.rs` — `load_settings()` und `update_settings()` auf TOML umstellen
3. `settings_file.rs` — File-Watcher auf `.toml` statt `.json`
4. `settings_file.rs` — `toml::to_string_pretty` für Serialisierung
5. `settings.rs` — `SettingsContent` Struct mit allen Sektionen für TOML (theme, general, terminal, appearance)
6. Cargo.toml — `toml` dependency hinzufügen, `serde_json`/`serde_json_lenient` entfernen wo nicht mehr gebraucht

**Referenz:**
- Referenz: `.reference/zed/crates/settings/src/settings_store.rs` (2579 Zeilen)
- Unser aktueller Stand: `inazuma-settings-framework/src/settings_store.rs` (2579 Zeilen, 1:1 Kopie)

### Phase 2: Default-Settings TOML erstellen

**Neue Datei:** `assets/settings/default.toml`

**Aufgaben:**
1. Referenz `assets/settings/default.json` als Vorlage nehmen
2. In TOML konvertieren
3. Raijin-Defaults einfügen (Raijin Dark Theme, unsere Font-Defaults, etc.)
4. Raijin-spezifische Sektionen hinzufügen (general, appearance mit symbol_map)
5. In `raijin-assets` einbinden (rust-embed)

### Phase 3: ThemeSettings von der Referenz übernehmen

**Dateien:**
- `raijin-theme-settings/src/settings.rs` — ERSETZEN mit Referenz-Version
- `raijin-theme-settings/src/schema.rs` — NEU von der Referenz kopieren
- `raijin-theme-settings/src/theme_settings.rs` — NEU (Referenz lib.rs Logik)
- `raijin-theme-settings/Cargo.toml` — Dependencies updaten

**Aufgaben:**

1. **`settings.rs`** von der Referenz kopieren (634 Zeilen):
   - `gpui::` → `inazuma::`
   - `settings::` → `inazuma_settings_framework::`
   - `theme::` → `raijin_theme::`
   - `collections::` → `std::collections::`
   - `refineable::` → `inazuma_refineable::`
   - Enthält: `ThemeSettings` Struct, `impl Settings for ThemeSettings`, `ThemeSelection`, `IconThemeSelection`, `BufferLineHeight`, Font-Size-Funktionen, `setup_ui_font()`, `apply_theme_overrides()`

2. **`schema.rs`** von der Referenz kopieren (850 Zeilen):
   - Gleiche Renames wie oben
   - Enthält: `ThemeStyleContent`, `ThemeColorsContent`, `StatusColorsContent`
   - Konvertierungsfunktionen: `theme_colors_refinement()`, `status_colors_refinement()`, `syntax_overrides()`
   - HSLA-Referenzen → OKLCH (unsere `parse_color()` nutzen statt der Referenz)

3. **`theme_settings.rs`** (Referenz `lib.rs` Logik, 412 Zeilen):
   - `init()`, `reload_theme()`, `reload_icon_theme()`
   - `configured_theme()`, `configured_icon_theme()`
   - `refine_theme()`, `refine_theme_family()` — auf TOML anpassen statt JSON
   - `load_bundled_themes()` — `.toml` statt `.json` Dateien laden
   - `merge_player_colors()`, `merge_accent_colors()`
   - `ThemeSettingsProviderImpl` für `ThemeSettingsProvider` Trait

4. **`Cargo.toml`** Dependencies:
   ```toml
   inazuma-settings-framework.workspace = true
   inazuma-refineable.workspace = true
   raijin-theme.workspace = true
   toml.workspace = true
   schemars.workspace = true
   uuid.workspace = true
   ```

**Referenz-Dateien:**
- `.reference/zed/crates/theme_settings/src/settings.rs` (634 Zeilen)
- `.reference/zed/crates/theme_settings/src/schema.rs` (850 Zeilen)
- `.reference/zed/crates/theme_settings/src/theme_settings.rs` (412 Zeilen)

### Phase 4: RaijinSettings ins SettingsStore integrieren

**Dateien:**
- `raijin-settings/src/lib.rs` — umbauen

**Aufgaben:**
1. `RaijinSettings` Sektionen als separate `Settings` Trait Implementoren:
   - `GeneralSettings` → `impl Settings for GeneralSettings`
   - `TerminalSettings` → `impl Settings for TerminalSettings`
   - `AppearanceSettings` → `impl Settings for AppearanceSettings`
2. Jede Sektion hat `from_settings()` die aus `SettingsContent` liest
3. `RaijinSettings::load()` / `save()` delegiert an SettingsStore
4. `home_dir()`, `themes_dir()`, etc. bleiben als statische Methoden
5. Watcher (`watcher.rs`) nutzt SettingsStore-Observer

### Phase 5: Init-Reihenfolge in raijin-app

**Datei:** `raijin-app/src/main.rs` (oder `raijin-app/src/app.rs`)

**Init-Reihenfolge (wie in der Referenz):**
```rust
// 1. SettingsStore registrieren + Default-Settings laden
SettingsStore::register(cx);
SettingsStore::update_global(cx, |store, _| {
    store.set_default_settings(&DEFAULT_SETTINGS_TOML);
});

// 2. User-Settings von ~/.raijin/settings.toml laden
if let Ok(content) = std::fs::read_to_string(RaijinSettings::settings_path()) {
    SettingsStore::update_global(cx, |store, cx| {
        store.set_user_settings(&content, cx);
    });
}

// 3. Settings-Typen registrieren
ThemeSettings::register(cx);
GeneralSettings::register(cx);
TerminalSettings::register(cx);
AppearanceSettings::register(cx);

// 4. Theme-System initialisieren (nutzt SettingsStore)
raijin_theme_settings::init(LoadThemes::All(cx.asset_source()), cx);

// 5. File-Watcher für Settings starten
watch_settings_file(cx);
```

### Phase 6: Cleanup + alle Callsites fixen

**Aufgaben:**
1. Alle `cx.global::<ThemeSettings>()` → funktionieren jetzt automatisch über `ThemeSettings::get_global(cx)`
2. `raijin-markdown` — `ThemeSettings::get_global(cx)` funktioniert
3. `raijin-editor` — Font-Settings über `ThemeSettings`
4. `raijin-ui` — `ThemeSettingsProvider` ist verdrahtet
5. Alte `raijin-theme-settings` Logik entfernen (unser custom init/reload)
6. `raijin-settings` alte `impl Global for RaijinSettings` entfernen

## Raijin-spezifische Erweiterungen (über die Referenz hinaus)

Diese Settings hat die Referenz nicht, wir behalten/fügen sie hinzu:
- `general.input_mode` (raijin/shell_ps1)
- `general.working_directory` (home/previous/custom)
- `appearance.window_colorspace` (srgb/display_p3/native)
- `appearance.symbol_map` (Nerd Font Unicode-Range → Font-Family)
- `terminal.cursor_style` (beam/block/underline)
- Block-Badge-Farben in ThemeColors (block_success_badge, block_error_badge, block_running_badge)

## Dateien-Mapping (Referenz → Raijin)

| Referenz Datei | Raijin Datei | Änderungen |
|---|---|---|
| `settings/src/settings_store.rs` | `inazuma-settings-framework/src/settings_store.rs` | JSON→TOML Parsing |
| `settings/src/settings_file.rs` | `inazuma-settings-framework/src/settings_file.rs` | .json→.toml, serde_json→toml |
| `settings/src/settings.rs` | `inazuma-settings-framework/src/settings.rs` | SettingsContent mit TOML-Sektionen |
| `theme_settings/src/settings.rs` | `raijin-theme-settings/src/settings.rs` | gpui→inazuma, Crate-Renames |
| `theme_settings/src/schema.rs` | `raijin-theme-settings/src/schema.rs` | gpui→inazuma, HSLA→OKLCH |
| `theme_settings/src/theme_settings.rs` | `raijin-theme-settings/src/theme_settings.rs` | gpui→inazuma, JSON→TOML |
| `assets/settings/default.json` | `assets/settings/default.toml` | JSON→TOML, Raijin-Defaults |

## Wichtige Referenz-Dateien zum Lesen

Vor der Implementierung diese Dateien in `.reference/zed/` genau lesen:

1. **`crates/settings/src/settings_store.rs`** — SettingsStore Kernlogik, `Settings` Trait Definition, `SettingsContent`
2. **`crates/theme_settings/src/settings.rs`** — ThemeSettings Struct + `impl Settings`, Font-Management
3. **`crates/theme_settings/src/schema.rs`** — ThemeStyleContent, Konvertierungsfunktionen
4. **`crates/theme_settings/src/theme_settings.rs`** — init(), refine_theme(), load_bundled_themes()
5. **`crates/zed/src/main.rs`** — Init-Reihenfolge der Settings

## Risiken

1. **`SettingsContent`** ist in der Referenz ein riesiges Struct mit ALLEN Sektionen aller Crates. Wir brauchen nicht alles — nur was wir nutzen. Nicht-benötigte Felder als `Option` mit `#[serde(skip)]` oder einfach weglassen.
2. **`RegisterSetting` derive macro** in `inazuma-settings-macros` muss auf unsere Crate-Namen zeigen (`inazuma_settings_framework` statt `settings`).
3. **TOML vs JSON Subtleties**: TOML hat kein `null` — alle optionalen Felder müssen über Abwesenheit statt `null` gehandhabt werden. `serde(default)` wird wichtiger.
4. **`serde_json_lenient` aus der Referenz** (erlaubt Kommentare in JSON) hat kein TOML-Äquivalent nötig — TOML hat native Kommentare.
