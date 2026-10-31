#!/bin/sh
# The OCCT oracle's Python: a venv outside the repo with the OCP bindings (OpenCascade 8).
# Test only; nothing in the workspace depends on it. Run the oracle with $VENV/bin/python.
set -e
VENV=${VENV:-$HOME/.local/share/forgecad-occt/venv}
# pinned: CI and local must judge parts with the same OpenCascade (8.x needs Python 3.11+)
[ -x "$VENV/bin/python" ] || "${PYTHON:-python3}" -m venv "$VENV"
"$VENV/bin/pip" install -q cadquery-ocp==8.0.1.1.0
"$VENV/bin/python" -c "import OCP; print('OCP', OCP.__version__, 'in', '$VENV')"
