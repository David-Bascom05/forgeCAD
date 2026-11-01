import json,glob,os,collections,sys
d=os.path.expanduser('~/.local/share/forgecad-find-parts/fusion/json/*.json')
fs=glob.glob(d)
kinds=collections.Counter();pc=collections.Counter();deg=collections.Counter();designs=collections.Counter();perprof=collections.Counter()
ex={}
nspline=[];rat=0;per=0
ops=collections.Counter()
for f in fs:
    try:j=json.load(open(f))
    except Exception: continue
    tl=j.get('timeline',[]);ents=j.get('entities',{})
    used=set()
    for e in ents.values():
        t=e.get('type')
        if t=='Sketch':
            cs=e.get('curves',{})
            for k,c in cs.items():
                kinds[c.get('type')]+=1
                if c.get('type') not in ex: ex[c.get('type')]=(os.path.basename(f),c)
            for pid,pr in e.get('profiles',{}).items():
                ks=set()
                for lp in pr.get('loops',[]):
                    for pc_ in lp.get('profile_curves',[]):
                        ty=pc_.get('type');pc[ty]+=1;ks.add(ty)
                        if ty not in ex: ex[ty]=(os.path.basename(f),pc_)
                        if ty=='NurbsCurve3D' or 'degree' in pc_:
                            deg[pc_.get('degree')]+=1
                            if pc_.get('is_rational'): rat+=1
                            if pc_.get('is_periodic'): per+=1
                            nspline.append(len(pc_.get('control_points',[])))
                perprof[tuple(sorted(ks))]+=1
                used|=ks
        elif t: ops[t]+=1
    for u in used: designs[u]+=1
print(len(fs));print('sketch curves',kinds);print('profile curves',pc);print('per-profile',perprof.most_common(12));print('designs with kind',designs);print('degree',deg,'rational',rat,'periodic',per);print('ops',ops)
if nspline:
    nspline.sort();print('ctrl pts pct',[nspline[int(len(nspline)*q)] for q in (0,.5,.9,.99)],nspline[-1])
for k,v in ex.items():
    print(k,v[0],json.dumps(v[1])[:700])
