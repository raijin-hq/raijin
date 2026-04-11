use std::{collections::HashMap, fmt::Display};

use crate::{Label, LabelCommon, component_prelude::*, v_flex};
use documented::{DocumentedFields, DocumentedVariants};
use inazuma::{App, Oklch, IntoElement, ParentElement, SharedString, Styled, hsla, oklch};
use raijin_theme::{ActiveTheme, Appearance};
use serde::{Deserialize, Deserializer, de::Error as _};

/// Returns whether the current theme is light.
pub fn is_light(cx: &App) -> bool {
    cx.theme().appearance() == Appearance::Light
}

/// Returns pure white in Oklch.
pub fn white() -> Oklch {
    oklch(1.0, 0.0, 0.0)
}

/// Returns pure black in Oklch.
pub fn black() -> Oklch {
    oklch(0.0, 0.0, 0.0)
}

/// Sets a color that has a consistent meaning across all themes.
#[derive(
    Debug,
    Default,
    Eq,
    PartialEq,
    Copy,
    Clone,
    RegisterComponent,
    Documented,
    DocumentedFields,
    DocumentedVariants,
)]
pub enum Color {
    #[default]
    /// The default text color. Might be known as "foreground" or "primary" in
    /// some theme systems.
    ///
    /// For less emphasis, consider using [`Color::Muted`] or [`Color::Hidden`].
    Default,
    /// A text color used for accents, such as links or highlights.
    Accent,
    /// A color used to indicate a conflict, such as a version control merge conflict, or a conflict between a file in the editor and the file system.
    Conflict,
    /// A color used to indicate a newly created item, such as a new file in
    /// version control, or a new file on disk.
    Created,
    /// It is highly, HIGHLY recommended not to use this! Using this color
    /// means detaching it from any semantic meaning across themes.
    ///
    /// A custom color specified by an HSLA value.
    Custom(Oklch),
    /// A color used for all debugger UI elements.
    Debugger,
    /// A color used to indicate a deleted item, such as a file removed from version control.
    Deleted,
    /// A color used for disabled UI elements or text, like a disabled button or menu item.
    Disabled,
    /// A color used to indicate an error condition, or something the user
    /// cannot do. In very rare cases, it might be used to indicate dangerous or
    /// destructive action.
    Error,
    /// A color used for elements that represent something that is hidden, like
    /// a hidden file, or an element that should be visually de-emphasized.
    Hidden,
    /// A color used for hint or suggestion text, often a blue color. Use this
    /// color to represent helpful, or semantically neutral information.
    Hint,
    /// A color used for items that are intentionally ignored, such as files ignored by version control.
    Ignored,
    /// A color used for informational messages or status indicators, often a blue color.
    Info,
    /// A color used to indicate a modified item, such as an edited file, or a modified entry in version control.
    Modified,
    /// A color used for text or UI elements that should be visually muted or de-emphasized.
    ///
    /// For more emphasis, consider using [`Color::Default`].
    ///
    /// For less emphasis, consider using [`Color::Hidden`].
    Muted,
    /// A color used for placeholder text in input fields.
    Placeholder,
    /// A color associated with a specific player number.
    Player(u32),
    /// A color used to indicate selected text or UI elements.
    Selected,
    /// A color used to indicate a successful operation or status.
    Success,
    /// A version control color used to indicate a newly added file or content in version control.
    VersionControlAdded,
    /// A version control color used to indicate conflicting changes that need resolution.
    VersionControlConflict,
    /// A version control color used to indicate a file or content that has been deleted in version control.
    VersionControlDeleted,
    /// A version control color used to indicate files or content that is being ignored by version control.
    VersionControlIgnored,
    /// A version control color used to indicate modified files or content in version control.
    VersionControlModified,
    /// A color used to indicate a warning condition.
    Warning,
}

impl Color {
    /// Returns the Color's HSLA value.
    pub fn color(&self, cx: &App) -> Oklch {
        match self {
            Color::Default => cx.theme().colors().text,
            Color::Muted => cx.theme().colors().text_muted,
            Color::Created => cx.theme().status().created.color,
            Color::Modified => cx.theme().status().modified.color,
            Color::Conflict => cx.theme().status().conflict.color,
            Color::Ignored => cx.theme().status().ignored.color,
            Color::Debugger => cx.theme().colors().debugger_accent,
            Color::Deleted => cx.theme().status().deleted.color,
            Color::Disabled => cx.theme().colors().text_disabled,
            Color::Hidden => cx.theme().status().hidden.color,
            Color::Hint => cx.theme().status().hint.color,
            Color::Info => cx.theme().status().info.color,
            Color::Placeholder => cx.theme().colors().text_placeholder,
            Color::Accent => cx.theme().colors().text_accent,
            Color::Player(i) => cx.theme().styles.players.color_for_participant(*i).cursor,
            Color::Error => cx.theme().status().error.color,
            Color::Selected => cx.theme().colors().text_accent,
            Color::Success => cx.theme().status().success.color,
            Color::VersionControlAdded => cx.theme().colors().version_control.added,
            Color::VersionControlConflict => cx.theme().colors().version_control.conflict,
            Color::VersionControlDeleted => cx.theme().colors().version_control.deleted,
            Color::VersionControlIgnored => cx.theme().colors().version_control.ignored,
            Color::VersionControlModified => cx.theme().colors().version_control.modified,
            Color::Warning => cx.theme().status().warning.color,
            Color::Custom(color) => *color,
        }
    }
}

impl From<Oklch> for Color {
    fn from(color: Oklch) -> Self {
        Color::Custom(color)
    }
}

impl Component for Color {
    fn scope() -> ComponentScope {
        ComponentScope::Utilities
    }

    fn description() -> Option<&'static str> {
        Some(Color::DOCS)
    }

    fn preview(_window: &mut inazuma::Window, _cx: &mut App) -> Option<inazuma::AnyElement> {
        Some(
            v_flex()
                .gap_6()
                .children(vec![
                    example_group_with_title(
                        "Text Colors",
                        vec![
                            single_example(
                                "Default",
                                Label::new("Default text color")
                                    .color(Color::Default)
                                    .into_any_element(),
                            )
                            .description(Color::Default.get_variant_docs()),
                            single_example(
                                "Muted",
                                Label::new("Muted text color")
                                    .color(Color::Muted)
                                    .into_any_element(),
                            )
                            .description(Color::Muted.get_variant_docs()),
                            single_example(
                                "Accent",
                                Label::new("Accent text color")
                                    .color(Color::Accent)
                                    .into_any_element(),
                            )
                            .description(Color::Accent.get_variant_docs()),
                            single_example(
                                "Disabled",
                                Label::new("Disabled text color")
                                    .color(Color::Disabled)
                                    .into_any_element(),
                            )
                            .description(Color::Disabled.get_variant_docs()),
                        ],
                    ),
                    example_group_with_title(
                        "Status Colors",
                        vec![
                            single_example(
                                "Success",
                                Label::new("Success status")
                                    .color(Color::Success)
                                    .into_any_element(),
                            )
                            .description(Color::Success.get_variant_docs()),
                            single_example(
                                "Warning",
                                Label::new("Warning status")
                                    .color(Color::Warning)
                                    .into_any_element(),
                            )
                            .description(Color::Warning.get_variant_docs()),
                            single_example(
                                "Error",
                                Label::new("Error status")
                                    .color(Color::Error)
                                    .into_any_element(),
                            )
                            .description(Color::Error.get_variant_docs()),
                            single_example(
                                "Info",
                                Label::new("Info status")
                                    .color(Color::Info)
                                    .into_any_element(),
                            )
                            .description(Color::Info.get_variant_docs()),
                        ],
                    ),
                    example_group_with_title(
                        "Version Control Colors",
                        vec![
                            single_example(
                                "Created",
                                Label::new("Created item")
                                    .color(Color::Created)
                                    .into_any_element(),
                            )
                            .description(Color::Created.get_variant_docs()),
                            single_example(
                                "Modified",
                                Label::new("Modified item")
                                    .color(Color::Modified)
                                    .into_any_element(),
                            )
                            .description(Color::Modified.get_variant_docs()),
                            single_example(
                                "Deleted",
                                Label::new("Deleted item")
                                    .color(Color::Deleted)
                                    .into_any_element(),
                            )
                            .description(Color::Deleted.get_variant_docs()),
                            single_example(
                                "Conflict",
                                Label::new("Conflict item")
                                    .color(Color::Conflict)
                                    .into_any_element(),
                            )
                            .description(Color::Conflict.get_variant_docs()),
                        ],
                    ),
                ])
                .into_any_element(),
        )
    }
}

// ── Color Name System (Tailwind/Shadcn color scales) ─────────────────────────

/// Create an [`Oklch`] color from HSL parameters.
///
/// - h: 0..360.0
/// - s: 0.0..100.0
/// - l: 0.0..100.0
#[inline]
fn color_hsl(h: f32, s: f32, l: f32) -> Oklch {
    hsla(h / 360., s / 100.0, l / 100.0, 1.0)
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

/// Enum representing the available color names (Tailwind/Shadcn palette).
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

    /// Returns a new color with the given opacity.
    pub fn opacity(&self, opacity: f32) -> Oklch {
        let mut color = self.scale(500);
        color.a = opacity;
        color
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

    Ok(color_hsl(h, s, l))
}
