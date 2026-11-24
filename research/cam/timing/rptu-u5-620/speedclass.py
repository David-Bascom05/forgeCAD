import csv, sys, re, collections, datetime as dt
base='kernel/forge-machine/tests/data/spinner-u5/'
# single executions at override 100% throughout (logtimes4.py)
P = {'CAM_SCHLUESSEL_LOGO_2_MPF': ('Keychain_Schluesselanhaenger.mpf', ['2024-03-20 08:26:00','2024-03-20 08:27:36','2024-03-22 07:41:34','2024-03-22 08:44:41']),
     'CAM_OEFFNER_MPF': ('Bottle_opener_Flaschenoeffner.txt', ['2024-03-20 10:49:33','2024-03-20 12:14:08','2024-03-20 14:17:48','2024-03-22 08:58:41']),
     'KOORDI_JEAN_CAM3_MPF': ('Coordinate_system_representation_Achskreuz.mpf', ['2024-03-20 07:35:42','2024-03-20 08:56:15','2024-03-20 10:14:04','2024-03-22 09:30:21'])}
FL={}; G0={}
for p,(f,_) in P.items():
    text=open(base+f).read().splitlines(); F=[None]*(len(text)+2); g0=[False]*(len(text)+2); fv=None; mode=None
    for i,l in enumerate(text, start=1):
        m=re.search(r'\bF([\d.]+)', l)
        if m: fv=float(m.group(1))
        for gm in re.findall(r'\bG([0-3])\b', l): mode=gm
        F[i]=fv; g0[i]=(mode=='0' or 'SUPA' in l)
    FL[p]=F; G0[p]=g0
rows=collections.defaultdict(list)
for row in csv.DictReader(open(sys.argv[1], newline='')):
    p=row['Program_path'].split('_N_')[-1]
    if p in P: rows[p].append((dt.datetime.fromisoformat(row['time']), row))
for p,(f,starts) in P.items():
    c=collections.Counter(); n=0
    for st in starts:
        t0=dt.datetime.fromisoformat(st); run=[]
        for a in rows[p]:
            if a[0]<t0 or a[1]['Program_status']!='3.0': continue
            if run and (a[0]-run[-1][0]).total_seconds()>120: break
            l=float(a[1]['Program_line_number'] or 0)
            if run and 0<l<=30 and max(float(x['Program_line_number'] or 0) for _,x in run)>len(FL[p])-12: break
            run.append(a)
        n+=1
        for (ta,xa),(tb,xb) in zip(run,run[1:]):
            d=(tb-ta).total_seconds()
            if d>2: continue
            ln=int(float(xa['Program_line_number'] or 0))
            if ln==-2: c['tool change']+=d; continue
            if not (0<ln<len(FL[p])): c['other']+=d; continue
            v=sum(float(xa[f'{a}-axis_feed'])**2 for a in 'XYZ')**0.5
            vr=abs(float(xa['B-axis_feed']))+abs(float(xa['C-axis_feed']))
            ov=float(xa['Feed_override'])
            if ov<99.9: c['override < 100%']+=d; continue
            if G0[p][ln]:
                c['rapid (G0/SUPA)' if v>1 or vr>1 else 'G0 line, not moving']+=d; continue
            F=FL[p][ln] or 1
            k='at feed (>=90% F)' if v>=0.9*F else 'reduced (10-90% F)' if v>=0.1*F else 'near stop (<10% F)'
            c[k]+=d
    tot=sum(c.values())
    print(f'== {f} (mean of {n} runs, {tot/n:.0f} s)')
    for k,v in c.most_common(): print(f'   {k:24s} {v/n:7.1f} s  {100*v/tot:5.1f}%')
