#!/usr/bin/env python3
# tally_census.py OUT: classes by rate.py rule (stopped always failed), per dataset, the run vs this build; what moved; each logged route
import json,os,sys,collections
D=os.path.expanduser('~/forgecad-data')
T={}
for f in ('truth20.jsonl','truth200.jsonl'):
    for l in open(f'{D}/cadfs/run/{f}'):
        d=json.loads(l)
        if d.get('solids') or 'err' not in d: T[d['id']]=d
for l in open(f'{D}/whucad/run/truth_man.jsonl'):
    d=json.loads(l); T.setdefault(d['id'],d)
def cmp(o,t,notes):
    lim=1e-2 if 'approx' in notes else 1e-3
    if o.get('vol') is None or o.get('area') is None: return False,1,1,1
    ev=abs(o['vol']-t['vol'])/max(t['vol'],1e-12); ea=abs(o['area']-t['area'])/max(t['area'],1e-12)
    a=o.get('bbox'); u=o.get('bboxu') or a; b=t.get('bbox'); eb=0.0
    if b and a:
        diag=max(sum((b[i+3]-b[i])**2 for i in range(3))**.5,1e-12)
        for i in range(6):
            lo,hi=min(a[i],u[i]),max(a[i],u[i]); eb=max(eb,lo-b[i],b[i]-hi)
        eb/=diag
    return ev<=lim and ea<=lim and eb<=lim,ev,ea,eb
def cl(o,t,st,notes):
    if o.get('v')!='ok': return 'failed'
    if not (t or {}).get('solids'): return 'notruth'
    if st=='stopped': return 'failed'
    return 'match' if cmp(o,t,notes)[0] else 'wrong'
rows=[json.loads(l) for l in open(sys.argv[1])]
tab=collections.defaultdict(collections.Counter); moved=collections.defaultdict(list); route=collections.defaultdict(collections.Counter); ex=collections.defaultdict(list)
for r in rows:
    t=r.get('truth') or T.get(r['k']) or {}
    a=cl(r['main'],t,r['status'],r['notes']); b=cl(r,t,r['status'],r['notes'])
    tab[r['ds']]['main '+a]+=1; tab[r['ds']]['new '+b]+=1
    if a!=b: moved[(r['ds'],a,b)].append(r['k'])
    for x in r['routes']:
        route[x][b]+=1; ex[(x,b)].append(r['ds'][0]+':'+r['k'])
for ds in tab:
    print(ds,sum(v for k,v in tab[ds].items() if k.startswith('main')))
    for c in ('match','wrong','failed','notruth'):
        print(f"  {c:8} main {tab[ds]['main '+c]:5}  new {tab[ds]['new '+c]:5}")
print('moved:')
for k,v in sorted(moved.items()): print(' ',k,len(v),' '.join(v[:12]))
print('routes:')
for x,c in sorted(route.items()):
    print(' ',x,dict(c))
    for b in c:
        if b!='match': print('    ',b,' '.join(ex[(x,b)][:15]))
