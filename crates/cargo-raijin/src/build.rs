use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

use crate::bundle::{self, BundleConfig};

/// Build the application and create the .app bundle.
pub fn run(workspace_root: &Path, release: bool) -> Result<()> {
    let profile = if release { "release" } else { "debug" };
    log::info!("Building raijin ({profile})...");

    // Run cargo build
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("-p")
        .arg("raijin-app")
        .current_dir(workspace_root);

    if release {
        cmd.arg("--release");
    }

    let status = cmd
        .status()
        .context("failed to run cargo build")?;

    if !status.success() {
        bail!("cargo build failed");
    }

    // Create .app bundle
    let target_dir = workspace_root.join("target").join(profile);
    let binary_path = target_dir.join("raijin");
    let assets_dir = workspace_root.join("crates/raijin-app/assets");

    let bundle_path = bundle::create_app_bundle(&BundleConfig {
        app_name: "Raijin",
        bundle_id: "dev.nyxb.raijin",
        version: "0.1.0",
        binary_path: &binary_path,
        assets_dir: &assets_dir,
        output_dir: &target_dir,
    })?;

    log::info!("✅ {}", bundle_path.display());
    log::info!("   Run with: open {}", bundle_path.display());

    Ok(())
}
