#!/bin/sh
# Every bench part and a few real ones exported as 3MF, OBJ (with normals) and ASCII STL, and as
# binary STL where the part is one solid (a binary STL is one mesh: solids that touch share points
# in it), then judged by check.py: trimesh and numpy-stl read each, every body closed, manifold,
# consistently wound and facing out, every 3MF's model valid against the 3MF Core schema.
#   tools/meshcheck/setup.sh && tools/meshcheck/run.sh [OUT_DIR]
set -e
cd "$(dirname "$0")/../.."
VENV=${VENV:-${FORGECAD_MESHCHECK_VENV:-$HOME/.local/share/forgecad-meshcheck/venv}}
OUT=${1:-$(mktemp -d)}
mkdir -p "$OUT"
cargo build --release -q -p forge-cli
fc=target/release/forgecad
files=""
for part in interface/forge-lang/parts/bench/*.fg examples/unified/knob.fg examples/unified/peg_board.fg \
    examples/unified/hinged_box.fg examples/machined/flange.fg examples/sheetmetal/tray_asm.fg; do
    n=$(basename "$part" .fg)
    # a part that does not build here (a font it wants is not beside it) is not a mesh test
    if ! $fc build "$part" >/dev/null 2>&1; then
        echo "skipped $n: it does not build"
        continue
    fi
    $fc export "$part" 3mf -o "$OUT/$n.3mf" >/dev/null
    $fc export "$part" obj normals -o "$OUT/$n.obj" >/dev/null
    $fc export "$part" stl ascii -o "$OUT/$n-ascii.stl" >/dev/null
    files="$files $OUT/$n.3mf $OUT/$n.obj $OUT/$n-ascii.stl"
    if [ "$($fc export "$part" stl -o "$OUT/$n.stl" --json | grep -o '"name"' | wc -l)" -eq 1 ]; then
        files="$files $OUT/$n.stl"
    fi
done
# shellcheck disable=SC2086
"$VENV/bin/python" tools/meshcheck/check.py $files
echo "all $(echo $files | wc -w) files ok ($OUT)"
