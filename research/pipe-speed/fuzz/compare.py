#!/usr/bin/env python3
"""Selector part files on an old and a new featcost, sequential: compare.py OLD NEW DIR OUT. A file's
history either fails (same message expected) or resolves, and then its measured result must match."""
import collections, concurrent.futures as cf, glob, os, subprocess, sys

old, new, d, out = sys.argv[1:5]
os.environ["FORGE_BATCH"] = "cpu"
os.makedirs(out, exist_ok=True)


def run(b, f, tag):
    r = out + "/" + tag + os.path.basename(f)
    try:
        p = subprocess.run([b, "resolve-file", f, r], capture_output=True, text=True, timeout=120)
    except subprocess.TimeoutExpired:
        return "timeout", None
    if p.returncode:
        return "crash", None
    if p.stdout.strip():
        return p.stdout.strip().splitlines()[-1][:70], None
    try:
        i = subprocess.run([b, "info", r, "seq"], capture_output=True, text=True, timeout=120).stdout.strip().splitlines()
    except subprocess.TimeoutExpired:
        return "timeout", None
    os.remove(r)
    if not i:
        return "crash", None
    w = i[-1].split()
    return "ok", {w[k]: float(w[k + 1]) for k in range(1, len(w) - 1, 2)}


def one(f):
    return f, run(old, f, "a"), run(new, f, "b")


c = collections.Counter()
errs = collections.Counter()
with cf.ThreadPoolExecutor(4) as pool:
    for f, (ea, a), (eb, b) in pool.map(one, sorted(glob.glob(d + "/*.fc"))):
        n = os.path.basename(f)
        c["parts"] += 1
        errs[ea.split(":")[0] if ea != "ok" else "ok"] += 1
        if ea == "ok" and eb == "ok":
            dv = abs(a["volume"] - b["volume"]) / max(a["volume"], 1e-9)
            if (a["faces"], a["edges"]) != (b["faces"], b["edges"]) or dv > 1e-6 or b["problems"] > a["problems"] or b["open"] > a["open"]:
                c["DIFFER"] += 1
                print("DIFFER", n, a, b)
            else:
                c["same"] += 1
        elif ea == eb:
            c["same failure"] += 1
        elif eb == "ok":
            c["fixed"] += 1
            print("fixed", n, ea, b)
        elif ea == "ok":
            c["WORSE"] += 1
            print("WORSE", n, eb)
        else:
            c["other failure"] += 1
            print("other", n, ea, "|", eb)
print(dict(c))
print("old outcomes", dict(errs))
