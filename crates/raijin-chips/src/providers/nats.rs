use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the NATS messaging context.
///
/// Detection: `NATS_CONTEXT` env var or `nats context info` available
/// Label:     Context name from `nats context info --json`
pub struct NatsProvider;

impl ChipProvider for NatsProvider {
    fn id(&self) -> ChipId {
        "nats"
    }

    fn display_name(&self) -> &str {
        "NATS"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.has_env("NATS_CONTEXT")
            || ctx.exec_cmd("nats", &["context", "info"]).is_some()
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let context_name = ctx
            .exec_cmd("nats", &["context", "info", "--json"])
            .and_then(|o| parse_nats_context(&o.stdout))
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: context_name,
            icon: Some("Cloud"),
            tooltip: Some("NATS context".into()),
            ..ChipOutput::default()
        }
    }
}

fn parse_nats_context(json_str: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json_str).ok()?;
    value.get("name")?.as_str().map(|s| s.to_string())
}
