#!/usr/bin/env python3
"""Second opinion on parts our kernel refused at a fillet or chamfer: OpenCascade replays the same history with
the very edges we picked, and its result is held against the dataset's STEP. That splits our real gaps from the
importer's selection errors.

  $VENV/bin/python -I occt_second.py RESULTS.jsonl PARTS_DIR --export PARTS.json --topo EXE --step STEP.zip --out OUT.jsonl

A: OCCT builds it, valid, and it matches the truth: the edges were right; our kernel's gap.
B: OCCT builds it, valid, and it does not match: a selection or an earlier step is wrong (importer).
C: OCCT builds it, valid, but features follow it, so no truth at this stage: probably our gap, unconfirmed.
D: OCCT refuses it too: OCCT's limit, or a bad selection; not decided here.
"""
import concurrent.futures as cf
import json
import os
import resource
import subprocess
import sys
import tempfile
import threading
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "..", "..", "occt"))
TOL = 1e-3


def child(job):
    resource.setrlimit(resource.RLIMIT_AS, (3 << 30, 3 << 30))
    resource.setrlimit(resource.RLIMIT_CPU, (120, 121))
    from OCP.BRepCheck import BRepCheck_Analyzer
    from OCP.BRepGProp import BRepGProp
    from OCP.GProp import GProp_GProps
    from OCP.Bnd import Bnd_Box
    from OCP.BRepBndLib import BRepBndLib
    from build import Opts, build

    d = json.load(open(job))
    s = build(d["features"], Opts(False, 0.0))
    g = GProp_GProps()
    BRepGProp.VolumeProperties_s(s, g)
    v = abs(g.Mass())
    g = GProp_GProps()
    BRepGProp.SurfaceProperties_s(s, g)
    box = Bnd_Box()
    BRepBndLib.AddOptimal_s(s, box, True, False)
    a, b = box.CornerMin(), box.CornerMax()
    print(json.dumps({"vol": v, "area": g.Mass(), "valid": bool(BRepCheck_Analyzer(s).IsValid()), "bbox": [a.X(), a.Y(), a.Z(), b.X(), b.Y(), b.Z()]}))


def edges_of(topo, lines):
    src = ("\n".join(lines) + "\n").encode()
    p = subprocess.run([topo], input=b"%d\n" % len(src) + src, capture_output=True, timeout=600)
    d = json.loads(p.stdout.splitlines()[0])
    if d.get("err") or "panic" in d:
        raise ValueError("the prefix does not build")
    return {e["h"]: e for e in d.get("edges", [])}


def main():
    a = sys.argv[1:]
    if a[0] == "--child":
        return child(a[1])
    opt = lambda k: a[a.index(k) + 1]
    rows = [json.loads(l) for l in open(a[0])]
    parts = {p["name"]: p for p in json.load(open(opt("--export")))["parts"]}
    z = zipfile.ZipFile(opt("--step"))
    lock = threading.Lock()
    tmp = tempfile.mkdtemp(prefix="second-", dir=os.path.dirname(os.path.abspath(opt("--out"))))
    import step_truth

    def one(r):
        i = r["id"]
        out = {"id": i, "sig": r["sig"], "line": r.get("line"), "features": r["features"]}
        lines = open(os.path.join(a[1], i + ".fc")).read().splitlines()
        L = r.get("line")
        if L is None or i not in parts or L >= len(lines):
            return dict(out, cls="X", why="not exported")
        words = lines[L].split()
        k = 1 if words[0].endswith(":") else 0
        kind = words[k]
        if kind not in ("fillet", "chamfer"):
            return dict(out, cls="X", why="fails at " + kind)
        size = float(words[k + 1])
        out["kind"] = kind
        try:
            es = edges_of(opt("--topo"), lines[:L])
            hs = [w for w in words[k + 2:] if w.startswith("@")]
            picks = [[q for q in es[h]["p"][1:-1:3]] for h in hs]
        except (KeyError, ValueError, subprocess.TimeoutExpired, IndexError) as x:
            return dict(out, cls="X", why="prefix: " + repr(x)[:60])
        feats = parts[i]["features"][:L]
        if len(feats) != L:
            return dict(out, cls="X", why=f"exported {len(feats)} of {L} lines")
        if kind == "fillet":
            feats = feats + [{"i": L, "kind": "fillet", "edges": [{"pts": p, "law": [[q, size] for q in p]} for p in picks]}]
        else:
            feats = feats + [{"i": L, "kind": "chamfer", "edges": picks, "d": [size]}]
        job = os.path.join(tmp, i + ".json")
        json.dump({"features": feats}, open(job, "w"))
        try:
            p = subprocess.run([sys.executable, "-I", os.path.abspath(__file__), "--child", job], capture_output=True, text=True, timeout=900)
        except subprocess.TimeoutExpired:
            return dict(out, cls="D", why="OCCT timed out")
        finally:
            pass
        os.remove(job)
        if p.returncode != 0 or not p.stdout.strip():
            return dict(out, cls="D", why=(p.stderr.strip().splitlines() or ["?"])[-1][:80])
        o = json.loads(p.stdout.strip().splitlines()[-1])
        out["occt"] = o
        if not o["valid"]:
            return dict(out, cls="D", why="OCCT's result is invalid")
        if L != len(lines) - 1 or r["status"] != "translated":
            return dict(out, cls="C", why="features follow it")
        st = os.path.join(tmp, i + ".step")
        with lock:
            data = z.read(f"step/{i[:4]}/{i}.step")
        open(st, "wb").write(data)
        t = step_truth.measure(st)
        os.remove(st)
        out["truth"] = t
        ev = abs(o["vol"] - t.get("vol", 0)) / max(t.get("vol", 0), 1e-12)
        ea = abs(o["area"] - t.get("area", 0)) / max(t.get("area", 0), 1e-12)
        out["evol"], out["earea"] = ev, ea
        return dict(out, cls="A" if ev <= TOL and ea <= TOL else "B", why="")

    with open(opt("--out"), "w") as f, cf.ThreadPoolExecutor(4) as ex:
        for res in ex.map(one, rows):
            f.write(json.dumps(res) + "\n")
            f.flush()
    os.rmdir(tmp)


if __name__ == "__main__":
    main()
