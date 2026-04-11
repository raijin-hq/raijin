#![allow(missing_docs)]

use inazuma::{App, Oklch, Pixels, SharedString, WindowBackgroundAppearance, px};
use inazuma_refineable::Refineable;
use std::sync::Arc;
use strum::{AsRefStr, EnumIter, IntoEnumIterator};

use crate::{
    AccentColors, ActiveTheme, PlayerColors, StatusColors, StatusColorsRefinement, SyntaxTheme,
    SystemColors,
};

// ─────────────────────────────────────────────────────────────────────────────
// Sub-Structs
// ─────────────────────────────────────────────────────────────────────────────

/// Editor area colors.
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct EditorColors {
    pub foreground: Oklch,
    pub background: Oklch,
    pub gutter_background: Oklch,
    pub subheader_background: Oklch,
    pub active_line_background: Oklch,
    pub highlighted_line_background: Oklch,
    pub debugger_active_line_background: Oklch,
    pub line_number: Oklch,
    pub active_line_number: Oklch,
    pub hover_line_number: Oklch,
    pub invisible: Oklch,
    pub wrap_guide: Oklch,
    pub active_wrap_guide: Oklch,
    pub indent_guide: Oklch,
    pub indent_guide_active: Oklch,
    pub document_highlight_read_background: Oklch,
    pub document_highlight_write_background: Oklch,
    pub document_highlight_bracket_background: Oklch,
}

/// Terminal ANSI color palette (standard, bright, dim).
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct TerminalAnsiColors {
    pub background: Oklch,
    pub black: Oklch,
    pub bright_black: Oklch,
    pub dim_black: Oklch,
    pub red: Oklch,
    pub bright_red: Oklch,
    pub dim_red: Oklch,
    pub green: Oklch,
    pub bright_green: Oklch,
    pub dim_green: Oklch,
    pub yellow: Oklch,
    pub bright_yellow: Oklch,
    pub dim_yellow: Oklch,
    pub blue: Oklch,
    pub bright_blue: Oklch,
    pub dim_blue: Oklch,
    pub magenta: Oklch,
    pub bright_magenta: Oklch,
    pub dim_magenta: Oklch,
    pub cyan: Oklch,
    pub bright_cyan: Oklch,
    pub dim_cyan: Oklch,
    pub white: Oklch,
    pub bright_white: Oklch,
    pub dim_white: Oklch,
}

/// Terminal colors.
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct TerminalColors {
    pub background: Oklch,
    pub foreground: Oklch,
    pub bright_foreground: Oklch,
    pub dim_foreground: Oklch,
    pub accent: Oklch,
    #[refineable]
    pub ansi: TerminalAnsiColors,
}

/// Panel colors.
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct PanelColors {
    pub background: Oklch,
    pub focused_border: Oklch,
    pub indent_guide: Oklch,
    pub indent_guide_hover: Oklch,
    pub indent_guide_active: Oklch,
    pub overlay_background: Oklch,
    pub overlay_hover: Oklch,
}

/// Pane colors.
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct PaneColors {
    pub focused_border: Oklch,
    pub group_border: Oklch,
}

/// Tab bar colors.
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct TabColors {
    pub bar_background: Oklch,
    pub inactive_background: Oklch,
    pub active_background: Oklch,
    pub inactive_foreground: Oklch,
    pub active_foreground: Oklch,
}

/// Scrollbar colors.
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct ScrollbarColors {
    pub thumb_background: Oklch,
    pub thumb_hover_background: Oklch,
    pub thumb_active_background: Oklch,
    pub thumb_border: Oklch,
    pub track_background: Oklch,
    pub track_border: Oklch,
}

/// Minimap colors.
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct MinimapColors {
    pub thumb_background: Oklch,
    pub thumb_hover_background: Oklch,
    pub thumb_active_background: Oklch,
    pub thumb_border: Oklch,
}

/// Status bar colors.
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct StatusBarColors {
    pub background: Oklch,
}

/// Title bar colors.
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct TitleBarColors {
    pub background: Oklch,
    pub inactive_background: Oklch,
}

/// Toolbar colors.
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct ToolbarColors {
    pub background: Oklch,
}

/// Search highlight colors.
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct SearchColors {
    pub match_background: Oklch,
    pub active_match_background: Oklch,
}

/// Vim mode indicator colors.
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct VimColors {
    pub normal_background: Oklch,
    pub insert_background: Oklch,
    pub replace_background: Oklch,
    pub visual_background: Oklch,
    pub visual_line_background: Oklch,
    pub visual_block_background: Oklch,
    pub yank_background: Oklch,
    pub helix_normal_background: Oklch,
    pub helix_select_background: Oklch,
    pub normal_foreground: Oklch,
    pub insert_foreground: Oklch,
    pub replace_foreground: Oklch,
    pub visual_foreground: Oklch,
    pub visual_line_foreground: Oklch,
    pub visual_block_foreground: Oklch,
    pub helix_normal_foreground: Oklch,
    pub helix_select_foreground: Oklch,
}

/// Version control status colors.
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct VersionControlColors {
    pub added: Oklch,
    pub deleted: Oklch,
    pub modified: Oklch,
    pub renamed: Oklch,
    pub conflict: Oklch,
    pub ignored: Oklch,
    pub word_added: Oklch,
    pub word_deleted: Oklch,
    pub conflict_marker_ours: Oklch,
    pub conflict_marker_theirs: Oklch,
}

/// Raijin command block badge colors.
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct BlockColors {
    pub success_badge: Oklch,
    pub error_badge: Oklch,
    pub running_badge: Oklch,
}

/// Chart series colors.
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct ChartColors {
    pub chart_1: Oklch,
    pub chart_2: Oklch,
    pub chart_3: Oklch,
    pub chart_4: Oklch,
    pub chart_5: Oklch,
}

// ─────────────────────────────────────────────────────────────────────────────
// ThemeColors
// ─────────────────────────────────────────────────────────────────────────────

/// The complete set of design tokens for a theme.
///
/// Follows W3C Design Tokens / shadcn conventions:
/// - **Semantic tokens** (`primary`, `secondary`, `destructive`, etc.) at the top level
/// - **Contextual groups** (`editor`, `terminal`, `panel`, etc.) as sub-structs
/// - **Extended base tokens** (element states, text variants, icons) stay flat
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct ThemeColors {
    // ─── Semantic Tokens (shadcn/W3C standard) ───

    /// Primary action color (buttons, links, active indicators).
    pub primary: Oklch,
    /// Foreground on primary backgrounds.
    pub primary_foreground: Oklch,
    /// Secondary action color (less prominent buttons, badges).
    pub secondary: Oklch,
    /// Foreground on secondary backgrounds.
    pub secondary_foreground: Oklch,
    /// Muted background for subdued elements.
    pub muted: Oklch,
    /// Muted/subdued foreground color.
    pub muted_foreground: Oklch,
    /// Accent color for highlights and emphasis.
    pub accent: Oklch,
    /// Foreground on accent backgrounds.
    pub accent_foreground: Oklch,
    /// Destructive/error action color.
    pub destructive: Oklch,
    /// Foreground on destructive backgrounds.
    pub destructive_foreground: Oklch,
    /// App background color.
    pub background: Oklch,
    /// Default foreground/text color.
    pub foreground: Oklch,
    /// Card/surface background.
    pub card: Oklch,
    /// Foreground on card backgrounds.
    pub card_foreground: Oklch,
    /// Popover/elevated surface background.
    pub popover: Oklch,
    /// Foreground on popover backgrounds.
    pub popover_foreground: Oklch,
    /// Default border color.
    pub border: Oklch,
    /// Input element background.
    pub input: Oklch,
    /// Focus ring color.
    pub ring: Oklch,

    // ─── Extended base tokens (cross-cutting, flat) ───

    /// Grounded surface background (panels, tabs).
    pub surface: Oklch,
    /// Elevated surface background (context menus, popups, dialogs).
    pub elevated_surface: Oklch,

    /// Deemphasized border (divider between sections).
    pub border_variant: Oklch,
    /// Border for focused elements.
    pub border_focused: Oklch,
    /// Border for selected elements.
    pub border_selected: Oklch,
    /// Transparent border placeholder.
    pub border_transparent: Oklch,
    /// Border for disabled elements.
    pub border_disabled: Oklch,

    /// Element background (buttons, inputs, checkboxes).
    pub element_background: Oklch,
    /// Element hover state.
    pub element_hover: Oklch,
    /// Element active/pressed state.
    pub element_active: Oklch,
    /// Element selected state.
    pub element_selected: Oklch,
    /// Element disabled state.
    pub element_disabled: Oklch,
    /// Selection background in UI elements.
    pub element_selection: Oklch,

    /// Ghost element background (same bg as container surface).
    pub ghost_element_background: Oklch,
    /// Ghost element hover state.
    pub ghost_element_hover: Oklch,
    /// Ghost element active state.
    pub ghost_element_active: Oklch,
    /// Ghost element selected state.
    pub ghost_element_selected: Oklch,
    /// Ghost element disabled state.
    pub ghost_element_disabled: Oklch,

    /// Drop target background.
    pub drop_target_background: Oklch,
    /// Drop target border.
    pub drop_target_border: Oklch,

    /// Default text color.
    pub text: Oklch,
    /// Muted text color.
    pub text_muted: Oklch,
    /// Placeholder text color.
    pub text_placeholder: Oklch,
    /// Disabled text color.
    pub text_disabled: Oklch,
    /// Accent text color (emphasis, highlighting).
    pub text_accent: Oklch,

    /// Default icon fill.
    pub icon: Oklch,
    /// Muted icon fill.
    pub icon_muted: Oklch,
    /// Disabled icon fill.
    pub icon_disabled: Oklch,
    /// Placeholder icon fill.
    pub icon_placeholder: Oklch,
    /// Accent icon fill.
    pub icon_accent: Oklch,

    /// Link text hover color.
    pub link_text_hover: Oklch,
    /// Debugger accent (breakpoints).
    pub debugger_accent: Oklch,

    // ─── Contextual (sub-structs) ───

    /// Editor colors.
    #[refineable]
    pub editor: EditorColors,
    /// Terminal colors.
    #[refineable]
    pub terminal: TerminalColors,
    /// Panel colors.
    #[refineable]
    pub panel: PanelColors,
    /// Pane colors.
    #[refineable]
    pub pane: PaneColors,
    /// Tab bar colors.
    #[refineable]
    pub tab: TabColors,
    /// Scrollbar colors.
    #[refineable]
    pub scrollbar: ScrollbarColors,
    /// Minimap colors.
    #[refineable]
    pub minimap: MinimapColors,
    /// Status bar colors.
    #[refineable]
    pub status_bar: StatusBarColors,
    /// Title bar colors.
    #[refineable]
    pub title_bar: TitleBarColors,
    /// Toolbar colors.
    #[refineable]
    pub toolbar: ToolbarColors,
    /// Search highlight colors.
    #[refineable]
    pub search: SearchColors,
    /// Vim mode indicator colors.
    #[refineable]
    pub vim: VimColors,
    /// Version control status colors.
    #[refineable]
    pub version_control: VersionControlColors,

    // ─── Layout tokens ───

    /// Default border radius. All variants (sm, md, lg, xl) scale from this value.
    pub radius: Pixels,

    // ─── Raijin-specific ───

    /// Command block badge colors.
    #[refineable]
    pub block: BlockColors,
    /// Chart series colors.
    #[refineable]
    pub chart: ChartColors,
}

impl ThemeColors {
    /// Small border radius (0.6× base).
    pub fn radius_sm(&self) -> Pixels {
        px(f32::from(self.radius) * 0.6)
    }

    /// Medium border radius (0.8× base).
    pub fn radius_md(&self) -> Pixels {
        px(f32::from(self.radius) * 0.8)
    }

    /// Large border radius (1.333× base).
    pub fn radius_lg(&self) -> Pixels {
        px(f32::from(self.radius) * 1.333)
    }

    /// Extra-large border radius (1.667× base).
    pub fn radius_xl(&self) -> Pixels {
        px(f32::from(self.radius) * 1.667)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ThemeColorField — programmatic access to individual color fields
// ─────────────────────────────────────────────────────────────────────────────

#[derive(EnumIter, Debug, Clone, Copy, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum ThemeColorField {
    // Semantic
    Primary,
    PrimaryForeground,
    Secondary,
    SecondaryForeground,
    Muted,
    MutedForeground,
    Accent,
    AccentForeground,
    Destructive,
    DestructiveForeground,
    Background,
    Foreground,
    Card,
    CardForeground,
    Popover,
    PopoverForeground,
    Border,
    Input,
    Ring,
    // Extended base
    Surface,
    ElevatedSurface,
    BorderVariant,
    BorderFocused,
    BorderSelected,
    BorderTransparent,
    BorderDisabled,
    ElementBackground,
    ElementHover,
    ElementActive,
    ElementSelected,
    ElementDisabled,
    ElementSelection,
    GhostElementBackground,
    GhostElementHover,
    GhostElementActive,
    GhostElementSelected,
    GhostElementDisabled,
    DropTargetBackground,
    DropTargetBorder,
    Text,
    TextMuted,
    TextPlaceholder,
    TextDisabled,
    TextAccent,
    Icon,
    IconMuted,
    IconDisabled,
    IconPlaceholder,
    IconAccent,
    LinkTextHover,
    DebuggerAccent,
    // Editor
    EditorForeground,
    EditorBackground,
    EditorGutterBackground,
    EditorSubheaderBackground,
    EditorActiveLineBackground,
    EditorHighlightedLineBackground,
    EditorDebuggerActiveLineBackground,
    EditorLineNumber,
    EditorActiveLineNumber,
    EditorHoverLineNumber,
    EditorInvisible,
    EditorWrapGuide,
    EditorActiveWrapGuide,
    EditorIndentGuide,
    EditorIndentGuideActive,
    EditorDocumentHighlightReadBackground,
    EditorDocumentHighlightWriteBackground,
    EditorDocumentHighlightBracketBackground,
    // Terminal
    TerminalBackground,
    TerminalForeground,
    TerminalBrightForeground,
    TerminalDimForeground,
    TerminalAccent,
    TerminalAnsiBackground,
    TerminalAnsiBlack,
    TerminalAnsiBrightBlack,
    TerminalAnsiDimBlack,
    TerminalAnsiRed,
    TerminalAnsiBrightRed,
    TerminalAnsiDimRed,
    TerminalAnsiGreen,
    TerminalAnsiBrightGreen,
    TerminalAnsiDimGreen,
    TerminalAnsiYellow,
    TerminalAnsiBrightYellow,
    TerminalAnsiDimYellow,
    TerminalAnsiBlue,
    TerminalAnsiBrightBlue,
    TerminalAnsiDimBlue,
    TerminalAnsiMagenta,
    TerminalAnsiBrightMagenta,
    TerminalAnsiDimMagenta,
    TerminalAnsiCyan,
    TerminalAnsiBrightCyan,
    TerminalAnsiDimCyan,
    TerminalAnsiWhite,
    TerminalAnsiBrightWhite,
    TerminalAnsiDimWhite,
    // Panel
    PanelBackground,
    PanelFocusedBorder,
    PanelIndentGuide,
    PanelIndentGuideHover,
    PanelIndentGuideActive,
    PanelOverlayBackground,
    PanelOverlayHover,
    // Pane
    PaneFocusedBorder,
    PaneGroupBorder,
    // Tab
    TabBarBackground,
    TabInactiveBackground,
    TabActiveBackground,
    TabInactiveForeground,
    TabActiveForeground,
    // Scrollbar
    ScrollbarThumbBackground,
    ScrollbarThumbHoverBackground,
    ScrollbarThumbActiveBackground,
    ScrollbarThumbBorder,
    ScrollbarTrackBackground,
    ScrollbarTrackBorder,
    // Minimap
    MinimapThumbBackground,
    MinimapThumbHoverBackground,
    MinimapThumbActiveBackground,
    MinimapThumbBorder,
    // Status bar
    StatusBarBackground,
    // Title bar
    TitleBarBackground,
    TitleBarInactiveBackground,
    // Toolbar
    ToolbarBackground,
    // Search
    SearchMatchBackground,
    SearchActiveMatchBackground,
    // Vim
    VimNormalBackground,
    VimInsertBackground,
    VimReplaceBackground,
    VimVisualBackground,
    VimVisualLineBackground,
    VimVisualBlockBackground,
    VimYankBackground,
    VimHelixNormalBackground,
    VimHelixSelectBackground,
    VimNormalForeground,
    VimInsertForeground,
    VimReplaceForeground,
    VimVisualForeground,
    VimVisualLineForeground,
    VimVisualBlockForeground,
    VimHelixNormalForeground,
    VimHelixSelectForeground,
    // Version control
    VersionControlAdded,
    VersionControlDeleted,
    VersionControlModified,
    VersionControlRenamed,
    VersionControlConflict,
    VersionControlIgnored,
    VersionControlWordAdded,
    VersionControlWordDeleted,
    VersionControlConflictMarkerOurs,
    VersionControlConflictMarkerTheirs,
    // Block
    BlockSuccessBadge,
    BlockErrorBadge,
    BlockRunningBadge,
    // Chart
    Chart1,
    Chart2,
    Chart3,
    Chart4,
    Chart5,
}

impl ThemeColors {
    pub fn color(&self, field: ThemeColorField) -> Oklch {
        match field {
            // Semantic
            ThemeColorField::Primary => self.primary,
            ThemeColorField::PrimaryForeground => self.primary_foreground,
            ThemeColorField::Secondary => self.secondary,
            ThemeColorField::SecondaryForeground => self.secondary_foreground,
            ThemeColorField::Muted => self.muted,
            ThemeColorField::MutedForeground => self.muted_foreground,
            ThemeColorField::Accent => self.accent,
            ThemeColorField::AccentForeground => self.accent_foreground,
            ThemeColorField::Destructive => self.destructive,
            ThemeColorField::DestructiveForeground => self.destructive_foreground,
            ThemeColorField::Background => self.background,
            ThemeColorField::Foreground => self.foreground,
            ThemeColorField::Card => self.card,
            ThemeColorField::CardForeground => self.card_foreground,
            ThemeColorField::Popover => self.popover,
            ThemeColorField::PopoverForeground => self.popover_foreground,
            ThemeColorField::Border => self.border,
            ThemeColorField::Input => self.input,
            ThemeColorField::Ring => self.ring,
            // Extended base
            ThemeColorField::Surface => self.surface,
            ThemeColorField::ElevatedSurface => self.elevated_surface,
            ThemeColorField::BorderVariant => self.border_variant,
            ThemeColorField::BorderFocused => self.border_focused,
            ThemeColorField::BorderSelected => self.border_selected,
            ThemeColorField::BorderTransparent => self.border_transparent,
            ThemeColorField::BorderDisabled => self.border_disabled,
            ThemeColorField::ElementBackground => self.element_background,
            ThemeColorField::ElementHover => self.element_hover,
            ThemeColorField::ElementActive => self.element_active,
            ThemeColorField::ElementSelected => self.element_selected,
            ThemeColorField::ElementDisabled => self.element_disabled,
            ThemeColorField::ElementSelection => self.element_selection,
            ThemeColorField::GhostElementBackground => self.ghost_element_background,
            ThemeColorField::GhostElementHover => self.ghost_element_hover,
            ThemeColorField::GhostElementActive => self.ghost_element_active,
            ThemeColorField::GhostElementSelected => self.ghost_element_selected,
            ThemeColorField::GhostElementDisabled => self.ghost_element_disabled,
            ThemeColorField::DropTargetBackground => self.drop_target_background,
            ThemeColorField::DropTargetBorder => self.drop_target_border,
            ThemeColorField::Text => self.text,
            ThemeColorField::TextMuted => self.text_muted,
            ThemeColorField::TextPlaceholder => self.text_placeholder,
            ThemeColorField::TextDisabled => self.text_disabled,
            ThemeColorField::TextAccent => self.text_accent,
            ThemeColorField::Icon => self.icon,
            ThemeColorField::IconMuted => self.icon_muted,
            ThemeColorField::IconDisabled => self.icon_disabled,
            ThemeColorField::IconPlaceholder => self.icon_placeholder,
            ThemeColorField::IconAccent => self.icon_accent,
            ThemeColorField::LinkTextHover => self.link_text_hover,
            ThemeColorField::DebuggerAccent => self.debugger_accent,
            // Editor
            ThemeColorField::EditorForeground => self.editor.foreground,
            ThemeColorField::EditorBackground => self.editor.background,
            ThemeColorField::EditorGutterBackground => self.editor.gutter_background,
            ThemeColorField::EditorSubheaderBackground => self.editor.subheader_background,
            ThemeColorField::EditorActiveLineBackground => self.editor.active_line_background,
            ThemeColorField::EditorHighlightedLineBackground => self.editor.highlighted_line_background,
            ThemeColorField::EditorDebuggerActiveLineBackground => self.editor.debugger_active_line_background,
            ThemeColorField::EditorLineNumber => self.editor.line_number,
            ThemeColorField::EditorActiveLineNumber => self.editor.active_line_number,
            ThemeColorField::EditorHoverLineNumber => self.editor.hover_line_number,
            ThemeColorField::EditorInvisible => self.editor.invisible,
            ThemeColorField::EditorWrapGuide => self.editor.wrap_guide,
            ThemeColorField::EditorActiveWrapGuide => self.editor.active_wrap_guide,
            ThemeColorField::EditorIndentGuide => self.editor.indent_guide,
            ThemeColorField::EditorIndentGuideActive => self.editor.indent_guide_active,
            ThemeColorField::EditorDocumentHighlightReadBackground => self.editor.document_highlight_read_background,
            ThemeColorField::EditorDocumentHighlightWriteBackground => self.editor.document_highlight_write_background,
            ThemeColorField::EditorDocumentHighlightBracketBackground => self.editor.document_highlight_bracket_background,
            // Terminal
            ThemeColorField::TerminalBackground => self.terminal.background,
            ThemeColorField::TerminalForeground => self.terminal.foreground,
            ThemeColorField::TerminalBrightForeground => self.terminal.bright_foreground,
            ThemeColorField::TerminalDimForeground => self.terminal.dim_foreground,
            ThemeColorField::TerminalAccent => self.terminal.accent,
            ThemeColorField::TerminalAnsiBackground => self.terminal.ansi.background,
            ThemeColorField::TerminalAnsiBlack => self.terminal.ansi.black,
            ThemeColorField::TerminalAnsiBrightBlack => self.terminal.ansi.bright_black,
            ThemeColorField::TerminalAnsiDimBlack => self.terminal.ansi.dim_black,
            ThemeColorField::TerminalAnsiRed => self.terminal.ansi.red,
            ThemeColorField::TerminalAnsiBrightRed => self.terminal.ansi.bright_red,
            ThemeColorField::TerminalAnsiDimRed => self.terminal.ansi.dim_red,
            ThemeColorField::TerminalAnsiGreen => self.terminal.ansi.green,
            ThemeColorField::TerminalAnsiBrightGreen => self.terminal.ansi.bright_green,
            ThemeColorField::TerminalAnsiDimGreen => self.terminal.ansi.dim_green,
            ThemeColorField::TerminalAnsiYellow => self.terminal.ansi.yellow,
            ThemeColorField::TerminalAnsiBrightYellow => self.terminal.ansi.bright_yellow,
            ThemeColorField::TerminalAnsiDimYellow => self.terminal.ansi.dim_yellow,
            ThemeColorField::TerminalAnsiBlue => self.terminal.ansi.blue,
            ThemeColorField::TerminalAnsiBrightBlue => self.terminal.ansi.bright_blue,
            ThemeColorField::TerminalAnsiDimBlue => self.terminal.ansi.dim_blue,
            ThemeColorField::TerminalAnsiMagenta => self.terminal.ansi.magenta,
            ThemeColorField::TerminalAnsiBrightMagenta => self.terminal.ansi.bright_magenta,
            ThemeColorField::TerminalAnsiDimMagenta => self.terminal.ansi.dim_magenta,
            ThemeColorField::TerminalAnsiCyan => self.terminal.ansi.cyan,
            ThemeColorField::TerminalAnsiBrightCyan => self.terminal.ansi.bright_cyan,
            ThemeColorField::TerminalAnsiDimCyan => self.terminal.ansi.dim_cyan,
            ThemeColorField::TerminalAnsiWhite => self.terminal.ansi.white,
            ThemeColorField::TerminalAnsiBrightWhite => self.terminal.ansi.bright_white,
            ThemeColorField::TerminalAnsiDimWhite => self.terminal.ansi.dim_white,
            // Panel
            ThemeColorField::PanelBackground => self.panel.background,
            ThemeColorField::PanelFocusedBorder => self.panel.focused_border,
            ThemeColorField::PanelIndentGuide => self.panel.indent_guide,
            ThemeColorField::PanelIndentGuideHover => self.panel.indent_guide_hover,
            ThemeColorField::PanelIndentGuideActive => self.panel.indent_guide_active,
            ThemeColorField::PanelOverlayBackground => self.panel.overlay_background,
            ThemeColorField::PanelOverlayHover => self.panel.overlay_hover,
            // Pane
            ThemeColorField::PaneFocusedBorder => self.pane.focused_border,
            ThemeColorField::PaneGroupBorder => self.pane.group_border,
            // Tab
            ThemeColorField::TabBarBackground => self.tab.bar_background,
            ThemeColorField::TabInactiveBackground => self.tab.inactive_background,
            ThemeColorField::TabActiveBackground => self.tab.active_background,
            ThemeColorField::TabInactiveForeground => self.tab.inactive_foreground,
            ThemeColorField::TabActiveForeground => self.tab.active_foreground,
            // Scrollbar
            ThemeColorField::ScrollbarThumbBackground => self.scrollbar.thumb_background,
            ThemeColorField::ScrollbarThumbHoverBackground => self.scrollbar.thumb_hover_background,
            ThemeColorField::ScrollbarThumbActiveBackground => self.scrollbar.thumb_active_background,
            ThemeColorField::ScrollbarThumbBorder => self.scrollbar.thumb_border,
            ThemeColorField::ScrollbarTrackBackground => self.scrollbar.track_background,
            ThemeColorField::ScrollbarTrackBorder => self.scrollbar.track_border,
            // Minimap
            ThemeColorField::MinimapThumbBackground => self.minimap.thumb_background,
            ThemeColorField::MinimapThumbHoverBackground => self.minimap.thumb_hover_background,
            ThemeColorField::MinimapThumbActiveBackground => self.minimap.thumb_active_background,
            ThemeColorField::MinimapThumbBorder => self.minimap.thumb_border,
            // Status bar
            ThemeColorField::StatusBarBackground => self.status_bar.background,
            // Title bar
            ThemeColorField::TitleBarBackground => self.title_bar.background,
            ThemeColorField::TitleBarInactiveBackground => self.title_bar.inactive_background,
            // Toolbar
            ThemeColorField::ToolbarBackground => self.toolbar.background,
            // Search
            ThemeColorField::SearchMatchBackground => self.search.match_background,
            ThemeColorField::SearchActiveMatchBackground => self.search.active_match_background,
            // Vim
            ThemeColorField::VimNormalBackground => self.vim.normal_background,
            ThemeColorField::VimInsertBackground => self.vim.insert_background,
            ThemeColorField::VimReplaceBackground => self.vim.replace_background,
            ThemeColorField::VimVisualBackground => self.vim.visual_background,
            ThemeColorField::VimVisualLineBackground => self.vim.visual_line_background,
            ThemeColorField::VimVisualBlockBackground => self.vim.visual_block_background,
            ThemeColorField::VimYankBackground => self.vim.yank_background,
            ThemeColorField::VimHelixNormalBackground => self.vim.helix_normal_background,
            ThemeColorField::VimHelixSelectBackground => self.vim.helix_select_background,
            ThemeColorField::VimNormalForeground => self.vim.normal_foreground,
            ThemeColorField::VimInsertForeground => self.vim.insert_foreground,
            ThemeColorField::VimReplaceForeground => self.vim.replace_foreground,
            ThemeColorField::VimVisualForeground => self.vim.visual_foreground,
            ThemeColorField::VimVisualLineForeground => self.vim.visual_line_foreground,
            ThemeColorField::VimVisualBlockForeground => self.vim.visual_block_foreground,
            ThemeColorField::VimHelixNormalForeground => self.vim.helix_normal_foreground,
            ThemeColorField::VimHelixSelectForeground => self.vim.helix_select_foreground,
            // Version control
            ThemeColorField::VersionControlAdded => self.version_control.added,
            ThemeColorField::VersionControlDeleted => self.version_control.deleted,
            ThemeColorField::VersionControlModified => self.version_control.modified,
            ThemeColorField::VersionControlRenamed => self.version_control.renamed,
            ThemeColorField::VersionControlConflict => self.version_control.conflict,
            ThemeColorField::VersionControlIgnored => self.version_control.ignored,
            ThemeColorField::VersionControlWordAdded => self.version_control.word_added,
            ThemeColorField::VersionControlWordDeleted => self.version_control.word_deleted,
            ThemeColorField::VersionControlConflictMarkerOurs => self.version_control.conflict_marker_ours,
            ThemeColorField::VersionControlConflictMarkerTheirs => self.version_control.conflict_marker_theirs,
            // Block
            ThemeColorField::BlockSuccessBadge => self.block.success_badge,
            ThemeColorField::BlockErrorBadge => self.block.error_badge,
            ThemeColorField::BlockRunningBadge => self.block.running_badge,
            // Chart
            ThemeColorField::Chart1 => self.chart.chart_1,
            ThemeColorField::Chart2 => self.chart.chart_2,
            ThemeColorField::Chart3 => self.chart.chart_3,
            ThemeColorField::Chart4 => self.chart.chart_4,
            ThemeColorField::Chart5 => self.chart.chart_5,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (ThemeColorField, Oklch)> + '_ {
        ThemeColorField::iter().map(move |field| (field, self.color(field)))
    }

    pub fn to_vec(&self) -> Vec<(ThemeColorField, Oklch)> {
        self.iter().collect()
    }
}

pub fn all_theme_colors(cx: &mut App) -> Vec<(Oklch, SharedString)> {
    let theme = cx.theme();
    ThemeColorField::iter()
        .map(|field| {
            let color = theme.colors().color(field);
            let name = field.as_ref().to_string();
            (color, SharedString::from(name))
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// ThemeStyles (kept here alongside ThemeColors for derive macro proximity)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Refineable, Clone, Debug, PartialEq)]
pub struct ThemeStyles {
    /// The background appearance of the window.
    pub window_background_appearance: WindowBackgroundAppearance,
    pub system: SystemColors,
    /// An array of colors used for theme elements that iterate through a series of colors.
    pub accents: AccentColors,

    #[refineable]
    pub colors: ThemeColors,

    #[refineable]
    pub status: StatusColors,

    pub player: PlayerColors,

    pub syntax: Arc<SyntaxTheme>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn override_a_single_theme_color() {
        let mut colors = ThemeColors::light();

        let magenta: Oklch = inazuma::rgb(0xff00ff).into();

        assert_ne!(colors.text, magenta);

        let overrides = ThemeColorsRefinement {
            text: Some(magenta),
            ..Default::default()
        };

        colors.refine(&overrides);

        assert_eq!(colors.text, magenta);
    }

    #[test]
    fn override_multiple_theme_colors() {
        let mut colors = ThemeColors::light();

        let magenta: Oklch = inazuma::rgb(0xff00ff).into();
        let green: Oklch = inazuma::rgb(0x00ff00).into();

        assert_ne!(colors.text, magenta);
        assert_ne!(colors.background, green);

        let overrides = ThemeColorsRefinement {
            text: Some(magenta),
            background: Some(green),
            ..Default::default()
        };

        colors.refine(&overrides);

        assert_eq!(colors.text, magenta);
        assert_eq!(colors.background, green);
    }

    #[test]
    fn deserialize_theme_colors_refinement_from_json() {
        let colors: ThemeColorsRefinement = serde_json::from_value(json!({
            "background": "#ff00ff",
            "text": "#ff0000"
        }))
        .unwrap();

        assert_eq!(colors.background, Some(inazuma::rgb(0xff00ff).into()));
        assert_eq!(colors.text, Some(inazuma::rgb(0xff0000).into()));
    }
}
