#!/usr/bin/env python3
"""A dataset's fillet parts in lots of 200 on a build: tally.py EXE PARTS_DIR TRUTH.jsonl OUT.jsonl [--jobs 3] [--kind fillet]

Works over any directory of .fc parts (WHUCAD, CADFS, FreeCAD). OUT holds a line per part; a stopped run carries on.
Prints the table: match, off, refused by signature. Truth is any jsonl with id, vol, area; a part with
none counts as built.
"""
import argparse, collections, concurrent.futures as cf, json, os, resource, subprocess

MATCH = 1e-4
CPU = 180


def lim():
    os.nice(15)
    resource.setrlimit(resource.RLIMIT_AS, (2500 << 20, 2500 << 20))
    resource.setrlimit(resource.RLIMIT_CPU, (CPU, CPU + 1))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def one(a):
    exe, path = a
    try:
        p = subprocess.run([exe, path], capture_output=True, text=True, preexec_fn=lim, timeout=CPU * 6)
    except subprocess.TimeoutExpired:
        return {"v": "timeout"}
    if p.returncode or not p.stdout.strip():
        return {"v": "crash", "sig": (p.stderr.strip().splitlines() or [""])[0][:160]}
    return json.loads(p.stdout.strip().splitlines()[-1])


def main():
    ap = argparse.ArgumentParser()
    for x in "exe parts truth out".split():
        ap.add_argument(x)
    ap.add_argument("--jobs", type=int, default=3)
    ap.add_argument("--kind", default="fillet")
    a = ap.parse_args()
    truth = {}
    if os.path.exists(a.truth):
        for l in open(a.truth):
            d = json.loads(l)
            truth[d["id"]] = d
    done = {}
    if os.path.exists(a.out):
        for l in open(a.out):
            d = json.loads(l)
            done[d["id"]] = d
    ids = []
    for f in sorted(os.listdir(a.parts)):
        if f.endswith(".fc") and f[:-3] not in done:
            src = open(os.path.join(a.parts, f), errors="replace").read()
            if any(w.split(":")[-1].split()[:1] == [a.kind] or w.startswith(a.kind) for w in src.splitlines()):
                ids.append(f[:-3])
    with open(a.out, "a") as o:
        for k in range(0, len(ids), 200):
            lot = ids[k:k + 200]
            with cf.ThreadPoolExecutor(a.jobs) as ex:
                for i, r in zip(lot, ex.map(one, [(a.exe, os.path.join(a.parts, i + ".fc")) for i in lot])):
                    r["id"] = i
                    done[i] = r
                    o.write(json.dumps(r) + "\n")
            o.flush()
    c, sigs = collections.Counter(), collections.Counter()
    for i, r in done.items():
        t = truth.get(i)
        if r["v"] == "ok":
            if not t or not t.get("vol"):
                k = "built"
            else:
                e = abs(r["vol"] - t["vol"]) / t["vol"]
                g = abs(r["area"] - t["area"]) / t["area"]
                k = "match" if e < MATCH and g < MATCH else "off"
        else:
            k = r["v"]
            sigs[r.get("sig", "")[:34].split("{")[0].strip()] += 1
        c[k] += 1
    print(dict(c))
    for s, n in sigs.most_common(25):
        print(n, s)


main()
