#!/usr/bin/env python3
"""tools/preverify/publish.py FILE [NAME]: put FILE on branch preverify/maps as NAME (default its
base name), e.g. a reads map made on a new main: reads-<main sha>.json."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pvitems import publish  # noqa: E402

f = sys.argv[1]
name = sys.argv[2] if len(sys.argv) > 2 else os.path.basename(f)
sys.exit(0 if publish(".", name, open(f).read()) else 1)
