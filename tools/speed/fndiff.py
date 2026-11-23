#!/usr/bin/env python3
"""Per-function self Ir of two callgrind out files, B less A: fndiff.py A.cg B.cg. Prints our
code (kernel, std, forge-lang) and glibc apart, then the functions that moved most."""
import subprocess, sys, re
def fns(path):
    out = subprocess.run(['callgrind_annotate', '--threshold=100', path], capture_output=True, text=True).stdout
    d = {}
    for line in out.splitlines():
        m = re.match(r'^\s*([\d,]+) \([^)]*\)\s+(.*)$', line)
        if not m or 'PROGRAM TOTALS' in line: continue
        # (keyed without the binary's path, so two builds' functions pair up; glibc's kept)
        k = re.sub(r' \[(/[^\]]*/)?([^/\]]+)\]$', lambda x: ' [' + x.group(2) + ']', m.group(2))
        d[k] = d.get(k, 0) + int(m.group(1).replace(',', ''))
    return d
a, b = fns(sys.argv[1]), fns(sys.argv[2])
keys = set(a) | set(b)
glibc = lambda k: 'libc.so' in k or 'ld-linux' in k
tot = {True: 0, False: 0}
rows = []
for k in keys:
    d = b.get(k, 0) - a.get(k, 0)
    if d:
        tot[glibc(k)] += d
        rows.append((d, k))
rows.sort()
print('  ours (kernel, std, forge-lang) %+d   glibc %+d   total %+d' % (tot[False], tot[True], tot[False] + tot[True]))
for d, k in rows[:4] + rows[-6:]:
    print('    %+9d  %s' % (d, k[:120]))
