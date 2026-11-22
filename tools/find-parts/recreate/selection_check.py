#!/usr/bin/env python3
"""Were the edges we gave a refused fillet or chamfer the right ones? Asked of the source CAD's own result, with no
second kernel: an edge really blended is gone from the dataset's STEP, and one left alone is still on it.

  $VENV/bin/python -I selection_check.py ROWS.jsonl PARTS_DIR --topo EXE --step STEP.zip --out OUT.jsonl

PARTS_DIR holds the .fg parts; a row's `line` is the judge's, the blend's statement is read back and its
selection asked of the part before it (forge-proof's topo).

picked_kept: picked edges whose middle still lies on the truth: picked too much (features after this one could not
put a sharp edge back there, so this holds at any stage).
unpicked_gone: unpicked edges whose middle is off the truth: picked too little, read only when nothing follows.
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

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fgw

ON = 1e-3
MAX_EDGES = 400


def child(path, job):
    resource.setrlimit(resource.RLIMIT_AS, (3 << 30, 3 << 30))
    resource.setrlimit(resource.RLIMIT_CPU, (300, 301))
    from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeVertex
    from OCP.BRepExtrema import BRepExtrema_DistShapeShape
    from OCP.gp import gp_Pnt
    from OCP.STEPControl import STEPControl_Reader

    r = STEPControl_Reader()
    r.ReadFile(path)
    r.TransferRoots()
    s = r.OneShape()
    out = []
    for p in json.load(open(job)):
        v = BRepBuilderAPI_MakeVertex(gp_Pnt(*p)).Vertex()
        d = BRepExtrema_DistShapeShape(v, s)
        d.Perform()
        out.append(d.Value())
    print(json.dumps(out))


def main():
    a = sys.argv[1:]
    if a[0] == "--child":
        return child(a[1], a[2])
    opt = lambda k: a[a.index(k) + 1]
    rows = [json.loads(l) for l in open(a[0])]
    z = zipfile.ZipFile(opt("--step"))
    lock = threading.Lock()
    base = os.path.dirname(os.path.abspath(opt("--out")))

    def one(r):
        i, L = r["id"], r.get("line")
        out = {"id": i, "sig": r["sig"], "line": L, "features": r["features"]}
        src = open(os.path.join(a[1], i + ".fg")).read()
        sts = fgw.statements(src)
        k = None if L is None else fgw.at_line(src, L)
        if k is None:
            return dict(out, cls="X", why="no failing line")
        b = fgw.blend(sts[k][1])
        if b is None or b[0] == "shell":
            return dict(out, cls="X", why="fails at " + sts[k][1].split("(")[0][-12:])
        out["kind"] = b[0]
        prefix = "\n".join(t for _, t in sts[:k]) + "\n"
        tp = fgw.Topo(opt("--topo"))
        try:
            st = tp.state(prefix)
            # the edges the blend's selection names on the part before it
            hs = set(tp.names(prefix, b[2]))
        except (fgw.Stop, IndexError, ValueError):
            return dict(out, cls="X", why="prefix did not rebuild")
        finally:
            tp.close()
        es = st.get("edges", [])
        if not hs or len(es) > MAX_EDGES:
            return dict(out, cls="X", why="no edges named, or too many edges")
        pts = [e["p"][8] for e in es]
        with tempfile.TemporaryDirectory(dir=base) as tmp:
            with lock:
                data = z.read(f"step/{i[:4]}/{i}.step")
            open(os.path.join(tmp, "a.step"), "wb").write(data)
            json.dump(pts, open(os.path.join(tmp, "p.json"), "w"))
            try:
                c = subprocess.run([sys.executable, "-I", os.path.abspath(__file__), "--child", os.path.join(tmp, "a.step"), os.path.join(tmp, "p.json")], capture_output=True, text=True, timeout=900)
                ds = json.loads(c.stdout.strip().splitlines()[-1])
            except (subprocess.TimeoutExpired, IndexError, ValueError):
                return dict(out, cls="X", why="truth would not measure")
        sharp = [(e, d) for e, d in zip(es, ds) if e["c"] != "smooth"]
        picked = [d for e, d in sharp if e["n"] in hs]
        rest = [d for e, d in sharp if e["n"] not in hs]
        out.update(npicked=len(picked), picked_kept=sum(d <= ON for d in picked), nrest=len(rest), unpicked_gone=sum(d > ON for d in rest), last=k == len(sts) - 1 and r["status"] == "translated")
        return dict(out, cls="checked")

    with open(opt("--out"), "w") as f, cf.ThreadPoolExecutor(4) as ex:
        for res in ex.map(one, rows):
            f.write(json.dumps(res) + "\n")
            f.flush()


if __name__ == "__main__":
    main()
