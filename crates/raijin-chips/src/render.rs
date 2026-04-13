use inazuma::{AnyElement, App, IntoElement, Oklch, Window};
use raijin_theme::ChipColors;
use raijin_ui::Chip;

use crate::icons::icon_name_from_str;
use crate::provider::{ChipId, ChipOutput};

/// Map a chip provider ID to its theme accent color.
///
/// Core chips (Tier 1) have dedicated theme tokens.
/// All other chips (Tier 2–7) use the default `chip.text` color.
pub fn chip_theme_color(colors: &ChipColors, id: ChipId) -> Oklch {
    match id {
        "username" => colors.username,
        "hostname" => colors.hostname,
        "directory" => colors.directory,
        "time" => colors.time,
        "shell" => colors.shell,
        _ => colors.text,
    }
}

/// Render a standard chip using the generic `Chip` component from raijin-ui.
///
/// This is the default renderer used for all chips that don't register a custom
/// render function. Reads colors from theme tokens, maps icon strings to IconName.
pub fn render_standard_chip(
    output: &ChipOutput,
    colors: &ChipColors,
    _window: &mut Window,
    _cx: &App,
) -> AnyElement {
    let color = chip_theme_color(colors, output.id);

    let mut chip = Chip::new(output.label.clone()).color(color);

    if let Some(icon_str) = output.icon {
        if let Some(icon) = icon_name_from_str(icon_str) {
            chip = chip.icon(icon);
        }
    }

    if output.interactive {
        chip = chip.interactive();
    }

    chip.into_any_element()
}
