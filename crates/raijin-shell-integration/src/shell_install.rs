// Shell installation detection and platform-specific install commands.
//
// Pure data + logic — no UI rendering. The modal UI lives in raijin-terminal-view.

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
