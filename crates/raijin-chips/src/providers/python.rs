use std::path::Path;

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Python version and virtual environment.
///
/// Detection: `requirements.txt`, `pyproject.toml`, `Pipfile`, `setup.py`,
///            `.python-version`, `tox.ini`, extension `py`,
///            folders `.venv`, `__pycache__`
///
/// Features:
/// - Python version via `python3 --version` / `python --version`
/// - pyenv version detection via `$PYENV_VERSION` or `pyenv version-name`
/// - Virtual environment via `$VIRTUAL_ENV` with pyvenv.cfg prompt parsing
/// - Conda environment via `$CONDA_DEFAULT_ENV`
pub struct PythonProvider;

/// Generic venv directory names that should be resolved to their parent.
const GENERIC_VENV_NAMES: &[&str] = &[".venv", "venv"];

impl ChipProvider for PythonProvider {
    fn id(&self) -> ChipId {
        "python"
    }

    fn display_name(&self) -> &str {
        "Python"
    }

    fn detect_files(&self) -> &[&str] {
        &[
            "requirements.txt",
            "pyproject.toml",
            "Pipfile",
            "setup.py",
            ".python-version",
            "tox.ini",
        ]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["py"]
    }

    fn detect_folders(&self) -> &[&str] {
        &[".venv", "__pycache__"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = get_python_version(ctx).unwrap_or_default();
        let venv = get_python_virtual_env(ctx);

        let label = match venv {
            Some(ref env_name) => format!("{version} ({env_name})"),
            None => version,
        };

        let mut tooltip_parts = vec!["Python version".to_string()];
        if let Some(ref env_name) = venv {
            tooltip_parts.push(format!("venv: {env_name}"));
        }

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Python"),
            tooltip: Some(tooltip_parts.join(", ")),
            ..ChipOutput::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Python version detection
// ---------------------------------------------------------------------------

fn get_python_version(ctx: &ChipContext) -> Option<String> {
    // Try pyenv first if PYENV_VERSION is set
    if let Some(pyenv_ver) = get_pyenv_version(ctx) {
        return Some(pyenv_ver);
    }

    // Try python3 first, then python
    let binaries: &[(&str, &[&str])] = &[
        ("python3", &["--version"]),
        ("python", &["--version"]),
    ];

    for (cmd, args) in binaries {
        if let Some(output) = ctx.exec_cmd(cmd, args) {
            if let Some(version) = parse_python_version(&output.stdout) {
                return Some(version);
            }
        }
    }

    None
}

fn get_pyenv_version(ctx: &ChipContext) -> Option<String> {
    let mut version_name = ctx.get_env("PYENV_VERSION");

    if version_name.is_none() {
        version_name = Some(
            ctx.exec_cmd("pyenv", &["version-name"])?
                .stdout
                .trim()
                .to_string(),
        );
    }

    version_name
}

fn parse_python_version(python_version_string: &str) -> Option<String> {
    // Output: "Python 3.8.6" or "Python 3.6.10 :: Anaconda, Inc."
    let version = python_version_string
        .split_whitespace()
        .nth(1)?;

    Some(version.to_string())
}

// ---------------------------------------------------------------------------
// Virtual environment detection
// ---------------------------------------------------------------------------

fn get_python_virtual_env(ctx: &ChipContext) -> Option<String> {
    // Check $VIRTUAL_ENV first (standard venv/virtualenv)
    if let Some(venv) = ctx.get_env("VIRTUAL_ENV") {
        let venv_path = Path::new(&venv);
        return get_prompt_from_venv(venv_path)
            .or_else(|| get_venv_from_path(venv_path))
            .and_then(|venv_name| {
                if GENERIC_VENV_NAMES.contains(&venv_name.as_str()) {
                    get_venv_from_path(venv_path.parent()?)
                } else {
                    Some(venv_name)
                }
            });
    }

    // Fall back to $CONDA_DEFAULT_ENV
    ctx.get_env("CONDA_DEFAULT_ENV")
}

/// Read the `prompt` field from pyvenv.cfg in the venv directory.
///
/// pyvenv.cfg is an INI-like file created by `python -m venv`. The `prompt`
/// key overrides the default venv display name. Strips surrounding
/// parentheses that some tools add.
fn get_prompt_from_venv(venv_path: &Path) -> Option<String> {
    let cfg_path = venv_path.join("pyvenv.cfg");
    let content = std::fs::read_to_string(cfg_path).ok()?;

    for line in content.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("prompt") {
            let value = value.trim_start_matches(|c: char| c == '=' || c.is_whitespace());
            let value = value.trim();
            if !value.is_empty() {
                // Strip surrounding quotes and parentheses
                let value = value.trim_matches('\'').trim_matches('"');
                let value = value.trim_matches(&['(', ')'] as &[_]);
                return Some(value.to_string());
            }
        }
    }

    None
}

fn get_venv_from_path(venv_path: &Path) -> Option<String> {
    venv_path.file_name()?.to_str().map(|s| s.to_string())
}
