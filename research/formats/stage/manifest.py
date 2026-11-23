#!/usr/bin/env python3
"""Writes a corpus manifest: the sources (URL, commit, licence) in the first line, then one line
per file used: its source, its path there, bytes and sha256. The files stay outside the repo.

  manifest.py OUT.jsonl DATA_DIR LIST [SOURCES.json]
LIST holds absolute paths under DATA_DIR/<source>/, one per line; SOURCES.json maps each source
directory to {"url", "commit", "licence"}.
"""
import hashlib, json, os, sys

out, data, lst = sys.argv[1:4]
known = json.load(open(sys.argv[4])) if len(sys.argv) > 4 else {}
rows, used = [], set()
for p in sorted(l.strip() for l in open(lst) if l.strip()):
    rel = os.path.relpath(p, data)
    src, path = rel.split(os.sep, 1)
    used.add(src)
    d = open(p, "rb").read()
    rows.append({"source": src, "path": path, "bytes": len(d), "sha256": hashlib.sha256(d).hexdigest()})
with open(out, "w") as o:
    o.write(json.dumps({"sources": {s: known.get(s, {}) for s in sorted(used)}}) + "\n")
    for r in rows:
        o.write(json.dumps(r) + "\n")
