#!/usr/bin/env python3
"""The determinism gate's verdict: every forge-det output file against the reference, bit for bit.

usage: compare.py REF.txt OTHER.txt ..

Each line of forge-det is `DOC OP REST..`. A file is equal to the reference when it has exactly
the reference's lines. Every difference is printed with the document, the op and which field
(body hash, mesh hash, error kinds, refusal) differs; a WARM_DIFFERS line in any file (the warm
rebuild gave other bits than a cold one) is a failure by itself. Exit 1 on any difference in
geometry.

Scope (David's ruling): bit identity is required for geometry (the kernel, model builds,
imports, assemblies, sheet). Toolpaths, CAM and simulations must be correct but need not be
bit-identical across platforms, so a document whose first statement is `machining` (a CAM
document) is report-only: its differences, warm≠cold included, are printed and never fail.
"""
import os
import sys

# forge-det names documents relative to the repository root; CI runs this from another folder
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


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


_cam = {}


def report_only(doc):
    """A CAM document (first statement `machining`) is reported, never failed. A stream's lines
    name their document `DOC+ops`. A document that cannot be read is geometry (fails hard)."""
    path = doc.split("+", 1)[0]
    if path not in _cam:
        first = ""
        try:
            with open(os.path.join(ROOT, path), encoding="utf-8", errors="replace") as f:
                for line in f:
                    t = line.split("#", 1)[0].strip()
                    if t:
                        first = t
                        break
        except OSError:
            pass
        _cam[path] = first == "machining"
    return _cam[path]


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
        cam = report_only(key[0])
        print(f"WARM≠COLD {sys.argv[1]}: {key[0]} op {key[1]}{' (CAM, report only)' if cam else ''}")
        bad += 0 if cam else 1
    print(f"reference {sys.argv[1]}: {len(ref)} lines")
    for other in sys.argv[2:]:
        rows, warm, _ = load(other)
        diffs = [k for k in order if rows.get(k) != ref[k]] + [k for k in rows if k not in ref]
        for key in warm:
            cam = report_only(key[0])
            print(f"WARM≠COLD {other}: {key[0]} op {key[1]}{' (CAM, report only)' if cam else ''}")
        print(f"{other}: {len(rows)} lines, {len(diffs)} differ, {len(warm)} warm≠cold")
        hard = [k for k in diffs if not report_only(k[0])]
        for k in hard[:200]:
            print(f"  DIVERGED {k[0]} op {k[1]}: {what(ref.get(k), rows.get(k))}")
        for k in [k for k in diffs if report_only(k[0])][:200]:
            print(f"  REPORT (CAM, not a failure) {k[0]} op {k[1]}: {what(ref.get(k), rows.get(k))}")
        bad += len(hard) + len([k for k in warm if not report_only(k[0])])
    print("IDENTICAL" if bad == 0 else f"{bad} DIFFERENCES")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
