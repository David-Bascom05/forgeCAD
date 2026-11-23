import re, subprocess, sys, collections
def fns(f):
    out = subprocess.run(['callgrind_annotate', '--threshold=100', '--auto=no', f], capture_output=True, text=True).stdout
    c = collections.Counter()
    for line in out.splitlines():
        m = re.match(r'\s*([\d,]+) \(\s*[\d.]+%\)\s+[^:]*:(.*?)( \[/.*\])?$', line)
        if m:
            k = re.sub(r"'\d+", "", m.group(2))[:110]
            c[k] += int(m.group(1).replace(',', ''))
    return c
import os; S = os.environ.get('CG', 'cg/')
p=sys.argv[1]; V=sys.argv[2]; pat=sys.argv[3]
g=fns(S+f'{V}.{p}.fga.full.cg')
d=sorted(((v,k) for k,v in g.items() if re.search(pat,k)), reverse=True)
print(sum(v for v,k in d))
for v,k in d[:int(sys.argv[4]) if len(sys.argv)>4 else 30]: print(f'{v:10d} {k}')
