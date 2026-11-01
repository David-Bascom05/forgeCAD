#!/usr/bin/env python3
"""Prints a survey.py JSON: report.py A.json B.json .."""
import json, sys
TOPS = ["schema", "system", "faces", "edges", "bs", "bc", "pc", "sor", "extr", "offset", "roots", "unc", "solids_per_file", "ents_per_file"]
for p in sys.argv[1:]:
    d = json.load(open(p))
    print(f"== {p}: {d['files']} files of {d['population']}, errors {d['errors']}, assemblies {d['with_assembly']}")
    for k in TOPS:
        t = sum(d[k].values()) or 1
        print(" ", k, ", ".join(f"{a} {100*b/t:.3g}%" for a, b in sorted(d[k].items(), key=lambda x: -x[1])[:12]), f"[total {t}]")
    u = sorted(d["unit"].items(), key=lambda x: -x[1])
    import re, collections
    c = collections.Counter()
    for a, b in u:
        if "LENGTH_UNIT" in a:
            c[re.sub(r"#\d+", "#", a)] += b
    print("  length units", c.most_common(6))
    other = [(a, b, d["files_with"].get(a, 0)) for a, b in d["n"].items()]
    other.sort(key=lambda x: -x[2])
    print("  entities by files:", ", ".join(f"{a} {c}" for a, b, c in other))
