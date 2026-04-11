use crate::ActiveTheme;
use inazuma::{App, Oklch, transparent_white};

use super::button_variant::ButtonVariantStyle;
use super::ButtonVariant;

impl ButtonVariant {
    fn bg_color(&self, outline: bool, cx: &mut App) -> Oklch {
        if outline {
            return cx.theme().colors().input;
        }

        match self {
            Self::Default => cx.theme().colors().input,
            Self::Primary => cx.theme().colors().primary,
            Self::Secondary => cx.theme().colors().secondary,
            Self::Outlined => Oklch::transparent_black(),
            Self::Danger => cx.theme().status().error.color.mix_oklab(Oklch::transparent_black(), 0.2),
            Self::Warning => cx.theme().status().warning.color.mix_oklab(Oklch::transparent_black(), 0.2),
            Self::Success => cx.theme().status().success.color.mix_oklab(Oklch::transparent_black(), 0.2),
            Self::Info => cx.theme().status().info.color.mix_oklab(Oklch::transparent_black(), 0.2),
            Self::Ghost | Self::Link | Self::Text => Oklch::transparent_black(),
            Self::Custom(colors) => colors.color.mix_oklab(Oklch::transparent_black(), 0.2),
        }
    }

    fn variant_text_color(&self, outline: bool, cx: &mut App) -> Oklch {
        match self {
            Self::Default => cx.theme().colors().foreground,
            Self::Primary => {
                if outline {
                    cx.theme().colors().primary
                } else {
                    cx.theme().colors().primary_foreground
                }
            }
            Self::Secondary | Self::Ghost | Self::Outlined => cx.theme().colors().secondary_foreground,
            Self::Danger => cx.theme().status().error.color,
            Self::Warning => cx.theme().status().warning.color,
            Self::Success => cx.theme().status().success.color,
            Self::Info => cx.theme().status().info.color,
            Self::Link => cx.theme().colors().accent,
            Self::Text => cx.theme().colors().foreground,
            Self::Custom(colors) => colors.color,
        }
    }

    fn variant_border_color(&self, _bg: Oklch, outline: bool, cx: &mut App) -> Oklch {
        match self {
            Self::Default => cx.theme().colors().input,
            Self::Secondary | Self::Outlined => cx.theme().colors().border,
            Self::Primary => cx.theme().colors().primary,
            Self::Danger => {
                if outline {
                    cx.theme()
                        .status().error.color
                        .mix_oklab(Oklch::from(transparent_white()), 0.4)
                } else {
                    cx.theme().status().error.color
                }
            }
            Self::Info => {
                if outline {
                    cx.theme()
                        .status().info.color
                        .mix_oklab(Oklch::from(transparent_white()), 0.4)
                } else {
                    cx.theme().status().info.color
                }
            }
            Self::Warning => {
                if outline {
                    cx.theme()
                        .status().warning.color
                        .mix_oklab(Oklch::from(transparent_white()), 0.4)
                } else {
                    cx.theme().status().warning.color
                }
            }
            Self::Success => {
                if outline {
                    cx.theme()
                        .status().success.color
                        .mix_oklab(Oklch::from(transparent_white()), 0.4)
                } else {
                    cx.theme().status().success.color
                }
            }
            Self::Ghost | Self::Link | Self::Text => Oklch::transparent_black(),
            Self::Custom(colors) => {
                if outline {
                    colors
                        .color
                        .mix_oklab(Oklch::from(transparent_white()), 0.4)
                } else {
                    colors.color
                }
            }
        }
    }

    fn variant_underline(&self) -> bool {
        matches!(self, Self::Link)
    }

    fn variant_shadow(&self, outline: bool) -> bool {
        match self {
            Self::Default => true,
            Self::Primary | Self::Secondary | Self::Danger => outline,
            Self::Custom(c) => c.shadow,
            _ => false,
        }
    }

    pub(crate) fn normal(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        let bg = self.bg_color(outline, cx);
        let border = self.variant_border_color(bg, outline, cx);
        let fg = self.variant_text_color(outline, cx);

        ButtonVariantStyle {
            bg,
            border,
            fg,
            underline: self.variant_underline(),
            shadow: self.variant_shadow(outline),
        }
    }

    pub(crate) fn hovered(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        let bg = match self {
            Self::Default => {
                cx.theme()
                    .colors().input
                    .mix_oklab(Oklch::transparent_black(), 0.5)
            }
            Self::Primary => {
                if outline {
                    cx.theme()
                        .colors().primary
                        .mix_oklab(Oklch::transparent_black(), 0.2)
                } else {
                    cx.theme().colors().primary.opacity(0.85)
                }
            }
            Self::Secondary | Self::Outlined => cx.theme().colors().element_hover,
            Self::Danger => {
                if outline {
                    cx.theme()
                        .status().error.color
                        .mix_oklab(Oklch::transparent_black(), 0.2)
                } else {
                    cx.theme()
                        .status().error.color
                        .mix_oklab(Oklch::transparent_black(), 0.3)
                }
            }
            Self::Warning => {
                if outline {
                    cx.theme()
                        .status().warning.color
                        .mix_oklab(Oklch::transparent_black(), 0.2)
                } else {
                    cx.theme()
                        .status().warning.color
                        .mix_oklab(Oklch::transparent_black(), 0.3)
                }
            }
            Self::Success => {
                if outline {
                    cx.theme()
                        .status().success.color
                        .mix_oklab(Oklch::transparent_black(), 0.2)
                } else {
                    cx.theme()
                        .status().success.color
                        .mix_oklab(Oklch::transparent_black(), 0.3)
                }
            }
            Self::Info => {
                if outline {
                    cx.theme()
                        .status().info.color
                        .mix_oklab(Oklch::transparent_black(), 0.2)
                } else {
                    cx.theme()
                        .status().info.color
                        .mix_oklab(Oklch::transparent_black(), 0.3)
                }
            }
            Self::Custom(colors) => {
                if outline {
                    colors.color.mix_oklab(Oklch::transparent_black(), 0.2)
                } else {
                    colors.color.mix_oklab(Oklch::transparent_black(), 0.3)
                }
            }
            Self::Ghost => {
                if cx.theme().is_dark() {
                    cx.theme().colors().secondary.lighten(0.1).opacity(0.8)
                } else {
                    cx.theme().colors().secondary.darken(0.1).opacity(0.8)
                }
            }
            Self::Link | Self::Text => Oklch::transparent_black(),
        };

        let border = self.variant_border_color(bg, outline, cx);
        let fg = match self {
            Self::Link => cx.theme().colors().link_text_hover,
            _ => self.variant_text_color(outline, cx),
        };

        ButtonVariantStyle {
            bg,
            border,
            fg,
            underline: self.variant_underline(),
            shadow: self.variant_shadow(outline),
        }
    }

    pub(crate) fn active(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        let bg = match self {
            Self::Default => {
                cx.theme()
                    .colors().input
                    .mix_oklab(Oklch::transparent_black(), 0.7)
            }
            Self::Primary => {
                if outline {
                    cx.theme()
                        .colors().primary
                        .mix_oklab(Oklch::transparent_black(), 0.4)
                } else {
                    cx.theme().colors().primary.opacity(0.7)
                }
            }
            Self::Secondary | Self::Outlined => cx.theme().colors().element_active,
            Self::Ghost => {
                if cx.theme().is_dark() {
                    cx.theme().colors().secondary.lighten(0.2).opacity(0.8)
                } else {
                    cx.theme().colors().secondary.darken(0.2).opacity(0.8)
                }
            }
            Self::Danger => cx
                .theme()
                .status().error.color
                .mix_oklab(Oklch::transparent_black(), 0.4),
            Self::Warning => cx
                .theme()
                .status().warning.color
                .mix_oklab(Oklch::transparent_black(), 0.4),
            Self::Success => cx
                .theme()
                .status().success.color
                .mix_oklab(Oklch::transparent_black(), 0.4),
            Self::Info => cx
                .theme()
                .status().info.color
                .mix_oklab(Oklch::transparent_black(), 0.4),
            Self::Custom(colors) => colors.color.mix_oklab(Oklch::transparent_black(), 0.4),
            Self::Link | Self::Text => Oklch::transparent_black(),
        };
        let border = self.variant_border_color(bg, outline, cx);
        let fg = match self {
            Self::Link => cx.theme().colors().accent,
            Self::Text => cx.theme().colors().foreground.opacity(0.7),
            _ => self.variant_text_color(outline, cx),
        };

        ButtonVariantStyle {
            bg,
            border,
            fg,
            underline: self.variant_underline(),
            shadow: self.variant_shadow(outline),
        }
    }

    pub(crate) fn selected(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        let bg = match self {
            Self::Default => {
                cx.theme()
                    .colors().input
                    .mix_oklab(Oklch::transparent_black(), 0.7)
            }
            Self::Primary => cx.theme().colors().primary.opacity(0.7),
            Self::Secondary | Self::Ghost | Self::Outlined => cx.theme().colors().element_active,
            Self::Danger => cx.theme().status().error.color.opacity(0.7),
            Self::Warning => cx.theme().status().warning.color.opacity(0.7),
            Self::Success => cx.theme().status().success.color.opacity(0.7),
            Self::Info => cx.theme().status().info.color.opacity(0.7),
            Self::Link | Self::Text => Oklch::transparent_black(),
            Self::Custom(colors) => colors.active,
        };

        let border = self.variant_border_color(bg, outline, cx);
        let fg = match self {
            Self::Link => cx.theme().colors().accent,
            Self::Text => cx.theme().colors().foreground.opacity(0.7),
            _ => self.variant_text_color(false, cx),
        };

        ButtonVariantStyle {
            bg,
            border,
            fg,
            underline: self.variant_underline(),
            shadow: self.variant_shadow(outline),
        }
    }

    pub(crate) fn disabled(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        let bg = match self {
            Self::Default | Self::Link | Self::Ghost | Self::Text | Self::Outlined => {
                Oklch::transparent_black()
            }
            Self::Primary => cx.theme().colors().primary.opacity(0.15),
            Self::Danger => cx.theme().status().error.color.opacity(0.15),
            Self::Warning => cx.theme().status().warning.color.opacity(0.15),
            Self::Success => cx.theme().status().success.color.opacity(0.15),
            Self::Info => cx.theme().status().info.color.opacity(0.15),
            Self::Secondary => cx.theme().colors().secondary.opacity(1.5),
            Self::Custom(style) => style.color.opacity(0.15),
        };
        let fg = cx.theme().colors().muted_foreground.opacity(0.5);
        let (bg, border) = if outline {
            (
                cx.theme().colors().input.opacity(0.5),
                cx.theme().colors().border.opacity(0.5),
            )
        } else if let Self::Default = self {
            (
                cx.theme().colors().input.opacity(0.5),
                cx.theme().colors().input.opacity(0.5),
            )
        } else {
            (bg, bg)
        };

        ButtonVariantStyle {
            bg,
            border,
            fg,
            underline: self.variant_underline(),
            shadow: false,
        }
    }
}
