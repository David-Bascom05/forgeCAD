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
printf '\nInstalled ForgeCAD in %s/bin.\nRun forgecad to open the app, or forgecad open FILE.\nEnsure %s/bin is on PATH.\n' "$install_root" "$install_root"

if [ -f tools/setup-speech.sh ]; then
    install -m 755 tools/setup-speech.sh "$install_root/bin/forgecad-setup-speech"
fi
