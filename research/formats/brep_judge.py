#!/usr/bin/env python3
"""STEP, IGES and OpenCascade .brep: our reader against OCCT 8.0.1, file by file, by a rule fixed
before any result was looked at (research/formats/real-files.md, "B-rep").

  $VENV/bin/python -I brep_judge.py ref FILE                       OCCT's reading, by the truth rule
  $VENV/bin/python -I brep_judge.py run STEPREAD OUT.jsonl FILE.. [@LIST] [--jobs 3] [--timeout 120]
  $VENV/bin/python -I brep_judge.py write PROBE OUT.jsonl FILE..   the STEP writer: read, export, OCCT reads it
  $VENV/bin/python -I brep_judge.py report OUT.jsonl..

The truth is research/import/truth.md's rule (tools/find-parts/recreate/truth_rule.py on the
import branch, copied here so this runs on its own): per solid the Gauss-Kronrod volume, and the
area of whichever of the default and precise measures agrees with it; uncertain where neither
does. Ours is kernel/forge-step/examples/stepread.rs (what research/step/rate.py runs).
"""
import collections, concurrent.futures as cf, json, os, resource, subprocess, sys

MEM = 4 << 30
EPS = 1e-9
TOL = 1e-3  # rate.py's: research/import/robustness.md section 2 judged 3146 files by it


def limit():
    resource.setrlimit(resource.RLIMIT_AS, (MEM, MEM))
    os.nice(10)


def measures(solid):
    from OCP.BRepGProp import BRepGProp
    from OCP.GProp import GProp_GProps

    def one(f, *a):
        g = GProp_GProps()
        f(solid, g, *a)
        return g.Mass()

    return {
        "vol": abs(one(BRepGProp.VolumeProperties_s)),
        "area": one(BRepGProp.SurfaceProperties_s),
        "vol_eps": abs(one(BRepGProp.VolumeProperties_s, EPS)),
        "area_eps": one(BRepGProp.SurfaceProperties_s, EPS),
        "vol_gk": abs(one(BRepGProp.VolumePropertiesGK_s, EPS, False, True)),
    }


def judged(t, tol):
    v, near = t["vol_gk"], tol / 10
    agree = lambda x: abs(x - v) <= near * max(v, 1e-12)
    cand = [a for a, w in ((t["area_eps"], t["vol_eps"]), (t["area"], t["vol"])) if agree(w)]
    if not cand:
        return None
    if len(cand) == 2 and abs(cand[0] - cand[1]) > near * max(abs(cand[0]), 1e-12):
        return None
    return v, cand[0]


def read_shape(path):
    from OCP.IFSelect import IFSelect_RetDone

    low = path.lower()
    if low.endswith((".brep", ".brp")):
        from OCP.BRep import BRep_Builder
        from OCP.BRepTools import BRepTools
        from OCP.TopoDS import TopoDS_Shape

        s = TopoDS_Shape()
        return s if BRepTools.Read_s(s, path, BRep_Builder()) else None
    if low.endswith((".igs", ".iges")):
        from OCP.IGESControl import IGESControl_Reader as R
    else:
        from OCP.STEPControl import STEPControl_Reader as R
    r = R()
    if r.ReadFile(path) != IFSelect_RetDone:
        return None
    r.TransferRoots()
    return r.OneShape()


def ref(path):
    from OCP.BRepCheck import BRepCheck_Analyzer
    from OCP.TopAbs import TopAbs_FACE, TopAbs_SHELL, TopAbs_SOLID
    from OCP.TopExp import TopExp_Explorer
    from OCP.TopoDS import TopoDS

    s = read_shape(path)
    if s is None or s.IsNull():
        return {"err": "unreadable"}

    def n(k):
        e, c = TopExp_Explorer(s, k), 0
        while e.More():
            c += 1
            e.Next()
        return c

    sums = dict.fromkeys(["vol", "area", "vol_eps", "area_eps", "vol_gk"], 0.0)
    ex, solids = TopExp_Explorer(s, TopAbs_SOLID), 0
    while ex.More():
        m = measures(TopoDS.Solid(ex.Current()))
        for k in sums:
            sums[k] += m[k]
        solids += 1
        ex.Next()
    out = {"valid": bool(BRepCheck_Analyzer(s).IsValid()), "solids": solids, "shells": n(TopAbs_SHELL), "faces": n(TopAbs_FACE)}
    out.update(sums)
    return out


def child(cmd, timeout, stdin=None):
    try:
        p = subprocess.run(cmd, input=stdin, capture_output=True, timeout=timeout, preexec_fn=limit)
    except subprocess.TimeoutExpired:
        return {"crash": "timeout"}
    for l in reversed(p.stdout.decode("utf8", "replace").strip().splitlines()):
        try:
            d = json.loads(l)
            return {"crash": "panic", "panic": d["panic"]} if "panic" in d else d
        except json.JSONDecodeError:
            continue
    err = p.stderr.decode("utf8", "replace")
    at = [l for l in err.splitlines() if "panicked" in l or "memory allocation" in l or "overflow" in l]
    return {"crash": f"exit {p.returncode}", "panic": (at[0] if at else err[-200:])[:200]}


def verdict(ours, t):
    """The rule, in order:
    crash / timeout: ours did not end in a value or an error;
    ref-none: OCCT reads no solid; then 'both-none' if neither do, else 'extra' (ours made a solid);
    refused: OCCT reads a solid, ours none;
    uncertain: the truth rule can't tell OCCT's own volume;
    invalid: ours has check problems, or warned (a body it could not check), or no mass;
    match: the sums of |volume| and of area within 1e-3 of the truth (and 'match6' within 1e-6);
    mismatch: otherwise."""
    if "crash" in ours:
        return ("timeout" if ours["crash"] == "timeout" else "crash"), [ours.get("panic", ours["crash"])]
    mine = ours.get("solids") or []
    if "crash" in t or "err" in t or not t.get("solids"):
        return ("both-none" if not mine else "extra"), [t.get("err") or t.get("crash") or "OCCT: no solid"]
    if "err" in ours or not mine:
        return "refused", [ours.get("err") or (ours.get("why") or "; ".join(ours.get("refusals", {})))[:200] or "no solid"]
    j = judged(t, TOL)
    if j is None:
        return "uncertain", ["OCCT's measures disagree"]
    probs = collections.Counter()
    for x in mine:
        for k, n in x["problems"].items():
            probs[k] += n
    if probs:
        return "invalid", [",".join(sorted(probs))]
    if any(x.get("warn") for x in mine):
        return "invalid", ["warned: " + mine[0]["warn"][:80]]
    if any(x["vol"] is None or x["area"] is None for x in mine):
        return "invalid", ["no mass: " + mine[0].get("merr", "")[:80]]
    v, a = sum(abs(x["vol"]) for x in mine), sum(x["area"] for x in mine)
    dv, da = (v - j[0]) / j[0], (a - j[1]) / j[1]
    why = [f"dvol {dv:.2e} darea {da:.2e} solids {len(mine)} vs {t['solids']}"]
    if abs(dv) <= 1e-6 and abs(da) <= 1e-6:
        return "match6", why
    if abs(dv) <= TOL and abs(da) <= TOL:
        return "match", why
    return "mismatch", why


def files(args):
    out = []
    for a in args:
        out += [l.strip() for l in open(a[1:]) if l.strip()] if a.startswith("@") else [a]
    return out


def opt(args, name, default):
    if name in args:
        i = args.index(name)
        v = args[i + 1]
        del args[i : i + 2]
        return type(default)(v)
    return default


def run(argv):
    jobs, timeout = opt(argv, "--jobs", 3), opt(argv, "--timeout", 120)
    stepread, out, fs = argv[0], argv[1], files(argv[2:])
    me = os.path.abspath(__file__)

    def one(f):
        ours = child([stepread, "--no-box"], timeout, stdin=open(f, "rb").read())
        t = child([sys.executable, "-I", me, "ref", f], timeout * 3)
        v, why = verdict(ours, t)
        return {"file": f, "verdict": v, "why": why, "ours": ours, "ref": t}

    # resumable: files already judged in OUT are skipped, new rows appended
    done = set()
    if os.path.exists(out):
        for l in open(out):
            try:
                done.add(json.loads(l)["file"])
            except Exception:
                pass
    fs = [f for f in fs if f not in done]
    with open(out, "a") as o, cf.ThreadPoolExecutor(jobs) as ex:
        for r in ex.map(one, fs):
            o.write(json.dumps(r) + "\n")
            o.flush()


def write(argv):
    """The STEP writer. Each solid ours reads is exported (forge_step::export, default options)
    and the file read by OCCT, which must find one valid solid whose truth-rule volume and area are
    our own body's to 1e-4 (the exporter fits procedural surfaces within 1e-4 mm): 'match';
    'uncertain' where the truth rule can't tell; else 'mismatch', 'ref-refused' or 'invalid'."""
    jobs, timeout = opt(argv, "--jobs", 3), opt(argv, "--timeout", 120)
    probe, out, fs = argv[0], argv[1], files(argv[2:])
    me = os.path.abspath(__file__)
    tmp = out + ".d"
    os.makedirs(tmp, exist_ok=True)

    def one(k_f):
        k, f = k_f
        w = child([probe, "step-rewrite", f, os.path.join(tmp, str(k))], timeout)
        if "crash" in w or "err" in w:
            return [{"file": f, "verdict": "not-read", "why": [w.get("err") or w.get("panic") or w["crash"]]}]
        rows = []
        for path, mine in zip(w["files"], [s for s in w["solids"] if "err" not in s]):
            t = child([sys.executable, "-I", me, "ref", path], timeout * 3)
            os.remove(path)
            r = {"file": f, "written": os.path.basename(path), "ours": mine, "ref": t}
            if "err" in t or "crash" in t or t.get("solids") != 1:
                r["verdict"], r["why"] = "ref-refused", [t.get("err") or t.get("crash") or f"{t.get('solids')} solids"]
            elif not t["valid"]:
                r["verdict"], r["why"] = "invalid", ["OCCT: not valid"]
            elif mine["vol"] is None:
                r["verdict"], r["why"] = "no-mass", [mine["merr"]]
            else:
                j = judged(t, 1e-4)
                if j is None:
                    r["verdict"], r["why"] = "uncertain", []
                else:
                    dv, da = (abs(mine["vol"]) - j[0]) / j[0], (mine["area"] - j[1]) / j[1]
                    ok = abs(dv) <= 1e-4 and abs(da) <= 1e-4
                    r["verdict"], r["why"] = ("match" if ok else "mismatch"), [f"dvol {dv:.2e} darea {da:.2e}"]
            rows.append(r)
        for s in w["solids"]:
            if "err" in s:
                rows.append({"file": f, "verdict": "not-written", "why": [s["err"]]})
        return rows

    with open(out, "w") as o, cf.ThreadPoolExecutor(jobs) as ex:
        for rows in ex.map(one, enumerate(fs)):
            for r in rows:
                o.write(json.dumps(r) + "\n")
            o.flush()


def report(paths):
    for p in paths:
        rows = [json.loads(l) for l in open(p)]
        c = collections.Counter(r["verdict"] for r in rows)
        print(f"== {p}: {len(rows)}")
        for k, n in c.most_common():
            print(f"  {k:12} {n}")
        causes = collections.defaultdict(collections.Counter)
        for r in rows:
            if not r["verdict"].startswith("match"):
                causes[r["verdict"]][str(r["why"][0])[:90] if r["why"] else ""] += 1
        for k, cc in causes.items():
            print(f"  -- {k}")
            for w, n in cc.most_common(8):
                print(f"     {n:5}  {w}")


if __name__ == "__main__":
    m, rest = sys.argv[1], sys.argv[2:]
    if m == "ref":
        print(json.dumps(ref(rest[0])))
    elif m in ("run", "write", "report"):
        {"run": run, "write": write, "report": report}[m](rest)


def write_fg(argv):
    """The STEP writer on the repository's own parts: `forgecad export F.fg` (the part's body, as
    a user exports it), read by OCCT, held to the part's own volume and area (`query F stat`'s
    part: every body, a sheet's flat pattern too; the first run took `build`'s physical, which
    counts only physical solids, and was corrected) to 1e-4, the truth rule deciding OCCT's measure. A part with no solid is skipped."""
    jobs, timeout = opt(argv, "--jobs", 3), opt(argv, "--timeout", 300)
    cli, out, fs = argv[0], argv[1], files(argv[2:])
    me = os.path.abspath(__file__)
    tmp = out + ".d"
    os.makedirs(tmp, exist_ok=True)

    def one(k_f):
        k, f = k_f
        w = os.path.join(tmp, f"{k}.step")
        e = child([cli, "export", f, "-o", w, "--json"], timeout)
        if not e.get("ok"):
            return {"file": f, "verdict": "no-solid" if "no solid" in str(e) else "not-exported", "why": [str(e.get("error") or e)[:160]]}
        b = child([cli, "query", f, "stat", "--json"], timeout)
        ph = b.get("part") or {}
        t = child([sys.executable, "-I", me, "ref", w], timeout * 3)
        os.remove(w)
        r = {"file": f, "solids": e.get("solids"), "ours": ph and {"vol": ph.get("volume"), "area": ph.get("area")}, "ref": t}
        if "err" in t or "crash" in t or not t.get("solids"):
            r["verdict"], r["why"] = "ref-refused", [t.get("err") or t.get("crash") or "OCCT: no solid"]
        elif not t["valid"]:
            r["verdict"], r["why"] = "invalid", ["OCCT: not valid"]
        elif not ph.get("volume"):
            r["verdict"], r["why"] = "no-mass", ["build reports no volume"]
        else:
            j = judged(t, 1e-4)
            if j is None:
                r["verdict"], r["why"] = "uncertain", []
            else:
                dv, da = (ph["volume"] - j[0]) / j[0], (ph["area"] - j[1]) / j[1]
                r["verdict"] = "match" if abs(dv) <= 1e-4 and abs(da) <= 1e-4 else "mismatch"
                r["why"] = [f"dvol {dv:.2e} darea {da:.2e} solids {e.get('solids')} vs {t['solids']}"]
        return r

    with open(out, "w") as o, cf.ThreadPoolExecutor(jobs) as ex:
        for r in ex.map(one, enumerate(fs)):
            o.write(json.dumps(r) + "\n")
            o.flush()


if __name__ == "__main__" and sys.argv[1] == "write-fg":
    write_fg(sys.argv[2:])
