#!/usr/bin/env bash
set -euo pipefail

# Build the binary
cargo build -p raijin-app "$@"

# Determine profile
PROFILE="debug"
for arg in "$@"; do
    if [ "$arg" = "--release" ]; then
        PROFILE="release"
    fi
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TARGET_DIR="$REPO_ROOT/target/$PROFILE"
APP_NAME="Raijin"
BUNDLE="$TARGET_DIR/$APP_NAME.app"
BUNDLE_ID="dev.nyxb.raijin"

# Create .app bundle structure
rm -rf "$BUNDLE"
mkdir -p "$BUNDLE/Contents/MacOS"
mkdir -p "$BUNDLE/Contents/Resources"

# Copy binary
cp "$TARGET_DIR/raijin" "$BUNDLE/Contents/MacOS/raijin"

# Copy Assets.car (compiled icon from Icon Composer)
cp "$SCRIPT_DIR/assets/Assets.car" "$BUNDLE/Contents/Resources/Assets.car"

# Generate Info.plist
cat > "$BUNDLE/Contents/Info.plist" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>raijin</string>
    <key>CFBundleIconName</key>
    <string>rajin</string>
    <key>LSMinimumSystemVersion</key>
    <string>14.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
PLIST

echo "✅ Built $BUNDLE"
echo "   Run with: open $BUNDLE"
