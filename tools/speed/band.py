"""band.py BASE NOW: the speed rule (research/PROOF-STANDARD.md, the owner, 2026-11-23). A strict line (synthetic regen1 parts; a bench part's app/edit/cli)
fails if it is more than BAND% over base; the sum of the strict lines fails if it is more than BAND% over base. Bench parts' seq/nary1
lines are diagnostic (shown, flagged past 0.5%). Exit 1 on failure."""
import sys
BAND = 0.03
syn = set("torture plate shaft housing coplanar patterns mixed hole-grid tour upto upto-flat".split())
def load(p):
    d = {}
    for l in open(p):
        f = l.split()
        if len(f) == 3: d[(f[0], f[1])] = f[2]
    return d
b, n = load(sys.argv[1]), load(sys.argv[2])
bad = False; sb = sn = 0; worst = (-1e9, None)
for k in sorted(n):
    o, v = b.get(k), n[k]
    diag = k[0] not in syn and k[1] in ("seq", "nary1")
    if v == "FAIL" or o in (None, "FAIL"):
        print(*k, v, "base", o, "FAIL"); bad = True; continue
    o, v = int(o), int(v); d = (v - o) * 100 / o
    flag = ""
    if diag:
        flag = "  (diagnostic)" + ("  >0.5%" if d > 0.5 else "")
    else:
        sb += o; sn += v; worst = max(worst, (d, k))
        if d > BAND: flag = "  OVER BAND"; bad = True
    print(f"{k[0]:9} {k[1]:6} {v:14} base {o:14} {d:+.4f}%{flag}")
tot = (sn - sb) * 100 / sb if sb else 0
print(f"strict total {sn} base {sb} {tot:+.4f}%  worst strict line {worst[0]:+.4f}% {worst[1]}")
if tot > BAND: print("TOTAL OVER BAND"); bad = True
sys.exit(1 if bad else 0)
