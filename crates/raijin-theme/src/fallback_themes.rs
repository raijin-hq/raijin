use std::sync::Arc;

use inazuma::{FontStyle, FontWeight, HighlightStyle, WindowBackgroundAppearance, oklch, oklcha, px};

use crate::{
    AccentColors, Appearance, BlockColors, ChartColors, DEFAULT_DARK_THEME, DEFAULT_LIGHT_THEME,
    EditorColors, MinimapColors, PaneColors, PanelColors, PlayerColor, PlayerColors,
    ScrollbarColors, SearchColors, StatusBarColors, StatusColors, StatusColorsRefinement,
    StatusStyle, StatusStyleRefinement, SyntaxTheme, SystemColors, TabColors, TerminalAnsiColors,
    TerminalColors, Theme, ThemeBackgroundImage, ThemeColors, ThemeColorsRefinement, ThemeFamily,
    ThemeStyles, TitleBarColors, ToolbarColors, VersionControlColors, VimColors,
    default_color_scales,
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
    if theme_colors.element_selection.is_none() {
        let mut selection = player_colors.local().selection;
        if selection.a == 1.0 {
            selection.a = 0.25;
        }
        theme_colors.element_selection = Some(selection);
    }
}

// ---------------------------------------------------------------------------
// Raijin Dark — from assets/themes/raijin-dark/theme.toml
// Fields not in our TOML are derived from the original One Dark, adapted to our palette.
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

    // Derived grays (for fields not in our TOML, adapted from reference)
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
                // --- Semantic tokens (shadcn/W3C) ---
                primary: accent,
                primary_foreground: bg,
                secondary: element_bg,
                secondary_foreground: fg,
                muted: surface,
                muted_foreground: text_muted,
                accent: hover,
                accent_foreground: fg,
                destructive: red,
                destructive_foreground: fg,
                background: bg,
                foreground: fg,
                card: surface,
                card_foreground: fg,
                popover: elevated,
                popover_foreground: fg,
                border: oklch(0.285, 0.0, 0.0),
                input: element_bg,
                ring: accent,

                // --- Extended base tokens ---
                surface,
                elevated_surface: elevated,
                element_background: element_bg,
                element_hover: hover,
                element_active: active,
                element_selected: active,
                element_disabled: element_bg,
                element_selection: player.local().selection.alpha(0.25),
                drop_target_background: oklcha(0.7554, 0.1534, 231.64, 0.251),
                drop_target_border: fg,
                ghost_element_background: transparent,
                ghost_element_hover: hover,
                ghost_element_active: active,
                ghost_element_selected: active,
                ghost_element_disabled: transparent,

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

                link_text_hover: accent,
                debugger_accent: red,

                // --- Contextual sub-structs ---
                editor: EditorColors {
                    foreground: fg,
                    background: bg,
                    gutter_background: bg,
                    subheader_background: surface,
                    active_line_background: oklcha(0.2178, 0.0, 0.0, 0.749),
                    highlighted_line_background: accent.alpha(0.1),
                    debugger_active_line_background: accent.alpha(0.2),
                    line_number: oklch(0.459, 0.0171, 224.8),
                    active_line_number: fg,
                    hover_line_number: fg,
                    invisible: active,
                    wrap_guide: oklcha(0.285, 0.0, 0.0, 0.051),
                    active_wrap_guide: oklcha(0.285, 0.0, 0.0, 0.1),
                    indent_guide: subtle_border,
                    indent_guide_active: oklch(0.285, 0.0, 0.0),
                    document_highlight_read_background: accent.alpha(0.15),
                    document_highlight_write_background: accent.alpha(0.25),
                    document_highlight_bracket_background: green.alpha(0.15),
                },
                terminal: TerminalColors {
                    background: bg,
                    foreground: fg,
                    bright_foreground: oklch(1.0, 0.0, 0.0),
                    dim_foreground: text_muted,
                    accent: accent,
                    ansi: TerminalAnsiColors {
                        background: bg,
                        black: oklch(0.2768, 0.0, 0.0),
                        red,
                        green,
                        yellow,
                        blue,
                        magenta,
                        cyan,
                        white: oklch(0.8456, 0.0611, 274.76),
                        bright_black: text_placeholder,
                        bright_red: oklch(0.7978, 0.1161, 19.96),
                        bright_green: oklch(0.8925, 0.1733, 165.58),
                        bright_yellow: oklch(0.881, 0.1039, 76.53),
                        bright_blue: oklch(0.7892, 0.1059, 267.95),
                        bright_magenta: oklch(0.8237, 0.1083, 303.65),
                        bright_cyan: oklch(0.8872, 0.0713, 227.53),
                        bright_white: oklch(1.0, 0.0, 0.0),
                        dim_black: surface,
                        dim_red: oklch(0.5406, 0.1129, 9.76),
                        dim_green: oklch(0.6375, 0.1445, 158.8),
                        dim_yellow: oklch(0.6247, 0.0792, 72.75),
                        dim_blue: oklch(0.5584, 0.0838, 258.29),
                        dim_magenta: oklch(0.5854, 0.0827, 302.1),
                        dim_cyan: oklch(0.6212, 0.0692, 222.95),
                        dim_white: text_muted,
                    },
                },
                panel: PanelColors {
                    background: tab_bg,
                    focused_border: accent,
                    indent_guide: subtle_border,
                    indent_guide_hover: oklch(0.285, 0.0, 0.0),
                    indent_guide_active: oklch(0.285, 0.0, 0.0),
                    overlay_background: bg,
                    overlay_hover: hover,
                },
                pane: PaneColors {
                    focused_border: accent,
                    group_border: oklch(0.285, 0.0, 0.0),
                },
                tab: TabColors {
                    bar_background: tab_bg,
                    inactive_background: tab_bg,
                    active_background: surface,
                    inactive_foreground: text_muted,
                    active_foreground: fg,
                },
                scrollbar: ScrollbarColors {
                    thumb_background: oklcha(1.0, 0.0, 0.0, 0.102),
                    thumb_hover_background: oklcha(1.0, 0.0, 0.0, 0.2),
                    thumb_active_background: oklcha(1.0, 0.0, 0.0, 0.3),
                    thumb_border: transparent,
                    track_background: transparent,
                    track_border: transparent,
                },
                minimap: MinimapColors {
                    thumb_background: oklcha(1.0, 0.0, 0.0, 0.1),
                    thumb_hover_background: oklcha(1.0, 0.0, 0.0, 0.15),
                    thumb_active_background: oklcha(1.0, 0.0, 0.0, 0.2),
                    thumb_border: transparent,
                },
                status_bar: StatusBarColors {
                    background: bg,
                },
                title_bar: TitleBarColors {
                    background: bg,
                    inactive_background: tab_bg,
                },
                toolbar: ToolbarColors {
                    background: surface,
                },
                search: SearchColors {
                    match_background: oklcha(0.7554, 0.1534, 231.64, 0.251),
                    active_match_background: oklcha(0.7554, 0.1534, 231.64, 0.4),
                },
                vim: VimColors {
                    normal_background: transparent,
                    insert_background: transparent,
                    replace_background: transparent,
                    visual_background: transparent,
                    visual_line_background: transparent,
                    visual_block_background: transparent,
                    yank_background: accent.alpha(0.2),
                    helix_normal_background: transparent,
                    helix_select_background: transparent,
                    normal_foreground: transparent,
                    insert_foreground: transparent,
                    replace_foreground: transparent,
                    visual_foreground: transparent,
                    visual_line_foreground: transparent,
                    visual_block_foreground: transparent,
                    helix_normal_foreground: transparent,
                    helix_select_foreground: transparent,
                },
                version_control: VersionControlColors {
                    added: green,
                    modified: yellow,
                    deleted: red,
                    renamed: yellow,
                    conflict: orange,
                    ignored: text_placeholder,
                    word_added: green.alpha(0.35),
                    word_deleted: red.alpha(0.8),
                    conflict_marker_ours: green.alpha(0.5),
                    conflict_marker_theirs: blue.alpha(0.5),
                },

                // --- Layout tokens ---
                radius: px(6.0),

                // --- Raijin-specific ---
                block: BlockColors {
                    success_badge: green,
                    error_badge: red,
                    running_badge: yellow,
                },
                chart: ChartColors {
                    chart_1: accent,
                    chart_2: green,
                    chart_3: yellow,
                    chart_4: red,
                    chart_5: magenta,
                },
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
// Uses light-adapted values, fills from the original One Light where needed.
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
                // --- Semantic tokens (shadcn/W3C) ---
                primary: accent,
                primary_foreground: bg,
                secondary: element_bg,
                secondary_foreground: fg,
                muted: surface,
                muted_foreground: text_muted,
                accent: hover,
                accent_foreground: fg,
                destructive: red,
                destructive_foreground: fg,
                background: bg,
                foreground: fg,
                card: surface,
                card_foreground: fg,
                popover: elevated,
                popover_foreground: fg,
                border: oklch(0.85, 0.0, 0.0),
                input: element_bg,
                ring: accent,

                // --- Extended base tokens ---
                surface,
                elevated_surface: elevated,
                element_background: element_bg,
                element_hover: hover,
                element_active: active,
                element_selected: active,
                element_disabled: element_bg,
                element_selection: player.local().selection.alpha(0.25),
                drop_target_background: accent.alpha(0.1),
                drop_target_border: fg,
                ghost_element_background: transparent,
                ghost_element_hover: hover,
                ghost_element_active: active,
                ghost_element_selected: active,
                ghost_element_disabled: transparent,

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

                link_text_hover: accent,
                debugger_accent: red,

                // --- Contextual sub-structs ---
                editor: EditorColors {
                    foreground: fg,
                    background: bg,
                    gutter_background: bg,
                    subheader_background: surface,
                    active_line_background: oklch(0.96, 0.0, 0.0),
                    highlighted_line_background: accent.alpha(0.08),
                    debugger_active_line_background: accent.alpha(0.15),
                    line_number: text_placeholder,
                    active_line_number: fg,
                    hover_line_number: fg,
                    invisible: oklch(0.85, 0.0, 0.0),
                    wrap_guide: oklch(0.92, 0.0, 0.0),
                    active_wrap_guide: oklch(0.9, 0.0, 0.0),
                    indent_guide: subtle_border,
                    indent_guide_active: oklch(0.82, 0.0, 0.0),
                    document_highlight_read_background: accent.alpha(0.1),
                    document_highlight_write_background: accent.alpha(0.2),
                    document_highlight_bracket_background: green.alpha(0.12),
                },
                terminal: TerminalColors {
                    background: bg,
                    foreground: fg,
                    bright_foreground: oklch(0.15, 0.0, 0.0),
                    dim_foreground: text_muted,
                    accent: accent,
                    ansi: TerminalAnsiColors {
                        background: bg,
                        black: oklch(0.25, 0.0, 0.0),
                        red,
                        green,
                        yellow,
                        blue,
                        magenta,
                        cyan,
                        white: oklch(0.9, 0.0, 0.0),
                        bright_black: oklch(0.45, 0.0, 0.0),
                        bright_red: red.lighten(0.08),
                        bright_green: green.lighten(0.08),
                        bright_yellow: yellow.lighten(0.08),
                        bright_blue: blue.lighten(0.08),
                        bright_magenta: magenta.lighten(0.08),
                        bright_cyan: cyan.lighten(0.08),
                        bright_white: oklch(0.98, 0.0, 0.0),
                        dim_black: oklch(0.35, 0.0, 0.0),
                        dim_red: red.darken(0.1),
                        dim_green: green.darken(0.1),
                        dim_yellow: yellow.darken(0.1),
                        dim_blue: blue.darken(0.1),
                        dim_magenta: magenta.darken(0.1),
                        dim_cyan: cyan.darken(0.1),
                        dim_white: oklch(0.7, 0.0, 0.0),
                    },
                },
                panel: PanelColors {
                    background: surface,
                    focused_border: accent,
                    indent_guide: subtle_border,
                    indent_guide_hover: oklch(0.82, 0.0, 0.0),
                    indent_guide_active: oklch(0.82, 0.0, 0.0),
                    overlay_background: bg,
                    overlay_hover: hover,
                },
                pane: PaneColors {
                    focused_border: accent,
                    group_border: oklch(0.88, 0.0, 0.0),
                },
                tab: TabColors {
                    bar_background: tab_bg,
                    inactive_background: tab_bg,
                    active_background: bg,
                    inactive_foreground: text_muted,
                    active_foreground: fg,
                },
                scrollbar: ScrollbarColors {
                    thumb_background: oklcha(0.0, 0.0, 0.0, 0.08),
                    thumb_hover_background: oklcha(0.0, 0.0, 0.0, 0.15),
                    thumb_active_background: oklcha(0.0, 0.0, 0.0, 0.2),
                    thumb_border: transparent,
                    track_background: transparent,
                    track_border: transparent,
                },
                minimap: MinimapColors {
                    thumb_background: oklcha(0.0, 0.0, 0.0, 0.06),
                    thumb_hover_background: oklcha(0.0, 0.0, 0.0, 0.1),
                    thumb_active_background: oklcha(0.0, 0.0, 0.0, 0.15),
                    thumb_border: transparent,
                },
                status_bar: StatusBarColors {
                    background: bg,
                },
                title_bar: TitleBarColors {
                    background: bg,
                    inactive_background: surface,
                },
                toolbar: ToolbarColors {
                    background: bg,
                },
                search: SearchColors {
                    match_background: accent.alpha(0.15),
                    active_match_background: accent.alpha(0.3),
                },
                vim: VimColors {
                    normal_background: transparent,
                    insert_background: transparent,
                    replace_background: transparent,
                    visual_background: transparent,
                    visual_line_background: transparent,
                    visual_block_background: transparent,
                    yank_background: accent.alpha(0.15),
                    helix_normal_background: transparent,
                    helix_select_background: transparent,
                    normal_foreground: transparent,
                    insert_foreground: transparent,
                    replace_foreground: transparent,
                    visual_foreground: transparent,
                    visual_line_foreground: transparent,
                    visual_block_foreground: transparent,
                    helix_normal_foreground: transparent,
                    helix_select_foreground: transparent,
                },
                version_control: VersionControlColors {
                    added: green,
                    modified: yellow,
                    deleted: red,
                    renamed: yellow,
                    conflict: orange,
                    ignored: text_placeholder,
                    word_added: green.alpha(0.25),
                    word_deleted: red.alpha(0.6),
                    conflict_marker_ours: green.alpha(0.3),
                    conflict_marker_theirs: blue.alpha(0.3),
                },

                // --- Layout tokens ---
                radius: px(6.0),

                // --- Raijin-specific ---
                block: BlockColors {
                    success_badge: green,
                    error_badge: red,
                    running_badge: yellow,
                },
                chart: ChartColors {
                    chart_1: accent,
                    chart_2: green,
                    chart_3: yellow,
                    chart_4: red,
                    chart_5: magenta,
                },
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
