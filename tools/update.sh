#!/bin/sh
# Build latest main in isolation; never reset or stash the user's checkout.
set -eu
if [ "$#" -gt 1 ]; then
    echo "usage: forgecad-update [INSTALL_ROOT]" >&2
    exit 2
fi
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
case "$script_dir" in
    */bin) default_root=$(dirname -- "$script_dir") ;;
    *) default_root=${CARGO_HOME:-$HOME/.cargo} ;;
esac
install_root=${1:-$default_root}
case "$install_root" in
    /*) ;;
    *) install_root="$PWD/$install_root" ;;
esac
repository=${FORGECAD_REPOSITORY:-ssh://forgejo@git.commonty.org/david/ForgeCAD.git}
checkout=$(mktemp -d "${TMPDIR:-/tmp}/forgecad-update.XXXXXXXX")
trap 'rm -rf -- "$checkout"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
git clone --depth 1 --single-branch --branch main -- "$repository" "$checkout/source"
cd "$checkout/source"
git log -1 --format='Installing ForgeCAD main %h (%cs)'
# Explicit commands also support main revisions predating tools/install.sh.
cargo install --locked --path interface/forge-render --bin forge-app --root "$install_root" --force
cargo install --locked --path interface/forge-cli --bin forgecad --root "$install_root" --force
if [ -f tools/update.sh ]; then
    install -m 755 tools/update.sh "$install_root/bin/forgecad-update"
fi
printf '\nUpdated ForgeCAD in %s/bin.\n' "$install_root"
