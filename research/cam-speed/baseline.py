#!/usr/bin/env python3
"""research/cam-speed: regenerate BASELINE.md (or another table) from the machining documents.

usage (from the checkout root):
  python3 research/cam-speed/baseline.py [--out research/cam-speed/BASELINE.md] [--reps N]
                                         [--outputs DIR] [--only SUBSTR] [--no-build]

Each document runs in a fresh process (`cam_bench`, forge-cam-lang's example) with
FORGE_CAM_PROF=1: cold, rerun unchanged, then the edits (last op's parameter, first op's, a
tool's stickout, the part's first number), each in that process as the app runs them. With
--reps N each document's process is run N times and each number is the least of them.
--outputs DIR keeps every cold reply (JSON and programs) for a byte comparison between builds.

Phases (seconds of wall, summed per kind of span; spans in forge-cam-lang/src/prof.rs):
  part     the part built and lowered (`part(..)`)
  setup    machine, tools and setups (`machine`, `tool`, `setup`, ..)
  pre      the 3D ops' generations made ahead side by side (Cam::prefetch); their statements
           then take them from the memo
  gen      each op's generation: its statement less its hops, feeds and stock update
  hops     links over the stock (hop_links)
  feeds    feeds from engagement (feeds_from_engagement)
  stock    the in-process stock updated with the op's voids
  post     the program statements' wall: lowering, certification, posting, round trip
  cert     inside post: certification of each posted path (certify_posted, G1-G3 and
           fixture) and G5, summed over the threads that ran it (can exceed post's wall)
  rt       inside post: the round trip (G6: print, read back, compare)
  g4       the leftover (G4)
  json     the reply's JSON (the app's run_unmeasured)
  other    the rest of the wall (parse, eval, ..)
"""
import argparse, json, os, subprocess, sys, hashlib

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

def docs():
    out = []
    for base in ["examples", "verify/forge-cam-quality/parts"]:
        for d, _, fs in os.walk(os.path.join(ROOT, base)):
            for f in fs:
                if not f.endswith(".fg"):
                    continue
                p = os.path.join(d, f)
                with open(p) as h:
                    first = next((l for l in h if l.strip() and not l.lstrip().startswith("#")), "")
                if first.strip() == "machining":
                    out.append(os.path.relpath(p, ROOT))
    return sorted(out)

def split(ds):
    """tune / held-out halves: by a hash of the path, fixed for good (not by speed)."""
    tune, held = [], []
    for d in ds:
        (tune if int(hashlib.sha256(d.encode()).hexdigest(), 16) % 2 == 0 else held).append(d)
    return tune, held

def phases(run):
    ph = {k: 0.0 for k in ["part", "setup", "pre", "gen", "hops", "feeds", "stock", "cert", "post", "rt", "g4"]}
    ops = {}
    stmt = {}
    sub = {}
    for name, s, _n in run["prof"]:
        kind, a, b = name.split("\t")
        if kind == "stmt":
            stmt[(a, b)] = stmt.get((a, b), 0.0) + s
        else:
            sub[(kind, b)] = sub.get((kind, b), 0.0) + s
            ph_k = {"hops": "hops", "feeds": "feeds", "stock": "stock", "certify": "cert", "axis": "cert",
                    "roundtrip": "rt", "leftover": "g4", "prefetch": "pre"}[kind]
            ph[ph_k] += s
            if kind in ("hops", "feeds", "stock", "certify", "axis"):
                o = ops.setdefault(b, {"kind": a})
                k2 = "cert" if kind in ("certify", "axis") else kind
                o[k2] = o.get(k2, 0.0) + s
    for (f, n), s in stmt.items():
        if f == "part":
            ph["part"] += s
        elif f == "program":
            ph["post"] += s
        elif f in ("machine", "tool", "setup", "fixture", "vise", "soft_jaw", "jaws", "holder", "probe"):
            ph["setup"] += s
        else:
            g = s - sum(sub.get((k, n), 0.0) for k in ("hops", "feeds", "stock"))
            ph["gen"] += g
            o = ops.setdefault(n, {"kind": f})
            o["gen"] = g
            o["stmt"] = s
    # certification runs on the pool: its spans are summed over threads (CPU-like), inside
    # the program's wall, as the round trip is
    ph["json"] = run["json"]
    ph["other"] = run["wall"] - sum(v for k, v in ph.items() if k not in ("cert", "rt"))
    return ph, ops

def bench(doc, outputs, env):
    args = [os.path.join(ROOT, "target/release/examples/cam_bench"), os.path.join(ROOT, doc)]
    if outputs:
        args.append(outputs)
    r = subprocess.run(args, capture_output=True, text=True, env=env, timeout=3600)
    line = [l for l in r.stdout.splitlines() if l.startswith("{")]
    if not line:
        return {"doc": doc, "error": (r.stderr or r.stdout)[-400:]}
    return json.loads(line[-1])

def least(a, b):
    """per run (by position), the run with the least wall."""
    if "error" in a:
        return b
    for i, (x, y) in enumerate(zip(a["runs"], b["runs"])):
        if y["wall"] < x["wall"]:
            a["runs"][i] = y
    return a

def f(x):
    return f"{x:.3f}" if x >= 0.0005 or x == 0 else f"{x*1000:.2f}ms"

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=os.path.join(ROOT, "research/cam-speed/BASELINE.md"))
    ap.add_argument("--json", default=None, help="also write the raw results here (JSON lines)")
    ap.add_argument("--reps", type=int, default=1)
    ap.add_argument("--outputs", default=None)
    ap.add_argument("--only", default=None)
    ap.add_argument("--no-build", action="store_true")
    ap.add_argument("--title", default="CAM speed baseline")
    ap.add_argument("--from-json", default=None, help="render the table from saved results (JSON lines)")
    ap.add_argument("--commit", default=None, help="the commit the saved results were made on")
    ap.add_argument("--note", default=None, help="a paragraph added under the header")
    a = ap.parse_args()
    if not a.no_build and not a.from_json:
        subprocess.run(["cargo", "build", "--release", "-q", "-p", "forge-cam-lang", "--example", "cam_bench"],
                       cwd=ROOT, check=True)
    if a.outputs:
        os.makedirs(a.outputs, exist_ok=True)
    env = dict(os.environ, FORGE_CAM_PROF="1", FORGECAD_CACHE="off")
    ds = docs()
    if a.only:
        ds = [d for d in ds if a.only in d]
    tune, held = split(docs())
    res = []
    if a.from_json:
        ds = []
        res = [json.loads(l) for l in open(a.from_json)]
    for d in ds:
        r = None
        for k in range(a.reps):
            x = bench(d, a.outputs if k == 0 else None, env)
            r = x if r is None else least(r, x)
        res.append(r)
        c = r["runs"][0] if "runs" in r else None
        print(f"{d}: {c['wall']:.3f} s" if c else f"{d}: ERROR", file=sys.stderr)
    if a.json:
        with open(a.json, "w") as h:
            for r in res:
                h.write(json.dumps(r) + "\n")
    for r in res:
        r["doc"] = os.path.relpath(r["doc"], ROOT) if r["doc"].startswith("/") else r["doc"]
    rev = a.commit or subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
    nproc = os.cpu_count()
    L = []
    L.append(f"# {a.title}\n")
    L.append(f"Regenerate: `python3 research/cam-speed/baseline.py` (from the checkout root; "
             f"`--reps 3` keeps the least of 3). Commit {rev}, native release (x86-64-v3), "
             f"{nproc} cores, FORGECAD_CACHE=off, one fresh process per document. Seconds of wall unless said.\n")
    L.append("**cold**: `forge_cam_lang::run` + the reply's JSON (the app's `run_unmeasured`), first run in "
             "the process. **rerun**: the same text again. **edit**: one change, then run, in the same process "
             "(the app reruns the whole text on each edit): `last op` = the document's last op's stepover/step/"
             "leave/to/depth changed, `first op` = its first op's (every later op of the setup sees new stock), "
             "`tool` = the last tool's stickout +1, `part` = the part's first number x1.02. "
             "**cpu/wall** = CPU seconds over wall (1.0 = one core busy). **thr** = most threads alive.\n")
    if a.note:
        L.append(a.note + "\n")
    L.append("Split (fixed, by a hash of the path): **T** = tune on, **H** = held out (never tuned on).\n")
    L.append("## Per document\n")
    L.append("| doc | set | cold | cpu/wall | thr | rerun | edit last op | edit first op | edit tool | edit part | ok |")
    L.append("|---|---|---|---|---|---|---|---|---|---|---|")
    tot = 0.0
    for r in res:
        d = r["doc"].replace(ROOT + "/", "")
        name = os.path.relpath(d, ROOT) if d.startswith("/") else d
        sset = "T" if name in tune else "H"
        if "runs" not in r:
            L.append(f"| {name} | {sset} | ERROR | | | | | | | | |")
            continue
        runs = r["runs"]
        c = runs[0]
        tot += c["wall"]
        by = {x["what"].split(":")[0]: x for x in runs}
        def e(k):
            x = by.get(k)
            return f(x["wall"]) if x else "-"
        L.append(f"| {name} | {sset} | **{f(c['wall'])}** | {c['cpu']/max(c['wall'],1e-9):.2f} | {c['threads']} | "
                 f"{e('rerun')} | {e('edit_last_op')} | {e('edit_first_op')} | {e('edit_tool')} | {e('edit_part')} | {c['ok']} |")
    L.append(f"\nTotal cold: {tot:.2f} s over {len(res)} documents; "
             f"{sum(1 for r in res if 'runs' in r and r['runs'][0]['wall'] > 1.0)} over 1 s.\n")
    L.append("## Cold phases per document\n")
    L.append("`post` is the programs' wall; `cert` and `rt` are inside it, `cert` summed over the threads "
             "that ran it. `other` is the wall outside every span: parsing, evaluation, and in a fresh process "
             "the built-in library's first load. The turning documents have no spans (forge-turn-lang).\n")
    cols = ["part", "setup", "pre", "gen", "hops", "feeds", "stock", "post", "cert", "rt", "g4", "json", "other"]
    L.append("| doc | cold | " + " | ".join(cols) + " |")
    L.append("|---|---|" + "---|" * len(cols))
    for r in res:
        if "runs" not in r:
            continue
        name = r["doc"].replace(ROOT + "/", "")
        ph, _ = phases(r["runs"][0])
        L.append(f"| {name} | {f(r['runs'][0]['wall'])} | " + " | ".join(f(ph[k]) for k in cols) + " |")
    L.append("\n## Edit phases per document (where the edit's time goes)\n")
    L.append("| doc | edit | wall | " + " | ".join(cols) + " |")
    L.append("|---|---|---|" + "---|" * len(cols))
    for r in res:
        if "runs" not in r:
            continue
        name = r["doc"].replace(ROOT + "/", "")
        for x in r["runs"][1:]:
            ph, _ = phases(x)
            L.append(f"| {name} | {x['what']} | {f(x['wall'])} | " + " | ".join(f(ph[k]) for k in cols) + " |")
    L.append("\n## Cold, per op\n")
    L.append("gen = the op's statement less hops, feeds, stock; cert = its certification when posted (G1-G3, fixture, G5).\n")
    L.append("| doc | op | kind | gen | hops | feeds | stock | cert |")
    L.append("|---|---|---|---|---|---|---|---|")
    for r in res:
        if "runs" not in r:
            continue
        name = r["doc"].replace(ROOT + "/", "")
        _, ops = phases(r["runs"][0])
        for n, o in ops.items():
            L.append(f"| {name} | {n} | {o.get('kind','')} | {f(o.get('gen',0))} | {f(o.get('hops',0))} | "
                     f"{f(o.get('feeds',0))} | {f(o.get('stock',0))} | {f(o.get('cert',0))} |")
    with open(a.out, "w") as h:
        h.write("\n".join(L) + "\n")
    print(f"wrote {a.out}", file=sys.stderr)

if __name__ == "__main__":
    main()
