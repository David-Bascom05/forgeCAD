#!/usr/bin/env python3
"""Writes results/: each run's verdict and cause per file, paths relative to their source.

  trim.py SCRATCH_DIR RESULTS_DIR
"""
import json, sys, os
S, R = sys.argv[1], sys.argv[2]
for name, src, extra in [("3mf", "3mf", []), ("stl-thingi10k", "stl_thingi", []), ("stl-tests", "stl_tests", []), ("obj", "obj", []),
                         ("stl-write", "rw", []), ("brep", "brep", []), ("abc", "abc_all", []), ("step-write-parts", "stepw_fg", ["solids"]),
                         ("step-write-real", "stepw_real", ["written"]), ("dwg", "dwg", []), ("dwg-write", "dwgw", ["ours_again"]),
                         ("dxf-write", "dxf", ["sheet"]), ("fonts", "fonts", ["cps"]), ("shx", "shx", ["shapes"]), ("fcstd", "fcstd", ["body", "status"])]:
    p = os.path.join(S, "runs", src + ".jsonl")
    if not os.path.exists(p):
        continue
    adj = {}
    for a in (["stl_adj"] if src.startswith("stl") else []):
        for l in open(os.path.join(S, "runs", a + ".jsonl")):
            d = json.loads(l); adj[d["file"]] = d["verdict2"]
    with open(os.path.join(R, name + ".jsonl"), "w") as o:
        for l in open(p):
            r = json.loads(l)
            f = r["file"]
            for pre in (S + "/data/", "data/"):
                if f.startswith(pre):
                    f = f[len(pre):]
            row = {"file": f, "verdict": r["verdict"], "why": [str(w)[:200] for w in r.get("why", [])][:3]}
            for k in extra:
                if k in r:
                    row[k] = r[k]
            if r["file"] in adj:
                row["adjudicated"] = adj[r["file"]]
            o.write(json.dumps(row) + "\n")
