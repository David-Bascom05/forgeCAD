#!/bin/sh
# The mesh files' independent check: a venv outside the repo with trimesh, numpy-stl and lxml (the
# 3MF schema). Test only; nothing in the workspace depends on it. forge-cli's mesh tests run
# check.py with $VENV/bin/python when it is there (FORGECAD_MESHCHECK_VENV, or this default).
set -e
VENV=${VENV:-$HOME/.local/share/forgecad-meshcheck/venv}
[ -x "$VENV/bin/python" ] || "${PYTHON:-python3}" -m venv "$VENV"
# pinned: everyone judges the files with the same readers
"$VENV/bin/pip" install -q numpy==2.5.5 trimesh==5.2.2 numpy-stl==4.1.4 lxml==6.1.3 networkx==3.7
"$VENV/bin/python" -c "import trimesh, stl, lxml; print('trimesh', trimesh.__version__, 'in', '$VENV')"
