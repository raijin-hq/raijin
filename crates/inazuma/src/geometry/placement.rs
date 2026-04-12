use super::*;

/// A placement direction for an element (4 edges).
///
/// See also: [`Side`] for left/right only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Placement {
    /// Place at the top edge.
    #[serde(rename = "top")]
    Top,
    /// Place at the bottom edge.
    #[serde(rename = "bottom")]
    Bottom,
    /// Place at the left edge.
    #[serde(rename = "left")]
    Left,
    /// Place at the right edge.
    #[serde(rename = "right")]
    Right,
}

impl Display for Placement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Placement::Top => write!(f, "Top"),
            Placement::Bottom => write!(f, "Bottom"),
            Placement::Left => write!(f, "Left"),
            Placement::Right => write!(f, "Right"),
        }
    }
}

impl Placement {
    /// Returns `true` if this placement is horizontal (left or right).
    #[inline]
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Placement::Left | Placement::Right)
    }

    /// Returns `true` if this placement is vertical (top or bottom).
    #[inline]
    pub fn is_vertical(&self) -> bool {
        matches!(self, Placement::Top | Placement::Bottom)
    }

    /// Returns the axis along which this placement is oriented.
    #[inline]
    pub fn axis(&self) -> Axis {
        match self {
            Placement::Top | Placement::Bottom => Axis::Vertical,
            Placement::Left | Placement::Right => Axis::Horizontal,
        }
    }
}

/// A side of an element (left or right).
///
/// See also: [`Placement`] for all 4 edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    /// The left side.
    #[serde(rename = "left")]
    Left,
    /// The right side.
    #[serde(rename = "right")]
    Right,
}

impl Side {
    /// Returns `true` if this is the left side.
    #[inline]
    pub fn is_left(&self) -> bool {
        matches!(self, Self::Left)
    }

    /// Returns `true` if this is the right side.
    #[inline]
    pub fn is_right(&self) -> bool {
        matches!(self, Self::Right)
    }
}

/// Extension trait for [`Axis`] with convenience methods.
pub trait AxisExt {
    /// Returns `true` if the axis is horizontal.
    fn is_horizontal(self) -> bool;
    /// Returns `true` if the axis is vertical.
    fn is_vertical(self) -> bool;
}

impl AxisExt for Axis {
    #[inline]
    fn is_horizontal(self) -> bool {
        self == Axis::Horizontal
    }

    #[inline]
    fn is_vertical(self) -> bool {
        self == Axis::Vertical
    }
}

/// Extension trait for [`Length`] with conversion methods.
pub trait LengthExt {
    /// Converts this length to pixels, returning `None` for `Auto`.
    fn to_pixels(&self, base_size: AbsoluteLength, rem_size: Pixels) -> Option<Pixels>;
}

impl LengthExt for Length {
    fn to_pixels(&self, base_size: AbsoluteLength, rem_size: Pixels) -> Option<Pixels> {
        match self {
            Length::Auto => None,
            Length::Definite(len) => Some(len.to_pixels(base_size, rem_size)),
        }
    }
}
