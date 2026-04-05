use crate::{ActiveTheme, Colorize as _};
use inazuma::{App, Oklch, transparent_white};

use super::ButtonVariant;

pub(super) struct ButtonVariantStyle {
    pub(super) bg: Oklch,
    pub(super) border: Oklch,
    pub(super) fg: Oklch,
    pub(super) underline: bool,
    pub(super) shadow: bool,
}

impl ButtonVariant {
    fn bg_color(&self, outline: bool, cx: &mut App) -> Oklch {
        if outline {
            return cx.theme().input_background();
        }

        match self {
            Self::Default => cx.theme().input_background(),
            Self::Primary => cx.theme().button_primary,
            Self::Secondary => cx.theme().secondary,
            Self::Danger => cx.theme().danger.mix_oklab(cx.theme().transparent, 0.2),
            Self::Warning => cx.theme().warning.mix_oklab(cx.theme().transparent, 0.2),
            Self::Success => cx.theme().success.mix_oklab(cx.theme().transparent, 0.2),
            Self::Info => cx.theme().info.mix_oklab(cx.theme().transparent, 0.2),
            Self::Ghost | Self::Link | Self::Text => cx.theme().transparent,
            Self::Custom(colors) => colors.color.mix_oklab(cx.theme().transparent, 0.2),
        }
    }

    fn text_color(&self, outline: bool, cx: &mut App) -> Oklch {
        match self {
            Self::Default => cx.theme().foreground,
            Self::Primary => {
                if outline {
                    cx.theme().button_primary
                } else {
                    cx.theme().button_primary_foreground
                }
            }
            Self::Secondary | Self::Ghost => cx.theme().secondary_foreground,
            Self::Danger => cx.theme().danger,
            Self::Warning => cx.theme().warning,
            Self::Success => cx.theme().success,
            Self::Info => cx.theme().info,
            Self::Link => cx.theme().link,
            Self::Text => cx.theme().foreground,
            Self::Custom(colors) => colors.color,
        }
    }

    fn border_color(&self, _bg: Oklch, outline: bool, cx: &mut App) -> Oklch {
        match self {
            Self::Default => cx.theme().input,
            Self::Secondary => cx.theme().border,
            Self::Primary => cx.theme().button_primary,
            Self::Danger => {
                if outline {
                    cx.theme().danger.mix_oklab(Oklch::from(transparent_white()), 0.4)
                } else {
                    cx.theme().danger
                }
            }
            Self::Info => {
                if outline {
                    cx.theme().info.mix_oklab(Oklch::from(transparent_white()), 0.4)
                } else {
                    cx.theme().info
                }
            }
            Self::Warning => {
                if outline {
                    cx.theme().warning.mix_oklab(Oklch::from(transparent_white()), 0.4)
                } else {
                    cx.theme().warning
                }
            }
            Self::Success => {
                if outline {
                    cx.theme().success.mix_oklab(Oklch::from(transparent_white()), 0.4)
                } else {
                    cx.theme().success
                }
            }
            Self::Ghost | Self::Link | Self::Text => cx.theme().transparent,
            Self::Custom(colors) => {
                if outline {
                    colors.color.mix_oklab(Oklch::from(transparent_white()), 0.4)
                } else {
                    colors.color
                }
            }
        }
    }

    fn underline(&self, _: &App) -> bool {
        match self {
            Self::Link => true,
            _ => false,
        }
    }

    fn shadow(&self, outline: bool, _: &App) -> bool {
        match self {
            Self::Default => true,
            Self::Primary | Self::Secondary | Self::Danger => outline,
            Self::Custom(c) => c.shadow,
            _ => false,
        }
    }

    pub(super) fn normal(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        let bg = self.bg_color(outline, cx);
        let border = self.border_color(bg, outline, cx);
        let fg = self.text_color(outline, cx);
        let underline = self.underline(cx);
        let shadow = self.shadow(outline, cx);

        ButtonVariantStyle {
            bg,
            border,
            fg,
            underline,
            shadow,
        }
    }

    pub(super) fn hovered(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        let bg = match self {
            Self::Default => cx.theme().input.mix_oklab(cx.theme().transparent, 0.5),
            Self::Primary => {
                if outline {
                    cx.theme()
                        .button_primary
                        .mix_oklab(cx.theme().transparent, 0.2)
                } else {
                    cx.theme().button_primary_hover
                }
            }
            Self::Secondary => cx.theme().secondary_hover,
            Self::Danger => {
                if outline {
                    cx.theme().danger.mix_oklab(cx.theme().transparent, 0.2)
                } else {
                    cx.theme().danger.mix_oklab(cx.theme().transparent, 0.3)
                }
            }
            Self::Warning => {
                if outline {
                    cx.theme().warning.mix_oklab(cx.theme().transparent, 0.2)
                } else {
                    cx.theme().warning.mix_oklab(cx.theme().transparent, 0.3)
                }
            }
            Self::Success => {
                if outline {
                    cx.theme().success.mix_oklab(cx.theme().transparent, 0.2)
                } else {
                    cx.theme().success.mix_oklab(cx.theme().transparent, 0.3)
                }
            }
            Self::Info => {
                if outline {
                    cx.theme().info.mix_oklab(cx.theme().transparent, 0.2)
                } else {
                    cx.theme().info.mix_oklab(cx.theme().transparent, 0.3)
                }
            }
            Self::Custom(colors) => {
                if outline {
                    colors.color.mix_oklab(cx.theme().transparent, 0.2)
                } else {
                    colors.color.mix_oklab(cx.theme().transparent, 0.3)
                }
            }
            Self::Ghost => {
                if cx.theme().mode.is_dark() {
                    cx.theme().secondary.lighten(0.1).opacity(0.8)
                } else {
                    cx.theme().secondary.darken(0.1).opacity(0.8)
                }
            }
            Self::Link => cx.theme().transparent,
            Self::Text => cx.theme().transparent,
        };

        let border = self.border_color(bg, outline, cx);
        let fg = match self {
            Self::Link => cx.theme().link_hover,
            _ => self.text_color(outline, cx),
        };

        let underline = self.underline(cx);
        let shadow = self.shadow(outline, cx);

        ButtonVariantStyle {
            bg,
            border,
            fg,
            underline,
            shadow,
        }
    }

    pub(super) fn active(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        let bg = match self {
            Self::Default => cx.theme().input.mix_oklab(cx.theme().transparent, 0.7),
            Self::Primary => {
                if outline {
                    cx.theme()
                        .button_primary
                        .mix_oklab(cx.theme().transparent, 0.4)
                } else {
                    cx.theme().button_primary_active
                }
            }
            Self::Secondary => cx.theme().secondary_active,
            Self::Ghost => {
                if cx.theme().mode.is_dark() {
                    cx.theme().secondary.lighten(0.2).opacity(0.8)
                } else {
                    cx.theme().secondary.darken(0.2).opacity(0.8)
                }
            }
            Self::Danger => cx.theme().danger.mix_oklab(cx.theme().transparent, 0.4),
            Self::Warning => cx.theme().warning.mix_oklab(cx.theme().transparent, 0.4),
            Self::Success => cx.theme().success.mix_oklab(cx.theme().transparent, 0.4),
            Self::Info => cx.theme().info.mix_oklab(cx.theme().transparent, 0.4),
            Self::Custom(colors) => colors.color.mix_oklab(cx.theme().transparent, 0.4),
            Self::Link => cx.theme().transparent,
            Self::Text => cx.theme().transparent,
        };
        let border = self.border_color(bg, outline, cx);
        let fg = match self {
            Self::Link => cx.theme().link_active,
            Self::Text => cx.theme().foreground.opacity(0.7),
            _ => self.text_color(outline, cx),
        };
        let underline = self.underline(cx);
        let shadow = self.shadow(outline, cx);

        ButtonVariantStyle {
            bg,
            border,
            fg,
            underline,
            shadow,
        }
    }

    pub(super) fn selected(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        let bg = match self {
            Self::Default => cx.theme().input.mix_oklab(cx.theme().transparent, 0.7),
            Self::Primary => cx.theme().button_primary_active,
            Self::Secondary | Self::Ghost => cx.theme().secondary_active,
            Self::Danger => cx.theme().danger_active,
            Self::Warning => cx.theme().warning_active,
            Self::Success => cx.theme().success_active,
            Self::Info => cx.theme().info_active,
            Self::Link => cx.theme().transparent,
            Self::Text => cx.theme().transparent,
            Self::Custom(colors) => colors.active,
        };

        let border = self.border_color(bg, outline, cx);
        let fg = match self {
            Self::Link => cx.theme().link_active,
            Self::Text => cx.theme().foreground.opacity(0.7),
            _ => self.text_color(false, cx),
        };
        let underline = self.underline(cx);
        let shadow = self.shadow(outline, cx);

        ButtonVariantStyle {
            bg,
            border,
            fg,
            underline,
            shadow,
        }
    }

    pub(super) fn disabled(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        let bg = match self {
            Self::Default | Self::Link | Self::Ghost | Self::Text => cx.theme().transparent,
            Self::Primary => cx.theme().button_primary.opacity(0.15),
            Self::Danger => cx.theme().danger.opacity(0.15),
            Self::Warning => cx.theme().warning.opacity(0.15),
            Self::Success => cx.theme().success.opacity(0.15),
            Self::Info => cx.theme().info.opacity(0.15),
            Self::Secondary => cx.theme().secondary.opacity(1.5),
            Self::Custom(style) => style.color.opacity(0.15),
        };
        let fg = cx.theme().muted_foreground.opacity(0.5);
        let (bg, border) = if outline {
            (
                cx.theme().input_background().opacity(0.5),
                cx.theme().border.opacity(0.5),
            )
        } else if let Self::Default = self {
            (
                cx.theme().input_background().opacity(0.5),
                cx.theme().input.opacity(0.5),
            )
        } else {
            (bg, bg)
        };

        let underline = self.underline(cx);
        let shadow = false;

        ButtonVariantStyle {
            bg,
            border,
            fg,
            underline,
            shadow,
        }
    }
}
