mod decorated_icon;
mod icon_decoration;

use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use decorated_icon::*;
use inazuma::{AnimationElement, AnyElement, Oklch, IntoElement, Rems, Transformation, img, svg};
pub use icon_decoration::*;
pub use inazuma_icons::*;

use crate::traits::transformable::Transformable;
use crate::{Indicator, prelude::*};

#[derive(IntoElement)]
pub enum AnyIcon {
    Icon(Icon),
    AnimatedIcon(AnimationElement<Icon>),
}

impl AnyIcon {
    /// Returns a new [`AnyIcon`] after applying the given mapping function
    /// to the contained [`Icon`].
    pub fn map(self, f: impl FnOnce(Icon) -> Icon) -> Self {
        match self {
            Self::Icon(icon) => Self::Icon(f(icon)),
            Self::AnimatedIcon(animated_icon) => Self::AnimatedIcon(animated_icon.map_element(f)),
        }
    }
}

impl From<Icon> for AnyIcon {
    fn from(value: Icon) -> Self {
        Self::Icon(value)
    }
}

impl From<AnimationElement<Icon>> for AnyIcon {
    fn from(value: AnimationElement<Icon>) -> Self {
        Self::AnimatedIcon(value)
    }
}

impl RenderOnce for AnyIcon {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        match self {
            Self::Icon(icon) => icon.into_any_element(),
            Self::AnimatedIcon(animated_icon) => animated_icon.into_any_element(),
        }
    }
}

#[derive(Default, PartialEq, Copy, Clone)]
pub enum IconSize {
    /// 10px
    Indicator,
    /// 12px
    XSmall,
    /// 14px
    Small,
    #[default]
    /// 16px
    Medium,
    /// 48px
    XLarge,
    Custom(Rems),
}

impl IconSize {
    pub fn rems(self) -> Rems {
        match self {
            IconSize::Indicator => rems_from_px(10.),
            IconSize::XSmall => rems_from_px(12.),
            IconSize::Small => rems_from_px(14.),
            IconSize::Medium => rems_from_px(16.),
            IconSize::XLarge => rems_from_px(48.),
            IconSize::Custom(size) => size,
        }
    }

    /// Returns the individual components of the square that contains this [`IconSize`].
    ///
    /// The returned tuple contains:
    ///   1. The length of one side of the square
    ///   2. The padding of one side of the square
    pub fn square_components(&self, window: &mut Window, cx: &mut App) -> (Pixels, Pixels) {
        let icon_size = self.rems() * window.rem_size();
        let padding = match self {
            IconSize::Indicator => DynamicSpacing::Base00.px(cx),
            IconSize::XSmall => DynamicSpacing::Base02.px(cx),
            IconSize::Small => DynamicSpacing::Base02.px(cx),
            IconSize::Medium => DynamicSpacing::Base02.px(cx),
            IconSize::XLarge => DynamicSpacing::Base02.px(cx),
            // TODO: Wire into dynamic spacing
            IconSize::Custom(size) => size.to_pixels(window.rem_size()),
        };

        (icon_size, padding)
    }

    /// Returns the length of a side of the square that contains this [`IconSize`], with padding.
    pub fn square(&self, window: &mut Window, cx: &mut App) -> Pixels {
        let (icon_size, padding) = self.square_components(window, cx);

        icon_size + padding * 2.
    }
}

impl From<IconName> for Icon {
    fn from(icon: IconName) -> Self {
        Icon::new(icon)
    }
}

/// The source of an icon.
#[derive(Clone)]
enum IconSource {
    /// An SVG embedded in the Raijin binary.
    Embedded(SharedString),
    /// An image file located at the specified path.
    ///
    /// Currently our SVG renderer is missing support for rendering polychrome SVGs.
    ///
    /// In order to support icon themes, we render the icons as images instead.
    External(Arc<Path>),
    /// An SVG not embedded in the Raijin binary.
    ExternalSvg(SharedString),
}

#[derive(Clone, IntoElement, RegisterComponent)]
pub struct Icon {
    source: IconSource,
    color: Color,
    oklch_color: Option<Oklch>,
    size: Rems,
    transformation: Transformation,
    no_flex_shrink: bool,
}

impl Icon {
    pub fn new(icon: IconName) -> Self {
        Self {
            source: IconSource::Embedded(icon.path().into()),
            color: Color::default(),
            oklch_color: None,
            size: IconSize::default().rems(),
            transformation: Transformation::default(),
            no_flex_shrink: false,
        }
    }

    /// Create an icon from a path. Uses a heuristic to determine if it's embedded or external:
    /// - Paths starting with "icons/" are treated as embedded SVGs
    /// - Other paths are treated as external raster images (from icon themes)
    pub fn from_path(path: impl Into<SharedString>) -> Self {
        let path = path.into();
        let source = if path.starts_with("icons/") {
            IconSource::Embedded(path)
        } else {
            IconSource::External(Arc::from(PathBuf::from(path.as_ref())))
        };
        Self {
            source,
            color: Color::default(),
            oklch_color: None,
            size: IconSize::default().rems(),
            transformation: Transformation::default(),
            no_flex_shrink: false,
        }
    }

    pub fn from_external_svg(svg: SharedString) -> Self {
        Self {
            source: IconSource::ExternalSvg(svg),
            color: Color::default(),
            oklch_color: None,
            size: IconSize::default().rems(),
            transformation: Transformation::default(),
            no_flex_shrink: false,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size.rems();
        self
    }

    /// Sets a custom size for the icon, in [`Rems`].
    ///
    /// Not to be exposed outside of the `ui` crate.
    pub(crate) fn custom_size(mut self, size: Rems) -> Self {
        self.size = size;
        self
    }

    /// Rotate the icon by the given angle.
    pub fn rotate(mut self, radians: impl Into<inazuma::Radians>) -> Self {
        self.transformation = Transformation::rotate(radians);
        self
    }

    /// Sets the icon color directly from an [`Oklch`] value.
    ///
    /// This overrides the semantic [`Color`] set via [`Icon::color`].
    pub fn text_color(mut self, color: Oklch) -> Self {
        self.oklch_color = Some(color);
        self
    }

    /// Sets the icon size to `0.75rem` (12px).
    /// Sets the icon size to `0.5rem` (8px).
    pub fn size_2(mut self) -> Self {
        self.size = rems(0.5);
        self
    }

    pub fn size_3(mut self) -> Self {
        self.size = rems(0.75);
        self
    }

    /// Sets the icon size to `0.625rem` (10px).
    pub fn size_2p5(mut self) -> Self {
        self.size = rems(0.625);
        self
    }

    /// Sets the icon size to `0.875rem` (14px).
    pub fn size_3p5(mut self) -> Self {
        self.size = rems(0.875);
        self
    }

    /// Sets the icon size to `1.0rem` (16px).
    pub fn size_4(mut self) -> Self {
        self.size = rems(1.0);
        self
    }

    /// Sets the icon size to `3.0rem` (48px).
    pub fn size_12(mut self) -> Self {
        self.size = rems(3.0);
        self
    }

    /// Prevents the icon from shrinking in a flex container.
    pub fn flex_shrink_0(mut self) -> Self {
        self.no_flex_shrink = true;
        self
    }

    /// Creates an empty icon (no path).
    pub fn empty() -> Self {
        Self {
            source: IconSource::Embedded("".into()),
            color: Color::default(),
            oklch_color: None,
            size: IconSize::default().rems(),
            transformation: Transformation::default(),
            no_flex_shrink: false,
        }
    }
}

impl Transformable for Icon {
    fn transform(mut self, transformation: Transformation) -> Self {
        self.transformation = transformation;
        self
    }
}

impl crate::Sizable for Icon {
    fn with_size(mut self, size: impl Into<crate::Size>) -> Self {
        let size = size.into();
        self.size = match size {
            crate::Size::XSmall => IconSize::XSmall.rems(),
            crate::Size::Small => IconSize::Small.rems(),
            crate::Size::Medium => IconSize::Medium.rems(),
            crate::Size::Large => IconSize::Indicator.rems(),
            crate::Size::Size(px) => inazuma::rems(px.as_f32() / 16.0),
        };
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self.oklch_color.unwrap_or_else(|| self.color.color(cx));
        let no_flex_shrink = self.no_flex_shrink;

        match self.source {
            IconSource::Embedded(path) => svg()
                .with_transformation(self.transformation)
                .size(self.size)
                .flex_none()
                .when(no_flex_shrink, |el| el.flex_shrink_0())
                .path(path)
                .text_color(color)
                .into_any_element(),
            IconSource::ExternalSvg(path) => svg()
                .external_path(path)
                .with_transformation(self.transformation)
                .size(self.size)
                .flex_none()
                .when(no_flex_shrink, |el| el.flex_shrink_0())
                .text_color(color)
                .into_any_element(),
            IconSource::External(path) => img(path)
                .size(self.size)
                .flex_none()
                .when(no_flex_shrink, |el| el.flex_shrink_0())
                .text_color(color)
                .into_any_element(),
        }
    }
}

#[derive(IntoElement)]
pub struct IconWithIndicator {
    icon: Icon,
    indicator: Option<Indicator>,
    indicator_border_color: Option<Oklch>,
}

impl IconWithIndicator {
    pub fn new(icon: Icon, indicator: Option<Indicator>) -> Self {
        Self {
            icon,
            indicator,
            indicator_border_color: None,
        }
    }

    pub fn indicator(mut self, indicator: Option<Indicator>) -> Self {
        self.indicator = indicator;
        self
    }

    pub fn indicator_color(mut self, color: Color) -> Self {
        if let Some(indicator) = self.indicator.as_mut() {
            indicator.color = color;
        }
        self
    }

    pub fn indicator_border_color(mut self, color: Option<Oklch>) -> Self {
        self.indicator_border_color = color;
        self
    }
}

impl RenderOnce for IconWithIndicator {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let indicator_border_color = self
            .indicator_border_color
            .unwrap_or_else(|| cx.theme().colors().elevated_surface);

        div()
            .relative()
            .child(self.icon)
            .when_some(self.indicator, |this, indicator| {
                this.child(
                    div()
                        .absolute()
                        .size_2p5()
                        .border_2()
                        .border_color(indicator_border_color)
                        .rounded_full()
                        .bottom_neg_0p5()
                        .right_neg_0p5()
                        .child(indicator),
                )
            })
    }
}

impl Component for Icon {
    fn scope() -> ComponentScope {
        ComponentScope::Images
    }

    fn description() -> Option<&'static str> {
        Some(
            "A versatile icon component that supports SVG and image-based icons with customizable size, color, and transformations.",
        )
    }

    fn preview(_window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        Some(
            v_flex()
                .gap_6()
                .children(vec![
                    example_group_with_title(
                        "Sizes",
                        vec![single_example(
                            "XSmall, Small, Default, Large",
                            h_flex()
                                .gap_1()
                                .child(Icon::new(IconName::Star).size(IconSize::XSmall))
                                .child(Icon::new(IconName::Star).size(IconSize::Small))
                                .child(Icon::new(IconName::Star))
                                .child(Icon::new(IconName::Star).size(IconSize::XLarge))
                                .into_any_element(),
                        )],
                    ),
                    example_group(vec![single_example(
                        "All Icons",
                        h_flex()
                            .image_cache(inazuma::retain_all("all icons"))
                            .flex_wrap()
                            .gap_2()
                            .children(<IconName as strum::IntoEnumIterator>::iter().map(
                                |icon_name: IconName| {
                                    let name: SharedString = format!("{icon_name:?}").into();
                                    v_flex()
                                        .min_w_0()
                                        .w_24()
                                        .p_1p5()
                                        .gap_2()
                                        .border_1()
                                        .border_color(cx.theme().colors().border_variant)
                                        .bg(cx.theme().colors().element_disabled)
                                        .rounded_sm()
                                        .items_center()
                                        .child(Icon::new(icon_name))
                                        .child(Label::new(name).size(LabelSize::XSmall).truncate())
                                },
                            ))
                            .into_any_element(),
                    )]),
                ])
                .into_any_element(),
        )
    }
}
