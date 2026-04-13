use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for OCaml language version.
///
/// Detection: `dune-project`, `.ocamlformat`, `*.opam`, `dune`, `jbuild`, `.merlin`,
///   `_opam/`, `esy.lock/`, `.ml`, `.mli`, `.re`, `.rei` files.
/// Version: `ocaml -vnum` -> `5.1.0` (or `esy ocaml -vnum` for esy projects).
/// Also shows the active opam switch name via `opam switch show --safe`.
///

pub struct OcamlProvider;

impl ChipProvider for OcamlProvider {
    fn id(&self) -> ChipId {
        "ocaml"
    }

    fn display_name(&self) -> &str {
        "OCaml"
    }

    fn detect_files(&self) -> &[&str] {
        &["dune-project", ".ocamlformat", "dune", "jbuild", "jbuild-ignore", ".merlin"]
    }

    fn detect_folders(&self) -> &[&str] {
        &["_opam", "esy.lock"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["ml", "mli", "re", "rei", "opam"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let is_esy = ctx.dir_contents.has_folder("esy.lock");

        // Get OCaml version — esy or direct
        let version = if is_esy {
            ctx.exec_cmd("esy", &["ocaml", "-vnum"])
                .map(|o| o.stdout.trim().to_string())
                .filter(|v| !v.is_empty())
        } else {
            ctx.exec_cmd("ocaml", &["-vnum"])
                .map(|o| o.stdout.trim().to_string())
                .filter(|v| !v.is_empty())
        }
        .unwrap_or_default();

        // Get opam switch info
        let switch = get_opam_switch(ctx);
        let tooltip = build_tooltip(&version, &switch);

        let mut label = version;
        if let Some((indicator, name)) = &switch {
            if !label.is_empty() {
                label = format!("{label} ({indicator}{name})");
            }
        }

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Ocaml"),
            tooltip,
            ..ChipOutput::default()
        }
    }
}

/// Get the active opam switch. Returns (indicator, switch_name).
///
/// - Global switch: `("", "ocaml-base-compiler.5.1.0")`
/// - Local switch:  `("*", "my-project")`
fn get_opam_switch(ctx: &ChipContext) -> Option<(String, String)> {
    let output = ctx.exec_cmd("opam", &["switch", "show", "--safe"])?;
    let switch = output.stdout.trim();
    if switch.is_empty() {
        return None;
    }

    let path = std::path::Path::new(switch);
    if path.has_root() {
        // Local switch — show just the directory name with * indicator
        let name = path.file_name()?.to_str()?.to_string();
        Some(("*".to_string(), name))
    } else {
        // Global switch — show the full switch name
        Some((String::new(), switch.to_string()))
    }
}

fn build_tooltip(version: &str, switch: &Option<(String, String)>) -> Option<String> {
    match (version.is_empty(), switch) {
        (false, Some((ind, name))) => Some(format!("OCaml {version} (switch: {ind}{name})")),
        (false, None) => Some(format!("OCaml {version}")),
        _ => None,
    }
}
