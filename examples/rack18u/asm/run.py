#!/usr/bin/env python3
"""Runs asm/rack.fg (capped: 6G, no swap, niced) and prints what failed. --quick runs a scratch copy
with every `expect .. at ..` line dropped (one solve, one overlap pass) for iterating; the full run is
the check. Writes out/rack.json (out/quick.json for --quick). FG=path picks another fg binary.
Run: python3 run.py [--quick] [--render PNG --view VIEW]"""
import collections
import json
import os
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
FG = os.environ.get("FG", os.path.expanduser("~/projects/ForgeCAD/.claude/worktrees/forge-lang/target/release/fg"))
CAP = ["systemd-run", "--user", "--scope", "-q", "-p", "MemoryMax=6G", "-p", "MemorySwapMax=0", "nice"]


def main():
    args = sys.argv[1:]
    quick = "--quick" in args
    args = [a for a in args if a != "--quick"]
    src = os.path.join(HERE, "rack.fg")
    if quick:
        lines = [l for l in open(src) if not (l.startswith("expect ") and " at " in l)]
        fd, src = tempfile.mkstemp(suffix=".fg", dir=HERE, prefix=".quick_")
        os.write(fd, "".join(lines).encode())
        os.close(fd)
    t0 = time.time()
    p = subprocess.run(CAP + [FG, "run", src, "--json"] + args, capture_output=True, text=True, cwd=HERE)
    wall = time.time() - t0
    if quick:
        os.replace(src, os.path.join(HERE, ".quick_last.fg"))
    try:
        d = json.loads(p.stdout)
    except ValueError:
        print(p.stdout[-2000:], p.stderr[-2000:])
        sys.exit(1)
    os.makedirs(os.path.join(HERE, "out"), exist_ok=True)
    json.dump(d, open(os.path.join(HERE, "out", "quick.json" if quick else "rack.json"), "w"), indent=1)
    a = d.get("assembly", {})
    print("ok", d["ok"], "wall %.1f s" % wall, "instances", len(a.get("instances", [])),
          "bom lines", len(a.get("bom", [])), "mobility", a.get("mobility"), "interferences", a.get("interferences"))
    print("timings", a.get("timings_ms"), "overlap", a.get("overlap"))
    for e in d["errors"]:
        if e.get("kind") != "ExpectFailed":
            print("ERROR", e.get("kind"), e.get("text"), e.get("span", {}).get("line"))
    print(collections.Counter(w["kind"] for w in d["warnings"]))
    for w in d["warnings"]:
        if w["kind"] not in ("IntendedContact",):
            print("WARN", w["kind"], w.get("text") or "", w.get("a", ""), w.get("b", ""), w.get("line", ""))
    ex = a.get("expects", [])
    bad = [e for e in ex if not e["ok"]]
    unmeasured = [e for e in ex if e.get("measured") is None]
    print("expects not measured", len(unmeasured))
    for e in unmeasured:
        print("UNMEASURED", e)
    print("expects", len(ex), "failed", len(bad))
    for e in bad:
        print("FAIL", e)
    short = [e for e in a.get("engagements", []) if e.get("short")]
    print("engagements", len(a.get("engagements", [])), "short", len(short))
    for e in short:
        print("SHORT", e)


if __name__ == "__main__":
    main()
