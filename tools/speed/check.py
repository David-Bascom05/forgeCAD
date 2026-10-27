#!/usr/bin/env python3
"""Speed and bit-identity gate: user-space instructions for one full regen of each part on each
path against a saved baseline, and examples/bits' hash of every regen's exact result against
tools/speed/bits.txt. Exit status: 1 a speed regression above the threshold, 2 a changed or
failing result hash (expected when a change means to alter geometry; re-save then), 3 both.

  tools/speed/check.py                 build regen1, compare with tools/speed/baseline.txt
  tools/speed/check.py --save          write the current counts as the baseline
  tools/speed/check.py --only torture  just one part
  tools/speed/check.py --save-bits     write only the bit hashes (results changed on purpose)
  options: --baseline FILE --bits FILE --threshold 0.5 (percent) --no-build --bin PATH --no-bits
"""
import argparse
import os
import subprocess
import sys

PARTS = ["torture", "shaft", "housing", "coplanar", "patterns", "mixed", "fuzz1", "fuzz2", "fuzz5"]
PATHS = ["seq", "nary1"]
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))


def count(binary, part, path):
    r = subprocess.run(
        ["perf", "stat", "-x,", "-e", "instructions:u", binary, part, path],
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        return None, r.stderr.strip().splitlines()[-3:]
    for line in r.stderr.splitlines():
        f = line.split(",")
        if len(f) > 2 and f[2].startswith("instructions"):
            return int(f[0]), None
    return None, ["no perf output"]


def load(path):
    base = {}
    if os.path.exists(path):
        for line in open(path):
            f = line.split()
            if len(f) == 3 and not line.startswith("#"):
                base[(f[0], f[1])] = int(f[2])
    return base


def check_bits(binary, a):
    r = subprocess.run([binary], capture_output=True, text=True)
    if r.returncode != 0:
        print("bits: FAILED to run")
        return True
    now = dict(line.rsplit(" ", 1) for line in r.stdout.splitlines() if line.strip())
    if a.save or a.save_bits:
        with open(a.bits, "w") as out:
            out.write(r.stdout)
        print(f"saved {a.bits} ({len(now)} hashes)")
        return False
    old = {}
    if os.path.exists(a.bits):
        old = dict(line.strip().rsplit(" ", 1) for line in open(a.bits) if line.strip())
    changed = [k for k in now if old.get(k) != now[k]] + [k for k in old if k not in now]
    if changed:
        print(f"bits: {len(changed)} of {len(now)} results CHANGED: {', '.join(changed)}")
    else:
        print(f"bits: all {len(now)} results bit-identical")
    return bool(changed)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--save", action="store_true")
    ap.add_argument("--baseline", default=os.path.join(HERE, "baseline.txt"))
    ap.add_argument("--threshold", type=float, default=0.5)
    ap.add_argument("--no-build", action="store_true")
    ap.add_argument("--bin")
    ap.add_argument("--bits", default=os.path.join(HERE, "bits.txt"))
    ap.add_argument("--no-bits", action="store_true")
    ap.add_argument("--save-bits", action="store_true")
    ap.add_argument("--only", action="append")
    a = ap.parse_args()
    target = os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target"))
    binary = a.bin or os.path.join(target, "release", "examples", "regen1")
    if not a.no_build and not a.bin:
        subprocess.run(
            ["cargo", "build", "-q", "--release", "-p", "forge-model", "--example", "regen1", "--example", "bits"],
            cwd=ROOT,
            check=True,
        )
    bits_bad = False if a.no_bits else check_bits(os.path.join(os.path.dirname(binary), "bits"), a)
    if a.save_bits:
        return 0
    base = load(a.baseline)
    rows, failed = [], False
    print(f"{'part':<10} {'path':<6} {'base M':>10} {'now M':>10} {'change':>8}")
    for part in a.only or PARTS:
        for path in PATHS:
            n, err = count(binary, part, path)
            if n is None:
                print(f"{part:<10} {path:<6} FAILED: {' / '.join(err)}")
                failed = True
                continue
            rows.append((part, path, n))
            b = base.get((part, path))
            if b is None:
                print(f"{part:<10} {path:<6} {'-':>10} {n / 1e6:>10.1f} {'new':>8}")
                continue
            ch = (n - b) / b * 100
            mark = "  REGRESSION" if ch > a.threshold else ""
            failed |= ch > a.threshold
            print(f"{part:<10} {path:<6} {b / 1e6:>10.1f} {n / 1e6:>10.1f} {ch:>+7.2f}%{mark}")
    if a.save:
        with open(a.baseline, "w") as out:
            out.write("# part path instructions:u (tools/speed/check.py --save)\n")
            for part, path, n in rows:
                out.write(f"{part} {path} {n}\n")
        print(f"saved {a.baseline}")
        return 0
    return (1 if failed else 0) | (2 if bits_bad else 0)


if __name__ == "__main__":
    sys.exit(main())
