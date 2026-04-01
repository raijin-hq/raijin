use super::*;

impl<T> Bounds<T>
where
    T: Add<T, Output = T> + Clone + Debug + Default + PartialEq,
{
    /// Returns the top edge of the bounds.
    ///
    /// # Returns
    ///
    /// A value of type `T` representing the y-coordinate of the top edge of the bounds.
    pub fn top(&self) -> T {
        self.origin.y.clone()
    }

    /// Returns the bottom edge of the bounds.
    ///
    /// # Returns
    ///
    /// A value of type `T` representing the y-coordinate of the bottom edge of the bounds.
    pub fn bottom(&self) -> T {
        self.origin.y.clone() + self.size.height.clone()
    }

    /// Returns the left edge of the bounds.
    ///
    /// # Returns
    ///
    /// A value of type `T` representing the x-coordinate of the left edge of the bounds.
    pub fn left(&self) -> T {
        self.origin.x.clone()
    }

    /// Returns the right edge of the bounds.
    ///
    /// # Returns
    ///
    /// A value of type `T` representing the x-coordinate of the right edge of the bounds.
    pub fn right(&self) -> T {
        self.origin.x.clone() + self.size.width.clone()
    }

    /// Returns the top right corner point of the bounds.
    ///
    /// # Returns
    ///
    /// A `Point<T>` representing the top right corner of the bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use inazuma::{Bounds, Point, Size};
    /// let bounds = Bounds {
    ///     origin: Point { x: 0, y: 0 },
    ///     size: Size { width: 10, height: 20 },
    /// };
    /// let top_right = bounds.top_right();
    /// assert_eq!(top_right, Point { x: 10, y: 0 });
    /// ```
    pub fn top_right(&self) -> Point<T> {
        Point {
            x: self.origin.x.clone() + self.size.width.clone(),
            y: self.origin.y.clone(),
        }
    }

    /// Returns the bottom right corner point of the bounds.
    ///
    /// # Returns
    ///
    /// A `Point<T>` representing the bottom right corner of the bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use inazuma::{Bounds, Point, Size};
    /// let bounds = Bounds {
    ///     origin: Point { x: 0, y: 0 },
    ///     size: Size { width: 10, height: 20 },
    /// };
    /// let bottom_right = bounds.bottom_right();
    /// assert_eq!(bottom_right, Point { x: 10, y: 20 });
    /// ```
    pub fn bottom_right(&self) -> Point<T> {
        Point {
            x: self.origin.x.clone() + self.size.width.clone(),
            y: self.origin.y.clone() + self.size.height.clone(),
        }
    }

    /// Returns the bottom left corner point of the bounds.
    ///
    /// # Returns
    ///
    /// A `Point<T>` representing the bottom left corner of the bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use inazuma::{Bounds, Point, Size};
    /// let bounds = Bounds {
    ///     origin: Point { x: 0, y: 0 },
    ///     size: Size { width: 10, height: 20 },
    /// };
    /// let bottom_left = bounds.bottom_left();
    /// assert_eq!(bottom_left, Point { x: 0, y: 20 });
    /// ```
    pub fn bottom_left(&self) -> Point<T> {
        Point {
            x: self.origin.x.clone(),
            y: self.origin.y.clone() + self.size.height.clone(),
        }
    }

    /// Returns the requested corner point of the bounds.
    ///
    /// # Returns
    ///
    /// A `Point<T>` representing the corner of the bounds requested by the parameter.
    ///
    /// # Examples
    ///
    /// ```
    /// use inazuma::{Bounds, Corner, Point, Size};
    /// let bounds = Bounds {
    ///     origin: Point { x: 0, y: 0 },
    ///     size: Size { width: 10, height: 20 },
    /// };
    /// let bottom_left = bounds.corner(Corner::BottomLeft);
    /// assert_eq!(bottom_left, Point { x: 0, y: 20 });
    /// ```
    pub fn corner(&self, corner: Corner) -> Point<T> {
        match corner {
            Corner::TopLeft => self.origin.clone(),
            Corner::TopRight => self.top_right(),
            Corner::BottomLeft => self.bottom_left(),
            Corner::BottomRight => self.bottom_right(),
        }
    }
}

impl<T> Bounds<T>
where
    T: Add<T, Output = T> + PartialOrd + Clone + Debug + Default + PartialEq,
{
    /// Checks if the given point is within the bounds.
    ///
    /// This method determines whether a point lies inside the rectangle defined by the bounds,
    /// including the edges. The point is considered inside if its x-coordinate is greater than
    /// or equal to the left edge and less than or equal to the right edge, and its y-coordinate
    /// is greater than or equal to the top edge and less than or equal to the bottom edge of the bounds.
    ///
    /// # Arguments
    ///
    /// * `point` - A reference to a `Point<T>` that represents the point to check.
    ///
    /// # Returns
    ///
    /// Returns `true` if the point is within the bounds, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// # use inazuma::{Point, Bounds, Size};
    /// let bounds = Bounds {
    ///     origin: Point { x: 0, y: 0 },
    ///     size: Size { width: 10, height: 10 },
    /// };
    /// let inside_point = Point { x: 5, y: 5 };
    /// let outside_point = Point { x: 15, y: 15 };
    ///
    /// assert!(bounds.contains(&inside_point));
    /// assert!(!bounds.contains(&outside_point));
    /// ```
    pub fn contains(&self, point: &Point<T>) -> bool {
        point.x >= self.origin.x
            && point.x < self.origin.x.clone() + self.size.width.clone()
            && point.y >= self.origin.y
            && point.y < self.origin.y.clone() + self.size.height.clone()
    }

    /// Checks if this bounds is completely contained within another bounds.
    ///
    /// This method determines whether the current bounds is entirely enclosed by the given bounds.
    /// A bounds is considered to be contained within another if its origin (top-left corner) and
    /// its bottom-right corner are both contained within the other bounds.
    ///
    /// # Arguments
    ///
    /// * `other` - A reference to another `Bounds` that might contain this bounds.
    ///
    /// # Returns
    ///
    /// Returns `true` if this bounds is completely inside the other bounds, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// # use inazuma::{Bounds, Point, Size};
    /// let outer_bounds = Bounds {
    ///     origin: Point { x: 0, y: 0 },
    ///     size: Size { width: 20, height: 20 },
    /// };
    /// let inner_bounds = Bounds {
    ///     origin: Point { x: 5, y: 5 },
    ///     size: Size { width: 10, height: 10 },
    /// };
    /// let overlapping_bounds = Bounds {
    ///     origin: Point { x: 15, y: 15 },
    ///     size: Size { width: 10, height: 10 },
    /// };
    ///
    /// assert!(inner_bounds.is_contained_within(&outer_bounds));
    /// assert!(!overlapping_bounds.is_contained_within(&outer_bounds));
    /// ```
    pub fn is_contained_within(&self, other: &Self) -> bool {
        other.contains(&self.origin) && other.contains(&self.bottom_right())
    }

    /// Applies a function to the origin and size of the bounds, producing a new `Bounds<U>`.
    ///
    /// This method allows for converting a `Bounds<T>` to a `Bounds<U>` by specifying a closure
    /// that defines how to convert between the two types. The closure is applied to the `origin` and
    /// `size` fields, resulting in new bounds of the desired type.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that takes a value of type `T` and returns a value of type `U`.
    ///
    /// # Returns
    ///
    /// Returns a new `Bounds<U>` with the origin and size mapped by the provided function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use inazuma::{Bounds, Point, Size};
    /// let bounds = Bounds {
    ///     origin: Point { x: 10.0, y: 10.0 },
    ///     size: Size { width: 10.0, height: 20.0 },
    /// };
    /// let new_bounds = bounds.map(|value| value as f64 * 1.5);
    ///
    /// assert_eq!(new_bounds, Bounds {
    ///     origin: Point { x: 15.0, y: 15.0 },
    ///     size: Size { width: 15.0, height: 30.0 },
    /// });
    /// ```
    pub fn map<U>(&self, f: impl Fn(T) -> U) -> Bounds<U>
    where
        U: Clone + Debug + Default + PartialEq,
    {
        Bounds {
            origin: self.origin.map(&f),
            size: self.size.map(f),
        }
    }

    /// Applies a function to the origin  of the bounds, producing a new `Bounds` with the new origin
    ///
    /// # Examples
    ///
    /// ```
    /// # use inazuma::{Bounds, Point, Size};
    /// let bounds = Bounds {
    ///     origin: Point { x: 10.0, y: 10.0 },
    ///     size: Size { width: 10.0, height: 20.0 },
    /// };
    /// let new_bounds = bounds.map_origin(|value| value * 1.5);
    ///
    /// assert_eq!(new_bounds, Bounds {
    ///     origin: Point { x: 15.0, y: 15.0 },
    ///     size: Size { width: 10.0, height: 20.0 },
    /// });
    /// ```
    pub fn map_origin(self, f: impl Fn(T) -> T) -> Bounds<T> {
        Bounds {
            origin: self.origin.map(f),
            size: self.size,
        }
    }

    /// Applies a function to the origin  of the bounds, producing a new `Bounds` with the new origin
    ///
    /// # Examples
    ///
    /// ```
    /// # use inazuma::{Bounds, Point, Size};
    /// let bounds = Bounds {
    ///     origin: Point { x: 10.0, y: 10.0 },
    ///     size: Size { width: 10.0, height: 20.0 },
    /// };
    /// let new_bounds = bounds.map_size(|value| value * 1.5);
    ///
    /// assert_eq!(new_bounds, Bounds {
    ///     origin: Point { x: 10.0, y: 10.0 },
    ///     size: Size { width: 15.0, height: 30.0 },
    /// });
    /// ```
    pub fn map_size(self, f: impl Fn(T) -> T) -> Bounds<T> {
        Bounds {
            origin: self.origin,
            size: self.size.map(f),
        }
    }
}

impl<T> Bounds<T>
where
    T: Add<T, Output = T> + Sub<T, Output = T> + PartialOrd + Clone + Debug + Default + PartialEq,
{
    /// Convert a point to the coordinate space defined by this Bounds
    pub fn localize(&self, point: &Point<T>) -> Option<Point<T>> {
        self.contains(point)
            .then(|| point.relative_to(&self.origin))
    }
}

/// Checks if the bounds represent an empty area.
///
/// # Returns
///
/// Returns `true` if either the width or the height of the bounds is less than or equal to zero, indicating an empty area.
impl<T: PartialOrd + Clone + Debug + Default + PartialEq> Bounds<T> {
    /// Checks if the bounds represent an empty area.
    ///
    /// # Returns
    ///
    /// Returns `true` if either the width or the height of the bounds is less than or equal to zero, indicating an empty area.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.size.width <= T::default() || self.size.height <= T::default()
    }
}

impl<T: Clone + Debug + Default + PartialEq + Display + Add<T, Output = T>> Display for Bounds<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} - {} (size {})",
            self.origin,
            self.bottom_right(),
            self.size
        )
    }
}

impl Size<DevicePixels> {
    /// Converts the size from physical to logical pixels.
    pub fn to_pixels(self, scale_factor: f32) -> Size<Pixels> {
        size(
            px(self.width.0 as f32 / scale_factor),
            px(self.height.0 as f32 / scale_factor),
        )
    }
}

impl Size<Pixels> {
    /// Converts the size from logical to physical pixels.
    pub fn to_device_pixels(self, scale_factor: f32) -> Size<DevicePixels> {
        size(
            DevicePixels((self.width.0 * scale_factor).round() as i32),
            DevicePixels((self.height.0 * scale_factor).round() as i32),
        )
    }
}

impl Bounds<Pixels> {
    /// Scales the bounds by a given factor, typically used to adjust for display scaling.
    ///
    /// This method multiplies the origin and size of the bounds by the provided scaling factor,
    /// resulting in a new `Bounds<ScaledPixels>` that is proportionally larger or smaller
    /// depending on the scaling factor. This can be used to ensure that the bounds are properly
    /// scaled for different display densities.
    ///
    /// # Arguments
    ///
    /// * `factor` - The scaling factor to apply to the origin and size, typically the display's scaling factor.
    ///
    /// # Returns
    ///
    /// Returns a new `Bounds<ScaledPixels>` that represents the scaled bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use inazuma::{Bounds, Point, Size, Pixels, ScaledPixels, DevicePixels};
    /// let bounds = Bounds {
    ///     origin: Point { x: Pixels::from(10.0), y: Pixels::from(20.0) },
    ///     size: Size { width: Pixels::from(30.0), height: Pixels::from(40.0) },
    /// };
    /// let display_scale_factor = 2.0;
    /// let scaled_bounds = bounds.scale(display_scale_factor);
    /// assert_eq!(scaled_bounds, Bounds {
    ///     origin: Point {
    ///         x: ScaledPixels::from(20.0),
    ///         y: ScaledPixels::from(40.0),
    ///     },
    ///     size: Size {
    ///         width: ScaledPixels::from(60.0),
    ///         height: ScaledPixels::from(80.0)
    ///     },
    /// });
    /// ```
    pub fn scale(&self, factor: f32) -> Bounds<ScaledPixels> {
        Bounds {
            origin: self.origin.scale(factor),
            size: self.size.scale(factor),
        }
    }

    /// Convert the bounds from logical pixels to physical pixels
    pub fn to_device_pixels(self, factor: f32) -> Bounds<DevicePixels> {
        Bounds {
            origin: point(
                DevicePixels((self.origin.x.0 * factor).round() as i32),
                DevicePixels((self.origin.y.0 * factor).round() as i32),
            ),
            size: self.size.to_device_pixels(factor),
        }
    }
}

impl Bounds<DevicePixels> {
    /// Convert the bounds from physical pixels to logical pixels
    pub fn to_pixels(self, scale_factor: f32) -> Bounds<Pixels> {
        Bounds {
            origin: point(
                px(self.origin.x.0 as f32 / scale_factor),
                px(self.origin.y.0 as f32 / scale_factor),
            ),
            size: self.size.to_pixels(scale_factor),
        }
    }
}
