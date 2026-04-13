use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider, ChipSegment};

/// Chip provider for git diff statistics.
///
/// Visible when git stats are available. Renders multi-colored segments:
/// `{files_changed} · +{insertions} -{deletions}`
pub struct GitStatusProvider;

impl ChipProvider for GitStatusProvider {
    fn id(&self) -> ChipId {
        "git_status"
    }

    fn display_name(&self) -> &str {
        "Git Status"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.shell_context.git_stats.is_some()
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let stats = ctx
            .shell_context
            .git_stats
            .as_ref()
            .expect("is_available guards this");

        let segments = vec![
            ChipSegment {
                text: stats.files_changed.to_string(),
                color_key: Some("git_stats_neutral"),
            },
            ChipSegment {
                text: " \u{00b7} ".to_string(),
                color_key: Some("git_stats_neutral"),
            },
            ChipSegment {
                text: format!("+{}", stats.insertions),
                color_key: Some("git_stats_insert"),
            },
            ChipSegment {
                text: " ".to_string(),
                color_key: None,
            },
            ChipSegment {
                text: format!("-{}", stats.deletions),
                color_key: Some("git_stats_delete"),
            },
        ];

        ChipOutput {
            id: self.id(),
            label: format!(
                "{} \u{00b7} +{} -{}",
                stats.files_changed, stats.insertions, stats.deletions
            ),
            icon: Some("FileDiff"),
            segments: Some(segments),
            ..ChipOutput::default()
        }
    }
}
