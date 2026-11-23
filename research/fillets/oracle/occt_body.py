"""occt_body.py DIR: label_body's DUMP directory (NAME.pts, NAME.before.step, NAME.after.step)
held against OCCT. A counted disagreement stands only when OCCT agrees with forge-bool's Index on
both bodies at that point: inside the result for `over` (outside for `under`), and on the same side
of the body before as Index put it (the spec's base side). One JSON line per fillet: NAME, the
points, how many stand, and how many fell on each witness."""
import json, os, sys
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.TopAbs import TopAbs_IN, TopAbs_ON
from OCP.gp import gp_Pnt


def load(p):
    if not os.path.exists(p):
        return None
    r = STEPControl_Reader()
    if r.ReadFile(p) != 1:
        return None
    r.TransferRoots()
    return BRepClass3d_SolidClassifier(r.OneShape())


def side(c, p):
    c.Perform(p, 1e-7)
    s = c.State()
    return None if s == TopAbs_ON else s == TopAbs_IN


d = sys.argv[1]
for f in sorted(os.listdir(d)):
    if not f.endswith(".pts"):
        continue
    name = f[:-4]
    ca, cb = load(f"{d}/{name}.after.step"), load(f"{d}/{name}.before.step")
    row = {"name": name, "points": 0, "stand": 0, "after_differs": 0, "before_differs": 0, "on": 0, "no_step": ca is None or cb is None}
    for line in open(f"{d}/{f}"):
        w = line.split()
        if len(w) < 5:
            continue
        row["points"] += 1
        if ca is None or cb is None:
            continue
        p = gp_Pnt(*map(float, w[:3]))
        a, b = side(ca, p), side(cb, p)
        if a is None or b is None:
            row["on"] += 1
            continue
        ok_a = a == (w[3] == "over")
        ok_b = b == (w[4] == "1")
        row["after_differs"] += not ok_a
        row["before_differs"] += not ok_b
        row["stand"] += ok_a and ok_b
    print(json.dumps(row), flush=True)
