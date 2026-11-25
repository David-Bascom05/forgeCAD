#!/usr/bin/env python3
"""The determinism gate's verdict: every forge-det output file against the reference, bit for bit.

usage: compare.py REF.txt OTHER.txt ..

Each line of forge-det is `DOC OP REST..`. A file is equal to the reference when it has exactly
the reference's lines. Every difference is printed with the document, the op and which field
(body hash, mesh hash, error kinds, refusal) differs; a WARM_DIFFERS line in any file (the warm
rebuild gave other bits than a cold one) is a failure by itself. Exit 1 on any difference.
"""
import sys


def load(path):
    rows, warm, order = {}, [], []
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            x = line.rstrip("\r\n").split(" ")
            if len(x) < 3:
                continue
            key = (x[0], x[1])
            if x[2] == "WARM_DIFFERS":
                warm.append(key)
                continue
            if key not in rows:
                order.append(key)
            rows[key] = x[2:]
    return rows, warm, order


def what(a, b):
    if a is None:
        return "missing in reference"
    if b is None:
        return "missing"
    names = ["body", "mesh", "errors"]
    if a[0] in ("REFUSED", "UNREADABLE", "BAD_OP", "BAD_STREAM") or b[0] in ("REFUSED", "UNREADABLE", "BAD_OP", "BAD_STREAM"):
        return f"outcome {' '.join(a)} vs {' '.join(b)}"
    diff = [n for n, x, y in zip(names, a, b) if x != y]
    return f"{'+'.join(diff) or 'line'} differs: {' '.join(a)} vs {' '.join(b)}"


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    ref, ref_warm, order = load(sys.argv[1])
    bad = 0
    for key in ref_warm:
        print(f"WARM≠COLD {sys.argv[1]}: {key[0]} op {key[1]}")
        bad += 1
    print(f"reference {sys.argv[1]}: {len(ref)} lines")
    for other in sys.argv[2:]:
        rows, warm, _ = load(other)
        diffs = [k for k in order if rows.get(k) != ref[k]] + [k for k in rows if k not in ref]
        for key in warm:
            print(f"WARM≠COLD {other}: {key[0]} op {key[1]}")
        print(f"{other}: {len(rows)} lines, {len(diffs)} differ, {len(warm)} warm≠cold")
        for k in diffs[:200]:
            print(f"  DIVERGED {k[0]} op {k[1]}: {what(ref.get(k), rows.get(k))}")
        bad += len(diffs) + len(warm)
    print("IDENTICAL" if bad == 0 else f"{bad} DIFFERENCES")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
