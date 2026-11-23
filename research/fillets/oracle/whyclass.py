# whyclass.py DIR: each filletwhy report summarised: error, alone/set, radius, shortest picked edge in the
# smallest set and at the blamed vertex (against the radius), the vertex's valence, surfaces met
import glob, os, re, sys
for f in sorted(glob.glob(os.path.join(sys.argv[1], '*.why'))):
    t = open(f).read()
    name = os.path.basename(f)[:-4]
    m = re.search(r'whole: Some\((\w+)', t)
    err = m.group(1) if m else ('timeout/none' if 'whole' not in t else 'built?')
    alone = re.search(r'(\d+) edges fail alone', t)
    sm = re.search(r'smallest refused set: (\d+) edges', t)
    rad = [float(x) for x in re.findall(r'Constant\(([\d.e-]+)\)', t)]
    r = max(rad) if rad else 0
    tail = t.split('smallest refused set')[-1] if 'smallest refused set' in t else ''
    picked = re.findall(r'\* Constant\([\d.e-]+\) e\d+ (\w+) .*?len~([\d.]+) (f\d+\[[^\]]+\]) (f\d+\[[^\]]+\])', tail)
    verts = re.findall(r'  vertex v\d+ [^\n]*\n((?:    [ *] e[^\n]*\n)+)', tail)
    val = [len(v.strip().splitlines()) for v in verts]
    lens = [float(x) for x in re.findall(r'len~([\d.]+)', tail)]
    short = min(lens) / r if lens and r else 0
    surf = sorted({s.split('[')[1].split()[0].rstrip(']') for p in picked for s in p[2:4]})
    kinds = sorted({p[0] for p in picked})
    print(f"{name:28s} {err:9s} alone={alone.group(1) if alone else '-':3s} set={sm.group(1) if sm else '-':3s} r={r:<7g} minlen/r={short:5.2f} valence={val} curves={kinds} surfs={surf}")
