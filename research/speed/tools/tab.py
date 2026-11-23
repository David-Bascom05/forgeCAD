import sys
d={}
for f in sys.argv[1:]:
    for l in open(f):
        p=l.split()
        if len(p)==4 and p[3].isdigit(): d[(p[0],p[1],p[2])]=int(p[3])
parts="gearbox volute crank valve heatsink quadarm conrod jack frame manifold bracket insert clamp hub guard".split()
print(f"{'part':9} {'fc full':>8} {'fga':>8} {'fg':>8}   {'fc edit':>8} {'fga':>8} {'fg':>8}")
for p in parts:
    r=[p.ljust(9)]
    for m in ['full','edit']:
        c=d.get((p,'fc',m))
        r.append(f"{c/1e6:7.1f}M" if c else "       -")
        for e in ['fga','fg']:
            g=d.get((p,e,m))
            r.append(f"{(g-c)/c*100:+7.1f}%" if g and c else "       -")
        r.append(' ')
    print(' '.join(r))
