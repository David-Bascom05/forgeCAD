#!/bin/sh
# Puts the ffmpeg ForgeCAD records video with (tools/ffmpeg/PIN: a public prebuilt GPL build, pinned
# by URL and sha256) in ROOT/bin beside forge-app: ffmpeg, ffprobe, the build's licence
# (bin/ffmpeg-LICENSE.txt) and bin/ffmpeg-THIRD-PARTY.md (what it is, where from, its source).
# Keeps what is there when the pinned build is already installed. Exit 0: installed or already
# there; 1: not (offline, a hash mismatch, none pinned for this platform), nothing half-written.
#   tools/fetch-ffmpeg.sh ROOT
set -eu
[ "$#" -eq 1 ] || { echo "usage: tools/fetch-ffmpeg.sh INSTALL_ROOT" >&2; exit 2; }
root=$1
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$here/ffmpeg/PIN"

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) p=LINUX_X86_64 ;;
    Linux-aarch64 | Linux-arm64) p=LINUX_ARM64 ;;
    *) p="" ;;
esac
get() { eval "printf '%s' \"\${${p}_$1:-}\""; }
url=$( [ -n "$p" ] && get URL || true)
if [ -z "$url" ]; then
    echo "fetch-ffmpeg: no ffmpeg pinned for $(uname -s) $(uname -m); recording uses your own (FORGE_FFMPEG or PATH)" >&2
    exit 1
fi
sha=$(get SHA256)
version=$(get VERSION)
stamp="$root/bin/ffmpeg.pin"
if [ -x "$root/bin/ffmpeg" ] && [ "$(cat "$stamp" 2>/dev/null)" = "$sha" ]; then
    echo "fetch-ffmpeg: ffmpeg $version already installed"
    exit 0
fi

work=$(mktemp -d "${TMPDIR:-/tmp}/forgecad-ffmpeg.XXXXXXXX")
trap 'rm -rf -- "$work"' EXIT
curl -fsSL --retry 3 -o "$work/a" "$url" || { echo "fetch-ffmpeg: could not download $url" >&2; exit 1; }
if command -v sha256sum >/dev/null; then got=$(sha256sum "$work/a" | cut -d' ' -f1)
else got=$(shasum -a 256 "$work/a" | cut -d' ' -f1); fi
[ "$got" = "$sha" ] || { echo "fetch-ffmpeg: $url: sha256 $got, pinned $sha; not installed" >&2; exit 1; }
mkdir "$work/x"
case "$url" in
    *.zip) unzip -q "$work/a" -d "$work/x" ;;
    *) tar xJf "$work/a" -C "$work/x" ;;
esac
bin="$work/x/$(get BIN)"
"$bin/ffmpeg" -hide_banner -version >/dev/null || { echo "fetch-ffmpeg: the downloaded ffmpeg does not run here" >&2; exit 1; }

mkdir -p "$root/bin"
for f in ffmpeg ffprobe; do
    install -m 755 "$bin/$f" "$root/bin/.$f.new" && mv -f "$root/bin/.$f.new" "$root/bin/$f"
done
cp "$work/x/$(get LICENSE)" "$root/bin/ffmpeg-LICENSE.txt"
cat > "$root/bin/ffmpeg-THIRD-PARTY.md" <<NOTE
# ffmpeg, beside ForgeCAD

ffmpeg $version, a GPL build by $(get BUILDER), as published:
$url (sha256 $sha). Its licence is ffmpeg-LICENSE.txt, next to this file.

ForgeCAD runs it as a separate program (frames over a pipe) to write videos and links none of it.
The source of this version: https://ffmpeg.org/releases/ffmpeg-$version.tar.xz (and the builder's
page above for the libraries it was built with).
NOTE
printf '%s\n' "$sha" > "$stamp"
echo "fetch-ffmpeg: installed ffmpeg $version in $root/bin"
