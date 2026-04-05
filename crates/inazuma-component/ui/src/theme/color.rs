use std::{collections::HashMap, fmt::Display};

use inazuma::{Oklch, SharedString, hsla};
use serde::{Deserialize, Deserializer, de::Error as _};

use anyhow::{Error, Result, anyhow};

/// Create an [`inazuma::Oklch`] color from HSL parameters.
///
/// - h: 0..360.0
/// - s: 0.0..100.0
/// - l: 0.0..100.0
#[inline]
pub fn hsl(h: f32, s: f32, l: f32) -> Oklch {
    hsla(h / 360., s / 100.0, l / 100.0, 1.0)
}

pub trait Colorize: Sized {
    /// Returns a new color with alpha set to the given divisor.
    ///
    /// The divisor in range of 0.0 .. 1.0
    fn divide(&self, divisor: f32) -> Self;
    /// Return inverted color
    fn invert(&self) -> Self;
    /// Return inverted lightness
    fn invert_l(&self) -> Self;
    /// Return a new color with the same lightness and alpha but different hue and chroma.
    fn apply(&self, base_color: Self) -> Self;

    /// Mix two colors together, the `factor` is a value between 0.0 and 1.0 for first color.
    fn mix(&self, other: Self, factor: f32) -> Self;
    /// Mix two colors together in Oklab color space, the `factor` is a value between 0.0 and 1.0 for first color.
    ///
    /// This is similar to CSS `color-mix(in oklab, color1 factor%, color2)`.
    fn mix_oklab(&self, other: Self, factor: f32) -> Self;
    /// Change the `Hue` of the color by the given angle in range: 0.0 .. 360.0
    fn hue(&self, hue: f32) -> Self;
    /// Change the `Chroma` of the color by the given value in range: 0.0 .. 0.4
    fn chroma(&self, chroma: f32) -> Self;
    /// Change the `Lightness` of the color by the given value in range: 0.0 .. 1.0
    fn lightness(&self, lightness: f32) -> Self;

    /// Convert the color to a hex string. For example, "#F8FAFC".
    fn to_hex(&self) -> String;
    /// Parse a hex string to a color.
    fn parse_hex(hex: &str) -> Result<Self>;
}

impl Colorize for Oklch {
    fn divide(&self, divisor: f32) -> Self {
        Self {
            a: divisor,
            ..*self
        }
    }

    fn invert(&self) -> Self {
        Self {
            l: 1.0 - self.l,
            c: self.c,
            h: (self.h + 180.0).rem_euclid(360.0),
            a: self.a,
        }
    }

    fn invert_l(&self) -> Self {
        Self {
            l: 1.0 - self.l,
            ..*self
        }
    }

    fn apply(&self, new_color: Self) -> Self {
        Oklch {
            l: self.l,
            c: new_color.c,
            h: new_color.h,
            a: self.a,
        }
    }

    fn mix(&self, other: Self, factor: f32) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        let inv = 1.0 - factor;

        #[inline]
        fn lerp_hue(a: f32, b: f32, t: f32) -> f32 {
            let diff = (b - a + 180.0).rem_euclid(360.) - 180.;
            (a + diff * t).rem_euclid(360.0)
        }

        Oklch {
            l: self.l * factor + other.l * inv,
            c: self.c * factor + other.c * inv,
            h: lerp_hue(self.h, other.h, factor),
            a: self.a * factor + other.a * inv,
        }
    }

    fn mix_oklab(&self, other: Self, factor: f32) -> Self {
        // Oklch is already in the Oklab family, so we interpolate in Oklab
        // by converting Oklch → Oklab (polar → cartesian), interpolating, and converting back.
        let factor = factor.clamp(0.0, 1.0);
        let inv = 1.0 - factor;

        let result_alpha = self.a * factor + other.a * inv;

        if result_alpha == 0.0 {
            return Oklch::transparent_black();
        }

        // Convert Oklch to Oklab (cartesian)
        let (a1, b1) = {
            let h_rad = self.h.to_radians();
            (self.c * h_rad.cos(), self.c * h_rad.sin())
        };
        let (a2, b2) = {
            let h_rad = other.h.to_radians();
            (other.c * h_rad.cos(), other.c * h_rad.sin())
        };

        // Premultiply alpha
        let l1_pm = self.l * self.a;
        let a1_pm = a1 * self.a;
        let b1_pm = b1 * self.a;
        let l2_pm = other.l * other.a;
        let a2_pm = a2 * other.a;
        let b2_pm = b2 * other.a;

        // Interpolate premultiplied values
        let l_pm = l1_pm * factor + l2_pm * inv;
        let a_pm = a1_pm * factor + a2_pm * inv;
        let b_pm = b1_pm * factor + b2_pm * inv;

        // Unpremultiply
        let l = l_pm / result_alpha;
        let a_val = a_pm / result_alpha;
        let b_val = b_pm / result_alpha;

        // Convert back to Oklch (polar)
        let c = (a_val * a_val + b_val * b_val).sqrt();
        let h = if c < 1e-8 {
            0.0
        } else {
            b_val.atan2(a_val).to_degrees().rem_euclid(360.0)
        };

        Oklch {
            l: l.clamp(0.0, 1.0),
            c: c.max(0.0),
            h,
            a: result_alpha,
        }
    }

    fn to_hex(&self) -> String {
        let rgb = self.to_rgb();

        if self.a < 1. {
            return format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                ((rgb.r * 255.) as u32),
                ((rgb.g * 255.) as u32),
                ((rgb.b * 255.) as u32),
                ((self.a * 255.) as u32)
            );
        }

        format!(
            "#{:02X}{:02X}{:02X}",
            ((rgb.r * 255.) as u32),
            ((rgb.g * 255.) as u32),
            ((rgb.b * 255.) as u32)
        )
    }

    fn parse_hex(hex: &str) -> Result<Self> {
        let hex = hex.trim_start_matches('#');
        let len = hex.len();
        if len != 6 && len != 8 {
            return Err(anyhow::anyhow!("invalid hex color"));
        }

        let r = u8::from_str_radix(&hex[0..2], 16)? as f32 / 255.;
        let g = u8::from_str_radix(&hex[2..4], 16)? as f32 / 255.;
        let b = u8::from_str_radix(&hex[4..6], 16)? as f32 / 255.;
        let a = if len == 8 {
            u8::from_str_radix(&hex[6..8], 16)? as f32 / 255.
        } else {
            1.
        };

        let v = inazuma::Rgba { r, g, b, a };
        let color: Oklch = v.into();
        Ok(color)
    }

    fn hue(&self, hue: f32) -> Self {
        let mut color = *self;
        color.h = hue.clamp(0., 360.);
        color
    }

    fn chroma(&self, chroma: f32) -> Self {
        let mut color = *self;
        color.c = chroma.max(0.);
        color
    }

    fn lightness(&self, lightness: f32) -> Self {
        let mut color = *self;
        color.l = lightness.clamp(0., 1.);
        color
    }
}

pub(crate) static DEFAULT_COLORS: once_cell::sync::Lazy<ShadcnColors> =
    once_cell::sync::Lazy::new(|| {
        serde_json::from_str(include_str!("./default-colors.json"))
            .expect("failed to parse default-colors.json")
    });

type ColorScales = HashMap<usize, ShadcnColor>;

mod color_scales {
    use std::collections::HashMap;

    use super::{ColorScales, ShadcnColor};

    use serde::de::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<ColorScales, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut map = HashMap::new();
        for color in Vec::<ShadcnColor>::deserialize(deserializer)? {
            map.insert(color.scale, color);
        }
        Ok(map)
    }
}

/// Enum representing the available color names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorName {
    White,
    Black,
    Neutral,
    Gray,
    Red,
    Orange,
    Amber,
    Yellow,
    Lime,
    Green,
    Emerald,
    Teal,
    Cyan,
    Sky,
    Blue,
    Indigo,
    Violet,
    Purple,
    Fuchsia,
    Pink,
    Rose,
}

impl Display for ColorName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// Strict color name parser.
impl TryFrom<&str> for ColorName {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "white" => Ok(ColorName::White),
            "black" => Ok(ColorName::Black),
            "neutral" => Ok(ColorName::Neutral),
            "gray" => Ok(ColorName::Gray),
            "red" => Ok(ColorName::Red),
            "orange" => Ok(ColorName::Orange),
            "amber" => Ok(ColorName::Amber),
            "yellow" => Ok(ColorName::Yellow),
            "lime" => Ok(ColorName::Lime),
            "green" => Ok(ColorName::Green),
            "emerald" => Ok(ColorName::Emerald),
            "teal" => Ok(ColorName::Teal),
            "cyan" => Ok(ColorName::Cyan),
            "sky" => Ok(ColorName::Sky),
            "blue" => Ok(ColorName::Blue),
            "indigo" => Ok(ColorName::Indigo),
            "violet" => Ok(ColorName::Violet),
            "purple" => Ok(ColorName::Purple),
            "fuchsia" => Ok(ColorName::Fuchsia),
            "pink" => Ok(ColorName::Pink),
            "rose" => Ok(ColorName::Rose),
            _ => Err(anyhow::anyhow!("Invalid color name")),
        }
    }
}

impl TryFrom<SharedString> for ColorName {
    type Error = anyhow::Error;
    fn try_from(value: SharedString) -> std::result::Result<Self, Self::Error> {
        value.as_ref().try_into()
    }
}

impl ColorName {
    /// Returns all available color names.
    pub fn all() -> [Self; 19] {
        [
            ColorName::Neutral,
            ColorName::Gray,
            ColorName::Red,
            ColorName::Orange,
            ColorName::Amber,
            ColorName::Yellow,
            ColorName::Lime,
            ColorName::Green,
            ColorName::Emerald,
            ColorName::Teal,
            ColorName::Cyan,
            ColorName::Sky,
            ColorName::Blue,
            ColorName::Indigo,
            ColorName::Violet,
            ColorName::Purple,
            ColorName::Fuchsia,
            ColorName::Pink,
            ColorName::Rose,
        ]
    }

    /// Returns the color for the given scale.
    ///
    /// The `scale` is any of `[50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950]`
    /// falls back to 500 if out of range.
    pub fn scale(&self, scale: usize) -> Oklch {
        if self == &ColorName::White {
            return DEFAULT_COLORS.white.color;
        }
        if self == &ColorName::Black {
            return DEFAULT_COLORS.black.color;
        }

        let colors = match self {
            ColorName::Neutral => &DEFAULT_COLORS.neutral,
            ColorName::Gray => &DEFAULT_COLORS.gray,
            ColorName::Red => &DEFAULT_COLORS.red,
            ColorName::Orange => &DEFAULT_COLORS.orange,
            ColorName::Amber => &DEFAULT_COLORS.amber,
            ColorName::Yellow => &DEFAULT_COLORS.yellow,
            ColorName::Lime => &DEFAULT_COLORS.lime,
            ColorName::Green => &DEFAULT_COLORS.green,
            ColorName::Emerald => &DEFAULT_COLORS.emerald,
            ColorName::Teal => &DEFAULT_COLORS.teal,
            ColorName::Cyan => &DEFAULT_COLORS.cyan,
            ColorName::Sky => &DEFAULT_COLORS.sky,
            ColorName::Blue => &DEFAULT_COLORS.blue,
            ColorName::Indigo => &DEFAULT_COLORS.indigo,
            ColorName::Violet => &DEFAULT_COLORS.violet,
            ColorName::Purple => &DEFAULT_COLORS.purple,
            ColorName::Fuchsia => &DEFAULT_COLORS.fuchsia,
            ColorName::Pink => &DEFAULT_COLORS.pink,
            ColorName::Rose => &DEFAULT_COLORS.rose,
            _ => unreachable!(),
        };

        if let Some(color) = colors.get(&scale) {
            color.color
        } else {
            colors.get(&500).unwrap().color
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub(crate) struct ShadcnColors {
    pub(crate) black: ShadcnColor,
    pub(crate) white: ShadcnColor,
    #[serde(with = "color_scales")]
    pub(crate) slate: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) gray: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) zinc: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) neutral: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) stone: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) red: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) orange: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) amber: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) yellow: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) lime: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) green: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) emerald: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) teal: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) cyan: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) sky: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) blue: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) indigo: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) violet: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) purple: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) fuchsia: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) pink: ColorScales,
    #[serde(with = "color_scales")]
    pub(crate) rose: ColorScales,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize)]
pub(crate) struct ShadcnColor {
    #[serde(default)]
    pub(crate) scale: usize,
    #[serde(deserialize_with = "from_hsl_channel", rename = "hslChannel")]
    pub(crate) color: Oklch,
}

/// Deserialize an Oklch color from a string in the format "210 40% 98%" (HSL channel format).
/// The JSON stores colors as HSL, which we convert to Oklch on load.
fn from_hsl_channel<'de, D>(deserializer: D) -> Result<Oklch, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer).unwrap();

    let mut parts = s.split_whitespace();
    if parts.clone().count() != 3 {
        return Err(D::Error::custom(
            "expected hslChannel has 3 parts, e.g: '210 40% 98%'",
        ));
    }

    fn parse_number(s: &str) -> f32 {
        s.trim_end_matches('%')
            .parse()
            .expect("failed to parse number")
    }

    let (h, s, l) = (
        parse_number(parts.next().unwrap()),
        parse_number(parts.next().unwrap()),
        parse_number(parts.next().unwrap()),
    );

    Ok(hsl(h, s, l))
}

macro_rules! color_method {
    ($color:tt, $scale:tt) => {
        paste::paste! {
            #[inline]
            #[allow(unused)]
            pub fn [<$color _ $scale>]() -> Oklch {
                if let Some(color) = DEFAULT_COLORS.$color.get(&($scale as usize)) {
                    return color.color;
                }

                black()
            }
        }
    };
}

macro_rules! color_methods {
    ($color:tt) => {
        paste::paste! {
            /// Get color by scale number.
            ///
            /// The possible scale numbers are:
            /// 50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950
            ///
            /// If the scale number is not found, it will return black color.
            #[inline]
            pub fn [<$color>](scale: usize) -> Oklch {
                if let Some(color) = DEFAULT_COLORS.$color.get(&scale) {
                    return color.color;
                }

                black()
            }
        }

        color_method!($color, 50);
        color_method!($color, 100);
        color_method!($color, 200);
        color_method!($color, 300);
        color_method!($color, 400);
        color_method!($color, 500);
        color_method!($color, 600);
        color_method!($color, 700);
        color_method!($color, 800);
        color_method!($color, 900);
        color_method!($color, 950);
    };
}

pub fn black() -> Oklch {
    DEFAULT_COLORS.black.color
}

pub fn white() -> Oklch {
    DEFAULT_COLORS.white.color
}

color_methods!(slate);
color_methods!(gray);
color_methods!(zinc);
color_methods!(neutral);
color_methods!(stone);
color_methods!(red);
color_methods!(orange);
color_methods!(amber);
color_methods!(yellow);
color_methods!(lime);
color_methods!(green);
color_methods!(emerald);
color_methods!(teal);
color_methods!(cyan);
color_methods!(sky);
color_methods!(blue);
color_methods!(indigo);
color_methods!(violet);
color_methods!(purple);
color_methods!(fuchsia);
color_methods!(pink);
color_methods!(rose);

/// Try to parse the color, HEX or [Tailwind Color](https://tailwindcss.com/docs/colors) expression.
///
/// # Parameter `color` should be one string value listed below:
///
/// - `#RRGGBB` - The HEX color string.
/// - `#RRGGBBAA` - The HEX color string with alpha.
///
/// Or the Tailwind Color format:
///
/// - `name` - The color name `black`, `white`, or any other defined in `crate::color`.
/// - `name-scale` - The color name with scale.
/// - `name/opacity` - The color name with opacity, `opacity` should be an integer between 0 and 100.
/// - `name-scale/opacity` - The color name with scale and opacity.
///
pub fn try_parse_color(color: &str) -> Result<Oklch> {
    if color.starts_with("#") {
        let rgba = inazuma::Rgba::try_from(color)?;
        return Ok(rgba.into());
    }

    let mut name = String::new();
    let mut scale = None;
    let mut opacity = None;
    // 0: name, 1: scale, 2: opacity
    let mut state = 0;
    let mut part = String::new();

    for c in color.chars() {
        match c {
            '-' if state == 0 => {
                name = std::mem::take(&mut part);
                state = 1;
            }
            '/' if state <= 1 => {
                if state == 0 {
                    name = std::mem::take(&mut part);
                } else if state == 1 {
                    scale = part.parse::<usize>().ok();
                    part.clear();
                }
                state = 2;
            }
            _ => part.push(c),
        }
    }

    match state {
        0 => name = part,
        1 => scale = part.parse::<usize>().ok(),
        2 => opacity = part.parse::<f32>().ok(),
        _ => {}
    }

    if name.is_empty() {
        return Err(anyhow!("Empty color name"));
    }

    let mut color = match name.as_str() {
        "black" => Ok::<Oklch, Error>(crate::black()),
        "white" => Ok(crate::white()),
        _ => {
            let color_name = ColorName::try_from(name.as_str())?;
            if let Some(scale) = scale {
                Ok(color_name.scale(scale))
            } else {
                Ok(color_name.scale(500))
            }
        }
    }?;

    if let Some(opacity) = opacity {
        if opacity > 100. {
            return Err(anyhow!("Invalid color opacity"));
        }
        color = color.opacity(opacity / 100.);
    }

    Ok(color)
}

#[cfg(test)]
mod tests {
    use inazuma::{rgb, rgba};

    use super::*;

    #[test]
    fn test_default_colors() {
        assert_eq!(white(), hsl(0.0, 0.0, 100.0));
        assert_eq!(black(), hsl(0.0, 0.0, 0.0));

        assert_eq!(slate_50(), hsl(210.0, 40.0, 98.0));
        assert_eq!(slate_100(), hsl(210.0, 40.0, 96.1));
        assert_eq!(slate_900(), hsl(222.2, 47.4, 11.2));

        assert_eq!(red_50(), hsl(0.0, 85.7, 97.3));
        assert_eq!(yellow_100(), hsl(54.9, 96.7, 88.0));
        assert_eq!(green_200(), hsl(141.0, 78.9, 85.1));
        assert_eq!(cyan_300(), hsl(187.0, 92.4, 69.0));
        assert_eq!(blue_400(), hsl(213.1, 93.9, 67.8));
        assert_eq!(indigo_500(), hsl(238.7, 83.5, 66.7));
    }

    #[test]
    fn test_to_hex_string() {
        // Oklch roundtrip may introduce 1-digit rounding differences
        let color: Oklch = rgb(0xf8fafc).into();
        let hex = color.to_hex();
        let rgb_back = inazuma::Rgba::try_from(hex.as_str()).unwrap();
        let rgb_orig = inazuma::Rgba::from(rgb(0xf8fafc));
        assert!((rgb_back.r - rgb_orig.r).abs() < 0.01);
        assert!((rgb_back.g - rgb_orig.g).abs() < 0.01);
        assert!((rgb_back.b - rgb_orig.b).abs() < 0.01);

        let color: Oklch = rgba(0x0413fcaa).into();
        let hex = color.to_hex();
        assert!(hex.starts_with('#'));
        assert!(hex.len() == 9); // #RRGGBBAA
    }

    #[test]
    fn test_from_hex_string() {
        let color = Oklch::parse_hex("#F8FAFC").unwrap();
        let expected: Oklch = rgb(0xf8fafc).into();
        let rgb_a = color.to_rgb();
        let rgb_b = expected.to_rgb();
        assert!((rgb_a.r - rgb_b.r).abs() < 0.01);
        assert!((rgb_a.g - rgb_b.g).abs() < 0.01);
        assert!((rgb_a.b - rgb_b.b).abs() < 0.01);

        let color = Oklch::parse_hex("#0413FCAA").unwrap();
        assert!((color.a - 0.6667).abs() < 0.01);
    }

    #[test]
    fn test_lighten() {
        // Oklch inherent lighten is additive: l + amount
        let color = super::hsl(240.0, 5.0, 30.0);
        let base_l = color.l;
        let color = color.lighten(0.1);
        assert!((color.l - (base_l + 0.1)).abs() < 1e-6);
        let base_l = color.l;
        let color = color.lighten(0.2);
        assert!((color.l - (base_l + 0.2)).abs() < 1e-6);
    }

    #[test]
    fn test_darken() {
        // Oklch inherent darken is subtractive: l - amount
        let color = super::hsl(240.0, 5.0, 96.0);
        let base_l = color.l;
        let color = color.darken(0.1);
        assert!((color.l - (base_l - 0.1)).abs() < 1e-6);
        let base_l = color.l;
        let color = color.darken(0.2);
        assert!((color.l - (base_l - 0.2)).abs() < 1e-6);
    }

    #[test]
    fn test_mix() {
        let red = Oklch::parse_hex("#FF0000").unwrap();
        let blue = Oklch::parse_hex("#0000FF").unwrap();

        // Mixing at 0.0 should give back the second color (blue)
        let result_0 = red.mix(blue, 0.0);
        assert!((result_0.l - blue.l).abs() < 0.05);

        // Mixing at 1.0 should give back the first color (red)
        let result_1 = red.mix(blue, 1.0);
        assert!((result_1.l - red.l).abs() < 0.05);

        // 50/50 mix should produce a color between both
        let mid = red.mix(blue, 0.5);
        assert!(mid.l > 0.0 && mid.l < 1.0);
    }

    #[test]
    fn test_mix_oklab() {
        let red = Oklch::parse_hex("#FF0000").unwrap();
        let blue = Oklch::parse_hex("#0000FF").unwrap();
        let transparent = Oklch::transparent_black();

        // Test mixing red with transparent (similar to CSS color-mix example)
        // color-mix(in oklab, red 20%, transparent) should give red with 20% opacity
        let result = red.mix_oklab(transparent, 0.2);
        assert!((result.a - 0.2).abs() < 0.01); // Alpha should be 20%

        // The color should remain red (hue should be preserved)
        let rgb_result = result.to_rgb();
        let rgb_red = red.to_rgb();
        // Allow some tolerance due to color space conversions
        assert!(
            (rgb_result.r - rgb_red.r).abs() < 0.05,
            "Red channel should be preserved"
        );
        assert!(rgb_result.g < 0.05, "Green channel should be near 0");
        assert!(rgb_result.b < 0.05, "Blue channel should be near 0");

        // Test basic color mixing in Oklab space
        let purple = red.mix_oklab(blue, 0.5);
        // Oklab mixing should produce different results than HSL mixing
        let purple_hsl = red.mix(blue, 0.5);
        assert_ne!(purple.to_hex(), purple_hsl.to_hex());

        // Test factor boundaries (allowing small floating point errors)
        let result_0 = red.mix_oklab(blue, 0.0);
        let result_1 = red.mix_oklab(blue, 1.0);

        // Check that result is close to expected (within 1 color unit per channel)
        let rgb_0 = result_0.to_rgb();
        let rgb_blue = blue.to_rgb();
        assert!((rgb_0.r - rgb_blue.r).abs() < 0.01);
        assert!((rgb_0.g - rgb_blue.g).abs() < 0.01);
        assert!((rgb_0.b - rgb_blue.b).abs() < 0.01);

        let rgb_1 = result_1.to_rgb();
        let rgb_red = red.to_rgb();
        assert!((rgb_1.r - rgb_red.r).abs() < 0.01);
        assert!((rgb_1.g - rgb_red.g).abs() < 0.01);
        assert!((rgb_1.b - rgb_red.b).abs() < 0.01);
    }

    #[test]
    fn test_color_name() {
        assert_eq!(ColorName::Purple.to_string(), "Purple");
        assert_eq!(format!("{}", ColorName::Green), "Green");
        assert_eq!(format!("{:?}", ColorName::Yellow), "Yellow");

        let color = ColorName::Green;
        assert_eq!(color.scale(500).to_hex(), "#21C55E");
        assert_eq!(color.scale(1500).to_hex(), "#21C55E");

        for name in ColorName::all().iter() {
            let name1: ColorName = name.to_string().as_str().try_into().unwrap();
            assert_eq!(name1, *name);
        }
    }

    #[test]
    fn test_h_c_l() {
        let color = hsl(260., 94., 80.);
        // Test hue change
        let changed = color.hue(200.);
        assert!((changed.h - 200.0).abs() < 0.01);
        assert_eq!(changed.l, color.l);
        assert_eq!(changed.c, color.c);
        // Test chroma change
        let changed = color.chroma(0.1);
        assert!((changed.c - 0.1).abs() < 1e-6);
        assert_eq!(changed.l, color.l);
        // Test lightness change
        let changed = color.lightness(0.5);
        assert!((changed.l - 0.5).abs() < 1e-6);
        assert_eq!(changed.c, color.c);
    }

    #[test]
    fn test_try_parse_color() {
        assert_eq!(
            try_parse_color("#F2F200").ok(),
            Some(hsla(0.16666667, 1., 0.4745098, 1.0))
        );
        assert_eq!(
            try_parse_color("#00f21888").ok(),
            Some(hsla(0.34986225, 1.0, 0.4745098, 0.53333336))
        );
        assert_eq!(try_parse_color("black").ok(), Some(crate::black()));
        assert_eq!(try_parse_color("white-800").ok(), Some(crate::white()));
        assert_eq!(try_parse_color("red").ok(), Some(crate::red_500()));
        assert_eq!(try_parse_color("blue-600").ok(), Some(crate::blue_600()));
        assert_eq!(
            try_parse_color("pink/33").ok(),
            Some(crate::pink_500().opacity(0.33))
        );
        assert_eq!(
            try_parse_color("orange-300/66").ok(),
            Some(crate::orange_300().opacity(0.66))
        );
    }
}
