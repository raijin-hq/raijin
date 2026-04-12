use crate::schema::{status_colors_refinement, syntax_overrides, theme_colors_refinement};
use inazuma_collections::HashMap;
use inazuma::{App, Font, FontFallbacks, FontStyle, Global, Pixels, Window, px};
use inazuma_refineable::Refineable;
use inazuma_settings_framework::{IntoInazuma, RegisterSetting, Settings, SettingsContent};
use std::sync::Arc;
use raijin_theme::{Appearance, SyntaxTheme, Theme, UiDensity};

pub use inazuma_settings_content::{FontFamilyName, IconThemeName, ThemeAppearanceMode, ThemeName};

const MIN_FONT_SIZE: Pixels = px(6.0);
const MAX_FONT_SIZE: Pixels = px(100.0);
const MIN_LINE_HEIGHT: f32 = 1.0;

fn ui_density_from_settings(val: inazuma_settings_content::UiDensity) -> UiDensity {
    match val {
        inazuma_settings_content::UiDensity::Compact => UiDensity::Compact,
        inazuma_settings_content::UiDensity::Default => UiDensity::Default,
        inazuma_settings_content::UiDensity::Comfortable => UiDensity::Comfortable,
    }
}

pub fn appearance_to_mode(appearance: Appearance) -> ThemeAppearanceMode {
    match appearance {
        Appearance::Light => ThemeAppearanceMode::Light,
        Appearance::Dark => ThemeAppearanceMode::Dark,
    }
}

/// Customizable settings for the UI and theme system.
#[derive(Clone, PartialEq, RegisterSetting)]
pub struct ThemeSettings {
    ui_font_size: Pixels,
    pub ui_font: Font,
    buffer_font_size: Pixels,
    pub buffer_font: Font,
    pub buffer_line_height: BufferLineHeight,
    pub theme: ThemeSelection,
    pub experimental_theme_overrides: Option<inazuma_settings_content::ThemeStyleContent>,
    pub theme_overrides: HashMap<String, inazuma_settings_content::ThemeStyleContent>,
    pub icon_theme: IconThemeSelection,
    pub ui_density: UiDensity,
    pub unnecessary_code_fade: f32,
}

/// Returns the name of the default theme for the given [`Appearance`].
pub fn default_theme(appearance: Appearance) -> &'static str {
    match appearance {
        Appearance::Light => inazuma_settings_content::DEFAULT_LIGHT_THEME,
        Appearance::Dark => inazuma_settings_content::DEFAULT_DARK_THEME,
    }
}

#[derive(Default)]
struct BufferFontSize(Pixels);

impl Global for BufferFontSize {}

#[derive(Default)]
pub(crate) struct UiFontSize(Pixels);

impl Global for UiFontSize {}

/// The theme selection — static or dynamic based on system appearance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThemeSelection {
    Static(ThemeName),
    Dynamic {
        mode: ThemeAppearanceMode,
        light: ThemeName,
        dark: ThemeName,
    },
}

impl From<inazuma_settings_content::ThemeSelection> for ThemeSelection {
    fn from(selection: inazuma_settings_content::ThemeSelection) -> Self {
        match selection {
            inazuma_settings_content::ThemeSelection::Static(theme) => {
                ThemeSelection::Static(theme)
            }
            inazuma_settings_content::ThemeSelection::Dynamic { mode, light, dark } => {
                ThemeSelection::Dynamic { mode, light, dark }
            }
        }
    }
}

impl ThemeSelection {
    pub fn name(&self, system_appearance: Appearance) -> ThemeName {
        match self {
            Self::Static(theme) => theme.clone(),
            Self::Dynamic { mode, light, dark } => match mode {
                ThemeAppearanceMode::Light => light.clone(),
                ThemeAppearanceMode::Dark => dark.clone(),
                ThemeAppearanceMode::System => match system_appearance {
                    Appearance::Light => light.clone(),
                    Appearance::Dark => dark.clone(),
                },
            },
        }
    }

    pub fn mode(&self) -> Option<ThemeAppearanceMode> {
        match self {
            ThemeSelection::Static(_) => None,
            ThemeSelection::Dynamic { mode, .. } => Some(*mode),
        }
    }
}

/// The icon theme selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IconThemeSelection {
    Static(IconThemeName),
    Dynamic {
        mode: ThemeAppearanceMode,
        light: IconThemeName,
        dark: IconThemeName,
    },
}

impl From<inazuma_settings_content::IconThemeSelection> for IconThemeSelection {
    fn from(selection: inazuma_settings_content::IconThemeSelection) -> Self {
        match selection {
            inazuma_settings_content::IconThemeSelection::Static(theme) => {
                IconThemeSelection::Static(theme)
            }
            inazuma_settings_content::IconThemeSelection::Dynamic { mode, light, dark } => {
                IconThemeSelection::Dynamic { mode, light, dark }
            }
        }
    }
}

impl IconThemeSelection {
    pub fn name(&self, system_appearance: Appearance) -> IconThemeName {
        match self {
            Self::Static(theme) => theme.clone(),
            Self::Dynamic { mode, light, dark } => match mode {
                ThemeAppearanceMode::Light => light.clone(),
                ThemeAppearanceMode::Dark => dark.clone(),
                ThemeAppearanceMode::System => match system_appearance {
                    Appearance::Light => light.clone(),
                    Appearance::Dark => dark.clone(),
                },
            },
        }
    }

    pub fn mode(&self) -> Option<ThemeAppearanceMode> {
        match self {
            IconThemeSelection::Static(_) => None,
            IconThemeSelection::Dynamic { mode, .. } => Some(*mode),
        }
    }
}

/// The buffer's line height.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum BufferLineHeight {
    #[default]
    Comfortable,
    Standard,
    Custom(f32),
}

impl From<inazuma_settings_content::BufferLineHeight> for BufferLineHeight {
    fn from(value: inazuma_settings_content::BufferLineHeight) -> Self {
        match value {
            inazuma_settings_content::BufferLineHeight::Comfortable => {
                BufferLineHeight::Comfortable
            }
            inazuma_settings_content::BufferLineHeight::Standard => BufferLineHeight::Standard,
            inazuma_settings_content::BufferLineHeight::Custom(line_height) => {
                BufferLineHeight::Custom(line_height)
            }
        }
    }
}

impl BufferLineHeight {
    pub fn value(&self) -> f32 {
        match self {
            BufferLineHeight::Comfortable => 1.618,
            BufferLineHeight::Standard => 1.3,
            BufferLineHeight::Custom(line_height) => *line_height,
        }
    }
}

impl ThemeSettings {
    pub fn buffer_font_size(&self, cx: &App) -> Pixels {
        let font_size = cx
            .try_global::<BufferFontSize>()
            .map(|size| size.0)
            .unwrap_or(self.buffer_font_size);
        clamp_font_size(font_size)
    }

    pub fn ui_font_size(&self, cx: &App) -> Pixels {
        let font_size = cx
            .try_global::<UiFontSize>()
            .map(|size| size.0)
            .unwrap_or(self.ui_font_size);
        clamp_font_size(font_size)
    }

    pub fn buffer_font_size_settings(&self) -> Pixels {
        self.buffer_font_size
    }

    pub fn ui_font_size_settings(&self) -> Pixels {
        self.ui_font_size
    }

    pub fn line_height(&self) -> f32 {
        f32::max(self.buffer_line_height.value(), MIN_LINE_HEIGHT)
    }

    pub fn apply_theme_overrides(&self, mut arc_theme: Arc<Theme>) -> Arc<Theme> {
        if let Some(experimental_theme_overrides) = &self.experimental_theme_overrides {
            let mut theme = (*arc_theme).clone();
            ThemeSettings::modify_theme(&mut theme, experimental_theme_overrides);
            arc_theme = Arc::new(theme);
        }

        if let Some(theme_overrides) = self.theme_overrides.get(arc_theme.name.as_ref()) {
            let mut theme = (*arc_theme).clone();
            ThemeSettings::modify_theme(&mut theme, theme_overrides);
            arc_theme = Arc::new(theme);
        }

        arc_theme
    }

    fn modify_theme(
        base_theme: &mut Theme,
        theme_overrides: &inazuma_settings_content::ThemeStyleContent,
    ) {
        let status_color_refinement = status_colors_refinement(&theme_overrides.status);

        base_theme.styles.colors.refine(&theme_colors_refinement(
            &theme_overrides.colors,
            &status_color_refinement,
        ));
        base_theme.styles.status.refine(&status_color_refinement);
        base_theme.styles.syntax = SyntaxTheme::merge(
            base_theme.styles.syntax.clone(),
            syntax_overrides(theme_overrides),
        );
    }
}

pub fn adjust_buffer_font_size(cx: &mut App, f: impl FnOnce(Pixels) -> Pixels) {
    let buffer_font_size = ThemeSettings::get_global(cx).buffer_font_size;
    let adjusted_size = cx
        .try_global::<BufferFontSize>()
        .map_or(buffer_font_size, |adjusted_size| adjusted_size.0);
    cx.set_global(BufferFontSize(clamp_font_size(f(adjusted_size))));
    cx.refresh_windows();
}

pub fn observe_buffer_font_size_adjustment<V: 'static>(
    cx: &mut inazuma::Context<V>,
    f: impl 'static + Fn(&mut V, &mut inazuma::Context<V>),
) -> inazuma::Subscription {
    cx.observe_global::<BufferFontSize>(f)
}

pub fn reset_buffer_font_size(cx: &mut App) {
    if cx.has_global::<BufferFontSize>() {
        cx.remove_global::<BufferFontSize>();
        cx.refresh_windows();
    }
}

pub fn setup_ui_font(window: &mut Window, cx: &App) -> Font {
    let (ui_font, ui_font_size) = {
        let theme_settings = ThemeSettings::get_global(cx);
        let font = theme_settings.ui_font.clone();
        (font, theme_settings.ui_font_size(cx))
    };

    window.set_rem_size(ui_font_size);
    ui_font
}

pub fn adjust_ui_font_size(cx: &mut App, f: impl FnOnce(Pixels) -> Pixels) {
    let ui_font_size = ThemeSettings::get_global(cx).ui_font_size(cx);
    let adjusted_size = cx
        .try_global::<UiFontSize>()
        .map_or(ui_font_size, |adjusted_size| adjusted_size.0);
    cx.set_global(UiFontSize(clamp_font_size(f(adjusted_size))));
    cx.refresh_windows();
}

pub fn reset_ui_font_size(cx: &mut App) {
    if cx.has_global::<UiFontSize>() {
        cx.remove_global::<UiFontSize>();
        cx.refresh_windows();
    }
}

pub fn clamp_font_size(size: Pixels) -> Pixels {
    size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
}

/// Sets the appearance mode on the theme selection in the settings content.
///
/// If the current selection is static, it converts to a dynamic selection
/// with the system defaults for light/dark. If already dynamic, it updates
/// the mode field.
pub fn set_mode(content: &mut SettingsContent, mode: ThemeAppearanceMode) {
    let theme = content.theme.as_mut();

    if let Some(selection) = theme.theme.as_mut() {
        match selection {
            inazuma_settings_content::ThemeSelection::Static(_) => {
                *selection = inazuma_settings_content::ThemeSelection::Dynamic {
                    mode: ThemeAppearanceMode::System,
                    light: ThemeName(inazuma_settings_content::DEFAULT_LIGHT_THEME.into()),
                    dark: ThemeName(inazuma_settings_content::DEFAULT_DARK_THEME.into()),
                };
            }
            inazuma_settings_content::ThemeSelection::Dynamic {
                mode: mode_to_update,
                ..
            } => *mode_to_update = mode,
        }
    } else {
        theme.theme = Some(inazuma_settings_content::ThemeSelection::Dynamic {
            mode,
            light: ThemeName(inazuma_settings_content::DEFAULT_LIGHT_THEME.into()),
            dark: ThemeName(inazuma_settings_content::DEFAULT_DARK_THEME.into()),
        });
    }
}

/// Sets the theme name on the theme selection in the settings content.
///
/// If the current selection is static, it replaces the theme name directly.
/// If dynamic, it updates the appropriate light/dark slot based on the
/// theme's appearance and the system appearance.
pub fn set_theme(
    content: &mut SettingsContent,
    theme_name: impl Into<Arc<str>>,
    theme_appearance: Appearance,
    _system_appearance: Appearance,
) {
    let theme_name = ThemeName(theme_name.into());
    let theme = content.theme.as_mut();

    let Some(selection) = theme.theme.as_mut() else {
        theme.theme = Some(inazuma_settings_content::ThemeSelection::Static(theme_name));
        return;
    };

    match selection {
        inazuma_settings_content::ThemeSelection::Static(name) => {
            *name = theme_name;
        }
        inazuma_settings_content::ThemeSelection::Dynamic { light, dark, .. } => {
            match theme_appearance {
                Appearance::Light => *light = theme_name,
                Appearance::Dark => *dark = theme_name,
            }
        }
    }
}

fn font_fallbacks_from_settings(
    fallbacks: Option<Vec<FontFamilyName>>,
) -> Option<FontFallbacks> {
    fallbacks.map(|fallbacks| {
        FontFallbacks::from_fonts(
            fallbacks
                .into_iter()
                .map(|font_family| font_family.0.to_string())
                .collect(),
        )
    })
}

impl Settings for ThemeSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        let content = &content.theme;
        let theme_selection: ThemeSelection = content
            .theme
            .clone()
            .unwrap_or_default()
            .into();
        let icon_theme_selection: IconThemeSelection = content
            .icon_theme
            .clone()
            .unwrap_or(inazuma_settings_content::IconThemeSelection::Static(
                IconThemeName(raijin_theme::DEFAULT_DARK_THEME.into()),
            ))
            .into();
        Self {
            ui_font_size: clamp_font_size(
                content
                    .ui_font_size
                    .unwrap_or(inazuma_settings_content::FontSize(15.0))
                    .into_inazuma(),
            ),
            ui_font: Font {
                family: content
                    .ui_font_family
                    .as_ref()
                    .map(|f| f.0.clone().into())
                    .unwrap_or_else(|| "DankMono Nerd Font Mono".into()),
                features: content
                    .ui_font_features
                    .clone()
                    .unwrap_or_default()
                    .into_inazuma(),
                fallbacks: font_fallbacks_from_settings(content.ui_font_fallbacks.clone()),
                weight: content
                    .ui_font_weight
                    .unwrap_or(inazuma_settings_content::FontWeightContent(400.0))
                    .into_inazuma(),
                style: Default::default(),
            },
            buffer_font: Font {
                family: content
                    .buffer_font_family
                    .as_ref()
                    .map(|f| f.0.clone().into())
                    .unwrap_or_else(|| "DankMono Nerd Font Mono".into()),
                features: content
                    .buffer_font_features
                    .clone()
                    .unwrap_or_default()
                    .into_inazuma(),
                fallbacks: font_fallbacks_from_settings(content.buffer_font_fallbacks.clone()),
                weight: content
                    .buffer_font_weight
                    .unwrap_or(inazuma_settings_content::FontWeightContent(400.0))
                    .into_inazuma(),
                style: FontStyle::default(),
            },
            buffer_font_size: clamp_font_size(
                content
                    .buffer_font_size
                    .unwrap_or(inazuma_settings_content::FontSize(15.0))
                    .into_inazuma(),
            ),
            buffer_line_height: content
                .buffer_line_height
                .unwrap_or_default()
                .into(),
            theme: theme_selection,
            experimental_theme_overrides: content.experimental_theme_overrides.clone(),
            theme_overrides: content.theme_overrides.clone(),
            icon_theme: icon_theme_selection,
            ui_density: ui_density_from_settings(content.ui_density.unwrap_or_default()),
            unnecessary_code_fade: content
                .unnecessary_code_fade
                .map(|f| f.0.clamp(0.0, 0.9))
                .unwrap_or(0.3),
        }
    }
}
