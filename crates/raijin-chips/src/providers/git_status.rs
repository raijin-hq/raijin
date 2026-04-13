use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider, ChipSegment};

pub struct GitStatusProvider;

impl ChipProvider for GitStatusProvider {
    fn id(&self) -> ChipId {
        "git_status"
    }

    fn display_name(&self) -> &str {
        "Git Status"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.shell_context.git_branch.is_some()
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        // Run git diff --shortstat directly for accurate stats
        let stats = ctx
            .exec_cmd("git", &["diff", "--shortstat", "HEAD"])
            .and_then(|o| parse_shortstat(&o.stdout));

        let (files, insertions, deletions) = match stats {
            Some(s) => s,
            None => return ChipOutput { id: self.id(), ..ChipOutput::default() },
        };

        // Don't show if nothing changed
        if files == 0 && insertions == 0 && deletions == 0 {
            return ChipOutput { id: self.id(), ..ChipOutput::default() };
        }

        let segments = vec![
            ChipSegment {
                text: files.to_string(),
                color_key: Some("git_stats_neutral"),
            },
            ChipSegment {
                text: " \u{00b7} ".to_string(),
                color_key: Some("git_stats_neutral"),
            },
            ChipSegment {
                text: format!("+{insertions}"),
                color_key: Some("git_stats_insert"),
            },
            ChipSegment {
                text: " ".to_string(),
                color_key: None,
            },
            ChipSegment {
                text: format!("-{deletions}"),
                color_key: Some("git_stats_delete"),
            },
        ];

        ChipOutput {
            id: self.id(),
            label: format!("{files} \u{00b7} +{insertions} -{deletions}"),
            icon: Some("FileDiff"),
            segments: Some(segments),
            ..ChipOutput::default()
        }
    }
}

/// Parse `git diff --shortstat` output.
/// "3 files changed, 378 insertions(+), 165 deletions(-)" → (3, 378, 165)
fn parse_shortstat(output: &str) -> Option<(u32, u32, u32)> {
    let output = output.trim();
    if output.is_empty() {
        return None;
    }

    let mut files = 0u32;
    let mut insertions = 0u32;
    let mut deletions = 0u32;

    for part in output.split(',') {
        let part = part.trim();
        if part.contains("file") {
            if let Some(n) = part.split_whitespace().next().and_then(|s| s.parse().ok()) {
                files = n;
            }
        } else if part.contains("insertion") {
            if let Some(n) = part.split_whitespace().next().and_then(|s| s.parse().ok()) {
                insertions = n;
            }
        } else if part.contains("deletion") {
            if let Some(n) = part.split_whitespace().next().and_then(|s| s.parse().ok()) {
                deletions = n;
            }
        }
    }

    Some((files, insertions, deletions))
}
