use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use inazuma::{FontStyle, FontWeight, HighlightStyle, Oklch, Rgba, WindowBackgroundAppearance, oklcha, px};
use inazuma_refineable::Refineable;

use crate::accent::AccentColors;
use crate::colors::{
    BlockColors, BlockColorsRefinement, ChartColors, ChartColorsRefinement, EditorColors,
    EditorColorsRefinement, MinimapColors, MinimapColorsRefinement, PaneColors,
    PaneColorsRefinement, PanelColors, PanelColorsRefinement, ScrollbarColors,
    ScrollbarColorsRefinement, SearchColors, SearchColorsRefinement, StatusBarColors,
    StatusBarColorsRefinement, TabColors, TabColorsRefinement, TerminalAnsiColors,
    TerminalAnsiColorsRefinement, TerminalColors, TerminalColorsRefinement, ThemeColors,
    ThemeColorsRefinement, TitleBarColors, TitleBarColorsRefinement, ToolbarColors,
    ToolbarColorsRefinement, VersionControlColors, VersionControlColorsRefinement, VimColors,
    VimColorsRefinement,
};
use crate::players::{PlayerColor, PlayerColors};
use crate::status::{StatusColors, StatusStyle};
use crate::syntax::SyntaxTheme;
use crate::system::SystemColors;
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

    // Flatten all remaining keys (dots in quoted keys merge with nested tables)
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
            window_background_appearance: WindowBackgroundAppearance::Opaque,
            system: SystemColors::default(),
            accents: AccentColors(Arc::from(Vec::<Oklch>::new())),
            colors,
            status,
            syntax: Arc::new(syntax),
            players: PlayerColors(players),
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
/// - `oklch(l c h)` (e.g. `oklch(0.627 0.258 29.23)`)
/// - `oklch(l c h / a)` (e.g. `oklch(0.627 0.258 29.23 / 0.5)`)
/// - `oklch(l 0 none)` for achromatic colors (hue is undefined)
pub fn parse_color(s: &str) -> Result<Oklch> {
    let s = s.trim();

    if s.starts_with("oklch(") {
        return parse_oklch_functional(s);
    }

    if !s.starts_with('#') {
        bail!("unsupported color format (expected #hex or oklch(...)): '{s}'");
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

/// Parses `oklch(l c h)` or `oklch(l c h / a)` functional notation.
///
/// - `l`: lightness 0.0–1.0
/// - `c`: chroma 0.0–0.4 (typically)
/// - `h`: hue 0–360 (degrees) or `none` for achromatic
/// - `a`: alpha 0.0–1.0 (defaults to 1.0)
fn parse_oklch_functional(s: &str) -> Result<Oklch> {
    let inner = s
        .strip_prefix("oklch(")
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| anyhow!("malformed oklch(): '{s}'"))?
        .trim();

    // Split on '/' to separate color components from alpha
    let (color_part, alpha) = if let Some((cp, ap)) = inner.split_once('/') {
        let a: f32 = ap.trim().parse().context("invalid oklch alpha")?;
        (cp.trim(), a)
    } else {
        (inner, 1.0f32)
    };

    let parts: Vec<&str> = color_part.split_whitespace().collect();
    if parts.len() != 3 {
        bail!("oklch() requires 3 components (l c h), got {}: '{s}'", parts.len());
    }

    let l: f32 = parts[0].parse().context("invalid oklch lightness")?;
    let c: f32 = parts[1].parse().context("invalid oklch chroma")?;
    let h: f32 = if parts[2] == "none" {
        0.0
    } else {
        parts[2].parse().context("invalid oklch hue")?
    };

    Ok(Oklch { l, c, h, a: alpha })
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
                // Some keys might be integers (e.g. font_weight in syntax, radius)
                out.insert(full_key, i.to_string());
            }
            toml::Value::Float(f) => {
                out.insert(full_key, f.to_string());
            }
            _ => {
                // Skip arrays and other types at the flat color level
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ThemeColors parsing — prefix-based routing
// ---------------------------------------------------------------------------

/// Returns whether a flattened key belongs to StatusColors.
fn is_status_key(key: &str) -> bool {
    let root = key.split('.').next().unwrap_or(key);
    STATUS_PREFIXES.contains(&root)
}

/// Parses the flattened style table into ThemeColors.
fn parse_theme_colors(flat: &HashMap<String, String>) -> Result<ThemeColors> {
    let mut colors = default_theme_colors();
    let mut refinement = ThemeColorsRefinement::default();

    for (key, value) in flat {
        // Skip status keys — handled separately
        if is_status_key(key) {
            continue;
        }

        // Special case: radius is a number, not a color
        if key == "radius" {
            if let Ok(val) = value.parse::<f32>() {
                refinement.radius = Some(px(val));
            }
            continue;
        }

        let color = parse_color(value)
            .with_context(|| format!("invalid color for '{key}': '{value}'"))?;

        // Route by prefix (first dot-separated segment)
        if let Some((prefix, rest)) = key.split_once('.') {
            match prefix {
                "editor" => set_editor_field(&mut refinement.editor, rest, color),
                "terminal" => {
                    if let Some(ansi_rest) = rest.strip_prefix("ansi.") {
                        set_terminal_ansi_field(&mut refinement.terminal.ansi, ansi_rest, color);
                    } else {
                        set_terminal_field(&mut refinement.terminal, rest, color);
                    }
                }
                "panel" => set_panel_field(&mut refinement.panel, rest, color),
                "pane" => set_pane_field(&mut refinement.pane, rest, color),
                "tab" => set_tab_field(&mut refinement.tab, rest, color),
                "scrollbar" => set_scrollbar_field(&mut refinement.scrollbar, rest, color),
                "minimap" => set_minimap_field(&mut refinement.minimap, rest, color),
                "status_bar" => set_status_bar_field(&mut refinement.status_bar, rest, color),
                "title_bar" => set_title_bar_field(&mut refinement.title_bar, rest, color),
                "toolbar" => set_toolbar_field(&mut refinement.toolbar, rest, color),
                "search" => set_search_field(&mut refinement.search, rest, color),
                "vim" => set_vim_field(&mut refinement.vim, rest, color),
                "version_control" => set_version_control_field(&mut refinement.version_control, rest, color),
                "block" => set_block_field(&mut refinement.block, rest, color),
                "chart" => set_chart_field(&mut refinement.chart, rest, color),
                _ => {
                    log::trace!("unknown theme color prefix: '{prefix}' (key: '{key}')");
                }
            }
        } else {
            // No dot — top-level field
            set_toplevel_field(&mut refinement, key, color);
        }
    }

    colors.refine(&refinement);
    Ok(colors)
}

// ---------------------------------------------------------------------------
// Per-sub-struct field setters
// ---------------------------------------------------------------------------

macro_rules! match_fields {
    ($target:expr, $name:expr, $color:expr, $($field:ident),* $(,)?) => {
        match $name {
            $(stringify!($field) => { $target.$field = Some($color); })*
            other => {
                log::trace!("unknown theme color key: '{other}'");
            }
        }
    };
}

fn set_toplevel_field(refinement: &mut ThemeColorsRefinement, name: &str, color: Oklch) {
    match_fields!(refinement, name, color,
        // Semantic tokens
        primary,
        primary_foreground,
        secondary,
        secondary_foreground,
        muted,
        muted_foreground,
        accent,
        accent_foreground,
        destructive,
        destructive_foreground,
        background,
        foreground,
        card,
        card_foreground,
        popover,
        popover_foreground,
        border,
        input,
        ring,
        // Extended base tokens
        surface,
        elevated_surface,
        border_variant,
        border_focused,
        border_selected,
        border_transparent,
        border_disabled,
        element_background,
        element_hover,
        element_active,
        element_selected,
        element_disabled,
        element_selection,
        ghost_element_background,
        ghost_element_hover,
        ghost_element_active,
        ghost_element_selected,
        ghost_element_disabled,
        drop_target_background,
        drop_target_border,
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
        link_text_hover,
        debugger_accent,
    );
}

fn set_editor_field(refinement: &mut EditorColorsRefinement, name: &str, color: Oklch) {
    match_fields!(refinement, name, color,
        foreground,
        background,
        gutter_background,
        subheader_background,
        active_line_background,
        highlighted_line_background,
        debugger_active_line_background,
        line_number,
        active_line_number,
        hover_line_number,
        invisible,
        wrap_guide,
        active_wrap_guide,
        indent_guide,
        indent_guide_active,
        document_highlight_read_background,
        document_highlight_write_background,
        document_highlight_bracket_background,
    );
}

fn set_terminal_field(refinement: &mut TerminalColorsRefinement, name: &str, color: Oklch) {
    match_fields!(refinement, name, color,
        background,
        foreground,
        bright_foreground,
        dim_foreground,
        accent,
    );
}

fn set_terminal_ansi_field(refinement: &mut TerminalAnsiColorsRefinement, name: &str, color: Oklch) {
    match_fields!(refinement, name, color,
        background,
        black,
        bright_black,
        dim_black,
        red,
        bright_red,
        dim_red,
        green,
        bright_green,
        dim_green,
        yellow,
        bright_yellow,
        dim_yellow,
        blue,
        bright_blue,
        dim_blue,
        magenta,
        bright_magenta,
        dim_magenta,
        cyan,
        bright_cyan,
        dim_cyan,
        white,
        bright_white,
        dim_white,
    );
}

fn set_panel_field(refinement: &mut PanelColorsRefinement, name: &str, color: Oklch) {
    match_fields!(refinement, name, color,
        background,
        focused_border,
        indent_guide,
        indent_guide_hover,
        indent_guide_active,
        overlay_background,
        overlay_hover,
    );
}

fn set_pane_field(refinement: &mut PaneColorsRefinement, name: &str, color: Oklch) {
    match_fields!(refinement, name, color,
        focused_border,
        group_border,
    );
}

fn set_tab_field(refinement: &mut TabColorsRefinement, name: &str, color: Oklch) {
    match_fields!(refinement, name, color,
        bar_background,
        inactive_background,
        active_background,
        inactive_foreground,
        active_foreground,
    );
}

fn set_scrollbar_field(refinement: &mut ScrollbarColorsRefinement, name: &str, color: Oklch) {
    match_fields!(refinement, name, color,
        thumb_background,
        thumb_hover_background,
        thumb_active_background,
        thumb_border,
        track_background,
        track_border,
    );
}

fn set_minimap_field(refinement: &mut MinimapColorsRefinement, name: &str, color: Oklch) {
    match_fields!(refinement, name, color,
        thumb_background,
        thumb_hover_background,
        thumb_active_background,
        thumb_border,
    );
}

fn set_status_bar_field(refinement: &mut StatusBarColorsRefinement, name: &str, color: Oklch) {
    match_fields!(refinement, name, color,
        background,
    );
}

fn set_title_bar_field(refinement: &mut TitleBarColorsRefinement, name: &str, color: Oklch) {
    match_fields!(refinement, name, color,
        background,
        inactive_background,
    );
}

fn set_toolbar_field(refinement: &mut ToolbarColorsRefinement, name: &str, color: Oklch) {
    match_fields!(refinement, name, color,
        background,
    );
}

fn set_search_field(refinement: &mut SearchColorsRefinement, name: &str, color: Oklch) {
    match_fields!(refinement, name, color,
        match_background,
        active_match_background,
    );
}

fn set_vim_field(refinement: &mut VimColorsRefinement, name: &str, color: Oklch) {
    match_fields!(refinement, name, color,
        normal_background,
        insert_background,
        replace_background,
        visual_background,
        visual_line_background,
        visual_block_background,
        yank_background,
        helix_normal_background,
        helix_select_background,
        normal_foreground,
        insert_foreground,
        replace_foreground,
        visual_foreground,
        visual_line_foreground,
        visual_block_foreground,
        helix_normal_foreground,
        helix_select_foreground,
    );
}

fn set_version_control_field(refinement: &mut VersionControlColorsRefinement, name: &str, color: Oklch) {
    match_fields!(refinement, name, color,
        added,
        deleted,
        modified,
        renamed,
        conflict,
        ignored,
        word_added,
        word_deleted,
        conflict_marker_ours,
        conflict_marker_theirs,
    );
}

fn set_block_field(refinement: &mut BlockColorsRefinement, name: &str, color: Oklch) {
    match_fields!(refinement, name, color,
        success_badge,
        error_badge,
        running_badge,
    );
}

fn set_chart_field(refinement: &mut ChartColorsRefinement, name: &str, color: Oklch) {
    match_fields!(refinement, name, color,
        chart_1,
        chart_2,
        chart_3,
        chart_4,
        chart_5,
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
/// Looks for `{name}`, `{name}.background`, `{name}.border`.
fn parse_status_style(flat: &HashMap<String, String>, name: &str) -> Option<StatusStyle> {
    let color_str = flat.get(name)?;
    let color = parse_color(color_str).ok()?;

    let bg_key = format!("{name}.background");
    let background = flat
        .get(&bg_key)
        .and_then(|s| parse_color(s).ok())
        .unwrap_or_else(|| {
            let mut c = color;
            c.a = 0.1;
            c
        });

    let border_key = format!("{name}.border");
    let border = flat
        .get(&border_key)
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
        _ => return Ok(SyntaxTheme::default()),
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
        // Semantic tokens
        primary: accent,
        primary_foreground: fg,
        secondary: surface,
        secondary_foreground: fg,
        muted: hex(0x1e1e1e),
        muted_foreground: muted,
        accent,
        accent_foreground: fg,
        destructive: hex(0xff5555),
        destructive_foreground: fg,
        background: bg,
        foreground: fg,
        card: surface,
        card_foreground: fg,
        popover: elevated,
        popover_foreground: fg,
        border: border_color,
        input: hex(0x1e1e1e),
        ring: accent,

        // Extended base tokens
        surface,
        elevated_surface: elevated,
        border_variant: hex(0x2a2a2a),
        border_focused: accent,
        border_selected: accent,
        border_transparent: transparent,
        border_disabled: hex(0x2a2a2a),
        element_background: hex(0x1e1e1e),
        element_hover: hex(0x252525),
        element_active: hex(0x2a2a2a),
        element_selected: hex(0x2e2e2e),
        element_disabled: hex(0x1a1a1a),
        element_selection: hex_a(0x14F195, 0.15),
        ghost_element_background: transparent,
        ghost_element_hover: hex_a(0xffffff, 0.05),
        ghost_element_active: hex_a(0xffffff, 0.08),
        ghost_element_selected: hex_a(0xffffff, 0.1),
        ghost_element_disabled: transparent,
        drop_target_background: hex_a(0x14F195, 0.1),
        drop_target_border: accent,
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
        link_text_hover: accent,
        debugger_accent: hex(0xff9e64),

        // Contextual sub-structs
        editor: EditorColors {
            foreground: fg,
            background: bg,
            gutter_background: bg,
            subheader_background: surface,
            active_line_background: hex_a(0xffffff, 0.03),
            highlighted_line_background: hex_a(0xffffff, 0.05),
            debugger_active_line_background: hex_a(0xff9e64, 0.15),
            line_number: hex(0x555555),
            active_line_number: fg,
            hover_line_number: fg,
            invisible: hex(0x444444),
            wrap_guide: hex(0x2a2a2a),
            active_wrap_guide: hex(0x333333),
            indent_guide: hex(0x2a2a2a),
            indent_guide_active: hex(0x444444),
            document_highlight_read_background: hex_a(0x14F195, 0.1),
            document_highlight_write_background: hex_a(0x14F195, 0.15),
            document_highlight_bracket_background: hex_a(0x14F195, 0.1),
        },
        terminal: TerminalColors {
            background: bg,
            foreground: fg,
            bright_foreground: hex(0xffffff),
            dim_foreground: hex(0x999999),
            accent: hex(0x00BFFF),
            ansi: TerminalAnsiColors {
                background: bg,
                black: hex(0x1a1a2e),
                bright_black: hex(0x555555),
                dim_black: hex(0x111111),
                red: hex(0xff5555),
                bright_red: hex(0xff7777),
                dim_red: hex(0xcc4444),
                green: hex(0x14F195),
                bright_green: hex(0x50fa7b),
                dim_green: hex(0x10c070),
                yellow: hex(0xf1fa8c),
                bright_yellow: hex(0xffffb0),
                dim_yellow: hex(0xc0c870),
                blue: hex(0x6272a4),
                bright_blue: hex(0x8be9fd),
                dim_blue: hex(0x4e5a80),
                magenta: hex(0xff79c6),
                bright_magenta: hex(0xff99dd),
                dim_magenta: hex(0xcc60a0),
                cyan: hex(0x8be9fd),
                bright_cyan: hex(0xa4ffff),
                dim_cyan: hex(0x6eb8cc),
                white: hex(0xf8f8f2),
                bright_white: hex(0xffffff),
                dim_white: hex(0xbbbbbb),
            },
        },
        panel: PanelColors {
            background: bg,
            focused_border: accent,
            indent_guide: hex(0x2a2a2a),
            indent_guide_hover: hex(0x444444),
            indent_guide_active: hex(0x555555),
            overlay_background: hex_a(0x000000, 0.5),
            overlay_hover: hex_a(0x000000, 0.6),
        },
        pane: PaneColors {
            focused_border: accent,
            group_border: border_color,
        },
        tab: TabColors {
            bar_background: bg,
            inactive_background: bg,
            active_background: surface,
            inactive_foreground: muted,
            active_foreground: fg,
        },
        scrollbar: ScrollbarColors {
            thumb_background: hex_a(0xffffff, 0.1),
            thumb_hover_background: hex_a(0xffffff, 0.2),
            thumb_active_background: hex_a(0xffffff, 0.3),
            thumb_border: transparent,
            track_background: transparent,
            track_border: transparent,
        },
        minimap: MinimapColors {
            thumb_background: hex_a(0xffffff, 0.1),
            thumb_hover_background: hex_a(0xffffff, 0.15),
            thumb_active_background: hex_a(0xffffff, 0.2),
            thumb_border: transparent,
        },
        status_bar: StatusBarColors {
            background: bg,
        },
        title_bar: TitleBarColors {
            background: bg,
            inactive_background: hex(0x101010),
        },
        toolbar: ToolbarColors {
            background: bg,
        },
        search: SearchColors {
            match_background: hex_a(0x14F195, 0.2),
            active_match_background: hex_a(0x14F195, 0.35),
        },
        vim: VimColors {
            normal_background: hex(0x6272a4),
            insert_background: hex(0x14F195),
            replace_background: hex(0xff5555),
            visual_background: hex(0xff79c6),
            visual_line_background: hex(0xff79c6),
            visual_block_background: hex(0xff79c6),
            yank_background: hex(0xf1fa8c),
            helix_normal_background: hex(0x6272a4),
            helix_select_background: hex(0x8be9fd),
            normal_foreground: fg,
            insert_foreground: hex(0x121212),
            replace_foreground: fg,
            visual_foreground: hex(0x121212),
            visual_line_foreground: hex(0x121212),
            visual_block_foreground: hex(0x121212),
            helix_normal_foreground: fg,
            helix_select_foreground: hex(0x121212),
        },
        version_control: VersionControlColors {
            added: hex(0x14F195),
            deleted: hex(0xff5555),
            modified: hex(0x8be9fd),
            renamed: hex(0x6272a4),
            conflict: hex(0xff79c6),
            ignored: hex(0x555555),
            word_added: hex_a(0x14F195, 0.25),
            word_deleted: hex_a(0xff5555, 0.25),
            conflict_marker_ours: hex_a(0x14F195, 0.2),
            conflict_marker_theirs: hex_a(0x8be9fd, 0.2),
        },
        radius: px(8.0),
        block: BlockColors {
            success_badge: hex(0x14F195),
            error_badge: hex(0xff5555),
            running_badge: hex(0xf1fa8c),
        },
        chart: ChartColors {
            chart_1: accent,
            chart_2: hex(0x14F195),
            chart_3: hex(0xff79c6),
            chart_4: hex(0xf1fa8c),
            chart_5: hex(0x6272a4),
        },
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
    fn test_parse_color_oklch() {
        let c = parse_color("oklch(0.627 0.258 29.23)").unwrap();
        assert!((c.l - 0.627).abs() < 0.001);
        assert!((c.c - 0.258).abs() < 0.001);
        assert!((c.h - 29.23).abs() < 0.01);
        assert!((c.a - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_color_oklch_with_alpha() {
        let c = parse_color("oklch(0.627 0.258 29.23 / 0.5)").unwrap();
        assert!((c.l - 0.627).abs() < 0.001);
        assert!((c.a - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_parse_color_oklch_achromatic() {
        let c = parse_color("oklch(0.156 0 none)").unwrap();
        assert!((c.l - 0.156).abs() < 0.001);
        assert!((c.c - 0.0).abs() < 0.001);
        assert!((c.h - 0.0).abs() < 0.001);
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
"editor.active_line_background" = "#1a1a1abf"
"##;
        let theme = load_theme_from_toml("dotted-test", toml).unwrap();
        assert_eq!(theme.id, "dotted-test");
    }

    #[test]
    fn test_load_theme_with_nested_tables() {
        let toml = r##"
[theme]
name = "Nested Test"
appearance = "dark"

[style.terminal.ansi]
red = "#ff5555"
bright_red = "#ff7777"

[style.editor]
active_line_background = "#1a1a1abf"
foreground = "#ffffff"
"##;
        let theme = load_theme_from_toml("nested-test", toml).unwrap();
        assert_eq!(theme.id, "nested-test");
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
        let kw = theme.styles.syntax.style_for_name("keyword").unwrap();
        assert!(kw.color.is_some());
        let comment = theme.styles.syntax.style_for_name("comment").unwrap();
        assert_eq!(comment.font_style, Some(FontStyle::Italic));
        let title = theme.styles.syntax.style_for_name("title").unwrap();
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

    #[test]
    fn test_load_theme_with_radius() {
        let toml = r##"
[theme]
name = "Radius Test"
appearance = "dark"

[style]
radius = 12
"##;
        let theme = load_theme_from_toml("radius-test", toml).unwrap();
        assert!((theme.styles.colors.radius.0 - 12.0).abs() < 0.001);
    }
}
