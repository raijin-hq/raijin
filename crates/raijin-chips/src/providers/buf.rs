use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

pub struct BufProvider;

impl ChipProvider for BufProvider {
    fn id(&self) -> ChipId { "buf" }
    fn display_name(&self) -> &str { "Buf" }

    fn detect_files(&self) -> &[&str] {
        &["buf.yaml", "buf.gen.yaml", "buf.work.yaml"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let label = gather_buf(ctx).unwrap_or_default();
        ChipOutput {
            id: self.id(), label,
            icon: Some("Package"),
            tooltip: Some("Buf version".into()),
            ..ChipOutput::default()
        }
    }
}

fn gather_buf(ctx: &ChipContext) -> Option<String> {
    let output = ctx.exec_cmd("buf", &["--version"])?;
    parse_buf_version(&output.stdout)
}


fn parse_buf_version(buf_version: &str) -> Option<String> {
    Some(buf_version.split_whitespace().next()?.to_string())
}
