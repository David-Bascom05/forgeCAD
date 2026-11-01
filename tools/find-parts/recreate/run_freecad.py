#!/usr/bin/env python3
"""Sweep: every PartDesign body in the FreeCAD library translated, judged and held against its stored shape.

  run_freecad.py run FILES.txt --files DIR --out RUN --judge EXE --topo EXE --venv PY [--jobs 4]
  run_freecad.py report RUN
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


def one(h, a):
    out, parts = a["out"], os.path.join(a["out"], "parts")
    try:
        p = subprocess.run([sys.executable, "-I", os.path.join(HERE, "freecad.py"), os.path.join(a["files"], h + ".FCStd"), "--topo", a["topo"], "--truth", a["venv"], "--out", parts, "--tmp", os.path.join(out, "tmp")], capture_output=True, text=True, timeout=1800)
    except subprocess.TimeoutExpired:
        return [{"file": h, "body": "", "status": "untranslated", "cls": "importer", "why": "translation timed out"}]
    rows = [json.loads(l) for l in p.stdout.splitlines() if l.startswith("{")]
    if not rows and p.returncode != 0:
        rows = [{"file": h, "body": "", "status": "untranslated", "cls": "importer", "why": "importer died: " + p.stderr[-70:]}]
    for r in rows:
        if r["status"] == "untranslated":
            continue
        j = R.judge(a["judge"], os.path.join(parts, f"{h}.{r['body']}.fc"))
        if j["v"] == "fail" and j["sig"].startswith("nary: the part is"):
            j = dict(j, v="ok", sig="", multi=True)
        r["v"], r["sig"] = j["v"], j["sig"]
        for k in ("vol", "area", "bbox", "bboxu", "bodies", "faces", "line"):
            if k in j:
                r[k] = j[k]
    return rows


def run(a):
    opt = lambda k, d=None: a[a.index(k) + 1] if k in a else d
    out = opt("--out")
    os.makedirs(os.path.join(out, "parts"), exist_ok=True)
    os.makedirs(os.path.join(out, "tmp"), exist_ok=True)
    args = {"files": opt("--files"), "out": out, "judge": os.path.abspath(opt("--judge")), "topo": os.path.abspath(opt("--topo")), "venv": opt("--venv")}
    path = os.path.join(out, "results.jsonl")
    done = {json.loads(l)["file"] for l in open(path)} if os.path.exists(path) else set()
    todo = [h for h in open(a[0]).read().split() if h not in done]
    with open(path, "a") as log, cf.ThreadPoolExecutor(min(int(opt("--jobs", "4")), 4)) as ex:
        for k, rows in enumerate(ex.map(lambda h: one(h, args), todo)):
            for r in rows:
                log.write(json.dumps(r) + "\n")
            log.flush()
            if k % 50 == 49:
                print(k + 1, "of", len(todo), flush=True)


def klass(r):
    if r["status"] == "untranslated":
        return "untranslated"
    if r["v"] != "ok":
        return r["v"]
    t = r.get("truth") or {}
    if not t.get("solids"):
        return "built, no truth"
    return "match" if R.compare(r, t, r.get("notes", []))["match"] else "wrong"


def report(a):
    rows = [json.loads(l) for l in open(os.path.join(a[0], "results.jsonl"))]
    C = collections.Counter
    print("bodies", len(rows), dict(C(klass(r) for r in rows).most_common()))
    print("why untranslated:")
    for k, v in C((r.get("cls"), r["why"]) for r in rows if r["status"] == "untranslated").most_common(25):
        print(f"  {v:5d} {k[0]}: {k[1]}")
    print("by feature kind:")
    for k in sorted({x for r in rows for x in r.get("features", [])}):
        rs = [r for r in rows if k in r.get("features", [])]
        print(f"  {k:18s} bodies {len(rs):5d} {dict(C(klass(r) for r in rs).most_common())}")
    print("by feature count:")
    for lo, hi in ((1, 1), (2, 3), (4, 7), (8, 15), (16, 10**6)):
        rs = [r for r in rows if r["status"] != "untranslated" and lo <= len(r.get("features", [])) <= hi]
        print(f"  {lo}-{hi}: {len(rs)} {dict(C(klass(r) for r in rs).most_common())}")
    print("signatures:")
    s = C(klass(r) + ": " + re.sub(r"-?\d+\.?\d*(?:e-?\d+)?", "N", r.get("sig") or r.get("why") or "")[:100] for r in rows if klass(r) not in ("match", "untranslated"))
    for k, v in s.most_common(30):
        print(f"  {v:5d} {k}")


if __name__ == "__main__":
    {"run": run, "report": report}[sys.argv[1]](sys.argv[2:])
