use inazuma::Oklch;
use inazuma_refineable::Refineable;

use crate::{blue, grass, neutral, red, yellow};

/// A single status color with base, background, and border variants.
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct StatusStyle {
    /// The base color for this status.
    pub color: Oklch,
    /// The background color for this status.
    pub background: Oklch,
    /// The border color for this status.
    pub border: Oklch,
}

/// Colors representing various status conditions throughout the UI.
///
/// Each status is a [`StatusStyle`] with `color`, `background`, and `border` variants.
/// Access: `status.error.color`, `status.error.background`, `status.error.border`.
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct StatusColors {
    /// Indicates some kind of conflict, like a file changed on disk while it was open, or
    /// merge conflicts in a Git repository.
    #[refineable]
    pub conflict: StatusStyle,
    /// Indicates something new, like a new file added to a Git repository.
    #[refineable]
    pub created: StatusStyle,
    /// Indicates that something no longer exists, like a deleted file.
    #[refineable]
    pub deleted: StatusStyle,
    /// Indicates a system error, a failed operation or a diagnostic error.
    #[refineable]
    pub error: StatusStyle,
    /// Represents a hidden status, such as a file being hidden in a file tree.
    #[refineable]
    pub hidden: StatusStyle,
    /// Indicates a hint or some kind of additional information.
    #[refineable]
    pub hint: StatusStyle,
    /// Indicates that something is deliberately ignored, such as a file or operation ignored by Git.
    #[refineable]
    pub ignored: StatusStyle,
    /// Represents informational status updates or messages.
    #[refineable]
    pub info: StatusStyle,
    /// Indicates a changed or altered status, like a file that has been edited.
    #[refineable]
    pub modified: StatusStyle,
    /// Indicates something that is predicted, like automatic code completion, or generated code.
    #[refineable]
    pub predictive: StatusStyle,
    /// Represents a renamed status, such as a file that has been renamed.
    #[refineable]
    pub renamed: StatusStyle,
    /// Indicates a successful operation or task completion.
    #[refineable]
    pub success: StatusStyle,
    /// Indicates some kind of unreachable status, like a block of code that can never be reached.
    #[refineable]
    pub unreachable: StatusStyle,
    /// Represents a warning status, like an operation that is about to fail.
    #[refineable]
    pub warning: StatusStyle,
}

/// Convenience struct for diagnostic severity colors.
pub struct DiagnosticColors {
    pub error: Oklch,
    pub warning: Oklch,
    pub info: Oklch,
}

impl StatusColors {
    pub fn dark() -> Self {
        Self {
            conflict: StatusStyle {
                color: red().dark().step_9(),
                background: red().dark().step_9(),
                border: red().dark().step_9(),
            },
            created: StatusStyle {
                color: grass().dark().step_9(),
                background: grass().dark().step_9().opacity(0.25),
                border: grass().dark().step_9(),
            },
            deleted: StatusStyle {
                color: red().dark().step_9(),
                background: red().dark().step_9().opacity(0.25),
                border: red().dark().step_9(),
            },
            error: StatusStyle {
                color: red().dark().step_9(),
                background: red().dark().step_9(),
                border: red().dark().step_9(),
            },
            hidden: StatusStyle {
                color: neutral().dark().step_9(),
                background: neutral().dark().step_9(),
                border: neutral().dark().step_9(),
            },
            hint: StatusStyle {
                color: blue().dark().step_9(),
                background: blue().dark().step_9(),
                border: blue().dark().step_9(),
            },
            ignored: StatusStyle {
                color: neutral().dark().step_9(),
                background: neutral().dark().step_9(),
                border: neutral().dark().step_9(),
            },
            info: StatusStyle {
                color: blue().dark().step_9(),
                background: blue().dark().step_9(),
                border: blue().dark().step_9(),
            },
            modified: StatusStyle {
                color: yellow().dark().step_9(),
                background: yellow().dark().step_9().opacity(0.25),
                border: yellow().dark().step_9(),
            },
            predictive: StatusStyle {
                color: neutral().dark_alpha().step_9(),
                background: neutral().dark_alpha().step_9(),
                border: neutral().dark_alpha().step_9(),
            },
            renamed: StatusStyle {
                color: blue().dark().step_9(),
                background: blue().dark().step_9(),
                border: blue().dark().step_9(),
            },
            success: StatusStyle {
                color: grass().dark().step_9(),
                background: grass().dark().step_9(),
                border: grass().dark().step_9(),
            },
            unreachable: StatusStyle {
                color: neutral().dark().step_10(),
                background: neutral().dark().step_10(),
                border: neutral().dark().step_10(),
            },
            warning: StatusStyle {
                color: yellow().dark().step_9(),
                background: yellow().dark().step_9(),
                border: yellow().dark().step_9(),
            },
        }
    }

    pub fn light() -> Self {
        Self {
            conflict: StatusStyle {
                color: red().light().step_9(),
                background: red().light().step_9(),
                border: red().light().step_9(),
            },
            created: StatusStyle {
                color: grass().light().step_9(),
                background: grass().light().step_9(),
                border: grass().light().step_9(),
            },
            deleted: StatusStyle {
                color: red().light().step_9(),
                background: red().light().step_9(),
                border: red().light().step_9(),
            },
            error: StatusStyle {
                color: red().light().step_9(),
                background: red().light().step_9(),
                border: red().light().step_9(),
            },
            hidden: StatusStyle {
                color: neutral().light().step_9(),
                background: neutral().light().step_9(),
                border: neutral().light().step_9(),
            },
            hint: StatusStyle {
                color: blue().light().step_9(),
                background: blue().light().step_9(),
                border: blue().light().step_9(),
            },
            ignored: StatusStyle {
                color: neutral().light().step_9(),
                background: neutral().light().step_9(),
                border: neutral().light().step_9(),
            },
            info: StatusStyle {
                color: blue().light().step_9(),
                background: blue().light().step_9(),
                border: blue().light().step_9(),
            },
            modified: StatusStyle {
                color: yellow().light().step_9(),
                background: yellow().light().step_9(),
                border: yellow().light().step_9(),
            },
            predictive: StatusStyle {
                color: neutral().light_alpha().step_9(),
                background: neutral().light_alpha().step_9(),
                border: neutral().light_alpha().step_9(),
            },
            renamed: StatusStyle {
                color: blue().light().step_9(),
                background: blue().light().step_9(),
                border: blue().light().step_9(),
            },
            success: StatusStyle {
                color: grass().light().step_9(),
                background: grass().light().step_9(),
                border: grass().light().step_9(),
            },
            unreachable: StatusStyle {
                color: neutral().light().step_10(),
                background: neutral().light().step_10(),
                border: neutral().light().step_10(),
            },
            warning: StatusStyle {
                color: yellow().light().step_9(),
                background: yellow().light().step_9(),
                border: yellow().light().step_9(),
            },
        }
    }

    pub fn diagnostic(&self) -> DiagnosticColors {
        DiagnosticColors {
            error: self.error.color,
            warning: self.warning.color,
            info: self.info.color,
        }
    }
}
