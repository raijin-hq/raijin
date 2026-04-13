use crate::prelude::*;
use inazuma::{AnyElement, AnyView, CursorStyle, Oklch, IntoElement, ParentElement, Styled};

/// A compact context chip with optional icon, used in the terminal prompt area.
///
/// Warp-style chip design: semi-transparent background, subtle border, colored text.
/// All base styling comes from theme tokens (`cx.theme().colors().chip.*`).
/// Per-chip accent colors can be overridden via `.color()` or `.bg_color()`.
///
/// # Usage Example
///
/// ```
/// use raijin_ui::Chip;
///
/// // Default styling from theme:
/// let chip = Chip::new("nyxb");
///
/// // With custom text color:
/// let chip = Chip::new("nyxb").color(some_oklch);
/// ```
#[derive(IntoElement, RegisterComponent)]
pub struct Chip {
    label: SharedString,
    color: Option<Oklch>,
    bg_color: Option<Oklch>,
    border_color: Option<Oklch>,
    label_size: LabelSize,
    label_color: Color,
    height: Option<Pixels>,
    icon: Option<IconName>,
    icon_color: Option<Oklch>,
    selected: bool,
    interactive: bool,
    tooltip: Option<Box<dyn Fn(&mut Window, &mut App) -> AnyView + 'static>>,
}

impl Chip {
    /// Creates a new `Chip` with theme-default styling.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            color: None,
            bg_color: None,
            border_color: None,
            label_size: LabelSize::XSmall,
            label_color: Color::Default,
            height: None,
            icon: None,
            icon_color: None,
            selected: false,
            interactive: false,
            tooltip: None,
        }
    }

    /// Sets the text color for the chip (overrides theme default).
    pub fn color(mut self, color: impl Into<Oklch>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets a custom background color (overrides theme chip.background).
    pub fn bg_color(mut self, color: Oklch) -> Self {
        self.bg_color = Some(color);
        self
    }

    /// Sets a custom border color (overrides theme chip.border).
    pub fn border_color(mut self, color: Oklch) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Sets the color of the label via the Color enum.
    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = color;
        self
    }

    /// Sets the size of the label.
    pub fn label_size(mut self, size: LabelSize) -> Self {
        self.label_size = size;
        self
    }

    /// Sets a custom height for the chip.
    pub fn height(mut self, height: Pixels) -> Self {
        self.height = Some(height);
        self
    }

    /// Adds an icon prefix to the chip.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Adds an icon prefix with a custom color.
    pub fn icon_colored(mut self, icon: IconName, color: Oklch) -> Self {
        self.icon = Some(icon);
        self.icon_color = Some(color);
        self
    }

    /// Marks this chip as selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Marks this chip as interactive (shows pointer cursor on hover).
    pub fn interactive(mut self) -> Self {
        self.interactive = true;
        self
    }

    /// Sets the tooltip for the chip.
    pub fn tooltip(mut self, tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self {
        self.tooltip = Some(Box::new(tooltip));
        self
    }
}

impl RenderOnce for Chip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let chip_colors = &cx.theme().colors().chip;

        let bg = self.bg_color.unwrap_or(chip_colors.background);
        let border = self.border_color.unwrap_or(chip_colors.border);
        let text_color = self.color;
        let label_color = match self.color {
            Some(c) => Color::Custom(c),
            None => self.label_color,
        };
        let hover_bg = chip_colors.hover;

        h_flex()
            .when_some(self.height, |this, h| this.h(h))
            .flex_none()
            .gap(px(4.0))
            .items_center()
            .px(px(6.0))
            .py(px(2.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_xs()
            .when_some(text_color, |this, c| this.text_color(c))
            .overflow_hidden()
            .when(self.interactive, |this| {
                this.cursor(CursorStyle::PointingHand)
                    .hover(move |this| this.bg(hover_bg))
            })
            .when(self.selected, |this| {
                this.bg(cx.theme().colors().element_active)
            })
            .when_some(self.icon, |this, icon| {
                let icon_color = self.icon_color.or(text_color);
                this.child(
                    Icon::new(icon)
                        .size(IconSize::XSmall)
                        .when_some(icon_color, |this, c| this.color(Color::Custom(c))),
                )
            })
            .child(
                Label::new(self.label.clone())
                    .size(self.label_size)
                    .color(label_color)
                    .buffer_font(cx)
                    .truncate(),
            )
            .id(self.label.clone())
            .when_some(self.tooltip, |this, tooltip| this.tooltip(tooltip))
    }
}

/// Git branch chip: icon and branch name with distinct colors from theme.
#[derive(IntoElement)]
pub struct GitBranchChip {
    branch: SharedString,
}

impl GitBranchChip {
    pub fn new(branch: impl Into<SharedString>) -> Self {
        Self {
            branch: branch.into(),
        }
    }
}

impl RenderOnce for GitBranchChip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let chip = &cx.theme().colors().chip;

        h_flex()
            .gap(px(4.0))
            .items_center()
            .px(px(6.0))
            .py(px(2.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(chip.border)
            .bg(chip.background)
            .text_xs()
            .child(
                Icon::new(IconName::GitBranch)
                    .size(IconSize::XSmall)
                    .color(Color::Custom(chip.git_branch_icon)),
            )
            .child(div().text_color(chip.git_branch_text).child(self.branch.to_string()))
    }
}

/// Git stats chip showing file count, insertions, and deletions in theme colors.
#[derive(IntoElement)]
pub struct GitStatsChip {
    files_changed: u32,
    insertions: u32,
    deletions: u32,
}

impl GitStatsChip {
    pub fn new(files_changed: u32, insertions: u32, deletions: u32) -> Self {
        Self {
            files_changed,
            insertions,
            deletions,
        }
    }
}

impl RenderOnce for GitStatsChip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let chip = &cx.theme().colors().chip;

        h_flex()
            .gap(px(4.0))
            .items_center()
            .px(px(6.0))
            .py(px(2.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(chip.border)
            .bg(chip.background)
            .text_xs()
            .child(
                Icon::new(IconName::FileDiff)
                    .size(IconSize::XSmall)
                    .color(Color::Custom(chip.git_stats_neutral)),
            )
            .child(
                div()
                    .text_color(chip.git_stats_neutral)
                    .child(format!("{} \u{00b7} ", self.files_changed)),
            )
            .child(
                div()
                    .text_color(chip.git_stats_insert)
                    .child(format!("+{}", self.insertions)),
            )
            .child(
                div()
                    .text_color(chip.git_stats_delete)
                    .child(format!("-{}", self.deletions)),
            )
    }
}

impl Component for Chip {
    fn scope() -> ComponentScope {
        ComponentScope::DataDisplay
    }

    fn preview(_window: &mut Window, _cx: &mut App) -> Option<AnyElement> {
        let chip_examples = vec![
            single_example(
                "Default",
                Chip::new("Chip Example").into_any_element(),
            ),
            single_example(
                "With Color",
                Chip::new("nyxb")
                    .label_color(Color::Accent)
                    .into_any_element(),
            ),
            single_example(
                "With Icon",
                Chip::new("Feature")
                    .icon(IconName::Sparkle)
                    .label_color(Color::Accent)
                    .into_any_element(),
            ),
            single_example(
                "Interactive",
                Chip::new("Clickable")
                    .icon(IconName::Settings)
                    .interactive()
                    .into_any_element(),
            ),
            single_example(
                "Git Branch",
                GitBranchChip::new("main").into_any_element(),
            ),
            single_example(
                "Git Stats",
                GitStatsChip::new(5, 42, 13).into_any_element(),
            ),
        ];

        Some(example_group(chip_examples).vertical().into_any_element())
    }
}
