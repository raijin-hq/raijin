use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider, parse_version_number};

/// Chip provider for Node.js runtime version.
///
/// Detection follows Node.js module patterns:
/// - Activates on `package.json`, `.nvmrc`, `.node-version`, JS extensions, `node_modules`
/// - Excludes esy projects (OCaml build system using `esy.lock`)
/// - Reads `.nvmrc` / `.node-version` for expected version constraints
/// - Reads `package.json` `engines.node` for semver range checking
/// - Reports mismatch in tooltip when installed version is outside expected range
pub struct NodejsProvider;

impl ChipProvider for NodejsProvider {
    fn id(&self) -> ChipId {
        "nodejs"
    }

    fn display_name(&self) -> &str {
        "Node.js"
    }

    fn detect_files(&self) -> &[&str] {
        &["package.json", ".nvmrc", ".node-version"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["js", "mjs", "cjs"]
    }

    fn detect_folders(&self) -> &[&str] {
        &["node_modules"]
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        // Exclude esy projects (OCaml build system that also uses package.json)
        if ctx.dir_contents.has_folder("esy.lock") {
            return false;
        }

        let files = self.detect_files();
        let folders = self.detect_folders();
        let extensions = self.detect_extensions();

        ctx.dir_contents.matches(files, folders, extensions)
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("node", &["--version"])
            .map(|o| parse_version_number(&o.stdout))
            .unwrap_or_default();

        if version.is_empty() {
            return ChipOutput {
                id: self.id(),
                label: String::new(),
                icon: Some("Hexagon"),
                tooltip: Some("Node.js not found".into()),
                ..ChipOutput::default()
            };
        }

        let expected = read_expected_version(ctx);
        let tooltip = build_tooltip(&version, expected.as_deref());

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Hexagon"),
            tooltip: Some(tooltip),
            ..ChipOutput::default()
        }
    }
}

/// Read expected Node.js version from version manager files or package.json engines.
///
/// Priority order (first found wins):
/// 1. `.node-version` — fnm / nodenv / nvm
/// 2. `.nvmrc` — nvm
/// 3. `package.json` `engines.node` — npm semver range
fn read_expected_version(ctx: &ChipContext) -> Option<String> {
    // .node-version takes priority (fnm, nodenv, asdf)
    if let Some(v) = read_version_file(ctx, ".node-version") {
        return Some(v);
    }

    // .nvmrc
    if let Some(v) = read_version_file(ctx, ".nvmrc") {
        return Some(v);
    }

    // package.json engines.node
    read_engines_node(ctx)
}

/// Read and normalize a version file (.nvmrc, .node-version).
///
/// These files contain a bare version string like `20.11.0`, `v20`, `lts/*`, or `lts/iron`.
/// We trim whitespace and strip the leading `v` prefix.
fn read_version_file(ctx: &ChipContext, filename: &str) -> Option<String> {
    let path = ctx.cwd.join(filename);
    let content = std::fs::read_to_string(&path).ok()?;
    let trimmed = content.trim();

    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

/// Read `engines.node` from package.json in the current directory.
fn read_engines_node(ctx: &ChipContext) -> Option<String> {
    let path = ctx.cwd.join("package.json");
    let content = std::fs::read_to_string(&path).ok()?;

    // Minimal JSON extraction without pulling in serde_json:
    // Find `"engines"` object, then `"node"` value within it.
    let engines_idx = content.find("\"engines\"")?;
    let after_engines = &content[engines_idx..];
    let brace_start = after_engines.find('{')?;
    let brace_section = &after_engines[brace_start..];

    // Find matching closing brace
    let mut depth = 0;
    let mut brace_end = None;
    for (i, ch) in brace_section.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    brace_end = Some(i + 1);
                    break;
                }
            }
            _ => {}
        }
    }

    let engines_block = &brace_section[..brace_end?];
    let node_idx = engines_block.find("\"node\"")?;
    let after_node = &engines_block[node_idx + 6..];

    // Skip to the colon, then find the value string
    let colon_idx = after_node.find(':')?;
    let after_colon = &after_node[colon_idx + 1..];
    let quote_start = after_colon.find('"')?;
    let value_start = &after_colon[quote_start + 1..];
    let quote_end = value_start.find('"')?;

    let value = &value_start[..quote_end];
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

/// Build tooltip with version mismatch info when applicable.
fn build_tooltip(installed: &str, expected: Option<&str>) -> String {
    match expected {
        Some(expected_ver) => {
            if version_satisfies(installed, expected_ver) {
                format!("Node.js v{installed} (matches {expected_ver})")
            } else {
                format!("Node.js v{installed} (expected {expected_ver})")
            }
        }
        None => format!("Node.js v{installed}"),
    }
}

/// Check if an installed version satisfies an expected version constraint.
///
/// Handles common patterns from .nvmrc / .node-version / engines.node:
/// - Exact match: `20.11.0` or `v20.11.0`
/// - Major only: `20` or `v20`
/// - Major.minor: `20.11` or `v20.11`
/// - LTS aliases: `lts/*`, `lts/iron` — always considered satisfied (can't resolve without registry)
/// - Semver ranges: `>=18.0.0`, `^20.0.0`, `>=18 <22` — basic prefix matching
fn version_satisfies(installed: &str, expected: &str) -> bool {
    let expected_trimmed = expected.trim();

    // LTS aliases — can't resolve without querying the Node.js release schedule
    if expected_trimmed.starts_with("lts/") || expected_trimmed == "lts" {
        return true;
    }

    // Aliases like "node", "stable", "current" — always considered matching
    if matches!(expected_trimmed, "node" | "stable" | "current" | "system") {
        return true;
    }

    let clean_expected = expected_trimmed.trim_start_matches('v');

    // Simple version prefix match (handles exact, major-only, major.minor)
    // e.g., installed="20.11.0" matches expected="20", "20.11", "20.11.0"
    if clean_expected
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit())
        && !clean_expected.contains(' ')
        && !clean_expected.starts_with('>')
        && !clean_expected.starts_with('<')
        && !clean_expected.starts_with('~')
        && !clean_expected.starts_with('^')
        && !clean_expected.starts_with('=')
    {
        // Exact or prefix match: "20" matches "20.x.y", "20.11" matches "20.11.y"
        if installed == clean_expected {
            return true;
        }
        if installed.starts_with(clean_expected)
            && installed[clean_expected.len()..]
                .starts_with('.')
        {
            return true;
        }
        return false;
    }

    // For complex semver ranges (^, ~, >=, ||, etc.) — do basic best-effort check.
    // Extract the first version number from the range and check major version match.
    if let Some(range_version) = extract_first_version(clean_expected) {
        let installed_major = installed.split('.').next().unwrap_or("");
        let range_major = range_version.split('.').next().unwrap_or("");
        // For caret (^) and tilde (~), major version match is a reasonable heuristic
        if expected_trimmed.starts_with('^') || expected_trimmed.starts_with('~') {
            return installed_major == range_major;
        }
        // For >= constraints, installed major should be >= range major
        if expected_trimmed.starts_with(">=")
            && let (Ok(inst), Ok(req)) =
                (installed_major.parse::<u32>(), range_major.parse::<u32>())
        {
            return inst >= req;
        }
        // For < constraints, installed major should be < range major
        if expected_trimmed.starts_with('<') && !expected_trimmed.starts_with("<=")
            && let (Ok(inst), Ok(req)) =
                (installed_major.parse::<u32>(), range_major.parse::<u32>())
        {
            return inst < req;
        }
        if expected_trimmed.starts_with("<=")
            && let (Ok(inst), Ok(req)) =
                (installed_major.parse::<u32>(), range_major.parse::<u32>())
        {
            return inst <= req;
        }
    }

    // Unknown format — assume satisfied to avoid false alarm
    true
}

/// Extract the first semver-like version number from a constraint string.
/// e.g., ">=18.0.0" → "18.0.0", "^20.0.0" → "20.0.0"
fn extract_first_version(s: &str) -> Option<&str> {
    let start = s.find(|c: char| c.is_ascii_digit())?;
    let version_part = &s[start..];
    let end = version_part
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(version_part.len());
    let v = &version_part[..end];
    if v.is_empty() { None } else { Some(v) }
}
