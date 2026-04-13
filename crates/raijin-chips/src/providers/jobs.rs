use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for background job count.
///
/// Reads `$RAIJIN_JOBS_COUNT` set by Raijin's shell hooks (zsh/bash/fish).
/// The hook runs `jobs -p | wc -l` before each prompt and exports the count.
///
/// Only shown when there are 1 or more background jobs.
/// Shows count with a layer icon: `"2"` → 2 background jobs.
pub struct JobsProvider;

impl ChipProvider for JobsProvider {
    fn id(&self) -> ChipId {
        "jobs"
    }

    fn display_name(&self) -> &str {
        "Jobs"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        get_job_count(ctx).is_some_and(|n| n > 0)
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let count = get_job_count(ctx).unwrap_or(0);

        ChipOutput {
            id: self.id(),
            label: count.to_string(),
            icon: Some("Layers"),
            tooltip: Some(format!(
                "{count} background {}",
                if count == 1 { "job" } else { "jobs" }
            )),
            ..ChipOutput::default()
        }
    }
}

/// Get the background job count from the environment.
///
/// Checks `RAIJIN_JOBS_COUNT` first (set by our shell hooks),
/// then falls back to `JOBS_COUNT` for compatibility.
fn get_job_count(ctx: &ChipContext) -> Option<u32> {
    ctx.get_env("RAIJIN_JOBS_COUNT")
        .or_else(|| ctx.get_env("JOBS_COUNT"))
        .and_then(|v| v.parse::<u32>().ok())
}
