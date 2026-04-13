use regex::Regex;

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

const HAXERC_VERSION_PATTERN: &str = "(?:[0-9a-zA-Z][-+0-9.a-zA-Z]+)";

pub struct HaxeProvider;

impl ChipProvider for HaxeProvider {
    fn id(&self) -> ChipId {
        "haxe"
    }

    fn display_name(&self) -> &str {
        "Haxe"
    }

    fn detect_files(&self) -> &[&str] {
        &["haxelib.json", "hxformat.json", ".haxerc"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["hx", "hxml"]
    }

    fn detect_folders(&self) -> &[&str] {
        &[".haxelib", "haxe_libraries"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = get_haxe_version(ctx).unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Haxe"),
            tooltip: Some("Haxe version".into()),
            ..ChipOutput::default()
        }
    }
}

fn get_haxe_version(ctx: &ChipContext) -> Option<String> {
    get_haxerc_version(ctx).or_else(|| {
        let cmd_output = ctx.exec_cmd("haxe", &["--version"])?;
        parse_haxe_version(cmd_output.stdout.as_str())
    })
}

fn get_haxerc_version(ctx: &ChipContext) -> Option<String> {
    let raw_json = std::fs::read_to_string(ctx.cwd.join(".haxerc")).ok()?;
    let package_json: serde_json::Value = serde_json::from_str(&raw_json).ok()?;

    let raw_version = package_json.get("version")?.as_str()?;
    if raw_version.contains('/') || raw_version.contains('\\') {
        return None;
    }
    Some(raw_version.to_string())
}

fn parse_haxe_version(raw_version: &str) -> Option<String> {
    let re = Regex::new(HAXERC_VERSION_PATTERN).ok()?;
    if !re.is_match(raw_version) {
        return None;
    }
    Some(raw_version.trim().to_string())
}
