use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

/// Compile .icon to Assets.car and generate .icns fallback via actool + iconutil.
pub fn run(workspace_root: &Path, icon_path: Option<&Path>) -> Result<()> {
    let default_icon = workspace_root.join("assets/icons/raijin.icon");
    let icon_path = icon_path.unwrap_or(&default_icon);

    if !icon_path.exists() {
        bail!("icon file not found at {}", icon_path.display());
    }

    let output_dir = workspace_root.join("crates/raijin-app/assets");
    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    compile_assets_car(icon_path, &output_dir)?;
    generate_icns(icon_path, &output_dir)?;

    log::info!("Icon assets written to {}", output_dir.display());
    Ok(())
}

/// Compile .icon → Assets.car via actool.
fn compile_assets_car(icon_path: &Path, output_dir: &Path) -> Result<()> {
    let plist_path = std::env::temp_dir().join("raijin_icon_plist.plist");

    log::info!("Compiling {} → Assets.car", icon_path.display());

    let status = Command::new("actool")
        .arg(icon_path)
        .arg("--compile")
        .arg(output_dir)
        .arg("--output-format")
        .arg("human-readable-text")
        .arg("--notices")
        .arg("--warnings")
        .arg("--errors")
        .arg("--output-partial-info-plist")
        .arg(&plist_path)
        .arg("--app-icon")
        .arg("raijin")
        .arg("--include-all-app-icons")
        .arg("--enable-on-demand-resources")
        .arg("NO")
        .arg("--development-region")
        .arg("en")
        .arg("--target-device")
        .arg("mac")
        .arg("--minimum-deployment-target")
        .arg("14.0")
        .arg("--platform")
        .arg("macosx")
        .status()
        .context("failed to run actool — is Xcode Command Line Tools installed?")?;

    if !status.success() {
        bail!("actool failed");
    }

    let _ = std::fs::remove_file(&plist_path);
    log::info!("Assets.car compiled");
    Ok(())
}

/// Generate .icns from the source PNG via sips + iconutil.
fn generate_icns(icon_path: &Path, output_dir: &Path) -> Result<()> {
    // Find the source PNG from the .icon asset
    let source_png = find_source_png(icon_path)?;

    let iconset_dir = output_dir.join("raijin.iconset");
    if iconset_dir.exists() {
        std::fs::remove_dir_all(&iconset_dir)?;
    }
    std::fs::create_dir_all(&iconset_dir)?;

    log::info!("Generating .iconset from {}", source_png.display());

    // macOS icon sizes: 16, 32, 64, 128, 256, 512 (each with @2x)
    let sizes: &[(u32, &str)] = &[
        (16, "icon_16x16.png"),
        (32, "icon_16x16@2x.png"),
        (32, "icon_32x32.png"),
        (64, "icon_32x32@2x.png"),
        (64, "icon_64x64.png"),
        (128, "icon_64x64@2x.png"),
        (128, "icon_128x128.png"),
        (256, "icon_128x128@2x.png"),
        (256, "icon_256x256.png"),
        (512, "icon_256x256@2x.png"),
        (512, "icon_512x512.png"),
        (1024, "icon_512x512@2x.png"),
    ];

    for (size, name) in sizes {
        let dest = iconset_dir.join(name);
        let status = Command::new("sips")
            .arg("-z")
            .arg(size.to_string())
            .arg(size.to_string())
            .arg(&source_png)
            .arg("--out")
            .arg(&dest)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .context("failed to run sips")?;

        if !status.success() {
            bail!("sips failed for {}x{}", size, size);
        }
    }

    // Convert iconset → .icns
    let icns_path = output_dir.join("raijin.icns");
    let status = Command::new("iconutil")
        .arg("-c")
        .arg("icns")
        .arg(&iconset_dir)
        .arg("-o")
        .arg(&icns_path)
        .status()
        .context("failed to run iconutil")?;

    if !status.success() {
        bail!("iconutil failed");
    }

    log::info!("raijin.icns generated");
    Ok(())
}

/// Find the source PNG inside a .icon bundle by reading icon.json.
fn find_source_png(icon_path: &Path) -> Result<std::path::PathBuf> {
    let json_path = icon_path.join("icon.json");
    let json_str = std::fs::read_to_string(&json_path)
        .with_context(|| format!("failed to read {}", json_path.display()))?;

    // Parse icon.json to find the image name
    let json: serde_json::Value = serde_json::from_str(&json_str)
        .with_context(|| format!("failed to parse {}", json_path.display()))?;

    let image_name = json["groups"]
        .as_array()
        .and_then(|groups| groups.first())
        .and_then(|group| group["layers"].as_array())
        .and_then(|layers| layers.first())
        .and_then(|layer| layer["image-name"].as_str())
        .context("could not find image-name in icon.json")?;

    let png_path = icon_path.join("Assets").join(image_name);
    if !png_path.exists() {
        bail!("source PNG not found at {}", png_path.display());
    }

    Ok(png_path)
}
