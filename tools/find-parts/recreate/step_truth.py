#!/usr/bin/env python3
"""Each STEP file's volume, area, bounding box and solid count by OpenCascade, one child per file.

  $VENV/bin/python -I step_truth.py STEP.zip --ids IDS.txt --out TRUTH.jsonl [--jobs 4] [--timeout 120]

The zip is read as data and never extracted whole. Files already in TRUTH.jsonl are skipped.
Sheet and wire bodies are counted, not measured: a truth is about solids.
"""
import concurrent.futures as cf
import json
import os
import resource
import subprocess
import sys
import tempfile
import zipfile

MEM = 3 << 30


def measure(path):
    from OCP.Bnd import Bnd_Box
    from OCP.BRepBndLib import BRepBndLib
    from OCP.BRepCheck import BRepCheck_Analyzer
    from OCP.BRepGProp import BRepGProp
    from OCP.GProp import GProp_GProps
    from OCP.IFSelect import IFSelect_RetDone
    from OCP.STEPControl import STEPControl_Reader
    from OCP.TopAbs import TopAbs_FACE, TopAbs_SHELL, TopAbs_SOLID
    from OCP.TopExp import TopExp_Explorer
    from OCP.TopoDS import TopoDS

    r = STEPControl_Reader()
    if r.ReadFile(path) != IFSelect_RetDone:
        return {"err": "unreadable"}
    r.TransferRoots()
    s = r.OneShape()
    vol = area = 0.0
    n = 0
    valid = True
    box = Bnd_Box()
    x = TopExp_Explorer(s, TopAbs_SOLID)
    while x.More():
        so = TopoDS.Solid_s(x.Current())
        g = GProp_GProps()
        BRepGProp.VolumeProperties_s(so, g)
        vol += abs(g.Mass())
        g = GProp_GProps()
        BRepGProp.SurfaceProperties_s(so, g)
        area += g.Mass()
        BRepBndLib.AddOptimal_s(so, box, True, False)
        valid = valid and BRepCheck_Analyzer(so).IsValid()
        n += 1
        x.Next()
    faces = 0
    x = TopExp_Explorer(s, TopAbs_FACE)
    while x.More():
        faces += 1
        x.Next()
    out = {"solids": n, "vol": vol, "area": area, "faces": faces, "valid": valid}
    if n:
        out["bbox"] = list(box.Get())
    return out


def child(path):
    resource.setrlimit(resource.RLIMIT_AS, (MEM, MEM))
    print(json.dumps(measure(path)))


def main():
    a = sys.argv[1:]
    if a[0] == "--child":
        return child(a[1])
    opt = lambda k, d=None: a[a.index(k) + 1] if k in a else d
    ids = open(opt("--ids")).read().split()
    out = opt("--out")
    done = set()
    if os.path.exists(out):
        done = {json.loads(l)["id"] for l in open(out)}
    z = zipfile.ZipFile(a[0])
    names = {os.path.splitext(os.path.basename(n))[0]: n for n in z.namelist() if not n.endswith("/")}
    timeout = float(opt("--timeout", "120"))
    log = open(out, "a")
    tmp = tempfile.mkdtemp(prefix="truth-", dir=opt("--tmp"))

    def one(i):
        if i not in names:
            return {"id": i, "err": "no step"}
        p = os.path.join(tmp, i + ".step")
        with open(p, "wb") as f:
            f.write(z.read(names[i]))
        try:
            c = subprocess.run([sys.executable, "-I", os.path.abspath(__file__), "--child", p], capture_output=True, text=True, timeout=timeout)
            r = json.loads(c.stdout.strip().splitlines()[-1]) if c.returncode == 0 and c.stdout.strip() else {"err": f"exit {c.returncode}"}
        except subprocess.TimeoutExpired:
            r = {"err": "timeout"}
        except ValueError:
            r = {"err": "bad output"}
        os.remove(p)
        return dict(r, id=i)

    jobs = min(int(opt("--jobs", "4")), 4)
    todo = [i for i in ids if i not in done]
    # z.read is not shared across threads without its lock: ZipFile holds one
    with cf.ThreadPoolExecutor(jobs) as ex:
        for k, r in enumerate(ex.map(one, todo)):
            log.write(json.dumps(r) + "\n")
            log.flush()
            if k % 1000 == 999:
                print(k + 1, "of", len(todo), flush=True)
    os.rmdir(tmp)


if __name__ == "__main__":
    main()
