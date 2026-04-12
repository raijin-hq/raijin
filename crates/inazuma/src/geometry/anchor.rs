use super::*;

/// The anchor position of an element, extending [`Corner`] with center variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub enum Anchor {
    /// Top-left corner.
    #[default]
    #[serde(rename = "top-left")]
    TopLeft,
    /// Top-center edge.
    #[serde(rename = "top-center")]
    TopCenter,
    /// Top-right corner.
    #[serde(rename = "top-right")]
    TopRight,
    /// Bottom-left corner.
    #[serde(rename = "bottom-left")]
    BottomLeft,
    /// Bottom-center edge.
    #[serde(rename = "bottom-center")]
    BottomCenter,
    /// Bottom-right corner.
    #[serde(rename = "bottom-right")]
    BottomRight,
}

impl Display for Anchor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Anchor::TopLeft => write!(f, "TopLeft"),
            Anchor::TopCenter => write!(f, "TopCenter"),
            Anchor::TopRight => write!(f, "TopRight"),
            Anchor::BottomLeft => write!(f, "BottomLeft"),
            Anchor::BottomCenter => write!(f, "BottomCenter"),
            Anchor::BottomRight => write!(f, "BottomRight"),
        }
    }
}

impl Anchor {
    /// Returns `true` if the anchor is along the top edge.
    #[inline]
    pub fn is_top(&self) -> bool {
        matches!(self, Self::TopLeft | Self::TopCenter | Self::TopRight)
    }

    /// Returns `true` if the anchor is along the bottom edge.
    #[inline]
    pub fn is_bottom(&self) -> bool {
        matches!(
            self,
            Self::BottomLeft | Self::BottomCenter | Self::BottomRight
        )
    }

    /// Returns `true` if the anchor is on the left side.
    #[inline]
    pub fn is_left(&self) -> bool {
        matches!(self, Self::TopLeft | Self::BottomLeft)
    }

    /// Returns `true` if the anchor is on the right side.
    #[inline]
    pub fn is_right(&self) -> bool {
        matches!(self, Self::TopRight | Self::BottomRight)
    }

    /// Returns `true` if the anchor is horizontally centered.
    #[inline]
    pub fn is_center(&self) -> bool {
        matches!(self, Self::TopCenter | Self::BottomCenter)
    }

    /// Returns the vertically mirrored anchor position.
    pub fn swap_vertical(&self) -> Self {
        match self {
            Anchor::TopLeft => Anchor::BottomLeft,
            Anchor::TopCenter => Anchor::BottomCenter,
            Anchor::TopRight => Anchor::BottomRight,
            Anchor::BottomLeft => Anchor::TopLeft,
            Anchor::BottomCenter => Anchor::TopCenter,
            Anchor::BottomRight => Anchor::TopRight,
        }
    }

    /// Returns the horizontally mirrored anchor position.
    pub fn swap_horizontal(&self) -> Self {
        match self {
            Anchor::TopLeft => Anchor::TopRight,
            Anchor::TopCenter => Anchor::TopCenter,
            Anchor::TopRight => Anchor::TopLeft,
            Anchor::BottomLeft => Anchor::BottomRight,
            Anchor::BottomCenter => Anchor::BottomCenter,
            Anchor::BottomRight => Anchor::BottomLeft,
        }
    }

    /// Returns the anchor on the opposite side along the given axis.
    pub fn other_side_corner_along(&self, axis: Axis) -> Anchor {
        match axis {
            Axis::Vertical => self.swap_vertical(),
            Axis::Horizontal => self.swap_horizontal(),
        }
    }
}

impl From<Corner> for Anchor {
    fn from(corner: Corner) -> Self {
        match corner {
            Corner::TopLeft => Anchor::TopLeft,
            Corner::TopRight => Anchor::TopRight,
            Corner::BottomLeft => Anchor::BottomLeft,
            Corner::BottomRight => Anchor::BottomRight,
        }
    }
}

impl From<Anchor> for Corner {
    fn from(anchor: Anchor) -> Self {
        match anchor {
            Anchor::TopLeft => Corner::TopLeft,
            Anchor::TopRight => Corner::TopRight,
            Anchor::BottomLeft => Corner::BottomLeft,
            Anchor::BottomRight => Corner::BottomRight,
            Anchor::TopCenter => Corner::TopLeft,
            Anchor::BottomCenter => Corner::BottomLeft,
        }
    }
}
