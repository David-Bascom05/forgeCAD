"""score_occ.py DIR: the decisive points of fillet-spec score, classified by OCCT against the kernel's
result (ID.kernel.step) and the truth (ID.truth.step). One JSON line per part."""
import sys, json, glob, random
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.TopAbs import TopAbs_IN, TopAbs_OUT
from OCP.gp import gp_Pnt
def load(p):
    r = STEPControl_Reader()
    if r.ReadFile(p) != 1: return None
    r.TransferRoots(); return r.OneShape()
d = sys.argv[1]
for f in sorted(glob.glob(f"{d}/*.pts")):
    i = f.split('/')[-1][:8]
    pts = [json.loads(l) for l in open(f)]
    conc = [p for p in pts if not p['convex'] and p['before'] == 'out' and p['covered']]
    conv = [p for p in pts if p['convex'] and p['pokes'] and p['before'] == 'in']
    if not conc and not conv: continue
    random.seed(1); conc = random.sample(conc, min(150, len(conc))); conv = random.sample(conv, min(150, len(conv)))
    truth = load(f"{d}/{i}.truth.step")
    try: kern = load(f"{d}/{i}.kernel.step")
    except Exception: kern = None
    def st(s, p):
        c = BRepClass3d_SolidClassifier(s, gp_Pnt(p['x'], p['y'], p['z']), 1e-6).State()
        return 'in' if c == TopAbs_IN else 'out' if c == TopAbs_OUT else 'on'
    out = {"id": i, "kernel": kern is not None}
    # concave, covered: (a) material, (b)/(d) air. Decisive only where the kernel fills.
    if conc:
        t = [(st(kern, p) if kern else None, st(truth, p)) for p in conc]
        dec = [tt for kk, tt in t if kk == 'in' or kern is None]
        out["concave"] = {"points": len(dec), "truth_air": sum(1 for x in dec if x == 'out'), "truth_material": sum(1 for x in dec if x == 'in')}
    # convex, the ball pokes out: (a)/(d) cut, (b) material. Decisive where the kernel cuts.
    if conv:
        t = [(st(kern, p) if kern else None, st(truth, p)) for p in conv]
        dec = [tt for kk, tt in t if kk == 'out']
        out["convex"] = {"points": len(dec), "truth_air": sum(1 for x in dec if x == 'out'), "truth_material": sum(1 for x in dec if x == 'in')}
    print(json.dumps(out), flush=True)
