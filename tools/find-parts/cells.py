#!/usr/bin/env python3
"""Coverage matrices from enum and matrix ledgers: which cells built, which only refused (untested in
practice: the kernel never reached the later feature), which fail.

  cells.py enum RUN        feature-after-feature, by the second feature's kind against the first's
  cells.py matrix RUN      base surface against tool placement
"""
import json
import os
import sys


def main():
    kind, d = sys.argv[1], sys.argv[2]
    cells = {}
    for l in open(os.path.join(d, "ledger.jsonl")):
        r = json.loads(l)
        p = r["name"].split("/" if kind == "matrix" else "+")
        if kind == "matrix":
            key = (p[1], p[3])
        elif len(p) == 3:
            key = (p[1], p[2])
        else:
            continue
        c = cells.setdefault(key, [0, 0, 0, set()])
        if r["v"] == "ok":
            c[0] += 1
        elif r["v"] in ("refused", "unread"):
            c[1] += 1
        else:
            c[2] += 1
            c[3].add(r["sig"][:50])
    rows = sorted({k[0] for k in cells})
    cols = sorted({k[1] for k in cells})
    print("| |" + "|".join(cols) + "|")
    print("|---|" + "---|" * len(cols))
    for a in rows:
        out = []
        for b in cols:
            c = cells.get((a, b))
            if c is None:
                out.append(" ")
            elif c[2]:
                out.append(f"**{c[2]}F**/{c[0]}")
            elif c[0] == 0:
                out.append("-")  # only refusals: nothing past this point was exercised
            else:
                out.append(str(c[0]))
        print(f"|{a}|" + "|".join(out) + "|")
    total = len(cells)
    dead = sum(1 for c in cells.values() if c[0] == 0 and c[2] == 0)
    bad = sum(1 for c in cells.values() if c[2])
    print(f"\n{total} cells: {bad} with failures (**nF**/built), {dead} where nothing built (-), rest all built or refused clearly.")


if __name__ == "__main__":
    main()
