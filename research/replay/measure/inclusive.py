import gzip,sys,collections as C
def stacks(fn):
    cur=[]
    with gzip.open(fn,'rt') as f:
        for l in f:
            if not l.strip():
                if cur: yield cur
                cur=[]
            elif l[0] in ' \t':
                p=l.split(None,1); cur.append(p[1].strip() if len(p)>1 else '?')
    if cur: yield cur
def keep(s):
    j=' '.join(s)
    return 'Model>::update' in j and 'Model>::edit' not in j and 'bench::model' not in j and 'Part' not in j
if __name__=='__main__':
    part=sys.argv[1]; top=int(sys.argv[2]) if len(sys.argv)>2 else 45
    inc=C.Counter(); self_=C.Counter(); n=0
    for s in stacks(f'prof/{part}.txt.gz'):
        if not keep(s): continue
        n+=1
        self_[s[0]]+=1
        for x in set(s): inc[x]+=1
    print(part,'samples',n,'Mi/step',n*0.2/17)
    for k,v in inc.most_common(top):
        print(f'{100*v/n:5.1f} {100*self_[k]/n:5.1f}  {k[:110]}')
