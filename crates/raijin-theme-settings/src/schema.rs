use inazuma::{HighlightStyle, Oklch};
use inazuma_settings_framework::IntoInazuma;
pub use inazuma_settings_content::{
    FontStyleContent, HighlightStyleContent, StatusColorsContent, ThemeColorsContent,
    ThemeStyleContent,
};

use raijin_theme::{
    StatusColorsRefinement, StatusStyleRefinement, ThemeColorsRefinement,
    EditorColorsRefinement, TerminalColorsRefinement, TerminalAnsiColorsRefinement,
    PanelColorsRefinement, PaneColorsRefinement, TabColorsRefinement,
    ScrollbarColorsRefinement, MinimapColorsRefinement, StatusBarColorsRefinement,
    TitleBarColorsRefinement, ToolbarColorsRefinement, SearchColorsRefinement,
    VimColorsRefinement, VersionControlColorsRefinement,
};

fn try_parse_color(color: &str) -> anyhow::Result<Oklch> {
    raijin_theme::parse_color(color)
}

/// Returns the syntax style overrides in the [`ThemeStyleContent`].
pub fn syntax_overrides(this: &ThemeStyleContent) -> Vec<(String, HighlightStyle)> {
    this.syntax
        .iter()
        .map(|(key, style)| {
            (
                key.clone(),
                HighlightStyle {
                    color: style
                        .color
                        .as_ref()
                        .and_then(|color| try_parse_color(color).ok()),
                    background_color: style
                        .background_color
                        .as_ref()
                        .and_then(|color| try_parse_color(color).ok()),
                    font_style: style.font_style.map(|s| s.into_inazuma()),
                    font_weight: style.font_weight.map(|w| w.into_inazuma()),
                    ..Default::default()
                },
            )
        })
        .collect()
}

fn parse_status_style(
    color: &Option<String>,
    background: &Option<String>,
    border: &Option<String>,
) -> StatusStyleRefinement {
    StatusStyleRefinement {
        color: color.as_ref().and_then(|c| try_parse_color(c).ok()),
        background: background.as_ref().and_then(|c| try_parse_color(c).ok()),
        border: border.as_ref().and_then(|c| try_parse_color(c).ok()),
    }
}

pub fn status_colors_refinement(colors: &StatusColorsContent) -> StatusColorsRefinement {
    StatusColorsRefinement {
        conflict: parse_status_style(&colors.conflict, &colors.conflict_background, &colors.conflict_border),
        created: parse_status_style(&colors.created, &colors.created_background, &colors.created_border),
        deleted: parse_status_style(&colors.deleted, &colors.deleted_background, &colors.deleted_border),
        error: parse_status_style(&colors.error, &colors.error_background, &colors.error_border),
        hidden: parse_status_style(&colors.hidden, &colors.hidden_background, &colors.hidden_border),
        hint: parse_status_style(&colors.hint, &colors.hint_background, &colors.hint_border),
        ignored: parse_status_style(&colors.ignored, &colors.ignored_background, &colors.ignored_border),
        info: parse_status_style(&colors.info, &colors.info_background, &colors.info_border),
        modified: parse_status_style(&colors.modified, &colors.modified_background, &colors.modified_border),
        predictive: parse_status_style(&colors.predictive, &colors.predictive_background, &colors.predictive_border),
        renamed: parse_status_style(&colors.renamed, &colors.renamed_background, &colors.renamed_border),
        success: parse_status_style(&colors.success, &colors.success_background, &colors.success_border),
        unreachable: parse_status_style(&colors.unreachable, &colors.unreachable_background, &colors.unreachable_border),
        warning: parse_status_style(&colors.warning, &colors.warning_background, &colors.warning_border),
    }
}

/// Helper to parse an optional color string.
fn c(field: &Option<String>) -> Option<Oklch> {
    field.as_ref().and_then(|c| try_parse_color(c).ok())
}

pub fn theme_colors_refinement(
    colors: &ThemeColorsContent,
    _status_colors: &StatusColorsRefinement,
) -> ThemeColorsRefinement {
    ThemeColorsRefinement {
        // ─── Flat fields ───
        border: c(&colors.border),
        border_variant: c(&colors.border_variant),
        border_focused: c(&colors.border_focused),
        border_selected: c(&colors.border_selected),
        border_transparent: c(&colors.border_transparent),
        border_disabled: c(&colors.border_disabled),
        elevated_surface: c(&colors.elevated_surface_background),
        surface: c(&colors.surface_background),
        background: c(&colors.background),
        element_background: c(&colors.element_background),
        element_hover: c(&colors.element_hover),
        element_active: c(&colors.element_active),
        element_selected: c(&colors.element_selected),
        element_disabled: c(&colors.element_disabled),
        element_selection: c(&colors.element_selection_background),
        drop_target_background: c(&colors.drop_target_background),
        drop_target_border: c(&colors.drop_target_border),
        ghost_element_background: c(&colors.ghost_element_background),
        ghost_element_hover: c(&colors.ghost_element_hover),
        ghost_element_active: c(&colors.ghost_element_active),
        ghost_element_selected: c(&colors.ghost_element_selected),
        ghost_element_disabled: c(&colors.ghost_element_disabled),
        text: c(&colors.text),
        text_muted: c(&colors.text_muted),
        text_placeholder: c(&colors.text_placeholder),
        text_disabled: c(&colors.text_disabled),
        text_accent: c(&colors.text_accent),
        icon: c(&colors.icon),
        icon_muted: c(&colors.icon_muted),
        icon_disabled: c(&colors.icon_disabled),
        icon_placeholder: c(&colors.icon_placeholder),
        icon_accent: c(&colors.icon_accent),
        link_text_hover: c(&colors.link_text_hover),
        debugger_accent: c(&colors.debugger_accent),

        // ─── Sub-refinement structs ───
        editor: EditorColorsRefinement {
            foreground: c(&colors.editor_foreground),
            background: c(&colors.editor_background),
            gutter_background: c(&colors.editor_gutter_background),
            subheader_background: c(&colors.editor_subheader_background),
            active_line_background: c(&colors.editor_active_line_background),
            highlighted_line_background: c(&colors.editor_highlighted_line_background),
            debugger_active_line_background: c(&colors.editor_debugger_active_line_background),
            line_number: c(&colors.editor_line_number),
            active_line_number: c(&colors.editor_active_line_number),
            hover_line_number: c(&colors.editor_hover_line_number),
            invisible: c(&colors.editor_invisible),
            wrap_guide: c(&colors.editor_wrap_guide),
            active_wrap_guide: c(&colors.editor_active_wrap_guide),
            indent_guide: c(&colors.editor_indent_guide),
            indent_guide_active: c(&colors.editor_indent_guide_active),
            document_highlight_read_background: c(&colors.editor_document_highlight_read_background),
            document_highlight_write_background: c(&colors.editor_document_highlight_write_background),
            document_highlight_bracket_background: c(&colors.editor_document_highlight_bracket_background),
            ..Default::default()
        },
        terminal: TerminalColorsRefinement {
            background: c(&colors.terminal_background),
            foreground: c(&colors.terminal_foreground),
            bright_foreground: c(&colors.terminal_bright_foreground),
            dim_foreground: c(&colors.terminal_dim_foreground),
            ansi: TerminalAnsiColorsRefinement {
                background: c(&colors.terminal_ansi_background),
                black: c(&colors.terminal_ansi_black),
                bright_black: c(&colors.terminal_ansi_bright_black),
                dim_black: c(&colors.terminal_ansi_dim_black),
                red: c(&colors.terminal_ansi_red),
                bright_red: c(&colors.terminal_ansi_bright_red),
                dim_red: c(&colors.terminal_ansi_dim_red),
                green: c(&colors.terminal_ansi_green),
                bright_green: c(&colors.terminal_ansi_bright_green),
                dim_green: c(&colors.terminal_ansi_dim_green),
                yellow: c(&colors.terminal_ansi_yellow),
                bright_yellow: c(&colors.terminal_ansi_bright_yellow),
                dim_yellow: c(&colors.terminal_ansi_dim_yellow),
                blue: c(&colors.terminal_ansi_blue),
                bright_blue: c(&colors.terminal_ansi_bright_blue),
                dim_blue: c(&colors.terminal_ansi_dim_blue),
                magenta: c(&colors.terminal_ansi_magenta),
                bright_magenta: c(&colors.terminal_ansi_bright_magenta),
                dim_magenta: c(&colors.terminal_ansi_dim_magenta),
                cyan: c(&colors.terminal_ansi_cyan),
                bright_cyan: c(&colors.terminal_ansi_bright_cyan),
                dim_cyan: c(&colors.terminal_ansi_dim_cyan),
                white: c(&colors.terminal_ansi_white),
                bright_white: c(&colors.terminal_ansi_bright_white),
                dim_white: c(&colors.terminal_ansi_dim_white),
                ..Default::default()
            },
            ..Default::default()
        },
        panel: PanelColorsRefinement {
            background: c(&colors.panel_background),
            focused_border: c(&colors.panel_focused_border),
            indent_guide: c(&colors.panel_indent_guide),
            indent_guide_hover: c(&colors.panel_indent_guide_hover),
            indent_guide_active: c(&colors.panel_indent_guide_active),
            overlay_background: c(&colors.panel_overlay_background),
            overlay_hover: c(&colors.panel_overlay_hover),
            ..Default::default()
        },
        pane: PaneColorsRefinement {
            focused_border: c(&colors.pane_focused_border),
            group_border: c(&colors.pane_group_border),
            ..Default::default()
        },
        tab: TabColorsRefinement {
            bar_background: c(&colors.tab_bar_background),
            inactive_background: c(&colors.tab_inactive_background),
            active_background: c(&colors.tab_active_background),
            ..Default::default()
        },
        scrollbar: ScrollbarColorsRefinement {
            thumb_background: c(&colors.scrollbar_thumb_background)
                .or_else(|| c(&colors.deprecated_scrollbar_thumb_background)),
            thumb_hover_background: c(&colors.scrollbar_thumb_hover_background),
            thumb_border: c(&colors.scrollbar_thumb_border),
            track_background: c(&colors.scrollbar_track_background),
            track_border: c(&colors.scrollbar_track_border),
            ..Default::default()
        },
        minimap: MinimapColorsRefinement {
            thumb_background: c(&colors.minimap_thumb_background),
            thumb_hover_background: c(&colors.minimap_thumb_hover_background),
            thumb_active_background: c(&colors.minimap_thumb_active_background),
            thumb_border: c(&colors.minimap_thumb_border),
            ..Default::default()
        },
        status_bar: StatusBarColorsRefinement {
            background: c(&colors.status_bar_background),
            ..Default::default()
        },
        title_bar: TitleBarColorsRefinement {
            background: c(&colors.title_bar_background),
            inactive_background: c(&colors.title_bar_inactive_background),
            ..Default::default()
        },
        toolbar: ToolbarColorsRefinement {
            background: c(&colors.toolbar_background),
            ..Default::default()
        },
        search: SearchColorsRefinement {
            match_background: c(&colors.search_match_background),
            active_match_background: c(&colors.search_active_match_background),
            ..Default::default()
        },
        vim: VimColorsRefinement {
            normal_background: c(&colors.vim_normal_background),
            insert_background: c(&colors.vim_insert_background),
            replace_background: c(&colors.vim_replace_background),
            visual_background: c(&colors.vim_visual_background),
            visual_line_background: c(&colors.vim_visual_line_background),
            visual_block_background: c(&colors.vim_visual_block_background),
            yank_background: c(&colors.vim_yank_background),
            helix_normal_background: c(&colors.vim_helix_normal_background),
            helix_select_background: c(&colors.vim_helix_select_background),
            normal_foreground: c(&colors.vim_normal_foreground),
            insert_foreground: c(&colors.vim_insert_foreground),
            replace_foreground: c(&colors.vim_replace_foreground),
            visual_foreground: c(&colors.vim_visual_foreground),
            visual_line_foreground: c(&colors.vim_visual_line_foreground),
            visual_block_foreground: c(&colors.vim_visual_block_foreground),
            helix_normal_foreground: c(&colors.vim_helix_normal_foreground),
            helix_select_foreground: c(&colors.vim_helix_select_foreground),
            ..Default::default()
        },
        version_control: VersionControlColorsRefinement {
            added: c(&colors.version_control_added),
            deleted: c(&colors.version_control_deleted),
            modified: c(&colors.version_control_modified),
            renamed: c(&colors.version_control_renamed),
            conflict: c(&colors.version_control_conflict),
            ignored: c(&colors.version_control_ignored),
            word_added: c(&colors.version_control_word_added),
            word_deleted: c(&colors.version_control_word_deleted),
            conflict_marker_ours: c(&colors.version_control_conflict_marker_ours),
            conflict_marker_theirs: c(&colors.version_control_conflict_marker_theirs),
            ..Default::default()
        },
        ..Default::default()
    }
}
