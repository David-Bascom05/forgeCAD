"""occt_refillet.py DIR LABELS.jsonl: for each label_body dump NAME.before.step / NAME.after.step
whose fillet is in LABELS, OCCT's own fillet of the body before: the edges it rounds are those of
the body before that no longer lie on the boundary of the body after (midpoint farther than 1e-5 from it), at the
label's radius. One JSON line each: NAME, edges picked against the label's count, our volume,
OCCT's, OCCT's validity, and the largest distance of the dump's counted points from OCCT's result
together with how many OCCT puts on the oracle's side. Run with the OCCT venv."""
import json, os, sys
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepFilletAPI import BRepFilletAPI_MakeFillet
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_EDGE, TopAbs_IN, TopAbs_OUT
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
from OCP.BRepExtrema import BRepExtrema_DistShapeShape
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeVertex
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.gp import gp_Pnt


def load(p):
    r = STEPControl_Reader()
    if r.ReadFile(p) != 1:
        return None
    r.TransferRoots()
    return r.OneShape()


def vol(s):
    g = GProp_GProps()
    BRepGProp.VolumeProperties_s(s, g)
    return g.Mass()


def faces(s):
    """The shape's faces as a compound: a distance to a solid is 0 inside it."""
    from OCP.TopAbs import TopAbs_FACE
    from OCP.BRep import BRep_Builder
    from OCP.TopoDS import TopoDS_Compound
    c, b = TopoDS_Compound(), BRep_Builder()
    b.MakeCompound(c)
    x = TopExp_Explorer(s, TopAbs_FACE)
    while x.More():
        b.Add(c, x.Current())
        x.Next()
    return c


def dist(shape, p):
    d = BRepExtrema_DistShapeShape(shape, BRepBuilderAPI_MakeVertex(p).Vertex())
    return d.Value() if d.IsDone() else float("nan")


def main():
    d, labels = sys.argv[1], sys.argv[2]
    want = {}
    for l in open(labels):
        r = json.loads(l)
        if r["label"] == "wrong":
            want[f"{r['id']}.{r['feature']}"] = r
    for name in sorted(want):
        lab = want[name]
        out = {"name": name, "r": lab["r"], "rounded": lab["rounded"]}
        b, a = load(os.path.join(d, name + ".before.step")), load(os.path.join(d, name + ".after.step"))
        if b is None or a is None:
            print(json.dumps(dict(out, err="no step")), flush=True)
            continue
        mk = BRepFilletAPI_MakeFillet(b)
        af = faces(a)
        n = 0
        x = TopExp_Explorer(b, TopAbs_EDGE)
        seen = []
        while x.More():
            e = TopoDS.Edge(x.Current())
            x.Next()
            c = BRepAdaptor_Curve(e)
            m = c.Value(0.5 * (c.FirstParameter() + c.LastParameter()))
            if any(m.Distance(q) < 1e-9 for q in seen):
                continue
            seen.append(m)
            if dist(af, m) > 1e-5:
                mk.Add(lab["r"], e)
                n += 1
        out.update(edges=n, ours=vol(a))
        try:
            mk.Build()
            ok = mk.IsDone()
        except Exception as ex:
            ok = False
            out["occt_err"] = str(ex)[:80]
        if not ok:
            print(json.dumps(dict(out, occt="refused")), flush=True)
            continue
        s = mk.Shape()
        out.update(occt=vol(s), valid=BRepCheck_Analyzer(s).IsValid())
        out["rel"] = (out["ours"] - out["occt"]) / out["occt"]
        cl = BRepClass3d_SolidClassifier(s)
        agree, far = 0, 0.0
        pts = [l.split() for l in open(os.path.join(d, name + ".pts")) if l.strip()]
        for t in pts:
            p = gp_Pnt(*map(float, t[:3]))
            cl.Perform(p, 1e-7)
            st = cl.State()
            # over: ours holds material the oracle removes; OCCT on the oracle's side means OUT
            if (t[3] == "over" and st == TopAbs_OUT) or (t[3] == "under" and st == TopAbs_IN):
                agree += 1
        out.update(points=len(pts), occt_with_oracle=agree)
        print(json.dumps(out), flush=True)


main()
