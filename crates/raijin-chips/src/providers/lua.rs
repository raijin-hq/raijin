use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Lua runtime version.
///
/// Detection: `.lua-version`, `.luarc.json`, `init.lua`, `lua/` folder, `.lua` files.
/// Version: Tries `lua -v` first, then `luajit -v`.
///   - Lua:    `Lua 5.4.6  Copyright (C) ...` -> `5.4.6`
///   - LuaJIT: `LuaJIT 2.1.0-beta3 -- Copyright ...` -> `2.1.0-beta3`
///
/// Shows which runtime is active (Lua vs LuaJIT) in the tooltip.
///

pub struct LuaProvider;

impl ChipProvider for LuaProvider {
    fn id(&self) -> ChipId {
        "lua"
    }

    fn display_name(&self) -> &str {
        "Lua"
    }

    fn detect_files(&self) -> &[&str] {
        &[".lua-version", ".luarc.json", "init.lua", "stylua.toml", ".stylua.toml"]
    }

    fn detect_folders(&self) -> &[&str] {
        &["lua"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["lua"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        // Try standard Lua first, then LuaJIT
        let (version, runtime) = get_lua_version(ctx);

        let tooltip = match (&version, runtime) {
            (v, Some(rt)) if !v.is_empty() => Some(format!("{rt} {v}")),
            (v, None) if !v.is_empty() => Some(format!("Lua {v}")),
            _ => None,
        };

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Moon"),
            tooltip,
            ..ChipOutput::default()
        }
    }
}

/// Get Lua version, trying `lua` then `luajit`.
/// Returns (version_string, runtime_name).
fn get_lua_version(ctx: &ChipContext) -> (String, Option<&'static str>) {
    if let Some(output) = ctx.exec_cmd("lua", &["-v"]) {
        let combined = if output.stdout.trim().is_empty() {
            &output.stderr
        } else {
            &output.stdout
        };
        if let Some(version) = parse_lua_version(combined) {
            return (version, Some("Lua"));
        }
    }

    if let Some(output) = ctx.exec_cmd("luajit", &["-v"]) {
        let combined = if output.stdout.trim().is_empty() {
            &output.stderr
        } else {
            &output.stdout
        };
        if let Some(version) = parse_lua_version(combined) {
            return (version, Some("LuaJIT"));
        }
    }

    (String::new(), None)
}

/// Parse Lua version from `-v` output.
///
/// - `Lua 5.4.6  Copyright (C) ...` -> `5.4.6`
/// - `LuaJIT 2.1.0-beta3 -- Copyright ...` -> `2.1.0-beta3`
///
/// Takes the second whitespace-delimited word.
fn parse_lua_version(output: &str) -> Option<String> {
    let version = output.split_whitespace().nth(1)?;
    if version.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        Some(version.to_string())
    } else {
        None
    }
}
