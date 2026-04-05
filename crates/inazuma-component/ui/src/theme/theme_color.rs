use std::sync::Arc;

use crate::{ThemeMode, theme::DEFAULT_THEME_COLORS};

use inazuma::Oklch;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Theme colors used throughout the UI components.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub struct ThemeColor {
    /// Used for accents such as hover background on MenuItem, ListItem, etc.
    pub accent: Oklch,
    /// Used for accent text color.
    pub accent_foreground: Oklch,
    /// Accordion background color.
    pub accordion: Oklch,
    /// Accordion hover background color.
    pub accordion_hover: Oklch,
    /// Default background color.
    pub background: Oklch,
    /// Default border color
    pub border: Oklch,
    /// Button primary background color, fallback to `primary`.
    pub button_primary: Oklch,
    /// Button primary active background color, fallback to `primary_active`.
    pub button_primary_active: Oklch,
    /// Button primary text color, fallback to `primary_foreground`.
    pub button_primary_foreground: Oklch,
    /// Button primary hover background color, fallback to `primary_hover`.
    pub button_primary_hover: Oklch,
    /// Background color for GroupBox.
    pub group_box: Oklch,
    /// Text color for GroupBox.
    pub group_box_foreground: Oklch,
    /// Input caret color (Blinking cursor).
    pub caret: Oklch,
    /// Chart 1 color.
    pub chart_1: Oklch,
    /// Chart 2 color.
    pub chart_2: Oklch,
    /// Chart 3 color.
    pub chart_3: Oklch,
    /// Chart 4 color.
    pub chart_4: Oklch,
    /// Chart 5 color.
    pub chart_5: Oklch,
    /// Bullish color for candlestick charts (upward price movement).
    pub chart_bullish: Oklch,
    /// Bearish color for candlestick charts (downward price movement).
    pub chart_bearish: Oklch,
    /// Danger background color.
    pub danger: Oklch,
    /// Danger active background color.
    pub danger_active: Oklch,
    /// Danger text color.
    pub danger_foreground: Oklch,
    /// Danger hover background color.
    pub danger_hover: Oklch,
    /// Description List label background color.
    pub description_list_label: Oklch,
    /// Description List label foreground color.
    pub description_list_label_foreground: Oklch,
    /// Drag border color.
    pub drag_border: Oklch,
    /// Drop target background color.
    pub drop_target: Oklch,
    /// Default text color.
    pub foreground: Oklch,
    /// Info background color.
    pub info: Oklch,
    /// Info active background color.
    pub info_active: Oklch,
    /// Info text color.
    pub info_foreground: Oklch,
    /// Info hover background color.
    pub info_hover: Oklch,
    /// Border color for inputs such as Input, Select, etc.
    pub input: Oklch,
    /// Link text color.
    pub link: Oklch,
    /// Active link text color.
    pub link_active: Oklch,
    /// Hover link text color.
    pub link_hover: Oklch,
    /// Background color for List and ListItem.
    pub list: Oklch,
    /// Background color for active ListItem.
    pub list_active: Oklch,
    /// Border color for active ListItem.
    pub list_active_border: Oklch,
    /// Stripe background color for even ListItem.
    pub list_even: Oklch,
    /// Background color for List header.
    pub list_head: Oklch,
    /// Hover background color for ListItem.
    pub list_hover: Oklch,
    /// Muted backgrounds such as Skeleton and Switch.
    pub muted: Oklch,
    /// Muted text color, as used in disabled text.
    pub muted_foreground: Oklch,
    /// Background color for Popover.
    pub popover: Oklch,
    /// Text color for Popover.
    pub popover_foreground: Oklch,
    /// Primary background color.
    pub primary: Oklch,
    /// Active primary background color.
    pub primary_active: Oklch,
    /// Primary text color.
    pub primary_foreground: Oklch,
    /// Hover primary background color.
    pub primary_hover: Oklch,
    /// Progress bar background color.
    pub progress_bar: Oklch,
    /// Used for focus ring.
    pub ring: Oklch,
    /// Scrollbar background color.
    pub scrollbar: Oklch,
    /// Scrollbar thumb background color.
    pub scrollbar_thumb: Oklch,
    /// Scrollbar thumb hover background color.
    pub scrollbar_thumb_hover: Oklch,
    /// Secondary background color.
    pub secondary: Oklch,
    /// Active secondary background color.
    pub secondary_active: Oklch,
    /// Secondary text color, used for secondary Button text color or secondary text.
    pub secondary_foreground: Oklch,
    /// Hover secondary background color.
    pub secondary_hover: Oklch,
    /// Input selection background color.
    pub selection: Oklch,
    /// Sidebar background color.
    pub sidebar: Oklch,
    /// Sidebar accent background color.
    pub sidebar_accent: Oklch,
    /// Sidebar accent text color.
    pub sidebar_accent_foreground: Oklch,
    /// Sidebar border color.
    pub sidebar_border: Oklch,
    /// Sidebar text color.
    pub sidebar_foreground: Oklch,
    /// Sidebar primary background color.
    pub sidebar_primary: Oklch,
    /// Sidebar primary text color.
    pub sidebar_primary_foreground: Oklch,
    /// Skeleton background color.
    pub skeleton: Oklch,
    /// Slider bar background color.
    pub slider_bar: Oklch,
    /// Slider thumb background color.
    pub slider_thumb: Oklch,
    /// Success background color.
    pub success: Oklch,
    /// Success text color.
    pub success_foreground: Oklch,
    /// Success hover background color.
    pub success_hover: Oklch,
    /// Success active background color.
    pub success_active: Oklch,
    /// Switch background color.
    pub switch: Oklch,
    /// Switch thumb background color.
    pub switch_thumb: Oklch,
    /// Tab background color.
    pub tab: Oklch,
    /// Tab active background color.
    pub tab_active: Oklch,
    /// Tab active text color.
    pub tab_active_foreground: Oklch,
    /// TabBar background color.
    pub tab_bar: Oklch,
    /// TabBar segmented background color.
    pub tab_bar_segmented: Oklch,
    /// Tab text color.
    pub tab_foreground: Oklch,
    /// Table background color.
    pub table: Oklch,
    /// Table active item background color.
    pub table_active: Oklch,
    /// Table active item border color.
    pub table_active_border: Oklch,
    /// Stripe background color for even TableRow.
    pub table_even: Oklch,
    /// Table head background color.
    pub table_head: Oklch,
    /// Table head text color.
    pub table_head_foreground: Oklch,
    /// Table footer background color.
    pub table_foot: Oklch,
    /// Table footer text color.
    pub table_foot_foreground: Oklch,
    /// Table item hover background color.
    pub table_hover: Oklch,
    /// Table row border color.
    pub table_row_border: Oklch,
    /// TitleBar background color, use for Window title bar.
    pub title_bar: Oklch,
    /// TitleBar border color.
    pub title_bar_border: Oklch,
    /// Background color for Tiles.
    pub tiles: Oklch,
    /// Warning background color.
    pub warning: Oklch,
    /// Warning active background color.
    pub warning_active: Oklch,
    /// Warning hover background color.
    pub warning_hover: Oklch,
    /// Warning foreground color.
    pub warning_foreground: Oklch,
    /// Overlay background color.
    pub overlay: Oklch,
    /// Window border color.
    ///
    /// # Platform specific:
    ///
    /// This is only works on Linux, other platforms we can't change the window border color.
    pub window_border: Oklch,

    /// The base red color.
    pub red: Oklch,
    /// The base red light color.
    pub red_light: Oklch,
    /// The base green color.
    pub green: Oklch,
    /// The base green light color.
    pub green_light: Oklch,
    /// The base blue color.
    pub blue: Oklch,
    /// The base blue light color.
    pub blue_light: Oklch,
    /// The base yellow color.
    pub yellow: Oklch,
    /// The base yellow light color.
    pub yellow_light: Oklch,
    /// The base magenta color.
    pub magenta: Oklch,
    /// The base magenta light color.
    pub magenta_light: Oklch,
    /// The base cyan color.
    pub cyan: Oklch,
    /// The base cyan light color.
    pub cyan_light: Oklch,
}

impl ThemeColor {
    /// Get the default light theme colors.
    pub fn light() -> Arc<Self> {
        DEFAULT_THEME_COLORS[&ThemeMode::Light].0.clone()
    }

    /// Get the default dark theme colors.
    pub fn dark() -> Arc<Self> {
        DEFAULT_THEME_COLORS[&ThemeMode::Dark].0.clone()
    }
}
