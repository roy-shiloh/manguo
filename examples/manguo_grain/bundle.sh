#!/bin/sh
# Build the release cdylib and wrap it as "Manguo Grain.vst3" and
# "Manguo Grain.clap" bundles (macOS layout). Pass --install to copy them
# into ~/Library/Audio/Plug-Ins/{VST3,CLAP}.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
NAME="Manguo Grain"
PKG=manguo_grain
OUT="$ROOT/target/release/bundled"

cargo build --release -p "$PKG" --manifest-path "$ROOT/Cargo.toml"
LIB="$ROOT/target/release/lib${PKG}.dylib"

write_bundle() {
    kind=$1 # vst3 | clap
    home="$OUT/$NAME.$kind"
    rm -rf "$home"
    mkdir -p "$home/Contents/MacOS"
    cp "$LIB" "$home/Contents/MacOS/$NAME"
    printf 'BNDL????' > "$home/Contents/PkgInfo"
    cat > "$home/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>CFBundleExecutable</key>
    <string>$NAME</string>
    <key>CFBundleIdentifier</key>
    <string>nl.makepad.manguo-grain.$kind</string>
    <key>CFBundleName</key>
    <string>$NAME</string>
    <key>CFBundleDisplayName</key>
    <string>$NAME</string>
    <key>CFBundlePackageType</key>
    <string>BNDL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
  </dict>
</plist>
EOF
    codesign -f -s - "$home" >/dev/null 2>&1 || echo "warning: could not self-sign $home"
    echo "bundled $home"
}

write_bundle vst3
write_bundle clap

if [ "${1:-}" = "--install" ]; then
    for kind in VST3 CLAP; do
        ext=$(printf '%s' "$kind" | tr '[:upper:]' '[:lower:]')
        dest="$HOME/Library/Audio/Plug-Ins/$kind"
        mkdir -p "$dest"
        rm -rf "$dest/$NAME.$ext"
        cp -R "$OUT/$NAME.$ext" "$dest/"
        echo "installed $dest/$NAME.$ext"
    done
fi
