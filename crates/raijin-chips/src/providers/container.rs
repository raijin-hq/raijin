use std::path::Path;

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

pub struct ContainerProvider;

impl ChipProvider for ContainerProvider {
    fn id(&self) -> ChipId { "container" }
    fn display_name(&self) -> &str { "Container" }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.has_env("container") || detect_container_name().is_some()
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let name = ctx.get_env("container")
            .filter(|v| !v.is_empty())
            .or_else(|| detect_container_name())
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: name.clone(),
            icon: Some("Container"),
            tooltip: Some(format!("Running in container: {name}")),
            ..ChipOutput::default()
        }
    }
}

/// Detect container runtime on Linux by checking well-known paths.
/// Returns None on non-Linux platforms.
fn detect_container_name() -> Option<String> {
    #[cfg(not(target_os = "linux"))]
    {
        // On macOS/Windows: check /.dockerenv only
        if Path::new("/.dockerenv").exists() {
            return Some("Docker".into());
        }
        return None;
    }

    #[cfg(target_os = "linux")]
    {
        if Path::new("/proc/vz").exists() && !Path::new("/proc/bc").exists() {
            return Some("OpenVZ".into());
        }

        if Path::new("/run/host/container-manager").exists() {
            return Some("OCI".into());
        }

        if Path::new("/dev/incus/sock").exists() {
            return Some("Incus".into());
        }

        let containerenv = Path::new("/run/.containerenv");
        if containerenv.exists() {
            let name = std::fs::read_to_string(containerenv)
                .ok()
                .and_then(|s| {
                    s.lines().find_map(|l| {
                        if let Some(name_val) = l.strip_prefix("name=\"") {
                            return name_val.strip_suffix('"').map(|n| n.to_string());
                        }
                        l.starts_with("image=\"").then(|| {
                            let r = l.split_at(7).1;
                            let name = r.rfind('/').map(|n| r.split_at(n + 1).1);
                            String::from(name.unwrap_or(r).trim_end_matches('"'))
                        })
                    })
                })
                .unwrap_or_else(|| "podman".into());
            return Some(name);
        }

        let systemd_path = Path::new("/run/systemd/container");
        if let Ok(s) = std::fs::read_to_string(systemd_path) {
            match s.trim() {
                "docker" => return Some("Docker".into()),
                "wsl" => (),
                _ => return Some("Systemd".into()),
            }
        }

        if Path::new("/.dockerenv").exists() {
            return Some("Docker".into());
        }

        None
    }
}
