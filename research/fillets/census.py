#!/usr/bin/env python3
# census.py JUDGE RUN OUT: every part of a dataset run holding a fillet, judged on this build; tally_census.py compares it with that run
import json,os,subprocess,sys,concurrent.futures as cf
BIN,RUN,OUT=sys.argv[1:4]
D=os.path.expanduser('~/forgecad-data')
def load(ds):
    run=f'{D}/{ds}/runs/{RUN}'
    res={}
    for l in open(f'{run}/results.jsonl'):
        r=json.loads(l)
        k=r.get('id') or (r['file']+'.'+r['body'])
        res[k]=r
    return run,res
def one(a):
    ds,k,p,t,r0=a
    try:
        x=subprocess.run(['nice','-n','15',BIN,p],capture_output=True,text=True,timeout=90)
        err=x.stderr; line=x.stdout.strip().splitlines()[-1] if x.stdout.strip() else '{}'
        o=json.loads(line)
    except subprocess.TimeoutExpired:
        err='';o={'v':'timeout'}
    except Exception as e:
        err='';o={'v':'crash:'+str(e)[:60]}
    routes=sorted({l.split()[1] for l in err.splitlines() if l.startswith('ROUTE ')})
    o2={'ds':ds,'k':k,'v':o.get('v'),'sig':(o.get('sig') or '')[:120],'vol':o.get('vol'),'area':o.get('area'),'problems':o.get('problems'),'bbox':o.get('bbox'),'bboxu':o.get('bboxu'),'routes':routes,'truth':t,'status':r0.get('status'),'notes':r0.get('notes',[]),'main':{x:r0.get(x) for x in ('v','vol','area','bbox','bboxu','sig')}}
    return o2
jobs=[]
for ds in ('whucad','freecad','cadfs'):
    run,res=load(ds)
    for fn in os.listdir(run+'/parts'):
        if not fn.endswith('.fc'): continue
        p=f'{run}/parts/{fn}'
        k=fn[:-3]
        s=open(p,errors='replace').read()
        if 'fillet' not in s: continue
        r=res.get(k,{})
        jobs.append((ds,k,p,r.get('truth'),r))
print(len(jobs),'parts',flush=True)
done=0
with open(OUT,'w') as out, cf.ThreadPoolExecutor(int(os.environ.get('JOBS','4'))) as ex:
    for r in ex.map(one,jobs):
        out.write(json.dumps(r)+'\n'); out.flush(); done+=1
        if done%200==0: print(done,flush=True)
