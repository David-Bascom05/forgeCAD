import collections as C
from inclusive import stacks
from matrix import cat
Q=['frame','heatsink','gearbox','manifold','crank','volute','conrod']
SEARCH=('Procedural>::within','Hull>::misses','Skinned>::hull','Eqn>::critical','Eqn>::count','loftmeet::halves','quartic_disc','Eqn>::coeffs','Trig>::roots','Hull>::extent','Eqn>::scaled','screw::misses','Surface>::implicit','patches')
print('part      total | parse+model | calc closed proc(+boxesP) | dec closed proc(of which valid) | caps A(all dec free) B(+validity) C(+moved rej proc pairs + search in kept ssi) D=C less same-value pairs')
for part in Q:
    c=C.Counter();N=0;srch=0;ssi=0
    for s in stacks(f'prof/{part}.txt.gz'):
        j=' '.join(s)
        if 'Model>::update' not in j or 'Model>::edit' in j or 'bench::model' in j: continue
        N+=1;c[cat(s)]+=1
        if 'ssi::searched' in j:
            ssi+=1
            if any(k in j for k in SEARCH): srch+=1
    Pr=[];Nn=[];Mi=[]
    for l in open(f'steps-{part}.txt'):
        if l.startswith('step'): Mi.append(float(l.split()[2]))
        if l.startswith('pairs'):
            a=l.split(' n '); Pr.append([float(x) for x in a[0].split()[1:]]); Nn.append(eval(a[1]))
    Pr=Pr[1:];Mi=Mi[1:];Nn=Nn[1:]
    pr=[sum(x)/len(x) for x in zip(*Pr)]; nn=[sum(x)/len(x) for x in zip(*Nn)]; T=sum(Mi)/len(Mi)
    T={'frame':500,'heatsink':559,'gearbox':353,'manifold':262,'crank':337,'volute':898,'conrod':52}[part]
    m={k:v/N*T for k,v in c.items()}
    g=lambda k:m.get(k,0)
    def b(p,r,i): return pr[p*8+r*4+i]+pr[16+p*8+r*4+i]
    over=g('parse')+g('modelother')+g('stepother_C')+g('stepother_P')
    keptc=b(0,0,0)+b(0,0,1)+b(0,0,3); keptp=b(1,0,0)+b(1,0,1)+b(1,0,3)
    calc_c=keptc+g('vertices_C')+g('sew_C')+g('tool_C')+g('fillet_C')+g('chamfer_C')+g('boolother_C')
    calc_p=keptp+g('vertices_P')+g('sew_P')+g('tool_P')+g('fillet_P')+g('chamfer_P')+g('boolother_P')+g('boxes_P')
    rejc=sum(b(0,1,i) for i in range(4))+b(0,0,2); rejp=sum(b(1,1,i) for i in range(4))+b(1,0,2)
    val=g('validity_C')+g('validity_P')
    dec_c=rejc+g('classify_C')+g('splitfaces_C')+g('boxes_C')+g('validity_C')
    dec_p=rejp+g('classify_P')+g('splitfaces_P')+g('validity_P')
    A=T/(over+calc_c+calc_p); Bc=T/(over+calc_c+calc_p+val)
    sh=srch/ssi if ssi else 0
    rej_ssi=b(1,1,0)+b(0,1,0)
    kept_search=max(0,sh*(b(1,0,0)+b(1,1,0))-rej_ssi)
    moved_rej=sum(pr[8+4+i] for i in range(4))+sum(pr[4+i] for i in range(4))
    Cc=over+calc_c+calc_p+val+moved_rej  # search within kept ssi already in keptp
    same_calc=pr[16]+pr[17]+pr[19]+pr[24]+pr[25]+pr[27]
    D=Cc-same_calc
    print(f'{part:9} {T:5.0f} | {over:4.0f} | {calc_c:4.0f} {calc_p:4.0f} | {dec_c:4.0f} {dec_p:4.0f} ({val:4.0f}) | A {A:4.1f}x B {Bc:4.1f}x C {T/Cc:4.1f}x D {T/D:4.1f}x | ssi search share {sh:.2f}; kept-ssi-search {kept_search:4.0f}; moved rej {moved_rej:4.0f}; same in kept {same_calc:4.0f}; same total {sum(pr[16:]):4.0f}/{sum(pr):4.0f} n {[round(x) for x in nn[:4]]} {[round(x) for x in nn[8:12]]}')
