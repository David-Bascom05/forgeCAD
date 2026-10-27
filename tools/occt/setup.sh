#!/bin/sh
# The OCCT oracle's Python: a venv outside the repo with the OCP bindings (OpenCascade 8).
# Test only; nothing in the workspace depends on it. Run the oracle with $VENV/bin/python.
set -e
VENV=${VENV:-$HOME/.local/share/forgecad-occt/venv}
[ -x "$VENV/bin/python" ] || python3 -m venv "$VENV"
"$VENV/bin/pip" install -q cadquery-ocp
"$VENV/bin/python" -c "import OCP; print('OCP', OCP.__version__, 'in', '$VENV')"
