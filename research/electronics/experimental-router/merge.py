from pathlib import Path
import re,sys
lines=Path(sys.argv[1]).read_text()+Path('extra.copper').read_text()
counts={}
def incr(m):
 k=m[1];i=counts.get(k,0);counts[k]=i+1
 return f'{k}_{i} ='
Path(sys.argv[2]).write_text(re.sub(r'^(wire|stitch)_\d+ =',incr,lines,flags=re.M))
