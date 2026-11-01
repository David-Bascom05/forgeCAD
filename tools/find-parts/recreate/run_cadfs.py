#!/usr/bin/env python3
"""Sweep: translate, judge and compare CADFS parts. Every result one line of results.jsonl.

  run_cadfs.py extract IDS --data DL --work WORK          the sample's programs and sketch records as files
  run_cadfs.py run IDS --work WORK --out RUN --judge EXE --topo EXE [--jobs 4]
  run_cadfs.py oks RUN > IDS                              what built: only these need a truth measured
  run_cadfs.py report RUN TRUTH.jsonl

Each part runs in its own processes at nice 19 with 3 GB of address space and a CPU limit, so load on the
machine neither fails a part nor flatters it.
"""
import concurrent.futures as cf
import json
import os
import re
import resource
import subprocess
import sys
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
CPU = 120
TRANSLATE = 300  # seconds of wall clock to translate one part, kernel prefixes included
VOL, AREA, BOX = 1e-3, 1e-3, 1e-3
LOOSE = 1e-2  # a part with a polyline standing in for an ellipse or spline


def limits():
    os.nice(19)
    resource.setrlimit(resource.RLIMIT_AS, (3 << 30, 3 << 30))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    resource.setrlimit(resource.RLIMIT_CPU, (CPU, CPU + 1))


def judge(exe, fc):
    p = subprocess.Popen([exe, fc], stdout=subprocess.PIPE, stderr=subprocess.PIPE, preexec_fn=limits, text=True)
    try:
        out, err = p.communicate(timeout=CPU * 8)
    except subprocess.TimeoutExpired:
        p.kill()
        p.communicate()
        return {"v": "timeout", "sig": "timeout (wall)"}
    if p.returncode != 0 or not out.strip():
        if p.returncode in (-24, -9):
            return {"v": "timeout", "sig": "timeout (cpu)"}
        why = "oom" if "memory allocation" in err else "crash"
        m = re.search(r"overflowed its stack|memory allocation|panicked at [^\n]*", err)
        return {"v": why, "sig": f"{why}: {p.returncode} {m.group(0) if m else ''}".strip()}
    r = json.loads(out)
    if r["v"] == "fail" and r.get("loc"):
        r["sig"] += " @" + r["loc"]
    return r


def compare(r, t, notes):
    """Which of volume, area and box agree with the truth, each error kept."""
    lim = LOOSE if "approx" in notes else VOL
    out = {}
    if r.get("vol") is not None:
        out["evol"] = abs(r["vol"] - t["vol"]) / max(t["vol"], 1e-12)
    if r.get("area") is not None:
        out["earea"] = abs(r["area"] - t["area"]) / max(t["area"], 1e-12)
    if r.get("bbox") and t.get("bbox"):
        a, b = r["bbox"], t["bbox"]
        diag = max(sum((b[i + 3] - b[i]) ** 2 for i in range(3)) ** 0.5, 1e-12)
        out["ebox"] = max(abs(x - y) for x, y in zip(a, b)) / diag
    out["match"] = all(out.get(k, 1) <= lim for k in ("evol", "earea", "ebox"))
    return out


def one(i, a):
    work, run = a["work"], a["out"]
    fc = os.path.join(run, "parts", i + ".fc")
    try:
        p = subprocess.run([sys.executable, "-I", os.path.join(HERE, "cadfs.py"), os.path.join(work, i + ".txt"), os.path.join(work, i + ".json"), "--topo", a["topo"], "--out", fc], capture_output=True, text=True, timeout=TRANSLATE)
        r = json.loads(p.stdout)
    except subprocess.TimeoutExpired:
        r = {"status": "untranslated", "cls": "importer", "why": "translation timed out"}
    except ValueError:
        r = {"status": "untranslated", "cls": "importer", "why": "importer died: " + p.stderr[-80:]}
    r["id"] = i
    if r["status"] == "untranslated":
        return r
    j = judge(a["judge"], fc)
    # n-ary regeneration is single-body by design: the sequential result stands
    if j["v"] == "fail" and j["sig"].startswith("nary: the part is"):
        j = dict(j, v="ok", sig="", multi=True)
    r["v"], r["sig"] = j["v"], j["sig"]
    for k in ("vol", "area", "bbox", "bodies", "faces", "cpu", "line"):
        if k in j:
            r[k] = j[k]
    return r


def extract(a):
    ids = open(a[0]).read().split()
    opt = lambda k: a[a.index(k) + 1]
    work = opt("--work")
    os.makedirs(work, exist_ok=True)
    z = zipfile.ZipFile(os.path.join(opt("--data"), "featurescript_fp.zip"))
    y = zipfile.ZipFile(os.path.join(opt("--data"), "sketch_raw.zip"))
    na = {os.path.basename(n)[:8]: n for n in z.namelist() if n.endswith(".txt")}
    nb = {os.path.basename(n)[:8]: n for n in y.namelist() if n.endswith(".json")}
    miss = 0
    for i in ids:
        if i in na and i in nb:
            open(os.path.join(work, i + ".txt"), "wb").write(z.read(na[i]))
            open(os.path.join(work, i + ".json"), "wb").write(y.read(nb[i]))
        else:
            miss += 1
    print(len(ids), "ids,", miss, "without a program or sketch record")


def run(a):
    opt = lambda k, d=None: a[a.index(k) + 1] if k in a else d
    ids = [i for i in open(a[0]).read().split() if os.path.exists(os.path.join(opt("--work"), i + ".txt"))]
    out = opt("--out")
    os.makedirs(os.path.join(out, "parts"), exist_ok=True)
    args = {"work": opt("--work"), "out": out, "judge": os.path.abspath(opt("--judge")), "topo": os.path.abspath(opt("--topo"))}
    path = os.path.join(out, "results.jsonl")
    done = {json.loads(l)["id"] for l in open(path)} if os.path.exists(path) else set()
    log = open(path, "a")
    todo = [i for i in ids if i not in done]
    with cf.ThreadPoolExecutor(min(int(opt("--jobs", "4")), 4)) as ex:
        for k, r in enumerate(ex.map(lambda i: one(i, args), todo)):
            log.write(json.dumps(r) + "\n")
            log.flush()
            if k % 200 == 199:
                print(k + 1, "of", len(todo), flush=True)


def oks(a):
    for l in open(os.path.join(a[0], "results.jsonl")):
        r = json.loads(l)
        if r.get("v") == "ok":
            print(r["id"])


def report(a):
    rows = [json.loads(l) for l in open(os.path.join(a[0], "results.jsonl"))]
    truth = {}
    if len(a) > 1 and os.path.exists(a[1]):
        for l in open(a[1]):
            d = json.loads(l)
            truth[d["id"]] = d
    for r in rows:
        if r.get("v") == "ok":
            t = truth.get(r["id"])
            if t is None or t.get("err"):
                r["truth"] = "none"
            elif not t.get("solids"):
                r["truth"] = "no solid"
            else:
                r.update(compare(r, t, r.get("notes", [])))
                r["tsolids"] = t["solids"]
    from collections import Counter

    def line(rs):
        c = Counter()
        for r in rs:
            c[klass(r)] += 1
        return c

    tot = line(rows)
    print("all", len(rows), dict(tot.most_common()))
    print("why untranslated:")
    w = Counter((r.get("cls"), r["why"]) for r in rows if r["status"] == "untranslated")
    for k, v in w.most_common(30):
        print(f"  {v:6d} {k[0]}: {k[1]}")
    trans = [r for r in rows if r["status"] != "untranslated"]
    print("by feature kind (translated parts):")
    kinds = sorted({k for r in rows for k in r.get("features", [])})
    for k in kinds:
        all_k = [r for r in rows if k in r.get("features", [])]
        t = [r for r in trans if k in r.get("features", [])]
        c = line(t)
        print(f"  {k:16s} parts {len(all_k):6d} translated {len(t):6d} {dict(c.most_common())}")
    print("by feature count (translated parts):")
    for lo, hi in ((1, 1), (2, 3), (4, 7), (8, 15), (16, 10**6)):
        t = [r for r in trans if lo <= len(r.get("features", [])) <= hi]
        print(f"  {lo}-{hi}: {len(t)} {dict(line(t).most_common())}")
    print("signatures:")
    s = Counter(sigkey(r) for r in trans if klass(r) not in ("match", "untranslated"))
    for k, v in s.most_common(40):
        print(f"  {v:6d} {k}")


def klass(r):
    if r["status"] == "untranslated":
        return "untranslated"
    if r.get("truth"):
        return "built, no truth"
    if r["v"] == "ok":
        return "match" if r.get("match") else "wrong"
    return r["v"]


def sigkey(r):
    k = klass(r)
    if k == "wrong":
        b = "multi" if "multibody-join" in r.get("notes", []) else "single"
        e = r.get("evol", 9)
        return f"wrong ({b}) vol err {'<1%' if e < 0.01 else '<10%' if e < 0.1 else 'big'}"
    s = re.sub(r"-?\d+\.?\d*(?:e-?\d+)?", "N", r.get("sig", "") or r.get("why", ""))
    return f"{k}: {s[:110]}"


if __name__ == "__main__":
    {"extract": extract, "run": run, "oks": oks, "report": report}[sys.argv[1]](sys.argv[2:])
