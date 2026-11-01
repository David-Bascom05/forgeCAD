#!/usr/bin/env python3
"""Sweep: WHUCAD parts translated, judged and held against their STEP. The STEP was built from the same quantised
values, so the comparison is strict.

  run_whucad.py run IDS --vec DIR --step DIR --out RUN --judge EXE --topo EXE --h5py PY --occt PY [--jobs 4]
  run_whucad.py report RUN
"""
import collections
import concurrent.futures as cf
import json
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import run_cadfs as R


def one(i, a):
    fc = os.path.join(a["out"], "parts", i + ".fc")
    h5 = a["index"].get(i)
    if h5 is None:
        return {"id": i, "status": "untranslated", "cls": "importer", "why": "no vector file"}
    try:
        p = subprocess.run([a["h5py"], "-I", os.path.join(HERE, "whucad.py"), h5, "--topo", a["topo"], "--out", fc], capture_output=True, text=True, timeout=300)
        r = json.loads(p.stdout)
    except subprocess.TimeoutExpired:
        r = {"status": "untranslated", "cls": "importer", "why": "translation timed out", "notes": []}
    except ValueError:
        r = {"status": "untranslated", "cls": "importer", "why": "importer died: " + p.stderr[-70:], "notes": []}
    r["id"] = i
    if r["status"] == "untranslated":
        return r
    j = R.judge(a["judge"], fc)
    if j["v"] == "fail" and j["sig"].startswith("nary: the part is"):
        j = dict(j, v="ok", sig="", multi=True)
    r["v"], r["sig"] = j["v"], j["sig"]
    for k in ("vol", "area", "bbox", "bboxu", "bodies", "faces", "line"):
        if k in j:
            r[k] = j[k]
    st = os.path.join(a["step"], i + ".stp")
    if r["v"] == "ok" and os.path.exists(st):
        c = subprocess.run([a["occt"], "-I", "-c", f"import sys;sys.path.insert(0,{HERE!r});import step_truth,json;print(json.dumps(step_truth.measure({st!r})))"], capture_output=True, text=True, timeout=300)
        try:
            r["truth"] = json.loads(c.stdout.strip().splitlines()[-1])
        except (ValueError, IndexError):
            pass
    return r


def run(a):
    opt = lambda k, d=None: a[a.index(k) + 1] if k in a else d
    out = opt("--out")
    os.makedirs(os.path.join(out, "parts"), exist_ok=True)
    args = {k: opt("--" + k) for k in ("vec", "step", "out", "judge", "topo", "h5py", "occt")}
    args["index"] = {f[:-3]: os.path.join(r, f) for r, _, fs in os.walk(args["vec"]) for f in fs if f.endswith(".h5")}
    path = os.path.join(out, "results.jsonl")
    done = {json.loads(l)["id"] for l in open(path)} if os.path.exists(path) else set()
    todo = [i for i in open(a[0]).read().split() if i not in done]
    with open(path, "a") as log, cf.ThreadPoolExecutor(min(int(opt("--jobs", "4")), 4)) as ex:
        for r in ex.map(lambda i: one(i, args), todo):
            log.write(json.dumps(r) + "\n")
            log.flush()


def klass(r):
    if r["status"] == "untranslated":
        return "untranslated"
    if r["v"] != "ok":
        return r["v"]
    t = r.get("truth") or {}
    if not t.get("solids"):
        return "built, no truth"
    return "match" if R.compare(r, t, [])["match"] else "wrong"


def report(a):
    rows = [json.loads(l) for l in open(os.path.join(a[0], "results.jsonl"))]
    C = collections.Counter
    print("parts", len(rows), dict(C(klass(r) for r in rows).most_common()))
    for k, v in C((r.get("cls"), r["why"]) for r in rows if r["status"] == "untranslated").most_common(20):
        print(f"  {v:5d} {k[0]}: {k[1]}")
    for k in sorted({x for r in rows for x in r.get("features", [])}):
        rs = [r for r in rows if k in r.get("features", [])]
        print(f"  {k:10s} parts {len(rs):5d} {dict(C(klass(r) for r in rs).most_common())}")
    s = C(klass(r) + ": " + re.sub(r"-?\d+\.?\d*(?:e-?\d+)?", "N", r.get("sig") or "")[:90] for r in rows if klass(r) not in ("match", "untranslated"))
    for k, v in s.most_common(25):
        print(f"  {v:5d} {k}")


if __name__ == "__main__":
    {"run": run, "report": report}[sys.argv[1]](sys.argv[2:])
