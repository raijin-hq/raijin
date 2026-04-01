use crate::{
    Background, BorderStyle, Bounds, Corners, Edges, Hsla, Pixels, transparent_black,
};

/// A rectangle to be rendered in the window at the given position and size.
/// Passed as an argument [`Window::paint_quad`].
#[derive(Clone)]
pub struct PaintQuad {
    /// The bounds of the quad within the window.
    pub bounds: Bounds<Pixels>,
    /// The radii of the quad's corners.
    pub corner_radii: Corners<Pixels>,
    /// The background color of the quad.
    pub background: Background,
    /// The widths of the quad's borders.
    pub border_widths: Edges<Pixels>,
    /// Per-side border colors (top, right, bottom, left).
    pub border_colors: Edges<Hsla>,
    /// The style of the quad's borders.
    pub border_style: BorderStyle,
}

impl PaintQuad {
    /// Sets the corner radii of the quad.
    pub fn corner_radii(self, corner_radii: impl Into<Corners<Pixels>>) -> Self {
        PaintQuad {
            corner_radii: corner_radii.into(),
            ..self
        }
    }

    /// Sets the border widths of the quad.
    pub fn border_widths(self, border_widths: impl Into<Edges<Pixels>>) -> Self {
        PaintQuad {
            border_widths: border_widths.into(),
            ..self
        }
    }

    /// Sets the border color for all sides.
    pub fn border_color(self, border_color: impl Into<Hsla>) -> Self {
        let c = border_color.into();
        PaintQuad {
            border_colors: Edges { top: c, right: c, bottom: c, left: c },
            ..self
        }
    }

    /// Sets the border color for a specific side.
    pub fn border_left_color(mut self, color: impl Into<Hsla>) -> Self {
        self.border_colors.left = color.into();
        self
    }

    /// Sets the border color for a specific side.
    pub fn border_right_color(mut self, color: impl Into<Hsla>) -> Self {
        self.border_colors.right = color.into();
        self
    }

    /// Sets the border color for a specific side.
    pub fn border_top_color(mut self, color: impl Into<Hsla>) -> Self {
        self.border_colors.top = color.into();
        self
    }

    /// Sets the border color for a specific side.
    pub fn border_bottom_color(mut self, color: impl Into<Hsla>) -> Self {
        self.border_colors.bottom = color.into();
        self
    }

    /// Sets the background color of the quad.
    pub fn background(self, background: impl Into<Background>) -> Self {
        PaintQuad {
            background: background.into(),
            ..self
        }
    }
}

/// Creates a quad with the given parameters.
pub fn quad(
    bounds: Bounds<Pixels>,
    corner_radii: impl Into<Corners<Pixels>>,
    background: impl Into<Background>,
    border_widths: impl Into<Edges<Pixels>>,
    border_colors: impl Into<Edges<Hsla>>,
    border_style: BorderStyle,
) -> PaintQuad {
    PaintQuad {
        bounds,
        corner_radii: corner_radii.into(),
        background: background.into(),
        border_widths: border_widths.into(),
        border_colors: border_colors.into(),
        border_style,
    }
}

/// Creates a filled quad with the given bounds and background color.
pub fn fill(bounds: impl Into<Bounds<Pixels>>, background: impl Into<Background>) -> PaintQuad {
    let transparent = transparent_black();
    PaintQuad {
        bounds: bounds.into(),
        corner_radii: (0.).into(),
        background: background.into(),
        border_widths: (0.).into(),
        border_colors: Edges { top: transparent, right: transparent, bottom: transparent, left: transparent },
        border_style: BorderStyle::default(),
    }
}

/// Creates a rectangle outline with the given bounds, border color, and a 1px border width
pub fn outline(
    bounds: impl Into<Bounds<Pixels>>,
    border_color: impl Into<Hsla>,
    border_style: BorderStyle,
) -> PaintQuad {
    let c = border_color.into();
    PaintQuad {
        bounds: bounds.into(),
        corner_radii: (0.).into(),
        background: transparent_black().into(),
        border_widths: (1.).into(),
        border_colors: Edges { top: c, right: c, bottom: c, left: c },
        border_style,
    }
}
