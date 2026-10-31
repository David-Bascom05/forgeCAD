import collections,subprocess,sys,os
B='target-inst/release/examples/featcost'
rows=[l.rstrip('\n').split('\t') for l in open('.scratch/seq.tsv')]
th=[(r[0][6:],int(r[1]),int(r[5])) for r in rows if r[0].startswith('bench/') and r[2].startswith('thread')]
tot=collections.Counter()
def rd(s,n):
    e=subprocess.run([B,'run',f'.scratch/fc/{s}.fc',str(n),'seq'],capture_output=True,text=True).stderr
    d={}
    for l in e.splitlines():
        p=l.split('\t')
        if len(p)==5: d[p[0]]=[int(x.split()[1]) for x in p[1:]]
    return d
for s,i,c in th:
    a,b=rd(s,i),rd(s,i+1)
    for k in b:
        de=b[k][1]-a.get(k,[0,0])[1]
        dc=b[k][0]-a.get(k,[0,0])[0]
        tot[k]+=de
    print(s,i,f'{c/1e6:.0f}M evals',sum(b[k][1]-a.get(k,[0,0])[1] for k in b))
T=sum(tot.values())
for k,c in tot.most_common(25):
    print(f'{c:10} {100*c/T:5.1f}%  {k}')
