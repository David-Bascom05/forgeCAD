# Names on main against names now. usage: renames.py DUMP_MAIN DUMP_NOW (nameprobe dump)
import sys, collections as C
def load(f):
    d=C.defaultdict(lambda: C.defaultdict(set))
    for l in open(f):
        p=l.split()
        if len(p)==4: d[p[0]][p[1]].add((p[2],p[3]))
    return d
a,b=load(sys.argv[1]),load(sys.argv[2])
T=C.Counter(); N=C.Counter()
for part in sorted(a):
    out=[]
    for k in 'fev':
        gone=a[part][k]-b[part][k]; T[k]+=len(gone); N[k]+=len(a[part][k])
        out.append(f"{k} {len(gone)}/{len(a[part][k])}")
    print(f"{part:<10}"+'  '.join(out))
print('total', ' '.join(f"{k} {T[k]}/{N[k]} = {100*T[k]/max(N[k],1):.1f}%" for k in 'fev'))
