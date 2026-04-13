use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for ongoing git operations (merge, rebase, cherry-pick, etc.).
///
/// Detects in-progress operations by checking for sentinel files and directories
/// inside the `.git` directory. The git directory is resolved via
/// `git rev-parse --git-dir` to support worktrees and non-standard layouts.
///
/// Detection order (first match wins):
///
/// 1. `rebase-merge/` or `rebase-apply/` → REBASING (with progress if available)
/// 2. `MERGE_HEAD` → MERGING
/// 3. `CHERRY_PICK_HEAD` → CHERRY-PICKING
/// 4. `REVERT_HEAD` → REVERTING
/// 5. `BISECT_LOG` → BISECTING
/// 6. `REBASE_HEAD` → REBASING (alternative indicator)
///
/// For interactive rebase, also reads step progress from
/// `rebase-merge/msgnum` and `rebase-merge/end` to show `"REBASING 3/7"`.
pub struct GitStateProvider;

impl ChipProvider for GitStateProvider {
    fn id(&self) -> ChipId {
        "git_state"
    }

    fn display_name(&self) -> &str {
        "Git State"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        detect_git_state(ctx).is_some()
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let (label, tooltip) = detect_git_state(ctx).unwrap_or_else(|| {
            (String::new(), String::new())
        });

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("GitMerge"),
            tooltip: if tooltip.is_empty() {
                None
            } else {
                Some(tooltip)
            },
            ..ChipOutput::default()
        }
    }
}

/// Detect the current git operation state.
///
/// Returns `(label, tooltip)` or None if no operation is in progress.
fn detect_git_state(ctx: &ChipContext) -> Option<(String, String)> {
    let git_dir = resolve_git_dir(ctx)?;

    // Interactive rebase (rebase-merge directory)
    if git_dir.join("rebase-merge").is_dir() {
        let progress = read_rebase_progress(&git_dir.join("rebase-merge"));
        let label = match &progress {
            Some((current, total)) => format!("REBASING {current}/{total}"),
            None => "REBASING".to_string(),
        };
        let tooltip = match &progress {
            Some((current, total)) => {
                format!("Interactive rebase in progress — step {current} of {total}")
            }
            None => "Interactive rebase in progress".to_string(),
        };
        return Some((label, tooltip));
    }

    // Apply-style rebase (rebase-apply directory)
    if git_dir.join("rebase-apply").is_dir() {
        let progress = read_rebase_progress(&git_dir.join("rebase-apply"));
        let label = match &progress {
            Some((current, total)) => format!("REBASING {current}/{total}"),
            None => "REBASING".to_string(),
        };
        let tooltip = match &progress {
            Some((current, total)) => format!("Rebase in progress — step {current} of {total}"),
            None => "Rebase in progress".to_string(),
        };
        return Some((label, tooltip));
    }

    // Merge
    if git_dir.join("MERGE_HEAD").exists() {
        return Some(("MERGING".into(), "Merge in progress".into()));
    }

    // Cherry-pick
    if git_dir.join("CHERRY_PICK_HEAD").exists() {
        return Some(("CHERRY-PICKING".into(), "Cherry-pick in progress".into()));
    }

    // Revert
    if git_dir.join("REVERT_HEAD").exists() {
        return Some(("REVERTING".into(), "Revert in progress".into()));
    }

    // Bisect
    if git_dir.join("BISECT_LOG").exists() {
        return Some(("BISECTING".into(), "Bisect in progress".into()));
    }

    // Fallback rebase indicator
    if git_dir.join("REBASE_HEAD").exists() {
        return Some(("REBASING".into(), "Rebase in progress".into()));
    }

    None
}

/// Resolve the `.git` directory, supporting worktrees.
///
/// Uses `git rev-parse --git-dir` when in a git worktree, since the `.git`
/// entry is a file (not a directory) that points to the actual git dir.
/// Falls back to `ctx.cwd/.git` for standard repos.
fn resolve_git_dir(ctx: &ChipContext) -> Option<std::path::PathBuf> {
    let dot_git = ctx.cwd.join(".git");

    if dot_git.is_dir() {
        return Some(dot_git);
    }

    // Worktree: .git is a file containing "gitdir: /path/to/actual/.git/worktrees/name"
    if dot_git.is_file() {
        if let Ok(content) = std::fs::read_to_string(&dot_git) {
            if let Some(path) = content.strip_prefix("gitdir: ") {
                let path = path.trim();
                let git_path = std::path::PathBuf::from(path);
                if git_path.is_dir() {
                    return Some(git_path);
                }
            }
        }
    }

    // Try git rev-parse as last resort
    ctx.exec_cmd("git", &["rev-parse", "--git-dir"])
        .map(|o| {
            let path = o.stdout.trim();
            if std::path::Path::new(path).is_absolute() {
                std::path::PathBuf::from(path)
            } else {
                ctx.cwd.join(path)
            }
        })
        .filter(|p| p.is_dir())
}

/// Read rebase progress (current step / total steps).
///
/// For `rebase-merge/`: reads `msgnum` (current) and `end` (total).
/// For `rebase-apply/`: reads `next` (current) and `last` (total).
fn read_rebase_progress(rebase_dir: &std::path::Path) -> Option<(String, String)> {
    let current = read_number_file(rebase_dir, "msgnum")
        .or_else(|| read_number_file(rebase_dir, "next"))?;
    let total = read_number_file(rebase_dir, "end")
        .or_else(|| read_number_file(rebase_dir, "last"))?;
    Some((current, total))
}

/// Read a file containing a single number.
fn read_number_file(dir: &std::path::Path, filename: &str) -> Option<String> {
    let content = std::fs::read_to_string(dir.join(filename)).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() || !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(trimmed.to_string())
}
