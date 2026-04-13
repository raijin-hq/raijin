use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

use ini::Ini;
use jsonc_parser::ParseOptions;
use quick_xml::Reader as QXReader;
use quick_xml::events::Event as QXEvent;
use regex::Regex;
use serde_json as json;
use std::fs;
use std::io::Read;
use versions::Version;

pub struct PackageProvider;

impl ChipProvider for PackageProvider {
    fn id(&self) -> ChipId { "package" }
    fn display_name(&self) -> &str { "Package" }
    fn detect_files(&self) -> &[&str] {
        &["package.json", "Cargo.toml", "pyproject.toml", "setup.cfg",
          "composer.json", "build.gradle", "build.gradle.kts",
          "Project.toml", "mix.exs", "Chart.yaml", "pom.xml",
          "meson.build", "shard.yml", "v.mod", "vpkg.json",
          "build.sbt", "daml.yaml", "pubspec.yaml", "DESCRIPTION",
          "galaxy.yml", "jsr.json", "deno.json"]
    }
    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = get_version(ctx).unwrap_or_default();
        ChipOutput {
            id: self.id(), label: version,
            icon: Some("Package"),
            tooltip: Some("Package version".into()),
            ..ChipOutput::default()
        }
    }
}

fn read_cwd_file(ctx: &ChipContext, name: &str) -> Option<String> {
    std::fs::read_to_string(ctx.cwd.join(name)).ok()
}
fn read_path_file(path: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}
fn format_version(version: &str) -> Option<String> {
    let c = version.replace('"', "").trim().trim_start_matches('v').to_string();
    if c.is_empty() { None } else { Some(format!("v{c}")) }
}


fn get_node_package_version(ctx: &ChipContext) -> Option<String> {
    let file_contents = read_cwd_file(ctx, "package.json")?;
    let package_json: json::Value = json::from_str(&file_contents).ok()?;

    if !true
        && package_json.get("private").and_then(json::Value::as_bool) == Some(true)
    {
        return None;
    }

    let raw_version = package_json.get("version")?.as_str()?;
    if raw_version == "null" {
        return None;
    }

    let formatted_version = format_version(raw_version)?;
    if formatted_version == "v0.0.0-development" || formatted_version.starts_with("v0.0.0-semantic")
    {
        return Some("semantic".to_string());
    }

    Some(formatted_version)
}

fn get_jsr_package_version(ctx: &ChipContext) -> Option<String> {
    let (filename, contents) = ["deno.json", "deno.jsonc", "jsr.json", "jsr.jsonc"]
        .iter()
        .find_map(|filename| {
            read_cwd_file(ctx, filename)
                .map(|contents| (*filename, contents))
        })?;

    let json_content: json::Value = if filename.ends_with(".jsonc") {
        jsonc_parser::parse_to_serde_value(&contents, &ParseOptions::default()).ok()?
    } else {
        json::from_str(&contents).ok()?
    };

    let raw_version = json_content.get("version")?.as_str()?;
    format_version(raw_version)
}

fn get_poetry_version(pyproject: &toml::Table) -> Option<String> {
    pyproject
        .get("tool")?
        .get("poetry")?
        .get("version")?
        .as_str()
        .map(|s| s.to_owned())
}

fn parse_file_version_for_hatchling(ctx: &ChipContext, path: &str) -> Option<String> {
    let file_contents = read_path_file(&ctx.cwd.join(path))?;
    // https://hatch.pypa.io/latest/version/
    let re = Regex::new(r#"(__version__|VERSION)\s*=\s*["']([^"']+)["']"#).ok()?;
    Some(
        re.captures(&file_contents)
            .and_then(|cap| cap.get(2))?
            .as_str()
            .to_owned(),
    )
}

fn parse_hatchling_dynamic_version(ctx: &ChipContext, pyproject: &toml::Table) -> Option<String> {
    let version_path = pyproject
        .get("tool")?
        .get("hatch")?
        .get("version")?
        .get("path")?
        .as_str()?;

    parse_file_version_for_hatchling(ctx, version_path)
        .filter(|s| Version::new(s.as_str()).is_some())
}

fn parse_pep621_dynamic_version(ctx: &ChipContext, pyproject: &toml::Table) -> Option<String> {
    // TODO: Flit, PDM, Setuptools
    // https://packaging.python.org/en/latest/discussions/single-source-version#build-system-version-handling
    parse_hatchling_dynamic_version(ctx, pyproject)
}

fn get_pep621_dynamic_version(ctx: &ChipContext, pyproject: &toml::Table) -> Option<String> {
    pyproject
        .get("project")?
        .get("dynamic")?
        .as_array()?
        .iter()
        .any(|v| v.as_str() == Some("version"))
        .then(|| parse_pep621_dynamic_version(ctx, pyproject))?
}

fn get_pep621_static_version(pyproject: &toml::Table) -> Option<String> {
    pyproject
        .get("project")?
        .get("version")?
        .as_str()
        .map(|s| s.to_owned())
}

fn get_pep621_version(ctx: &ChipContext, pyproject: &toml::Table) -> Option<String> {
    get_pep621_static_version(pyproject).or_else(|| get_pep621_dynamic_version(ctx, pyproject))
}

fn get_pyproject_version(ctx: &ChipContext) -> Option<String> {
    let file_contents = read_cwd_file(ctx, "pyproject.toml")?;
    let pyproject_toml: toml::Table = toml::from_str(&file_contents).ok()?;

    get_pep621_version(ctx, &pyproject_toml)
        .or_else(|| get_poetry_version(&pyproject_toml))
        .and_then(|raw_version| format_version(&raw_version))
}

fn get_setup_cfg_version(ctx: &ChipContext) -> Option<String> {
    let file_contents = read_cwd_file(ctx, "setup.cfg")?;
    let ini = Ini::load_from_str(&file_contents).ok()?;
    let raw_version = ini.get_from(Some("metadata"), "version")?;

    if raw_version.starts_with("attr:") || raw_version.starts_with("file:") {
        None
    } else {
        format_version(raw_version)
    }
}

fn get_gradle_version(ctx: &ChipContext) -> Option<String> {
    read_cwd_file(ctx, "gradle.properties")
        .and_then(|contents| {
            let re = Regex::new(r"(?m)^\s*version\s*=\s*(?P<version>.*)").unwrap();
            let caps = re.captures(&contents)?;
            format_version(&caps["version"])
        }).or_else(|| {
            let build_file_contents = read_cwd_file(ctx, "build.gradle")?;
            let re = Regex::new(r#"(?m)^version( |\s*=\s*)['"](?P<version>[^'"]+)['"]$"#).unwrap(); /*dark magic*/
            let caps = re.captures(&build_file_contents)?;
            format_version(&caps["version"])

        })
}

fn get_composer_version(ctx: &ChipContext) -> Option<String> {
    let file_contents = read_cwd_file(ctx, "composer.json")?;
    let composer_json: json::Value = json::from_str(&file_contents).ok()?;
    let raw_version = composer_json.get("version")?.as_str()?;

    format_version(raw_version)
}

fn get_julia_project_version(ctx: &ChipContext) -> Option<String> {
    let file_contents = read_cwd_file(ctx, "Project.toml")?;
    let project_toml: toml::Table = toml::from_str(&file_contents).ok()?;
    let raw_version = project_toml.get("version")?.as_str()?;

    format_version(raw_version)
}

fn get_helm_package_version(ctx: &ChipContext) -> Option<String> {
    let file_contents = read_cwd_file(ctx, "Chart.yaml")?;
    let yaml = yaml_rust2::YamlLoader::load_from_str(&file_contents).ok()?;
    let version = yaml.first()?["version"].as_str()?;

    format_version(version)
}

fn get_mix_version(ctx: &ChipContext) -> Option<String> {
    let file_contents = read_cwd_file(ctx, "mix.exs")?;
    let re = Regex::new(r#"(?m)version: "(?P<version>[^"]+)""#).unwrap();
    let caps = re.captures(&file_contents)?;

    format_version(&caps["version"])
}

fn get_maven_version(ctx: &ChipContext) -> Option<String> {
    let file_contents = read_cwd_file(ctx, "pom.xml")?;
    let mut reader = QXReader::from_str(&file_contents);
    reader.config_mut().trim_text(true);

    let mut buf = vec![];
    let mut in_ver = false;
    let mut depth = 0;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(QXEvent::Start(ref e)) => {
                in_ver = depth == 1 && e.name().as_ref() == b"version";
                depth += 1;
            }
            Ok(QXEvent::End(_)) => {
                in_ver = false;
                depth -= 1;
            }
            Ok(QXEvent::Text(t)) if in_ver => {
                let ver = t.decode().ok().map(std::borrow::Cow::into_owned);
                return match ver {
                    // Ignore version which is just a property reference
                    Some(ref v) if !v.starts_with('$') => format_version(v),
                    _ => None,
                };
            }
            Ok(QXEvent::Eof) => break,
            Ok(_) => (),

            Err(err) => {
                log::warn!("Error parsing pom.xml`:\n{err}");
                break;
            }
        }
    }

    None
}

fn get_meson_version(ctx: &ChipContext) -> Option<String> {
    let file_contents = read_cwd_file(ctx, "meson.build")?
        .split_ascii_whitespace()
        .collect::<String>();

    let re = Regex::new(r"project\([^())]*,version:'(?P<version>[^']+)'[^())]*\)").unwrap();
    let caps = re.captures(&file_contents)?;

    format_version(&caps["version"])
}

fn get_vmod_version(ctx: &ChipContext) -> Option<String> {
    let file_contents = read_cwd_file(ctx, "v.mod")?;
    let re = Regex::new(r"(?m)^\s*version\s*:\s*'(?P<version>[^']+)'").unwrap();
    let caps = re.captures(&file_contents)?;
    format_version(&caps["version"])
}

fn get_vpkg_version(ctx: &ChipContext) -> Option<String> {
    let file_contents = read_cwd_file(ctx, "vpkg.json")?;
    let vpkg_json: json::Value = json::from_str(&file_contents).ok()?;
    let raw_version = vpkg_json.get("version")?.as_str()?;

    format_version(raw_version)
}

fn get_sbt_version(ctx: &ChipContext) -> Option<String> {
    let file_contents = read_cwd_file(ctx, "build.sbt")?;
    let re = Regex::new(r"(?m)^(.*/)*\s*version\s*:=\s*.(?P<version>[\d\.]+)").unwrap();
    let caps = re.captures(&file_contents)?;
    format_version(&caps["version"])
}

fn get_cargo_version(ctx: &ChipContext) -> Option<String> {
    let mut file_contents = read_cwd_file(ctx, "Cargo.toml")?;

    let mut cargo_toml: toml::Table = toml::from_str(&file_contents).ok()?;
    let cargo_version = cargo_toml.get("package").and_then(|p| p.get("version"));
    let raw_version = if let Some(v) = cargo_version.and_then(toml::Value::as_str) {
        // regular version string
        v
    } else if cargo_version
        .and_then(|v| v.get("workspace"))
        .and_then(toml::Value::as_bool)
        .unwrap_or_default()
    {
        // workspace version string (`package.version.workspace = true`)
        // need to read the Cargo.toml file from the workspace root
        let mut version = None;
        if let Some(workspace) = cargo_toml.get("workspace") {
            // current Cargo.toml file is also the workspace root
            version = workspace.get("package")?.get("version")?.as_str();
        } else {
            // discover the workspace root
            for path in ctx.cwd.ancestors().skip(1) {
                // Assume the workspace root is the first ancestor that contains a Cargo.toml file
                if let Ok(mut file) = fs::File::open(path.join("Cargo.toml")) {
                    file_contents.clear(); // clear the buffer for reading new Cargo.toml
                    file.read_to_string(&mut file_contents).ok()?;
                    cargo_toml = toml::from_str(&file_contents).ok()?;
                    // Read workspace.package.version
                    version = cargo_toml
                        .get("workspace")?
                        .get("package")?
                        .get("version")?
                        .as_str();
                    break;
                }
            }
        }
        version?
    } else {
        // This might be a workspace file
        cargo_toml
            .get("workspace")?
            .get("package")?
            .get("version")?
            .as_str()?
    };

    format_version(raw_version)
}

fn get_nimble_version(ctx: &ChipContext) -> Option<String> {
    if !ctx.dir_contents.has_extension("nimble") {
        return None;
    }

    let cmd_output = ctx.exec_cmd("nimble", &["dump", "--json"])?;
    let nimble_json: json::Value = json::from_str(&cmd_output.stdout).ok()?;

    let raw_version = nimble_json.get("version")?.as_str()?;

    format_version(raw_version)
}

fn get_shard_version(ctx: &ChipContext) -> Option<String> {
    let file_contents = read_cwd_file(ctx, "shard.yml")?;

    let data = yaml_rust2::YamlLoader::load_from_str(&file_contents).ok()?;
    let raw_version = data.first()?["version"].as_str()?;

    format_version(raw_version)
}

fn get_daml_project_version(ctx: &ChipContext) -> Option<String> {
    let file_contents = read_cwd_file(ctx, "daml.yaml")?;

    let daml_yaml = yaml_rust2::YamlLoader::load_from_str(&file_contents).ok()?;
    let raw_version = daml_yaml.first()?["version"].as_str()?;

    format_version(raw_version)
}

fn get_dart_pub_version(ctx: &ChipContext) -> Option<String> {
    let file_contents = read_cwd_file(ctx, "pubspec.yaml")?;

    let data = yaml_rust2::YamlLoader::load_from_str(&file_contents).ok()?;
    let raw_version = data.first()?["version"].as_str()?;

    format_version(raw_version)
}

fn get_rlang_version(ctx: &ChipContext) -> Option<String> {
    let file_contents = read_cwd_file(ctx, "DESCRIPTION")?;
    let re = Regex::new(r"(?m)^Version:\s*(?P<version>.*$)").unwrap();
    let caps = re.captures(&file_contents)?;
    format_version(&caps["version"])
}

fn get_galaxy_version(ctx: &ChipContext) -> Option<String> {
    let file_contents = read_cwd_file(ctx, "galaxy.yml")?;
    let data = yaml_rust2::YamlLoader::load_from_str(&file_contents).ok()?;
    let raw_version = data.first()?["version"].as_str()?;

    format_version(raw_version)
}

fn get_version(ctx: &ChipContext) -> Option<String> {
    let package_version_fn: Vec<fn(&ChipContext) -> Option<String>> = vec![
        get_cargo_version,
        get_nimble_version,
        get_node_package_version,
        get_jsr_package_version,
        get_pyproject_version,
        get_setup_cfg_version,
        get_composer_version,
        get_gradle_version,
        get_julia_project_version,
        get_mix_version,
        get_helm_package_version,
        get_maven_version,
        get_meson_version,
        get_shard_version,
        get_vmod_version,
        get_vpkg_version,
        get_sbt_version,
        get_daml_project_version,
        get_dart_pub_version,
        get_rlang_version,
        get_galaxy_version,
    ];

    package_version_fn.iter().find_map(|f| f(ctx))
}






#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_version() {
        assert_eq!(format_version("1.2.3"), Some("v1.2.3".to_string()));
        assert_eq!(format_version("v1.2.3"), Some("v1.2.3".to_string()));
        assert_eq!(format_version(""), None);
        assert_eq!(format_version("\"1.0.0\""), Some("v1.0.0".to_string()));
    }
}
