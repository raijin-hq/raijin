use inazuma::{App, Edges, Oklch, Pixels, px};
use raijin_theme::ActiveTheme;

use crate::traits::size::Size;

/// Tab visual variants.
#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, Hash)]
pub enum TabVariant {
    /// Workspace tab with position-awareness and close-side borders.
    Workspace,
    #[default]
    /// Default tab style with left/right borders.
    Tab,
    /// Outlined tab with full border.
    Outline,
    /// Pill-shaped tab with rounded background.
    Pill,
    /// Segmented control style with inner background and shadow.
    Segmented,
    /// Underlined tab with bottom border indicator.
    Underline,
}

pub(super) struct TabStyle {
    pub borders: Edges<Pixels>,
    pub border_color: Oklch,
    pub bg: Oklch,
    pub fg: Oklch,
    pub shadow: bool,
    pub inner_bg: Oklch,
}

impl Default for TabStyle {
    fn default() -> Self {
        TabStyle {
            borders: Edges::all(px(0.)),
            border_color: inazuma::transparent_white(),
            bg: inazuma::transparent_white(),
            fg: inazuma::transparent_white(),
            shadow: false,
            inner_bg: inazuma::transparent_white(),
        }
    }
}

impl TabVariant {
    pub(super) fn height(&self, size: Size) -> Pixels {
        match size {
            Size::XSmall => match self {
                TabVariant::Underline => px(26.),
                _ => px(20.),
            },
            Size::Small => match self {
                TabVariant::Underline => px(30.),
                _ => px(24.),
            },
            Size::Large => match self {
                TabVariant::Underline => px(44.),
                _ => px(36.),
            },
            _ => match self {
                TabVariant::Underline => px(36.),
                _ => px(32.),
            },
        }
    }

    pub(super) fn inner_height(&self, size: Size) -> Pixels {
        match size {
            Size::XSmall => match self {
                TabVariant::Tab | TabVariant::Workspace | TabVariant::Outline | TabVariant::Pill => px(18.),
                TabVariant::Segmented => px(16.),
                TabVariant::Underline => px(20.),
            },
            Size::Small => match self {
                TabVariant::Tab | TabVariant::Workspace | TabVariant::Outline | TabVariant::Pill => px(22.),
                TabVariant::Segmented => px(18.),
                TabVariant::Underline => px(22.),
            },
            Size::Large => match self {
                TabVariant::Tab | TabVariant::Workspace | TabVariant::Outline | TabVariant::Pill => px(36.),
                TabVariant::Segmented => px(28.),
                TabVariant::Underline => px(32.),
            },
            _ => match self {
                TabVariant::Tab | TabVariant::Workspace => px(30.),
                TabVariant::Outline | TabVariant::Pill => px(26.),
                TabVariant::Segmented => px(24.),
                TabVariant::Underline => px(26.),
            },
        }
    }

    pub(super) fn inner_paddings(&self, size: Size) -> Edges<Pixels> {
        let mut padding_x = match size {
            Size::XSmall => px(8.),
            Size::Small => px(10.),
            Size::Large => px(16.),
            _ => px(12.),
        };

        if matches!(self, TabVariant::Underline) {
            padding_x = px(0.);
        }

        Edges {
            left: padding_x,
            right: padding_x,
            ..Default::default()
        }
    }

    pub(super) fn inner_margins(&self, size: Size) -> Edges<Pixels> {
        if !matches!(self, TabVariant::Underline) {
            return Edges::all(px(0.));
        }
        match size {
            Size::XSmall => Edges { top: px(1.), bottom: px(2.), ..Default::default() },
            Size::Small => Edges { top: px(2.), bottom: px(3.), ..Default::default() },
            Size::Large => Edges { top: px(5.), bottom: px(6.), ..Default::default() },
            _ => Edges { top: px(3.), bottom: px(4.), ..Default::default() },
        }
    }

    pub(super) fn normal(&self, cx: &App) -> TabStyle {
        let colors = cx.theme().colors();
        let transparent = inazuma::transparent_white();
        match self {
            TabVariant::Tab | TabVariant::Workspace => TabStyle {
                fg: colors.tab.inactive_foreground,
                bg: transparent,
                borders: Edges { left: px(1.), right: px(1.), ..Default::default() },
                border_color: transparent,
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: colors.tab.inactive_foreground,
                bg: transparent,
                borders: Edges::all(px(1.)),
                border_color: colors.border,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: colors.text,
                bg: transparent,
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: colors.tab.inactive_foreground,
                bg: transparent,
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: colors.tab.inactive_foreground,
                bg: transparent,
                inner_bg: transparent,
                borders: Edges { bottom: px(2.), ..Default::default() },
                border_color: transparent,
                ..Default::default()
            },
        }
    }

    pub(super) fn hovered(&self, selected: bool, cx: &App) -> TabStyle {
        let colors = cx.theme().colors();
        let transparent = inazuma::transparent_white();
        match self {
            TabVariant::Tab | TabVariant::Workspace => TabStyle {
                fg: colors.tab.inactive_foreground,
                bg: transparent,
                borders: Edges { left: px(1.), right: px(1.), ..Default::default() },
                border_color: transparent,
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: colors.text,
                bg: colors.element_hover,
                borders: Edges::all(px(1.)),
                border_color: colors.border,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: colors.text,
                bg: colors.element_hover,
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: colors.tab.inactive_foreground,
                bg: transparent,
                inner_bg: if selected { colors.background } else { transparent },
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: colors.tab.inactive_foreground,
                bg: transparent,
                inner_bg: transparent,
                borders: Edges { bottom: px(2.), ..Default::default() },
                border_color: transparent,
                ..Default::default()
            },
        }
    }

    pub(super) fn selected(&self, cx: &App) -> TabStyle {
        let colors = cx.theme().colors();
        let transparent = inazuma::transparent_white();
        match self {
            TabVariant::Tab | TabVariant::Workspace => TabStyle {
                fg: colors.tab.active_foreground,
                bg: colors.tab.active_background,
                borders: Edges { left: px(1.), right: px(1.), ..Default::default() },
                border_color: colors.border,
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: colors.text_accent,
                bg: transparent,
                borders: Edges::all(px(1.)),
                border_color: colors.text_accent,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: colors.background,
                bg: colors.text_accent,
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: colors.tab.active_foreground,
                bg: transparent,
                inner_bg: colors.background,
                shadow: true,
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: colors.tab.active_foreground,
                bg: transparent,
                borders: Edges { bottom: px(2.), ..Default::default() },
                border_color: colors.text_accent,
                ..Default::default()
            },
        }
    }

    pub(super) fn disabled(&self, selected: bool, cx: &App) -> TabStyle {
        let colors = cx.theme().colors();
        let transparent = inazuma::transparent_white();
        match self {
            TabVariant::Tab | TabVariant::Workspace => TabStyle {
                fg: colors.text_muted,
                bg: transparent,
                border_color: if selected { colors.border } else { transparent },
                borders: Edges { left: px(1.), right: px(1.), ..Default::default() },
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: colors.text_muted,
                bg: transparent,
                borders: Edges::all(px(1.)),
                border_color: if selected { colors.text_accent } else { colors.border },
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: if selected { colors.text_muted } else { colors.text_muted },
                bg: if selected { colors.text_accent.opacity(0.5) } else { transparent },
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: colors.text_muted,
                bg: colors.tab.bar_background,
                inner_bg: if selected { colors.background } else { transparent },
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: colors.text_muted,
                bg: transparent,
                border_color: if selected { colors.border } else { transparent },
                borders: Edges { bottom: px(2.), ..Default::default() },
                ..Default::default()
            },
        }
    }

    pub(super) fn tab_bar_radius(&self, size: Size, _cx: &App) -> Pixels {
        if *self != TabVariant::Segmented {
            return px(0.);
        }
        // Use a reasonable default radius
        match size {
            Size::XSmall | Size::Small => px(6.),
            _ => px(8.),
        }
    }

    pub(super) fn radius(&self, size: Size, cx: &App) -> Pixels {
        match self {
            TabVariant::Outline | TabVariant::Pill => px(99.),
            TabVariant::Segmented => self.tab_bar_radius(size, cx),
            _ => px(0.),
        }
    }

    pub(super) fn inner_radius(&self, size: Size, cx: &App) -> Pixels {
        match self {
            TabVariant::Segmented => match size {
                Size::Large => self.tab_bar_radius(size, cx) - px(3.),
                _ => self.tab_bar_radius(size, cx) - px(2.),
            },
            _ => px(0.),
        }
    }
}
