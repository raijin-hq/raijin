use crate::{
    AbsoluteLength, App, BackgroundTag, BorderStyle, Bounds, ContentMask, Corners,
    CornersRefinement, CursorStyle, DefiniteLength, DevicePixels, Edges, EdgesRefinement,
    GridLocation, Hsla, Length, Pixels, Point, PointRefinement, Size, SizeRefinement, Styled,
    Window, point, quad, size,
};
use refineable::Refineable;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    AlignContent, AlignItems, AlignSelf, BoxShadow, Display, Fill, FlexDirection, FlexWrap,
    JustifyContent, Overflow, Position, TextStyleRefinement, TextStyleRefinementRefinement,
    Visibility,
};

/// Use this struct for interfacing with the 'debug_below' styling from your own elements.
/// If a parent element has this style set on it, then this struct will be set as a global in
/// GPUI.
#[cfg(debug_assertions)]
pub struct DebugBelow;

#[cfg(debug_assertions)]
impl crate::Global for DebugBelow {}

/// How to fit the image into the bounds of the element.
pub enum ObjectFit {
    /// The image will be stretched to fill the bounds of the element.
    Fill,
    /// The image will be scaled to fit within the bounds of the element.
    Contain,
    /// The image will be scaled to cover the bounds of the element.
    Cover,
    /// The image will be scaled down to fit within the bounds of the element.
    ScaleDown,
    /// The image will maintain its original size.
    None,
}

impl ObjectFit {
    /// Get the bounds of the image within the given bounds.
    pub fn get_bounds(
        &self,
        bounds: Bounds<Pixels>,
        image_size: Size<DevicePixels>,
    ) -> Bounds<Pixels> {
        let image_size = image_size.map(|dimension| Pixels::from(u32::from(dimension)));
        let image_ratio = image_size.width / image_size.height;
        let bounds_ratio = bounds.size.width / bounds.size.height;

        match self {
            ObjectFit::Fill => bounds,
            ObjectFit::Contain => {
                let new_size = if bounds_ratio > image_ratio {
                    size(
                        image_size.width * (bounds.size.height / image_size.height),
                        bounds.size.height,
                    )
                } else {
                    size(
                        bounds.size.width,
                        image_size.height * (bounds.size.width / image_size.width),
                    )
                };

                Bounds {
                    origin: point(
                        bounds.origin.x + (bounds.size.width - new_size.width) / 2.0,
                        bounds.origin.y + (bounds.size.height - new_size.height) / 2.0,
                    ),
                    size: new_size,
                }
            }
            ObjectFit::ScaleDown => {
                // Check if the image is larger than the bounds in either dimension.
                if image_size.width > bounds.size.width || image_size.height > bounds.size.height {
                    // If the image is larger, use the same logic as Contain to scale it down.
                    let new_size = if bounds_ratio > image_ratio {
                        size(
                            image_size.width * (bounds.size.height / image_size.height),
                            bounds.size.height,
                        )
                    } else {
                        size(
                            bounds.size.width,
                            image_size.height * (bounds.size.width / image_size.width),
                        )
                    };

                    Bounds {
                        origin: point(
                            bounds.origin.x + (bounds.size.width - new_size.width) / 2.0,
                            bounds.origin.y + (bounds.size.height - new_size.height) / 2.0,
                        ),
                        size: new_size,
                    }
                } else {
                    // If the image is smaller than or equal to the container, display it at its original size,
                    // centered within the container.
                    let original_size = size(image_size.width, image_size.height);
                    Bounds {
                        origin: point(
                            bounds.origin.x + (bounds.size.width - original_size.width) / 2.0,
                            bounds.origin.y + (bounds.size.height - original_size.height) / 2.0,
                        ),
                        size: original_size,
                    }
                }
            }
            ObjectFit::Cover => {
                let new_size = if bounds_ratio > image_ratio {
                    size(
                        bounds.size.width,
                        image_size.height * (bounds.size.width / image_size.width),
                    )
                } else {
                    size(
                        image_size.width * (bounds.size.height / image_size.height),
                        bounds.size.height,
                    )
                };

                Bounds {
                    origin: point(
                        bounds.origin.x + (bounds.size.width - new_size.width) / 2.0,
                        bounds.origin.y + (bounds.size.height - new_size.height) / 2.0,
                    ),
                    size: new_size,
                }
            }
            ObjectFit::None => Bounds {
                origin: bounds.origin,
                size: image_size,
            },
        }
    }
}

/// The minimum size of a column or row in a grid layout
#[derive(
    Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, JsonSchema, Serialize, Deserialize,
)]
pub enum TemplateColumnMinSize {
    /// The column size may be 0
    #[default]
    Zero,
    /// The column size can be determined by the min content
    MinContent,
    /// The column size can be determined by the max content
    MaxContent,
}

/// A simplified representation of the grid-template-* value
#[derive(
    Copy,
    Clone,
    Refineable,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Default,
    JsonSchema,
    Serialize,
    Deserialize,
)]
pub struct GridTemplate {
    /// How this template directive should be repeated
    pub repeat: u16,
    /// The minimum size in the repeat(<>, minmax(_, 1fr)) equation
    pub min_size: TemplateColumnMinSize,
}

/// The CSS styling that can be applied to an element via the `Styled` trait
#[derive(Clone, Refineable, Debug)]
#[refineable(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Style {
    /// What layout strategy should be used?
    pub display: Display,

    /// Should the element be painted on screen?
    pub visibility: Visibility,

    // Overflow properties
    /// How children overflowing their container should affect layout
    #[refineable]
    pub overflow: Point<Overflow>,
    /// How much space (in points) should be reserved for the scrollbars of `Overflow::Scroll` and `Overflow::Auto` nodes.
    pub scrollbar_width: AbsoluteLength,
    /// Whether both x and y axis should be scrollable at the same time.
    pub allow_concurrent_scroll: bool,
    /// Whether scrolling should be restricted to the axis indicated by the mouse wheel.
    ///
    /// This means that:
    /// - The mouse wheel alone will only ever scroll the Y axis.
    /// - Holding `Shift` and using the mouse wheel will scroll the X axis.
    ///
    /// ## Motivation
    ///
    /// On the web when scrolling with the mouse wheel, scrolling up and down will always scroll the Y axis, even when
    /// the mouse is over a horizontally-scrollable element.
    ///
    /// The only way to scroll horizontally is to hold down `Shift` while scrolling, which then changes the scroll axis
    /// to the X axis.
    ///
    /// Currently, GPUI operates differently from the web in that it will scroll an element in either the X or Y axis
    /// when scrolling with just the mouse wheel. This causes problems when scrolling in a vertical list that contains
    /// horizontally-scrollable elements, as when you get to the horizontally-scrollable elements the scroll will be
    /// hijacked.
    ///
    /// Ideally we would match the web's behavior and not have a need for this, but right now we're adding this opt-in
    /// style property to limit the potential blast radius.
    pub restrict_scroll_to_axis: bool,

    // Position properties
    /// What should the `position` value of this struct use as a base offset?
    pub position: Position,
    /// How should the position of this element be tweaked relative to the layout defined?
    #[refineable]
    pub inset: Edges<Length>,

    // Size properties
    /// Sets the initial size of the item
    #[refineable]
    pub size: Size<Length>,
    /// Controls the minimum size of the item
    #[refineable]
    pub min_size: Size<Length>,
    /// Controls the maximum size of the item
    #[refineable]
    pub max_size: Size<Length>,
    /// Sets the preferred aspect ratio for the item. The ratio is calculated as width divided by height.
    pub aspect_ratio: Option<f32>,

    // Spacing Properties
    /// How large should the margin be on each side?
    #[refineable]
    pub margin: Edges<Length>,
    /// How large should the padding be on each side?
    #[refineable]
    pub padding: Edges<DefiniteLength>,
    /// How large should the border be on each side?
    #[refineable]
    pub border_widths: Edges<AbsoluteLength>,

    // Alignment properties
    /// How this node's children aligned in the cross/block axis?
    pub align_items: Option<AlignItems>,
    /// How this node should be aligned in the cross/block axis. Falls back to the parents [`AlignItems`] if not set
    pub align_self: Option<AlignSelf>,
    /// How should content contained within this item be aligned in the cross/block axis
    pub align_content: Option<AlignContent>,
    /// How should contained within this item be aligned in the main/inline axis
    pub justify_content: Option<JustifyContent>,
    /// How large should the gaps between items in a flex container be?
    #[refineable]
    pub gap: Size<DefiniteLength>,

    // Flexbox properties
    /// Which direction does the main axis flow in?
    pub flex_direction: FlexDirection,
    /// Should elements wrap, or stay in a single line?
    pub flex_wrap: FlexWrap,
    /// Sets the initial main axis size of the item
    pub flex_basis: Length,
    /// The relative rate at which this item grows when it is expanding to fill space, 0.0 is the default value, and this value must be positive.
    pub flex_grow: f32,
    /// The relative rate at which this item shrinks when it is contracting to fit into space, 1.0 is the default value, and this value must be positive.
    pub flex_shrink: f32,

    /// The fill color of this element
    pub background: Option<Fill>,

    /// Per-side border colors of this element.
    pub border_colors: Edges<Option<Hsla>>,

    /// The border style of this element
    pub border_style: BorderStyle,

    /// The radius of the corners of this element
    #[refineable]
    pub corner_radii: Corners<AbsoluteLength>,

    /// Box shadow of the element
    pub box_shadow: Vec<BoxShadow>,

    /// The text style of this element
    #[refineable]
    pub text: TextStyleRefinement,

    /// The mouse cursor style shown when the mouse pointer is over an element.
    pub mouse_cursor: Option<CursorStyle>,

    /// The opacity of this element
    pub opacity: Option<f32>,

    /// The grid columns of this element
    /// Roughly equivalent to the Tailwind `grid-cols-<number>`
    pub grid_cols: Option<GridTemplate>,

    /// The row span of this element
    /// Equivalent to the Tailwind `grid-rows-<number>`
    pub grid_rows: Option<GridTemplate>,

    /// The grid location of this element
    pub grid_location: Option<GridLocation>,

    /// Whether to draw a red debugging outline around this element
    #[cfg(debug_assertions)]
    pub debug: bool,

    /// Whether to draw a red debugging outline around this element and all of its conforming children
    #[cfg(debug_assertions)]
    pub debug_below: bool,
}

impl Styled for StyleRefinement {
    fn style(&mut self) -> &mut StyleRefinement {
        self
    }
}

impl StyleRefinement {
    /// The grid location of this element
    pub fn grid_location_mut(&mut self) -> &mut GridLocation {
        self.grid_location.get_or_insert_default()
    }
}

impl Style {
    /// Returns true if the style is visible and the background is opaque.
    pub fn has_opaque_background(&self) -> bool {
        self.background
            .as_ref()
            .is_some_and(|fill| fill.color().is_some_and(|color| !color.is_transparent()))
    }

    /// Get the text style in this element style.
    pub fn text_style(&self) -> Option<&TextStyleRefinement> {
        if self.text.is_some() {
            Some(&self.text)
        } else {
            None
        }
    }

    /// Get the content mask for this element style, based on the given bounds.
    /// If the element does not hide its overflow, this will return `None`.
    pub fn overflow_mask(
        &self,
        bounds: Bounds<Pixels>,
        rem_size: Pixels,
    ) -> Option<ContentMask<Pixels>> {
        match self.overflow {
            Point {
                x: Overflow::Visible,
                y: Overflow::Visible,
            } => None,
            _ => {
                let mut min = bounds.origin;
                let mut max = bounds.bottom_right();

                let has_visible_border = [
                    &self.border_colors.top,
                    &self.border_colors.right,
                    &self.border_colors.bottom,
                    &self.border_colors.left,
                ]
                .iter()
                .any(|c| c.is_some_and(|color| !color.is_transparent()));
                if has_visible_border {
                    min.x += self.border_widths.left.to_pixels(rem_size);
                    max.x -= self.border_widths.right.to_pixels(rem_size);
                    min.y += self.border_widths.top.to_pixels(rem_size);
                    max.y -= self.border_widths.bottom.to_pixels(rem_size);
                }

                let bounds = match (
                    self.overflow.x == Overflow::Visible,
                    self.overflow.y == Overflow::Visible,
                ) {
                    // x and y both visible
                    (true, true) => return None,
                    // x visible, y hidden
                    (true, false) => Bounds::from_corners(
                        point(min.x, bounds.origin.y),
                        point(max.x, bounds.bottom_right().y),
                    ),
                    // x hidden, y visible
                    (false, true) => Bounds::from_corners(
                        point(bounds.origin.x, min.y),
                        point(bounds.bottom_right().x, max.y),
                    ),
                    // both hidden
                    (false, false) => Bounds::from_corners(min, max),
                };

                Some(ContentMask { bounds })
            }
        }
    }

    /// Paints the background of an element styled with this style.
    pub fn paint(
        &self,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut App,
        continuation: impl FnOnce(&mut Window, &mut App),
    ) {
        #[cfg(debug_assertions)]
        if self.debug_below {
            cx.set_global(DebugBelow)
        }

        #[cfg(debug_assertions)]
        if self.debug || cx.has_global::<DebugBelow>() {
            window.paint_quad(crate::outline(bounds, crate::red(), BorderStyle::default()));
        }

        let rem_size = window.rem_size();
        let corner_radii = self
            .corner_radii
            .to_pixels(rem_size)
            .clamp_radii_for_quad_size(bounds.size);

        window.paint_shadows(bounds, corner_radii, &self.box_shadow);

        let background_color = self.background.as_ref().and_then(Fill::color);
        if background_color.is_some_and(|color| !color.is_transparent()) {
            let mut bg_border = match background_color {
                Some(color) => match color.tag {
                    BackgroundTag::Solid
                    | BackgroundTag::PatternSlash
                    | BackgroundTag::Checkerboard => color.solid,

                    BackgroundTag::LinearGradient => color
                        .colors
                        .first()
                        .map(|stop| stop.color)
                        .unwrap_or_default(),
                },
                None => Hsla::default(),
            };
            bg_border.a = 0.;
            let transparent_edges = Edges { top: bg_border, right: bg_border, bottom: bg_border, left: bg_border };
            window.paint_quad(quad(
                bounds,
                corner_radii,
                background_color.unwrap_or_default(),
                Edges::default(),
                transparent_edges,
                self.border_style,
            ));
        }

        continuation(window, cx);

        if self.is_border_visible() {
            let border_widths = self.border_widths.to_pixels(rem_size);
            let max_border_width = border_widths.max();
            let max_corner_radius = corner_radii.max();
            let zero_size = Size {
                width: Pixels::ZERO,
                height: Pixels::ZERO,
            };

            let mut top_bounds = Bounds::from_corners(
                bounds.origin,
                bounds.top_right() + point(Pixels::ZERO, max_border_width.max(max_corner_radius)),
            );
            top_bounds.size = top_bounds.size.max(&zero_size);
            let mut bottom_bounds = Bounds::from_corners(
                bounds.bottom_left() - point(Pixels::ZERO, max_border_width.max(max_corner_radius)),
                bounds.bottom_right(),
            );
            bottom_bounds.size = bottom_bounds.size.max(&zero_size);
            let mut left_bounds = Bounds::from_corners(
                top_bounds.bottom_left(),
                bottom_bounds.origin + point(max_border_width, Pixels::ZERO),
            );
            left_bounds.size = left_bounds.size.max(&zero_size);
            let mut right_bounds = Bounds::from_corners(
                top_bounds.bottom_right() - point(max_border_width, Pixels::ZERO),
                bottom_bounds.top_right(),
            );
            right_bounds.size = right_bounds.size.max(&zero_size);

            let resolved_border_colors = Edges {
                top: self.border_colors.top.unwrap_or_default(),
                right: self.border_colors.right.unwrap_or_default(),
                bottom: self.border_colors.bottom.unwrap_or_default(),
                left: self.border_colors.left.unwrap_or_default(),
            };
            let mut background_hsla = resolved_border_colors.top;
            background_hsla.a = 0.;
            let quad = quad(
                bounds,
                corner_radii,
                background_hsla,
                border_widths,
                resolved_border_colors,
                self.border_style,
            );

            window.with_content_mask(Some(ContentMask { bounds: top_bounds }), |window| {
                window.paint_quad(quad.clone());
            });
            window.with_content_mask(
                Some(ContentMask {
                    bounds: right_bounds,
                }),
                |window| {
                    window.paint_quad(quad.clone());
                },
            );
            window.with_content_mask(
                Some(ContentMask {
                    bounds: bottom_bounds,
                }),
                |window| {
                    window.paint_quad(quad.clone());
                },
            );
            window.with_content_mask(
                Some(ContentMask {
                    bounds: left_bounds,
                }),
                |window| {
                    window.paint_quad(quad);
                },
            );
        }

        #[cfg(debug_assertions)]
        if self.debug_below {
            cx.remove_global::<DebugBelow>();
        }
    }

    fn is_border_visible(&self) -> bool {
        [
            &self.border_colors.top,
            &self.border_colors.right,
            &self.border_colors.bottom,
            &self.border_colors.left,
        ]
        .iter()
        .any(|c| c.is_some_and(|color| !color.is_transparent()))
            && self.border_widths.any(|length| !length.is_zero())
    }
}

impl Default for Style {
    fn default() -> Self {
        Style {
            display: Display::Block,
            visibility: Visibility::Visible,
            overflow: Point {
                x: Overflow::Visible,
                y: Overflow::Visible,
            },
            allow_concurrent_scroll: false,
            restrict_scroll_to_axis: false,
            scrollbar_width: AbsoluteLength::default(),
            position: Position::Relative,
            inset: Edges::auto(),
            margin: Edges::<Length>::zero(),
            padding: Edges::<DefiniteLength>::zero(),
            border_widths: Edges::<AbsoluteLength>::zero(),
            size: Size::auto(),
            min_size: Size::auto(),
            max_size: Size::auto(),
            aspect_ratio: None,
            gap: Size::default(),
            // Alignment
            align_items: None,
            align_self: None,
            align_content: None,
            justify_content: None,
            // Flexbox
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Length::Auto,
            background: None,
            border_colors: Edges::default(),
            border_style: BorderStyle::default(),
            corner_radii: Corners::default(),
            box_shadow: Default::default(),
            text: TextStyleRefinement::default(),
            mouse_cursor: None,
            opacity: None,
            grid_rows: None,
            grid_cols: None,
            grid_location: None,

            #[cfg(debug_assertions)]
            debug: false,
            #[cfg(debug_assertions)]
            debug_below: false,
        }
    }
}
