import re, subprocess, sys, collections
# exclusive cost by category, fg profile vs fc profile, per part
def cats(f):
    out = subprocess.run(['callgrind_annotate', '--threshold=100', '--auto=no', f], capture_output=True, text=True).stdout
    c = collections.Counter()
    tot = 0
    for line in out.splitlines():
        m = re.match(r'\s*([\d,]+) \(\s*[\d.]+%\)\s+(.*)$', line)
        if not m:
            continue
        n = int(m.group(1).replace(',', ''))
        rest = m.group(2)
        if rest.startswith('PROGRAM'):
            tot = n
            continue
        fn = rest
        if 'forge_lang::syntax' in fn or 'syntax::V' in fn or 'syntax::Parser' in fn or 'syntax::Chain' in fn:
            k = 'parse'
        elif 'forge_lang' in fn:
            k = 'lang'
        elif 'forge_agent' in fn or 'forge_doc' in fn:
            k = 'fc-front'
        elif 'forge_sketch' in fn:
            k = 'sketch'
        elif re.search(r'forge_(bool|geom|math|topo|ops|model|mesh|font|step)', fn):
            k = 'kernel'
        elif 'malloc' in fn or 'memmove' in fn or 'memset' in fn or 'memcmp' in fn:
            k = 'libc-mem'
        else:
            k = 'rust-std/other'
        c[k] += n
    return tot, c
import os; S = os.environ.get('CG', 'cg/')
keys=['parse','lang','sketch','kernel','libc-mem','rust-std/other','fc-front']
print('part      gap      ' + ' '.join(f'{k:>14}' for k in keys))
V=sys.argv[1]
for p in sys.argv[2:]:
    tg, g = cats(S+f'{V}.{p}.fga.full.cg')
    tf, c = cats(S+f'before.{p}.fc.full.cg')
    print(f'{p:9} {(tg-tf)/1e6:+6.2f}M ' + ' '.join(f'{(g[k]-c[k])/1e6:+14.2f}' for k in keys))
