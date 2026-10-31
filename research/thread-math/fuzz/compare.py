#!/usr/bin/env python3
"""Each part of DIR on an old and a new binary (featcost info), sequential and n-ary 2:
compare.py OLD NEW DIR [--workers 4]. New must not fail, be invalid or open where old was not;
faces, edges and volume must match. Prints each difference; counts at the end."""
import collections, concurrent.futures as cf, glob, os, subprocess, sys

old, new, d = sys.argv[1:4]
os.environ["FORGE_BATCH"] = "cpu"
def run(b, f, m):
    try:
        r = subprocess.run([b, "info", f, m], capture_output=True, text=True, timeout=120)
    except subprocess.TimeoutExpired:
        return {"state": "timeout"}
    last = (r.stdout.strip().splitlines() or ["crash"])[-1]
    if r.returncode or not last.startswith("ok"):
        return {"state": "err", "why": (last if last != "crash" else r.stderr.strip()[-120:])[:120]}
    w = last.split()
    return {"state": "ok", **{w[i]: float(w[i + 1]) for i in range(1, len(w) - 1, 2)}}
def one(f):
    out = []
    for m in ("seq", "nary2"):
        a, b = run(old, f, m), run(new, f, m)
        out.append((f, m, a, b))
    return out
c = collections.Counter()
with cf.ThreadPoolExecutor(4) as pool:
    for res in pool.map(one, sorted(glob.glob(d + "/*.fc"))):
        for f, m, a, b in res:
            n = os.path.basename(f)
            c["runs"] += 1
            if a["state"] != "ok" and b["state"] != "ok":
                c["both fail"] += 1
            elif a["state"] != "ok":
                ok = b["problems"] == 0 and b["open"] == 0
                c["fixed" if ok else "new invalid (old failed)"] += 1
                if not ok:
                    print("NEW INVALID (old failed)", n, m, b)
            elif b["state"] != "ok":
                c["WORSE fail"] += 1
                print("WORSE", n, m, a["state"], b)
            else:
                if (b["problems"] > a["problems"]) or (b["open"] > a["open"]):
                    c["WORSE valid"] += 1
                    print("WORSE VALID", n, m, a, b)
                elif b["problems"] < a["problems"] or b["open"] < a["open"]:
                    c["better valid"] += 1
                dv = abs(a["volume"] - b["volume"]) / max(a["volume"], 1e-9)
                if a["faces"] != b["faces"] or a["edges"] != b["edges"] or dv > 1e-6:
                    c["differ"] += 1
                    print("DIFFER", n, m, "faces", a["faces"], b["faces"], "edges", a["edges"], b["edges"], "dV", f"{dv:.2e}", "problems", a["problems"], b["problems"], "open", a["open"], b["open"])
                else:
                    c["same"] += 1
print(dict(c))
