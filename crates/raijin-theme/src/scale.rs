use inazuma::Oklch;

/// A 12-step OKLCH color scale, inspired by Radix UI color scales.
///
/// Steps progress from near-black (step 1) to near-white (step 12),
/// maintaining consistent chroma and hue from the base color.
#[derive(Clone, Debug)]
pub struct ColorScale {
    /// The 12 color steps, from darkest to lightest.
    pub colors: [Oklch; 12],
}

/// Named steps in a 12-step color scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ColorScaleStep {
    /// Step 1: App background (darkest).
    One = 0,
    /// Step 2: Subtle background.
    Two = 1,
    /// Step 3: UI element background.
    Three = 2,
    /// Step 4: Hovered UI element background.
    Four = 3,
    /// Step 5: Active / selected UI element background.
    Five = 4,
    /// Step 6: Subtle borders and separators.
    Six = 5,
    /// Step 7: UI element border and focus rings.
    Seven = 6,
    /// Step 8: Hovered UI element border.
    Eight = 7,
    /// Step 9: Solid backgrounds (most saturated).
    Nine = 8,
    /// Step 10: Hovered solid backgrounds.
    Ten = 9,
    /// Step 11: Low-contrast text.
    Eleven = 10,
    /// Step 12: High-contrast text (lightest).
    Twelve = 11,
}

/// Lightness values for each of the 12 scale steps.
const SCALE_LIGHTNESS: [f32; 12] = [
    0.13, 0.17, 0.22, 0.27, 0.33, 0.40, 0.48, 0.57, 0.66, 0.75, 0.85, 0.95,
];

impl ColorScale {
    /// Generates a 12-step color scale from a base OKLCH color.
    ///
    /// The base color's chroma and hue are preserved across all steps.
    /// Lightness is distributed from ~0.13 (dark) to ~0.95 (light).
    pub fn from_base(base: Oklch) -> Self {
        let colors = SCALE_LIGHTNESS.map(|l| Oklch {
            l,
            c: base.c,
            h: base.h,
            a: base.a,
        });
        Self { colors }
    }

    /// Returns the color at the given scale step.
    pub fn step(&self, step: ColorScaleStep) -> Oklch {
        self.colors[step as usize]
    }
}
