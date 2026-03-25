use inazuma::{
    App, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, Styled, StyleRefinement,
    Window, div, hsla, prelude::FluentBuilder as _, px, rgb,
};

use crate::{Icon, IconName, Sizable, h_flex};

// ---------------------------------------------------------------------------
// Chip base styling constants
// ---------------------------------------------------------------------------

const CHIP_BORDER_OPACITY: f32 = 0.08;
const CHIP_BG_OPACITY: f32 = 0.03;

fn chip_base_style() -> (Hsla, Hsla) {
    (
        hsla(0.0, 0.0, 1.0, CHIP_BORDER_OPACITY),
        hsla(0.0, 0.0, 1.0, CHIP_BG_OPACITY),
    )
}

// ---------------------------------------------------------------------------
// Chip — simple text chip
// ---------------------------------------------------------------------------

/// A compact context chip with optional icon, used in the terminal prompt area.
///
/// Matches the Warp-style chip design: subtle border, faint transparent background,
/// small text, optional SVG icon prefix.
#[derive(IntoElement)]
pub struct Chip {
    label: SharedString,
    color: Hsla,
    icon: Option<(IconName, Option<Hsla>)>,
    style: StyleRefinement,
}

impl Chip {
    /// Create a text-only chip.
    pub fn new(label: impl Into<SharedString>, color: Hsla) -> Self {
        Self {
            label: label.into(),
            color,
            icon: None,
            style: StyleRefinement::default(),
        }
    }

    /// Add an icon prefix with the same color as the label.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some((icon, None));
        self
    }

    /// Add an icon prefix with a different color than the label.
    pub fn icon_colored(mut self, icon: IconName, icon_color: Hsla) -> Self {
        self.icon = Some((icon, Some(icon_color)));
        self
    }
}

impl Styled for Chip {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Chip {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (border, bg) = chip_base_style();
        let icon_color = self
            .icon
            .as_ref()
            .and_then(|(_, c)| *c)
            .unwrap_or(self.color);

        h_flex()
            .gap(px(4.0))
            .items_center()
            .px(px(6.0))
            .py(px(2.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_xs()
            .text_color(self.color)
            .when_some(self.icon, |this, (icon, _)| {
                this.child(Icon::new(icon).small().text_color(icon_color))
            })
            .child(self.label.to_string())
    }
}

// ---------------------------------------------------------------------------
// GitBranchChip — icon and label have different colors
// ---------------------------------------------------------------------------

/// Git branch chip: mint icon, cyan branch name.
#[derive(IntoElement)]
pub struct GitBranchChip {
    branch: SharedString,
    icon_color: Hsla,
    text_color: Hsla,
}

impl GitBranchChip {
    pub fn new(branch: impl Into<SharedString>) -> Self {
        Self {
            branch: branch.into(),
            icon_color: rgb(0x6ee7b7).into(),
            text_color: rgb(0x7dd3fc).into(),
        }
    }

    pub fn icon_color(mut self, color: Hsla) -> Self {
        self.icon_color = color;
        self
    }

    pub fn text_color(mut self, color: Hsla) -> Self {
        self.text_color = color;
        self
    }
}

impl RenderOnce for GitBranchChip {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (border, bg) = chip_base_style();

        h_flex()
            .gap(px(4.0))
            .items_center()
            .px(px(6.0))
            .py(px(2.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_xs()
            .child(Icon::new(IconName::GitBranch).small().text_color(self.icon_color))
            .child(div().text_color(self.text_color).child(self.branch.to_string()))
    }
}

// ---------------------------------------------------------------------------
// GitStatsChip — multi-colored: count (neutral), +N (green), -N (red)
// ---------------------------------------------------------------------------

/// Git stats chip showing file count, insertions, and deletions in different colors.
#[derive(IntoElement)]
pub struct GitStatsChip {
    files_changed: u32,
    insertions: u32,
    deletions: u32,
    neutral_color: Hsla,
    insert_color: Hsla,
    delete_color: Hsla,
}

impl GitStatsChip {
    pub fn new(files_changed: u32, insertions: u32, deletions: u32) -> Self {
        Self {
            files_changed,
            insertions,
            deletions,
            neutral_color: rgb(0xc8c8c8).into(),
            insert_color: rgb(0x14F195).into(),
            delete_color: rgb(0xff5f5f).into(),
        }
    }
}

impl RenderOnce for GitStatsChip {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (border, bg) = chip_base_style();

        h_flex()
            .gap(px(4.0))
            .items_center()
            .px(px(6.0))
            .py(px(2.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_xs()
            .child(Icon::new(IconName::FileDiff).small().text_color(self.neutral_color))
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
