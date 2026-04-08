use std::sync::Arc;

use inazuma::{FontStyle, FontWeight, HighlightStyle, WindowBackgroundAppearance, oklch, oklcha};

use crate::{
    AccentColors, Appearance, DEFAULT_DARK_THEME, DEFAULT_LIGHT_THEME, PlayerColor, PlayerColors,
    StatusColors, StatusColorsRefinement, StatusStyle, StatusStyleRefinement, SyntaxTheme,
    SystemColors, Theme, ThemeBackgroundImage, ThemeColors, ThemeColorsRefinement, ThemeFamily,
    ThemeStyles, default_color_scales,
};

/// The default theme family for Raijin.
///
/// Contains Raijin Dark and Raijin Light as compile-time fallback themes.
pub fn raijin_default_themes() -> ThemeFamily {
    ThemeFamily {
        id: "raijin-default".to_string(),
        name: "Raijin Default".into(),
        author: "nyxb".into(),
        themes: vec![raijin_default_dark(), raijin_default_light()],
        scales: default_color_scales(),
    }
}

/// Applies default status color backgrounds from their foreground counterparts.
///
/// If a theme customizes the base color of a status but not the background,
/// a 25% opacity version of the base color is used as the background.
pub fn apply_status_color_defaults(status: &mut StatusColorsRefinement) {
    fn apply(style: &mut StatusStyleRefinement) {
        if style.background.is_none() {
            if let Some(color) = style.color {
                style.background = Some(color.opacity(0.25));
            }
        }
    }
    apply(&mut status.deleted);
    apply(&mut status.created);
    apply(&mut status.modified);
    apply(&mut status.conflict);
    apply(&mut status.error);
    apply(&mut status.hidden);
}

/// Applies default theme color values derived from player colors.
pub fn apply_theme_color_defaults(
    theme_colors: &mut ThemeColorsRefinement,
    player_colors: &PlayerColors,
) {
    if theme_colors.element_selection_background.is_none() {
        let mut selection = player_colors.local().selection;
        if selection.a == 1.0 {
            selection.a = 0.25;
        }
        theme_colors.element_selection_background = Some(selection);
    }
}

// ---------------------------------------------------------------------------
// Raijin Dark — from assets/themes/raijin-dark/theme.toml
// Fields not in our TOML are derived from Zed's One Dark, adapted to our palette.
// ---------------------------------------------------------------------------

pub(crate) fn raijin_default_dark() -> Theme {
    let transparent = oklcha(0.0, 0.0, 0.0, 0.0);

    // === Core palette (from raijin-dark/theme.toml) ===
    let bg = oklch(0.1822, 0.0, 0.0);
    let surface = oklch(0.2178, 0.0, 0.0);
    let elevated = oklch(0.252, 0.0, 0.0);
    let element_bg = oklch(0.235, 0.0, 0.0);
    let hover = oklch(0.285, 0.0, 0.0);
    let active = oklch(0.3211, 0.0, 0.0);
    let subtle_border = oklch(0.252, 0.0, 0.0);
    let tab_bg = oklch(0.1638, 0.0, 0.0);

    // Accent: Cyan #00BFFF
    let accent = oklch(0.7554, 0.1534, 231.64);

    // Semantic colors (from our TOML status section)
    let red = oklch(0.7227, 0.1589, 10.28);
    let green = oklch(0.8441, 0.1991, 156.83);
    let yellow = oklch(0.7839, 0.1057, 75.43);
    let cyan = oklch(0.82, 0.1051, 235.72);
    let blue = oklch(0.719, 0.1322, 264.2);
    let magenta = oklch(0.7515, 0.1344, 299.5);
    let orange = oklch(0.787, 0.1373, 50.56);

    // Text
    let fg = oklch(0.9581, 0.0, 0.0);
    let text_muted = oklch(0.6268, 0.0, 0.0);
    let text_placeholder = oklch(0.4495, 0.0, 0.0);

    // Derived grays (for fields not in our TOML, adapted from Zed)
    let comment_gray = oklch(0.4955, 0.0682, 274.37);

    let player = PlayerColors(vec![
        PlayerColor {
            cursor: accent,
            background: accent,
            selection: oklcha(0.7554, 0.1534, 231.64, 0.239),
        },
        PlayerColor {
            cursor: blue,
            background: blue,
            selection: oklcha(0.719, 0.1322, 264.2, 0.239),
        },
        PlayerColor {
            cursor: magenta,
            background: magenta,
            selection: oklcha(0.7515, 0.1344, 299.5, 0.239),
        },
        PlayerColor {
            cursor: red,
            background: red,
            selection: oklcha(0.7227, 0.1589, 10.28, 0.239),
        },
    ]);

    Theme {
        id: "raijin_dark".to_string(),
        name: DEFAULT_DARK_THEME.into(),
        appearance: Appearance::Dark,
        base_dir: None,
        styles: ThemeStyles {
            window_background_appearance: WindowBackgroundAppearance::Opaque,
            system: SystemColors::default(),
            accents: AccentColors(Arc::from(vec![
                accent, blue, magenta, red, green, yellow, orange,
            ])),
            colors: ThemeColors {
                // --- From raijin-dark/theme.toml ---
                background: bg,
                surface_background: surface,
                elevated_surface_background: elevated,
                element_background: element_bg,
                element_hover: hover,
                element_active: active,
                element_selected: active,
                element_disabled: element_bg,
                element_selection_background: player.local().selection.alpha(0.25),
                drop_target_background: oklcha(0.7554, 0.1534, 231.64, 0.251),
                drop_target_border: fg,
                ghost_element_background: transparent,
                ghost_element_hover: hover,
                ghost_element_active: active,
                ghost_element_selected: active,
                ghost_element_disabled: transparent,

                border: oklch(0.285, 0.0, 0.0),
                border_variant: subtle_border,
                border_focused: accent,
                border_selected: oklcha(0.7554, 0.1534, 231.64, 0.4),
                border_transparent: transparent,
                border_disabled: element_bg,

                text: fg,
                text_muted,
                text_placeholder,
                text_disabled: text_placeholder,
                text_accent: accent,

                icon: fg,
                icon_muted: text_muted,
                icon_disabled: text_placeholder,
                icon_placeholder: text_placeholder,
                icon_accent: accent,

                debugger_accent: red,

                status_bar_background: bg,
                title_bar_background: bg,
                title_bar_inactive_background: tab_bg,
                toolbar_background: surface,
                tab_bar_background: tab_bg,
                tab_inactive_background: tab_bg,
                tab_active_background: surface,

                search_match_background: oklcha(0.7554, 0.1534, 231.64, 0.251),
                search_active_match_background: oklcha(0.7554, 0.1534, 231.64, 0.4),

                editor_foreground: fg,
                editor_background: bg,
                editor_gutter_background: bg,
                editor_active_line_background: oklcha(0.2178, 0.0, 0.0, 0.749),
                editor_line_number: oklch(0.459, 0.0171, 224.8),
                editor_active_line_number: fg,
                editor_invisible: active,
                editor_wrap_guide: oklcha(0.285, 0.0, 0.0, 0.051),

                // --- Terminal from raijin-dark/theme.toml ---
                terminal_background: bg,
                terminal_accent: accent,
                terminal_foreground: fg,
                terminal_bright_foreground: oklch(1.0, 0.0, 0.0),
                terminal_dim_foreground: text_muted,
                terminal_ansi_background: bg,
                terminal_ansi_black: oklch(0.2768, 0.0, 0.0),
                terminal_ansi_red: red,
                terminal_ansi_green: green,
                terminal_ansi_yellow: yellow,
                terminal_ansi_blue: blue,
                terminal_ansi_magenta: magenta,
                terminal_ansi_cyan: cyan,
                terminal_ansi_white: oklch(0.8456, 0.0611, 274.76),
                terminal_ansi_bright_black: text_placeholder,
                terminal_ansi_bright_red: oklch(0.7978, 0.1161, 19.96),
                terminal_ansi_bright_green: oklch(0.8925, 0.1733, 165.58),
                terminal_ansi_bright_yellow: oklch(0.881, 0.1039, 76.53),
                terminal_ansi_bright_blue: oklch(0.7892, 0.1059, 267.95),
                terminal_ansi_bright_magenta: oklch(0.8237, 0.1083, 303.65),
                terminal_ansi_bright_cyan: oklch(0.8872, 0.0713, 227.53),
                terminal_ansi_bright_white: oklch(1.0, 0.0, 0.0),
                terminal_ansi_dim_black: surface,
                terminal_ansi_dim_red: oklch(0.5406, 0.1129, 9.76),
                terminal_ansi_dim_green: oklch(0.6375, 0.1445, 158.8),
                terminal_ansi_dim_yellow: oklch(0.6247, 0.0792, 72.75),
                terminal_ansi_dim_blue: oklch(0.5584, 0.0838, 258.29),
                terminal_ansi_dim_magenta: oklch(0.5854, 0.0827, 302.1),
                terminal_ansi_dim_cyan: oklch(0.6212, 0.0692, 222.95),
                terminal_ansi_dim_white: text_muted,

                // --- Version Control (from TOML + Zed derived) ---
                version_control_added: green,
                version_control_modified: yellow,
                version_control_deleted: red,
                version_control_renamed: yellow,
                version_control_conflict: orange,
                version_control_ignored: text_placeholder,
                version_control_word_added: green.alpha(0.35),
                version_control_word_deleted: red.alpha(0.8),
                version_control_conflict_marker_ours: green.alpha(0.5),
                version_control_conflict_marker_theirs: blue.alpha(0.5),

                link_text_hover: accent,

                // --- Block badges (from TOML) ---
                block_success_badge: green,
                block_error_badge: red,
                block_running_badge: yellow,

                // --- Fields from Zed One Dark, adapted to our palette ---
                panel_background: tab_bg,
                panel_focused_border: accent,
                panel_indent_guide: subtle_border,
                panel_indent_guide_hover: oklch(0.285, 0.0, 0.0),
                panel_indent_guide_active: oklch(0.285, 0.0, 0.0),
                panel_overlay_background: bg,
                panel_overlay_hover: hover,
                pane_focused_border: accent,
                pane_group_border: oklch(0.285, 0.0, 0.0),

                scrollbar_thumb_background: oklcha(1.0, 0.0, 0.0, 0.102),
                scrollbar_thumb_hover_background: oklcha(1.0, 0.0, 0.0, 0.2),
                scrollbar_thumb_active_background: oklcha(1.0, 0.0, 0.0, 0.3),
                scrollbar_thumb_border: transparent,
                scrollbar_track_background: transparent,
                scrollbar_track_border: transparent,

                minimap_thumb_background: oklcha(1.0, 0.0, 0.0, 0.1),
                minimap_thumb_hover_background: oklcha(1.0, 0.0, 0.0, 0.15),
                minimap_thumb_active_background: oklcha(1.0, 0.0, 0.0, 0.2),
                minimap_thumb_border: transparent,

                editor_subheader_background: surface,
                editor_highlighted_line_background: accent.alpha(0.1),
                editor_debugger_active_line_background: accent.alpha(0.2),
                editor_hover_line_number: fg,
                editor_active_wrap_guide: oklcha(0.285, 0.0, 0.0, 0.1),
                editor_indent_guide: subtle_border,
                editor_indent_guide_active: oklch(0.285, 0.0, 0.0),
                editor_document_highlight_read_background: accent.alpha(0.15),
                editor_document_highlight_write_background: accent.alpha(0.25),
                editor_document_highlight_bracket_background: green.alpha(0.15),

                vim_normal_background: transparent,
                vim_insert_background: transparent,
                vim_replace_background: transparent,
                vim_visual_background: transparent,
                vim_visual_line_background: transparent,
                vim_visual_block_background: transparent,
                vim_yank_background: accent.alpha(0.2),
                vim_helix_normal_background: transparent,
                vim_helix_select_background: transparent,
                vim_normal_foreground: transparent,
                vim_insert_foreground: transparent,
                vim_replace_foreground: transparent,
                vim_visual_foreground: transparent,
                vim_visual_line_foreground: transparent,
                vim_visual_block_foreground: transparent,
                vim_helix_normal_foreground: transparent,
                vim_helix_select_foreground: transparent,
            },
            status: StatusColors {
                error: StatusStyle {
                    color: red,
                    background: red.alpha(0.102),
                    border: red.alpha(0.2),
                },
                warning: StatusStyle {
                    color: yellow,
                    background: yellow.alpha(0.102),
                    border: yellow.alpha(0.2),
                },
                success: StatusStyle {
                    color: green,
                    background: green.alpha(0.102),
                    border: green.alpha(0.2),
                },
                info: StatusStyle {
                    color: cyan,
                    background: cyan.alpha(0.102),
                    border: cyan.alpha(0.2),
                },
                hint: StatusStyle {
                    color: blue,
                    background: blue.alpha(0.102),
                    border: blue.alpha(0.2),
                },
                conflict: StatusStyle {
                    color: orange,
                    background: orange.alpha(0.102),
                    border: orange.alpha(0.2),
                },
                created: StatusStyle {
                    color: green,
                    background: green.alpha(0.102),
                    border: green.alpha(0.2),
                },
                deleted: StatusStyle {
                    color: red,
                    background: red.alpha(0.102),
                    border: red.alpha(0.2),
                },
                hidden: StatusStyle {
                    color: text_placeholder,
                    background: text_placeholder.alpha(0.102),
                    border: text_placeholder.alpha(0.2),
                },
                ignored: StatusStyle {
                    color: text_placeholder,
                    background: text_placeholder.alpha(0.102),
                    border: text_placeholder.alpha(0.2),
                },
                modified: StatusStyle {
                    color: yellow,
                    background: yellow.alpha(0.102),
                    border: yellow.alpha(0.2),
                },
                predictive: StatusStyle {
                    color: comment_gray,
                    background: comment_gray.alpha(0.102),
                    border: comment_gray.alpha(0.2),
                },
                renamed: StatusStyle {
                    color: blue,
                    background: blue.alpha(0.102),
                    border: blue.alpha(0.2),
                },
                unreachable: StatusStyle {
                    color: text_placeholder,
                    background: text_placeholder.alpha(0.102),
                    border: text_placeholder.alpha(0.2),
                },
            },
            players: player,
            syntax: Arc::new(SyntaxTheme::new(vec![
                ("attribute".into(), blue.into()),
                ("boolean".into(), orange.into()),
                (
                    "comment".into(),
                    HighlightStyle {
                        color: Some(comment_gray),
                        font_style: Some(FontStyle::Italic),
                        ..HighlightStyle::default()
                    },
                ),
                (
                    "comment.doc".into(),
                    HighlightStyle {
                        color: Some(oklch(0.5605, 0.0524, 272.75)),
                        font_style: Some(FontStyle::Italic),
                        ..HighlightStyle::default()
                    },
                ),
                ("constant".into(), orange.into()),
                ("constructor".into(), blue.into()),
                ("embedded".into(), HighlightStyle::default()),
                (
                    "emphasis".into(),
                    HighlightStyle {
                        font_style: Some(FontStyle::Italic),
                        ..HighlightStyle::default()
                    },
                ),
                (
                    "emphasis.strong".into(),
                    HighlightStyle {
                        font_weight: Some(FontWeight::BOLD),
                        ..HighlightStyle::default()
                    },
                ),
                ("enum".into(), oklch(0.7537, 0.1243, 213.18).into()),
                ("function".into(), blue.into()),
                ("function.method".into(), blue.into()),
                ("function.definition".into(), blue.into()),
                ("hint".into(), blue.into()),
                ("keyword".into(), magenta.into()),
                ("label".into(), HighlightStyle::default()),
                ("link_text".into(), accent.into()),
                (
                    "link_uri".into(),
                    HighlightStyle {
                        color: Some(accent),
                        font_style: Some(FontStyle::Italic),
                        ..HighlightStyle::default()
                    },
                ),
                ("number".into(), orange.into()),
                ("operator".into(), oklch(0.8561, 0.0943, 225.87).into()),
                ("predictive".into(), HighlightStyle::default()),
                ("preproc".into(), HighlightStyle::default()),
                ("primary".into(), HighlightStyle::default()),
                ("property".into(), oklch(0.8217, 0.0996, 182.49).into()),
                ("punctuation".into(), oklch(0.8456, 0.0611, 274.76).into()),
                ("punctuation.bracket".into(), oklch(0.7276, 0.0609, 273.09).into()),
                ("punctuation.delimiter".into(), HighlightStyle::default()),
                ("punctuation.list_marker".into(), HighlightStyle::default()),
                ("punctuation.special".into(), HighlightStyle::default()),
                ("string".into(), green.into()),
                ("string.escape".into(), oklch(0.8561, 0.0943, 225.87).into()),
                ("string.regex".into(), oklch(0.9363, 0.0684, 194.91).into()),
                ("string.special".into(), HighlightStyle::default()),
                ("string.special.symbol".into(), HighlightStyle::default()),
                ("tag".into(), red.into()),
                ("text.literal".into(), HighlightStyle::default()),
                (
                    "title".into(),
                    HighlightStyle {
                        color: Some(red),
                        font_weight: Some(FontWeight(700.0)),
                        ..HighlightStyle::default()
                    },
                ),
                ("type".into(), oklch(0.7537, 0.1243, 213.18).into()),
                ("variable".into(), oklch(0.8456, 0.0611, 274.76).into()),
                ("variable.special".into(), orange.into()),
                ("variant".into(), HighlightStyle::default()),
            ])),
            background_image: Some(ThemeBackgroundImage {
                path: "bg.png".to_string(),
                opacity: 30,
            }),
        },
    }
}

// ---------------------------------------------------------------------------
// Raijin Light — inverted version of Raijin Dark
// Uses light-adapted values, fills from Zed's One Light where needed.
// ---------------------------------------------------------------------------

pub(crate) fn raijin_default_light() -> Theme {
    let transparent = oklcha(0.0, 0.0, 0.0, 0.0);

    // === Light palette ===
    let bg = oklch(0.985, 0.0, 0.0);
    let surface = oklch(0.965, 0.0, 0.0);
    let elevated = oklch(0.975, 0.0, 0.0);
    let element_bg = oklch(0.955, 0.0, 0.0);
    let hover = oklch(0.935, 0.0, 0.0);
    let active = oklch(0.92, 0.0, 0.0);
    let subtle_border = oklch(0.88, 0.0, 0.0);
    let tab_bg = oklch(0.955, 0.0, 0.0);

    // Same accent
    let accent = oklch(0.7554, 0.1534, 231.64);

    // Semantic colors (slightly deeper for light backgrounds)
    let red = oklch(0.58, 0.22, 18.0);
    let green = oklch(0.55, 0.17, 150.0);
    let yellow = oklch(0.65, 0.15, 75.0);
    let cyan = oklch(0.60, 0.13, 230.0);
    let blue = oklch(0.55, 0.17, 260.0);
    let magenta = oklch(0.58, 0.18, 300.0);
    let orange = oklch(0.62, 0.16, 50.0);

    let fg = oklch(0.2, 0.0, 0.0);
    let text_muted = oklch(0.45, 0.0, 0.0);
    let text_placeholder = oklch(0.6, 0.0, 0.0);
    let comment_gray = oklch(0.55, 0.04, 260.0);

    let player = PlayerColors(vec![
        PlayerColor {
            cursor: accent,
            background: accent,
            selection: accent.alpha(0.15),
        },
        PlayerColor {
            cursor: blue,
            background: blue,
            selection: blue.alpha(0.15),
        },
        PlayerColor {
            cursor: magenta,
            background: magenta,
            selection: magenta.alpha(0.15),
        },
        PlayerColor {
            cursor: red,
            background: red,
            selection: red.alpha(0.15),
        },
    ]);

    Theme {
        id: "raijin_light".to_string(),
        name: DEFAULT_LIGHT_THEME.into(),
        appearance: Appearance::Light,
        base_dir: None,
        styles: ThemeStyles {
            window_background_appearance: WindowBackgroundAppearance::Opaque,
            system: SystemColors::default(),
            accents: AccentColors(Arc::from(vec![
                accent, blue, magenta, red, green, yellow, orange,
            ])),
            colors: ThemeColors {
                background: bg,
                surface_background: surface,
                elevated_surface_background: elevated,
                element_background: element_bg,
                element_hover: hover,
                element_active: active,
                element_selected: active,
                element_disabled: element_bg,
                element_selection_background: player.local().selection.alpha(0.25),
                drop_target_background: accent.alpha(0.1),
                drop_target_border: fg,
                ghost_element_background: transparent,
                ghost_element_hover: hover,
                ghost_element_active: active,
                ghost_element_selected: active,
                ghost_element_disabled: transparent,

                border: oklch(0.85, 0.0, 0.0),
                border_variant: oklch(0.88, 0.0, 0.0),
                border_focused: accent,
                border_selected: accent.alpha(0.4),
                border_transparent: transparent,
                border_disabled: oklch(0.9, 0.0, 0.0),

                text: fg,
                text_muted,
                text_placeholder,
                text_disabled: text_placeholder,
                text_accent: accent,

                icon: fg,
                icon_muted: text_muted,
                icon_disabled: text_placeholder,
                icon_placeholder: text_placeholder,
                icon_accent: accent,

                debugger_accent: red,

                status_bar_background: bg,
                title_bar_background: bg,
                title_bar_inactive_background: surface,
                toolbar_background: bg,
                tab_bar_background: tab_bg,
                tab_inactive_background: tab_bg,
                tab_active_background: bg,

                search_match_background: accent.alpha(0.15),
                search_active_match_background: accent.alpha(0.3),

                editor_foreground: fg,
                editor_background: bg,
                editor_gutter_background: bg,
                editor_active_line_background: oklch(0.96, 0.0, 0.0),
                editor_line_number: text_placeholder,
                editor_active_line_number: fg,
                editor_invisible: oklch(0.85, 0.0, 0.0),
                editor_wrap_guide: oklch(0.92, 0.0, 0.0),

                terminal_background: bg,
                terminal_accent: accent,
                terminal_foreground: fg,
                terminal_bright_foreground: oklch(0.15, 0.0, 0.0),
                terminal_dim_foreground: text_muted,
                terminal_ansi_background: bg,
                terminal_ansi_black: oklch(0.25, 0.0, 0.0),
                terminal_ansi_red: red,
                terminal_ansi_green: green,
                terminal_ansi_yellow: yellow,
                terminal_ansi_blue: blue,
                terminal_ansi_magenta: magenta,
                terminal_ansi_cyan: cyan,
                terminal_ansi_white: oklch(0.9, 0.0, 0.0),
                terminal_ansi_bright_black: oklch(0.45, 0.0, 0.0),
                terminal_ansi_bright_red: red.lighten(0.08),
                terminal_ansi_bright_green: green.lighten(0.08),
                terminal_ansi_bright_yellow: yellow.lighten(0.08),
                terminal_ansi_bright_blue: blue.lighten(0.08),
                terminal_ansi_bright_magenta: magenta.lighten(0.08),
                terminal_ansi_bright_cyan: cyan.lighten(0.08),
                terminal_ansi_bright_white: oklch(0.98, 0.0, 0.0),
                terminal_ansi_dim_black: oklch(0.35, 0.0, 0.0),
                terminal_ansi_dim_red: red.darken(0.1),
                terminal_ansi_dim_green: green.darken(0.1),
                terminal_ansi_dim_yellow: yellow.darken(0.1),
                terminal_ansi_dim_blue: blue.darken(0.1),
                terminal_ansi_dim_magenta: magenta.darken(0.1),
                terminal_ansi_dim_cyan: cyan.darken(0.1),
                terminal_ansi_dim_white: oklch(0.7, 0.0, 0.0),

                version_control_added: green,
                version_control_modified: yellow,
                version_control_deleted: red,
                version_control_renamed: yellow,
                version_control_conflict: orange,
                version_control_ignored: text_placeholder,
                version_control_word_added: green.alpha(0.25),
                version_control_word_deleted: red.alpha(0.6),
                version_control_conflict_marker_ours: green.alpha(0.3),
                version_control_conflict_marker_theirs: blue.alpha(0.3),

                link_text_hover: accent,

                block_success_badge: green,
                block_error_badge: red,
                block_running_badge: yellow,

                panel_background: surface,
                panel_focused_border: accent,
                panel_indent_guide: subtle_border,
                panel_indent_guide_hover: oklch(0.82, 0.0, 0.0),
                panel_indent_guide_active: oklch(0.82, 0.0, 0.0),
                panel_overlay_background: bg,
                panel_overlay_hover: hover,
                pane_focused_border: accent,
                pane_group_border: oklch(0.88, 0.0, 0.0),

                scrollbar_thumb_background: oklcha(0.0, 0.0, 0.0, 0.08),
                scrollbar_thumb_hover_background: oklcha(0.0, 0.0, 0.0, 0.15),
                scrollbar_thumb_active_background: oklcha(0.0, 0.0, 0.0, 0.2),
                scrollbar_thumb_border: transparent,
                scrollbar_track_background: transparent,
                scrollbar_track_border: transparent,

                minimap_thumb_background: oklcha(0.0, 0.0, 0.0, 0.06),
                minimap_thumb_hover_background: oklcha(0.0, 0.0, 0.0, 0.1),
                minimap_thumb_active_background: oklcha(0.0, 0.0, 0.0, 0.15),
                minimap_thumb_border: transparent,

                editor_subheader_background: surface,
                editor_highlighted_line_background: accent.alpha(0.08),
                editor_debugger_active_line_background: accent.alpha(0.15),
                editor_hover_line_number: fg,
                editor_active_wrap_guide: oklch(0.9, 0.0, 0.0),
                editor_indent_guide: subtle_border,
                editor_indent_guide_active: oklch(0.82, 0.0, 0.0),
                editor_document_highlight_read_background: accent.alpha(0.1),
                editor_document_highlight_write_background: accent.alpha(0.2),
                editor_document_highlight_bracket_background: green.alpha(0.12),

                vim_normal_background: transparent,
                vim_insert_background: transparent,
                vim_replace_background: transparent,
                vim_visual_background: transparent,
                vim_visual_line_background: transparent,
                vim_visual_block_background: transparent,
                vim_yank_background: accent.alpha(0.15),
                vim_helix_normal_background: transparent,
                vim_helix_select_background: transparent,
                vim_normal_foreground: transparent,
                vim_insert_foreground: transparent,
                vim_replace_foreground: transparent,
                vim_visual_foreground: transparent,
                vim_visual_line_foreground: transparent,
                vim_visual_block_foreground: transparent,
                vim_helix_normal_foreground: transparent,
                vim_helix_select_foreground: transparent,
            },
            status: StatusColors {
                error: StatusStyle {
                    color: red,
                    background: red.alpha(0.08),
                    border: red.alpha(0.2),
                },
                warning: StatusStyle {
                    color: yellow,
                    background: yellow.alpha(0.08),
                    border: yellow.alpha(0.2),
                },
                success: StatusStyle {
                    color: green,
                    background: green.alpha(0.08),
                    border: green.alpha(0.2),
                },
                info: StatusStyle {
                    color: cyan,
                    background: cyan.alpha(0.08),
                    border: cyan.alpha(0.2),
                },
                hint: StatusStyle {
                    color: blue,
                    background: blue.alpha(0.08),
                    border: blue.alpha(0.2),
                },
                conflict: StatusStyle {
                    color: orange,
                    background: orange.alpha(0.08),
                    border: orange.alpha(0.2),
                },
                created: StatusStyle {
                    color: green,
                    background: green.alpha(0.08),
                    border: green.alpha(0.2),
                },
                deleted: StatusStyle {
                    color: red,
                    background: red.alpha(0.08),
                    border: red.alpha(0.2),
                },
                hidden: StatusStyle {
                    color: text_placeholder,
                    background: text_placeholder.alpha(0.08),
                    border: text_placeholder.alpha(0.2),
                },
                ignored: StatusStyle {
                    color: text_placeholder,
                    background: text_placeholder.alpha(0.08),
                    border: text_placeholder.alpha(0.2),
                },
                modified: StatusStyle {
                    color: yellow,
                    background: yellow.alpha(0.08),
                    border: yellow.alpha(0.2),
                },
                predictive: StatusStyle {
                    color: comment_gray,
                    background: comment_gray.alpha(0.08),
                    border: comment_gray.alpha(0.2),
                },
                renamed: StatusStyle {
                    color: blue,
                    background: blue.alpha(0.08),
                    border: blue.alpha(0.2),
                },
                unreachable: StatusStyle {
                    color: text_placeholder,
                    background: text_placeholder.alpha(0.08),
                    border: text_placeholder.alpha(0.2),
                },
            },
            players: player,
            syntax: Arc::new(SyntaxTheme::new(vec![
                ("attribute".into(), blue.into()),
                ("boolean".into(), orange.into()),
                (
                    "comment".into(),
                    HighlightStyle {
                        color: Some(comment_gray),
                        font_style: Some(FontStyle::Italic),
                        ..HighlightStyle::default()
                    },
                ),
                (
                    "comment.doc".into(),
                    HighlightStyle {
                        color: Some(comment_gray.lighten(0.05)),
                        font_style: Some(FontStyle::Italic),
                        ..HighlightStyle::default()
                    },
                ),
                ("constant".into(), orange.into()),
                ("constructor".into(), blue.into()),
                ("embedded".into(), HighlightStyle::default()),
                (
                    "emphasis".into(),
                    HighlightStyle {
                        font_style: Some(FontStyle::Italic),
                        ..HighlightStyle::default()
                    },
                ),
                (
                    "emphasis.strong".into(),
                    HighlightStyle {
                        font_weight: Some(FontWeight::BOLD),
                        ..HighlightStyle::default()
                    },
                ),
                ("enum".into(), cyan.into()),
                ("function".into(), blue.into()),
                ("function.method".into(), blue.into()),
                ("function.definition".into(), blue.into()),
                ("hint".into(), blue.into()),
                ("keyword".into(), magenta.into()),
                ("label".into(), HighlightStyle::default()),
                ("link_text".into(), accent.into()),
                (
                    "link_uri".into(),
                    HighlightStyle {
                        color: Some(accent),
                        font_style: Some(FontStyle::Italic),
                        ..HighlightStyle::default()
                    },
                ),
                ("number".into(), orange.into()),
                ("operator".into(), HighlightStyle::default()),
                ("predictive".into(), HighlightStyle::default()),
                ("preproc".into(), HighlightStyle::default()),
                ("primary".into(), HighlightStyle::default()),
                ("property".into(), red.into()),
                ("punctuation".into(), HighlightStyle::default()),
                ("punctuation.bracket".into(), HighlightStyle::default()),
                ("punctuation.delimiter".into(), HighlightStyle::default()),
                ("punctuation.list_marker".into(), HighlightStyle::default()),
                ("punctuation.special".into(), HighlightStyle::default()),
                ("string".into(), green.into()),
                ("string.escape".into(), HighlightStyle::default()),
                ("string.regex".into(), red.into()),
                ("string.special".into(), HighlightStyle::default()),
                ("string.special.symbol".into(), HighlightStyle::default()),
                ("tag".into(), red.into()),
                ("text.literal".into(), HighlightStyle::default()),
                (
                    "title".into(),
                    HighlightStyle {
                        color: Some(red),
                        font_weight: Some(FontWeight(700.0)),
                        ..HighlightStyle::default()
                    },
                ),
                ("type".into(), cyan.into()),
                ("variable".into(), HighlightStyle::default()),
                ("variable.special".into(), orange.into()),
                ("variant".into(), HighlightStyle::default()),
            ])),
            background_image: Some(ThemeBackgroundImage {
                path: "bg.png".to_string(),
                opacity: 30,
            }),
        },
    }
}
