use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use inazuma::{FontStyle, FontWeight, HighlightStyle, Oklch, Rgba, SharedString};
use serde::Deserialize;

use raijin_theme::{
    AccentColors, Appearance, PlayerColor, PlayerColors, StatusColors, StatusStyle,
    SyntaxTheme, SystemColors, Theme, ThemeColors, ThemeFamily, ThemeStyles,
};

// === VS Code Theme JSON Schema ===

#[derive(Deserialize)]
struct VsCodeTheme {
    name: Option<String>,
    #[serde(rename = "type")]
    theme_type: Option<String>,
    colors: Option<HashMap<String, serde_json::Value>>,
    #[serde(rename = "tokenColors")]
    token_colors: Option<Vec<VsCodeTokenColor>>,
}

#[derive(Deserialize)]
struct VsCodeTokenColor {
    scope: Option<VsCodeTokenScope>,
    settings: VsCodeTokenSettings,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum VsCodeTokenScope {
    One(String),
    Many(Vec<String>),
}

#[derive(Deserialize)]
struct VsCodeTokenSettings {
    foreground: Option<String>,
    #[serde(rename = "fontStyle")]
    font_style: Option<String>,
}

// === Color Parsing ===

fn parse_color(hex_str: &str) -> Result<Oklch> {
    // VS Code colors can be #rgb, #rrggbb, or #rrggbbaa
    let rgba = Rgba::try_from(hex_str)
        .with_context(|| format!("failed to parse color '{hex_str}'"))?;
    Ok(Oklch::from(rgba))
}

fn try_color_from_map(colors: &HashMap<String, serde_json::Value>, key: &str) -> Option<Oklch> {
    colors
        .get(key)
        .and_then(|v| v.as_str())
        .and_then(|s| parse_color(s).ok())
}

fn color_or_map(colors: &HashMap<String, serde_json::Value>, key: &str, fallback: Oklch) -> Oklch {
    try_color_from_map(colors, key).unwrap_or(fallback)
}

// === Import Entry Point ===

/// Imports a VS Code theme JSON file and converts it to a raijin ThemeFamily.
///
/// VS Code themes use a different token name structure than Zed. The `colors` object
/// contains UI colors with keys like `editor.background`, `terminal.ansiRed`, etc.
/// The `tokenColors` array provides syntax highlighting via TextMate scopes.
pub fn import_vscode_theme(json: &str) -> Result<ThemeFamily> {
    let vscode_theme: VsCodeTheme =
        serde_json::from_str(json).context("failed to deserialize VS Code theme JSON")?;

    let name = vscode_theme
        .name
        .as_deref()
        .unwrap_or("Imported VS Code Theme");

    let appearance = match vscode_theme.theme_type.as_deref() {
        Some("light") | Some("hc-light") => Appearance::Light,
        _ => Appearance::Dark,
    };

    let ui_colors = vscode_theme.colors.unwrap_or_default();
    let token_colors = vscode_theme.token_colors.unwrap_or_default();

    let theme_colors = convert_vscode_colors(&ui_colors, appearance);
    let syntax = convert_vscode_syntax(&token_colors);
    let status = derive_status_colors(&ui_colors, &theme_colors);
    let players = vec![PlayerColor {
        cursor: theme_colors.text,
        background: theme_colors.background,
        selection: theme_colors.element_selection_background,
    }];

    let theme = Theme {
        id: slug(name),
        name: SharedString::from(name.to_owned()),
        appearance,
        styles: ThemeStyles {
            system: SystemColors::default(),
            accents: AccentColors(Arc::from(Vec::<Oklch>::new())),
            colors: theme_colors,
            status,
            syntax: Arc::new(syntax),
            players: PlayerColors(players),
            background_image: None,
        },
        base_dir: None,
    };

    Ok(ThemeFamily {
        id: slug(name),
        name: SharedString::from(name.to_owned()),
        author: String::new(),
        themes: vec![theme],
    })
}

fn slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect()
}

// === VS Code to Raijin Color Mapping ===
//
// VS Code uses camelCase keys like `editor.background`, `terminal.ansiRed`.
// Raijin uses snake_case field names. This function maps between them.

fn convert_vscode_colors(colors: &HashMap<String, serde_json::Value>, appearance: Appearance) -> ThemeColors {
    let (default_bg, default_fg, default_border, default_surface, default_muted) = match appearance {
        Appearance::Dark => (
            parse_color("#1e1e1eff").unwrap_or_default(),
            parse_color("#d4d4d4ff").unwrap_or_default(),
            parse_color("#474747ff").unwrap_or_default(),
            parse_color("#252526ff").unwrap_or_default(),
            parse_color("#808080ff").unwrap_or_default(),
        ),
        Appearance::Light => (
            parse_color("#ffffffff").unwrap_or_default(),
            parse_color("#1e1e1eff").unwrap_or_default(),
            parse_color("#c8c8c8ff").unwrap_or_default(),
            parse_color("#f3f3f3ff").unwrap_or_default(),
            parse_color("#767676ff").unwrap_or_default(),
        ),
    };

    let transparent = Oklch { l: 0.0, c: 0.0, h: 0.0, a: 0.0 };

    let bg = color_or_map(colors, "editor.background", default_bg);
    let fg = color_or_map(colors, "editor.foreground", default_fg);
    let border = color_or_map(colors, "panel.border", default_border);
    let surface = color_or_map(colors, "sideBar.background", default_surface);
    let elevated = color_or_map(colors, "editorWidget.background", surface);
    let muted = color_or_map(colors, "descriptionForeground", default_muted);
    let disabled = default_muted;
    let selection_bg = color_or_map(colors, "editor.selectionBackground", surface);
    let active_bg = color_or_map(colors, "list.activeSelectionBackground", surface);
    let hover_bg = color_or_map(colors, "list.hoverBackground", surface);

    ThemeColors {
        // Borders
        border,
        border_variant: color_or_map(colors, "editorGroup.border", border),
        border_focused: color_or_map(colors, "focusBorder", border),
        border_selected: color_or_map(colors, "focusBorder", border),
        border_transparent: transparent,
        border_disabled: disabled,

        // Surfaces
        elevated_surface_background: elevated,
        surface_background: surface,
        background: bg,
        element_background: color_or_map(colors, "button.background", surface),

        // Element states
        element_hover: hover_bg,
        element_active: active_bg,
        element_selected: active_bg,
        element_selection_background: selection_bg,
        element_disabled: surface,
        drop_target_background: color_or_map(colors, "list.dropBackground", surface),
        drop_target_border: border,
        ghost_element_background: transparent,
        ghost_element_hover: hover_bg,
        ghost_element_active: active_bg,
        ghost_element_selected: active_bg,
        ghost_element_disabled: transparent,

        // Text
        text: fg,
        text_muted: muted,
        text_placeholder: color_or_map(colors, "input.placeholderForeground", muted),
        text_disabled: disabled,
        text_accent: color_or_map(colors, "textLink.foreground", fg),

        // Icons
        icon: color_or_map(colors, "icon.foreground", fg),
        icon_muted: muted,
        icon_disabled: disabled,
        icon_placeholder: muted,
        icon_accent: color_or_map(colors, "textLink.foreground", fg),

        // Workspace chrome
        status_bar_background: color_or_map(colors, "statusBar.background", bg),
        title_bar_background: color_or_map(colors, "titleBar.activeBackground", bg),
        title_bar_inactive_background: color_or_map(colors, "titleBar.inactiveBackground", bg),
        toolbar_background: bg,
        tab_bar_background: color_or_map(colors, "editorGroupHeader.tabsBackground", bg),
        tab_inactive_background: color_or_map(colors, "tab.inactiveBackground", bg),
        tab_active_background: color_or_map(colors, "tab.activeBackground", surface),
        search_match_background: color_or_map(colors, "editor.findMatchHighlightBackground", surface),
        search_active_match_background: color_or_map(colors, "editor.findMatchBackground", surface),
        panel_background: color_or_map(colors, "panel.background", bg),
        panel_focused_border: color_or_map(colors, "focusBorder", border),
        panel_indent_guide: color_or_map(colors, "tree.indentGuidesStroke", border),
        panel_indent_guide_hover: border,
        panel_indent_guide_active: border,
        panel_overlay_background: bg,
        panel_overlay_hover: bg,
        pane_focused_border: color_or_map(colors, "focusBorder", border),
        pane_group_border: color_or_map(colors, "editorGroup.border", border),

        // Scrollbar
        scrollbar_thumb_background: color_or_map(colors, "scrollbarSlider.background", muted),
        scrollbar_thumb_hover_background: color_or_map(colors, "scrollbarSlider.hoverBackground", muted),
        scrollbar_thumb_active_background: color_or_map(colors, "scrollbarSlider.activeBackground", muted),
        scrollbar_thumb_border: transparent,
        scrollbar_track_background: transparent,
        scrollbar_track_border: transparent,

        // Editor
        editor_foreground: fg,
        editor_background: bg,
        editor_gutter_background: bg,
        editor_subheader_background: surface,
        editor_active_line_background: color_or_map(colors, "editor.lineHighlightBackground", surface),
        editor_highlighted_line_background: color_or_map(colors, "editor.lineHighlightBackground", surface),
        editor_line_number: color_or_map(colors, "editorLineNumber.foreground", muted),
        editor_active_line_number: color_or_map(colors, "editorLineNumber.activeForeground", fg),
        editor_invisible: color_or_map(colors, "editorWhitespace.foreground", muted),
        editor_wrap_guide: color_or_map(colors, "editorRuler.foreground", border),
        editor_active_wrap_guide: border,
        editor_indent_guide: color_or_map(colors, "editorIndentGuide.background", border),
        editor_indent_guide_active: color_or_map(colors, "editorIndentGuide.activeBackground", border),
        editor_document_highlight_read_background: color_or_map(colors, "editor.wordHighlightBackground", surface),
        editor_document_highlight_write_background: color_or_map(colors, "editor.wordHighlightStrongBackground", surface),
        editor_document_highlight_bracket_background: color_or_map(colors, "editorBracketMatch.background", surface),

        // Terminal ANSI
        terminal_background: color_or_map(colors, "terminal.background", bg),
        terminal_foreground: color_or_map(colors, "terminal.foreground", fg),
        terminal_bright_foreground: fg,
        terminal_dim_foreground: muted,
        terminal_accent: color_or_map(colors, "terminal.selectionBackground",
            color_or_map(colors, "focusBorder", parse_color("#00BFFFff").unwrap_or_default())),
        terminal_ansi_background: bg,
        terminal_ansi_black: color_or_map(colors, "terminal.ansiBlack", default_bg),
        terminal_ansi_bright_black: color_or_map(colors, "terminal.ansiBrightBlack", muted),
        terminal_ansi_dim_black: default_bg,
        terminal_ansi_red: color_or_map(colors, "terminal.ansiRed", parse_color("#cd3131ff").unwrap_or_default()),
        terminal_ansi_bright_red: color_or_map(colors, "terminal.ansiBrightRed", parse_color("#f14c4cff").unwrap_or_default()),
        terminal_ansi_dim_red: dim_color(color_or_map(colors, "terminal.ansiRed", parse_color("#cd3131ff").unwrap_or_default())),
        terminal_ansi_green: color_or_map(colors, "terminal.ansiGreen", parse_color("#0dbc79ff").unwrap_or_default()),
        terminal_ansi_bright_green: color_or_map(colors, "terminal.ansiBrightGreen", parse_color("#23d18bff").unwrap_or_default()),
        terminal_ansi_dim_green: dim_color(color_or_map(colors, "terminal.ansiGreen", parse_color("#0dbc79ff").unwrap_or_default())),
        terminal_ansi_yellow: color_or_map(colors, "terminal.ansiYellow", parse_color("#e5e510ff").unwrap_or_default()),
        terminal_ansi_bright_yellow: color_or_map(colors, "terminal.ansiBrightYellow", parse_color("#f5f543ff").unwrap_or_default()),
        terminal_ansi_dim_yellow: dim_color(color_or_map(colors, "terminal.ansiYellow", parse_color("#e5e510ff").unwrap_or_default())),
        terminal_ansi_blue: color_or_map(colors, "terminal.ansiBlue", parse_color("#2472c8ff").unwrap_or_default()),
        terminal_ansi_bright_blue: color_or_map(colors, "terminal.ansiBrightBlue", parse_color("#3b8eedff").unwrap_or_default()),
        terminal_ansi_dim_blue: dim_color(color_or_map(colors, "terminal.ansiBlue", parse_color("#2472c8ff").unwrap_or_default())),
        terminal_ansi_magenta: color_or_map(colors, "terminal.ansiMagenta", parse_color("#bc3fbcff").unwrap_or_default()),
        terminal_ansi_bright_magenta: color_or_map(colors, "terminal.ansiBrightMagenta", parse_color("#d670d6ff").unwrap_or_default()),
        terminal_ansi_dim_magenta: dim_color(color_or_map(colors, "terminal.ansiMagenta", parse_color("#bc3fbcff").unwrap_or_default())),
        terminal_ansi_cyan: color_or_map(colors, "terminal.ansiCyan", parse_color("#11a8cdff").unwrap_or_default()),
        terminal_ansi_bright_cyan: color_or_map(colors, "terminal.ansiBrightCyan", parse_color("#29b8dbff").unwrap_or_default()),
        terminal_ansi_dim_cyan: dim_color(color_or_map(colors, "terminal.ansiCyan", parse_color("#11a8cdff").unwrap_or_default())),
        terminal_ansi_white: color_or_map(colors, "terminal.ansiWhite", parse_color("#e5e5e5ff").unwrap_or_default()),
        terminal_ansi_bright_white: color_or_map(colors, "terminal.ansiBrightWhite", parse_color("#e5e5e5ff").unwrap_or_default()),
        terminal_ansi_dim_white: dim_color(color_or_map(colors, "terminal.ansiWhite", parse_color("#e5e5e5ff").unwrap_or_default())),

        // Links
        link_text_hover: color_or_map(colors, "textLink.foreground", fg),

        // Version control
        version_control_added: color_or_map(colors, "gitDecoration.addedResourceForeground", parse_color("#81b88bff").unwrap_or_default()),
        version_control_deleted: color_or_map(colors, "gitDecoration.deletedResourceForeground", parse_color("#c74e39ff").unwrap_or_default()),
        version_control_modified: color_or_map(colors, "gitDecoration.modifiedResourceForeground", parse_color("#e2c08dff").unwrap_or_default()),
        version_control_renamed: color_or_map(colors, "gitDecoration.renamedResourceForeground", muted),
        version_control_conflict: color_or_map(colors, "gitDecoration.conflictingResourceForeground", parse_color("#ff79c6ff").unwrap_or_default()),
        version_control_ignored: color_or_map(colors, "gitDecoration.ignoredResourceForeground", muted),
        version_control_word_added: {
            let mut c = color_or_map(colors, "diffEditor.insertedTextBackground", parse_color("#9bb95540").unwrap_or_default());
            c.a = c.a.min(0.4);
            c
        },
        version_control_word_deleted: {
            let mut c = color_or_map(colors, "diffEditor.removedTextBackground", parse_color("#ff000033").unwrap_or_default());
            c.a = c.a.min(0.4);
            c
        },
        version_control_conflict_marker_ours: color_or_map(colors, "merge.currentHeaderBackground", parse_color("#14F19533").unwrap_or_default()),
        version_control_conflict_marker_theirs: color_or_map(colors, "merge.incomingHeaderBackground", parse_color("#8be9fd33").unwrap_or_default()),

        // Minimap
        minimap_thumb_background: color_or_map(colors, "minimapSlider.background", muted),
        minimap_thumb_hover_background: color_or_map(colors, "minimapSlider.hoverBackground", muted),
        minimap_thumb_active_background: color_or_map(colors, "minimapSlider.activeBackground", muted),
        minimap_thumb_border: transparent,

        // Vim modes (no VS Code equivalent, use sensible defaults)
        vim_normal_background: parse_color("#6272a4ff").unwrap_or_default(),
        vim_insert_background: parse_color("#14F195ff").unwrap_or_default(),
        vim_replace_background: parse_color("#ff5555ff").unwrap_or_default(),
        vim_visual_background: parse_color("#ff79c6ff").unwrap_or_default(),
        vim_visual_line_background: parse_color("#ff79c6ff").unwrap_or_default(),
        vim_visual_block_background: parse_color("#ff79c6ff").unwrap_or_default(),
        vim_yank_background: parse_color("#f1fa8cff").unwrap_or_default(),
        vim_helix_normal_background: parse_color("#6272a4ff").unwrap_or_default(),
        vim_helix_select_background: parse_color("#8be9fdff").unwrap_or_default(),
        vim_normal_foreground: fg,
        vim_insert_foreground: default_bg,
        vim_replace_foreground: fg,
        vim_visual_foreground: default_bg,
        vim_visual_line_foreground: default_bg,
        vim_visual_block_foreground: default_bg,
        vim_helix_normal_foreground: fg,
        vim_helix_select_foreground: default_bg,

        // Debugger
        debugger_accent: parse_color("#ff9e64ff").unwrap_or_default(),
        editor_debugger_active_line_background: color_or_map(colors, "editor.stackFrameHighlightBackground", parse_color("#ff9e6426").unwrap_or_default()),
        editor_hover_line_number: fg,

        // Raijin-specific (derived from terminal colors)
        block_success_badge: color_or_map(colors, "terminal.ansiGreen", parse_color("#14F195ff").unwrap_or_default()),
        block_error_badge: color_or_map(colors, "terminal.ansiRed", parse_color("#ff5555ff").unwrap_or_default()),
        block_running_badge: color_or_map(colors, "terminal.ansiYellow", parse_color("#f1fa8cff").unwrap_or_default()),
    }
}

/// Creates a dimmed variant of a color by reducing its lightness.
fn dim_color(color: Oklch) -> Oklch {
    Oklch {
        l: color.l * 0.7,
        c: color.c * 0.7,
        h: color.h,
        a: color.a,
    }
}

// === Syntax Conversion ===

/// Maps VS Code TextMate scopes to raijin syntax token names.
/// Uses a ranked matching approach: each raijin syntax token has a list of VS Code
/// scopes it can match against, and the best match (most specific) wins.
fn convert_vscode_syntax(token_colors: &[VsCodeTokenColor]) -> SyntaxTheme {
    let mut highlights = HashMap::new();

    // Raijin syntax token -> list of VS Code scopes to search for, ordered by priority
    let syntax_mappings: &[(&str, &[&str])] = &[
        ("attribute", &["entity.other.attribute-name"]),
        ("boolean", &["constant.language"]),
        ("comment", &["comment"]),
        ("comment.doc", &["comment.block.documentation", "comment"]),
        ("constant", &["constant", "constant.language", "constant.character"]),
        ("constructor", &["entity.name.tag", "entity.name.function.definition.special.constructor"]),
        ("embedded", &["meta.embedded"]),
        ("emphasis", &["markup.italic"]),
        ("emphasis.strong", &["markup.bold"]),
        ("enum", &["support.type.enum"]),
        ("function", &["entity.name.function", "variable.function", "support.function"]),
        ("hint", &[]),
        ("keyword", &["keyword", "keyword.control", "storage.type", "storage.modifier"]),
        ("label", &["entity.name", "entity.name.import", "entity.name.package"]),
        ("link_text", &["markup.underline.link", "string.other.link"]),
        ("link_uri", &["markup.underline.link", "string.other.link"]),
        ("number", &["constant.numeric"]),
        ("operator", &["keyword.operator"]),
        ("predictive", &[]),
        ("preproc", &["meta.preprocessor", "punctuation.definition.preprocessor"]),
        ("primary", &[]),
        ("property", &["variable.other.property", "support.type.property-name", "variable.other.field", "variable.member"]),
        ("punctuation", &["punctuation", "punctuation.section", "punctuation.separator"]),
        ("punctuation.bracket", &["punctuation.definition.tag.begin", "punctuation.definition.tag.end"]),
        ("punctuation.delimiter", &["punctuation.separator", "punctuation.terminator"]),
        ("punctuation.list_marker", &["markup.list punctuation.definition.list.begin"]),
        ("punctuation.special", &["punctuation.special"]),
        ("string", &["string"]),
        ("string.escape", &["constant.character.escape", "constant.character"]),
        ("string.regex", &["string.regexp"]),
        ("string.special", &["string.special", "constant.other.symbol"]),
        ("string.special.symbol", &["constant.other.symbol"]),
        ("tag", &["entity.name.tag"]),
        ("text.literal", &["string"]),
        ("title", &["entity.name", "entity.name.section"]),
        ("type", &["entity.name.type", "support.type", "support.class", "storage.type"]),
        ("variable", &["variable", "variable.language", "variable.parameter"]),
        ("variable.special", &["variable.language", "variable.annotation"]),
        ("variant", &[]),
    ];

    for (raijin_token, vscode_scopes) in syntax_mappings {
        if vscode_scopes.is_empty() {
            continue;
        }

        if let Some(highlight) = find_best_match(token_colors, vscode_scopes) {
            highlights.insert(raijin_token.to_string(), highlight);
        }
    }

    SyntaxTheme::new(highlights)
}

/// Finds the best matching token color for a set of candidate scopes.
/// Returns the HighlightStyle from the most specifically matching token color.
fn find_best_match(
    token_colors: &[VsCodeTokenColor],
    target_scopes: &[&str],
) -> Option<HighlightStyle> {
    let mut best_match: Option<(usize, &VsCodeTokenColor)> = None;

    for token_color in token_colors {
        if token_color.settings.foreground.is_none() {
            continue;
        }

        let candidate_scopes = match token_color.scope.as_ref() {
            Some(VsCodeTokenScope::One(scope)) => vec![scope.as_str()],
            Some(VsCodeTokenScope::Many(scopes)) => scopes.iter().map(|s| s.as_str()).collect(),
            None => continue,
        };

        let mut score = 0usize;
        for (priority, target) in target_scopes.iter().enumerate() {
            let weight = target_scopes.len() - priority;
            for candidate in &candidate_scopes {
                // Split comma-separated scopes
                for part in candidate.split(',') {
                    let part = part.trim();
                    if part == *target || part.starts_with(&format!("{target}.")) {
                        score += weight;
                    }
                }
            }
        }

        if score > 0 {
            match &best_match {
                Some((best_score, _)) if score <= *best_score => {}
                _ => best_match = Some((score, token_color)),
            }
        }
    }

    let token_color = best_match.map(|(_, tc)| tc)?;

    let color = token_color
        .settings
        .foreground
        .as_deref()
        .and_then(|s| parse_color(s).ok());

    let (font_weight, font_style) = match token_color.settings.font_style.as_deref() {
        Some(s) if s.contains("bold") && s.contains("italic") => {
            (Some(FontWeight::BOLD), Some(FontStyle::Italic))
        }
        Some(s) if s.contains("bold") => (Some(FontWeight::BOLD), None),
        Some(s) if s.contains("italic") => (None, Some(FontStyle::Italic)),
        Some(s) if s.contains("oblique") => (None, Some(FontStyle::Oblique)),
        _ => (None, None),
    };

    Some(HighlightStyle {
        color,
        font_weight,
        font_style,
        background_color: None,
        underline: None,
        strikethrough: None,
        fade_out: None,
    })
}

// === Status Colors ===

fn derive_status_colors(
    colors: &HashMap<String, serde_json::Value>,
    theme_colors: &ThemeColors,
) -> StatusColors {
    let make_status = |base: Oklch| -> StatusStyle {
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

    let error_color = color_or_map(colors, "errorForeground", theme_colors.version_control_deleted);
    let warning_color = color_or_map(colors, "editorWarning.foreground", theme_colors.terminal_ansi_yellow);
    let info_color = color_or_map(colors, "editorInfo.foreground", theme_colors.version_control_modified);

    StatusColors {
        conflict: make_status(theme_colors.version_control_conflict),
        created: make_status(theme_colors.version_control_added),
        deleted: make_status(theme_colors.version_control_deleted),
        error: make_status(error_color),
        hidden: make_status(theme_colors.text_muted),
        hint: make_status(color_or_map(colors, "editorHint.foreground", theme_colors.text_muted)),
        ignored: make_status(theme_colors.version_control_ignored),
        info: make_status(info_color),
        modified: make_status(theme_colors.version_control_modified),
        predictive: make_status(theme_colors.text_muted),
        renamed: make_status(theme_colors.version_control_renamed),
        success: make_status(theme_colors.version_control_added),
        unreachable: make_status(theme_colors.text_muted),
        warning: make_status(warning_color),
    }
}
