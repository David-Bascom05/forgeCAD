import sys
base, new = sys.argv[1], sys.argv[2]
syn = set("torture plate shaft housing coplanar patterns mixed hole-grid tour upto upto-flat".split())
b = {}
for l in open(base):
    k1, k2, v = l.split()
    b[(k1, k2)] = v
up = down = same = 0
bad = []
tot_b = tot_n = 0
for l in open(new):
    k1, k2, v = l.split()
    o = b.get((k1, k2))
    if v == 'FAIL' or o is None or o == 'FAIL':
        bad.append((k1, k2, o, v)); continue
    o, v = int(o), int(v)
    diag = k1 not in syn and k2 in ('seq', 'nary1')
    if v > o:
        up += 1
        print(f"UP   {k1:9} {k2:6} {o:>12} {v:>12} {100*(v-o)/o:+.4f}%{' (diag)' if diag else ''}")
    elif v < o:
        down += 1
        print(f"DOWN {k1:9} {k2:6} {o:>12} {v:>12} {100*(v-o)/o:+.4f}%{' (diag)' if diag else ''}")
    else:
        same += 1
print(f"lines up {up}, down {down}, same {same}; bad {bad}")
