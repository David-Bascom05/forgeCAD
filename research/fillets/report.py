#!/usr/bin/env python3
"""A translated run's fillet parts held against the STEP: report.py RESULTS.jsonl TRUTH.jsonl SPLIT.tsv
The rate leaves out what the dataset split calls an importer's error, and parts stopped elsewhere."""
import collections, json, sys
res, tr, sp = sys.argv[1:4]
truth = {json.loads(l)["id"]: json.loads(l) for l in open(tr)}
split = {l.split("\t")[0]: l.rstrip("\n").split("\t") for l in open(sp)}
c = collections.Counter(); sigs = collections.Counter(); ex = {}
for l in open(res):
    r = json.loads(l)
    if "fillet" not in (r.get("features") or []) or r.get("status") not in ("translated", "stopped"):
        continue
    lab = split[r["id"]][2] if r["id"] in split else ""
    v = r.get("v")
    t = truth.get(r["id"])
    if v == "ok":
        if not t or not t.get("vol"):
            k = "built-no-truth"
        elif r.get("status") == "stopped":
            k = "built-stopped"
        else:
            e = abs(r["vol"] - t["vol"]) / t["vol"]; a = abs(r["area"] - t["area"]) / t["area"]
            k = "match" if e < 1e-4 and a < 1e-4 else "off"
    else:
        k = "refused"
        s = r.get("sig", "")[:36].split("{")[0]
        sigs[s] += 1
        ex.setdefault(s, []).append(r["id"])
    c[k] += 1
print(dict(c))
for s, n in sigs.most_common(25):
    print(n, s, ex[s][:6])
