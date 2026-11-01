# drive.py IDS OUT JOBS: tc3.py one part per child, 150 s each, a timed-out part written as such
import sys,subprocess,json,os,concurrent.futures as cf
ids=open(sys.argv[1]).read().split(); out=sys.argv[2]
done={json.loads(l)['id'] for l in open(out)} if os.path.exists(out) else set()
py=os.path.expanduser('~/.local/share/forgecad-occt/venv/bin/python')
def one(i):
    tf='one.%s.jsonl'%i; open(tf,'w').close(); open(tf+'.ids','w').write(i)
    try:
        subprocess.run([py,'-I','tc3.py',tf+'.ids',tf],timeout=150,capture_output=True)
        l=open(tf).read().strip() or json.dumps(dict(id=i,err='no output'))
    except subprocess.TimeoutExpired: l=json.dumps(dict(id=i,err='timeout'))
    os.remove(tf); os.remove(tf+'.ids'); return l
with cf.ThreadPoolExecutor(int(sys.argv[3])) as ex, open(out,'a') as o:
    for l in ex.map(one,[i for i in ids if i not in done]): o.write(l+'\n'); o.flush()
