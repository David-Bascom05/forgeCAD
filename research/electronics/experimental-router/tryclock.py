import sys
from pathlib import Path
import re,math,subprocess
from collections import Counter
def dist(p,a,b):
 dx=b[0]-a[0];dy=b[1]-a[1];t=max(0,min(1,((p[0]-a[0])*dx+(p[1]-a[1])*dy)/(dx*dx+dy*dy or 1)))
 return math.hypot(p[0]-a[0]-dx*t,p[1]-a[1]-dy*t)
for rad in [.55,1.1,1.7,2.5]:
 keep=[];drop=[]
 for l in Path('final.copper').read_text().splitlines(True):
  n=re.search(r'net:(\w+)',l)[1]
  if n=='HSE8':continue
  if 'layer:top' not in l and 'through_via' not in l:keep.append(l);continue
  pts=[tuple(map(float,xy)) for xy in re.findall(r'\(([\d.]+),([\d.]+)\)',l)]
  ls=list(zip(pts,pts[1:])) or [(pts[0],pts[0])]
  d=min(dist((70.25+1.91*j/20,42.60-1.87*j/20),a,b) for j in range(21) for a,b in ls)
  if d<rad:drop.append(n)
  else:keep.append(l)
 Path('cut.copper').write_text(''.join(keep));subprocess.run([sys.executable,'fast.py','cut.copper'],check=True);subprocess.run(['./repair2','--clock'],check=True)
 if 'net:HSE8,' not in Path('extra.copper').read_text():continue
 subprocess.run([sys.executable,'merge.py','cut.copper',f'c{rad}.copper'],check=True)
 print(rad,Counter(drop),flush=True)
