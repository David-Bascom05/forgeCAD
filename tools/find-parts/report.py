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
NOT_KERNEL = re.compile(r"unknown parameter|: expected |: unknown |: unexpected |volume off")
EARLY = 60.0
FAILS = ("fail", "wrong", "timeout", "crash", "oom")


def root(sig):
    """The failure itself: no verdict, no feature kind, no wrappers, one name for one kind of error."""
    s = re.sub(r"^\w+:\s*", "", sig, count=1)
    s = re.sub(r" @.*$", "", s) if "panic" not in s else s
    nary = s.startswith("nary: ")
    parts = s.split(": ")
    while len(parts) > 1 and re.fullmatch(r"\w+", parts[0]) and parts[0] not in ("boolean", "mesh"):
        parts = parts[1:]
    s = ": ".join(parts)
    s = re.sub(r"\b(Direct|Rib|Thicken|Boolean|Rebuild)\(", "", s).rstrip(")") if "panic" not in s else s
    m = re.search(r"Internal\(\"([^\"]*)\"?", s)
    if m:
        s = "boolean: " + m.group(1)
    m = re.search(r"Invalid\(\[(\w+)", s) or re.fullmatch(r"invalid (\w+)", s)
    if m:
        s = "invalid " + m.group(1)
    return ("nary " if nary else "") + s.split(" {")[0]


def load(d):
    cases, cpu, sigs, refused = 0, 0.0, {}, {}
    early = set()
    for l in open(os.path.join(d, "ledger.jsonl")):
        r = json.loads(l)
        cpu += r.get("cpu") or 0
        if r["v"] == "ok" or r["cpu"]:
            cases += 1
        s = r["sig"]
        # a generator's bad line is no kernel failure; a volume off the data's truth is first a translation question
        if not s or KNOWN_LIMIT in s or NOT_KERNEL.search(s):
            continue
        if r["v"] in FAILS:
            sigs[s] = sigs.get(s, 0) + 1
            if cpu <= EARLY:
                early.add(root(s))
        elif r["v"] == "refused":
            refused[s] = refused.get(s, 0) + 1
    return cases, cpu, sigs, refused, early


def main():
    a = sys.argv[1:]
    real = set()
    if "--real" in a:
        i = a.index("--real")
        _, _, s, rf, _ = load(a[i + 1])
        real = {root(k) for k in list(s) + list(rf)}
        del a[i : i + 2]
    print("| method | parts | CPU h | failing parts | distinct failures | distinct roots | roots per CPU h | roots in first 60 CPU-s | roots seen in real parts | distinct refusals |")
    print("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    allr = {}
    for d in a:
        c, cpu, s, rf, early = load(d)
        roots = {root(k) for k in s}
        for r in roots:
            allr.setdefault(r, set()).add(os.path.basename(d))
        h = max(cpu / 3600, 1e-9)
        print(f"| {os.path.basename(d)} | {c} | {h:.2f} | {sum(s.values())} | {len(s)} | {len(roots)} | {len(roots) / h:.0f} | {len(early)} | {len(roots & real)} | {len(rf)} |")
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
