#!/usr/bin/env python3
"""Before and after an importer change, per dataset: classes, untranslated causes, and every newly translated part
held to its source's volume and area. Reads run directories (results.jsonl, as run_freecad.py, run_cadfs.py and
run_whucad.py write them) and, for CADFS and WHUCAD, step_truth.py's measures. Data stays outside the repo.

  tally.py freecad BEFORE AFTER [--tol 1e-4]
  tally.py cadfs BEFORE AFTER --truth TRUTH.jsonl [--tol 1e-4] [--fillet]
  tally.py whucad BEFORE AFTER --truth TRUTH.jsonl [--tol 1e-3] [--fillet]

--fillet keeps the parts with a fillet feature. A part matches when it built and both its volume and area are within
--tol of the source's (relative); a part that built with no source measure is "no truth".
"""
import collections
import json
import os
import sys


def rows(run):
    return [json.loads(l) for l in open(os.path.join(run, "results.jsonl"))]


def key(kind, r):
    return (r["file"], r["body"]) if kind == "freecad" else r["id"]


def truth_of(kind, r, truths):
    if kind == "freecad":
        t = r.get("truth") or {}
    else:
        t = truths.get(r["id"]) or r.get("truth") or {}
    return t if t.get("vol") and t.get("area") and t.get("solids", 1) else None


def klass(kind, r, truths, tol):
    if r["status"] == "untranslated":
        return "untranslated"
    v = r.get("v")
    if v != "ok":
        return v or r["status"]
    t = truth_of(kind, r, truths)
    if t is None:
        return "no truth"
    ev = abs(r["vol"] - t["vol"]) / t["vol"]
    ea = abs(r["area"] - t["area"]) / t["area"]
    return "match" if ev <= tol and ea <= tol else "mismatch"


def err(kind, r, truths):
    t = truth_of(kind, r, truths) if r.get("v") == "ok" else None
    if t is None:
        return ""
    return f"vol {abs(r['vol'] - t['vol']) / t['vol']:.1e}, area {abs(r['area'] - t['area']) / t['area']:.1e}"


def main(a):
    kind, before, after = a[0], a[1], a[2]
    opt = lambda k, d=None: a[a.index(k) + 1] if k in a else d
    tol = float(opt("--tol", "1e-3" if kind == "whucad" else "1e-4"))
    truths = {}
    if opt("--truth"):
        for l in open(opt("--truth")):
            d = json.loads(l)
            truths[d["id"]] = d
    B = {key(kind, r): r for r in rows(before)}
    A = {key(kind, r): r for r in rows(after)}
    if "--fillet" in a:
        keep = lambda r: any(f.lower() == "fillet" for f in r.get("features", []))
        B = {k: r for k, r in B.items() if keep(r)}
        A = {k: r for k, r in A.items() if k in B}
    C = collections.Counter
    kb = {k: klass(kind, r, truths, tol) for k, r in B.items()}
    ka = {k: klass(kind, A[k], truths, tol) for k in B if k in A}
    print(f"## {kind}: {len(B)} parts, match at {tol:g}\n")
    print("| class | before | after |\n|---|---:|---:|")
    for c in ("match", "mismatch", "no truth", "refused", "fail", "crash", "oom", "timeout", "stopped", "untranslated"):
        x, y = C(kb.values())[c], C(ka.values())[c]
        if x or y:
            print(f"| {c} | {x} | {y} |")
    print("\n| untranslated cause | before | after |\n|---|---:|---:|")
    cb = C(B[k]["why"] for k in B if kb[k] == "untranslated")
    ca = C(A[k]["why"] for k in ka if ka[k] == "untranslated")
    for w, _ in (cb + ca).most_common():
        print(f"| {w} | {cb[w]} | {ca[w]} |")
    print("\n### Newly translated, by cause before\n")
    new = [k for k in ka if kb[k] == "untranslated" and ka[k] != "untranslated"]
    by = collections.defaultdict(list)
    for k in new:
        by[B[k]["why"]].append(k)
    print("| cause before | translated | match | mismatch | refused/fail | no truth |\n|---|---:|---:|---:|---:|---:|")
    for w, ks in sorted(by.items(), key=lambda x: -len(x[1])):
        c = C(ka[k] for k in ks)
        print(f"| {w} | {len(ks)} | {c['match']} | {c['mismatch']} | {sum(v for x, v in c.items() if x not in ('match', 'mismatch', 'no truth'))} | {c['no truth']} |")
    print("\n### Every change of class\n")
    for k in sorted(ka, key=str):
        if kb[k] != ka[k]:
            why = A[k].get("why") or (A[k].get("sig") or "")[:80]
            print(f"- {k if isinstance(k, str) else '.'.join(k)}: {kb[k]} -> {ka[k]} {err(kind, A[k], truths)} {why}".rstrip())


if __name__ == "__main__":
    main(sys.argv[1:])
