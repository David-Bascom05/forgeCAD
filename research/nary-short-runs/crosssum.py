import sys, statistics
d = {}
for line in open(sys.argv[1]):
    l, s, m, c = line.split()
    d.setdefault(int(l), {}).setdefault(s, {})[m] = int(c)
for l in sorted(d):
    r = [v['nary1'] / v['seq'] for v in d[l].values()]
    ss = sum(v['seq'] for v in d[l].values()); sn = sum(v['nary1'] for v in d[l].values())
    print(f"{l:3d} tools: median {statistics.median(r):.3f} min {min(r):.3f} max {max(r):.3f} sum {sn/ss:.3f} seq {ss/len(r)/1e6:.1f}M")
