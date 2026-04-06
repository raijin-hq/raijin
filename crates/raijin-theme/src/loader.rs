use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use inazuma::{FontStyle, FontWeight, HighlightStyle, Oklch, Rgba, oklcha};

use crate::colors::ThemeColors;
use crate::players::PlayerColor;
use crate::refinement::ThemeColorsRefinement;
use crate::status::{StatusColors, StatusStyle};
use crate::syntax::SyntaxTheme;
use crate::theme::{Appearance, Theme, ThemeBackgroundImage, ThemeStyles};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Loads a complete theme from TOML source.
///
/// `id` is the theme identifier (typically the filename without extension).
/// `content` is the raw TOML string.
/// `base_dir` is the directory containing the theme file (for resolving relative
/// asset paths like background images). `None` for bundled themes.
pub fn load_theme_from_toml(id: &str, content: &str) -> Result<Theme> {
    load_theme_from_toml_with_base_dir(id, content, None)
}

/// Loads a theme with an explicit base directory for asset resolution.
pub fn load_theme_from_toml_with_base_dir(
    id: &str,
    content: &str,
    base_dir: Option<PathBuf>,
) -> Result<Theme> {
    let root: toml::Value =
        toml::from_str(content).with_context(|| format!("failed to parse theme '{id}'"))?;
    let root = root
        .as_table()
        .ok_or_else(|| anyhow!("theme '{id}' root is not a table"))?;

    // --- [theme] metadata ---
    let meta = root
        .get("theme")
        .and_then(|v| v.as_table())
        .ok_or_else(|| anyhow!("theme '{id}' missing [theme] section"))?;

    let name = meta
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("theme '{id}' missing theme.name"))?
        .to_string();

    let author = meta
        .get("author")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();
    let _ = author; // stored in ThemeFamily, not Theme — reserved for future use

    let appearance = match meta.get("appearance").and_then(|v| v.as_str()) {
        Some("light") => Appearance::Light,
        _ => Appearance::Dark,
    };

    // --- [style] section ---
    let style = root
        .get("style")
        .and_then(|v| v.as_table())
        .ok_or_else(|| anyhow!("theme '{id}' missing [style] section"))?;

    // Separate special keys from flat color keys
    let players_value = style.get("players");
    let syntax_value = style.get("syntax");
    let bg_image_value = style.get("background_image");

    // Flatten all remaining keys (dots → underscores) for color parsing
    let mut flat: HashMap<String, String> = HashMap::new();
    flatten_table(style, "", &mut flat);

    // --- Parse components ---
    let colors = parse_theme_colors(&flat)?;
    let status = parse_status_colors(&flat)?;
    let players = parse_players(players_value)?;
    let syntax = parse_syntax_theme(syntax_value)?;
    let background_image = parse_background_image(bg_image_value)?;

    Ok(Theme {
        id: id.to_string(),
        name: name.into(),
        appearance,
        styles: ThemeStyles {
            colors,
            status,
            syntax,
            players,
            background_image,
        },
        base_dir,
    })
}

// ---------------------------------------------------------------------------
// Color parsing
// ---------------------------------------------------------------------------

/// Parses a CSS-style color string to Oklch.
///
/// Supported formats:
/// - `#RGB` (e.g. `#f00`)
/// - `#RRGGBB` (e.g. `#ff0000`)
/// - `#RRGGBBAA` (e.g. `#ff0000ff`)
pub fn parse_color(s: &str) -> Result<Oklch> {
    let s = s.trim();
    if !s.starts_with('#') {
        bail!("unsupported color format (expected #hex): '{s}'");
    }

    let hex = &s[1..];
    let (r, g, b, a) = match hex.len() {
        // #RGB → expand to #RRGGBB
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16)? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16)? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16)? * 17;
            (r, g, b, 255u8)
        }
        // #RRGGBB
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16)?;
            let g = u8::from_str_radix(&hex[2..4], 16)?;
            let b = u8::from_str_radix(&hex[4..6], 16)?;
            (r, g, b, 255u8)
        }
        // #RRGGBBAA
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16)?;
            let g = u8::from_str_radix(&hex[2..4], 16)?;
            let b = u8::from_str_radix(&hex[4..6], 16)?;
            let a = u8::from_str_radix(&hex[6..8], 16)?;
            (r, g, b, a)
        }
        _ => bail!("invalid hex color length: '{s}'"),
    };

    let rgba = Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: a as f32 / 255.0,
    };
    Ok(Oklch::from(rgba))
}

// ---------------------------------------------------------------------------
// TOML table flattening
// ---------------------------------------------------------------------------

/// Reserved keys in [style] that are NOT flat color values.
const RESERVED_KEYS: &[&str] = &["players", "syntax", "background_image"];

/// Status key prefixes — these map to StatusColors, not ThemeColors.
const STATUS_PREFIXES: &[&str] = &[
    "error", "warning", "success", "info", "conflict", "created", "deleted", "hidden", "hint",
    "ignored", "modified", "predictive", "renamed", "unreachable",
];

/// Recursively flattens a TOML table into dot-separated keys.
///
/// `"terminal.ansi.red" = "#f7768e"` (quoted key) stays as `terminal.ansi.red`.
/// Nested tables like `[style.terminal.ansi]` + `red = "#f7768e"` flatten to `terminal.ansi.red`.
/// Both produce the same output.
fn flatten_table(table: &toml::value::Table, prefix: &str, out: &mut HashMap<String, String>) {
    for (key, value) in table {
        // Skip reserved keys
        if prefix.is_empty() && RESERVED_KEYS.contains(&key.as_str()) {
            continue;
        }

        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };

        match value {
            toml::Value::String(s) => {
                out.insert(full_key, s.clone());
            }
            toml::Value::Table(t) => {
                // Don't recurse into reserved keys
                if prefix.is_empty() && RESERVED_KEYS.contains(&key.as_str()) {
                    continue;
                }
                flatten_table(t, &full_key, out);
            }
            toml::Value::Integer(i) => {
                // Some keys might be integers (e.g. font_weight in syntax)
                out.insert(full_key, i.to_string());
            }
            _ => {
                // Skip arrays and other types at the flat color level
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ThemeColors parsing
// ---------------------------------------------------------------------------

/// Maps a flattened TOML key to the ThemeColors field name.
/// Rule: replace all dots with underscores.
fn toml_key_to_field(key: &str) -> String {
    key.replace('.', "_")
}

/// Returns whether a flattened key belongs to StatusColors.
fn is_status_key(key: &str) -> bool {
    let root = key.split('.').next().unwrap_or(key);
    STATUS_PREFIXES.contains(&root)
}

/// Parses the flattened style table into ThemeColors.
fn parse_theme_colors(flat: &HashMap<String, String>) -> Result<ThemeColors> {
    let base = default_theme_colors();
    let mut refinement = ThemeColorsRefinement::default();

    for (key, value) in flat {
        // Skip status keys — handled separately
        if is_status_key(key) {
            continue;
        }

        let field_name = toml_key_to_field(key);
        let color = parse_color(value)
            .with_context(|| format!("invalid color for '{key}': '{value}'"))?;

        set_refinement_field(&mut refinement, &field_name, color);
    }

    Ok(refinement.apply_to(&base))
}

/// Sets a field on ThemeColorsRefinement by name.
///
/// Uses a macro to match all 126 fields without manual boilerplate.
macro_rules! match_refinement_fields {
    ($refinement:expr, $name:expr, $color:expr, $($field:ident),* $(,)?) => {
        match $name {
            $(stringify!($field) => { $refinement.$field = Some($color); })*
            other => {
                log::trace!("unknown theme color key: '{other}'");
            }
        }
    };
}

fn set_refinement_field(refinement: &mut ThemeColorsRefinement, name: &str, color: Oklch) {
    match_refinement_fields!(
        refinement,
        name,
        color,
        border,
        border_variant,
        border_focused,
        border_selected,
        border_transparent,
        border_disabled,
        elevated_surface_background,
        surface_background,
        background,
        element_background,
        element_hover,
        element_active,
        element_selected,
        element_selection_background,
        element_disabled,
        drop_target_background,
        drop_target_border,
        ghost_element_background,
        ghost_element_hover,
        ghost_element_active,
        ghost_element_selected,
        ghost_element_disabled,
        text,
        text_muted,
        text_placeholder,
        text_disabled,
        text_accent,
        icon,
        icon_muted,
        icon_disabled,
        icon_placeholder,
        icon_accent,
        status_bar_background,
        title_bar_background,
        title_bar_inactive_background,
        toolbar_background,
        tab_bar_background,
        tab_inactive_background,
        tab_active_background,
        search_match_background,
        search_active_match_background,
        panel_background,
        panel_focused_border,
        panel_indent_guide,
        panel_indent_guide_hover,
        panel_indent_guide_active,
        panel_overlay_background,
        panel_overlay_hover,
        pane_focused_border,
        pane_group_border,
        scrollbar_thumb_background,
        scrollbar_thumb_hover_background,
        scrollbar_thumb_active_background,
        scrollbar_thumb_border,
        scrollbar_track_background,
        scrollbar_track_border,
        editor_foreground,
        editor_background,
        editor_gutter_background,
        editor_subheader_background,
        editor_active_line_background,
        editor_highlighted_line_background,
        editor_line_number,
        editor_active_line_number,
        editor_invisible,
        editor_wrap_guide,
        editor_active_wrap_guide,
        editor_indent_guide,
        editor_indent_guide_active,
        editor_document_highlight_read_background,
        editor_document_highlight_write_background,
        editor_document_highlight_bracket_background,
        terminal_background,
        terminal_foreground,
        terminal_bright_foreground,
        terminal_dim_foreground,
        terminal_accent,
        terminal_ansi_background,
        terminal_ansi_black,
        terminal_ansi_bright_black,
        terminal_ansi_dim_black,
        terminal_ansi_red,
        terminal_ansi_bright_red,
        terminal_ansi_dim_red,
        terminal_ansi_green,
        terminal_ansi_bright_green,
        terminal_ansi_dim_green,
        terminal_ansi_yellow,
        terminal_ansi_bright_yellow,
        terminal_ansi_dim_yellow,
        terminal_ansi_blue,
        terminal_ansi_bright_blue,
        terminal_ansi_dim_blue,
        terminal_ansi_magenta,
        terminal_ansi_bright_magenta,
        terminal_ansi_dim_magenta,
        terminal_ansi_cyan,
        terminal_ansi_bright_cyan,
        terminal_ansi_dim_cyan,
        terminal_ansi_white,
        terminal_ansi_bright_white,
        terminal_ansi_dim_white,
        link_text_hover,
        version_control_added,
        version_control_deleted,
        version_control_modified,
        version_control_renamed,
        version_control_conflict,
        version_control_ignored,
        version_control_word_added,
        version_control_word_deleted,
        version_control_conflict_marker_ours,
        version_control_conflict_marker_theirs,
        block_success_badge,
        block_error_badge,
        block_running_badge,
    );
}

// ---------------------------------------------------------------------------
// StatusColors parsing
// ---------------------------------------------------------------------------

fn parse_status_colors(flat: &HashMap<String, String>) -> Result<StatusColors> {
    let defaults = default_status_colors();

    Ok(StatusColors {
        conflict: parse_status_style(flat, "conflict").unwrap_or(defaults.conflict),
        created: parse_status_style(flat, "created").unwrap_or(defaults.created),
        deleted: parse_status_style(flat, "deleted").unwrap_or(defaults.deleted),
        error: parse_status_style(flat, "error").unwrap_or(defaults.error),
        hidden: parse_status_style(flat, "hidden").unwrap_or(defaults.hidden),
        hint: parse_status_style(flat, "hint").unwrap_or(defaults.hint),
        ignored: parse_status_style(flat, "ignored").unwrap_or(defaults.ignored),
        info: parse_status_style(flat, "info").unwrap_or(defaults.info),
        modified: parse_status_style(flat, "modified").unwrap_or(defaults.modified),
        predictive: parse_status_style(flat, "predictive").unwrap_or(defaults.predictive),
        renamed: parse_status_style(flat, "renamed").unwrap_or(defaults.renamed),
        success: parse_status_style(flat, "success").unwrap_or(defaults.success),
        unreachable: parse_status_style(flat, "unreachable").unwrap_or(defaults.unreachable),
        warning: parse_status_style(flat, "warning").unwrap_or(defaults.warning),
    })
}

/// Parses a single StatusStyle from flattened keys.
/// Looks for `{name}`, `{name}.background` / `{name}_background`, `{name}.border` / `{name}_border`.
fn parse_status_style(flat: &HashMap<String, String>, name: &str) -> Option<StatusStyle> {
    let color_str = flat.get(name)?;
    let color = parse_color(color_str).ok()?;

    let bg_key_dot = format!("{name}.background");
    let bg_key_us = format!("{name}_background");
    let background = flat
        .get(&bg_key_dot)
        .or_else(|| flat.get(&bg_key_us))
        .and_then(|s| parse_color(s).ok())
        .unwrap_or_else(|| {
            let mut c = color;
            c.a = 0.1;
            c
        });

    let border_key_dot = format!("{name}.border");
    let border_key_us = format!("{name}_border");
    let border = flat
        .get(&border_key_dot)
        .or_else(|| flat.get(&border_key_us))
        .and_then(|s| parse_color(s).ok())
        .unwrap_or_else(|| {
            let mut c = color;
            c.a = 0.2;
            c
        });

    Some(StatusStyle {
        color,
        background,
        border,
    })
}

// ---------------------------------------------------------------------------
// Player colors parsing
// ---------------------------------------------------------------------------

fn parse_players(value: Option<&toml::Value>) -> Result<Vec<PlayerColor>> {
    let arr = match value {
        Some(toml::Value::Array(arr)) => arr,
        _ => return Ok(default_players()),
    };

    let mut players = Vec::with_capacity(arr.len());
    for (i, entry) in arr.iter().enumerate() {
        let table = entry
            .as_table()
            .ok_or_else(|| anyhow!("style.players[{i}] is not a table"))?;

        let cursor = table
            .get("cursor")
            .and_then(|v| v.as_str())
            .map(parse_color)
            .transpose()
            .with_context(|| format!("invalid cursor color in players[{i}]"))?
            .ok_or_else(|| anyhow!("players[{i}] missing 'cursor'"))?;

        let background = table
            .get("background")
            .and_then(|v| v.as_str())
            .map(parse_color)
            .transpose()
            .with_context(|| format!("invalid background color in players[{i}]"))?
            .ok_or_else(|| anyhow!("players[{i}] missing 'background'"))?;

        let selection = table
            .get("selection")
            .and_then(|v| v.as_str())
            .map(parse_color)
            .transpose()
            .with_context(|| format!("invalid selection color in players[{i}]"))?
            .ok_or_else(|| anyhow!("players[{i}] missing 'selection'"))?;

        players.push(PlayerColor {
            cursor,
            background,
            selection,
        });
    }

    if players.is_empty() {
        return Ok(default_players());
    }

    Ok(players)
}

// ---------------------------------------------------------------------------
// Syntax theme parsing
// ---------------------------------------------------------------------------

fn parse_syntax_theme(value: Option<&toml::Value>) -> Result<SyntaxTheme> {
    let table = match value {
        Some(toml::Value::Table(t)) => t,
        _ => return Ok(SyntaxTheme::empty()),
    };

    let mut highlights = HashMap::new();

    for (scope, entry) in table {
        let style = match entry {
            // Simple form: [style.syntax.keyword] with color/font_style/font_weight fields
            toml::Value::Table(t) => parse_highlight_style(t)?,
            // String shorthand: "keyword" = "#bb9af7"
            toml::Value::String(s) => {
                let color = parse_color(s)
                    .with_context(|| format!("invalid syntax color for scope '{scope}'"))?;
                HighlightStyle {
                    color: Some(color),
                    ..Default::default()
                }
            }
            _ => continue,
        };

        highlights.insert(scope.clone(), style);
    }

    Ok(SyntaxTheme::new(highlights))
}

fn parse_highlight_style(table: &toml::value::Table) -> Result<HighlightStyle> {
    let color = table
        .get("color")
        .and_then(|v| v.as_str())
        .map(parse_color)
        .transpose()?;

    let font_style = table
        .get("font_style")
        .and_then(|v| v.as_str())
        .map(|s| match s.to_lowercase().as_str() {
            "italic" => FontStyle::Italic,
            "oblique" => FontStyle::Oblique,
            _ => FontStyle::Normal,
        });

    let font_weight = table
        .get("font_weight")
        .and_then(|v| v.as_integer())
        .map(|w| FontWeight(w as f32));

    Ok(HighlightStyle {
        color,
        font_style,
        font_weight,
        ..Default::default()
    })
}

// ---------------------------------------------------------------------------
// Background image parsing
// ---------------------------------------------------------------------------

fn parse_background_image(value: Option<&toml::Value>) -> Result<Option<ThemeBackgroundImage>> {
    let table = match value {
        Some(toml::Value::Table(t)) => t,
        _ => return Ok(None),
    };

    let path = table
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("background_image missing 'path'"))?
        .to_string();

    let opacity = table
        .get("opacity")
        .and_then(|v| v.as_integer())
        .map(|v| v.clamp(0, 100) as u32)
        .unwrap_or(15);

    Ok(Some(ThemeBackgroundImage { path, opacity }))
}

// ---------------------------------------------------------------------------
// Defaults — base values for partial themes
// ---------------------------------------------------------------------------

/// Converts a hex u32 (0xRRGGBB) to Oklch.
fn hex(value: u32) -> Oklch {
    let r = ((value >> 16) & 0xFF) as f32 / 255.0;
    let g = ((value >> 8) & 0xFF) as f32 / 255.0;
    let b = (value & 0xFF) as f32 / 255.0;
    Oklch::from(Rgba { r, g, b, a: 1.0 })
}

fn hex_a(value: u32, alpha: f32) -> Oklch {
    let mut c = hex(value);
    c.a = alpha;
    c
}

/// Neutral gray base for ThemeColors — used as fallback for any field
/// not specified in a TOML theme file.
pub fn default_theme_colors() -> ThemeColors {
    let bg = hex(0x121212);
    let fg = hex(0xf1f1f1);
    let accent = hex(0x00BFFF);
    let surface = hex(0x1a1a1a);
    let elevated = hex(0x222222);
    let border_color = hex(0x333333);
    let muted = hex(0x888888);
    let disabled = hex(0x555555);
    let transparent = oklcha(0.0, 0.0, 0.0, 0.0);

    ThemeColors {
        border: border_color,
        border_variant: hex(0x2a2a2a),
        border_focused: accent,
        border_selected: accent,
        border_transparent: transparent,
        border_disabled: hex(0x2a2a2a),
        elevated_surface_background: elevated,
        surface_background: surface,
        background: bg,
        element_background: hex(0x1e1e1e),
        element_hover: hex(0x252525),
        element_active: hex(0x2a2a2a),
        element_selected: hex(0x2e2e2e),
        element_selection_background: hex_a(0x14F195, 0.15),
        element_disabled: hex(0x1a1a1a),
        drop_target_background: hex_a(0x14F195, 0.1),
        drop_target_border: accent,
        ghost_element_background: transparent,
        ghost_element_hover: hex_a(0xffffff, 0.05),
        ghost_element_active: hex_a(0xffffff, 0.08),
        ghost_element_selected: hex_a(0xffffff, 0.1),
        ghost_element_disabled: transparent,
        text: fg,
        text_muted: muted,
        text_placeholder: hex(0x666666),
        text_disabled: disabled,
        text_accent: accent,
        icon: fg,
        icon_muted: muted,
        icon_disabled: disabled,
        icon_placeholder: hex(0x666666),
        icon_accent: accent,
        status_bar_background: bg,
        title_bar_background: bg,
        title_bar_inactive_background: hex(0x101010),
        toolbar_background: bg,
        tab_bar_background: bg,
        tab_inactive_background: bg,
        tab_active_background: surface,
        search_match_background: hex_a(0x14F195, 0.2),
        search_active_match_background: hex_a(0x14F195, 0.35),
        panel_background: bg,
        panel_focused_border: accent,
        panel_indent_guide: hex(0x2a2a2a),
        panel_indent_guide_hover: hex(0x444444),
        panel_indent_guide_active: hex(0x555555),
        panel_overlay_background: hex_a(0x000000, 0.5),
        panel_overlay_hover: hex_a(0x000000, 0.6),
        pane_focused_border: accent,
        pane_group_border: border_color,
        scrollbar_thumb_background: hex_a(0xffffff, 0.1),
        scrollbar_thumb_hover_background: hex_a(0xffffff, 0.2),
        scrollbar_thumb_active_background: hex_a(0xffffff, 0.3),
        scrollbar_thumb_border: transparent,
        scrollbar_track_background: transparent,
        scrollbar_track_border: transparent,
        editor_foreground: fg,
        editor_background: bg,
        editor_gutter_background: bg,
        editor_subheader_background: surface,
        editor_active_line_background: hex_a(0xffffff, 0.03),
        editor_highlighted_line_background: hex_a(0xffffff, 0.05),
        editor_line_number: hex(0x555555),
        editor_active_line_number: fg,
        editor_invisible: hex(0x444444),
        editor_wrap_guide: hex(0x2a2a2a),
        editor_active_wrap_guide: hex(0x333333),
        editor_indent_guide: hex(0x2a2a2a),
        editor_indent_guide_active: hex(0x444444),
        editor_document_highlight_read_background: hex_a(0x14F195, 0.1),
        editor_document_highlight_write_background: hex_a(0x14F195, 0.15),
        editor_document_highlight_bracket_background: hex_a(0x14F195, 0.1),
        terminal_background: bg,
        terminal_foreground: fg,
        terminal_bright_foreground: hex(0xffffff),
        terminal_dim_foreground: hex(0x999999),
        terminal_accent: hex(0x00BFFF),
        terminal_ansi_background: bg,
        terminal_ansi_black: hex(0x1a1a2e),
        terminal_ansi_bright_black: hex(0x555555),
        terminal_ansi_dim_black: hex(0x111111),
        terminal_ansi_red: hex(0xff5555),
        terminal_ansi_bright_red: hex(0xff7777),
        terminal_ansi_dim_red: hex(0xcc4444),
        terminal_ansi_green: hex(0x14F195),
        terminal_ansi_bright_green: hex(0x50fa7b),
        terminal_ansi_dim_green: hex(0x10c070),
        terminal_ansi_yellow: hex(0xf1fa8c),
        terminal_ansi_bright_yellow: hex(0xffffb0),
        terminal_ansi_dim_yellow: hex(0xc0c870),
        terminal_ansi_blue: hex(0x6272a4),
        terminal_ansi_bright_blue: hex(0x8be9fd),
        terminal_ansi_dim_blue: hex(0x4e5a80),
        terminal_ansi_magenta: hex(0xff79c6),
        terminal_ansi_bright_magenta: hex(0xff99dd),
        terminal_ansi_dim_magenta: hex(0xcc60a0),
        terminal_ansi_cyan: hex(0x8be9fd),
        terminal_ansi_bright_cyan: hex(0xa4ffff),
        terminal_ansi_dim_cyan: hex(0x6eb8cc),
        terminal_ansi_white: hex(0xf8f8f2),
        terminal_ansi_bright_white: hex(0xffffff),
        terminal_ansi_dim_white: hex(0xbbbbbb),
        link_text_hover: accent,
        version_control_added: hex(0x14F195),
        version_control_deleted: hex(0xff5555),
        version_control_modified: hex(0x8be9fd),
        version_control_renamed: hex(0x6272a4),
        version_control_conflict: hex(0xff79c6),
        version_control_ignored: hex(0x555555),
        version_control_word_added: hex_a(0x14F195, 0.25),
        version_control_word_deleted: hex_a(0xff5555, 0.25),
        version_control_conflict_marker_ours: hex_a(0x14F195, 0.2),
        version_control_conflict_marker_theirs: hex_a(0x8be9fd, 0.2),
        block_success_badge: hex(0x14F195),
        block_error_badge: hex(0xff5555),
        block_running_badge: hex(0xf1fa8c),
    }
}

fn default_status_colors() -> StatusColors {
    StatusColors {
        conflict: StatusStyle {
            color: hex(0xff79c6),
            background: hex_a(0xff79c6, 0.15),
            border: hex_a(0xff79c6, 0.3),
        },
        created: StatusStyle {
            color: hex(0x14F195),
            background: hex_a(0x14F195, 0.15),
            border: hex_a(0x14F195, 0.3),
        },
        deleted: StatusStyle {
            color: hex(0xff5555),
            background: hex_a(0xff5555, 0.15),
            border: hex_a(0xff5555, 0.3),
        },
        error: StatusStyle {
            color: hex(0xff5555),
            background: hex_a(0xff5555, 0.15),
            border: hex_a(0xff5555, 0.3),
        },
        hidden: StatusStyle {
            color: hex(0x555555),
            background: hex_a(0x555555, 0.15),
            border: hex_a(0x555555, 0.3),
        },
        hint: StatusStyle {
            color: hex(0x6272a4),
            background: hex_a(0x6272a4, 0.15),
            border: hex_a(0x6272a4, 0.3),
        },
        ignored: StatusStyle {
            color: hex(0x555555),
            background: hex_a(0x555555, 0.15),
            border: hex_a(0x555555, 0.3),
        },
        info: StatusStyle {
            color: hex(0x8be9fd),
            background: hex_a(0x8be9fd, 0.15),
            border: hex_a(0x8be9fd, 0.3),
        },
        modified: StatusStyle {
            color: hex(0x8be9fd),
            background: hex_a(0x8be9fd, 0.15),
            border: hex_a(0x8be9fd, 0.3),
        },
        predictive: StatusStyle {
            color: hex(0x6272a4),
            background: hex_a(0x6272a4, 0.15),
            border: hex_a(0x6272a4, 0.3),
        },
        renamed: StatusStyle {
            color: hex(0x6272a4),
            background: hex_a(0x6272a4, 0.15),
            border: hex_a(0x6272a4, 0.3),
        },
        success: StatusStyle {
            color: hex(0x14F195),
            background: hex_a(0x14F195, 0.15),
            border: hex_a(0x14F195, 0.3),
        },
        unreachable: StatusStyle {
            color: hex(0x555555),
            background: hex_a(0x555555, 0.15),
            border: hex_a(0x555555, 0.3),
        },
        warning: StatusStyle {
            color: hex(0xf1fa8c),
            background: hex_a(0xf1fa8c, 0.15),
            border: hex_a(0xf1fa8c, 0.3),
        },
    }
}

fn default_players() -> Vec<PlayerColor> {
    let accent = hex(0x00BFFF);
    vec![
        PlayerColor {
            cursor: accent,
            background: hex_a(0x14F195, 0.2),
            selection: hex_a(0x14F195, 0.15),
        },
        PlayerColor {
            cursor: hex(0x8be9fd),
            background: hex_a(0x8be9fd, 0.2),
            selection: hex_a(0x8be9fd, 0.15),
        },
        PlayerColor {
            cursor: hex(0xff79c6),
            background: hex_a(0xff79c6, 0.2),
            selection: hex_a(0xff79c6, 0.15),
        },
        PlayerColor {
            cursor: hex(0xf1fa8c),
            background: hex_a(0xf1fa8c, 0.2),
            selection: hex_a(0xf1fa8c, 0.15),
        },
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color_hex6() {
        let c = parse_color("#ff0000").unwrap();
        assert!(c.l > 0.0);
        assert!(c.a > 0.99);
    }

    #[test]
    fn test_parse_color_hex8() {
        let c = parse_color("#ff000080").unwrap();
        assert!(c.a < 0.6);
        assert!(c.a > 0.4);
    }

    #[test]
    fn test_parse_color_hex3() {
        let c = parse_color("#f00").unwrap();
        assert!(c.l > 0.0);
    }

    #[test]
    fn test_load_minimal_theme() {
        let toml = r##"
[theme]
name = "Test Theme"
appearance = "dark"

[style]
background = "#1a1a1a"
text = "#ffffff"
"##;
        let theme = load_theme_from_toml("test", toml).unwrap();
        assert_eq!(theme.name.as_ref(), "Test Theme");
        assert_eq!(theme.appearance, Appearance::Dark);
    }

    #[test]
    fn test_load_theme_with_dotted_keys() {
        let toml = r##"
[theme]
name = "Dotted Test"
appearance = "dark"

[style]
"terminal.ansi.red" = "#ff5555"
"terminal.ansi.bright_red" = "#ff7777"
"editor.active_line.background" = "#1a1a1abf"
"##;
        let theme = load_theme_from_toml("dotted-test", toml).unwrap();
        // Verify the theme loaded without error
        assert_eq!(theme.id, "dotted-test");
    }

    #[test]
    fn test_load_theme_with_syntax() {
        let toml = r##"
[theme]
name = "Syntax Test"
appearance = "dark"

[style]

[style.syntax.keyword]
color = "#bb9af7"

[style.syntax.comment]
color = "#565f89"
font_style = "italic"

[style.syntax.title]
color = "#f7768e"
font_weight = 700
"##;
        let theme = load_theme_from_toml("syntax-test", toml).unwrap();
        let kw = theme.styles.syntax.get("keyword").unwrap();
        assert!(kw.color.is_some());
        let comment = theme.styles.syntax.get("comment").unwrap();
        assert_eq!(comment.font_style, Some(FontStyle::Italic));
        let title = theme.styles.syntax.get("title").unwrap();
        assert_eq!(title.font_weight, Some(FontWeight::BOLD));
    }

    #[test]
    fn test_load_theme_with_players() {
        let toml = r##"
[theme]
name = "Players Test"
appearance = "dark"

[style]

[[style.players]]
cursor = "#14F195"
background = "#14F195"
selection = "#14F1953d"

[[style.players]]
cursor = "#7aa2f7"
background = "#7aa2f7"
selection = "#7aa2f73d"
"##;
        let theme = load_theme_from_toml("players-test", toml).unwrap();
        assert_eq!(theme.styles.players.len(), 2);
    }

    #[test]
    fn test_load_theme_with_status() {
        let toml = r##"
[theme]
name = "Status Test"
appearance = "dark"

[style]
error = "#f7768e"
"error.background" = "#f7768e1a"
"error.border" = "#f7768e33"
success = "#14F195"
"##;
        let theme = load_theme_from_toml("status-test", toml).unwrap();
        // error should have all three specified
        assert!(theme.styles.status.error.color.l > 0.0);
        // success should have auto-derived background/border
        assert!(theme.styles.status.success.color.l > 0.0);
    }

    #[test]
    fn test_load_theme_with_background_image() {
        let toml = r##"
[theme]
name = "BG Image Test"
appearance = "dark"

[style]
background = "#121212"

[style.background_image]
path = "my-background.png"
opacity = 20
"##;
        let theme = load_theme_from_toml("bg-test", toml).unwrap();
        let bg = theme.styles.background_image.as_ref().unwrap();
        assert_eq!(bg.path, "my-background.png");
        assert_eq!(bg.opacity, 20);
    }
}
