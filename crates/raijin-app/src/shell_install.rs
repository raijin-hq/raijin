/// Shell installation detection and modal for missing shells.
///
/// When the user selects a shell that is not installed (e.g., Nushell, Fish),
/// Raijin shows a modal dialog with platform-specific installation instructions
/// instead of silently falling back to a default shell.
use inazuma::{App, Oklch, Window, oklcha, px};

/// Target platform for install commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOS,
    Linux,
    #[allow(dead_code)]
    Windows,
}

/// A single platform-specific installation method.
#[derive(Debug, Clone)]
pub struct PlatformInstall {
    pub platform: Platform,
    pub package_manager: &'static str,
    pub command: &'static str,
    /// Command to check if this package manager is available.
    pub check: &'static str,
}

/// Information about a shell and how to install it on each platform.
#[derive(Debug, Clone)]
pub struct ShellInstallInfo {
    pub name: &'static str,
    /// The binary name used for `command -v` and shell switching (e.g. "nu", not "Nushell").
    pub binary: &'static str,
    pub url: &'static str,
    pub install_commands: &'static [PlatformInstall],
}

/// Registry of all shells Raijin knows how to install.
pub const NUSHELL: ShellInstallInfo = ShellInstallInfo {
    name: "Nushell",
    binary: "nu",
    url: "https://www.nushell.sh/book/installation.html",
    install_commands: &[
        PlatformInstall {
            platform: Platform::MacOS,
            package_manager: "brew",
            command: "brew install nushell",
            check: "brew --version",
        },
        PlatformInstall {
            platform: Platform::Linux,
            package_manager: "apt",
            command: "sudo apt install nushell",
            check: "apt --version",
        },
        PlatformInstall {
            platform: Platform::Linux,
            package_manager: "pacman",
            command: "sudo pacman -S nushell",
            check: "pacman --version",
        },
        PlatformInstall {
            platform: Platform::Linux,
            package_manager: "dnf",
            command: "sudo dnf install nushell",
            check: "dnf --version",
        },
        PlatformInstall {
            platform: Platform::MacOS,
            package_manager: "cargo",
            command: "cargo install nu",
            check: "cargo --version",
        },
        PlatformInstall {
            platform: Platform::Linux,
            package_manager: "cargo",
            command: "cargo install nu",
            check: "cargo --version",
        },
        PlatformInstall {
            platform: Platform::Windows,
            package_manager: "winget",
            command: "winget install nushell",
            check: "winget --version",
        },
        PlatformInstall {
            platform: Platform::Windows,
            package_manager: "scoop",
            command: "scoop install nu",
            check: "scoop --version",
        },
    ],
};

pub const ZSH: ShellInstallInfo = ShellInstallInfo {
    name: "Zsh",
    binary: "zsh",
    url: "https://www.zsh.org",
    install_commands: &[
        PlatformInstall {
            platform: Platform::MacOS,
            package_manager: "brew",
            command: "brew install zsh",
            check: "brew --version",
        },
        PlatformInstall {
            platform: Platform::Linux,
            package_manager: "apt",
            command: "sudo apt install zsh",
            check: "apt --version",
        },
        PlatformInstall {
            platform: Platform::Linux,
            package_manager: "pacman",
            command: "sudo pacman -S zsh",
            check: "pacman --version",
        },
        PlatformInstall {
            platform: Platform::Linux,
            package_manager: "dnf",
            command: "sudo dnf install zsh",
            check: "dnf --version",
        },
        PlatformInstall {
            platform: Platform::Windows,
            package_manager: "scoop",
            command: "scoop install zsh",
            check: "scoop --version",
        },
    ],
};

pub const BASH: ShellInstallInfo = ShellInstallInfo {
    name: "Bash",
    binary: "bash",
    url: "https://www.gnu.org/software/bash/",
    install_commands: &[
        PlatformInstall {
            platform: Platform::MacOS,
            package_manager: "brew",
            command: "brew install bash",
            check: "brew --version",
        },
        PlatformInstall {
            platform: Platform::Linux,
            package_manager: "apt",
            command: "sudo apt install bash",
            check: "apt --version",
        },
        PlatformInstall {
            platform: Platform::Linux,
            package_manager: "pacman",
            command: "sudo pacman -S bash",
            check: "pacman --version",
        },
        PlatformInstall {
            platform: Platform::Linux,
            package_manager: "dnf",
            command: "sudo dnf install bash",
            check: "dnf --version",
        },
        PlatformInstall {
            platform: Platform::Windows,
            package_manager: "winget",
            command: "winget install Git.Git",
            check: "winget --version",
        },
    ],
};

pub const FISH: ShellInstallInfo = ShellInstallInfo {
    name: "Fish",
    binary: "fish",
    url: "https://fishshell.com",
    install_commands: &[
        PlatformInstall {
            platform: Platform::MacOS,
            package_manager: "brew",
            command: "brew install fish",
            check: "brew --version",
        },
        PlatformInstall {
            platform: Platform::Linux,
            package_manager: "apt",
            command: "sudo apt install fish",
            check: "apt --version",
        },
        PlatformInstall {
            platform: Platform::Linux,
            package_manager: "pacman",
            command: "sudo pacman -S fish",
            check: "pacman --version",
        },
        PlatformInstall {
            platform: Platform::Linux,
            package_manager: "dnf",
            command: "sudo dnf install fish",
            check: "dnf --version",
        },
        PlatformInstall {
            platform: Platform::Windows,
            package_manager: "winget",
            command: "winget install fish",
            check: "winget --version",
        },
    ],
};

/// Returns the current platform.
fn current_platform() -> Platform {
    if cfg!(target_os = "macos") {
        Platform::MacOS
    } else if cfg!(target_os = "windows") {
        Platform::Windows
    } else {
        Platform::Linux
    }
}

/// Check if a shell binary is available via `command -v`.
pub fn check_shell_available(shell: &str) -> bool {
    std::process::Command::new("sh")
        .args(["-c", &format!("command -v {}", shell)])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Resolve the full path to a shell binary via `command -v`.
/// Used by NuLspClient to find the exact nu binary path.
pub fn resolve_shell_path(shell: &str) -> Option<String> {
    let output = std::process::Command::new("sh")
        .args(["-c", &format!("command -v {}", shell)])
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Find the best available package manager for installing a shell on the current platform.
pub fn detect_available_installer(shell: &ShellInstallInfo) -> Option<&'static PlatformInstall> {
    let current = current_platform();
    shell
        .install_commands
        .iter()
        .filter(|i| i.platform == current)
        .find(|i| {
            std::process::Command::new("sh")
                .args(["-c", i.check])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        })
}

/// Look up the ShellInstallInfo for a shell name, if we know how to install it.
pub fn shell_install_info(shell_name: &str) -> Option<&'static ShellInstallInfo> {
    match shell_name {
        "zsh" => Some(&ZSH),
        "bash" | "sh" => Some(&BASH),
        "fish" => Some(&FISH),
        "nu" | "nushell" => Some(&NUSHELL),
        _ => None,
    }
}

/// Modal view for shell installation — implements ModalView for the ModalLayer system.
pub struct ShellInstallModal {
    shell: &'static ShellInstallInfo,
    install_command: Option<String>,
    pm_name: Option<&'static str>,
    terminal_handle: raijin_terminal::TerminalHandle,
    focus_handle: inazuma::FocusHandle,
}

impl inazuma::EventEmitter<inazuma::DismissEvent> for ShellInstallModal {}
impl inazuma_component::modal_layer::ModalView for ShellInstallModal {
    fn fade_out_background(&self) -> bool {
        true
    }
}

impl inazuma::Focusable for ShellInstallModal {
    fn focus_handle(&self, _cx: &App) -> inazuma::FocusHandle {
        self.focus_handle.clone()
    }
}

impl ShellInstallModal {
    pub fn new(
        shell: &'static ShellInstallInfo,
        terminal_handle: raijin_terminal::TerminalHandle,
        _window: &mut Window,
        cx: &mut inazuma::Context<Self>,
    ) -> Self {
        let installer = detect_available_installer(shell);
        Self {
            shell,
            install_command: installer.map(|i| i.command.to_string()),
            pm_name: installer.map(|i| i.package_manager),
            terminal_handle,
            focus_handle: cx.focus_handle(),
        }
    }

    fn install(&mut self, _window: &mut Window, cx: &mut inazuma::Context<Self>) {
        if let Some(ref cmd) = self.install_command {
            // Set pending install flag for auto-switch on CommandEnd
            cx.global_mut::<crate::workspace::PendingShellInstallName>().0 =
                Some(self.shell.binary.to_string());
            // Set command text on block router so the block header shows the command
            {
                let mut term = self.terminal_handle.lock();
                term.set_pending_block_command(cmd.clone());
            }
            let mut bytes = cmd.as_bytes().to_vec();
            bytes.push(b'\r');
            self.terminal_handle.write(&bytes);
        } else {
            let _ = std::process::Command::new("open")
                .arg(self.shell.url)
                .spawn();
        }
        cx.emit(inazuma::DismissEvent);
    }

    fn cancel(&mut self, _window: &mut Window, cx: &mut inazuma::Context<Self>) {
        cx.emit(inazuma::DismissEvent);
    }
}

impl inazuma::Render for ShellInstallModal {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut inazuma::Context<Self>,
    ) -> impl inazuma::IntoElement {
        use inazuma::{
            Animation, AnimationExt as _, CursorStyle, ElementId, InteractiveElement as _,
            ParentElement as _, Styled as _, div, rgb,
        };
        use inazuma_component::{
            Sizable as _,
            animation::cubic_bezier,
            button::{Button, ButtonVariants as _},
            h_flex, v_flex,
        };

        let description = if let Some(pm) = self.pm_name {
            format!(
                "Raijin supports {} natively with intelligent completions.\n\
                 Install via {} to get started.",
                self.shell.name, pm,
            )
        } else {
            format!(
                "Raijin supports {} natively with intelligent completions.\n\
                 Visit the installation page to get started.",
                self.shell.name,
            )
        };

        let install_label = if self.install_command.is_some() {
            format!("Install via {}", self.pm_name.unwrap_or("package manager"))
        } else {
            "Open installation page".to_string()
        };

        v_flex()
            .key_context("ShellInstallModal")
            .track_focus(&self.focus_handle)
            .w(px(380.0))
            .bg(oklcha(0.23, 0.0, 0.0, 1.0))
            .border_1()
            .border_color(oklcha(0.30, 0.0, 0.0, 1.0))
            .rounded(px(8.0))
            .shadow_lg()
            .p(px(16.0))
            .gap(px(12.0))
            // Header
            .child(
                v_flex()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_base()
                            .font_weight(inazuma::FontWeight::SEMIBOLD)
                            .text_color(rgb(0xf1f1f1))
                            .child(format!("{} is not installed", self.shell.name)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(Oklch::white().opacity(0.5))
                            .child(description),
                    ),
            )
            // Footer buttons
            .child(
                h_flex()
                    .justify_end()
                    .gap(px(6.0))
                    .child(
                        Button::new("cancel")
                            .label("Cancel")
                            .small()
                            .cursor(CursorStyle::PointingHand)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.cancel(window, cx);
                            })),
                    )
                    .child(
                        Button::new("install")
                            .label(install_label)
                            .small()
                            .primary()
                            .cursor(CursorStyle::PointingHand)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.install(window, cx);
                            })),
                    ),
            )
            .with_animation(
                ElementId::Name("shell-install-appear".into()),
                Animation::new(std::time::Duration::from_millis(250))
                    .with_easing(cubic_bezier(0.32, 0.72, 0.0, 1.0)),
                move |this, delta| {
                    use inazuma::{BoxShadow, point};
                    let shadow = vec![
                        BoxShadow {
                            color: inazuma::Oklch::black().opacity(0.25 * delta),
                            offset: point(px(0.0), px(25.0)),
                            blur_radius: px(50.0),
                            spread_radius: px(-12.0),
                        },
                        BoxShadow {
                            color: inazuma::Oklch::black().opacity(0.15 * delta),
                            offset: point(px(0.0), px(8.0)),
                            blur_radius: px(16.0),
                            spread_radius: px(-8.0),
                        },
                    ];
                    // Slide down from -12px to 0, fade in, shadow grows
                    let offset = px(-12.0 + 12.0 * delta);
                    this.mt(offset).opacity(delta).shadow(shadow)
                },
            )
    }
}
