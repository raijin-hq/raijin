use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

/// Compile .icon to Assets.car via actool.
pub fn run(workspace_root: &Path, icon_path: Option<&Path>) -> Result<()> {
    let default_icon = workspace_root.join("crates/raijin-app/assets/rajin.icon");
    let icon_path = icon_path.unwrap_or(&default_icon);

    if !icon_path.exists() {
        bail!("icon file not found at {}", icon_path.display());
    }

    let output_dir = workspace_root.join("crates/raijin-app/assets");
    let plist_path = std::env::temp_dir().join("raijin_icon_plist.plist");

    log::info!("Compiling {} → Assets.car", icon_path.display());

    let status = Command::new("actool")
        .arg(icon_path)
        .arg("--compile")
        .arg(&output_dir)
        .arg("--output-format")
        .arg("human-readable-text")
        .arg("--notices")
        .arg("--warnings")
        .arg("--errors")
        .arg("--output-partial-info-plist")
        .arg(&plist_path)
        .arg("--app-icon")
        .arg("rajin")
        .arg("--include-all-app-icons")
        .arg("--enable-on-demand-resources")
        .arg("NO")
        .arg("--development-region")
        .arg("en")
        .arg("--target-device")
        .arg("mac")
        .arg("--minimum-deployment-target")
        .arg("26.0")
        .arg("--platform")
        .arg("macosx")
        .status()
        .context("failed to run actool — is Xcode Command Line Tools installed?")?;

    if !status.success() {
        bail!("actool failed");
    }

    // Clean up temp plist
    let _ = std::fs::remove_file(&plist_path);

    log::info!("✅ Assets.car written to {}", output_dir.display());
    Ok(())
}
