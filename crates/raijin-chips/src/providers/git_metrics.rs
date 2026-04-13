use regex::Regex;

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider, ChipSegment};

pub struct GitMetricsProvider;

impl ChipProvider for GitMetricsProvider {
    fn id(&self) -> ChipId {
        "git_metrics"
    }

    fn display_name(&self) -> &str {
        "Git Metrics"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.shell_context.git_branch.is_some()
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let diff = ctx
            .exec_cmd("git", &["diff", "--shortstat"])
            .map(|o| GitDiff::parse(&o.stdout))
            .unwrap_or_default();

        let mut segments = Vec::new();
        let show_added = diff.added != "0";
        let show_deleted = diff.deleted != "0";

        if show_added {
            segments.push(ChipSegment {
                text: format!("+{}", diff.added),
                color_key: Some("added"),
            });
        }
        if show_deleted {
            if !segments.is_empty() {
                segments.push(ChipSegment {
                    text: " ".to_string(),
                    color_key: None,
                });
            }
            segments.push(ChipSegment {
                text: format!("-{}", diff.deleted),
                color_key: Some("deleted"),
            });
        }

        let label = if show_added || show_deleted {
            let added_part = if show_added {
                format!("+{}", diff.added)
            } else {
                String::new()
            };
            let deleted_part = if show_deleted {
                format!("-{}", diff.deleted)
            } else {
                String::new()
            };
            format!("{added_part} {deleted_part}").trim().to_string()
        } else {
            String::new()
        };

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("GitBranch"),
            tooltip: Some("Git diff stats".into()),
            segments: if segments.is_empty() {
                None
            } else {
                Some(segments)
            },
            ..ChipOutput::default()
        }
    }
}

/// Represents the parsed output from a git diff.
#[derive(Default)]
struct GitDiff {
    added: String,
    deleted: String,
}

impl GitDiff {
    /// Returns the first capture group given a regular expression and a string.
    /// If it fails to get the capture group it will return "0".
    fn get_matched_str<'a>(diff: &'a str, re: &Regex) -> &'a str {
        match re.captures(diff) {
            Some(caps) => caps.get(1).unwrap().as_str(),
            _ => "0",
        }
    }

    /// Parses the result of 'git diff --shortstat' as a `GitDiff` struct.
    pub fn parse(diff: &str) -> Self {
        let added_re = Regex::new(r"(\d+) \w+\(\+\)").unwrap();
        let deleted_re = Regex::new(r"(\d+) \w+\(\-\)").unwrap();

        Self {
            added: Self::get_matched_str(diff, &added_re).to_owned(),
            deleted: Self::get_matched_str(diff, &deleted_re).to_owned(),
        }
    }
}
