use super::*;

/// Convert an RGB hex color code number to a color type
pub fn rgb(hex: u32) -> Rgba {
    let [_, r, g, b] = hex.to_be_bytes().map(|b| (b as f32) / 255.0);
    Rgba { r, g, b, a: 1.0 }
}

/// Convert an RGBA hex color code number to [`Rgba`]
pub fn rgba(hex: u32) -> Rgba {
    let [r, g, b, a] = hex.to_be_bytes().map(|b| (b as f32) / 255.0);
    Rgba { r, g, b, a }
}

/// Swap from RGBA with premultiplied alpha to BGRA
pub fn swap_rgba_pa_to_bgra(color: &mut [u8]) {
    color.swap(0, 2);
    if color[3] > 0 {
        let a = color[3] as f32 / 255.;
        color[0] = (color[0] as f32 / a) as u8;
        color[1] = (color[1] as f32 / a) as u8;
        color[2] = (color[2] as f32 / a) as u8;
    }
}

/// An RGBA color
#[derive(PartialEq, Clone, Copy, Default)]
#[repr(C)]
pub struct Rgba {
    /// The red component of the color, in the range 0.0 to 1.0
    pub r: f32,
    /// The green component of the color, in the range 0.0 to 1.0
    pub g: f32,
    /// The blue component of the color, in the range 0.0 to 1.0
    pub b: f32,
    /// The alpha component of the color, in the range 0.0 to 1.0
    pub a: f32,
}

impl fmt::Debug for Rgba {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "rgba({:#010x})", u32::from(*self))
    }
}

impl Rgba {
    /// Create a new [`Rgba`] color by blending this and another color together
    pub fn blend(&self, other: Rgba) -> Self {
        if other.a >= 1.0 {
            other
        } else if other.a <= 0.0 {
            *self
        } else {
            Rgba {
                r: (self.r * (1.0 - other.a)) + (other.r * other.a),
                g: (self.g * (1.0 - other.a)) + (other.g * other.a),
                b: (self.b * (1.0 - other.a)) + (other.b * other.a),
                a: self.a,
            }
        }
    }
}

impl From<Rgba> for u32 {
    fn from(rgba: Rgba) -> Self {
        let r = (rgba.r * 255.0) as u32;
        let g = (rgba.g * 255.0) as u32;
        let b = (rgba.b * 255.0) as u32;
        let a = (rgba.a * 255.0) as u32;
        (r << 24) | (g << 16) | (b << 8) | a
    }
}

struct RgbaVisitor;

impl Visitor<'_> for RgbaVisitor {
    type Value = Rgba;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string in the format #rrggbb or #rrggbbaa")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Rgba, E> {
        Rgba::try_from(value).map_err(E::custom)
    }
}

impl JsonSchema for Rgba {
    fn schema_name() -> Cow<'static, str> {
        "Rgba".into()
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        json_schema!({
            "type": "string",
            "pattern": "^#([0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$"
        })
    }
}

impl<'de> Deserialize<'de> for Rgba {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(RgbaVisitor)
    }
}

impl Serialize for Rgba {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let r = (self.r * 255.0).round() as u8;
        let g = (self.g * 255.0).round() as u8;
        let b = (self.b * 255.0).round() as u8;
        let a = (self.a * 255.0).round() as u8;

        let s = format!("#{r:02x}{g:02x}{b:02x}{a:02x}");
        serializer.serialize_str(&s)
    }
}

/// Convert HSL values (h, s, l in 0..1) + alpha to an Rgba color.
/// Used internally by the `hsla()` convenience function.
fn hsl_to_rgba(h: f32, s: f32, l: f32, a: f32) -> Rgba {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let cm = c + m;
    let xm = x + m;

    let (r, g, b) = match (h * 6.0).floor() as i32 {
        0 | 6 => (cm, xm, m),
        1 => (xm, cm, m),
        2 => (m, cm, xm),
        3 => (m, xm, cm),
        4 => (xm, m, cm),
        _ => (cm, m, xm),
    };

    Rgba {
        r: r.clamp(0., 1.),
        g: g.clamp(0., 1.),
        b: b.clamp(0., 1.),
        a,
    }
}

impl TryFrom<&'_ str> for Rgba {
    type Error = anyhow::Error;

    fn try_from(value: &'_ str) -> Result<Self, Self::Error> {
        const RGB: usize = "rgb".len();
        const RGBA: usize = "rgba".len();
        const RRGGBB: usize = "rrggbb".len();
        const RRGGBBAA: usize = "rrggbbaa".len();

        const EXPECTED_FORMATS: &str = "Expected #rgb, #rgba, #rrggbb, or #rrggbbaa";
        const INVALID_UNICODE: &str = "invalid unicode characters in color";

        let Some(("", hex)) = value.trim().split_once('#') else {
            bail!("invalid RGBA hex color: '{value}'. {EXPECTED_FORMATS}");
        };

        let (r, g, b, a) = match hex.len() {
            RGB | RGBA => {
                let r = u8::from_str_radix(
                    hex.get(0..1).with_context(|| {
                        format!("{INVALID_UNICODE}: r component of #rgb/#rgba for value: '{value}'")
                    })?,
                    16,
                )?;
                let g = u8::from_str_radix(
                    hex.get(1..2).with_context(|| {
                        format!("{INVALID_UNICODE}: g component of #rgb/#rgba for value: '{value}'")
                    })?,
                    16,
                )?;
                let b = u8::from_str_radix(
                    hex.get(2..3).with_context(|| {
                        format!("{INVALID_UNICODE}: b component of #rgb/#rgba for value: '{value}'")
                    })?,
                    16,
                )?;
                let a = if hex.len() == RGBA {
                    u8::from_str_radix(
                        hex.get(3..4).with_context(|| {
                            format!("{INVALID_UNICODE}: a component of #rgba for value: '{value}'")
                        })?,
                        16,
                    )?
                } else {
                    0xf
                };

                /// Duplicates a given hex digit.
                /// E.g., `0xf` -> `0xff`.
                const fn duplicate(value: u8) -> u8 {
                    (value << 4) | value
                }

                (duplicate(r), duplicate(g), duplicate(b), duplicate(a))
            }
            RRGGBB | RRGGBBAA => {
                let r = u8::from_str_radix(
                    hex.get(0..2).with_context(|| {
                        format!(
                            "{}: r component of #rrggbb/#rrggbbaa for value: '{}'",
                            INVALID_UNICODE, value
                        )
                    })?,
                    16,
                )?;
                let g = u8::from_str_radix(
                    hex.get(2..4).with_context(|| {
                        format!(
                            "{INVALID_UNICODE}: g component of #rrggbb/#rrggbbaa for value: '{value}'"
                        )
                    })?,
                    16,
                )?;
                let b = u8::from_str_radix(
                    hex.get(4..6).with_context(|| {
                        format!(
                            "{INVALID_UNICODE}: b component of #rrggbb/#rrggbbaa for value: '{value}'"
                        )
                    })?,
                    16,
                )?;
                let a = if hex.len() == RRGGBBAA {
                    u8::from_str_radix(
                        hex.get(6..8).with_context(|| {
                            format!(
                                "{INVALID_UNICODE}: a component of #rrggbbaa for value: '{value}'"
                            )
                        })?,
                        16,
                    )?
                } else {
                    0xff
                };
                (r, g, b, a)
            }
            _ => bail!("invalid RGBA hex color: '{value}'. {EXPECTED_FORMATS}"),
        };

        Ok(Rgba {
            r: r as f32 / 255.,
            g: g as f32 / 255.,
            b: b as f32 / 255.,
            a: a as f32 / 255.,
        })
    }
}

/// An OKLCH color — Inazuma's primary perceptually uniform color type.
///
/// OKLCH is the polar form of Oklab: L = perceived lightness, C = chroma (colorfulness),
/// H = hue angle, A = alpha. It provides perceptually uniform lightness and hue, making it
/// ideal for palette generation, contrast checks, and color blending.
///
/// User-facing input remains hex (#rrggbb). OKLCH is the internal representation.
#[derive(Default, Copy, Clone, Debug)]
#[repr(C)]
pub struct Oklch {
    /// Perceived lightness, range 0.0 (black) to 1.0 (white)
    pub l: f32,
    /// Chroma (colorfulness), range 0.0 (gray) to ~0.4 (most vivid)
    pub c: f32,
    /// Hue angle in degrees, range 0.0 to 360.0
    pub h: f32,
    /// Alpha, range 0.0 (transparent) to 1.0 (opaque)
    pub a: f32,
}

// --- sRGB gamma encode/decode (culori reference) ---

/// sRGB → Linear RGB (gamma decode)
fn srgb_to_linear(c: f32) -> f32 {
    let abs = c.abs();
    if abs <= 0.04045 {
        c / 12.92
    } else {
        c.signum() * ((abs + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear RGB → sRGB (gamma encode)
fn linear_to_srgb(c: f32) -> f32 {
    let abs = c.abs();
    if abs > 0.0031308 {
        c.signum() * (1.055 * abs.powf(1.0 / 2.4) - 0.055)
    } else {
        c * 12.92
    }
}

impl From<Oklch> for Rgba {
    fn from(color: Oklch) -> Self {
        // Oklch → Oklab (polar → cartesian)
        let (ca, cb) = if color.c == 0.0 || color.l == 0.0 || color.l == 1.0 {
            (0.0, 0.0)
        } else {
            let h_rad = color.h.to_radians();
            (color.c * h_rad.cos(), color.c * h_rad.sin())
        };

        // Oklab → LMS (cube)
        let l_ = color.l + 0.3963377773761749 * ca + 0.2158037573099136 * cb;
        let m_ = color.l - 0.1055613458156586 * ca - 0.0638541728258133 * cb;
        let s_ = color.l - 0.0894841775298119 * ca - 1.2914855480194092 * cb;

        let l_3 = l_ * l_ * l_;
        let m_3 = m_ * m_ * m_;
        let s_3 = s_ * s_ * s_;

        // LMS → Linear RGB
        let r_lin = 4.0767416360759574 * l_3 - 3.3077115392580616 * m_3 + 0.2309699031821044 * s_3;
        let g_lin = -1.2684379732850317 * l_3 + 2.6097573492876887 * m_3 - 0.3413193760026573 * s_3;
        let b_lin = -0.0041960761386756 * l_3 - 0.7034186179359362 * m_3 + 1.7076146940746117 * s_3;

        // Linear RGB → sRGB (gamma encode + clamp)
        Rgba {
            r: linear_to_srgb(r_lin).clamp(0.0, 1.0),
            g: linear_to_srgb(g_lin).clamp(0.0, 1.0),
            b: linear_to_srgb(b_lin).clamp(0.0, 1.0),
            a: color.a,
        }
    }
}

impl From<Rgba> for Oklch {
    fn from(color: Rgba) -> Self {
        // sRGB → Linear RGB (gamma decode)
        let r_lin = srgb_to_linear(color.r);
        let g_lin = srgb_to_linear(color.g);
        let b_lin = srgb_to_linear(color.b);

        // Linear RGB → LMS (cbrt)
        let l_ = (0.412221469470763 * r_lin + 0.5363325372617348 * g_lin + 0.0514459932675022 * b_lin).cbrt();
        let m_ = (0.2119034958178252 * r_lin + 0.6806995506452344 * g_lin + 0.1073969535369406 * b_lin).cbrt();
        let s_ = (0.0883024591900564 * r_lin + 0.2817188391361215 * g_lin + 0.6299787016738222 * b_lin).cbrt();

        // LMS → Oklab (cartesian)
        let l = 0.210454268309314 * l_ + 0.7936177747023054 * m_ - 0.0040720430116193 * s_;
        let ca = 1.9779985324311684 * l_ - 2.4285922420485799 * m_ + 0.450593709617411 * s_;
        let cb = 0.0259040424655478 * l_ + 0.7827717124575296 * m_ - 0.8086757549230774 * s_;

        // Oklab → Oklch (cartesian → polar)
        let c = (ca * ca + cb * cb).sqrt();
        let h = if c < 1e-8 {
            0.0 // achromatic
        } else {
            cb.atan2(ca).to_degrees().rem_euclid(360.0)
        };

        Oklch { l, c, h, a: color.a }
    }
}

impl PartialEq for Oklch {
    fn eq(&self, other: &Self) -> bool {
        self.l.total_cmp(&other.l)
            .then(self.c.total_cmp(&other.c))
            .then(self.h.total_cmp(&other.h).then(self.a.total_cmp(&other.a)))
            .is_eq()
    }
}

impl PartialOrd for Oklch {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Oklch {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.l.total_cmp(&other.l)
            .then(self.c.total_cmp(&other.c))
            .then(self.h.total_cmp(&other.h).then(self.a.total_cmp(&other.a)))
    }
}

impl Eq for Oklch {}

impl Hash for Oklch {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(u32::from_be_bytes(self.l.to_be_bytes()));
        state.write_u32(u32::from_be_bytes(self.c.to_be_bytes()));
        state.write_u32(u32::from_be_bytes(self.h.to_be_bytes()));
        state.write_u32(u32::from_be_bytes(self.a.to_be_bytes()));
    }
}

impl Display for Oklch {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if (self.a - 1.0).abs() < f32::EPSILON {
            write!(f, "oklch({:.4} {:.4} {:.2})", self.l, self.c, self.h)
        } else {
            write!(f, "oklch({:.4} {:.4} {:.2} / {:.2})", self.l, self.c, self.h, self.a)
        }
    }
}

impl JsonSchema for Oklch {
    fn schema_name() -> Cow<'static, str> {
        Rgba::schema_name()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        Rgba::json_schema(generator)
    }
}

impl Serialize for Oklch {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Rgba::from(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Oklch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Oklch::from(Rgba::deserialize(deserializer)?))
    }
}

/// Construct an [`Oklch`] color from lightness, chroma, and hue
pub fn oklch(l: f32, c: f32, h: f32) -> Oklch {
    Oklch {
        l: l.clamp(0.0, 1.0),
        c: c.max(0.0),
        h: h.rem_euclid(360.0),
        a: 1.0,
    }
}

/// Construct an [`Oklch`] color from lightness, chroma, hue, and alpha
pub fn oklcha(l: f32, c: f32, h: f32, a: f32) -> Oklch {
    Oklch {
        l: l.clamp(0.0, 1.0),
        c: c.max(0.0),
        h: h.rem_euclid(360.0),
        a: a.clamp(0.0, 1.0),
    }
}

impl Oklch {
    /// Converts this OKLCH color to an RGBA color.
    pub fn to_rgb(self) -> Rgba {
        self.into()
    }

    /// The color black
    pub const fn black() -> Self {
        Oklch { l: 0.0, c: 0.0, h: 0.0, a: 1.0 }
    }

    /// The color white
    pub const fn white() -> Self {
        Oklch { l: 1.0, c: 0.0, h: 0.0, a: 1.0 }
    }

    /// Transparent black
    pub const fn transparent_black() -> Self {
        Oklch { l: 0.0, c: 0.0, h: 0.0, a: 0.0 }
    }

    /// Returns true if fully transparent
    pub fn is_transparent(&self) -> bool {
        self.a == 0.0
    }

    /// Returns true if fully opaque
    pub fn is_opaque(&self) -> bool {
        self.a == 1.0
    }

    /// Multiplies the alpha value by a given factor, returns a new color.
    pub fn opacity(&self, factor: f32) -> Self {
        Oklch {
            l: self.l,
            c: self.c,
            h: self.h,
            a: self.a * factor.clamp(0.0, 1.0),
        }
    }

    /// Returns a new color with the given alpha value.
    pub fn alpha(&self, a: f32) -> Self {
        Oklch {
            l: self.l,
            c: self.c,
            h: self.h,
            a: a.clamp(0.0, 1.0),
        }
    }

    /// Fade out by a given factor (0.0 = unchanged, 1.0 = fully transparent).
    pub fn fade_out(&mut self, factor: f32) {
        self.a *= 1.0 - factor.clamp(0.0, 1.0);
    }

    /// Lighten by increasing L towards 1.0.
    pub fn lighten(&self, amount: f32) -> Self {
        Oklch {
            l: (self.l + amount).clamp(0.0, 1.0),
            c: self.c,
            h: self.h,
            a: self.a,
        }
    }

    /// Darken by decreasing L towards 0.0.
    pub fn darken(&self, amount: f32) -> Self {
        Oklch {
            l: (self.l - amount).clamp(0.0, 1.0),
            c: self.c,
            h: self.h,
            a: self.a,
        }
    }

    /// Returns a grayscale version of this color by setting chroma to 0.
    pub fn grayscale(&self) -> Self {
        Oklch {
            l: self.l,
            c: 0.0,
            h: self.h,
            a: self.a,
        }
    }

    /// Check if this color is within the sRGB gamut.
    ///
    /// A color is in-gamut when its sRGB R, G, B components are all in [0.0, 1.0]
    /// after conversion from Oklch (without clamping).
    pub fn in_srgb_gamut(&self) -> bool {
        let rgba = self.to_rgb_unclamped();
        rgba.r >= 0.0 && rgba.r <= 1.0 &&
        rgba.g >= 0.0 && rgba.g <= 1.0 &&
        rgba.b >= 0.0 && rgba.b <= 1.0
    }

    /// Clamp this color to the sRGB gamut by reducing chroma.
    ///
    /// Uses binary search on chroma (C) while keeping lightness (L) and hue (H) constant.
    /// This preserves the perceived color as much as possible while bringing it into gamut.
    pub fn clamp_to_srgb(self) -> Self {
        if self.in_srgb_gamut() {
            return self;
        }
        Self::gamut_clamp_by_chroma(self, |oklch| oklch.in_srgb_gamut())
    }

    /// Clamp this color to the Display P3 gamut by reducing chroma.
    ///
    /// Uses binary search on chroma (C) while keeping lightness (L) and hue (H) constant.
    /// Display P3 covers approximately 25% more colors than sRGB.
    pub fn clamp_to_p3(self) -> Self {
        if self.in_p3_gamut() {
            return self;
        }
        Self::gamut_clamp_by_chroma(self, |oklch| oklch.in_p3_gamut())
    }

    /// Check if this color is within the Display P3 gamut.
    ///
    /// Converts to linear P3 RGB and checks that all components are in [0.0, 1.0].
    pub fn in_p3_gamut(&self) -> bool {
        let (r, g, b) = self.to_linear_p3();
        r >= 0.0 && r <= 1.0 &&
        g >= 0.0 && g <= 1.0 &&
        b >= 0.0 && b <= 1.0
    }

    /// Binary-search chroma reduction to fit within a target gamut.
    fn gamut_clamp_by_chroma(color: Self, in_gamut: impl Fn(&Self) -> bool) -> Self {
        let mut lo = 0.0_f32;
        let mut hi = color.c;
        let epsilon = 0.001;

        // Binary search: find the highest chroma that is still in gamut
        while hi - lo > epsilon {
            let mid = (lo + hi) * 0.5;
            let candidate = Oklch { c: mid, ..color };
            if in_gamut(&candidate) {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        Oklch { c: lo, ..color }
    }

    /// Convert to unclamped sRGB (components may exceed [0,1] for out-of-gamut colors).
    fn to_rgb_unclamped(&self) -> Rgba {
        let (ca, cb) = if self.c == 0.0 || self.l == 0.0 || self.l == 1.0 {
            (0.0, 0.0)
        } else {
            let h_rad = self.h.to_radians();
            (self.c * h_rad.cos(), self.c * h_rad.sin())
        };

        let l_ = self.l + 0.3963377773761749 * ca + 0.2158037573099136 * cb;
        let m_ = self.l - 0.1055613458156586 * ca - 0.0638541728258133 * cb;
        let s_ = self.l - 0.0894841775298119 * ca - 1.2914855480194092 * cb;

        let l_3 = l_ * l_ * l_;
        let m_3 = m_ * m_ * m_;
        let s_3 = s_ * s_ * s_;

        let r_lin = 4.0767416360759574 * l_3 - 3.3077115392580616 * m_3 + 0.2309699031821044 * s_3;
        let g_lin = -1.2684379732850317 * l_3 + 2.6097573492876887 * m_3 - 0.3413193760026573 * s_3;
        let b_lin = -0.0041960761386756 * l_3 - 0.7034186179359362 * m_3 + 1.7076146940746117 * s_3;

        Rgba {
            r: linear_to_srgb(r_lin),
            g: linear_to_srgb(g_lin),
            b: linear_to_srgb(b_lin),
            a: self.a,
        }
    }

    /// Convert Oklch to linear Display P3 RGB (unclamped).
    ///
    /// Uses the Oklab-to-LMS-to-linear-P3 matrix (D65-adapted).
    fn to_linear_p3(&self) -> (f32, f32, f32) {
        let (ca, cb) = if self.c == 0.0 || self.l == 0.0 || self.l == 1.0 {
            (0.0, 0.0)
        } else {
            let h_rad = self.h.to_radians();
            (self.c * h_rad.cos(), self.c * h_rad.sin())
        };

        let l_ = self.l + 0.3963377773761749 * ca + 0.2158037573099136 * cb;
        let m_ = self.l - 0.1055613458156586 * ca - 0.0638541728258133 * cb;
        let s_ = self.l - 0.0894841775298119 * ca - 1.2914855480194092 * cb;

        let l_3 = l_ * l_ * l_;
        let m_3 = m_ * m_ * m_;
        let s_3 = s_ * s_ * s_;

        // Linear sRGB from LMS (same as existing conversion)
        let r_lin = 4.0767416360759574 * l_3 - 3.3077115392580616 * m_3 + 0.2309699031821044 * s_3;
        let g_lin = -1.2684379732850317 * l_3 + 2.6097573492876887 * m_3 - 0.3413193760026573 * s_3;
        let b_lin = -0.0041960761386756 * l_3 - 0.7034186179359362 * m_3 + 1.7076146940746117 * s_3;

        // sRGB-to-P3 matrix (linear): convert linear sRGB to linear Display P3
        // Derived from: P3_from_XYZ * XYZ_from_sRGB
        // Display P3 uses the same D65 whitepoint and transfer function as sRGB
        // but with wider primaries, so this is a simple 3x3 matrix multiply.
        let p3_r =  0.8224621 * r_lin + 0.17753792 * g_lin + 0.0 * b_lin;
        let p3_g =  0.033194 * r_lin  + 0.96680605 * g_lin + 0.0 * b_lin;
        let p3_b =  0.017082631 * r_lin + 0.072396955 * g_lin + 0.9105204 * b_lin;

        (p3_r, p3_g, p3_b)
    }

    /// Convert this Oklch color to gamma-encoded Display P3 RGB.
    ///
    /// Display P3 uses the same transfer function (piecewise gamma 2.4) as sRGB
    /// but with wider primaries, covering ~25% more visible colors.
    pub fn to_p3_rgb(self) -> Rgba {
        let (r, g, b) = self.to_linear_p3();
        Rgba {
            r: linear_to_srgb(r).clamp(0.0, 1.0),
            g: linear_to_srgb(g).clamp(0.0, 1.0),
            b: linear_to_srgb(b).clamp(0.0, 1.0),
            a: self.a,
        }
    }

    /// Blend this color with another in OKLCH space.
    /// Uses shortest-arc hue interpolation.
    pub fn blend(self, other: Oklch) -> Oklch {
        let alpha = other.a;
        if alpha >= 1.0 {
            return other;
        }
        if alpha <= 0.0 {
            return self;
        }

        let t = alpha;
        let l = self.l + (other.l - self.l) * t;
        let c = self.c + (other.c - self.c) * t;
        let a = self.a + (other.a - self.a) * t;

        // Hue: shortest-arc interpolation
        let h = if self.c < 1e-8 {
            other.h // self is achromatic, use other's hue
        } else if other.c < 1e-8 {
            self.h // other is achromatic, use self's hue
        } else {
            let mut diff = other.h - self.h;
            if diff > 180.0 {
                diff -= 360.0;
            } else if diff < -180.0 {
                diff += 360.0;
            }
            (self.h + diff * t).rem_euclid(360.0)
        };

        Oklch { l, c, h, a }
    }
}

/// Construct an [`Oklch`] color from HSL values (all in 0..1 range) and alpha.
///
/// This is a convenience function for specifying colors in HSL terms.
/// The conversion goes HSL → sRGB → Oklch internally.
pub fn hsla(h: f32, s: f32, l: f32, a: f32) -> Oklch {
    let h = h.clamp(0., 1.);
    let s = s.clamp(0., 1.);
    let l = l.clamp(0., 1.);
    let a = a.clamp(0., 1.);
    Oklch::from(hsl_to_rgba(h, s, l, a))
}

/// Convert an Oklch color to HSL values (h, s, l, a), all in 0..1 range.
///
/// Useful for color pickers and UI that need HSL slider representation.
pub fn oklch_to_hsla(color: Oklch) -> (f32, f32, f32, f32) {
    let rgba = Rgba::from(color);
    let r = rgba.r;
    let g = rgba.g;
    let b = rgba.b;

    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let delta = max - min;

    let l = (max + min) / 2.0;
    let s = if l == 0.0 || l == 1.0 {
        0.0
    } else if l < 0.5 {
        delta / (2.0 * l)
    } else {
        delta / (2.0 - 2.0 * l)
    };

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / delta).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / delta + 2.0) / 6.0
    } else {
        ((r - g) / delta + 4.0) / 6.0
    };

    (h, s, l, rgba.a)
}

/// Pure black in [`Oklch`]
pub const fn black() -> Oklch {
    Oklch::black()
}

/// Transparent black in [`Oklch`]
pub const fn transparent_black() -> Oklch {
    Oklch::transparent_black()
}

/// Transparent white in [`Oklch`]
pub const fn transparent_white() -> Oklch {
    Oklch { l: 1.0, c: 0.0, h: 0.0, a: 0.0 }
}

/// Opaque grey in [`Oklch`], values will be clamped to the range [0, 1]
pub fn opaque_grey(lightness: f32, opacity: f32) -> Oklch {
    hsla(0., 0., lightness.clamp(0., 1.), opacity.clamp(0., 1.))
}

/// Pure white in [`Oklch`]
pub const fn white() -> Oklch {
    Oklch::white()
}

/// The color red in [`Oklch`]
pub fn red() -> Oklch {
    hsla(0., 1., 0.5, 1.)
}

/// The color blue in [`Oklch`]
pub fn blue() -> Oklch {
    hsla(0.6666666667, 1., 0.5, 1.)
}

/// The color green in [`Oklch`]
pub fn green() -> Oklch {
    hsla(0.3333333333, 1., 0.25, 1.)
}

/// The color yellow in [`Oklch`]
pub fn yellow() -> Oklch {
    hsla(0.1666666667, 1., 0.5, 1.)
}
