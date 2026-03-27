use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub struct BundleConfig<'a> {
    pub app_name: &'a str,
    pub bundle_id: &'a str,
    pub version: &'a str,
    pub binary_path: &'a Path,
    pub assets_dir: &'a Path,
    pub output_dir: &'a Path,
}

/// Create a macOS .app bundle and return the path to it.
pub fn create_app_bundle(config: &BundleConfig) -> Result<PathBuf> {
    let bundle_path = config
        .output_dir
        .join(format!("{}.app", config.app_name));

    // Clean previous bundle
    if bundle_path.exists() {
        fs::remove_dir_all(&bundle_path)
            .with_context(|| format!("failed to remove old bundle at {}", bundle_path.display()))?;
    }

    let contents = bundle_path.join("Contents");
    let macos_dir = contents.join("MacOS");
    let resources_dir = contents.join("Resources");

    fs::create_dir_all(&macos_dir)
        .with_context(|| format!("failed to create {}", macos_dir.display()))?;
    fs::create_dir_all(&resources_dir)
        .with_context(|| format!("failed to create {}", resources_dir.display()))?;

    // Copy binary
    let dest_binary = macos_dir.join("raijin");
    fs::copy(config.binary_path, &dest_binary).with_context(|| {
        format!(
            "failed to copy binary from {} to {}",
            config.binary_path.display(),
            dest_binary.display()
        )
    })?;

    // Copy Assets.car (compiled icon)
    let assets_car = config.assets_dir.join("Assets.car");
    if assets_car.exists() {
        let dest = resources_dir.join("Assets.car");
        fs::copy(&assets_car, &dest)
            .with_context(|| format!("failed to copy Assets.car to {}", dest.display()))?;
    } else {
        log::warn!("Assets.car not found at {}, skipping icon", assets_car.display());
    }

    // Copy .icns fallback (for older macOS)
    let icns_file = config.assets_dir.join("raijin.icns");
    if icns_file.exists() {
        let dest = resources_dir.join("raijin.icns");
        fs::copy(&icns_file, &dest)
            .with_context(|| format!("failed to copy raijin.icns to {}", dest.display()))?;
    }

    // Generate Info.plist
    let plist = generate_info_plist(config);
    let plist_path = contents.join("Info.plist");
    fs::write(&plist_path, plist)
        .with_context(|| format!("failed to write {}", plist_path.display()))?;

    log::info!("Bundled {}", bundle_path.display());
    Ok(bundle_path)
}

fn generate_info_plist(config: &BundleConfig) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>{name}</string>
    <key>CFBundleDisplayName</key>
    <string>{name}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_id}</string>
    <key>CFBundleVersion</key>
    <string>{version}</string>
    <key>CFBundleShortVersionString</key>
    <string>{version}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>raijin</string>
    <key>CFBundleIconFile</key>
    <string>raijin.icns</string>
    <key>CFBundleIconName</key>
    <string>AppIcon</string>
    <key>LSMinimumSystemVersion</key>
    <string>14.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
"#,
        name = config.app_name,
        bundle_id = config.bundle_id,
        version = config.version,
    )
}
