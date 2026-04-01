use super::*;

/// Represents a rectangular area in a 2D space with an origin point and a size.
///
/// The `Bounds` struct is generic over a type `T` which represents the type of the coordinate system.
/// The origin is represented as a `Point<T>` which defines the top left corner of the rectangle,
/// and the size is represented as a `Size<T>` which defines the width and height of the rectangle.
///
/// # Examples
///
/// ```
/// # use inazuma::{Bounds, Point, Size};
/// let origin = Point { x: 0, y: 0 };
/// let size = Size { width: 10, height: 20 };
/// let bounds = Bounds::new(origin, size);
///
/// assert_eq!(bounds.origin, origin);
/// assert_eq!(bounds.size, size);
/// ```
#[derive(Refineable, Copy, Clone, Default, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
#[refineable(Debug)]
#[repr(C)]
pub struct Bounds<T: Clone + Debug + Default + PartialEq> {
    /// The origin point of this area.
    pub origin: Point<T>,
    /// The size of the rectangle.
    pub size: Size<T>,
}

/// Create a bounds with the given origin and size
pub fn bounds<T: Clone + Debug + Default + PartialEq>(
    origin: Point<T>,
    size: Size<T>,
) -> Bounds<T> {
    Bounds { origin, size }
}

impl Bounds<Pixels> {
    /// Generate a centered bounds for the given display or primary display if none is provided
    pub fn centered(display_id: Option<DisplayId>, size: Size<Pixels>, cx: &App) -> Self {
        let display = display_id
            .and_then(|id| cx.find_display(id))
            .or_else(|| cx.primary_display());

        display
            .map(|display| Bounds::centered_at(display.bounds().center(), size))
            .unwrap_or_else(|| Bounds {
                origin: point(px(0.), px(0.)),
                size,
            })
    }

    /// Generate maximized bounds for the given display or primary display if none is provided
    pub fn maximized(display_id: Option<DisplayId>, cx: &App) -> Self {
        let display = display_id
            .and_then(|id| cx.find_display(id))
            .or_else(|| cx.primary_display());

        display
            .map(|display| display.bounds())
            .unwrap_or_else(|| Bounds {
                origin: point(px(0.), px(0.)),
                size: size(px(1024.), px(768.)),
            })
    }
}

impl<T> Bounds<T>
where
    T: Clone + Debug + Default + PartialEq,
{
    /// Creates a new `Bounds` with the specified origin and size.
    ///
    /// # Arguments
    ///
    /// * `origin` - A `Point<T>` representing the origin of the bounds.
    /// * `size` - A `Size<T>` representing the size of the bounds.
    ///
    /// # Returns
    ///
    /// Returns a `Bounds<T>` that has the given origin and size.
    pub fn new(origin: Point<T>, size: Size<T>) -> Self {
        Bounds { origin, size }
    }
}

impl<T> Bounds<T>
where
    T: Sub<Output = T> + Clone + Debug + Default + PartialEq,
{
    /// Constructs a `Bounds` from two corner points: the top left and bottom right corners.
    ///
    /// This function calculates the origin and size of the `Bounds` based on the provided corner points.
    /// The origin is set to the top left corner, and the size is determined by the difference between
    /// the x and y coordinates of the bottom right and top left points.
    ///
    /// # Arguments
    ///
    /// * `top_left` - A `Point<T>` representing the top left corner of the rectangle.
    /// * `bottom_right` - A `Point<T>` representing the bottom right corner of the rectangle.
    ///
    /// # Returns
    ///
    /// Returns a `Bounds<T>` that encompasses the area defined by the two corner points.
    ///
    /// # Examples
    ///
    /// ```
    /// # use inazuma::{Bounds, Point};
    /// let top_left = Point { x: 0, y: 0 };
    /// let bottom_right = Point { x: 10, y: 10 };
    /// let bounds = Bounds::from_corners(top_left, bottom_right);
    ///
    /// assert_eq!(bounds.origin, top_left);
    /// assert_eq!(bounds.size.width, 10);
    /// assert_eq!(bounds.size.height, 10);
    /// ```
    pub fn from_corners(top_left: Point<T>, bottom_right: Point<T>) -> Self {
        let origin = Point {
            x: top_left.x.clone(),
            y: top_left.y.clone(),
        };
        let size = Size {
            width: bottom_right.x - top_left.x,
            height: bottom_right.y - top_left.y,
        };
        Bounds { origin, size }
    }

    /// Constructs a `Bounds` from a corner point and size. The specified corner will be placed at
    /// the specified origin.
    pub fn from_corner_and_size(corner: Corner, origin: Point<T>, size: Size<T>) -> Bounds<T> {
        let origin = match corner {
            Corner::TopLeft => origin,
            Corner::TopRight => Point {
                x: origin.x - size.width.clone(),
                y: origin.y,
            },
            Corner::BottomLeft => Point {
                x: origin.x,
                y: origin.y - size.height.clone(),
            },
            Corner::BottomRight => Point {
                x: origin.x - size.width.clone(),
                y: origin.y - size.height.clone(),
            },
        };

        Bounds { origin, size }
    }
}

impl<T> Bounds<T>
where
    T: Sub<T, Output = T> + Half + Clone + Debug + Default + PartialEq,
{
    /// Creates a new bounds centered at the given point.
    pub fn centered_at(center: Point<T>, size: Size<T>) -> Self {
        let origin = Point {
            x: center.x - size.width.half(),
            y: center.y - size.height.half(),
        };
        Self::new(origin, size)
    }
}

impl<T> Bounds<T>
where
    T: PartialOrd + Add<T, Output = T> + Clone + Debug + Default + PartialEq,
{
    /// Checks if this `Bounds` intersects with another `Bounds`.
    ///
    /// Two `Bounds` instances intersect if they overlap in the 2D space they occupy.
    /// This method checks if there is any overlapping area between the two bounds.
    ///
    /// # Arguments
    ///
    /// * `other` - A reference to another `Bounds` to check for intersection with.
    ///
    /// # Returns
    ///
    /// Returns `true` if there is any intersection between the two bounds, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// # use inazuma::{Bounds, Point, Size};
    /// let bounds1 = Bounds {
    ///     origin: Point { x: 0, y: 0 },
    ///     size: Size { width: 10, height: 10 },
    /// };
    /// let bounds2 = Bounds {
    ///     origin: Point { x: 5, y: 5 },
    ///     size: Size { width: 10, height: 10 },
    /// };
    /// let bounds3 = Bounds {
    ///     origin: Point { x: 20, y: 20 },
    ///     size: Size { width: 10, height: 10 },
    /// };
    ///
    /// assert_eq!(bounds1.intersects(&bounds2), true); // Overlapping bounds
    /// assert_eq!(bounds1.intersects(&bounds3), false); // Non-overlapping bounds
    /// ```
    pub fn intersects(&self, other: &Bounds<T>) -> bool {
        let my_lower_right = self.bottom_right();
        let their_lower_right = other.bottom_right();

        self.origin.x < their_lower_right.x
            && my_lower_right.x > other.origin.x
            && self.origin.y < their_lower_right.y
            && my_lower_right.y > other.origin.y
    }
}

impl<T> Bounds<T>
where
    T: Add<T, Output = T> + Half + Clone + Debug + Default + PartialEq,
{
    /// Returns the center point of the bounds.
    ///
    /// Calculates the center by taking the origin's x and y coordinates and adding half the width and height
    /// of the bounds, respectively. The center is represented as a `Point<T>` where `T` is the type of the
    /// coordinate system.
    ///
    /// # Returns
    ///
    /// A `Point<T>` representing the center of the bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use inazuma::{Bounds, Point, Size};
    /// let bounds = Bounds {
    ///     origin: Point { x: 0, y: 0 },
    ///     size: Size { width: 10, height: 20 },
    /// };
    /// let center = bounds.center();
    /// assert_eq!(center, Point { x: 5, y: 10 });
    /// ```
    pub fn center(&self) -> Point<T> {
        Point {
            x: self.origin.x.clone() + self.size.width.clone().half(),
            y: self.origin.y.clone() + self.size.height.clone().half(),
        }
    }
}

impl<T> Bounds<T>
where
    T: Add<T, Output = T> + Clone + Debug + Default + PartialEq,
{
    /// Calculates the half perimeter of a rectangle defined by the bounds.
    ///
    /// The half perimeter is calculated as the sum of the width and the height of the rectangle.
    /// This method is generic over the type `T` which must implement the `Sub` trait to allow
    /// calculation of the width and height from the bounds' origin and size, as well as the `Add` trait
    /// to sum the width and height for the half perimeter.
    ///
    /// # Examples
    ///
    /// ```
    /// # use inazuma::{Bounds, Point, Size};
    /// let bounds = Bounds {
    ///     origin: Point { x: 0, y: 0 },
    ///     size: Size { width: 10, height: 20 },
    /// };
    /// let half_perimeter = bounds.half_perimeter();
    /// assert_eq!(half_perimeter, 30);
    /// ```
    pub fn half_perimeter(&self) -> T {
        self.size.width.clone() + self.size.height.clone()
    }
}

impl<T> Bounds<T>
where
    T: Add<T, Output = T> + Sub<Output = T> + Clone + Debug + Default + PartialEq,
{
    /// Dilates the bounds by a specified amount in all directions.
    ///
    /// This method expands the bounds by the given `amount`, increasing the size
    /// and adjusting the origin so that the bounds grow outwards equally in all directions.
    /// The resulting bounds will have its width and height increased by twice the `amount`
    /// (since it grows in both directions), and the origin will be moved by `-amount`
    /// in both the x and y directions.
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount by which to dilate the bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use inazuma::{Bounds, Point, Size};
    /// let mut bounds = Bounds {
    ///     origin: Point { x: 10, y: 10 },
    ///     size: Size { width: 10, height: 10 },
    /// };
    /// let expanded_bounds = bounds.dilate(5);
    /// assert_eq!(expanded_bounds, Bounds {
    ///     origin: Point { x: 5, y: 5 },
    ///     size: Size { width: 20, height: 20 },
    /// });
    /// ```
    #[must_use]
    pub fn dilate(&self, amount: T) -> Bounds<T> {
        let double_amount = amount.clone() + amount.clone();
        Bounds {
            origin: self.origin.clone() - point(amount.clone(), amount),
            size: self.size.clone() + size(double_amount.clone(), double_amount),
        }
    }

    /// Extends the bounds different amounts in each direction.
    #[must_use]
    pub fn extend(&self, amount: Edges<T>) -> Bounds<T> {
        Bounds {
            origin: self.origin.clone() - point(amount.left.clone(), amount.top.clone()),
            size: self.size.clone()
                + size(
                    amount.left.clone() + amount.right.clone(),
                    amount.top.clone() + amount.bottom,
                ),
        }
    }
}

impl<T> Bounds<T>
where
    T: Add<T, Output = T>
        + Sub<T, Output = T>
        + Neg<Output = T>
        + Clone
        + Debug
        + Default
        + PartialEq,
{
    /// Inset the bounds by a specified amount. Equivalent to `dilate` with the amount negated.
    ///
    /// Note that this may panic if T does not support negative values.
    pub fn inset(&self, amount: T) -> Self {
        self.dilate(-amount)
    }
}

impl<T: PartialOrd + Add<T, Output = T> + Sub<Output = T> + Clone + Debug + Default + PartialEq>
    Bounds<T>
{
    /// Calculates the intersection of two `Bounds` objects.
    ///
    /// This method computes the overlapping region of two `Bounds`. If the bounds do not intersect,
    /// the resulting `Bounds` will have a size with width and height of zero.
    ///
    /// # Arguments
    ///
    /// * `other` - A reference to another `Bounds` to intersect with.
    ///
    /// # Returns
    ///
    /// Returns a `Bounds` representing the intersection area. If there is no intersection,
    /// the returned `Bounds` will have a size with width and height of zero.
    ///
    /// # Examples
    ///
    /// ```
    /// # use inazuma::{Bounds, Point, Size};
    /// let bounds1 = Bounds {
    ///     origin: Point { x: 0, y: 0 },
    ///     size: Size { width: 10, height: 10 },
    /// };
    /// let bounds2 = Bounds {
    ///     origin: Point { x: 5, y: 5 },
    ///     size: Size { width: 10, height: 10 },
    /// };
    /// let intersection = bounds1.intersect(&bounds2);
    ///
    /// assert_eq!(intersection, Bounds {
    ///     origin: Point { x: 5, y: 5 },
    ///     size: Size { width: 5, height: 5 },
    /// });
    /// ```
    pub fn intersect(&self, other: &Self) -> Self {
        let upper_left = self.origin.max(&other.origin);
        let bottom_right = self
            .bottom_right()
            .min(&other.bottom_right())
            .max(&upper_left);
        Self::from_corners(upper_left, bottom_right)
    }

    /// Computes the union of two `Bounds`.
    ///
    /// This method calculates the smallest `Bounds` that contains both the current `Bounds` and the `other` `Bounds`.
    /// The resulting `Bounds` will have an origin that is the minimum of the origins of the two `Bounds`,
    /// and a size that encompasses the furthest extents of both `Bounds`.
    ///
    /// # Arguments
    ///
    /// * `other` - A reference to another `Bounds` to create a union with.
    ///
    /// # Returns
    ///
    /// Returns a `Bounds` representing the union of the two `Bounds`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use inazuma::{Bounds, Point, Size};
    /// let bounds1 = Bounds {
    ///     origin: Point { x: 0, y: 0 },
    ///     size: Size { width: 10, height: 10 },
    /// };
    /// let bounds2 = Bounds {
    ///     origin: Point { x: 5, y: 5 },
    ///     size: Size { width: 15, height: 15 },
    /// };
    /// let union_bounds = bounds1.union(&bounds2);
    ///
    /// assert_eq!(union_bounds, Bounds {
    ///     origin: Point { x: 0, y: 0 },
    ///     size: Size { width: 20, height: 20 },
    /// });
    /// ```
    pub fn union(&self, other: &Self) -> Self {
        let top_left = self.origin.min(&other.origin);
        let bottom_right = self.bottom_right().max(&other.bottom_right());
        Bounds::from_corners(top_left, bottom_right)
    }
}

impl<T> Bounds<T>
where
    T: Add<T, Output = T> + Sub<T, Output = T> + Clone + Debug + Default + PartialEq,
{
    /// Computes the space available within outer bounds.
    pub fn space_within(&self, outer: &Self) -> Edges<T> {
        Edges {
            top: self.top() - outer.top(),
            right: outer.right() - self.right(),
            bottom: outer.bottom() - self.bottom(),
            left: self.left() - outer.left(),
        }
    }
}

impl<T, Rhs> Mul<Rhs> for Bounds<T>
where
    T: Mul<Rhs, Output = Rhs> + Clone + Debug + Default + PartialEq,
    Point<T>: Mul<Rhs, Output = Point<Rhs>>,
    Rhs: Clone + Debug + Default + PartialEq,
{
    type Output = Bounds<Rhs>;

    fn mul(self, rhs: Rhs) -> Self::Output {
        Bounds {
            origin: self.origin * rhs.clone(),
            size: self.size * rhs,
        }
    }
}

impl<T, S> MulAssign<S> for Bounds<T>
where
    T: Mul<S, Output = T> + Clone + Debug + Default + PartialEq,
    S: Clone,
{
    fn mul_assign(&mut self, rhs: S) {
        self.origin *= rhs.clone();
        self.size *= rhs;
    }
}

impl<T, S> Div<S> for Bounds<T>
where
    Size<T>: Div<S, Output = Size<T>>,
    T: Div<S, Output = T> + Clone + Debug + Default + PartialEq,
    S: Clone,
{
    type Output = Self;

    fn div(self, rhs: S) -> Self {
        Self {
            origin: self.origin / rhs.clone(),
            size: self.size / rhs,
        }
    }
}

impl<T> Add<Point<T>> for Bounds<T>
where
    T: Add<T, Output = T> + Clone + Debug + Default + PartialEq,
{
    type Output = Self;

    fn add(self, rhs: Point<T>) -> Self {
        Self {
            origin: self.origin + rhs,
            size: self.size,
        }
    }
}

impl<T> Sub<Point<T>> for Bounds<T>
where
    T: Sub<T, Output = T> + Clone + Debug + Default + PartialEq,
{
    type Output = Self;

    fn sub(self, rhs: Point<T>) -> Self {
        Self {
            origin: self.origin - rhs,
            size: self.size,
        }
    }
}

impl<T: Clone + Debug + Default + PartialEq> From<Size<T>> for Point<T> {
    fn from(size: Size<T>) -> Self {
        Self {
            x: size.width,
            y: size.height,
        }
    }
}
