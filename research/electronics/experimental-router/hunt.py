from pathlib import Path
import random,sys,shutil,subprocess,collections,re,math,json,time
random.seed(24603)
base=sys.argv[1];protect={'RECT','VPROT','V12','VIN','FUSED'}
def call(cmd):return subprocess.run(cmd,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
pads={l.split()[0]:tuple(map(float,l.split()[2:])) for l in Path('pads.tsv').read_text().splitlines()}
def stats(copper):
 Path('study.fg').write_text(Path('no-plane.fg').read_text()+copper)
 rows=json.loads(subprocess.check_output(['./check',str(Path('study.fg').resolve()),'connectivity']))['result']
 stalled=[]
 for row in rows:
  n=row['net']; groups=sorted([g for g in row['islands'] if any(x in pads for x in g)],key=len,reverse=True)
  if n=='GND': groups=[['ground']]+[g for g in groups if not any(x.startswith('stitch_') or (x in pads and pads[x][-1]>0) for x in g)]
  for g in groups[1:]:stalled.append((n,[x for x in g if x in pads]))
 return stalled
def distseg(p,a,b):
 dx,dy=b[0]-a[0],b[1]-a[1]; t=max(0,min(1,((p[0]-a[0])*dx+(p[1]-a[1])*dy)/(dx*dx+dy*dy or 1)));return math.hypot(a[0]+dx*t-p[0],a[1]+dy*t-p[1])
best=Path(base).read_text(); working=best; bs=len(stats(best));print('start',bs,flush=True)
for step in range(90):
 if step%6==0:working=best
 errs=stats(working); targets=[e for e in errs if e[0] not in {'VPROT','RECT'}]
 if not targets:break
 target,g=random.choice(targets); pin=random.choice(g);x,y=pads[pin][:2]
 rank=[];lines=working.splitlines(True)
 for i,line in enumerate(lines):
  net=re.search(r'net:(\w+)',line)[1]
  if net==target or net in protect:continue
  pts=[tuple(map(float,p)) for p in re.findall(r'\(([\d.]+),([\d.]+)\)',line)]
  if len(pts)==1:distance=math.hypot(pts[0][0]-x,pts[0][1]-y)-.27
  else:
   distance=min(distseg((x,y),a,b) for a,b in zip(pts,pts[1:]))-float(re.search(r'width:([\d.]+)',line)[1])/2
  if distance<3.1:rank.append((distance+random.uniform(0,0.30),i))
 cut={i for _,i in sorted(rank)[:random.randint(3,14)]}
 Path('cut.copper').write_text(''.join(l for i,l in enumerate(lines) if i not in cut))
 call([sys.executable,'fast.py','cut.copper']);call(['./repair2',target]);call([sys.executable,'merge.py','cut.copper','fixed.copper'])
 fixed=Path('fixed.copper').read_text();score=len(stats(fixed))
 if score<bs:best=fixed;bs=score;Path('best.copper').write_text(best);print(step,target,pin,'new best',bs,flush=True)
 if score<=len(errs):working=fixed
