#!/usr/bin/env python3
"""Per enclosing fn: the corpus-unrun regions and branch sides, with whether the tests run them.
usage: group.py OUTDIR FILE_REGEX > listing.txt"""
import csv, os, re, sys, collections
out, rx = sys.argv[1], re.compile(sys.argv[2])
WT = os.environ['WT']
src = {}
def line(f, l):
    if f not in src:
        src[f] = open(os.path.join(WT, f)).read().split('\n')
    s = src[f]
    return s[l - 1].strip() if 0 < l <= len(s) else ''

def rows(name):
    with open(os.path.join(out, name)) as fh:
        r = csv.reader(fh, delimiter='\t')
        hdr = next(r)
        for row in r:
            yield dict(zip(hdr, row))

by = collections.defaultdict(lambda: {'whole': [], 'rg': [], 'br': []})
for r in rows('fn_uncovered.tsv'):
    if not rx.search(r['file']) or r['in_test_mod'] == '1':
        continue
    l = int(r['line'])
    txt = line(r['file'], l)
    kind = 'FN' if re.search(r'\bfn\s', txt) else 'closure'
    by[(r['file'], r['fn'])]['whole'].append((l, kind, r['tests_entries'], txt[:100]))
for r in rows('rg_uncovered.tsv'):
    if not rx.search(r['file']):
        continue
    by[(r['file'], r['fn'])]['rg'].append((int(r['line']), int(r['end_line']), r['tests_count'], r['source'][:100]))
for r in rows('br_uncovered.tsv'):
    if not rx.search(r['file']):
        continue
    by[(r['file'], r['fn'])]['br'].append((int(r['line']), r['corpus_true'], r['corpus_false'], r['tests_true'], r['tests_false'], r['source'][:100]))

def tn(x):
    return 'T' if x not in ('0', 'n/a') else ('-' if x == '0' else '?')

for (f, fn), v in sorted(by.items(), key=lambda kv: (kv[0][0], min([x[0] for x in kv[1]['whole'] + kv[1]['rg'] + kv[1]['br']] or [0]))):
    n = len(v['whole']) + len(v['rg']) + len(v['br'])
    print(f"== {f.split('/src/')[-1]} :: {fn}   (unrun closures/fns {len(v['whole'])}, regions {len(v['rg'])}, branch sites {len(v['br'])})")
    for l, kind, t, txt in v['whole']:
        print(f"  {kind:7} L{l:<5} tests={t:<8} {txt}")
    for l, le, t, txt in v['rg']:
        print(f"  region  L{l}-{le:<5} tests={tn(t)} {txt}")
    for l, ct, cf, tt, tf, txt in v['br']:
        side = ('never true' if ct == '0' else '') + ('never false' if cf == '0' else '')
        if ct == '0' and cf == '0':
            side = 'never reached'
        print(f"  branch  L{l:<5} corpus {ct}/{cf} tests {tt}/{tf}  [{side}] {txt}")
