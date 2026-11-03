import json,sys,subprocess,os
FG=os.path.expanduser("~/projects/ForgeCAD/.claude/worktrees/forge-lang/target/release/fg")
out=subprocess.run([FG,"run",sys.argv[1],"--json"],capture_output=True,text=True).stdout
d=json.loads(out.strip().splitlines()[-1])
for e in d['errors']: print("ERR",e['kind'],e.get('text'))
a=d.get('assembly') or {}
r=lambda v:[round(x,4) for x in v]
for i in a.get('instances',[]):
    p=i['pose']; print(f"{i['id']:12s} o={r(p['origin'])} x={r(p['x'])} z={r(p['z'])}")
for k in ('mobility','interferences'): print(k, a.get(k))
