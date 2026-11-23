#!/usr/bin/env python3
"""Writes the per-module tables of research/aeneas/SURVEY.md from the survey's results.

    python3 research/aeneas/survey_report.py OUT_DIR... > research/aeneas/survey-modules.md

Each OUT_DIR is a tools/aeneas/survey.sh output (modules.tsv, crates.tsv) or a
tools/aeneas/survey-modules.sh output (survey.tsv). A module run on its own (survey-modules.sh)
wins over the crate-level attribution. The static columns are counted from the source itself.
"""
import os, re, sys, glob, collections

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# What each module is for, and which PROOF-STANDARD feature (and spec session) its L4 serves.
# H: a feature with an L1-L3 model in progress or done; M: a feature in the table without one
# yet; L: not a geometric feature (I/O, drawing, tooling).
VALUE = {
    "forge-math/predicates": ("H", "sign predicates (ieee: L5 done; L4 pilot here)"),
    "forge-math/dmath": ("H", "deterministic math (ieee: dmath, rem_pio2 model)"),
    "forge-math/interval": ("H", "certified intervals (bendsim, trace: Krawczyk)"),
    "forge-math/frame": ("H", "rigid transforms (fundamentals: L1 L3)"),
    "forge-math/vec": ("H", "vectors under every model"),
    "forge-math/poly": ("M", "polynomial roots (trace, closed forms)"),
    "forge-math/linalg": ("M", "small linear solves (sketch solver, fits)"),
    "forge-math/tol": ("M", "tolerances (every check)"),
    "forge-topo/check": ("H", "validity check (fundamentals: L1 L3 check_sound)"),
    "forge-topo/mass": ("H", "mass properties (fundamentals: L1 rules exact)"),
    "forge-topo/body": ("H", "the B-rep itself: every feature"),
    "forge-topo/uv": ("M", "parameter-space loops (tessellation, booleans)"),
    "forge-topo/name": ("M", "persistent names (edits, references)"),
    "forge-mesh/tessellate": ("H", "tessellation (fundamentals: closed, oriented, chord sag)"),
    "forge-mesh/cdt": ("H", "constrained Delaunay (tessellation)"),
    "forge-mesh/mesh": ("M", "mesh type (mesh export)"),
    "forge-mesh/stl": ("M", "mesh export (mesh-export)"),
    "forge-sketch/solve": ("M", "sketch solver (fundamentals item 5)"),
    "forge-sketch/profile": ("M", "sketch profiles (every extrude)"),
    "forge-bool/boolean": ("H", "boolean (decide-once: L1-L3; core in forge-bool-core)"),
    "forge-bool/classify": ("H", "boolean classification"),
    "forge-bool/decisions": ("H", "boolean decisions (decide-once's ledger)"),
    "forge-bool/unify": ("H", "boolean unify"),
    "forge-bool/nary": ("H", "n-ary boolean"),
    "forge-bool/imprint": ("H", "boolean imprint"),
    "forge-bool/ssi": ("H", "surface intersection (trace)"),
    "forge-bool/shape": ("M", "boolean shape helpers"),
    "forge-geom/spline": ("H", "splines (spline-spec)"),
    "forge-geom/blend": ("H", "fillets / rolling-ball blends (fillet-spec)"),
    "forge-geom/varblend": ("H", "variable fillets (fillet-spec)"),
    "forge-geom/corner": ("H", "fillet corners (fillet-spec, corner-patch)"),
    "forge-geom/intersect": ("H", "intersection tracing (trace)"),
    "forge-geom/surface": ("H", "surfaces: every feature"),
    "forge-geom/curve": ("H", "curves: every feature"),
    "forge-geom/intcurve": ("H", "intersection curves (trace)"),
    "forge-geom/meet": ("H", "surface meets (booleans, trace)"),
    "forge-geom/screw": ("M", "coil / thread surfaces (sweep-spec)"),
    "forge-geom/skinned": ("M", "lofts (sweep-spec)"),
    "forge-geom/loftmeet": ("M", "loft meets (sweep-spec)"),
    "forge-geom/tube": ("M", "pipes / sweeps (sweep-spec)"),
    "forge-geom/ruled": ("M", "ruled surfaces (sweep-spec, draft)"),
    "forge-geom/sheet": ("M", "sheet surfaces (surfaces session)"),
    "forge-ops/fillet": ("H", "fillets (fillet-spec)"),
    "forge-ops/chamfer": ("M", "chamfer (ops-spec)"),
    "forge-ops/shell": ("M", "shell (ops-spec)"),
    "forge-ops/draft": ("M", "draft (ops-spec)"),
    "forge-ops/extrude": ("M", "extrude (sweep-spec)"),
    "forge-ops/revolve": ("M", "revolve (sweep-spec)"),
    "forge-ops/sweep": ("M", "sweep (sweep-spec)"),
    "forge-ops/loft": ("M", "loft (sweep-spec)"),
    "forge-ops/glyph": ("M", "sketch text (sketch-text)"),
    "forge-sheet/region": ("H", "sheet regions (sheet-spec: L1 fold)"),
    "forge-sheet/form": ("H", "sheet forming (sheet-spec)"),
    "forge-sheet/bendline": ("H", "bends (sheet-spec)"),
    "forge-sheet/build": ("M", "sheet build (sheet-spec)"),
    "forge-sheet/graph": ("M", "sheet graph (sheet-spec: refold checker)"),
    "forge-model/pattern": ("M", "patterns (fundamentals)"),
    "forge-model/hole": ("M", "holes (ops-spec)"),
    "forge-model/rib": ("M", "ribs (ops-spec)"),
    "forge-model/thread": ("M", "threads (sweep-spec)"),
    "forge-model/upto": ("M", "up-to extents (upto-proofs)"),
    "forge-step/read": ("M", "STEP import (import-robust)"),
    "forge-step/write": ("M", "STEP export (import-robust)"),
    "forge-step/part21": ("M", "STEP parsing (import-robust)"),
    "forge-asm/check": ("M", "assembly interference (fix-mesh-asm)"),
    "forge-asm/solve": ("M", "assembly mates"),
}
CRATE_VALUE = {"forge-dwg": "L", "forge-draw": "L", "forge-thermo": "L", "forge-git": "L",
               "forge-dirs": "L", "forge-material": "L", "forge-alloc": "L", "forge-gpu": "L",
               "forge-font": "L", "forge-harness": "L", "forge-asm": "M", "forge-step": "M",
               "forge-model": "M"}

SHARED = re.compile(r"\b(Body|Face|Edge|Coedge|Shell|Curve|Surface|Solid|Feature|Part)\b")
STATIC = {
    "dyn": re.compile(r"\bdyn\s"),
    "rc": re.compile(r"\b(Rc|Arc|RefCell|Cell|OnceLock|OnceCell|Mutex|Cached)\s*<"),
    "hash": re.compile(r"\bHash(Map|Set)\b"),
    "iter": re.compile(r"\.(zip|map|filter|filter_map|flat_map|fold|collect|all|any|position|rposition|enumerate|rev|chain|sum|min_by|max_by|find)\("),
    "ord": re.compile(r"\bOrdering\b|\.cmp\(|partial_cmp|sort_by|max_by|min_by"),
    "unsafe": re.compile(r"\bunsafe\b"),
    "libm": re.compile(r"\.(sin|cos|tan|exp|ln|log2|log10|powf|powi|atan2|atan|asin|acos|sinh|cosh|tanh|cbrt|hypot|mul_add)\("),
}


def static_counts(path):
    try:
        src = open(os.path.join(ROOT, path), errors="replace").read()
    except OSError:
        return {}, 0, 0
    # drop #[cfg(test)] modules roughly: from "#[cfg(test)]\nmod" to the end
    m = re.search(r"#\[cfg\(test\)\]\s*\nmod\s+\w+\s*\{", src)
    body = src[:m.start()] if m else src
    c = {k: len(r.findall(body)) for k, r in STATIC.items()}
    return c, len(SHARED.findall(body)), body.count("\n")


def load(dirs):
    crate_status, rows, solo = {}, {}, {}
    for d in dirs:
        p = os.path.join(d, "crates.tsv")
        if os.path.exists(p):
            for l in open(p):
                f = l.rstrip("\n").split("\t")
                crate_status[f[0]] = f[1]
        p = os.path.join(d, "modules.tsv")
        if os.path.exists(p):
            for l in open(p):
                f = l.rstrip("\n").split("\t")
                if len(f) >= 9:
                    rows[f[1]] = dict(crate=f[0], file=f[1], defs=int(f[3]), body=int(f[4]),
                                      sig=int(f[5]), lean=int(f[6]), aerr=f[7], lkinds=f[8])
        p = os.path.join(d, "survey.tsv")
        if os.path.exists(p):
            for l in open(p):
                f = l.rstrip("\n").split("\t")
                if len(f) >= 9:
                    solo[f[2]] = dict(crate=f[0], mod=f[1], file=f[2], fns=int(f[4] or 0),
                                      failed=int(f[5] or 0), fatal=f[6], errs=f[7], lean=f[8])
    return crate_status, rows, solo


def key(path):
    m = re.match(r"kernel/([^/]+)/src/(.*)\.rs$", path)
    if not m:
        return None, None
    crate, rel = m.groups()
    rel = re.sub(r"/mod$", "", rel)
    top = rel.split("/")[0]
    return crate, top


def classify(path, crate_status, rows, solo):
    st, shared, lines = static_counts(path)
    s = solo.get(path)
    r = rows.get(path)
    crate, top = key(path)
    if s is not None:
        if s["fatal"] != "-":
            dyn = "tool: " + s["fatal"][:48]
        else:
            dyn = f"{s['fns']} fns, {s['failed']} bodies failed; " + (s["lean"][:60] if s["lean"] != "lean ok" else "Lean ok")
        lean_ok = s["lean"] == "lean ok" and s["fatal"] == "-"
        clean = lean_ok and s["failed"] == 0
    elif r is not None and crate_status.get(crate, "").startswith("ok"):
        dyn = f"{r['defs']} defs, {r['body']} bodies / {r['sig']} sigs failed, {r['lean']} Lean errors (crate run)"
        lean_ok = r["lean"] == 0 and r["defs"] > 0
        clean = lean_ok and r["body"] == 0 and r["sig"] == 0
    else:
        dyn = "crate run: " + crate_status.get(crate, "not run")[:50]
        lean_ok = clean = False
    if clean:
        cls = "A"
    elif shared > 0:
        cls = "C"
    elif s is not None and s["fatal"].startswith(("charon", "Fatal")):
        cls = "D"
    elif lean_ok or (st.get("dyn", 0) == 0 and st.get("rc", 0) == 0):
        cls = "B"
    else:
        cls = "C" if st.get("dyn", 0) or st.get("rc", 0) else "B"
    val = VALUE.get(f"{crate}/{top}", (CRATE_VALUE.get(crate, "M"), ""))
    return dict(path=path, lines=lines, st=st, shared=shared, dyn=dyn, cls=cls, value=val[0], what=val[1])


def main():
    crate_status, rows, solo = load(sys.argv[1:])
    files = sorted(glob.glob(os.path.join(ROOT, "kernel/*/src/**/*.rs"), recursive=True))
    files = [os.path.relpath(f, ROOT) for f in files if "/bin/" not in f]
    bycrate = collections.defaultdict(list)
    for f in files:
        c, _ = key(f)
        bycrate[c].append(classify(f, crate_status, rows, solo))
    out = []
    tot = collections.Counter()
    for c in sorted(bycrate):
        ms = bycrate[c]
        cnt = collections.Counter(m["cls"] for m in ms)
        tot.update(cnt)
        out.append(f"\n### {c} ({sum(m['lines'] for m in ms)} lines; crate run: {crate_status.get(c, 'not run')[:70]})\n")
        out.append("| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |")
        out.append("|---|---|---|---|---|---|---|---|---|---|---|---|")
        for m in ms:
            st = m["st"]
            name = m["path"].replace(f"kernel/{c}/src/", "")
            out.append(f"| `{name}` | {m['lines']} | {m['value']} | {m['cls']} | {m['shared']} | {st.get('dyn',0)} | {st.get('rc',0)} | "
                       f"{st.get('hash',0)} | {st.get('iter',0)} | {st.get('ord',0)} | {st.get('libm',0)} | {m['dyn'].replace('|','/')} |")
    print(f"<!-- generated by research/aeneas/survey_report.py; classes over {len(files)} modules: "
          f"{dict(sorted(tot.items()))} -->")
    print("\n".join(out))


if __name__ == "__main__":
    main()
