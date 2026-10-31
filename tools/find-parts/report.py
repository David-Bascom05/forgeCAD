#!/usr/bin/env python3
"""The method table: from run ledgers, distinct failures per CPU-hour.

  report.py RUN_DIR.. [--real RUN_DIR]    prints the markdown table

A signature names the feature and the failure; its root drops the feature (the same boolean error on a
hole and on an extrude count once). "seen in real parts" counts roots a --real run also hit.
"""
import json
import os
import re
import sys

KNOWN_LIMIT = "nary: the part is"
FAILS = ("fail", "wrong", "timeout", "crash", "oom")


def root(sig):
    s = re.sub(r"^\w+:\s*", "", sig, count=1)  # verdict
    s = re.sub(r" @.*$", "", s) if "panic" not in s else s
    parts = s.split(": ")
    while len(parts) > 1 and re.fullmatch(r"\w+", parts[0]) and parts[0] not in ("boolean", "nary", "mesh"):
        parts = parts[1:]
    return ": ".join(parts)


def load(d):
    cases, cpu, sigs, refused = 0, 0.0, {}, {}
    seen = set()
    for l in open(os.path.join(d, "ledger.jsonl")):
        r = json.loads(l)
        cpu += r.get("cpu") or 0
        if r["v"] == "ok" or r["cpu"]:
            cases += 1
        s = r["sig"]
        if not s or KNOWN_LIMIT in s:
            continue
        if r["v"] in FAILS:
            sigs[s] = sigs.get(s, 0) + 1
        elif r["v"] == "refused":
            refused[s] = refused.get(s, 0) + 1
    return cases, cpu, sigs, refused


def main():
    a = sys.argv[1:]
    real = set()
    if "--real" in a:
        i = a.index("--real")
        _, _, s, rf = load(a[i + 1])
        real = {root(k) for k in list(s) + list(rf)}
        del a[i : i + 2]
    print("| method | parts | CPU h | failing parts | distinct failures | distinct roots | roots per CPU h | roots seen in real parts | distinct refusals |")
    print("|---|---:|---:|---:|---:|---:|---:|---:|---:|")
    allr = {}
    for d in a:
        c, cpu, s, rf = load(d)
        roots = {root(k) for k in s}
        for r in roots:
            allr.setdefault(r, set()).add(os.path.basename(d))
        h = max(cpu / 3600, 1e-9)
        print(f"| {os.path.basename(d)} | {c} | {h:.2f} | {sum(s.values())} | {len(s)} | {len(roots)} | {len(roots) / h:.0f} | {len(roots & real)} | {len(rf)} |")
    only = {}
    for r, ms in allr.items():
        if len(ms) == 1:
            only.setdefault(next(iter(ms)), []).append(r)
    print()
    for m, rs in sorted(only.items()):
        print(f"only {m} ({len(rs)}):")
        for r in sorted(rs)[:40]:
            print("  ", r[:110])


if __name__ == "__main__":
    main()
