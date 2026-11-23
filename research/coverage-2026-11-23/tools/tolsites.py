#!/usr/bin/env python3
"""Tolerance decision sites in forge-bool (lines with tol::LINEAR/ANGULAR, RIM, .tol, 1e-N):
per branch condition, evaluations and true/false split. usage: tolsites.py EXPORT.json"""
import json, os, re, sys, collections
WT = os.environ['WT']
d = json.load(open(sys.argv[1]))['data'][0]
br = collections.defaultdict(lambda: [0, 0])
for f in d['functions']:
    fn = f['filenames'][0]
    fn = fn[fn.find('kernel/'):]
    if 'forge-bool/src/' not in fn or 'imprint' in fn:
        continue
    for b in f.get('branches', []):
        if b[6] != 0:
            continue
        k = (fn, b[0], b[1])
        br[k][0] += b[4]
        br[k][1] += b[5]
src = {}
rx = re.compile(r'tol::(LINEAR|ANGULAR)|\bRIM\b|\.tol\b|\btol\b|1e-\d')
rows = []
for (fn, l, c), (t, f) in br.items():
    if fn not in src:
        src[fn] = open(os.path.join(WT, fn)).read().split('\n')
    s = src[fn][l - 1]
    if rx.search(s):
        rows.append((t + f, fn.split('/')[-1], l, c, t, f, s.strip()[:100]))
rows.sort(reverse=True)
print('evals\tfile:line:col\ttrue\tfalse\tminority%\tsource')
for n, fn, l, c, t, f, s in rows:
    m = f"{100*min(t, f)/n:.3f}" if n else '-'
    print(f"{n}\t{fn}:{l}:{c}\t{t}\t{f}\t{m}\t{s}")
