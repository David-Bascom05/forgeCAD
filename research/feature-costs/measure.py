#!/usr/bin/env python3
"""Per-feature instructions:u by prefix difference, and mesh/mass/check per part.

usage (from the checkout root, inside one fbuild scope):
  measure.py BIN FCDIR OUT.tsv MODE [WORKERS] [FILTER]
MODE: seq | nary1 (prefixes), post (mesh, mass, check on each finished part).
FCDIR holds the resolved bench parts; every other source comes from `featcost sources`.
"""
import concurrent.futures as cf, glob, os, re, subprocess, sys

bin_, fcdir, out, mode = sys.argv[1:5]
workers = int(sys.argv[5]) if len(sys.argv) > 5 else 4
pat = re.compile(sys.argv[6]) if len(sys.argv) > 6 else None
os.environ["FORGE_BATCH"] = "cpu"


def count(*args):
    r = subprocess.run(
        ["perf", "stat", "-x,", "-e", "instructions:u", bin_, *args],
        capture_output=True, text=True,
    )
    n = -1
    for line in r.stderr.splitlines():
        if "instructions" in line:
            try:
                n = int(line.split(",")[0])
            except ValueError:
                pass
    return n, r.stdout.strip() if r.returncode == 0 else "crash"


srcs = [("bench/" + os.path.basename(f)[:-3], f) for f in sorted(glob.glob(fcdir + "/*.fc"))]
srcs += [(s, s) for s in subprocess.run([bin_, "sources"], capture_output=True, text=True).stdout.split("\n") if s]
if pat:
    srcs = [s for s in srcs if pat.search(s[0])]
pool = cf.ThreadPoolExecutor(workers)
tsv = open(out, "w")

if mode == "post":
    for name, src in srcs:
        fs = {w: pool.submit(count, "post", src, w) for w in ("none", "mesh", "mass", "check")}
        base = fs["none"].result()[0]
        row = [fs[w].result() for w in ("none", "mesh", "mass", "check")]
        print(name, base, *[r[0] - base if r[1] != "crash" else -1 for r in row[1:]], sep="\t", file=tsv, flush=True)
    sys.exit(0)

for name, src in srcs:
    feats = [l.split("\t") for l in subprocess.run([bin_, "list", src], capture_output=True, text=True).stdout.splitlines()]
    # a prefix ending on a feature that makes nothing costs the same as the one before it
    wanted = [0] + [i + 1 for i, f in enumerate(feats) if f[1] != "none"]
    res = dict(zip(wanted, pool.map(lambda n: count("run", src, str(n), mode), wanted)))
    prev = res[0][0]
    for i, f in enumerate(feats):
        if f[1] == "none":
            continue
        n, status = res[i + 1]
        print(name, i, f[1], f[2], status.replace(" ", ":"), n - prev, n, sep="\t", file=tsv, flush=True)
        prev = n
