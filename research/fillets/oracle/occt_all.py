"""occt_all.py FILE.step FILE.all: label_pipe's ALL dump (`x y z index oracle`, 1 = material)
classified by OCCT, as a witness independent of forge-bool's Index. Prints one JSON line: how many
points, how many OCCT puts on each side of Index, and the points where OCCT and the oracle disagree
(`occt_vs_oracle`), split by whether Index sided with the oracle (`masked`: Index hid it) or with
OCCT (`both`: the disagreement label_pipe saw)."""
import json, sys
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.TopAbs import TopAbs_IN, TopAbs_ON
from OCP.gp import gp_Pnt

r = STEPControl_Reader()
assert r.ReadFile(sys.argv[1]) == 1
r.TransferRoots()
s = r.OneShape()
c = BRepClass3d_SolidClassifier(s)
n = on = index_wrong = 0
masked, both = [], []
for line in open(sys.argv[2]):
    w = line.split()
    p = gp_Pnt(*map(float, w[:3]))
    k, o = w[3] == "1", w[4] == "1"
    c.Perform(p, 1e-7)
    st = c.State()
    n += 1
    if st == TopAbs_ON:
        on += 1
        continue
    oc = st == TopAbs_IN
    if oc != k:
        index_wrong += 1
    if oc != o:
        (masked if k == o else both).append([round(float(x), 4) for x in w[:3]] + [int(oc)])
print(json.dumps({"step": sys.argv[1].rsplit("/", 1)[-1], "valid": BRepCheck_Analyzer(s).IsValid(),
                  "points": n, "on": on, "index_vs_occt": index_wrong,
                  "masked": len(masked), "both": len(both), "masked_at": masked[:8], "both_at": both[:4]}))
