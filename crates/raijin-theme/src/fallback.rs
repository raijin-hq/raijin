use inazuma::{Oklch, Rgba, SharedString, oklcha};

use crate::colors::ThemeColors;
use crate::players::PlayerColor;
use crate::status::{StatusColors, StatusStyle};
use crate::syntax::SyntaxTheme;
use crate::theme::{Appearance, Theme, ThemeStyles};

/// Converts a hex color (0xRRGGBB) to Oklch via Rgba intermediary.
fn hex(value: u32) -> Oklch {
    let r = ((value >> 16) & 0xFF) as f32 / 255.0;
    let g = ((value >> 8) & 0xFF) as f32 / 255.0;
    let b = (value & 0xFF) as f32 / 255.0;
    Oklch::from(Rgba { r, g, b, a: 1.0 })
}

/// Converts a hex color with alpha (0xRRGGBBAA) to Oklch.
fn hex_a(value: u32, alpha: f32) -> Oklch {
    let mut c = hex(value);
    c.a = alpha;
    c
}

/// Returns the hardcoded Raijin Dark fallback theme.
///
/// Colors: #121212 background, #00BFFF accent (Cyan), #f1f1f1 foreground.
/// Used for bootstrap and tests when no theme file is available.
pub fn fallback_theme() -> Theme {
    let bg = hex(0x121212);
    let fg = hex(0xf1f1f1);
    let accent = hex(0x00BFFF);
    let surface = hex(0x1a1a1a);
    let elevated = hex(0x222222);
    let border_color = hex(0x333333);
    let muted = hex(0x888888);
    let disabled = hex(0x555555);
    let transparent = oklcha(0.0, 0.0, 0.0, 0.0);

    let colors = ThemeColors {
        // Borders
        border: border_color,
        border_variant: hex(0x2a2a2a),
        border_focused: accent,
        border_selected: accent,
        border_transparent: transparent,
        border_disabled: hex(0x2a2a2a),

        // Surfaces
        elevated_surface_background: elevated,
        surface_background: surface,
        background: bg,
        element_background: hex(0x1e1e1e),

        // Element states
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

        // Text
        text: fg,
        text_muted: muted,
        text_placeholder: hex(0x666666),
        text_disabled: disabled,
        text_accent: accent,

        // Icons
        icon: fg,
        icon_muted: muted,
        icon_disabled: disabled,
        icon_placeholder: hex(0x666666),
        icon_accent: accent,

        // Workspace chrome
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

        // Scrollbar
        scrollbar_thumb_background: hex_a(0xffffff, 0.1),
        scrollbar_thumb_hover_background: hex_a(0xffffff, 0.2),
        scrollbar_thumb_active_background: hex_a(0xffffff, 0.3),
        scrollbar_thumb_border: transparent,
        scrollbar_track_background: transparent,
        scrollbar_track_border: transparent,

        // Editor
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

        // Terminal ANSI — standard dark terminal palette
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

        // Links
        link_text_hover: accent,

        // Version control
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

        // Raijin-specific
        block_success_badge: hex(0x14F195),
        block_error_badge: hex(0xff5555),
        block_running_badge: hex(0xf1fa8c),
    };

    let status = StatusColors {
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
    };

    let players = vec![
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
        PlayerColor {
            cursor: hex(0x6272a4),
            background: hex_a(0x6272a4, 0.2),
            selection: hex_a(0x6272a4, 0.15),
        },
        PlayerColor {
            cursor: hex(0xff5555),
            background: hex_a(0xff5555, 0.2),
            selection: hex_a(0xff5555, 0.15),
        },
    ];

    Theme {
        id: "raijin-dark".into(),
        name: SharedString::from("Raijin Dark"),
        appearance: Appearance::Dark,
        styles: ThemeStyles {
            colors,
            status,
            syntax: SyntaxTheme::empty(),
            players,
        },
    }
}
