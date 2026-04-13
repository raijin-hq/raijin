use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the active Pulumi stack.
///
/// Reads `Pulumi.yaml` for the project name and uses
/// `pulumi stack --show-name` for the current stack.
///
/// Label format: "project:stack" or just "stack" if project can't be read.
pub struct PulumiProvider;

impl ChipProvider for PulumiProvider {
    fn id(&self) -> ChipId {
        "pulumi"
    }

    fn display_name(&self) -> &str {
        "Pulumi"
    }

    fn detect_files(&self) -> &[&str] {
        &["Pulumi.yaml", "Pulumi.yml"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let label = gather_pulumi(ctx).unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Cloud"),
            tooltip: Some("Pulumi stack".into()),
            ..ChipOutput::default()
        }
    }
}

fn gather_pulumi(ctx: &ChipContext) -> Option<String> {
    let project_name = read_pulumi_project_name(ctx);
    let stack = ctx
        .exec_cmd("pulumi", &["stack", "--show-name", "--non-interactive"])
        .map(|o| o.stdout.trim().to_string())
        .filter(|s| !s.is_empty());

    match (project_name, stack) {
        (Some(proj), Some(stack)) => Some(format!("{proj}:{stack}")),
        (None, Some(stack)) => Some(stack),
        (Some(proj), None) => Some(proj),
        (None, None) => None,
    }
}

/// Parse the `name:` field from `Pulumi.yaml` line-by-line.
fn read_pulumi_project_name(ctx: &ChipContext) -> Option<String> {
    let yaml_path = ctx.cwd.join("Pulumi.yaml");
    let content = std::fs::read_to_string(&yaml_path)
        .or_else(|_| std::fs::read_to_string(ctx.cwd.join("Pulumi.yml")))
        .ok()?;

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("name:") {
            let name = rest.trim().trim_matches('"').trim_matches('\'');
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}
