"""Cross-validated calibration, the hybrid model: rapids trapezoidal at A_rapid (grbl's planner,
outside CYCLE832's jerk limitation), cutting S-curve at (A, J) with G642 tolerance 0.02 mm.
Fit (A_rapid, A, J) on one program's per-operation times; predict the others' totals and
per-operation times. Also the single-model baselines: t_la (grbl, A 2000) and S-curve alone."""
import csv, json, collections, math, itertools, sys
LOG = json.load(open('research/cam/timing/rptu-u5-620/out/log_perop.json'))
CUT = collections.defaultdict(lambda: collections.defaultdict(float))   # (prog,a,j) -> op -> t
RAP = collections.defaultdict(lambda: collections.defaultdict(float))   # (prog,a,j) -> op -> t (S-curve rapids)
TRAP = collections.defaultdict(lambda: collections.defaultdict(float))  # (prog,ar) -> op -> t
for r in csv.DictReader(open('research/cam/timing/rptu-u5-620/out/scurve.csv')):
    k = r['kind']; t = float(r['t'])
    if k == 'rapid_trap': TRAP[(r['program'], float(r['a']))][r['op']] += t
    elif k == 'cut': CUT[(r['program'], float(r['a']), float(r['j']))][r['op']] += t
    else: RAP[(r['program'], float(r['a']), float(r['j']))][r['op']] += t
AR = sorted({a for (_, a) in TRAP}); AJ = sorted({(a, j) for (_, a, j) in CUT})
def pred(prog, ar, a, j):
    out = collections.defaultdict(float)
    for o, t in CUT[(prog, a, j)].items(): out[o] += t
    for o, t in TRAP[(prog, ar)].items(): out[o] += t
    return out
def err(prog, s):
    lg = LOG[prog]; keys = [o for o in lg if lg[o] >= 5.0]
    return math.sqrt(sum(((s.get(o, 0) - lg[o]) / lg[o]) ** 2 for o in keys) / len(keys))
progs = ['Keychain', 'Bottle', 'Achskreuz']
for fitp in progs:
    best = min(((ar, a, j) for ar in AR for (a, j) in AJ), key=lambda x: err(fitp, pred(fitp, *x)))
    print(f'== fitted on {fitp}: A_rapid {best[0]:g}, A {best[1]:g} mm/s², J {best[2]:g} mm/s³')
    for prog in progs:
        s = pred(prog, *best); lg = sum(LOG[prog].values()); sm = sum(s.values())
        print(f'   {prog:10s} logged {lg:7.1f}  predicted {sm:7.1f}  total error {100*(sm-lg)/lg:+6.1f} %  per-op rms {100*err(prog, s):5.1f} %' + ('  (fit)' if prog == fitp else ''))
json.dump({'note': 'see calib2.py'}, open('research/cam/timing/rptu-u5-620/out/calib2.json', 'w'))

print('== in-sample: the best single parameter set over all three programs (not a prediction)')
def scurve_only(prog, a, j):
    out = collections.defaultdict(float)
    for o, t in CUT[(prog, a, j)].items(): out[o] += t
    for o, t in RAP[(prog, a, j)].items(): out[o] += t
    return out
def tot_err(s, prog): return (sum(s.values()) - sum(LOG[prog].values())) / sum(LOG[prog].values())
for name, cands, f in (('S-curve alone (A, J)', AJ, lambda p, x: scurve_only(p, *x)),
                       ('rapids trapezoidal + cutting S-curve (A_rapid, A, J)', [(ar, a, j) for ar in AR for (a, j) in AJ], lambda p, x: pred(p, *x))):
    best = min(cands, key=lambda x: max(abs(tot_err(f(p, x), p)) for p in progs))
    print(f'   {name}: minimax total error at {best}: ' + ', '.join(f'{p} {100*tot_err(f(p, best), p):+.1f} % (per-op rms {100*err(p, f(p, best)):.1f} %)' for p in progs))
    best2 = min(cands, key=lambda x: sum(err(p, f(p, x)) ** 2 for p in progs))
    print(f'   {name}: least per-op rms over all three at {best2}: ' + ', '.join(f'{p} {100*tot_err(f(p, best2), p):+.1f} % (per-op {100*err(p, f(p, best2)):.1f} %)' for p in progs))
