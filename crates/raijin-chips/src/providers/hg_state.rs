use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

pub struct HgStateProvider;

impl ChipProvider for HgStateProvider {
    fn id(&self) -> ChipId {
        "hg_state"
    }

    fn display_name(&self) -> &str {
        "Mercurial State"
    }

    fn detect_folders(&self) -> &[&str] {
        &[".hg"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let label = get_state_label(&ctx.cwd).unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: label.to_string(),
            icon: Some("GitMerge"),
            tooltip: Some("Mercurial repository state".into()),
            ..ChipOutput::default()
        }
    }
}

fn get_state_label(repo_root: &Path) -> Option<&'static str> {
    let hg_dir = repo_root.join(".hg");

    if hg_dir.join("rebasestate").exists() {
        Some("REBASING")
    } else if hg_dir.join("updatestate").exists() {
        Some("UPDATING")
    } else if hg_dir.join("bisect.state").exists() {
        Some("BISECTING")
    } else if hg_dir.join("graftstate").exists() {
        Some("GRAFTING")
    } else if hg_dir.join("transplant").join("journal").exists() {
        Some("TRANSPLANTING")
    } else if hg_dir.join("histedit-state").exists() {
        Some("HISTEDITING")
    } else if is_merge_state(repo_root).unwrap_or(false) {
        Some("MERGING")
    } else {
        None
    }
}

fn is_merge_state(hg_root: &Path) -> io::Result<bool> {
    let dirstate_path = hg_root.join(".hg").join("dirstate");

    let mut file = File::open(dirstate_path)?;
    let mut buffer = [0u8; 40]; // First 40 bytes: 20 for p1, 20 for p2
    file.read_exact(&mut buffer)?;

    let p2 = &buffer[20..40];
    let is_merge = p2.iter().any(|&b| b != 0);

    Ok(is_merge)
}
