#!/usr/bin/env python3
"""Compare two llvm-cov JSON exports (corpus, tests) over chosen source files.

usage: analyze.py CORPUS.json TESTS.json OUTDIR [PER_PART_DIR]

Writes:
  OUTDIR/files.tsv          per file: lines/regions/branches covered, corpus vs tests
  OUTDIR/fn_uncovered.tsv   every function (dedup over monomorphs) the corpus never enters
  OUTDIR/br_uncovered.tsv   every branch with a side the corpus never takes (in entered fns)
  OUTDIR/rg_uncovered.tsv   every code region never run by the corpus, in entered functions
  OUTDIR/hot_branches.tsv   most-run branches (corpus) in forge-bool and forge-geom meet/ssi
  OUTDIR/hot_fns.tsv        most-entered functions
"""
import json, os, re, sys, collections

ROOT = None
FOCUS = re.compile(r'kernel/(forge-bool|forge-geom|forge-topo)/src/')


def load(path):
    d = json.load(open(path))['data'][0]
    return d


def rel(fn):
    i = fn.find('kernel/')
    return fn[i:] if i >= 0 else fn


src_cache = {}


def src(file):
    if file not in src_cache:
        p = os.path.join(ROOT, file)
        try:
            src_cache[file] = open(p).read().split('\n')
        except OSError:
            src_cache[file] = []
    return src_cache[file]


FN_RE = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:unsafe\s+)?fn\s+([A-Za-z0-9_]+)')
IMPL_RE = re.compile(r'^\s*impl\b[^{]*?(?:for\s+)?([A-Z][A-Za-z0-9_]*)\s*(?:<[^{]*>)?\s*(?:where[^{]*)?\{?\s*$')
TEST_RE = re.compile(r'^\s*#\[cfg\(test\)\]')


def fn_at(file, line):
    """name of the innermost `fn` starting at or above `line` (1-based), with its line."""
    s = src(file)
    for i in range(min(line, len(s)) - 1, -1, -1):
        m = FN_RE.match(s[i])
        if m:
            return m.group(1), i + 1
    return '?', 0


def test_mod_start(file):
    s = src(file)
    for i, l in enumerate(s):
        if TEST_RE.match(l) and i + 1 < len(s) and re.match(r'\s*mod\s', s[i + 1]):
            return i + 1
    return 10 ** 9


def collect(d):
    """per file: line->count (max over regions starting... use segments), branches, functions"""
    fns = {}      # (file, startline, startcol) -> dict(name, count, regions{(l,c,le,ce,kind)->count})
    brs = {}      # (file, l, c, le, ce) -> [true, false]
    for f in d['functions']:
        fname = rel(f['filenames'][0])
        if not FOCUS.search(fname):
            continue
        regs = f['regions']
        if not regs:
            continue
        r0 = regs[0]
        # the function's own file is fileId 0
        key = (fname, r0[0], r0[1])
        e = fns.setdefault(key, {'mangled': f['name'], 'count': 0, 'regions': collections.Counter(),
                                 'end': r0[2]})
        e['count'] += f['count']
        for r in regs:
            if r[5] != 0 or r[7] != 0:   # other files (macros) / non-code regions
                continue
            e['regions'][(r[0], r[1], r[2], r[3])] += r[4]
        for b in f.get('branches', []):
            if b[6] != 0:
                continue
            k = (fname, b[0], b[1], b[2], b[3])
            v = brs.setdefault(k, [0, 0])
            v[0] += b[4]
            v[1] += b[5]
    return fns, brs


def line_cov(d):
    """file -> {line: covered?} from segments (llvm-cov's own line rule approximated)"""
    out = {}
    for f in d['files']:
        fname = rel(f['filename'])
        if not FOCUS.search(fname):
            continue
        s = f['summary']
        out[fname] = s
    return out


def main():
    global ROOT
    cj, tj, outdir = sys.argv[1:4]
    ROOT = os.environ.get('WT', '.')
    os.makedirs(outdir, exist_ok=True)
    C, T = load(cj), load(tj)
    cs, ts = line_cov(C), line_cov(T)
    cf, cb = collect(C)
    tf, tb = collect(T)

    def pct(s, k):
        x = s.get(k, {})
        return f"{x.get('covered', 0)}/{x.get('count', 0)}"

    with open(f'{outdir}/files.tsv', 'w') as o:
        o.write('file\tcorpus_lines\ttests_lines\tcorpus_regions\ttests_regions\tcorpus_branches\ttests_branches\tcorpus_fns\ttests_fns\n')
        for fn in sorted(set(cs) | set(ts)):
            a, b = cs.get(fn, {}), ts.get(fn, {})
            o.write('\t'.join([fn] + [pct(x, k) for k in ('lines', 'regions', 'branches', 'functions') for x in (a, b)]) + '\n')

    # per file, test modules excluded, over the union of functions either build has
    def summ(fns, brs, file):
        tm = test_mod_start(file)
        F = [(k, v) for k, v in fns.items() if k[0] == file and k[1] < tm]
        nf, ef = len(F), sum(1 for k, v in F if v['count'] > 0)
        nr = sum(len(v['regions']) for k, v in F)
        cr = sum(1 for k, v in F for n in v['regions'].values() if n > 0)
        B = [(k, v) for k, v in brs.items() if k[0] == file and k[1] < tm]
        nb = 2 * len(B)
        cb_ = sum((v[0] > 0) + (v[1] > 0) for k, v in B)
        return (ef, nf, cr, nr, cb_, nb)

    with open(f'{outdir}/files_notest.tsv', 'w') as o:
        o.write('file\tfns_corpus\tfns_tests\tregions_corpus\tregions_tests\tbranch_sides_corpus\tbranch_sides_tests\n')
        files = sorted({k[0] for k in cf} | {k[0] for k in tf})
        for file in files:
            a, b = summ(cf, cb, file), summ(tf, tb, file)
            cell = lambda x, i: f"{x[i]}/{x[i+1]} ({100*x[i]/x[i+1]:.1f}%)" if x[i + 1] else "0/0"
            o.write('\t'.join([file] + [cell(x, i) for i in (0, 2, 4) for x in (a, b)]) + '\n')

    def fname_of(key):
        file, l, c = key
        n, _ = fn_at(file, l)
        return n

    # functions: corpus never enters
    with open(f'{outdir}/fn_uncovered.tsv', 'w') as o:
        o.write('file\tline\tfn\tcorpus_entries\ttests_entries\tin_test_mod\tlines\n')
        for key in sorted(cf):
            file, l, c = key
            if cf[key]['count'] == 0:
                tm = l >= test_mod_start(file)
                tc = tf.get(key, {}).get('count', 'n/a')
                o.write(f"{file}\t{l}\t{fname_of(key)}\t0\t{tc}\t{int(tm)}\t{cf[key]['end'] - l + 1}\n")
        # functions only in the tests' build (not even compiled into corpus binaries)
        for key in sorted(set(tf) - set(cf)):
            file, l, c = key
            if l >= test_mod_start(file):
                continue
            o.write(f"{file}\t{l}\t{fname_of(key)}\tnot-in-binary\t{tf[key]['count']}\t0\t{tf[key]['end'] - l + 1}\n")

    # regions never run, inside entered functions
    with open(f'{outdir}/rg_uncovered.tsv', 'w') as o:
        o.write('file\tline\tcol\tend_line\tfn\ttests_count\tsource\n')
        for key in sorted(cf):
            file, l, c = key
            if cf[key]['count'] == 0 or l >= test_mod_start(file):
                continue
            treg = tf.get(key, {}).get('regions', {})
            for r, n in sorted(cf[key]['regions'].items()):
                if n == 0:
                    s = src(file)
                    text = s[r[0] - 1].strip()[:110] if r[0] - 1 < len(s) else ''
                    o.write(f"{file}\t{r[0]}\t{r[1]}\t{r[2]}\t{fn_at(file, r[0])[0]}\t{treg.get(r, 'n/a')}\t{text}\n")

    # branches with a side never taken (corpus), with the tests' counts
    with open(f'{outdir}/br_uncovered.tsv', 'w') as o:
        o.write('file\tline\tcol\tfn\tcorpus_true\tcorpus_false\ttests_true\ttests_false\tsource\n')
        for k in sorted(cb):
            file, l, c, le, ce = k
            if l >= test_mod_start(file):
                continue
            t, f = cb[k]
            if t and f:
                continue
            tt = tb.get(k, ['n/a', 'n/a'])
            s = src(file)
            text = s[l - 1].strip()[:110] if l - 1 < len(s) else ''
            o.write(f"{file}\t{l}\t{c}\t{fn_at(file, l)[0]}\t{t}\t{f}\t{tt[0]}\t{tt[1]}\t{text}\n")

    # hot spots
    hot = sorted(((t + f, k, t, f) for k, (t, f) in cb.items()), reverse=True)
    with open(f'{outdir}/hot_branches.tsv', 'w') as o:
        o.write('file\tline\tfn\ttotal\ttrue\tfalse\tsource\n')
        for tot, k, t, f in hot[:400]:
            file, l = k[0], k[1]
            s = src(file)
            text = s[l - 1].strip()[:110] if l - 1 < len(s) else ''
            o.write(f"{file}\t{l}\t{fn_at(file, l)[0]}\t{tot}\t{t}\t{f}\t{text}\n")
    hf = sorted(((v['count'], k) for k, v in cf.items()), reverse=True)
    with open(f'{outdir}/hot_fns.tsv', 'w') as o:
        o.write('file\tline\tfn\tentries\n')
        for n, k in hf[:300]:
            o.write(f"{k[0]}\t{k[1]}\t{fname_of(k)}\t{n}\n")
    print('done', outdir)


if __name__ == '__main__':
    main()
