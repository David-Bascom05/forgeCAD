#!/usr/bin/env python3
"""Each failing signature's smallest part, shrunk to a minimal .fc, one per signature.

  JUDGE=EXE shrink_sigs.py RESULTS.jsonl PARTS_DIR --out DIR --name DATASET [--key file,body] [--each 1]

Repros keep the dataset's numbers: they stay in the gate's repros, outside the repo, and a test restates them.
"""
import concurrent.futures as cf
import json
import os
import re
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
import fp

BLANK = re.compile(r"-?\d+\.?\d*(?:e-?\d+)?|#[0-9a-f]+|@[0-9a-z]+")


def main():
    a = sys.argv[1:]
    opt = lambda k, d=None: a[a.index(k) + 1] if k in a else d
    out, name = opt("--out"), opt("--name")
    os.makedirs(out, exist_ok=True)
    key = opt("--key", "id").split(",")
    by = {}
    for l in open(a[0]):
        r = json.loads(l)
        if r.get("v") in ("refused", "fail", "crash", "timeout", "oom"):
            stem = ".".join(r[k] for k in key)
            path = os.path.join(a[1], stem + ".fc")
            if os.path.exists(path):
                by.setdefault(BLANK.sub("N", r["sig"]), []).append((os.path.getsize(path), path, stem))

    def one(s):
        rows = sorted(by[s])
        size, path, stem = rows[0]
        src = open(path).read()
        first = fp.judge_text(src, 60)
        want = fp.sig_of(first)
        if BLANK.sub("N", first.get("sig", "")) != s:
            return s, "gone", len(rows)
        mini, probes = fp.shrink(src, want, timeout=60, budget=120, seconds=300.0)
        with open(os.path.join(out, f"{name}-{fp.sig_id(s)}.fc"), "w") as f:
            f.write(f"# {s}\n# {name} {stem}; {len(rows)} parts sign this; shrunk in {probes} probes\n{mini}")
        return s, "shrunk", len(rows)

    with cf.ThreadPoolExecutor(2) as ex:
        for s, how, n in ex.map(one, sorted(by, key=lambda x: -len(by[x]))):
            print(n, how, s[:90], flush=True)


if __name__ == "__main__":
    main()
