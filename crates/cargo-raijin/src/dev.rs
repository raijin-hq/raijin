use anyhow::{bail, Context, Result};
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::path::Path;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use crate::bundle::{self, BundleConfig};

/// Run in development mode: build, bundle, launch, watch, rebuild.
pub fn run(workspace_root: &Path, release: bool) -> Result<()> {
    let profile = if release { "release" } else { "debug" };
    let target_dir = workspace_root.join("target").join(profile);
    let binary_path = target_dir.join("raijin");
    let assets_dir = workspace_root.join("crates/raijin-app/assets");

    // Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .context("failed to set Ctrl+C handler")?;

    // Initial build + bundle + launch
    if !cargo_build(workspace_root, release)? {
        bail!("initial build failed");
    }

    let bundle_path = bundle::create_app_bundle(&BundleConfig {
        app_name: "Raijin",
        bundle_id: "dev.nyxb.raijin",
        version: "0.1.0",
        binary_path: &binary_path,
        assets_dir: &assets_dir,
        output_dir: &target_dir,
    })?;

    let mut child = launch_app(&bundle_path)?;

    // Set up file watcher
    let (tx, rx) = mpsc::channel();
    let mut debouncer =
        new_debouncer(Duration::from_millis(500), tx).context("failed to create file watcher")?;

    let watch_dirs = [
        "crates/raijin-app/src",
        "crates/raijin-terminal/src",
        "crates/raijin-shell/src",
        "crates/raijin-ui/src",
        "crates/inazuma-component/ui/src",
    ];

    for dir in &watch_dirs {
        let path = workspace_root.join(dir);
        if path.exists() {
            debouncer
                .watcher()
                .watch(&path, RecursiveMode::Recursive)
                .with_context(|| format!("failed to watch {}", path.display()))?;
        }
    }

    log::info!("👁 Watching for changes...");

    // Watch loop
    while running.load(Ordering::SeqCst) {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Ok(events)) => {
                // Filter: only .rs files
                let has_rs_change = events
                    .iter()
                    .any(|e| e.path.extension().is_some_and(|ext| ext == "rs"));

                if !has_rs_change {
                    continue;
                }

                log::info!("🔄 Change detected, rebuilding...");

                // Kill running app
                let _ = child.kill();
                let _ = child.wait();

                // Rebuild
                if cargo_build(workspace_root, release)? {
                    let _ = bundle::create_app_bundle(&BundleConfig {
                        app_name: "Raijin",
                        bundle_id: "dev.nyxb.raijin",
                        version: "0.1.0",
                        binary_path: &binary_path,
                        assets_dir: &assets_dir,
                        output_dir: &target_dir,
                    });

                    child = launch_app(&bundle_path)?;
                    log::info!("👁  Watching for changes...");
                } else {
                    log::error!("Build failed, waiting for next change...");
                }
            }
            Ok(Err(err)) => {
                log::warn!("watch error: {err}");
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    // Cleanup
    log::info!("Shutting down...");
    let _ = child.kill();
    let _ = child.wait();

    Ok(())
}

fn cargo_build(workspace_root: &Path, release: bool) -> Result<bool> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("-p")
        .arg("raijin-app")
        .current_dir(workspace_root);

    if release {
        cmd.arg("--release");
    }

    let status = cmd.status().context("failed to run cargo build")?;
    Ok(status.success())
}

fn launch_app(bundle_path: &Path) -> Result<Child> {
    // Launch the binary directly from the .app bundle so we get a real
    // Child handle for kill/restart. Using `open` detaches the process
    // and opens new instances on each rebuild.
    let binary = bundle_path.join("Contents/MacOS/raijin");
    log::info!("🚀 Launching {}", binary.display());

    // Set RAIJIN_BUNDLE_PATH so the app knows it's running from a bundle
    // (macOS needs this for icon loading via Assets.car)
    Command::new(&binary)
        .env("RAIJIN_BUNDLE_PATH", bundle_path)
        .spawn()
        .with_context(|| format!("failed to launch {}", binary.display()))
}
