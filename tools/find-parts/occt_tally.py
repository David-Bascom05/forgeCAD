#!/usr/bin/env python3
"""Tallies an oracle run (tools/occt/oracle.py --out): who built what, who disagrees."""
import json
import sys

rows = json.load(open(sys.argv[1]))
both = ours_only = occt_only = neither = 0
agree, disagree, refused = 0, [], {}
for r in rows:
    o, f = r["occt"], r["forge"]["seq"]
    a, b = o["status"] == "ok", f["status"] == "ok"
    both += a and b
    ours_only += b and not a
    occt_only += a and not b
    neither += not a and not b
    v = r["verdict"]["seq"]
    if a and b:
        if v.endswith("agree"):
            agree += 1
        else:
            disagree.append((v, r["name"], o.get("volume"), f.get("volume")))
    if a and not b:
        k = (f.get("error") or "?")[:70]
        refused.setdefault(k, []).append(r["name"])
print(f"{len(rows)} parts: both build {both} (agree {agree}, disagree {len(disagree)}), only ours {ours_only}, only OCCT {occt_only}, neither {neither}")
print(f"OCCT build rate {100 * (both + occt_only) / max(len(rows), 1):.1f}%, ours {100 * (both + ours_only) / max(len(rows), 1):.1f}%")
for k, v in sorted(refused.items(), key=lambda x: -len(x[1]))[:25]:
    print(f"  only OCCT builds {len(v):>4}  {k}  e.g. {v[0]}")
for v, n, a, b in disagree[:40]:
    print(f"  disagree {v:<12} {n}  occt {a} ours {b}")
