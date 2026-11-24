import csv, sys, re, collections, statistics as st
base='kernel/forge-machine/tests/data/spinner-u5/'
P = {'CAM_SCHLUESSEL_LOGO_2_MPF': 'Keychain_Schluesselanhaenger.mpf', 'CAM_OEFFNER_MPF': 'Bottle_opener_Flaschenoeffner.txt', 'KOORDI_JEAN_CAM3_MPF': 'Coordinate_system_representation_Achskreuz.mpf'}
FL={}; G0={}
for p,f in P.items():
    text=open(base+f).read().splitlines(); F=[None]*(len(text)+2); g0=[False]*(len(text)+2); fv=None; mode=None
    for i,l in enumerate(text, start=1):
        m=re.search(r'\bF([\d.]+)', l)
        if m: fv=float(m.group(1))
        for gm in re.findall(r'\bG([0-3])\b', l): mode=gm
        F[i]=fv; g0[i]=(mode=='0')
    FL[p]=F; G0[p]=g0
obs=collections.defaultdict(list)
r = csv.DictReader(open(sys.argv[1], newline=''))
for row in r:
    p=row['Program_path'].split('_N_')[-1]
    if p not in P or row['Program_status']!='3.0' or row['Feed_override']!='100.0': continue
    ln=int(float(row['Program_line_number'] or 0))
    if not (0<ln<len(FL[p])) or FL[p][ln] is None or G0[p][ln]: continue
    v=sum(float(row[f'{a}-axis_feed'])**2 for a in 'XYZ')**0.5
    if v>1: obs[(p[:8], FL[p][ln])].append(v)
for k,v in sorted(obs.items()):
    if len(v)>=20:
        q=sorted(v); p90=q[int(0.9*len(q))]
        print(f'{k[0]:9s} F{k[1]:7.0f}: n={len(v):5d} median {st.median(v):7.0f} p90 {p90:7.0f} mm/min  p90/F {p90/k[1]:.3f}')
