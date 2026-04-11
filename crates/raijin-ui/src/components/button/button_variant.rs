use inazuma::{App, Oklch, Pixels, Rems, Window, transparent_black};
use raijin_theme::ActiveTheme;

use crate::{Color, ElevationIndex};
use crate::prelude::{Clickable, Disableable, Toggleable, rems_from_px};

// ---------------------------------------------------------------------------
// ButtonVariant — the unified variant enum
// ---------------------------------------------------------------------------

/// The semantic variant of a button.
///
/// Every variant maps to a complete set of colors for normal, hovered, active,
/// selected, focused, and disabled states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    /// The default button — subtle ghost-element background, no border.
    #[default]
    Default,

    /// A high-emphasis filled button with a solid element background.
    Primary,

    /// A secondary button with a visible border and element background.
    Secondary,

    /// A destructive action button colored with the error/danger status color.
    Danger,

    /// An informational button colored with the info/accent status color.
    Info,

    /// A success/positive button colored with the success status color.
    Success,

    /// A warning button colored with the warning status color.
    Warning,

    /// A ghost button — fully transparent background, visible only on hover.
    Ghost,

    /// A link-styled button — no background, uses accent text color.
    Link,

    /// A text button — looks like plain text with no background or border.
    Text,

    /// An outlined button with visible border but transparent background.
    Outlined,

    /// A fully custom-colored button.
    Custom(ButtonCustomVariant),
}

impl ButtonVariant {
    /// Returns `true` if this variant renders as a link.
    #[inline]
    pub fn is_link(&self) -> bool {
        matches!(self, Self::Link)
    }

    /// Returns `true` if this variant renders as plain text.
    #[inline]
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text)
    }

    /// Returns `true` if this variant renders as a ghost button.
    #[inline]
    pub fn is_ghost(&self) -> bool {
        matches!(self, Self::Ghost)
    }

    /// Returns `true` if this variant renders as outlined.
    #[inline]
    pub fn is_outlined(&self) -> bool {
        matches!(self, Self::Outlined)
    }

    /// Returns `true` if this variant should have no padding (link, text).
    #[inline]
    pub fn no_padding(&self) -> bool {
        self.is_link() || self.is_text()
    }

    /// Returns `true` if this is the default variant.
    #[inline]
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Default)
    }

    /// Returns `true` if this variant needs a visible border in its enabled state.
    #[inline]
    pub fn has_border(&self) -> bool {
        matches!(self, Self::Secondary | Self::Outlined)
    }
}

// ---------------------------------------------------------------------------
// ButtonCustomVariant
// ---------------------------------------------------------------------------

/// A fully custom set of colors for a button variant.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ButtonCustomVariant {
    /// Background color.
    pub color: Oklch,
    /// Foreground (text/icon) color.
    pub foreground: Oklch,
    /// Hover background color.
    pub hover: Oklch,
    /// Active/pressed background color.
    pub active: Oklch,
    /// Whether to render a shadow.
    pub shadow: bool,
}

impl ButtonCustomVariant {
    /// Creates a new custom variant with transparent backgrounds and the theme's
    /// default foreground color.
    pub fn new(cx: &App) -> Self {
        Self {
            color: transparent_black(),
            foreground: cx.theme().colors().text,
            hover: transparent_black(),
            active: transparent_black(),
            shadow: false,
        }
    }

    /// Set the background color.
    pub fn color(mut self, color: Oklch) -> Self {
        self.color = color;
        self
    }

    /// Set the foreground (text/icon) color.
    pub fn foreground(mut self, color: Oklch) -> Self {
        self.foreground = color;
        self
    }

    /// Set the hover background color.
    pub fn hover(mut self, color: Oklch) -> Self {
        self.hover = color;
        self
    }

    /// Set the active/pressed background color.
    pub fn active(mut self, color: Oklch) -> Self {
        self.active = color;
        self
    }

    /// Set whether to show a shadow.
    pub fn shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }
}

// ---------------------------------------------------------------------------
// ButtonVariants trait — fluent builder for setting variants
// ---------------------------------------------------------------------------

/// A trait providing fluent builder methods for setting button variants.
///
/// Implement `with_variant` on your button type, and you get `.primary()`,
/// `.danger()`, `.ghost()`, etc. for free.
pub trait ButtonVariants: Sized {
    /// Set the variant directly.
    fn with_variant(self, variant: ButtonVariant) -> Self;

    /// Primary (high-emphasis filled) variant.
    fn primary(self) -> Self {
        self.with_variant(ButtonVariant::Primary)
    }

    /// Secondary variant.
    fn secondary(self) -> Self {
        self.with_variant(ButtonVariant::Secondary)
    }

    /// Danger/destructive variant.
    fn danger(self) -> Self {
        self.with_variant(ButtonVariant::Danger)
    }

    /// Warning variant.
    fn warning(self) -> Self {
        self.with_variant(ButtonVariant::Warning)
    }

    /// Success variant.
    fn success(self) -> Self {
        self.with_variant(ButtonVariant::Success)
    }

    /// Info variant.
    fn info(self) -> Self {
        self.with_variant(ButtonVariant::Info)
    }

    /// Ghost (transparent background) variant.
    fn ghost(self) -> Self {
        self.with_variant(ButtonVariant::Ghost)
    }

    /// Link variant (underlined, no background).
    fn link(self) -> Self {
        self.with_variant(ButtonVariant::Link)
    }

    /// Text variant (looks like plain text).
    fn text(self) -> Self {
        self.with_variant(ButtonVariant::Text)
    }

    /// Outlined variant (visible border, transparent fill).
    fn outlined(self) -> Self {
        self.with_variant(ButtonVariant::Outlined)
    }

    /// Custom variant with user-specified colors.
    fn custom(self, style: ButtonCustomVariant) -> Self {
        self.with_variant(ButtonVariant::Custom(style))
    }
}

// ---------------------------------------------------------------------------
// ButtonRounded
// ---------------------------------------------------------------------------

/// Controls the border radius of a button.
#[derive(Default, Clone, Copy)]
pub enum ButtonRounded {
    /// No rounding (sharp corners).
    None,
    /// Small rounding.
    Small,
    /// Medium rounding (default).
    #[default]
    Medium,
    /// Large rounding.
    Large,
    /// Custom pixel size.
    Size(Pixels),
}

impl From<Pixels> for ButtonRounded {
    fn from(px: Pixels) -> Self {
        ButtonRounded::Size(px)
    }
}

// ---------------------------------------------------------------------------
// TintColor — backwards compat for the original Zed API
// ---------------------------------------------------------------------------

/// A semantic tint color used to color a button, mapping to status colors
/// in the theme.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Default)]
pub enum TintColor {
    #[default]
    Accent,
    Error,
    Warning,
    Success,
}

impl TintColor {
    /// Resolve the tint color to a [`ButtonLikeStyles`] using the theme's
    /// status colors.
    pub fn button_like_style(self, cx: &mut App) -> ButtonLikeStyles {
        match self {
            TintColor::Accent => ButtonLikeStyles {
                background: cx.theme().status().info.background,
                border_color: cx.theme().status().info.border,
                label_color: cx.theme().colors().text,
                icon_color: cx.theme().colors().text,
            },
            TintColor::Error => ButtonLikeStyles {
                background: cx.theme().status().error.background,
                border_color: cx.theme().status().error.border,
                label_color: cx.theme().colors().text,
                icon_color: cx.theme().colors().text,
            },
            TintColor::Warning => ButtonLikeStyles {
                background: cx.theme().status().warning.background,
                border_color: cx.theme().status().warning.border,
                label_color: cx.theme().colors().text,
                icon_color: cx.theme().colors().text,
            },
            TintColor::Success => ButtonLikeStyles {
                background: cx.theme().status().success.background,
                border_color: cx.theme().status().success.border,
                label_color: cx.theme().colors().text,
                icon_color: cx.theme().colors().text,
            },
        }
    }
}

impl From<TintColor> for Color {
    fn from(tint: TintColor) -> Self {
        match tint {
            TintColor::Accent => Color::Accent,
            TintColor::Error => Color::Error,
            TintColor::Warning => Color::Warning,
            TintColor::Success => Color::Success,
        }
    }
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// ButtonStyle — convenience alias
// ---------------------------------------------------------------------------

/// Convenience alias for [`ButtonVariant`].
pub type ButtonStyle = ButtonVariant;

/// Shorthand constants for common variants.
impl ButtonVariant {
    pub const FILLED: Self = Self::Primary;
    pub const SUBTLE: Self = Self::Default;
    pub const TRANSPARENT: Self = Self::Ghost;
    pub const OUTLINED: Self = Self::Outlined;
    pub const OUTLINED_GHOST: Self = Self::Outlined;

    /// Create a variant from a [`TintColor`].
    pub fn tinted(tint: TintColor) -> Self {
        match tint {
            TintColor::Accent => Self::Info,
            TintColor::Error => Self::Danger,
            TintColor::Warning => Self::Warning,
            TintColor::Success => Self::Success,
        }
    }
}

/// Allow converting from `ButtonVariant` to the generic `Color` enum (used for
/// label coloring based on button style).
impl From<ButtonVariant> for Color {
    fn from(variant: ButtonVariant) -> Self {
        match variant {
            ButtonVariant::Danger => Color::Error,
            ButtonVariant::Info => Color::Accent,
            ButtonVariant::Warning => Color::Warning,
            ButtonVariant::Success => Color::Success,
            _ => Color::Default,
        }
    }
}

// ---------------------------------------------------------------------------
// ButtonLikeStyles — resolved color set for a button state
// ---------------------------------------------------------------------------

/// A fully resolved set of colors for a single button state (normal, hovered,
/// active, focused, disabled).
///
/// This is consumed by `ButtonLike` when rendering.
#[derive(Debug, Clone)]
pub struct ButtonLikeStyles {
    pub background: Oklch,
    pub border_color: Oklch,
    pub label_color: Oklch,
    pub icon_color: Oklch,
}

// ---------------------------------------------------------------------------
// ButtonVariantStyle — extended resolved style
// ---------------------------------------------------------------------------

/// Resolved style for a button variant state with additional flags.
///
/// Compared to `ButtonLikeStyles`, this also carries `underline` and `shadow`
/// flags for richer rendering.
#[derive(Debug, Clone)]
pub struct ButtonVariantStyle {
    pub bg: Oklch,
    pub border: Oklch,
    pub fg: Oklch,
    pub underline: bool,
    pub shadow: bool,
}

// ---------------------------------------------------------------------------
// ButtonLikeRounding — per-corner rounding control
// ---------------------------------------------------------------------------

/// Per-corner rounding control for `ButtonLike`, used for button groups where
/// only some corners should be rounded.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct ButtonLikeRounding {
    pub top_left: bool,
    pub top_right: bool,
    pub bottom_right: bool,
    pub bottom_left: bool,
}

impl ButtonLikeRounding {
    pub const ALL: Self = Self {
        top_left: true,
        top_right: true,
        bottom_right: true,
        bottom_left: true,
    };
    pub const LEFT: Self = Self {
        top_left: true,
        top_right: false,
        bottom_right: false,
        bottom_left: true,
    };
    pub const RIGHT: Self = Self {
        top_left: false,
        top_right: true,
        bottom_right: true,
        bottom_left: false,
    };
}

// ---------------------------------------------------------------------------
// ButtonSize
// ---------------------------------------------------------------------------

/// The height of a button.
///
/// Can also be used to size non-button elements to align with buttons.
#[derive(Default, PartialEq, Clone, Copy)]
pub enum ButtonSize {
    Large,
    Medium,
    #[default]
    Default,
    Compact,
    None,
}

impl ButtonSize {
    pub fn rems(self) -> Rems {
        match self {
            ButtonSize::Large => rems_from_px(32.),
            ButtonSize::Medium => rems_from_px(28.),
            ButtonSize::Default => rems_from_px(22.),
            ButtonSize::Compact => rems_from_px(18.),
            ButtonSize::None => rems_from_px(16.),
        }
    }
}

// ---------------------------------------------------------------------------
// IconPosition / KeybindingPosition
// ---------------------------------------------------------------------------

/// Where to place an icon relative to the button label.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Default)]
pub enum IconPosition {
    #[default]
    Start,
    End,
}

/// Where to place a keybinding hint relative to the button label.
#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum KeybindingPosition {
    Start,
    #[default]
    End,
}

// ---------------------------------------------------------------------------
// ButtonCommon trait
// ---------------------------------------------------------------------------

/// A common set of traits all buttons must implement.
pub trait ButtonCommon: Disableable {
    /// A unique element ID to identify the button.
    fn id(&self) -> &inazuma::ElementId;

    /// Set the visual variant of the button.
    fn style(self, style: ButtonVariant) -> Self;

    /// Set the size of the button.
    fn size(self, size: ButtonSize) -> Self;

    /// Set the tooltip shown on hover.
    fn button_tooltip(
        self,
        tooltip: impl Fn(&mut Window, &mut App) -> inazuma::AnyView + 'static,
    ) -> Self;

    /// Set the tab index for keyboard navigation.
    fn tab_index(self, tab_index: impl Into<isize>) -> Self;

    /// Set the elevation layer the button sits on (affects background color).
    fn layer(self, elevation: ElevationIndex) -> Self;

    /// Track a focus handle on this button.
    fn track_focus(self, focus_handle: &inazuma::FocusHandle) -> Self;
}

// ---------------------------------------------------------------------------
// SelectableButton trait
// ---------------------------------------------------------------------------

/// A trait for buttons that can be selected. Enables setting the
/// [`ButtonVariant`] used when the button is in a selected state.
pub trait SelectableButton: Toggleable {
    fn selected_style(self, style: ButtonVariant) -> Self;
}

// ---------------------------------------------------------------------------
// Style resolution helpers
// ---------------------------------------------------------------------------

fn element_bg_from_elevation(elevation: Option<ElevationIndex>, cx: &mut App) -> Oklch {
    match elevation {
        Some(ElevationIndex::Background) => cx.theme().colors().element_background,
        Some(ElevationIndex::ElevatedSurface) => cx.theme().colors().elevated_surface,
        Some(ElevationIndex::Surface) => cx.theme().colors().surface,
        Some(ElevationIndex::ModalSurface) => cx.theme().colors().background,
        _ => cx.theme().colors().element_background,
    }
}

/// Resolve a tint-style variant to its status-based `ButtonLikeStyles`,
/// applying a darken for the hover state.
fn tint_for_variant(variant: &ButtonVariant, cx: &mut App) -> Option<TintColor> {
    match variant {
        ButtonVariant::Info => Some(TintColor::Accent),
        ButtonVariant::Danger => Some(TintColor::Error),
        ButtonVariant::Warning => Some(TintColor::Warning),
        ButtonVariant::Success => Some(TintColor::Success),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// ButtonVariant — Zed-style resolution (ButtonLikeStyles)
// ---------------------------------------------------------------------------

impl ButtonVariant {
    /// Resolve the enabled/normal style for this variant.
    pub fn enabled(self, elevation: Option<ElevationIndex>, cx: &mut App) -> ButtonLikeStyles {
        match self {
            ButtonVariant::Primary => ButtonLikeStyles {
                background: element_bg_from_elevation(elevation, cx),
                border_color: transparent_black(),
                label_color: Color::Default.color(cx),
                icon_color: Color::Default.color(cx),
            },
            ButtonVariant::Info
            | ButtonVariant::Danger
            | ButtonVariant::Warning
            | ButtonVariant::Success => {
                tint_for_variant(&self, cx).unwrap().button_like_style(cx)
            }
            ButtonVariant::Secondary => ButtonLikeStyles {
                background: element_bg_from_elevation(elevation, cx),
                border_color: cx.theme().colors().border_variant,
                label_color: Color::Default.color(cx),
                icon_color: Color::Default.color(cx),
            },
            ButtonVariant::Outlined => ButtonLikeStyles {
                background: transparent_black(),
                border_color: cx.theme().colors().border_variant,
                label_color: Color::Default.color(cx),
                icon_color: Color::Default.color(cx),
            },
            ButtonVariant::Default => ButtonLikeStyles {
                background: cx.theme().colors().ghost_element_background,
                border_color: transparent_black(),
                label_color: Color::Default.color(cx),
                icon_color: Color::Default.color(cx),
            },
            ButtonVariant::Ghost | ButtonVariant::Link | ButtonVariant::Text => ButtonLikeStyles {
                background: transparent_black(),
                border_color: transparent_black(),
                label_color: Color::Default.color(cx),
                icon_color: Color::Default.color(cx),
            },
            ButtonVariant::Custom(cv) => ButtonLikeStyles {
                background: cv.color,
                border_color: transparent_black(),
                label_color: cv.foreground,
                icon_color: cv.foreground,
            },
        }
    }

    /// Resolve the hovered style.
    pub fn hovered_style(
        self,
        elevation: Option<ElevationIndex>,
        cx: &mut App,
    ) -> ButtonLikeStyles {
        match self {
            ButtonVariant::Primary => {
                let mut bg = element_bg_from_elevation(elevation, cx);
                bg.fade_out(0.5);
                ButtonLikeStyles {
                    background: bg,
                    border_color: transparent_black(),
                    label_color: Color::Default.color(cx),
                    icon_color: Color::Default.color(cx),
                }
            }
            ButtonVariant::Info
            | ButtonVariant::Danger
            | ButtonVariant::Warning
            | ButtonVariant::Success => {
                let mut styles = tint_for_variant(&self, cx).unwrap().button_like_style(cx);
                let theme = cx.theme();
                styles.background = theme.darken(styles.background, 0.05, 0.2);
                styles
            }
            ButtonVariant::Secondary | ButtonVariant::Outlined => ButtonLikeStyles {
                background: cx.theme().colors().ghost_element_hover,
                border_color: cx.theme().colors().border,
                label_color: Color::Default.color(cx),
                icon_color: Color::Default.color(cx),
            },
            ButtonVariant::Default => ButtonLikeStyles {
                background: cx.theme().colors().ghost_element_hover,
                border_color: transparent_black(),
                label_color: Color::Default.color(cx),
                icon_color: Color::Default.color(cx),
            },
            ButtonVariant::Ghost | ButtonVariant::Link | ButtonVariant::Text => ButtonLikeStyles {
                background: transparent_black(),
                border_color: transparent_black(),
                label_color: Color::Muted.color(cx),
                icon_color: Color::Muted.color(cx),
            },
            ButtonVariant::Custom(cv) => ButtonLikeStyles {
                background: cv.hover,
                border_color: transparent_black(),
                label_color: cv.foreground,
                icon_color: cv.foreground,
            },
        }
    }

    /// Resolve the active/pressed style.
    pub fn active_style(self, cx: &mut App) -> ButtonLikeStyles {
        match self {
            ButtonVariant::Primary => ButtonLikeStyles {
                background: cx.theme().colors().element_active,
                border_color: transparent_black(),
                label_color: Color::Default.color(cx),
                icon_color: Color::Default.color(cx),
            },
            ButtonVariant::Info
            | ButtonVariant::Danger
            | ButtonVariant::Warning
            | ButtonVariant::Success => {
                tint_for_variant(&self, cx).unwrap().button_like_style(cx)
            }
            ButtonVariant::Default => ButtonLikeStyles {
                background: cx.theme().colors().ghost_element_active,
                border_color: transparent_black(),
                label_color: Color::Default.color(cx),
                icon_color: Color::Default.color(cx),
            },
            ButtonVariant::Secondary => ButtonLikeStyles {
                background: cx.theme().colors().element_active,
                border_color: cx.theme().colors().border_variant,
                label_color: Color::Default.color(cx),
                icon_color: Color::Default.color(cx),
            },
            ButtonVariant::Outlined => ButtonLikeStyles {
                background: transparent_black(),
                border_color: cx.theme().colors().border_variant,
                label_color: Color::Default.color(cx),
                icon_color: Color::Default.color(cx),
            },
            ButtonVariant::Ghost | ButtonVariant::Link | ButtonVariant::Text => ButtonLikeStyles {
                background: transparent_black(),
                border_color: transparent_black(),
                label_color: Color::Muted.color(cx),
                icon_color: Color::Muted.color(cx),
            },
            ButtonVariant::Custom(cv) => ButtonLikeStyles {
                background: cv.active,
                border_color: transparent_black(),
                label_color: cv.foreground,
                icon_color: cv.foreground,
            },
        }
    }

    /// Resolve the focused style.
    pub fn focused_style(self, _window: &mut Window, cx: &mut App) -> ButtonLikeStyles {
        match self {
            ButtonVariant::Primary => ButtonLikeStyles {
                background: cx.theme().colors().element_background,
                border_color: cx.theme().colors().border_focused,
                label_color: Color::Default.color(cx),
                icon_color: Color::Default.color(cx),
            },
            ButtonVariant::Info
            | ButtonVariant::Danger
            | ButtonVariant::Warning
            | ButtonVariant::Success => {
                tint_for_variant(&self, cx).unwrap().button_like_style(cx)
            }
            ButtonVariant::Default => ButtonLikeStyles {
                background: cx.theme().colors().ghost_element_background,
                border_color: cx.theme().colors().border_focused,
                label_color: Color::Default.color(cx),
                icon_color: Color::Default.color(cx),
            },
            ButtonVariant::Secondary => ButtonLikeStyles {
                background: cx.theme().colors().ghost_element_background,
                border_color: cx.theme().colors().border,
                label_color: Color::Default.color(cx),
                icon_color: Color::Default.color(cx),
            },
            ButtonVariant::Outlined => ButtonLikeStyles {
                background: transparent_black(),
                border_color: cx.theme().colors().border,
                label_color: Color::Default.color(cx),
                icon_color: Color::Default.color(cx),
            },
            ButtonVariant::Ghost | ButtonVariant::Link | ButtonVariant::Text => ButtonLikeStyles {
                background: transparent_black(),
                border_color: cx.theme().colors().border_focused,
                label_color: Color::Accent.color(cx),
                icon_color: Color::Accent.color(cx),
            },
            ButtonVariant::Custom(cv) => ButtonLikeStyles {
                background: cv.color,
                border_color: cx.theme().colors().border_focused,
                label_color: cv.foreground,
                icon_color: cv.foreground,
            },
        }
    }

    /// Resolve the disabled style.
    pub fn disabled_style(
        self,
        elevation: Option<ElevationIndex>,
        _window: &mut Window,
        cx: &mut App,
    ) -> ButtonLikeStyles {
        match self {
            ButtonVariant::Primary => ButtonLikeStyles {
                background: cx.theme().colors().element_disabled,
                border_color: cx.theme().colors().border_disabled,
                label_color: Color::Disabled.color(cx),
                icon_color: Color::Disabled.color(cx),
            },
            ButtonVariant::Info
            | ButtonVariant::Danger
            | ButtonVariant::Warning
            | ButtonVariant::Success => {
                // Tinted variants keep their tint even when disabled
                tint_for_variant(&self, cx).unwrap().button_like_style(cx)
            }
            ButtonVariant::Default => ButtonLikeStyles {
                background: cx.theme().colors().ghost_element_disabled,
                border_color: cx.theme().colors().border_disabled,
                label_color: Color::Disabled.color(cx),
                icon_color: Color::Disabled.color(cx),
            },
            ButtonVariant::Secondary | ButtonVariant::Outlined => ButtonLikeStyles {
                background: cx.theme().colors().element_disabled,
                border_color: cx.theme().colors().border_disabled,
                label_color: Color::Default.color(cx),
                icon_color: Color::Default.color(cx),
            },
            ButtonVariant::Ghost | ButtonVariant::Link | ButtonVariant::Text => ButtonLikeStyles {
                background: transparent_black(),
                border_color: transparent_black(),
                label_color: Color::Disabled.color(cx),
                icon_color: Color::Disabled.color(cx),
            },
            ButtonVariant::Custom(cv) => ButtonLikeStyles {
                background: cv.color,
                border_color: transparent_black(),
                label_color: cv.foreground,
                icon_color: cv.foreground,
            },
        }
    }
}

