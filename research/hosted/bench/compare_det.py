#!/usr/bin/env python3
"""Compare det outputs: per file, body hash and mesh hash against the first file; and cold-build
time ratios. usage: compare_det.py REF OTHER.."""
import sys
def load(p):
    d = {}
    for l in open(p):
        x = l.split()
        if len(x) < 5: continue
        kv = dict(t.split("=", 1) for t in x[4:] if "=" in t)
        d[x[0]] = (x[1], x[2], x[3], float(kv.get("cold_ms", "nan")))
    return d
ref = load(sys.argv[1])
for o in sys.argv[2:]:
    d = load(o)
    bd = [f for f in ref if f in d and ref[f][1] != d[f][1]]
    md = [f for f in ref if f in d and ref[f][2] != d[f][2]]
    ratios = sorted(d[f][3] / ref[f][3] for f in ref if f in d and ref[f][3] > 20)
    med = ratios[len(ratios) // 2] if ratios else float("nan")
    print(f"{o}: files {len(d)}/{len(ref)}  body differ {len(bd)}  mesh differ {len(md)}  cold-time ratio vs ref (files >20ms, n={len(ratios)}): median {med:.2f}  p10 {ratios[len(ratios)//10]:.2f}  p90 {ratios[len(ratios)*9//10]:.2f}")
    for f in (bd + md)[:10]: print("   DIFF", f, ref[f][:3], d[f][:3])
