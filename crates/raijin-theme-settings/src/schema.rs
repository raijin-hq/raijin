use inazuma::{HighlightStyle, Oklch};
use inazuma_settings_framework::IntoInazuma;
pub use inazuma_settings_content::{
    FontStyleContent, HighlightStyleContent, StatusColorsContent, ThemeColorsContent,
    ThemeStyleContent,
};

use raijin_theme::{StatusColorsRefinement, StatusStyleRefinement, ThemeColorsRefinement};

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

pub fn theme_colors_refinement(
    colors: &ThemeColorsContent,
    _status_colors: &StatusColorsRefinement,
) -> ThemeColorsRefinement {
    ThemeColorsRefinement {
        border: colors.border.as_ref().and_then(|c| try_parse_color(c).ok()),
        border_variant: colors.border_variant.as_ref().and_then(|c| try_parse_color(c).ok()),
        border_focused: colors.border_focused.as_ref().and_then(|c| try_parse_color(c).ok()),
        border_selected: colors.border_selected.as_ref().and_then(|c| try_parse_color(c).ok()),
        border_transparent: colors.border_transparent.as_ref().and_then(|c| try_parse_color(c).ok()),
        border_disabled: colors.border_disabled.as_ref().and_then(|c| try_parse_color(c).ok()),
        elevated_surface_background: colors.elevated_surface_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        surface_background: colors.surface_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        background: colors.background.as_ref().and_then(|c| try_parse_color(c).ok()),
        element_background: colors.element_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        element_hover: colors.element_hover.as_ref().and_then(|c| try_parse_color(c).ok()),
        element_active: colors.element_active.as_ref().and_then(|c| try_parse_color(c).ok()),
        element_selected: colors.element_selected.as_ref().and_then(|c| try_parse_color(c).ok()),
        element_disabled: colors.element_disabled.as_ref().and_then(|c| try_parse_color(c).ok()),
        drop_target_background: colors.drop_target_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        ghost_element_background: colors.ghost_element_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        ghost_element_hover: colors.ghost_element_hover.as_ref().and_then(|c| try_parse_color(c).ok()),
        ghost_element_active: colors.ghost_element_active.as_ref().and_then(|c| try_parse_color(c).ok()),
        ghost_element_selected: colors.ghost_element_selected.as_ref().and_then(|c| try_parse_color(c).ok()),
        ghost_element_disabled: colors.ghost_element_disabled.as_ref().and_then(|c| try_parse_color(c).ok()),
        text: colors.text.as_ref().and_then(|c| try_parse_color(c).ok()),
        text_muted: colors.text_muted.as_ref().and_then(|c| try_parse_color(c).ok()),
        text_placeholder: colors.text_placeholder.as_ref().and_then(|c| try_parse_color(c).ok()),
        text_disabled: colors.text_disabled.as_ref().and_then(|c| try_parse_color(c).ok()),
        text_accent: colors.text_accent.as_ref().and_then(|c| try_parse_color(c).ok()),
        icon: colors.icon.as_ref().and_then(|c| try_parse_color(c).ok()),
        icon_muted: colors.icon_muted.as_ref().and_then(|c| try_parse_color(c).ok()),
        icon_disabled: colors.icon_disabled.as_ref().and_then(|c| try_parse_color(c).ok()),
        icon_placeholder: colors.icon_placeholder.as_ref().and_then(|c| try_parse_color(c).ok()),
        icon_accent: colors.icon_accent.as_ref().and_then(|c| try_parse_color(c).ok()),
        status_bar_background: colors.status_bar_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        title_bar_background: colors.title_bar_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        title_bar_inactive_background: colors.title_bar_inactive_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        toolbar_background: colors.toolbar_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        tab_bar_background: colors.tab_bar_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        tab_inactive_background: colors.tab_inactive_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        tab_active_background: colors.tab_active_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        search_match_background: colors.search_match_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        panel_background: colors.panel_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        panel_focused_border: colors.panel_focused_border.as_ref().and_then(|c| try_parse_color(c).ok()),
        pane_focused_border: colors.pane_focused_border.as_ref().and_then(|c| try_parse_color(c).ok()),
        scrollbar_thumb_background: colors.scrollbar_thumb_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        scrollbar_thumb_hover_background: colors.scrollbar_thumb_hover_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        scrollbar_thumb_border: colors.scrollbar_thumb_border.as_ref().and_then(|c| try_parse_color(c).ok()),
        scrollbar_track_background: colors.scrollbar_track_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        scrollbar_track_border: colors.scrollbar_track_border.as_ref().and_then(|c| try_parse_color(c).ok()),
        editor_foreground: colors.editor_foreground.as_ref().and_then(|c| try_parse_color(c).ok()),
        editor_background: colors.editor_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        editor_gutter_background: colors.editor_gutter_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        editor_subheader_background: colors.editor_subheader_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        editor_active_line_background: colors.editor_active_line_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        editor_highlighted_line_background: colors.editor_highlighted_line_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        editor_line_number: colors.editor_line_number.as_ref().and_then(|c| try_parse_color(c).ok()),
        editor_active_line_number: colors.editor_active_line_number.as_ref().and_then(|c| try_parse_color(c).ok()),
        editor_invisible: colors.editor_invisible.as_ref().and_then(|c| try_parse_color(c).ok()),
        editor_wrap_guide: colors.editor_wrap_guide.as_ref().and_then(|c| try_parse_color(c).ok()),
        editor_active_wrap_guide: colors.editor_active_wrap_guide.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_background: colors.terminal_background.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_foreground: colors.terminal_foreground.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_bright_foreground: colors.terminal_bright_foreground.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_dim_foreground: colors.terminal_dim_foreground.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_black: colors.terminal_ansi_black.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_bright_black: colors.terminal_ansi_bright_black.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_dim_black: colors.terminal_ansi_dim_black.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_red: colors.terminal_ansi_red.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_bright_red: colors.terminal_ansi_bright_red.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_dim_red: colors.terminal_ansi_dim_red.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_green: colors.terminal_ansi_green.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_bright_green: colors.terminal_ansi_bright_green.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_dim_green: colors.terminal_ansi_dim_green.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_yellow: colors.terminal_ansi_yellow.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_bright_yellow: colors.terminal_ansi_bright_yellow.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_dim_yellow: colors.terminal_ansi_dim_yellow.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_blue: colors.terminal_ansi_blue.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_bright_blue: colors.terminal_ansi_bright_blue.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_dim_blue: colors.terminal_ansi_dim_blue.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_magenta: colors.terminal_ansi_magenta.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_bright_magenta: colors.terminal_ansi_bright_magenta.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_dim_magenta: colors.terminal_ansi_dim_magenta.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_cyan: colors.terminal_ansi_cyan.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_bright_cyan: colors.terminal_ansi_bright_cyan.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_dim_cyan: colors.terminal_ansi_dim_cyan.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_white: colors.terminal_ansi_white.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_bright_white: colors.terminal_ansi_bright_white.as_ref().and_then(|c| try_parse_color(c).ok()),
        terminal_ansi_dim_white: colors.terminal_ansi_dim_white.as_ref().and_then(|c| try_parse_color(c).ok()),
        link_text_hover: colors.link_text_hover.as_ref().and_then(|c| try_parse_color(c).ok()),
        ..Default::default()
    }
}
