#!/bin/sh
# Builds the LGPL ffmpeg ForgeCAD records with, from the sources tools/ffmpeg/PIN pins: openh264
# (BSD-2-Clause) static, then ffmpeg static with only what recording needs. Linux, macOS, and
# Windows under MSYS2 MINGW64. Output: OUT/ffmpeg-VER-BUILD-PLATFORM.tar.xz (or .zip on Windows)
# holding bin/ffmpeg, bin/ffprobe, the licences and THIRD-PARTY.md with this configure line.
#   sh tools/ffmpeg/build.sh PLATFORM OUT     (PLATFORM: linux-x86_64 | macos-arm64 | windows-x86_64)
set -eu
platform=$1
out=$(mkdir -p "$2" && cd "$2" && pwd)
here=$(cd "$(dirname "$0")" && pwd)
. "$here/PIN"
[ -n "$OPENH264_SHA256" ] || { echo "build.sh: OPENH264_SHA256 is not pinned in tools/ffmpeg/PIN" >&2; exit 1; }
work=$(mktemp -d)
prefix="$work/prefix"
sum() { if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi; }
fetch() {
    curl -fsSL -o "$2" "$1"
    got=$(sum "$2")
    [ "$got" = "$3" ] || { echo "build.sh: $1: sha256 $got, pinned $3" >&2; exit 1; }
}
cd "$work"
fetch "$FFMPEG_URL" ffmpeg.tar.xz "$FFMPEG_SHA256"
fetch "$OPENH264_URL" openh264.tar.gz "$OPENH264_SHA256"
tar xf ffmpeg.tar.xz
tar xf openh264.tar.gz
jobs=$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)

case "$platform" in
    windows-*) os_flags="OS=mingw_nt ARCH=x86_64"; cxx="-lstdc++" ;;
    macos-*) os_flags=""; cxx="-lc++" ;;
    *) os_flags=""; cxx="-lstdc++" ;;
esac
# shellcheck disable=SC2086
make -C "openh264-$OPENH264_VERSION" -j"$jobs" $os_flags PREFIX="$prefix" install-static

configure="--prefix=$prefix/ffmpeg --disable-gpl --disable-nonfree --enable-static --disable-shared \
--disable-autodetect --disable-everything --disable-doc --disable-network --disable-ffplay \
--enable-ffmpeg --enable-ffprobe --enable-zlib --enable-libopenh264 \
--enable-protocol=file,pipe --enable-demuxer=rawvideo,image2,mov --enable-muxer=mp4,image2 \
--enable-decoder=rawvideo,png,h264,mpeg4 --enable-encoder=libopenh264,mpeg4,png \
--enable-parser=h264,png --enable-bsf=h264_mp4toannexb,extract_extradata \
--enable-filter=scale,format,pad,null,copy --pkg-config-flags=--static"
cd "ffmpeg-$FFMPEG_VERSION"
# shellcheck disable=SC2086
PKG_CONFIG_PATH="$prefix/lib/pkgconfig" ./configure $configure --extra-libs="$cxx -lm"
make -j"$jobs"
make install
cd "$work"

stage="ffmpeg-$FFMPEG_VERSION-$BUILD-$platform"
mkdir -p "$stage/bin"
ext=""; case "$platform" in windows-*) ext=".exe" ;; esac
cp "$prefix/ffmpeg/bin/ffmpeg$ext" "$prefix/ffmpeg/bin/ffprobe$ext" "$stage/bin/"
cp "ffmpeg-$FFMPEG_VERSION/COPYING.LGPLv2.1" "$stage/LICENSE.ffmpeg.LGPL-2.1.txt"
cp "openh264-$OPENH264_VERSION/LICENSE" "$stage/LICENSE.openh264.BSD-2-Clause.txt"
cat > "$stage/THIRD-PARTY.md" <<NOTE
# ffmpeg, as ForgeCAD ships it

ffmpeg $FFMPEG_VERSION (LGPL 2.1 or later; this build has no GPL or non-free parts), with
OpenH264 $OPENH264_VERSION (BSD-2-Clause) linked in statically. ForgeCAD runs it as a separate program
to write videos; nothing of it is linked into ForgeCAD.

Source (the offer the LGPL asks for: these are the exact sources of this binary):
- $FFMPEG_URL  sha256 $FFMPEG_SHA256
- $OPENH264_URL  sha256 $OPENH264_SHA256

Built by ForgeCAD's .github/workflows/ffmpeg.yml (tools/ffmpeg/build.sh), platform $platform, with:

    ./configure $configure --extra-libs="$cxx -lm"

To rebuild it, or a changed one, run tools/ffmpeg/build.sh $platform OUT from ForgeCAD's source.
NOTE
case "$platform" in
    windows-*) (cd "$work" && 7z a -tzip "$out/$stage.zip" "$stage" >/dev/null) ;;
    *) tar cJf "$out/$stage.tar.xz" "$stage" ;;
esac
ls -l "$out"
