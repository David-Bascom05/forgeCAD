# cls2.py CENSUS.jsonl OURS.json: the parts whose file gprop and file mesh differ by >.3%, sorted by cause, with which
# truth to score against, and how each reading compares with OCCT's build from the feature list and with ours
import json,sys,collections,statistics as st
T='/home/david-bascom/forgecad-data/occt-rates/whucad/'
tc={json.loads(l)['id']:json.loads(l) for l in open(T+'truthcheck.jsonl')}
raw={}
for l in open(T+'raw.jsonl'):
    try: d=json.loads(l); raw[d['id']]=d
    except Exception: pass
ours=json.load(open(sys.argv[2]))
def trust(d):
    # the mesh when it is whole, closed and settled; else gprop when its faces close and lie on their edges; else neither
    if 'err' in d: return 'none'
    mv=d['mv2']
    # a curve ending this far from its vertex leaves the shape itself undecided at the 0.3% the judge asks for
    if d['gap3']>1e-3*d['diag']: return 'none'
    closed=abs(d['mv1']-d['mv1shift'])<=1e-6*abs(mv) and abs(d['mv2']-d['mv2shift'])<=1e-6*abs(mv)
    if d['solids']==1 and not (d['miss1'] or d['miss2'] or d['free']) and closed and abs(d['mv1']-mv)<=1e-3*abs(mv): return 'mesh'
    if d['cnorm']*d['diag']/3<=1e-4*abs(mv) and d['pdev']<=1e-6 and d['gap3']<=1e-6: return 'gprop'
    return 'none'
def why(d):
    if 'err' in d: return 'X','census timed out','none'
    mv=d['mv2']
    meshopen=abs(d['mv1']-d['mv1shift'])>1e-6*abs(mv) or abs(d['mv2']-d['mv2shift'])>1e-6*abs(mv)
    if d['solids']!=1: return 'S','several solids in the file','none'
    if d['miss1'] or d['miss2'] or meshopen or d['free']: return 'M','the mesh is open or misses faces','none'
    if abs(d['mv1']-mv)/abs(mv)>1e-3: return 'N','the truth mesh (deflection 5e-4) is not converged','gprop'
    if d['gap3']>1e-6: return 'G','curve ends off their vertices: open pcurve loops','mesh'
    if d['pdev']>1e-6: return 'P','pcurves off their 3D edges by up to the edge tolerance','mesh'
    return 'U','unexplained','none'
C=collections.defaultdict(list); TR=collections.Counter(); TID=collections.defaultdict(list)
for l in open(sys.argv[1]):
    d=json.loads(l); C[why(d)].append(d); TR[(why(d)[0],trust(d))]+=1; TID[trust(d)].append(d['id'])
out={}
for k in sorted(C):
    rows=C[k]; n=len(rows); b=[];o=[]
    for d in rows:
        i=d['id']; t=tc[i]; bv=((raw.get(i) or {}).get('occt') or {}).get('vol'); ov=ours.get(i,{})
        if bv: b.append((abs(t['mv']-bv)/bv,abs(t['gv']-bv)/bv))
        if ov.get('v')=='ok' and ov.get('vol'): o.append((abs(t['mv']-ov['vol'])/ov['vol'],abs(t['gv']-ov['vol'])/ov['vol']))
    f=lambda L,j: '%3d: mesh closer %3d, within .1%% mesh %3d gprop %3d'%(len(L),sum(x[0]<x[1] for x in L),sum(x[0]<=1e-3 for x in L),sum(x[1]<=1e-3 for x in L)) if L else '  0'
    print('%s %-55s %3d  trust %-22s | vs OCCT build %s | vs ours %s'%(k[0],k[1],n,' '.join('%s %d'%(t,TR[(k[0],t)]) for t in ('mesh','gprop','none') if TR[(k[0],t)]),f(b,0),f(o,0)))
    out[k[0]]=dict(cause=k[1],ids=[d['id'] for d in rows])
print({t:len(v) for t,v in TID.items()}); json.dump(dict(causes=out,trust=TID),open('causes.json','w'),indent=0)
