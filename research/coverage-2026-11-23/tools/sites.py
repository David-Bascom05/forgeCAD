#!/usr/bin/env python3
"""Count of the regions starting on each source line matching a pattern, per JSON export.
usage: sites.py FILE_REGEX LINE_REGEX A.json [B.json ...]  (prints file:line, counts per export, text)"""
import json, os, re, sys, collections
frx, lrx = re.compile(sys.argv[1]), re.compile(sys.argv[2])
WT = os.environ['WT']
def counts(path):
    d = json.load(open(path))['data'][0]
    c = collections.defaultdict(lambda: -1)
    for f in d['functions']:
        fn = f['filenames'][0]
        fn = fn[fn.find('kernel/'):]
        if not frx.search(fn):
            continue
        for r in f['regions']:
            if r[5] != 0 or r[7] != 0:
                continue
            k = (fn, r[0])
            c[k] = max(c[k], 0) + r[4] if c[k] >= 0 else r[4]
    return c
cs = [counts(p) for p in sys.argv[3:]]
files = sorted({k[0] for c in cs for k in c})
for fn in files:
    lines = open(os.path.join(WT, fn)).read().split('\n')
    tm = next((i + 1 for i, l in enumerate(lines) if l.strip() == '#[cfg(test)]' and i + 1 < len(lines) and lines[i + 1].lstrip().startswith('mod ')), 10**9)
    for i, l in enumerate(lines, 1):
        if i >= tm or not lrx.search(l):
            continue
        vals = [c.get((fn, i), -1) for c in cs]
        v = '\t'.join('·' if x < 0 else str(x) for x in vals)
        print(f"{fn.split('/src/')[-1]}:{i}\t{v}\t{l.strip()[:110]}")
