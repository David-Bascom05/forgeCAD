#!/bin/sh
# Install the native app and CLI together, without requiring root.
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
install_root=${1:-"${CARGO_HOME:-$HOME/.cargo}"}
case "$install_root" in
    /*) ;;
    *) install_root="$PWD/$install_root" ;;
esac
if [ "$#" -gt 1 ]; then
    echo "usage: tools/install.sh [INSTALL_ROOT]" >&2
    exit 2
fi

cd "$repo"
cargo install --locked --path "$repo/interface/forge-render" --bin forge-app --root "$install_root" --force
cargo install --locked --path "$repo/interface/forge-cli" --bin forgecad --root "$install_root" --force
if [ -f interface/forge-speech/Cargo.toml ]; then
    cargo install --locked --path interface/forge-speech --bin forge-speech --root "$install_root" --force
fi
install -m 755 "$repo/tools/update.sh" "$install_root/bin/forgecad-update"
# ffmpeg for recording video (tools/fetch-ffmpeg.sh); a failed fetch leaves the install as it is
if [ -f "$repo/tools/fetch-ffmpeg.sh" ] && ! sh "$repo/tools/fetch-ffmpeg.sh" "$install_root"; then
    printf '\nNo ffmpeg installed: recording saves PNG frames until forgecad-update succeeds.\n' >&2
fi
# macOS: an app in ~/Applications, so Launchpad, Spotlight and the Dock open it
if [ "$(uname -s)" = Darwin ]; then
    app="$HOME/Applications/ForgeCAD.app/Contents"
    mkdir -p "$app/MacOS"
    cat > "$app/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>ForgeCAD</string>
  <key>CFBundleIdentifier</key><string>org.commonty.forgecad</string>
  <key>CFBundleExecutable</key><string>ForgeCAD</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSMicrophoneUsageDescription</key><string>ForgeCAD listens when you hold the microphone button to talk to the agent.</string>
</dict></plist>
PLIST
    printf '#!/bin/sh\nexec "%s/bin/forge-app" "$@"\n' "$install_root" > "$app/MacOS/ForgeCAD"
    chmod 755 "$app/MacOS/ForgeCAD"
fi
printf '\nInstalled ForgeCAD in %s/bin.\nRun forgecad to open the app, or forgecad open FILE.\nEnsure %s/bin is on PATH.\n' "$install_root" "$install_root"

if [ -f tools/setup-speech.sh ]; then
    install -m 755 tools/setup-speech.sh "$install_root/bin/forgecad-setup-speech"
fi
