use inazuma::Oklch;

/// A set of accent colors used for iterating elements like rainbow brackets,
/// indent guides, or other UI elements that cycle through a color series.
#[derive(Clone, Debug)]
pub struct AccentColors(pub Vec<Oklch>);

impl AccentColors {
    /// Returns the accent color at the given index, wrapping around.
    pub fn color_at(&self, index: usize) -> Oklch {
        self.0[index % self.0.len()]
    }

    /// Returns the number of accent colors available.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if there are no accent colors.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
