#!/bin/zsh
# Build Manguo in release mode and wrap it in a macOS app bundle at
# ~/Applications/Manguo.app (launchable from Spotlight / Launchpad, drag it to
# the Dock for a shortcut).
#
# NOTE: makepad resolves `dep("crate://self/resources/...")` and the presets
# through the compile-time path of this checkout, so the bundle depends on
# this repository staying where it is. Re-run this script after moving or
# rebuilding.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
APP_NAME="Manguo"
BUNDLE_ID="com.manguo.app"
DEST="${1:-$HOME/Applications/$APP_NAME.app}"

cd "$ROOT"
source "$HOME/.cargo/env" 2>/dev/null || true
cargo build --release -p manguo
TARGET_DIR="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys;print(json.load(sys.stdin)["target_directory"])')"
BIN="$TARGET_DIR/release/manguo"

rm -rf "$DEST"
mkdir -p "$DEST/Contents/MacOS" "$DEST/Contents/Resources"
cp "$BIN" "$DEST/Contents/MacOS/$APP_NAME"
cp "$HERE/resources/icon/manguo.icns" "$DEST/Contents/Resources/manguo.icns"

cat > "$DEST/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>$APP_NAME</string>
    <key>CFBundleDisplayName</key><string>$APP_NAME</string>
    <key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
    <key>CFBundleVersion</key><string>1</string>
    <key>CFBundleShortVersionString</key><string>0.7.0</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleExecutable</key><string>$APP_NAME</string>
    <key>CFBundleIconFile</key><string>manguo</string>
    <key>LSMinimumSystemVersion</key><string>12.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSMicrophoneUsageDescription</key><string>Manguo listens to your voice to play the synth and feed the grain sequencer.</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.music</string>
</dict>
</plist>
PLIST

# ad-hoc signature so the mic permission is remembered per bundle id
codesign --force --sign - "$DEST" >/dev/null 2>&1 || true
touch "$DEST"
echo "bundled $DEST"
