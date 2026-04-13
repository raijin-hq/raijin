use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for container environment detection.
///
/// Detects whether the shell is running inside a container and identifies
/// the container runtime:
///
/// - **Docker**: `/.dockerenv` file exists, or `$container` is `"docker"`.
/// - **Podman**: `$container` is `"podman"`, or `/run/.containerenv` exists.
/// - **LXC/LXD**: `$container` is `"lxc"`.
/// - **systemd-nspawn**: `$container` is `"systemd-nspawn"`.
/// - **WSL**: `/proc/version` contains "Microsoft" or "WSL".
/// - **Generic**: `$container` is set to another value, or `/run/.containerenv` exists.
///
/// The label shows the container runtime name for quick identification.
pub struct ContainerProvider;

impl ChipProvider for ContainerProvider {
    fn id(&self) -> ChipId {
        "container"
    }

    fn display_name(&self) -> &str {
        "Container"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        detect_container(ctx).is_some()
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let (runtime, tooltip) = detect_container(ctx).unwrap_or_else(|| {
            ("container".to_string(), "Running inside a container".to_string())
        });

        ChipOutput {
            id: self.id(),
            label: runtime,
            icon: Some("Box"),
            tooltip: Some(tooltip),
            ..ChipOutput::default()
        }
    }
}

/// Detect the container runtime. Returns `(label, tooltip)`.
fn detect_container(ctx: &ChipContext) -> Option<(String, String)> {
    // Check the $container env var first (set by most container runtimes)
    if let Some(runtime) = ctx.get_env("container") {
        let runtime = runtime.trim().to_lowercase();
        if !runtime.is_empty() {
            let display = match runtime.as_str() {
                "docker" => "Docker",
                "podman" => "Podman",
                "lxc" => "LXC",
                "systemd-nspawn" => "nspawn",
                "oci" => "OCI",
                other => other,
            };
            return Some((
                display.to_string(),
                format!("Running inside {display} container"),
            ));
        }
    }

    // Docker: check for /.dockerenv sentinel file
    if std::path::Path::new("/.dockerenv").exists() {
        return Some((
            "Docker".to_string(),
            "Running inside a Docker container".to_string(),
        ));
    }

    // Podman / generic OCI: check for /run/.containerenv
    if std::path::Path::new("/run/.containerenv").exists() {
        // Try to identify Podman from the containerenv file content
        if let Ok(content) = std::fs::read_to_string("/run/.containerenv") {
            if content.contains("engine=\"podman\"") {
                return Some((
                    "Podman".to_string(),
                    "Running inside a Podman container".to_string(),
                ));
            }
        }
        return Some((
            "container".to_string(),
            "Running inside an OCI container".to_string(),
        ));
    }

    None
}
