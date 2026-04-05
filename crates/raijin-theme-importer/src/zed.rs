use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use inazuma::{FontStyle, FontWeight, HighlightStyle, Oklch, Rgba, SharedString};
use serde::Deserialize;

use raijin_theme::{
    Appearance, PlayerColor, StatusColors, StatusStyle, SyntaxTheme, Theme, ThemeColors,
    ThemeFamily, ThemeStyles,
};

// === Zed Theme JSON Schema ===

#[derive(Deserialize)]
struct ZedThemeFamily {
    name: String,
    author: String,
    themes: Vec<ZedTheme>,
}

#[derive(Deserialize)]
struct ZedTheme {
    name: String,
    appearance: ZedAppearance,
    style: ZedStyle,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ZedAppearance {
    Light,
    Dark,
}

#[derive(Deserialize)]
struct ZedStyle {
    #[serde(flatten)]
    colors: HashMap<String, serde_json::Value>,
    #[serde(default)]
    syntax: HashMap<String, ZedSyntaxStyle>,
    #[serde(default)]
    players: Vec<ZedPlayer>,
}

#[derive(Deserialize)]
struct ZedSyntaxStyle {
    color: Option<String>,
    font_weight: Option<f32>,
    font_style: Option<String>,
}

#[derive(Deserialize)]
struct ZedPlayer {
    cursor: Option<String>,
    background: Option<String>,
    selection: Option<String>,
}

// === Color Parsing ===

/// Parses a hex color string (#rrggbb or #rrggbbaa) into Oklch.
fn parse_color(hex_str: &str) -> Result<Oklch> {
    let rgba = Rgba::try_from(hex_str)
        .with_context(|| format!("failed to parse color '{hex_str}'"))?;
    Ok(Oklch::from(rgba))
}

/// Tries to parse a color from a JSON value (string). Returns None for null/missing.
fn try_color(value: Option<&serde_json::Value>) -> Option<Oklch> {
    value
        .and_then(|v| v.as_str())
        .and_then(|s| parse_color(s).ok())
}

/// Gets a color from the style map, returning a fallback if not present.
fn color_or(colors: &HashMap<String, serde_json::Value>, key: &str, fallback: Oklch) -> Oklch {
    try_color(colors.get(key)).unwrap_or(fallback)
}

// === Import Entry Point ===

/// Imports a Zed theme JSON file and converts it to a raijin ThemeFamily.
///
/// Zed themes use `#rrggbbaa` hex color strings. The token names in Zed's style
/// object use dot-separated paths (e.g. `terminal.ansi.red`) which map directly
/// to raijin-theme's ThemeColors field names with dots replaced by underscores.
pub fn import_zed_theme(json: &str) -> Result<ThemeFamily> {
    let zed_family: ZedThemeFamily =
        serde_json::from_str(json).context("failed to deserialize Zed theme JSON")?;

    if zed_family.themes.is_empty() {
        bail!("Zed theme family '{}' contains no themes", zed_family.name);
    }

    let themes: Vec<Theme> = zed_family
        .themes
        .into_iter()
        .map(|zed_theme| convert_zed_theme(&zed_theme, &zed_family.name))
        .collect::<Result<Vec<_>>>()?;

    Ok(ThemeFamily {
        id: slug(&zed_family.name),
        name: SharedString::from(zed_family.name.clone()),
        author: zed_family.author,
        themes,
    })
}

fn slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect()
}

fn convert_zed_theme(theme: &ZedTheme, _family_name: &str) -> Result<Theme> {
    let appearance = match theme.appearance {
        ZedAppearance::Light => Appearance::Light,
        ZedAppearance::Dark => Appearance::Dark,
    };

    let colors = convert_theme_colors(&theme.style.colors, appearance);
    let syntax = convert_syntax(&theme.style.syntax);
    let players = convert_players(&theme.style.players, &colors);
    let status = derive_status_colors(&theme.style.colors, &colors);

    Ok(Theme {
        id: slug(&theme.name),
        name: SharedString::from(theme.name.clone()),
        appearance,
        styles: ThemeStyles {
            colors,
            status,
            syntax,
            players,
        },
    })
}

// === ThemeColors Conversion ===

/// Maps Zed's dot-separated color keys to ThemeColors fields.
/// Zed uses keys like "border", "text", "terminal.ansi.red" etc.
/// The raijin ThemeColors field names use underscores instead of dots.
fn convert_theme_colors(colors: &HashMap<String, serde_json::Value>, appearance: Appearance) -> ThemeColors {
    // Default colors based on appearance
    let (default_bg, default_fg, default_border, default_surface, default_muted) = match appearance {
        Appearance::Dark => (
            parse_color("#121212ff").unwrap_or_default(),
            parse_color("#f1f1f1ff").unwrap_or_default(),
            parse_color("#333333ff").unwrap_or_default(),
            parse_color("#1a1a1aff").unwrap_or_default(),
            parse_color("#888888ff").unwrap_or_default(),
        ),
        Appearance::Light => (
            parse_color("#ffffffff").unwrap_or_default(),
            parse_color("#1a1a1aff").unwrap_or_default(),
            parse_color("#d0d0d0ff").unwrap_or_default(),
            parse_color("#f5f5f5ff").unwrap_or_default(),
            parse_color("#666666ff").unwrap_or_default(),
        ),
    };

    let transparent = Oklch { l: 0.0, c: 0.0, h: 0.0, a: 0.0 };
    let disabled = default_muted;

    let bg = color_or(colors, "background", default_bg);
    let fg = color_or(colors, "text", default_fg);
    let border = color_or(colors, "border", default_border);
    let surface = color_or(colors, "surface.background", default_surface);
    let elevated = color_or(colors, "elevated_surface.background", surface);

    ThemeColors {
        // Borders
        border,
        border_variant: color_or(colors, "border.variant", border),
        border_focused: color_or(colors, "border.focused", border),
        border_selected: color_or(colors, "border.selected", border),
        border_transparent: color_or(colors, "border.transparent", transparent),
        border_disabled: color_or(colors, "border.disabled", disabled),

        // Surfaces
        elevated_surface_background: elevated,
        surface_background: surface,
        background: bg,
        element_background: color_or(colors, "element.background", surface),

        // Element states
        element_hover: color_or(colors, "element.hover", surface),
        element_active: color_or(colors, "element.active", surface),
        element_selected: color_or(colors, "element.selected", surface),
        element_selection_background: color_or(colors, "element.selection_background", surface),
        element_disabled: color_or(colors, "element.disabled", surface),
        drop_target_background: color_or(colors, "drop_target.background", surface),
        drop_target_border: color_or(colors, "drop_target.border", border),
        ghost_element_background: color_or(colors, "ghost_element.background", transparent),
        ghost_element_hover: color_or(colors, "ghost_element.hover", surface),
        ghost_element_active: color_or(colors, "ghost_element.active", surface),
        ghost_element_selected: color_or(colors, "ghost_element.selected", surface),
        ghost_element_disabled: color_or(colors, "ghost_element.disabled", transparent),

        // Text
        text: fg,
        text_muted: color_or(colors, "text.muted", default_muted),
        text_placeholder: color_or(colors, "text.placeholder", default_muted),
        text_disabled: color_or(colors, "text.disabled", disabled),
        text_accent: color_or(colors, "text.accent", fg),

        // Icons
        icon: color_or(colors, "icon", fg),
        icon_muted: color_or(colors, "icon.muted", default_muted),
        icon_disabled: color_or(colors, "icon.disabled", disabled),
        icon_placeholder: color_or(colors, "icon.placeholder", default_muted),
        icon_accent: color_or(colors, "icon.accent", fg),

        // Workspace chrome
        status_bar_background: color_or(colors, "status_bar.background", bg),
        title_bar_background: color_or(colors, "title_bar.background", bg),
        title_bar_inactive_background: color_or(colors, "title_bar.inactive_background", bg),
        toolbar_background: color_or(colors, "toolbar.background", bg),
        tab_bar_background: color_or(colors, "tab_bar.background", bg),
        tab_inactive_background: color_or(colors, "tab.inactive_background", bg),
        tab_active_background: color_or(colors, "tab.active_background", surface),
        search_match_background: color_or(colors, "search.match_background", surface),
        search_active_match_background: color_or(colors, "search.active_match_background", surface),
        panel_background: color_or(colors, "panel.background", bg),
        panel_focused_border: color_or(colors, "panel.focused_border", border),
        panel_indent_guide: color_or(colors, "panel.indent_guide", border),
        panel_indent_guide_hover: color_or(colors, "panel.indent_guide_hover", border),
        panel_indent_guide_active: color_or(colors, "panel.indent_guide_active", border),
        panel_overlay_background: color_or(colors, "panel.overlay_background", bg),
        panel_overlay_hover: color_or(colors, "panel.overlay_hover", bg),
        pane_focused_border: color_or(colors, "pane.focused_border", border),
        pane_group_border: color_or(colors, "pane_group.border", border),

        // Scrollbar
        scrollbar_thumb_background: color_or(colors, "scrollbar.thumb.background", default_muted),
        scrollbar_thumb_hover_background: color_or(colors, "scrollbar.thumb.hover_background", default_muted),
        scrollbar_thumb_active_background: color_or(colors, "scrollbar.thumb.active_background", default_muted),
        scrollbar_thumb_border: color_or(colors, "scrollbar.thumb.border", transparent),
        scrollbar_track_background: color_or(colors, "scrollbar.track.background", transparent),
        scrollbar_track_border: color_or(colors, "scrollbar.track.border", transparent),

        // Editor
        editor_foreground: color_or(colors, "editor.foreground", fg),
        editor_background: color_or(colors, "editor.background", bg),
        editor_gutter_background: color_or(colors, "editor.gutter.background", bg),
        editor_subheader_background: color_or(colors, "editor.subheader.background", surface),
        editor_active_line_background: color_or(colors, "editor.active_line.background", surface),
        editor_highlighted_line_background: color_or(colors, "editor.highlighted_line.background", surface),
        editor_line_number: color_or(colors, "editor.line_number", default_muted),
        editor_active_line_number: color_or(colors, "editor.active_line_number", fg),
        editor_invisible: color_or(colors, "editor.invisible", default_muted),
        editor_wrap_guide: color_or(colors, "editor.wrap_guide", border),
        editor_active_wrap_guide: color_or(colors, "editor.active_wrap_guide", border),
        editor_indent_guide: color_or(colors, "editor.indent_guide", border),
        editor_indent_guide_active: color_or(colors, "editor.indent_guide_active", border),
        editor_document_highlight_read_background: color_or(colors, "editor.document_highlight.read_background", surface),
        editor_document_highlight_write_background: color_or(colors, "editor.document_highlight.write_background", surface),
        editor_document_highlight_bracket_background: color_or(colors, "editor.document_highlight.bracket_background", surface),

        // Terminal ANSI
        terminal_background: color_or(colors, "terminal.background", bg),
        terminal_foreground: color_or(colors, "terminal.foreground", fg),
        terminal_bright_foreground: color_or(colors, "terminal.bright_foreground", fg),
        terminal_dim_foreground: color_or(colors, "terminal.dim_foreground", default_muted),
        terminal_accent: color_or(colors, "terminal.accent",
            color_or(colors, "text.accent", parse_color("#00BFFFff").unwrap_or_default())),
        terminal_ansi_background: color_or(colors, "terminal.ansi.background", bg),
        terminal_ansi_black: color_or(colors, "terminal.ansi.black", default_bg),
        terminal_ansi_bright_black: color_or(colors, "terminal.ansi.bright_black", default_muted),
        terminal_ansi_dim_black: color_or(colors, "terminal.ansi.dim_black", default_bg),
        terminal_ansi_red: color_or(colors, "terminal.ansi.red", parse_color("#ff5555ff").unwrap_or_default()),
        terminal_ansi_bright_red: color_or(colors, "terminal.ansi.bright_red", parse_color("#ff7777ff").unwrap_or_default()),
        terminal_ansi_dim_red: color_or(colors, "terminal.ansi.dim_red", parse_color("#cc4444ff").unwrap_or_default()),
        terminal_ansi_green: color_or(colors, "terminal.ansi.green", parse_color("#50fa7bff").unwrap_or_default()),
        terminal_ansi_bright_green: color_or(colors, "terminal.ansi.bright_green", parse_color("#50fa7bff").unwrap_or_default()),
        terminal_ansi_dim_green: color_or(colors, "terminal.ansi.dim_green", parse_color("#10c070ff").unwrap_or_default()),
        terminal_ansi_yellow: color_or(colors, "terminal.ansi.yellow", parse_color("#f1fa8cff").unwrap_or_default()),
        terminal_ansi_bright_yellow: color_or(colors, "terminal.ansi.bright_yellow", parse_color("#ffffb0ff").unwrap_or_default()),
        terminal_ansi_dim_yellow: color_or(colors, "terminal.ansi.dim_yellow", parse_color("#c0c870ff").unwrap_or_default()),
        terminal_ansi_blue: color_or(colors, "terminal.ansi.blue", parse_color("#6272a4ff").unwrap_or_default()),
        terminal_ansi_bright_blue: color_or(colors, "terminal.ansi.bright_blue", parse_color("#8be9fdff").unwrap_or_default()),
        terminal_ansi_dim_blue: color_or(colors, "terminal.ansi.dim_blue", parse_color("#4e5a80ff").unwrap_or_default()),
        terminal_ansi_magenta: color_or(colors, "terminal.ansi.magenta", parse_color("#ff79c6ff").unwrap_or_default()),
        terminal_ansi_bright_magenta: color_or(colors, "terminal.ansi.bright_magenta", parse_color("#ff99ddff").unwrap_or_default()),
        terminal_ansi_dim_magenta: color_or(colors, "terminal.ansi.dim_magenta", parse_color("#cc60a0ff").unwrap_or_default()),
        terminal_ansi_cyan: color_or(colors, "terminal.ansi.cyan", parse_color("#8be9fdff").unwrap_or_default()),
        terminal_ansi_bright_cyan: color_or(colors, "terminal.ansi.bright_cyan", parse_color("#a4ffffff").unwrap_or_default()),
        terminal_ansi_dim_cyan: color_or(colors, "terminal.ansi.dim_cyan", parse_color("#6eb8ccff").unwrap_or_default()),
        terminal_ansi_white: color_or(colors, "terminal.ansi.white", parse_color("#f8f8f2ff").unwrap_or_default()),
        terminal_ansi_bright_white: color_or(colors, "terminal.ansi.bright_white", parse_color("#ffffffff").unwrap_or_default()),
        terminal_ansi_dim_white: color_or(colors, "terminal.ansi.dim_white", parse_color("#bbbbbbff").unwrap_or_default()),

        // Links
        link_text_hover: color_or(colors, "link_text.hover", fg),

        // Version control
        version_control_added: color_or(colors, "version_control.added", parse_color("#14F195ff").unwrap_or_default()),
        version_control_deleted: color_or(colors, "version_control.deleted", parse_color("#ff5555ff").unwrap_or_default()),
        version_control_modified: color_or(colors, "version_control.modified", parse_color("#8be9fdff").unwrap_or_default()),
        version_control_renamed: color_or(colors, "version_control.renamed", parse_color("#6272a4ff").unwrap_or_default()),
        version_control_conflict: color_or(colors, "version_control.conflict", parse_color("#ff79c6ff").unwrap_or_default()),
        version_control_ignored: color_or(colors, "version_control.ignored", parse_color("#555555ff").unwrap_or_default()),
        version_control_word_added: color_or(colors, "version_control.word_added", parse_color("#14F19540").unwrap_or_default()),
        version_control_word_deleted: color_or(colors, "version_control.word_deleted", parse_color("#ff555540").unwrap_or_default()),
        version_control_conflict_marker_ours: color_or(colors, "version_control.conflict_marker_ours", parse_color("#14F19533").unwrap_or_default()),
        version_control_conflict_marker_theirs: color_or(colors, "version_control.conflict_marker_theirs", parse_color("#8be9fd33").unwrap_or_default()),

        // Raijin-specific (derived from other colors since Zed themes won't have these)
        block_success_badge: color_or(colors, "block_success.badge",
            color_or(colors, "terminal.ansi.green", parse_color("#14F195ff").unwrap_or_default())),
        block_error_badge: color_or(colors, "block_error.badge",
            color_or(colors, "terminal.ansi.red", parse_color("#ff5555ff").unwrap_or_default())),
        block_running_badge: color_or(colors, "block_running.badge",
            color_or(colors, "terminal.ansi.yellow", parse_color("#f1fa8cff").unwrap_or_default())),
    }
}

// === Syntax Conversion ===

fn convert_syntax(syntax: &HashMap<String, ZedSyntaxStyle>) -> SyntaxTheme {
    let mut highlights = HashMap::new();

    for (scope, style) in syntax {
        let color = style.color.as_deref().and_then(|s| parse_color(s).ok());
        let font_weight = style.font_weight.map(FontWeight);
        let font_style = style.font_style.as_deref().map(parse_font_style);

        let highlight = HighlightStyle {
            color,
            font_weight,
            font_style,
            background_color: None,
            underline: None,
            strikethrough: None,
            fade_out: None,
        };

        highlights.insert(scope.clone(), highlight);
    }

    SyntaxTheme::new(highlights)
}

fn parse_font_style(s: &str) -> FontStyle {
    match s {
        "italic" => FontStyle::Italic,
        "oblique" => FontStyle::Oblique,
        _ => FontStyle::Normal,
    }
}

// === Player Colors ===

fn convert_players(players: &[ZedPlayer], colors: &ThemeColors) -> Vec<PlayerColor> {
    if players.is_empty() {
        return default_players(colors);
    }

    players
        .iter()
        .map(|p| {
            let cursor = p
                .cursor
                .as_deref()
                .and_then(|s| parse_color(s).ok())
                .unwrap_or(colors.text);
            let background = p
                .background
                .as_deref()
                .and_then(|s| parse_color(s).ok())
                .unwrap_or(colors.background);
            let selection = p
                .selection
                .as_deref()
                .and_then(|s| parse_color(s).ok())
                .unwrap_or(colors.element_selection_background);
            PlayerColor {
                cursor,
                background,
                selection,
            }
        })
        .collect()
}

fn default_players(colors: &ThemeColors) -> Vec<PlayerColor> {
    vec![PlayerColor {
        cursor: colors.text,
        background: colors.background,
        selection: colors.element_selection_background,
    }]
}

// === Status Colors ===

fn derive_status_colors(
    colors: &HashMap<String, serde_json::Value>,
    theme_colors: &ThemeColors,
) -> StatusColors {
    let make_status = |key: &str, fallback: Oklch| -> StatusStyle {
        let base = color_or(colors, key, fallback);
        let mut bg = base;
        bg.a = 0.15;
        let mut brd = base;
        brd.a = 0.3;
        StatusStyle {
            color: base,
            background: bg,
            border: brd,
        }
    };

    StatusColors {
        conflict: make_status("conflict", theme_colors.version_control_conflict),
        created: make_status("created", theme_colors.version_control_added),
        deleted: make_status("deleted", theme_colors.version_control_deleted),
        error: make_status("error", theme_colors.version_control_deleted),
        hidden: make_status("hidden", theme_colors.text_muted),
        hint: make_status("hint", theme_colors.text_muted),
        ignored: make_status("ignored", theme_colors.text_muted),
        info: make_status("info", theme_colors.version_control_modified),
        modified: make_status("modified", theme_colors.version_control_modified),
        predictive: make_status("predictive", theme_colors.text_muted),
        renamed: make_status("renamed", theme_colors.version_control_renamed),
        success: make_status("success", theme_colors.version_control_added),
        unreachable: make_status("unreachable", theme_colors.text_muted),
        warning: make_status("warning", theme_colors.terminal_ansi_yellow),
    }
}
