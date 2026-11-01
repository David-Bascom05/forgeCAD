import sys,collections as C
from inclusive import stacks
part=sys.argv[1]; root=sys.argv[2]; depth=int(sys.argv[3]); 
tot=C.Counter(); N=0
for s in stacks(f'prof/{part}.txt.gz'):
    j=' '.join(s)
    if 'Model>::update' not in j or 'Model>::edit' in j or 'bench::model' in j: continue
    N+=1
    if root in j:
        for i in range(len(s)-1,-1,-1):
            if root in s[i]:
                ch=s[max(0,i-depth):i+1][::-1]
                tot[' > '.join(y.split('::')[-2][-14:]+'::'+y.split('::')[-1][:26] if '::' in y else y[:30] for y in ch[1:])]+=1
                break
print(part,N)
for k,v in tot.most_common(int(sys.argv[4]) if len(sys.argv)>4 else 25): print(f'{100*v/N:5.1f} {k[:150]}')
