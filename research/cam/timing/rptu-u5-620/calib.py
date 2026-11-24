"""Cross-validated calibration of the jerk-limited planner on the RPTU/Spinner log.
Fit (A, J) on ONE program's per-operation times (single executions at override 100 %, tool
changes apart), predict the others. Inputs: the log CSV and scurve.csv (tests/timing_u5.rs)."""
import csv, sys, re, collections, datetime as dt, itertools, math
base = sys.argv[3] if len(sys.argv) > 3 else 'kernel/forge-machine/tests/data/spinner-u5/'
P = {'Keychain': ('CAM_SCHLUESSEL_LOGO_2_MPF', 'Keychain_Schluesselanhaenger.mpf', ['2024-03-20 08:26:00','2024-03-20 08:27:36','2024-03-22 07:41:34','2024-03-22 08:44:41','2024-03-12 13:49:58','2024-03-20 09:26:45','2024-03-22 10:22:51']),
     'Bottle': ('CAM_OEFFNER_MPF', 'Bottle_opener_Flaschenoeffner.txt', ['2024-03-20 10:49:33','2024-03-20 12:14:08','2024-03-20 14:17:48','2024-03-22 08:58:41','2024-03-12 10:33:45','2024-03-20 08:10:10','2024-03-20 09:41:33','2024-03-22 10:08:39']),
     'Achskreuz': ('KOORDI_JEAN_CAM3_MPF', 'Coordinate_system_representation_Achskreuz.mpf', ['2024-03-20 07:35:42','2024-03-20 08:56:15','2024-03-20 10:14:04','2024-03-22 09:30:21','2024-03-20 11:32:58','2024-03-20 12:45:13','2024-03-22 08:09:03','2024-03-12 13:17:51'])}
def ops(text):
    op = ['(start)']; cur = '(start)'
    for l in text:
        m = re.search(r'Operation : (\S+)', l)
        if m: cur = m.group(1)
        op.append(cur)
    return op
rows = collections.defaultdict(list)
for row in csv.DictReader(open(sys.argv[1], newline='')):
    p = row['Program_path'].split('_N_')[-1]
    for k, (path, _, _) in P.items():
        if p == path: rows[k].append((dt.datetime.fromisoformat(row['time']), row))
LOG = {}
for k, (path, f, starts) in P.items():
    text = open(base + f).read().splitlines(); op = ops(text); n = len(text)
    acc = collections.Counter(); cnt = 0
    for st in starts:
        t0 = dt.datetime.fromisoformat(st); run = []
        for a in rows[k]:
            if a[0] < t0 or a[1]['Program_status'] != '3.0': continue
            if run and (a[0] - run[-1][0]).total_seconds() > 120: break
            l = float(a[1]['Program_line_number'] or 0)
            if run and 0 < l <= 30 and max(float(x['Program_line_number'] or 0) for _, x in run) > n - 12: break
            run.append(a)
        cnt += 1
        for (ta, xa), (tb, xb) in zip(run, run[1:]):
            d = (tb - ta).total_seconds()
            if d > 2: continue
            ln = int(float(xa['Program_line_number'] or 0))
            if ln == -2 or not (0 < ln < len(op)): continue
            acc[op[ln]] += d
    LOG[k] = {o: v / cnt for o, v in acc.items()}
SIM = collections.defaultdict(lambda: collections.defaultdict(float))
for r in csv.DictReader(open(sys.argv[2])):
    SIM[(r['program'], float(r['a']), float(r['j']))][r['op']] += float(r['t'])
grid = sorted({(a, j) for (_, a, j) in SIM})
def err(prog, aj):
    s = SIM[(prog, *aj)]; lg = LOG[prog]
    keys = [o for o in lg if lg[o] >= 5.0]          # operations of at least 5 s
    return math.sqrt(sum(((s.get(o, 0) - lg[o]) / lg[o]) ** 2 for o in keys) / len(keys))
def total(prog, aj): return sum(SIM[(prog, *aj)].values())
for fitp in ['Bottle', 'Keychain', 'Achskreuz']:
    best = min(grid, key=lambda aj: err(fitp, aj))
    print(f'== fitted on {fitp}: A = {best[0]:g} mm/s², J = {best[1]:g} mm/s³ (per-operation rms relative error there {100*err(fitp, best):.1f} %)')
    for prog in ['Keychain', 'Bottle', 'Achskreuz']:
        lg = sum(LOG[prog].values()); sm = total(prog, best)
        print(f'   {prog:10s} logged {lg:7.1f}  predicted {sm:7.1f}  error {100*(sm-lg)/lg:+6.1f} %  per-op rms {100*err(prog, best):5.1f} %' + ('   (fit)' if prog == fitp else ''))
# the landscape on Bottle: a few (A, J)
print('== Bottle per-op rms error over the grid (rows A, columns J)')
js = sorted({j for _, j in grid}); As = sorted({a for a, _ in grid})
print('      ' + ' '.join(f'{j:>7g}' for j in js))
for a in As: print(f'{a:6g} ' + ' '.join(f'{100*err("Bottle", (a, j)):7.1f}' for j in js))
import json; json.dump(LOG, open('research/cam/timing/rptu-u5-620/out/log_perop.json', 'w'), indent=1)
