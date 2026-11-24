#!/bin/sh
# Puts the LGPL ffmpeg ForgeCAD records video with (tools/ffmpeg/PIN, built by
# .github/workflows/ffmpeg.yml) in ROOT/bin beside forge-app, with its licences and THIRD-PARTY note
# (bin/ffmpeg-THIRD-PARTY.md, bin/ffmpeg-LICENSE.*). Checks the pinned sha256; keeps what is there
# when the pinned build is already installed. Exit 0: installed or already there; 1: failed (offline,
# a hash mismatch, no build published for this platform); nothing is left half-written either way.
#   tools/fetch-ffmpeg.sh ROOT
set -eu
[ "$#" -eq 1 ] || { echo "usage: tools/fetch-ffmpeg.sh INSTALL_ROOT" >&2; exit 2; }
root=$1
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$here/ffmpeg/PIN"
releases=${FORGECAD_FFMPEG_RELEASES:-https://github.com/David-Bascom05/forgeCAD/releases/download}

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) platform=linux-x86_64; sha=$ASSET_LINUX_X86_64_SHA256 ;;
    Darwin-arm64) platform=macos-arm64; sha=$ASSET_MACOS_ARM64_SHA256 ;;
    *) echo "fetch-ffmpeg: no ForgeCAD ffmpeg build for $(uname -s) $(uname -m)" >&2; exit 1 ;;
esac
pin="$FFMPEG_VERSION-$BUILD"
stamp="$root/bin/ffmpeg.pin"
if [ -x "$root/bin/ffmpeg" ] && [ "$(cat "$stamp" 2>/dev/null)" = "$pin-$sha" ]; then
    echo "fetch-ffmpeg: ffmpeg $pin already installed"
    exit 0
fi
[ -n "$sha" ] || { echo "fetch-ffmpeg: no ffmpeg $pin published for $platform yet" >&2; exit 1; }

name="ffmpeg-$pin-$platform"
url="$releases/ffmpeg-$pin/$name.tar.xz"
work=$(mktemp -d "${TMPDIR:-/tmp}/forgecad-ffmpeg.XXXXXXXX")
trap 'rm -rf -- "$work"' EXIT
curl -fsSL --retry 3 -o "$work/a.tar.xz" "$url" || { echo "fetch-ffmpeg: could not download $url" >&2; exit 1; }
if command -v sha256sum >/dev/null; then got=$(sha256sum "$work/a.tar.xz" | cut -d' ' -f1)
else got=$(shasum -a 256 "$work/a.tar.xz" | cut -d' ' -f1); fi
[ "$got" = "$sha" ] || { echo "fetch-ffmpeg: $url: sha256 $got, pinned $sha; not installed" >&2; exit 1; }
tar xJf "$work/a.tar.xz" -C "$work"
s="$work/$name"
"$s/bin/ffmpeg" -hide_banner -version >/dev/null || { echo "fetch-ffmpeg: the downloaded ffmpeg does not run here" >&2; exit 1; }

mkdir -p "$root/bin"
for f in ffmpeg ffprobe; do
    install -m 755 "$s/bin/$f" "$root/bin/.$f.new" && mv -f "$root/bin/.$f.new" "$root/bin/$f"
done
cp "$s/THIRD-PARTY.md" "$root/bin/ffmpeg-THIRD-PARTY.md"
for l in "$s"/LICENSE.*; do cp "$l" "$root/bin/ffmpeg-$(basename "$l")"; done
printf '%s\n' "$pin-$sha" > "$stamp"
echo "fetch-ffmpeg: installed ffmpeg $pin (LGPL, with OpenH264) in $root/bin"
