#!/usr/bin/env python3
"""Heap allocations by phase from valgrind's dhat (--tool=dhat --dhat-out-file=F.json): each
allocation site goes to the innermost phase function on its stack. dhatphase.py F.json [top]
prints blocks and bytes per phase, then the top sites (default 12) by blocks."""
import json, sys, re
PHASES = [
    ('arrangement', r'decide_once::(inputs|snap|region|vertices|classes|arrange)\b'),
    ('sample', r'decide_once::(in_face|witness_inside|interior)\b'),
    ('ledger', r'decide_once::build_ledger\b|forge_bool_core::witnesses\b'),
    ('core', r'forge_bool_core::core\b|decide_once::sliver\b'),
    ('write', r'decide_once::(write|build_body|apply|chain)\b'),
    ('decide-once rest', r'decide_once::'),
    ('old: ssi/trim', r'forge_bool::boolean::trim\b|forge_bool::ssi::'),
    ('old: imprint/split', r'forge_bool::boolean::(imprint|split_face|edge_split)\b|forge_bool::imprint::'),
    ('old: classify', r'forge_bool::boolean::classify_specs\b|forge_bool::classify::'),
    ('old: unify', r'forge_bool::unify::'),
    ('old: write (apply)', r'forge_bool::boolean::apply\b'),
    ('old: rest of the boolean', r'forge_bool::'),
]
d = json.load(open(sys.argv[1]))
top = int(sys.argv[2]) if len(sys.argv) > 2 else 12
ft = d['ftbl']
tot = {}
sites = []
for pp in d['pps']:
    frames = [ft[i] for i in pp['fs']]
    ph = 'outside the boolean'
    for f in frames:
        hit = next((name for name, rx in PHASES if re.search(rx, f)), None)
        if hit:
            ph = hit
            break
    b, n = tot.get(ph, (0, 0))
    tot[ph] = (b + pp['tb'], n + pp['tbk'])
    where = next((f for f in frames if 'forge' in f and 'alloc::' not in f), frames[0] if frames else '?')
    sites.append((pp['tbk'], pp['tb'], ph, where))
print('%-26s %10s %14s' % ('phase', 'blocks', 'bytes'))
for ph, (b, n) in sorted(tot.items(), key=lambda x: -x[1][1]):
    print('%-26s %10d %14d' % (ph, n, b))
print('top sites by blocks:')
for n, b, ph, w in sorted(sites, reverse=True)[:top]:
    w = re.sub(r'^0x[0-9A-F]+: ', '', w)
    print('  %9d blocks %11d bytes  %-18s %s' % (n, b, ph, w[:110]))
