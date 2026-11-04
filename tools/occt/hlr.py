#!/usr/bin/env python3
"""OpenCascade's exact hidden-line removal (HLRBRep_Algo) on a STEP file, against ForgeCAD's
(`fg view FILE --points`): projected length by visibility and class, and how much of each side's
lines lies on the other's.

  $VENV/bin/python -I hlr.py PART.step OURS.json [--tol 0.01]

OURS.json is `fg view` output with --points (same views). Sharp and outline lines are compared;
smooth (tangent) edges and seams are not drawn by either. OCCT keeps hidden lines lying under seen
ones and second copies of a line; drawing practice (and ours) leaves both out, so OCCT's hidden
lines are compared net of them (`hidden_occt_net`). Prints one JSON line per view: lengths, and the
share of each side's lines within --tol of the other's.
"""
import argparse
import json
import math
import sys


VIEWS = {
    "front": ((0, -1, 0), (0, 0, 1)),
    "back": ((0, 1, 0), (0, 0, 1)),
    "right": ((1, 0, 0), (0, 0, 1)),
    "left": ((-1, 0, 0), (0, 0, 1)),
    "top": ((0, 0, 1), (0, 1, 0)),
    "bottom": ((0, 0, -1), (0, -1, 0)),
    "iso": ((1, -1, 1), (0, 0, 1)),
    "iso_left": ((-1, -1, 1), (0, 0, 1)),
}


def norm(v):
    n = math.sqrt(sum(x * x for x in v))
    return tuple(x / n for x in v)


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def read_step(path):
    from OCP.IFSelect import IFSelect_RetDone
    from OCP.STEPControl import STEPControl_Reader

    r = STEPControl_Reader()
    if r.ReadFile(path) != IFSelect_RetDone:
        sys.exit(f"{path}: not read")
    r.TransferRoots()
    return r.OneShape()


def occt_lines(shape, toward, up, chord):
    from OCP.BRepAdaptor import BRepAdaptor_Curve
    from OCP.GCPnts import GCPnts_QuasiUniformDeflection
    from OCP.gp import gp_Ax2, gp_Dir, gp_Pnt
    from OCP.HLRAlgo import HLRAlgo_Projector
    from OCP.HLRBRep import HLRBRep_Algo, HLRBRep_HLRToShape
    from OCP.TopAbs import TopAbs_EDGE
    from OCP.TopExp import TopExp_Explorer
    from OCP.TopoDS import TopoDS

    t = norm(toward)
    right = norm(cross(up, t))
    ax = gp_Ax2(gp_Pnt(0, 0, 0), gp_Dir(*t), gp_Dir(*right))
    algo = HLRBRep_Algo()
    algo.Add(shape)
    algo.Projector(HLRAlgo_Projector(ax))
    algo.Update()
    algo.Hide()
    shapes = HLRBRep_HLRToShape(algo)
    out = {}
    for key, fn in [
        ("seen_sharp", shapes.VCompound),
        ("seen_outline", shapes.OutLineVCompound),
        ("hidden_sharp", shapes.HCompound),
        ("hidden_outline", shapes.OutLineHCompound),
    ]:
        comp = fn()
        lines = []
        if not comp.IsNull():
            ex = TopExp_Explorer(comp, TopAbs_EDGE)
            while ex.More():
                c = BRepAdaptor_Curve(TopoDS.Edge(ex.Current()))
                d = GCPnts_QuasiUniformDeflection(c, chord)
                if d.IsDone():
                    pts = [d.Value(i + 1) for i in range(d.NbPoints())]
                    lines.append([(p.X(), p.Y()) for p in pts])
                ex.Next()
        out[key] = lines
    return out


def length(lines):
    return sum(math.dist(a, b) for l in lines for a, b in zip(l, l[1:]))


class Near:
    """Segments binned on a grid: how far a point is from the nearest."""

    def __init__(self, lines, cell):
        self.cell = cell
        self.bins = {}
        for l in lines:
            for a, b in zip(l, l[1:]):
                x0, x1 = sorted((a[0], b[0]))
                y0, y1 = sorted((a[1], b[1]))
                for i in range(int(math.floor(x0 / cell)), int(math.floor(x1 / cell)) + 1):
                    for j in range(int(math.floor(y0 / cell)), int(math.floor(y1 / cell)) + 1):
                        self.bins.setdefault((i, j), []).append((a, b))

    def dist(self, p):
        i, j = int(math.floor(p[0] / self.cell)), int(math.floor(p[1] / self.cell))
        best = math.inf
        for di in (-1, 0, 1):
            for dj in (-1, 0, 1):
                for a, b in self.bins.get((i + di, j + dj), ()):
                    ab = (b[0] - a[0], b[1] - a[1])
                    l2 = ab[0] ** 2 + ab[1] ** 2
                    t = 0 if l2 == 0 else max(0, min(1, ((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / l2))
                    q = (a[0] + t * ab[0], a[1] + t * ab[1])
                    best = min(best, math.dist(p, q))
        return best


def points(lines, step=0.05):
    """Points along `lines` about step apart, each with the length it stands for."""
    for l in lines:
        for a, b in zip(l, l[1:]):
            n = max(1, int(math.dist(a, b) / step))
            seg = math.dist(a, b) / n
            for k in range(n):
                t = (k + 0.5) / n
                yield (a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])), seg


def covered(pts, near, tol, misses=None):
    """The length of the points within tol of `near`'s lines, and their whole length; the points
    off them into `misses`."""
    on = total = 0.0
    for p, seg in pts:
        total += seg
        if near.dist(p) <= tol:
            on += seg
        elif misses is not None:
            misses.append(p)
    return on, total


def net_hidden(hidden, seen, tol, step=0.05):
    """OCCT's hidden lines less what lies on its seen ones (drawing practice: seen wins) and less
    second copies of the same hidden line, as points."""
    near = Near(seen, 2.0)
    taken = set()
    out = []
    for p, seg in points(hidden, step):
        if near.dist(p) <= tol:
            continue
        cell = (round(p[0] / tol), round(p[1] / tol))
        if cell in taken:
            continue
        taken.add(cell)
        out.append((p, seg))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("step")
    ap.add_argument("ours")
    ap.add_argument("--tol", type=float, default=0.01)
    ap.add_argument("--chord", type=float, default=0.002)
    ap.add_argument("--misses", action="store_true", help="where each side's lines are off the other's")
    a = ap.parse_args()
    shape = read_step(a.step)
    ours = json.load(open(a.ours))
    for v in ours:
        toward, up = VIEWS[v["view"]]
        occ = occt_lines(shape, toward, up, a.chord)
        mine = {"seen": [], "hidden": []}
        for p in v["polylines"]:
            if p["class"] == "tangent":
                continue
            mine["seen" if p["visible"] else "hidden"].append([tuple(q) for q in p["points"]])
        row = {"view": v["view"]}
        seen = occ["seen_sharp"] + occ["seen_outline"]
        hidden = occ["hidden_sharp"] + occ["hidden_outline"]
        for side in ("seen", "hidden"):
            o = seen if side == "seen" else hidden
            row[f"{side}_occt"] = round(length(o), 3)
            row[f"{side}_ours"] = round(length(mine[side]), 3)
            opts = list(points(o)) if side == "seen" else net_hidden(hidden, seen, a.tol)
            if side == "hidden":
                row["hidden_occt_net"] = round(sum(s for _, s in opts), 3)
            m1, m2 = [], []
            on, tot = covered(opts, Near(mine[side], 2.0), a.tol, m1)
            row[f"{side}_occt_on_ours"] = round(on / tot, 5) if tot else 1.0
            on, tot = covered(list(points(mine[side])), Near(o, 2.0), a.tol, m2)
            row[f"{side}_ours_on_occt"] = round(on / tot, 5) if tot else 1.0
            if a.misses:
                row[f"{side}_occt_only"] = [[round(x, 2) for x in p] for p in m1[:: max(1, len(m1) // 12)]]
                row[f"{side}_ours_only"] = [[round(x, 2) for x in p] for p in m2[:: max(1, len(m2) // 12)]]
        print(json.dumps(row))


if __name__ == "__main__":
    main()
