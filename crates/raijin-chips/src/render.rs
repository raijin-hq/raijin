use inazuma::{AnyElement, App, InteractiveElement, IntoElement, Oklch, ParentElement, StatefulInteractiveElement, Styled, Window, div, px};
use raijin_theme::{ActiveTheme, ChipColors};
use raijin_ui::{Chip, ChipTooltip, Icon, IconSize, Color, h_flex};

use crate::icons::icon_name_from_str;
use crate::provider::{ChipId, ChipOutput};

/// Map a chip provider ID to its theme accent color.
///
/// Core chips (Tier 1) have dedicated theme tokens.
/// All other chips (Tier 2–7) use the default `chip.text` color.
pub fn chip_theme_color(colors: &ChipColors, id: ChipId) -> Oklch {
    match id {
        // Tier 1
        "username" => colors.username,
        "hostname" => colors.hostname,
        "directory" => colors.directory,
        "time" => colors.time,
        "shell" => colors.shell,

        // Languages
        "nodejs" => BOLD_GREEN,
        "python" => BOLD_YELLOW,
        "rust" => BOLD_RED,
        "golang" | "go" => BOLD_CYAN,
        "java" | "kotlin" | "scala" => BOLD_RED,
        "ruby" | "crystal" | "erlang" => BOLD_RED,
        "php" | "perl" | "raku" => BOLD_PURPLE,
        "swift" | "bun" => BOLD_RED,
        "deno" | "fennel" | "gleam" | "xmake" => BOLD_GREEN,
        "dotnet" | "dart" | "lua" | "cobol" | "cmake" | "buf" | "opa" | "solidity" | "rlang" | "odin" => BOLD_BLUE,
        "elixir" | "haskell" | "julia" | "fortran" | "fossil_branch" | "hg_branch" | "pijul_channel" | "mise" | "nats" => BOLD_PURPLE,
        "elm" | "daml" | "vagrant" | "kubernetes" => BOLD_CYAN,
        "nim" | "ocaml" | "zig" | "openstack" | "pixi" | "vcsh" | "shlvl" | "cmd_duration" => BOLD_YELLOW,
        "mojo" | "package" => BOLD_ORANGE,
        "c" | "cpp" => BOLD_GREEN,  // 149 ≈ green
        "haxe" | "terraform" | "meson" => BOLD_BLUE,

        // DevOps
        "aws" => BOLD_YELLOW,
        "azure" | "docker_context" | "gcloud" | "nix_shell" | "sudo" | "jobs" | "spack" => BOLD_BLUE,
        "helm" | "os" | "purescript" => colors.text,
        "battery" | "container" | "git_status" | "status" | "red" => BOLD_RED,
        "conda" | "git_commit" => BOLD_GREEN,
        "git_branch" => BOLD_PURPLE,
        "git_state" | "guix_shell" | "direnv" | "localip" => BOLD_YELLOW,
        "memory_usage" | "singularity" | "netns" => BOLD_BLUE,

        _ => colors.text,
    }
}


const BOLD_RED: Oklch = Oklch { l: 0.7227, c: 0.1589, h: 10.28, a: 1.0 };
const BOLD_GREEN: Oklch = Oklch { l: 0.8441, c: 0.1991, h: 156.83, a: 1.0 };
const BOLD_BLUE: Oklch = Oklch { l: 0.719, c: 0.1322, h: 264.2, a: 1.0 };
const BOLD_YELLOW: Oklch = Oklch { l: 0.7839, c: 0.1057, h: 75.43, a: 1.0 };
const BOLD_CYAN: Oklch = Oklch { l: 0.82, c: 0.1051, h: 235.72, a: 1.0 };
const BOLD_PURPLE: Oklch = Oklch { l: 0.7515, c: 0.1344, h: 299.5, a: 1.0 };
const BOLD_ORANGE: Oklch = Oklch { l: 0.787, c: 0.1373, h: 50.56, a: 1.0 };

/// Map a segment color_key to a theme color.
fn segment_color(colors: &ChipColors, key: Option<&str>) -> Oklch {
    match key {
        Some("git_stats_neutral") => colors.git_stats_neutral,
        Some("git_stats_insert") => colors.git_stats_insert,
        Some("git_stats_delete") => colors.git_stats_delete,
        Some("git_branch_icon") => colors.git_branch_icon,
        Some("git_branch_text") => colors.git_branch_text,
        _ => colors.text,
    }
}

/// Render a standard chip using the generic `Chip` component from raijin-ui.
///
/// Handles both simple labels and multi-segment chips (like git stats with colored +/- text).
pub fn render_standard_chip(
    output: &ChipOutput,
    colors: &ChipColors,
    _window: &mut Window,
    cx: &App,
) -> AnyElement {
    // Multi-segment chips (e.g., git_status: "17 · +401 -147" with colored segments)
    if let Some(ref segments) = output.segments {
        let chip_colors = &cx.theme().colors().chip;

        let mut container = h_flex()
            .gap(px(4.0))
            .items_center()
            .px(px(6.0))
            .py(px(2.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(chip_colors.border)
            .bg(chip_colors.background)
            .text_xs();

        // Add icon if present
        if let Some(icon_str) = output.icon
            && let Some(icon) = icon_name_from_str(icon_str)
        {
            let icon_color = segment_color(colors, segments.first().and_then(|s| s.color_key));
            container = container.child(
                Icon::new(icon)
                    .size(IconSize::Small)
                    .color(Color::Custom(icon_color)),
            );
        }

        // Add each segment with its own color
        for seg in segments {
            let color = segment_color(colors, seg.color_key);
            container = container.child(
                div().text_color(color).child(seg.text.clone()),
            );
        }

        if let Some(ref tip) = output.tooltip {
            let tip = tip.clone();
            return container
                .id(output.id)
                .tooltip(ChipTooltip::text(tip))
                .tooltip_placement(inazuma::TooltipPlacement::AboveElement)
                .into_any_element();
        }

        return container.into_any_element();
    }

    // Simple label chip
    let color = chip_theme_color(colors, output.id);

    let mut chip = Chip::new(output.label.clone()).color(color);

    if let Some(icon_str) = output.icon
        && let Some(icon) = icon_name_from_str(icon_str)
    {
        chip = chip.icon(icon);
    }

    if output.interactive {
        chip = chip.interactive();
    }

    if let Some(ref tip) = output.tooltip {
        let tip = tip.clone();
        chip = chip.tooltip(ChipTooltip::text(tip));
    }

    chip.into_any_element()
}
