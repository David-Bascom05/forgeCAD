#!/usr/bin/env python3
"""Per-profile attribution over the per-part JSON exports in PPDIR.
usage: attrib.py PPDIR OUT.txt FILE:LINE ...
Prints: per profile, forge-bool regions covered and regions covered by it alone;
then, for each FILE:LINE (file relative to kernel/, e.g. forge-bool/src/boolean.rs:1033),
the profiles whose regions starting on that line ran, with counts."""
import json, os, sys, glob, collections
pp, out = sys.argv[1], sys.argv[2]
want = [(a.rsplit(':', 1)[0], int(a.rsplit(':', 1)[1])) for a in sys.argv[3:]]
per = {}
for path in sorted(glob.glob(os.path.join(pp, '*.json'))):
    name = os.path.basename(path)[:-5]
    d = json.load(open(path))['data'][0]
    reg = {}
    for f in d['functions']:
        fn = f['filenames'][0]
        fn = fn[fn.find('kernel/') + 7:]
        for r in f['regions']:
            if r[5] != 0 or r[7] != 0:
                continue
            k = (fn, r[0], r[1], r[2], r[3])
            reg[k] = reg.get(k, 0) + r[4]
    per[name] = reg
o = open(out, 'w')
allkeys = set().union(*[set(r) for r in per.values()])
cover = collections.Counter()
for name, reg in per.items():
    for k, n in reg.items():
        if n > 0 and k[0].startswith('forge-bool'):
            cover[k] += 1
o.write('profile\tbool_regions_run\tbool_regions_only_this\n')
for name, reg in sorted(per.items()):
    run = [k for k, n in reg.items() if n > 0 and k[0].startswith('forge-bool')]
    only = [k for k in run if cover[k] == 1]
    o.write(f'{name}\t{len(run)}\t{len(only)}\n')
o.write('\n')
for fn, line in want:
    hits = []
    for name, reg in sorted(per.items()):
        n = sum(c for k, c in reg.items() if k[0] == fn and k[1] == line)
        if n > 0:
            hits.append(f'{name}={n}')
    o.write(f'{fn}:{line}\t{len(hits)} profiles\t' + ' '.join(hits) + '\n')
print('ok')
