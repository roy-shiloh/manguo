#!/bin/sh
# Build the release library and wrap it as plugin bundles, with the fonts and
# icons the editor needs shipped inside (see src/resources.rs).
#
#   bundle.sh [--install]            macOS: "Manguo Grain.vst3" + ".clap" bundles,
#                                    --install copies them to ~/Library/Audio/Plug-Ins
#   bundle.sh --windows <libdir>     wrap an already built manguo_grain.dll from <libdir>
#                                    in the Windows VST3/CLAP layouts (used by CI)
#
# Output goes to target/release/bundled/.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
NAME="Manguo Grain"
PKG=manguo_grain
OUT="$ROOT/target/release/bundled"

# Resources referenced by the editor's live design (crate -> files).
copy_resources() {
    res=$1 # .../Resources/makepad
    mkdir -p "$res/manguo_grain/resources/px" "$res/manguo_grain/resources/icons" "$res/makepad_widgets/resources"
    cp "$HERE/resources/PressStart2P-Regular.ttf" "$res/manguo_grain/resources/"
    cp "$HERE/resources/px/cell.png" "$HERE/resources/px/seed.png" "$res/manguo_grain/resources/px/"
    cp "$HERE/resources/icons/Icon_Down.svg" "$HERE/resources/icons/Icon_Up.svg" "$HERE/resources/icons/Icon_Seq_Sweep.svg" "$res/manguo_grain/resources/icons/"
    for f in IBMPlexSans-Text.ttf IBMPlexSans-SemiBold.ttf LXGWWenKaiRegular.ttf LXGWWenKaiBold.ttf NotoColorEmoji.ttf; do
        cp "$ROOT/widgets/resources/$f" "$res/makepad_widgets/resources/"
    done
}

write_plist() {
    kind=$1
    cat > "$2" <<EOF
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
}

mac_bundle() {
    kind=$1 # vst3 | clap
    home="$OUT/$NAME.$kind"
    rm -rf "$home"
    mkdir -p "$home/Contents/MacOS"
    cp "$ROOT/target/release/lib${PKG}.dylib" "$home/Contents/MacOS/$NAME"
    printf 'BNDL????' > "$home/Contents/PkgInfo"
    write_plist "$kind" "$home/Contents/Info.plist"
    copy_resources "$home/Contents/Resources/makepad"
    codesign -f -s - "$home" >/dev/null 2>&1 || echo "warning: could not self-sign $home"
    echo "bundled $home"
}

windows_bundle() {
    libdir=$1
    dll="$libdir/${PKG}.dll"
    [ -f "$dll" ] || { echo "no $dll"; exit 1; }
    # VST3: bundle folder, binary under Contents/x86_64-win
    home="$OUT/$NAME.vst3"
    rm -rf "$home"
    mkdir -p "$home/Contents/x86_64-win"
    cp "$dll" "$home/Contents/x86_64-win/$NAME.vst3"
    copy_resources "$home/Contents/Resources/makepad"
    echo "bundled $home"
    # CLAP: a folder with the .clap file and its Resources next to it
    home="$OUT/$NAME"
    rm -rf "$home"
    mkdir -p "$home"
    cp "$dll" "$home/$NAME.clap"
    copy_resources "$home/Resources/makepad"
    echo "bundled $home/$NAME.clap"
}

mkdir -p "$OUT"
case "${1:-}" in
    --windows)
        windows_bundle "${2:-$ROOT/target/release}"
        ;;
    *)
        cargo build --release -p "$PKG" --manifest-path "$ROOT/Cargo.toml"
        mac_bundle vst3
        mac_bundle clap
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
        ;;
esac
