#!/usr/bin/env python3
"""Real parts' fillets on a build: run.py EXE PARTS_DIR TRUTH.jsonl SPLIT.tsv OUT.jsonl [--jobs 4] [--only ID,ID]

Each part in its own process, one JSON line per part (verify/forge-proof/examples/filletcheck.rs), held
against the source's STEP. Prints the rate leaving out what the dataset split calls an importer's error.
"""
import argparse, collections, concurrent.futures as cf, json, os, resource, subprocess

CPU = 180
# tighter than the pilot's 1e-3: a wrong corner on a big part hid under that (00307228, 00702917)
MATCH = 1e-4


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
        return {"v": "crash", "sig": (p.stderr.strip().splitlines() or [""])[0][:160], "rc": p.returncode}
    return json.loads(p.stdout.strip().splitlines()[-1])


def main():
    ap = argparse.ArgumentParser()
    for x in "exe parts truth split out".split():
        ap.add_argument(x)
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--only", default="")
    a = ap.parse_args()
    truth = {}
    for l in open(a.truth):
        d = json.loads(l)
        truth[d["id"]] = d
    split = {}
    for l in open(a.split):
        w = l.rstrip("\n").split("\t")
        split[w[0]] = w
    ids = []
    for f in sorted(os.listdir(a.parts)):
        src = open(os.path.join(a.parts, f)).read()
        if any(l.split(":")[-1].split()[:1] == ["fillet"] or l.startswith("fillet") for l in src.splitlines()):
            ids.append(f[:-3])
    if a.only:
        ids = a.only.split(",")
    with cf.ThreadPoolExecutor(a.jobs) as ex:
        res = list(ex.map(one, [(a.exe, os.path.join(a.parts, i + ".fc")) for i in ids]))
    c, sigs = collections.Counter(), collections.Counter()
    with open(a.out, "w") as o:
        for i, r in zip(ids, res):
            r["id"] = i
            lab = split[i][2] if i in split else ""
            r["label"] = lab
            t = truth.get(i)
            if r["v"] == "ok" and t and t.get("vol"):
                r["evol"] = abs(r["vol"] - t["vol"]) / max(t["vol"], 1e-12)
                r["earea"] = abs(r["area"] - t["area"]) / max(t["area"], 1e-12)
                k = "match" if r["evol"] < MATCH and r["area"] and r["earea"] < MATCH else "off"
            elif r["v"] == "ok":
                k = "no-truth"
            else:
                k = r["v"]
                sigs[(lab, r.get("sig", "")[:50].split("{")[0])] += 1
            r["k"] = k
            c[(k, "importer" if lab == "importer-suspect" else "")] += 1
            o.write(json.dumps(r) + "\n")
    print(sorted(c.items()))
    for s, n in sigs.most_common(30):
        print(n, s)
    bad = [r for r in map(json.loads, open(a.out)) for x in r.get("audits", []) if x["unsup"] or x["kink"] > 1e-6 or x.get("needless") or x.get("patch_off", 0) > 1e-5]
    print("audit flags", len(bad))


main()
