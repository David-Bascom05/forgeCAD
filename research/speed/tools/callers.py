import re, subprocess, sys, collections
# inclusive cost of FN by caller, fg profile vs fc profile: callers.py V PART FNREGEX
def callers(f, fn):
    out = subprocess.run(['callgrind_annotate', '--inclusive=yes', '--tree=caller', '--threshold=100', '--auto=no', f],
                         capture_output=True, text=True).stdout
    c = collections.Counter()
    block = []
    for line in out.splitlines():
        if not line.strip():
            block = []
            continue
        block.append(line)
        m = re.match(r'\s*([\d,]+) \(\s*[\d.]+%\)\s+\*\s+[^:]*:(.*?)( \[/.*\])?$', line)
        if m and re.search(fn, m.group(2)):
            for b in block[:-1]:
                mm = re.match(r'\s*([\d,]+) \(\s*[\d.]+%\)\s+<\s+[^:]*:(.*?)( \(\d+x\))?( \[/.*\])?$', b)
                if mm:
                    k = re.sub(r"'\d+", "", mm.group(2))[:120]
                    c[k] += int(mm.group(1).replace(',', ''))
    return c
import os; S = os.environ.get('CG', 'cg/')
V, p, fn = sys.argv[1], sys.argv[2], sys.argv[3]
g = callers(S + f'{V}.{p}.fga.full.cg', fn)
c = callers(S + f'before.{p}.fc.full.cg', fn)
d = sorted(((g[k] - c[k], g[k], c[k], k) for k in set(g) | set(c)), reverse=True)
n = int(sys.argv[4]) if len(sys.argv) > 4 else 25
for x in d[:n]:
    print(f'{x[0]:+10d} {x[1]:10d} {x[2]:10d} {x[3]}')
print('...')
for x in d[-8:]:
    print(f'{x[0]:+10d} {x[1]:10d} {x[2]:10d} {x[3]}')
