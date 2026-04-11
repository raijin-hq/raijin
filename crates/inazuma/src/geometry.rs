//! The GPUI geometry module is a collection of types and traits that
//! can be used to describe common units, concepts, and the relationships
//! between them.

mod anchor;
mod bounds;
mod bounds_convert;
mod corners;
mod edges;
mod lengths;
mod placement;
mod point;
mod size;
mod units;

pub use anchor::*;
pub use bounds::*;
pub use corners::*;
pub use edges::*;
pub use lengths::*;
pub use placement::*;
pub use point::*;
pub use size::*;
pub use units::*;

// Shared imports for sub-modules (accessed via `use super::*;`)
use anyhow::{Context as _, anyhow};
use core::fmt::Debug;
use derive_more::{Add, AddAssign, Div, DivAssign, Mul, Neg, Sub, SubAssign};
use refineable::Refineable;
use schemars::{JsonSchema, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::borrow::Cow;
use std::ops::Range;
use std::{
    cmp::{self, PartialOrd},
    fmt::{self, Display},
    hash::Hash,
    ops::{Add, Div, Mul, MulAssign, Neg, Sub},
};
use taffy::prelude::{TaffyGridLine, TaffyGridSpan};
use crate::{App, DisplayId};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds_intersects() {
        let bounds1 = Bounds {
            origin: Point { x: 0.0, y: 0.0 },
            size: Size {
                width: 5.0,
                height: 5.0,
            },
        };
        let bounds2 = Bounds {
            origin: Point { x: 4.0, y: 4.0 },
            size: Size {
                width: 5.0,
                height: 5.0,
            },
        };
        let bounds3 = Bounds {
            origin: Point { x: 10.0, y: 10.0 },
            size: Size {
                width: 5.0,
                height: 5.0,
            },
        };

        // Test Case 1: Intersecting bounds
        assert!(bounds1.intersects(&bounds2));

        // Test Case 2: Non-Intersecting bounds
        assert!(!bounds1.intersects(&bounds3));

        // Test Case 3: Bounds intersecting with themselves
        assert!(bounds1.intersects(&bounds1));
    }
}
