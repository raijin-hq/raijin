use regex::Regex;

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider, ChipSegment};

pub struct FossilMetricsProvider;

impl ChipProvider for FossilMetricsProvider {
    fn id(&self) -> ChipId {
        "fossil_metrics"
    }

    fn display_name(&self) -> &str {
        "Fossil Metrics"
    }

    fn detect_files(&self) -> &[&str] {
        &[".fslckout"]
    }

    fn detect_folders(&self) -> &[&str] {
        &[".fossil"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let output = match ctx.exec_cmd("fossil", &["diff", "-i", "--numstat"]) {
            Some(o) => o.stdout,
            None => {
                return ChipOutput {
                    id: self.id(),
                    label: String::new(),
                    icon: Some("Code"),
                    ..ChipOutput::default()
                };
            }
        };

        let stats = FossilDiff::parse(&output, true);

        let mut segments = Vec::new();
        if !stats.added.is_empty() {
            segments.push(ChipSegment {
                text: format!("+{}", stats.added),
                color_key: Some("added"),
            });
        }
        if !stats.deleted.is_empty() {
            if !segments.is_empty() {
                segments.push(ChipSegment {
                    text: " ".to_string(),
                    color_key: None,
                });
            }
            segments.push(ChipSegment {
                text: format!("-{}", stats.deleted),
                color_key: Some("deleted"),
            });
        }

        let label = if !stats.added.is_empty() || !stats.deleted.is_empty() {
            let added_part = if !stats.added.is_empty() {
                format!("+{}", stats.added)
            } else {
                String::new()
            };
            let deleted_part = if !stats.deleted.is_empty() {
                format!("-{}", stats.deleted)
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
            icon: Some("Code"),
            tooltip: Some("Fossil diff stats".into()),
            segments: if segments.is_empty() {
                None
            } else {
                Some(segments)
            },
            ..ChipOutput::default()
        }
    }
}

/// Represents the parsed output from a Fossil diff with the -i --numstat option enabled.
#[derive(Debug, PartialEq)]
struct FossilDiff<'a> {
    added: &'a str,
    deleted: &'a str,
}

impl<'a> FossilDiff<'a> {
    /// Parses the output of `fossil diff -i --numstat` as a `FossilDiff` struct.
    pub fn parse(diff_numstat: &'a str, only_nonzero_diffs: bool) -> Self {
        // Fossil formats the last line of the output as "%10d %10d TOTAL over %d changed files\n"
        // where the 1st and 2nd placeholders are the number of added and deleted lines respectively
        let re = Regex::new(r"^\s*(\d+)\s+(\d+) TOTAL over \d+ changed files?$").unwrap();

        let (added, deleted) = diff_numstat
            .lines()
            .last()
            .and_then(|s| re.captures(s))
            .and_then(|caps| {
                let added = match caps.get(1)?.as_str() {
                    "0" if only_nonzero_diffs => "",
                    s => s,
                };

                let deleted = match caps.get(2)?.as_str() {
                    "0" if only_nonzero_diffs => "",
                    s => s,
                };

                Some((added, deleted))
            })
            .unwrap_or_default();

        Self { added, deleted }
    }
}
