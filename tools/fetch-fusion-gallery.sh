#!/usr/bin/env bash
# Fetch chosen subsets of the Fusion 360 Gallery Dataset (Autodesk) into a git-ignored cache.
#
# LICENCE: the Dataset may be used "only for non-commercial research purposes"; it may not be
# redistributed in its entirety; Autodesk may terminate the licence at any time; if you are
# employed by a for-profit entity your employer is bound too (LICENSE.md items 1, 2, 9, 11, read
# 2026-11-24 at commit 1084b881 of github.com/AutodeskAILab/Fusion360GalleryDataset). David's
# decision (2026-11-24): use it where it has data we cannot get elsewhere, keep every file of it
# OUT of this repository. Commit only our own code and aggregate results (counts, pass rates);
# never models, meshes, labels or per-part derived geometry. tools/ci/no-fusion-gallery.py fails
# CI if any file of the set is committed.
#
# Source: Autodesk's public bucket, linked from the dataset's README; no account or sign-up.
#
# Usage: tools/fetch-fusion-gallery.sh [segmentation-step] [reconstruction] [--dest DIR]
#   segmentation-step  s2.0.1_extended_step.zip, 506 MB: 42,912 STEP parts, a per-face label of
#                      the modelling operation that made each face (ExtrudeSide/End, CutSide/End,
#                      Fillet, Chamfer, RevolveSide/End), and per-face timeline info.
#   reconstruction     r1.0.1.zip, 2.1 GB: 8,625 sketch+extrude construction sequences (JSON)
#                      with STEP/SMT/OBJ at each step (also read by tools/find-parts/fusion.py).
#   (no argument: both)
# Default destination: $FORGECAD_FUSION_GALLERY, else <repo>/.cache/fusion-gallery (git-ignored).
# Each archive is verified against the SHA-256 pinned below and refused on a mismatch; it is then
# unpacked, and the licence text is copied next to it.
set -euo pipefail

BASE=https://fusion-360-gallery-dataset.s3.us-west-2.amazonaws.com
LICENSE_URL=https://raw.githubusercontent.com/AutodeskAILab/Fusion360GalleryDataset/1084b881f3bb710267801d812d6e9286b8667059/LICENSE.md
# name  path  bytes  sha256 (computed 2026-11-24 from the bucket objects, Last-Modified 2021-12-02/03)
SETS=(
  "segmentation-step segmentation/s2.0.1/s2.0.1_extended_step.zip 506333119 9bebcf0722951bef166c2160cda6e2d3225692b90a90d7dc9cf34dd80fc5a39e"
  "reconstruction reconstruction/r1.0.1/r1.0.1.zip 2103449157 485601de3d23e25a5d63a75588ca780fa7881d7a0bea1ab6c24dbcefa57ad5c9"
)

repo=$(cd "$(dirname "$0")/.." && pwd)
dest=${FORGECAD_FUSION_GALLERY:-$repo/.cache/fusion-gallery}
want=()
while [ $# -gt 0 ]; do
  case "$1" in
    --dest) dest=$2; shift 2 ;;
    segmentation-step|reconstruction) want+=("$1"); shift ;;
    -h|--help) sed -n '2,27p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
[ ${#want[@]} -eq 0 ] && want=(segmentation-step reconstruction)

# Refuse a destination inside the repository unless git ignores it.
case "$dest" in
  "$repo"/*)
    mkdir -p "$dest"
    if ! git -C "$repo" check-ignore -q "$dest/x"; then
      echo "refused: $dest is inside the repository and not git-ignored" >&2; exit 1
    fi ;;
esac
mkdir -p "$dest"

curl -fsSL "$LICENSE_URL" -o "$dest/LICENSE.md"
cat > "$dest/NOTICE.txt" <<NOTE
Fusion 360 Gallery Dataset, Autodesk, Inc. https://github.com/AutodeskAILab/Fusion360GalleryDataset
Used under its licence (LICENSE.md here): non-commercial research purposes only; not to be
redistributed; this cache must never be committed to the forgeCAD repository.
NOTE

for entry in "${SETS[@]}"; do
  read -r name path bytes sha <<<"$entry"
  printf '%s\n' "${want[@]}" | grep -qx "$name" || continue
  zip="$dest/$(basename "$path")"
  if [ ! -f "$zip" ] || [ "$(stat -c %s "$zip")" != "$bytes" ]; then
    echo "fetching $name ($bytes bytes)"
    curl -fSL --retry 4 -C - "$BASE/$path" -o "$zip.part" && mv "$zip.part" "$zip"
  fi
  got=$(sha256sum "$zip" | cut -d' ' -f1)
  if [ "$got" != "$sha" ]; then
    echo "refused: $zip sha256 $got, pinned $sha" >&2; exit 1
  fi
  if [ ! -f "$dest/$name.unpacked" ]; then
    (cd "$dest" && python3 -I -c 'import sys,zipfile; zipfile.ZipFile(sys.argv[1]).extractall(".")' "$zip")
    touch "$dest/$name.unpacked"
  fi
  echo "$name: ok ($dest)"
done
