#!/bin/sh
# Fetches the Render Studio's HDRIs (interface/forge-render/data/hdri/MANIFEST.csv, Poly Haven, CC0)
# into the user's cache, each checked against its pinned sha256; a mismatch is refused and removed.
#   sh tools/fetch-hdri.sh [DIR]   (default: forge-dirs' cache, as the app looks there:
#   $XDG_CACHE_HOME, else ~/Library/Caches on macOS, else ~/.cache; then forgecad/hdri)
set -e
here=$(cd "$(dirname "$0")/.." && pwd)
manifest="$here/interface/forge-render/data/hdri/MANIFEST.csv"
if [ -n "$1" ]; then
  dir=$1
elif [ -n "$XDG_CACHE_HOME" ]; then
  dir="$XDG_CACHE_HOME/forgecad/hdri"
elif [ "$(uname)" = Darwin ]; then
  dir="$HOME/Library/Caches/forgecad/hdri"
else
  dir="${XDG_CACHE_HOME:-$HOME/.cache}/forgecad/hdri"
fi
mkdir -p "$dir"
sum() { if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi; }
grep -v '^#' "$manifest" | tail -n +2 | while IFS=, read -r key name authors licence page file sha bundled; do
  out="$dir/$key.hdr"
  if [ -f "$out" ] && [ "$(sum "$out")" = "$sha" ]; then
    echo "have $key"
    continue
  fi
  echo "fetching $key ($name, $authors, $licence) from $file"
  curl -fsSL -o "$out.part" "$file"
  got=$(sum "$out.part")
  if [ "$got" != "$sha" ]; then
    rm -f "$out.part"
    echo "fetch-hdri: $key: sha256 $got is not the pinned $sha: refused" >&2
    exit 1
  fi
  mv "$out.part" "$out"
done
echo "HDRIs in $dir"
