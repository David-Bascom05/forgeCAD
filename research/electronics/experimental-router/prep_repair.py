import json,re,sys,subprocess,pathlib
from pathlib import Path
base=sys.argv[1]
copper=Path(base).read_text()
s=Path('panel.fg').read_text()+copper
Path('repairinput.fg').write_text(s)
conn=json.loads(subprocess.check_output(['./check',str(Path('repairinput.fg').resolve()),'connectivity']))['result']
pads={l.split()[0]:l for l in Path('pads.tsv').read_text().splitlines()}
partial={}
for n in conn:
 groups=[g for g in n['islands'] if any(x in pads for x in g)]
 if len(groups)<2:continue
 groups.sort(key=lambda g:-len(g))
 for i,g in enumerate(groups):
  for x in g:partial[x]=i
with open('metals.tsv','w') as f:
 for s in copper.splitlines():
  k=s.split()[0]; net=re.search(r'net:(\w+)',s).group(1)
  if 'through_via' in s:
   xy=re.search(r'at:\(([^)]+)',s).group(1)
   print(k,net,partial.get(k,-1),'v',xy.replace(',',' '),file=f)
  else:
   lay=re.search(r'layer:(\w+)',s).group(1)
   width=re.search(r'width:([\d.]+)',s).group(1)
   pts=re.findall(r'\(([\d.]+),([\d.]+)\)',s)
   print(k,net,partial.get(k,-1),['top','signal','bottom'].index(lay),width,*[v for xy in pts for v in xy],file=f)
 for k,l in pads.items():
  v=l.split()
  print(k,v[1],partial.get(k,-1),'p',*v[2:],file=f)
