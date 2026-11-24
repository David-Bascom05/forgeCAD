"""Per-execution run times of the three RPTU/Spinner programs from the Zenodo log.

An execution: from the first running sample at a program line <= 30 to the sample before the
line returns to <= 30 after reaching the program's last lines (each logged session holds two).
Running time: the sum of the intervals between consecutive running samples (status 3, AUTO) no
longer than 2 s (longer gaps: pauses); tool-change subprograms (line -2) apart."""
import csv, datetime as dt, sys, statistics as stt
NL = {'CAM_SCHLUESSEL_LOGO_2_MPF': 644, 'CAM_OEFFNER_MPF': 2792, 'KOORDI_JEAN_CAM3_MPF': 3073}
rows = {p: [] for p in NL}
for row in csv.DictReader(open(sys.argv[1], newline='')):
    p = row['Program_path'].split('_N_')[-1]
    if p in NL and row['Program_status'] == '3.0' and row['Operation_mode'] == '2.0':
        rows[p].append((dt.datetime.fromisoformat(row['time']), float(row['Program_line_number'] or 0), float(row['Feed_override'])))
for p, rs in rows.items():
    # split into executions
    ex = []; cur = []; top = 0
    for i, (t, l, o) in enumerate(rs):
        restart = cur and l > 0 and l <= 30 and top >= NL[p] - 10
        gap = cur and (t - cur[-1][0]).total_seconds() > 600
        if restart or gap:
            ex.append(cur); cur = []; top = 0
        cur.append((t, l, o)); top = max(top, l)
    if cur: ex.append(cur)
    out = []
    for e in ex:
        ls = [l for _, l, _ in e if l > 0]
        if not ls or max(ls) < NL[p] - 10 or min(ls) > 30: continue
        tot = tc = 0.0
        for (ta, la, _), (tb, lb, _) in zip(e, e[1:]):
            d = (tb - ta).total_seconds()
            if d > 2: continue
            tot += d
            if la == -2 and lb == -2: tc += d
        full = sum(1 for *_, o in e if abs(o - 100) < 1e-3) / len(e)
        out.append((e[0][0].isoformat(' '), tot, tc, tot - tc, full))
    print(p)
    for x in out: print('   %s  running %5.0f  tool changes %3.0f  rest %5.0f  override=100%% %.3f' % x)
    sel = [x for x in out if x[4] >= 0.95]
    for nm, s in (('all', out), ('override=100% in >=95% of samples', sel)):
        if s:
            print(f'   {nm}: n={len(s)}  running median {stt.median([x[1] for x in s]):.0f} [{min(x[1] for x in s):.0f}, {max(x[1] for x in s):.0f}]  tool changes median {stt.median([x[2] for x in s]):.0f}  rest median {stt.median([x[3] for x in s]):.0f} [{min(x[3] for x in s):.0f}, {max(x[3] for x in s):.0f}]')
