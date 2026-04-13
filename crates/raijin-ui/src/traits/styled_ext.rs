use inazuma::{
    App, BoxShadow, Corners, DefiniteLength, Edges, FocusHandle, Pixels, Styled, Window, oklch,
    point,
};
use raijin_theme::ActiveTheme;

use crate::ElevationIndex;
use crate::prelude::*;

fn elevated<E: Styled>(this: E, cx: &App, index: ElevationIndex) -> E {
    this.bg(cx.theme().colors().elevated_surface)
        .rounded_lg()
        .border_1()
        .border_color(cx.theme().colors().border_variant)
        .shadow(index.shadow(cx))
}

fn elevated_borderless<E: Styled>(this: E, cx: &mut App, index: ElevationIndex) -> E {
    this.bg(cx.theme().colors().elevated_surface)
        .rounded_lg()
        .shadow(index.shadow(cx))
}

/// Extends [`inazuma::Styled`] with Raijin-specific styling methods.
// gate on rust-analyzer so rust-analyzer never needs to expand this macro, it takes up to 10 seconds to expand due to inefficiencies in rust-analyzers proc-macro srv
#[cfg_attr(
    all(debug_assertions, not(rust_analyzer)),
    inazuma_macros::derive_inspector_reflection
)]
pub trait StyledExt: Styled + Sized {
    /// Horizontally stacks elements.
    ///
    /// Sets `flex()`, `flex_row()`, `items_center()`
    fn h_flex(self) -> Self {
        self.flex().flex_row().items_center()
    }

    /// Vertically stacks elements.
    ///
    /// Sets `flex()`, `flex_col()`
    fn v_flex(self) -> Self {
        self.flex().flex_col()
    }

    /// The [`Surface`](ElevationIndex::Surface) elevation level, located above the app background, is the standard level for all elements
    ///
    /// Sets `bg()`, `rounded_lg()`, `border()`, `border_color()`, `shadow()`
    ///
    /// Example Elements: Title Bar, Panel, Tab Bar, Editor
    fn elevation_1(self, cx: &App) -> Self {
        elevated(self, cx, ElevationIndex::Surface)
    }

    /// See [`elevation_1`](Self::elevation_1).
    ///
    /// Renders a borderless version [`elevation_1`](Self::elevation_1).
    fn elevation_1_borderless(self, cx: &mut App) -> Self {
        elevated_borderless(self, cx, ElevationIndex::Surface)
    }

    /// Non-Modal Elevated Surfaces appear above the [`Surface`](ElevationIndex::Surface) layer and is used for things that should appear above most UI elements like an editor or panel, but not elements like popovers, context menus, modals, etc.
    ///
    /// Sets `bg()`, `rounded_lg()`, `border()`, `border_color()`, `shadow()`
    ///
    /// Examples: Notifications, Palettes, Detached/Floating Windows, Detached/Floating Panels
    fn elevation_2(self, cx: &App) -> Self {
        elevated(self, cx, ElevationIndex::ElevatedSurface)
    }

    /// See [`elevation_2`](Self::elevation_2).
    ///
    /// Renders a borderless version [`elevation_2`](Self::elevation_2).
    fn elevation_2_borderless(self, cx: &mut App) -> Self {
        elevated_borderless(self, cx, ElevationIndex::ElevatedSurface)
    }

    /// Modal Surfaces are used for elements that should appear above all other UI elements and are located above the wash layer. This is the maximum elevation at which UI elements can be rendered in their default state.
    ///
    /// Elements rendered at this layer should have an enforced behavior: Any interaction outside of the modal will either dismiss the modal or prompt an action (Save your progress, etc) then dismiss the modal.
    ///
    /// If the element does not have this behavior, it should be rendered at the [`Elevated Surface`](ElevationIndex::ElevatedSurface) layer.
    ///
    /// Sets `bg()`, `rounded_lg()`, `border()`, `border_color()`, `shadow()`
    ///
    /// Examples: Settings Modal, Channel Management, Wizards/Setup UI, Dialogs
    fn elevation_3(self, cx: &App) -> Self {
        elevated(self, cx, ElevationIndex::ModalSurface)
    }

    /// See [`elevation_3`](Self::elevation_3).
    ///
    /// Renders a borderless version [`elevation_3`](Self::elevation_3).
    fn elevation_3_borderless(self, cx: &mut App) -> Self {
        elevated_borderless(self, cx, ElevationIndex::ModalSurface)
    }

    /// The theme's primary border color.
    fn border_primary(self, cx: &mut App) -> Self {
        self.border_color(cx.theme().colors().border)
    }

    /// The theme's secondary or muted border color.
    fn border_muted(self, cx: &mut App) -> Self {
        self.border_color(cx.theme().colors().border_variant)
    }

    /// Sets the background color to red for debugging when building UI.
    fn debug_bg_red(self) -> Self {
        self.bg(oklch(0.628, 0.2577, 29.2339))
    }

    /// Sets the background color to green for debugging when building UI.
    fn debug_bg_green(self) -> Self {
        self.bg(oklch(0.8664, 0.2948, 142.4949))
    }

    /// Sets the background color to blue for debugging when building UI.
    fn debug_bg_blue(self) -> Self {
        self.bg(oklch(0.452, 0.3132, 264.0554))
    }

    /// Sets the background color to yellow for debugging when building UI.
    fn debug_bg_yellow(self) -> Self {
        self.bg(oklch(0.968, 0.211, 109.7692))
    }

    /// Sets the background color to cyan for debugging when building UI.
    fn debug_bg_cyan(self) -> Self {
        self.bg(oklch(0.8824, 0.1992, 160.117))
    }

    /// Sets the background color to magenta for debugging when building UI.
    fn debug_bg_magenta(self) -> Self {
        self.bg(oklch(0.7016, 0.3225, 328.3525))
    }

    // ── From inazuma-component StyledExt ─────────────────────────────

    /// Refine the style of this element.
    fn refine_style(mut self, style: &inazuma::StyleRefinement) -> Self {
        self.style().refine(style);
        self
    }

    /// Apply paddings from an Edges value.
    fn paddings<L>(self, paddings: impl Into<Edges<L>>) -> Self
    where
        L: Into<DefiniteLength> + Clone + Default + std::fmt::Debug + PartialEq,
    {
        let paddings = paddings.into();
        self.pt(paddings.top.into())
            .pb(paddings.bottom.into())
            .pl(paddings.left.into())
            .pr(paddings.right.into())
    }

    /// Apply margins from an Edges value.
    fn margins<L>(self, margins: impl Into<Edges<L>>) -> Self
    where
        L: Into<DefiniteLength> + Clone + Default + std::fmt::Debug + PartialEq,
    {
        let margins = margins.into();
        self.mt(margins.top.into())
            .mb(margins.bottom.into())
            .ml(margins.left.into())
            .mr(margins.right.into())
    }

    /// Set corner radii for the element.
    fn corner_radii(self, radius: Corners<Pixels>) -> Self {
        self.rounded_tl(radius.top_left)
            .rounded_tr(radius.top_right)
            .rounded_bl(radius.bottom_left)
            .rounded_br(radius.bottom_right)
    }

    /// Render a 1px border for focus indication.
    fn focused_border(self, cx: &App) -> Self {
        self.border_1()
            .border_color(cx.theme().colors().border_focused)
    }

    // Font weight convenience methods

    fn font_thin(self) -> Self {
        self.font_weight(inazuma::FontWeight::THIN)
    }
    fn font_extralight(self) -> Self {
        self.font_weight(inazuma::FontWeight::EXTRA_LIGHT)
    }
    fn font_light(self) -> Self {
        self.font_weight(inazuma::FontWeight::LIGHT)
    }
    fn font_normal(self) -> Self {
        self.font_weight(inazuma::FontWeight::NORMAL)
    }
    fn font_medium(self) -> Self {
        self.font_weight(inazuma::FontWeight::MEDIUM)
    }
    fn font_semibold(self) -> Self {
        self.font_weight(inazuma::FontWeight::SEMIBOLD)
    }
    fn font_bold(self) -> Self {
        self.font_weight(inazuma::FontWeight::BOLD)
    }
    fn font_extrabold(self) -> Self {
        self.font_weight(inazuma::FontWeight::EXTRA_BOLD)
    }
    fn font_black(self) -> Self {
        self.font_weight(inazuma::FontWeight::BLACK)
    }

    // Debug border helpers (from inazuma-component)

    fn debug_red(self) -> Self {
        if cfg!(debug_assertions) {
            self.border_1().border_color(inazuma::red())
        } else {
            self
        }
    }

    fn debug_blue(self) -> Self {
        if cfg!(debug_assertions) {
            self.border_1().border_color(inazuma::blue())
        } else {
            self
        }
    }

    fn debug_green(self) -> Self {
        if cfg!(debug_assertions) {
            self.border_1().border_color(inazuma::green())
        } else {
            self
        }
    }

    fn debug_yellow(self) -> Self {
        if cfg!(debug_assertions) {
            self.border_1().border_color(inazuma::yellow())
        } else {
            self
        }
    }

    fn debug_focused(self, focus_handle: &FocusHandle, window: &Window, cx: &App) -> Self {
        if cfg!(debug_assertions) {
            if focus_handle.contains_focused(window, cx) {
                self.debug_blue()
            } else {
                self
            }
        } else {
            self
        }
    }

    /// Apply standard popover styling: elevated surface background, border, rounded corners.
    fn popover_style(self, cx: &App) -> Self {
        self.bg(cx.theme().colors().elevated_surface)
            .border_1()
            .border_color(cx.theme().colors().border)
            .rounded_lg()
    }
}

impl<E: Styled> StyledExt for E {}

/// Create a [`BoxShadow`] like CSS.
///
/// e.g: `box_shadow(0., 0., 10., 0., Oklch::black().opacity(0.1))`
#[inline(always)]
pub fn box_shadow(
    x: impl Into<Pixels>,
    y: impl Into<Pixels>,
    blur: impl Into<Pixels>,
    spread: impl Into<Pixels>,
    color: impl Into<inazuma::Oklch>,
) -> BoxShadow {
    BoxShadow {
        offset: point(x.into(), y.into()),
        blur_radius: blur.into(),
        spread_radius: spread.into(),
        color: color.into(),
    }
}
