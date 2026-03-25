use inazuma::{IntoElement, ParentElement, SharedString, div};
use inazuma_component::setting::{
    NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage, Settings,
};
use raijin_settings::RaijinConfig;

/// Build the Settings view as a full-page component (rendered inside a tab).
pub fn build_settings() -> impl IntoElement {
    Settings::new("raijin-settings")
        .sidebar_width(inazuma::px(200.0))
        .page(general_page())
        .page(appearance_page())
        .page(terminal_page())
        .page(about_page())
}

fn general_page() -> SettingPage {
    SettingPage::new("General")
        .group(
            SettingGroup::new()
                .title("Working Directory")
                .item(SettingItem::new(
                    "Start in",
                    SettingField::<SharedString>::dropdown(
                        vec![
                            ("home".into(), "Home (~)".into()),
                            ("previous".into(), "Previous Session".into()),
                        ],
                        |cx| {
                            let config = cx.global::<RaijinConfig>();
                            match &config.general.working_directory {
                                raijin_settings::WorkingDirectory::Home => "home".into(),
                                raijin_settings::WorkingDirectory::Previous => "previous".into(),
                                raijin_settings::WorkingDirectory::Custom(p) => {
                                    SharedString::from(p.clone())
                                }
                            }
                        },
                        |val, cx| {
                            let config = cx.global_mut::<RaijinConfig>();
                            config.general.working_directory = match val.as_ref() {
                                "home" => raijin_settings::WorkingDirectory::Home,
                                "previous" => raijin_settings::WorkingDirectory::Previous,
                                _ => raijin_settings::WorkingDirectory::Custom(val.to_string()),
                            };
                            let _ = config.save();
                        },
                    )
                    .default_value(SharedString::from("home")),
                )),
        )
        .group(
            SettingGroup::new()
                .title("Input Mode")
                .item(SettingItem::new(
                    "Prompt style",
                    SettingField::<SharedString>::dropdown(
                        vec![
                            ("raijin".into(), "Raijin (Context Chips)".into()),
                            ("shell_ps1".into(), "Shell PS1 (Starship, P10k)".into()),
                        ],
                        |cx| {
                            let config = cx.global::<RaijinConfig>();
                            match config.general.input_mode {
                                raijin_settings::InputMode::Raijin => "raijin".into(),
                                raijin_settings::InputMode::ShellPs1 => "shell_ps1".into(),
                            }
                        },
                        |val, cx| {
                            let config = cx.global_mut::<RaijinConfig>();
                            config.general.input_mode = match val.as_ref() {
                                "shell_ps1" => raijin_settings::InputMode::ShellPs1,
                                _ => raijin_settings::InputMode::Raijin,
                            };
                            let _ = config.save();
                        },
                    )
                    .default_value(SharedString::from("raijin")),
                )),
        )
}

fn appearance_page() -> SettingPage {
    SettingPage::new("Appearance")
        .group(
            SettingGroup::new()
                .title("Font")
                .item(SettingItem::new(
                    "Font Family",
                    SettingField::<SharedString>::input(
                        |cx| {
                            let config = cx.global::<RaijinConfig>();
                            SharedString::from(config.appearance.font_family.clone())
                        },
                        |val, cx| {
                            let config = cx.global_mut::<RaijinConfig>();
                            config.appearance.font_family = val.to_string();
                            let _ = config.save();
                        },
                    )
                    .default_value(SharedString::from(
                        raijin_settings::defaults::FONT_FAMILY,
                    )),
                ))
                .item(SettingItem::new(
                    "Font Size",
                    SettingField::<f64>::number_input(
                        NumberFieldOptions {
                            min: 8.0,
                            max: 32.0,
                            step: 1.0,
                        },
                        |cx| cx.global::<RaijinConfig>().appearance.font_size,
                        |val, cx| {
                            let config = cx.global_mut::<RaijinConfig>();
                            config.appearance.font_size = val;
                            let _ = config.save();
                        },
                    )
                    .default_value(raijin_settings::defaults::FONT_SIZE),
                )),
        )
}

fn terminal_page() -> SettingPage {
    SettingPage::new("Terminal")
        .group(
            SettingGroup::new()
                .title("Scrollback")
                .item(SettingItem::new(
                    "History Lines",
                    SettingField::<f64>::number_input(
                        NumberFieldOptions {
                            min: 0.0,
                            max: 100_000.0,
                            step: 1000.0,
                        },
                        |cx| {
                            cx.global::<RaijinConfig>().terminal.scrollback_history as f64
                        },
                        |val, cx| {
                            let config = cx.global_mut::<RaijinConfig>();
                            config.terminal.scrollback_history = val as u32;
                            let _ = config.save();
                        },
                    )
                    .default_value(raijin_settings::defaults::SCROLLBACK_HISTORY as f64),
                )),
        )
        .group(
            SettingGroup::new()
                .title("Cursor")
                .item(SettingItem::new(
                    "Cursor Style",
                    SettingField::<SharedString>::dropdown(
                        vec![
                            ("beam".into(), "Beam".into()),
                            ("block".into(), "Block".into()),
                            ("underline".into(), "Underline".into()),
                        ],
                        |cx| {
                            let config = cx.global::<RaijinConfig>();
                            match config.terminal.cursor_style {
                                raijin_settings::CursorStyle::Beam => "beam".into(),
                                raijin_settings::CursorStyle::Block => "block".into(),
                                raijin_settings::CursorStyle::Underline => "underline".into(),
                            }
                        },
                        |val, cx| {
                            let config = cx.global_mut::<RaijinConfig>();
                            config.terminal.cursor_style = match val.as_ref() {
                                "block" => raijin_settings::CursorStyle::Block,
                                "underline" => raijin_settings::CursorStyle::Underline,
                                _ => raijin_settings::CursorStyle::Beam,
                            };
                            let _ = config.save();
                        },
                    )
                    .default_value(SharedString::from("beam")),
                )),
        )
}

fn about_page() -> SettingPage {
    SettingPage::new("About").group(
        SettingGroup::new().item(SettingItem::render(|_, _, _| {
            div().child("Raijin Terminal v0.1.0")
        })),
    )
}
