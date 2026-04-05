use inazuma::Oklch;

/// All theme colors used throughout the application UI.
///
/// Field names are token-compatible with Zed's ThemeColors for ecosystem interop.
/// All colors use the OKLCH color space for perceptual uniformity.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeColors {
    // === Borders ===

    /// Border color. Used for most borders, usually a high contrast color.
    pub border: Oklch,
    /// Border color. Used for deemphasized borders, like a visual divider.
    pub border_variant: Oklch,
    /// Border color. Used for focused elements, like keyboard focused list item.
    pub border_focused: Oklch,
    /// Border color. Used for selected elements, like an active search filter.
    pub border_selected: Oklch,
    /// Border color. Used for transparent borders as placeholders.
    pub border_transparent: Oklch,
    /// Border color. Used for disabled elements.
    pub border_disabled: Oklch,

    // === Surfaces / Backgrounds ===

    /// Background color for elevated surfaces like context menus, popups, dialogs.
    pub elevated_surface_background: Oklch,
    /// Background color for grounded surfaces like panels or tabs.
    pub surface_background: Oklch,
    /// Background color for the app background and blank panels.
    pub background: Oklch,
    /// Background color for elements that differ from their surface (buttons, inputs).
    pub element_background: Oklch,

    // === Element States ===

    /// Background for hovered elements.
    pub element_hover: Oklch,
    /// Background for active (pressed) elements.
    pub element_active: Oklch,
    /// Background for selected elements.
    pub element_selected: Oklch,
    /// Background for selections within UI elements.
    pub element_selection_background: Oklch,
    /// Background for disabled elements.
    pub element_disabled: Oklch,
    /// Background for drop target areas.
    pub drop_target_background: Oklch,
    /// Border for drop target areas.
    pub drop_target_border: Oklch,
    /// Background for ghost elements (same bg as surface).
    pub ghost_element_background: Oklch,
    /// Hover state for ghost elements.
    pub ghost_element_hover: Oklch,
    /// Active state for ghost elements.
    pub ghost_element_active: Oklch,
    /// Selected state for ghost elements.
    pub ghost_element_selected: Oklch,
    /// Disabled state for ghost elements.
    pub ghost_element_disabled: Oklch,

    // === Text ===

    /// Default text color.
    pub text: Oklch,
    /// Muted or deemphasized text color.
    pub text_muted: Oklch,
    /// Placeholder text color in input fields.
    pub text_placeholder: Oklch,
    /// Text color for disabled elements.
    pub text_disabled: Oklch,
    /// Accent text color for emphasis or highlights.
    pub text_accent: Oklch,

    // === Icons ===

    /// Default icon fill color.
    pub icon: Oklch,
    /// Muted icon fill color.
    pub icon_muted: Oklch,
    /// Disabled icon fill color.
    pub icon_disabled: Oklch,
    /// Placeholder icon fill color.
    pub icon_placeholder: Oklch,
    /// Accent icon fill color.
    pub icon_accent: Oklch,

    // === Workspace Chrome ===

    /// Status bar background.
    pub status_bar_background: Oklch,
    /// Title bar background.
    pub title_bar_background: Oklch,
    /// Title bar background when window is inactive.
    pub title_bar_inactive_background: Oklch,
    /// Toolbar background.
    pub toolbar_background: Oklch,
    /// Tab bar background.
    pub tab_bar_background: Oklch,
    /// Inactive tab background.
    pub tab_inactive_background: Oklch,
    /// Active tab background.
    pub tab_active_background: Oklch,
    /// Search match highlight background.
    pub search_match_background: Oklch,
    /// Active search match highlight background.
    pub search_active_match_background: Oklch,
    /// Panel background.
    pub panel_background: Oklch,
    /// Panel focused border.
    pub panel_focused_border: Oklch,
    /// Panel indent guide color.
    pub panel_indent_guide: Oklch,
    /// Panel indent guide hover color.
    pub panel_indent_guide_hover: Oklch,
    /// Panel indent guide active color.
    pub panel_indent_guide_active: Oklch,
    /// Panel overlay background.
    pub panel_overlay_background: Oklch,
    /// Panel overlay hover background.
    pub panel_overlay_hover: Oklch,
    /// Pane focused border.
    pub pane_focused_border: Oklch,
    /// Pane group border.
    pub pane_group_border: Oklch,

    // === Scrollbar ===

    /// Scrollbar thumb background.
    pub scrollbar_thumb_background: Oklch,
    /// Scrollbar thumb hover background.
    pub scrollbar_thumb_hover_background: Oklch,
    /// Scrollbar thumb active (dragging) background.
    pub scrollbar_thumb_active_background: Oklch,
    /// Scrollbar thumb border.
    pub scrollbar_thumb_border: Oklch,
    /// Scrollbar track background.
    pub scrollbar_track_background: Oklch,
    /// Scrollbar track border.
    pub scrollbar_track_border: Oklch,

    // === Editor ===

    /// Editor foreground (text) color.
    pub editor_foreground: Oklch,
    /// Editor background color.
    pub editor_background: Oklch,
    /// Editor gutter background.
    pub editor_gutter_background: Oklch,
    /// Editor subheader background.
    pub editor_subheader_background: Oklch,
    /// Editor active line background.
    pub editor_active_line_background: Oklch,
    /// Editor highlighted line background.
    pub editor_highlighted_line_background: Oklch,
    /// Editor line number text color.
    pub editor_line_number: Oklch,
    /// Editor active line number text color.
    pub editor_active_line_number: Oklch,
    /// Editor invisible characters color.
    pub editor_invisible: Oklch,
    /// Editor wrap guide color.
    pub editor_wrap_guide: Oklch,
    /// Editor active wrap guide color.
    pub editor_active_wrap_guide: Oklch,
    /// Editor indent guide color.
    pub editor_indent_guide: Oklch,
    /// Editor active indent guide color.
    pub editor_indent_guide_active: Oklch,
    /// Document highlight read-access background.
    pub editor_document_highlight_read_background: Oklch,
    /// Document highlight write-access background.
    pub editor_document_highlight_write_background: Oklch,
    /// Matching bracket highlight background.
    pub editor_document_highlight_bracket_background: Oklch,

    // === Terminal ANSI ===

    /// Terminal background color.
    pub terminal_background: Oklch,
    /// Terminal foreground color.
    pub terminal_foreground: Oklch,
    /// Terminal bright foreground color.
    pub terminal_bright_foreground: Oklch,
    /// Terminal dim foreground color.
    pub terminal_dim_foreground: Oklch,
    /// Terminal accent color — used for selection highlights, hover states, and UI accents.
    pub terminal_accent: Oklch,
    /// Terminal ANSI background color.
    pub terminal_ansi_background: Oklch,
    /// Black ANSI terminal color.
    pub terminal_ansi_black: Oklch,
    /// Bright black ANSI terminal color.
    pub terminal_ansi_bright_black: Oklch,
    /// Dim black ANSI terminal color.
    pub terminal_ansi_dim_black: Oklch,
    /// Red ANSI terminal color.
    pub terminal_ansi_red: Oklch,
    /// Bright red ANSI terminal color.
    pub terminal_ansi_bright_red: Oklch,
    /// Dim red ANSI terminal color.
    pub terminal_ansi_dim_red: Oklch,
    /// Green ANSI terminal color.
    pub terminal_ansi_green: Oklch,
    /// Bright green ANSI terminal color.
    pub terminal_ansi_bright_green: Oklch,
    /// Dim green ANSI terminal color.
    pub terminal_ansi_dim_green: Oklch,
    /// Yellow ANSI terminal color.
    pub terminal_ansi_yellow: Oklch,
    /// Bright yellow ANSI terminal color.
    pub terminal_ansi_bright_yellow: Oklch,
    /// Dim yellow ANSI terminal color.
    pub terminal_ansi_dim_yellow: Oklch,
    /// Blue ANSI terminal color.
    pub terminal_ansi_blue: Oklch,
    /// Bright blue ANSI terminal color.
    pub terminal_ansi_bright_blue: Oklch,
    /// Dim blue ANSI terminal color.
    pub terminal_ansi_dim_blue: Oklch,
    /// Magenta ANSI terminal color.
    pub terminal_ansi_magenta: Oklch,
    /// Bright magenta ANSI terminal color.
    pub terminal_ansi_bright_magenta: Oklch,
    /// Dim magenta ANSI terminal color.
    pub terminal_ansi_dim_magenta: Oklch,
    /// Cyan ANSI terminal color.
    pub terminal_ansi_cyan: Oklch,
    /// Bright cyan ANSI terminal color.
    pub terminal_ansi_bright_cyan: Oklch,
    /// Dim cyan ANSI terminal color.
    pub terminal_ansi_dim_cyan: Oklch,
    /// White ANSI terminal color.
    pub terminal_ansi_white: Oklch,
    /// Bright white ANSI terminal color.
    pub terminal_ansi_bright_white: Oklch,
    /// Dim white ANSI terminal color.
    pub terminal_ansi_dim_white: Oklch,

    // === Links ===

    /// Link text hover color.
    pub link_text_hover: Oklch,

    // === Version Control ===

    /// Added entry/hunk color.
    pub version_control_added: Oklch,
    /// Deleted entry color.
    pub version_control_deleted: Oklch,
    /// Modified entry color.
    pub version_control_modified: Oklch,
    /// Renamed entry color.
    pub version_control_renamed: Oklch,
    /// Conflicting entry color.
    pub version_control_conflict: Oklch,
    /// Ignored entry color.
    pub version_control_ignored: Oklch,
    /// Added word in word diff.
    pub version_control_word_added: Oklch,
    /// Deleted word in word diff.
    pub version_control_word_deleted: Oklch,
    /// "Ours" region of a merge conflict.
    pub version_control_conflict_marker_ours: Oklch,
    /// "Theirs" region of a merge conflict.
    pub version_control_conflict_marker_theirs: Oklch,

    // === Raijin-specific ===

    /// Block success badge color (exit code 0).
    pub block_success_badge: Oklch,
    /// Block error badge color (non-zero exit code).
    pub block_error_badge: Oklch,
    /// Block running badge color (command in progress).
    pub block_running_badge: Oklch,
}
