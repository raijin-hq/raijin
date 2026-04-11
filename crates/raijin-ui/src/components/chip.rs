use crate::prelude::*;
use inazuma::{AnyElement, AnyView, CursorStyle, Oklch, IntoElement, ParentElement, Styled, rgb};

/// Chips provide a container for an informative label with optional icon.
///
/// Supports both general-purpose info chips and interactive context chips
/// with icon prefixes, selection state, and hover effects.
///
/// # Usage Example
///
/// ```
/// use raijin_ui::Chip;
///
/// let chip = Chip::new("This Chip");
/// ```
#[derive(IntoElement, RegisterComponent)]
pub struct Chip {
    label: SharedString,
    label_color: Color,
    label_size: LabelSize,
    bg_color: Option<Oklch>,
    border_color: Option<Oklch>,
    height: Option<Pixels>,
    icon: Option<IconName>,
    icon_color: Option<Oklch>,
    selected: bool,
    interactive: bool,
    tooltip: Option<Box<dyn Fn(&mut Window, &mut App) -> AnyView + 'static>>,
}

impl Chip {
    /// Creates a new `Chip` component with the specified label.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            label_color: Color::Default,
            label_size: LabelSize::XSmall,
            bg_color: None,
            border_color: None,
            height: None,
            icon: None,
            icon_color: None,
            selected: false,
            interactive: false,
            tooltip: None,
        }
    }

    /// Sets the color of the label.
    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = color;
        self
    }

    /// Sets the size of the label.
    pub fn label_size(mut self, size: LabelSize) -> Self {
        self.label_size = size;
        self
    }

    /// Sets a custom background color for the chip.
    pub fn bg_color(mut self, color: Oklch) -> Self {
        self.bg_color = Some(color);
        self
    }

    /// Sets a custom border color for the chip.
    pub fn border_color(mut self, color: Oklch) -> Self {
        self.border_color = Some(color);
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
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let bg_color = self
            .bg_color
            .unwrap_or(cx.theme().colors().element_background);

        let border_color = self.border_color.unwrap_or(cx.theme().colors().border);

        h_flex()
            .when_some(self.height, |this, h| this.h(h))
            .flex_none()
            .gap(px(4.))
            .items_center()
            .px_1()
            .border_1()
            .rounded_sm()
            .border_color(border_color)
            .bg(bg_color)
            .overflow_hidden()
            .when(self.interactive, |this| {
                this.cursor(CursorStyle::PointingHand)
                    .hover(|this| this.bg(cx.theme().colors().element_hover))
            })
            .when(self.selected, |this| {
                this.bg(cx.theme().colors().element_active)
            })
            .when_some(self.icon, |this, icon| {
                let icon_color = self.icon_color;
                this.child(
                    Icon::new(icon)
                        .size(IconSize::XSmall)
                        .when_some(icon_color, |this, c| this.color(Color::Custom(c))),
                )
            })
            .child(
                Label::new(self.label.clone())
                    .size(self.label_size)
                    .color(self.label_color)
                    .buffer_font(cx)
                    .truncate(),
            )
            .id(self.label.clone())
            .when_some(self.tooltip, |this, tooltip| this.tooltip(tooltip))
    }
}

/// Git branch chip: icon and branch name with distinct colors.
#[derive(IntoElement)]
pub struct GitBranchChip {
    branch: SharedString,
    icon_color: Oklch,
    text_color: Oklch,
}

impl GitBranchChip {
    pub fn new(branch: impl Into<SharedString>) -> Self {
        Self {
            branch: branch.into(),
            icon_color: rgb(0x6ee7b7).into(),
            text_color: rgb(0x7dd3fc).into(),
        }
    }

    pub fn icon_color(mut self, color: Oklch) -> Self {
        self.icon_color = color;
        self
    }

    pub fn text_color(mut self, color: Oklch) -> Self {
        self.text_color = color;
        self
    }
}

impl RenderOnce for GitBranchChip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .gap(px(4.0))
            .items_center()
            .px(px(6.0))
            .py(px(2.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(cx.theme().colors().border)
            .bg(cx.theme().colors().element_background)
            .text_xs()
            .child(
                Icon::new(IconName::GitBranch)
                    .size(IconSize::XSmall)
                    .color(Color::Custom(self.icon_color)),
            )
            .child(div().text_color(self.text_color).child(self.branch.to_string()))
    }
}

/// Git stats chip showing file count, insertions, and deletions in different colors.
#[derive(IntoElement)]
pub struct GitStatsChip {
    files_changed: u32,
    insertions: u32,
    deletions: u32,
    neutral_color: Oklch,
    insert_color: Oklch,
    delete_color: Oklch,
}

impl GitStatsChip {
    pub fn new(files_changed: u32, insertions: u32, deletions: u32) -> Self {
        Self {
            files_changed,
            insertions,
            deletions,
            neutral_color: rgb(0xc8c8c8).into(),
            insert_color: rgb(0x00BFFF).into(),
            delete_color: rgb(0xff5f5f).into(),
        }
    }
}

impl RenderOnce for GitStatsChip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .gap(px(4.0))
            .items_center()
            .px(px(6.0))
            .py(px(2.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(cx.theme().colors().border)
            .bg(cx.theme().colors().element_background)
            .text_xs()
            .child(
                Icon::new(IconName::FileDiff)
                    .size(IconSize::XSmall)
                    .color(Color::Custom(self.neutral_color)),
            )
            .child(
                div()
                    .text_color(self.neutral_color)
                    .child(format!("{} \u{00b7} ", self.files_changed)),
            )
            .child(
                div()
                    .text_color(self.insert_color)
                    .child(format!("+{}", self.insertions)),
            )
            .child(
                div()
                    .text_color(self.delete_color)
                    .child(format!("-{}", self.deletions)),
            )
    }
}

impl Component for Chip {
    fn scope() -> ComponentScope {
        ComponentScope::DataDisplay
    }

    fn preview(_window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let chip_examples = vec![
            single_example("Default", Chip::new("Chip Example").into_any_element()),
            single_example(
                "Customized Label Color",
                Chip::new("Chip Example")
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
