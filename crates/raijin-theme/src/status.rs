use inazuma::Oklch;

/// A single status color with base, background, and border variants.
#[derive(Clone, Debug, PartialEq)]
pub struct StatusStyle {
    /// The base color for this status.
    pub color: Oklch,
    /// The background color for this status.
    pub background: Oklch,
    /// The border color for this status.
    pub border: Oklch,
}

/// Colors representing various status conditions throughout the UI.
#[derive(Clone, Debug, PartialEq)]
pub struct StatusColors {
    /// Conflict status (e.g. merge conflicts).
    pub conflict: StatusStyle,
    /// Created status (e.g. new files).
    pub created: StatusStyle,
    /// Deleted status (e.g. removed files).
    pub deleted: StatusStyle,
    /// Error status (e.g. build failures).
    pub error: StatusStyle,
    /// Hidden status (e.g. hidden files).
    pub hidden: StatusStyle,
    /// Hint status (e.g. inlay hints).
    pub hint: StatusStyle,
    /// Ignored status (e.g. gitignored files).
    pub ignored: StatusStyle,
    /// Info status (e.g. informational messages).
    pub info: StatusStyle,
    /// Modified status (e.g. changed files).
    pub modified: StatusStyle,
    /// Predictive status (e.g. AI predictions).
    pub predictive: StatusStyle,
    /// Renamed status (e.g. renamed files).
    pub renamed: StatusStyle,
    /// Success status (e.g. passed tests).
    pub success: StatusStyle,
    /// Unreachable status (e.g. dead code).
    pub unreachable: StatusStyle,
    /// Warning status (e.g. compiler warnings).
    pub warning: StatusStyle,
}
