#!/usr/bin/env python3
"""TrueType/OpenType fonts and SHX shape fonts: our readers against independent ones, font by
font, by a rule fixed before any result was looked at (research/formats/real-files.md, "Fonts").

  $VENV/bin/python -I font_judge.py run PROBE OUT.jsonl FILE.. [@LIST] [--jobs 3] [--timeout 120]
  $VENV/bin/python -I font_judge.py shx PROBE OUT.jsonl FILE..
  $VENV/bin/python -I font_judge.py report OUT.jsonl..

TrueType: fontTools (FreeType's reference parser in Python; the outlines are its glyf decoding).
For every code point of the BMP in fontTools' best cmap that names a glyph other than .notdef:
the glyph id; the advance (hmtx, exact); and the outline drawn through fontTools' glyph set
(composites resolved): its contour count (contours with a segment: a one-point contour, an
anchor, draws nothing; counted so after the first font met one), signed area (AreaPen, against ours by Green's theorem,
exact for lines and quadratics) to 1e-9 of the glyph's em-square scale, and exact bounds
(BoundsPen) to 1e-6 units. A glyph ours could not outline counts as a mismatch. CFF and CFF2
outlines are refused by design (FontError::Cff): counted apart. forge-dwg's TtfFont must take
every font forge-font takes.

SHX: ezdxf's shape-file reader (ezdxf.fonts.shapefile), the only independent one there is: the
set of shape numbers, and each shape's pen end (its advance) and stroke box, to 1e-9 of the
font's height. (Arc bulges are flattened differently by the two, so the box is held to the
flattening's chord error, 1% of the height.)
"""
import collections, concurrent.futures as cf, json, os, resource, subprocess, sys

MEM = 4 << 30


def limit():
    resource.setrlimit(resource.RLIMIT_AS, (MEM, MEM))


def child(cmd, timeout):
    try:
        p = subprocess.run(cmd, capture_output=True, timeout=timeout, preexec_fn=limit)
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


def cross(a, b):
    return a[0] * b[1] - a[1] * b[0]


def our_measures(contours):
    """Signed area and exact bounds of our segments."""
    area, lo, hi = 0.0, [float("inf")] * 2, [float("-inf")] * 2

    def take(p):
        for k in range(2):
            lo[k], hi[k] = min(lo[k], p[k]), max(hi[k], p[k])

    for c in contours:
        for s in c:
            if len(s) == 4:
                p0, p2 = (s[0], s[1]), (s[2], s[3])
                area += cross(p0, p2) / 2
                take(p0), take(p2)
            else:
                p0, p1, p2 = (s[0], s[1]), (s[2], s[3]), (s[4], s[5])
                area += (2 * cross(p0, p1) + 2 * cross(p1, p2) + cross(p0, p2)) / 6
                take(p0), take(p2)
                for k in range(2):
                    d = p0[k] - 2 * p1[k] + p2[k]
                    if d != 0:
                        t = (p0[k] - p1[k]) / d
                        if 0 < t < 1:
                            take(tuple((1 - t) ** 2 * p0[j] + 2 * t * (1 - t) * p1[j] + t * t * p2[j] for j in range(2)))
    return area, (lo + hi if lo[0] != float("inf") else None)


def ref_font(path):
    from fontTools.pens.areaPen import AreaPen
    from fontTools.pens.boundsPen import BoundsPen
    from fontTools.pens.recordingPen import DecomposingRecordingPen
    from fontTools.ttLib import TTFont

    f = TTFont(path, lazy=True, fontNumber=0)
    cff = "CFF " in f or "CFF2" in f
    order = f.getGlyphOrder()
    gid = {n: i for i, n in enumerate(order)}
    gs = f.getGlyphSet()
    hmtx = f["hmtx"].metrics
    out = {"cff": cff, "upem": f["head"].unitsPerEm, "cp": {}}
    for cp, name in (f.getBestCmap() or {}).items():
        if cp > 0xFFFF or gid.get(name, 0) == 0:
            continue
        a, b, r = AreaPen(gs), BoundsPen(gs), DecomposingRecordingPen(gs)
        gs[name].draw(a)
        gs[name].draw(b)
        gs[name].draw(r)
        # contours that draw something: a one-point contour (an anchor) has no segment
        contours, segs = 0, 0
        for op, _ in r.value:
            if op == "moveTo":
                segs = 0
            elif op in ("lineTo", "qCurveTo", "curveTo"):
                segs += 1
            elif op in ("closePath", "endPath"):
                contours += segs > 0
        out["cp"][str(cp)] = {"g": gid[name], "adv": hmtx[name][0], "area": a.value, "box": list(b.bounds) if b.bounds else None, "contours": contours}
    return out


def verdict(ours, t):
    if "crash" in ours:
        return ("timeout" if ours["crash"] == "timeout" else "crash"), [ours.get("panic", ours["crash"])], 0
    if "crash" in t or "err" in t:
        return ("both-refused" if "err" in ours else "ref-refused"), [str(t.get("err") or t.get("crash"))], 0
    if "err" in ours:
        if ours["err"] == "Cff" and t["cff"]:
            return "refused-cff", ["CFF outlines"], 0
        return "refused", [ours["err"]], 0
    if ours.get("dwg") is not True:
        return "mismatch", [f"forge-dwg refuses what forge-font takes: {ours.get('dwg')}"], 0
    scale = t["upem"]
    bad = collections.Counter()
    first = {}
    mine, theirs = ours["cp"], t["cp"]
    for cp in set(mine) | set(theirs):
        a, b = mine.get(cp), theirs.get(cp)
        why = None
        if a is None or b is None:
            why = "mapped by one only"
        elif a["g"] != b["g"]:
            why = f"glyph {a['g']} vs {b['g']}"
        elif a["adv"] != b["adv"]:
            why = f"advance {a['adv']} vs {b['adv']}"
        elif "err" in a:
            why = f"outline refused: {a['err']}"
        else:
            area, box = our_measures(a["c"])
            n = sum(1 for c in a["c"] if c)
            if n != b["contours"]:
                why = f"contours {n} vs {b['contours']}"
            elif abs(area - b["area"]) > 1e-9 * scale * scale:
                why = f"area {area} vs {b['area']}"
            elif (box is None) != (b["box"] is None) or (box and any(abs(x - y) > 1e-6 for x, y in zip(box, b["box"]))):
                why = f"box {box} vs {b['box']}"
        if why:
            k = why.split(" ")[0] if not why.startswith("outline") else why[:60]
            bad[k] += 1
            first.setdefault(k, f"U+{int(cp):04X}: {why}"[:160])
    n = len(set(mine) | set(theirs))
    if not bad:
        return "match", [f"{n} code points"], n
    return "mismatch", [f"{sum(bad.values())} of {n} code points: " + ", ".join(f"{k} {v}" for k, v in bad.most_common())] + list(first.values())[:4], n


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
    probe, out, fs = argv[0], argv[1], files(argv[2:])
    me = os.path.abspath(__file__)

    def one(f):
        ours = child([probe, "font", f], timeout)
        t = child([sys.executable, "-I", me, "ref", f], timeout * 3)
        v, why, n = verdict(ours, t)
        return {"file": f, "verdict": v, "why": why, "cps": n}

    with open(out, "w") as o, cf.ThreadPoolExecutor(jobs) as ex:
        for r in ex.map(one, fs):
            o.write(json.dumps(r) + "\n")
            o.flush()


def ref_shx(path):
    from ezdxf.fonts import shapefile

    data = open(path, "rb").read()
    f = shapefile.readfile(path) if hasattr(shapefile, "readfile") else shapefile.shx_loads(data)
    out = {"height": None, "run": {}}
    try:
        out["height"] = float(f.above + f.below) if hasattr(f, "above") else None
    except Exception:
        pass
    for n in sorted(f.shapes):
        try:
            p = f.render_shape(n)
        except Exception as e:
            out["run"][str(n)] = {"err": str(e)[:80]}
            continue
        # the strokes only: a pen-up move (the last, to the shape's end, often) draws nothing;
        # curves by their flattening at 1e-4 of the height
        pts = []
        for sp in p.sub_paths() if len(p) else []:
            if len(sp):
                pts += [(v.x, v.y) for v in sp.flattening(1e-4 * max(1.0, float(f.above + f.below)))]
        end = p.end
        box = [min(x for x, _ in pts), min(y for _, y in pts), max(x for x, _ in pts), max(y for _, y in pts)] if pts else None
        out["run"][str(n)] = {"end": [end.x, end.y], "box": box}
    return out


def shx(argv):
    jobs, timeout = opt(argv, "--jobs", 3), opt(argv, "--timeout", 120)
    probe, out, fs = argv[0], argv[1], files(argv[2:])
    me = os.path.abspath(__file__)
    with open(out, "w") as o:
        for f in fs:
            ours = child([probe, "shx", f], timeout)
            t = child([sys.executable, "-I", me, "refshx", f], timeout)
            if "crash" in ours or "err" in ours or "crash" in t or "err" in t:
                v = "crash" if "crash" in ours else "both-refused" if ("err" in ours and ("err" in t or "crash" in t)) else "refused" if "err" in ours else "ref-refused"
                o.write(json.dumps({"file": f, "verdict": v, "why": [str(ours.get("err") or ours.get("panic") or t.get("err") or t.get("panic"))[:200]]}) + "\n")
                continue
            h = t["height"] or ours.get("height") or 1.0
            bad, first = collections.Counter(), {}
            a, b = ours["run"], t["run"]
            for n in set(a) | set(b):
                x, y = a.get(n), b.get(n)
                why = None
                if x is None or y is None:
                    why = "shape in one only"
                elif "err" in y:
                    why = "reference could not run it"
                elif any(abs(p - q) > 1e-9 * h for p, q in zip(x["end"], y["end"])):
                    why = f"end {x['end']} vs {y['end']}"
                elif (x["box"][0] is None) != (y["box"] is None) or (y["box"] and any(abs(p - q) > 1e-2 * h for p, q in zip(x["box"], y["box"]))):
                    why = f"box {x['box']} vs {y['box']}"
                if why:
                    k = why.split(" ")[0]
                    bad[k] += 1
                    first.setdefault(k, f"shape {n}: {why}"[:160])
            v = "match" if not bad else "mismatch"
            o.write(json.dumps({"file": f, "verdict": v, "shapes": len(set(a) | set(b)), "why": [dict(bad)] + list(first.values())}) + "\n")


def report(paths):
    for p in paths:
        rows = [json.loads(l) for l in open(p)]
        print(f"== {p}: {len(rows)} fonts, {sum(r.get('cps', 0) for r in rows)} code points compared")
        for k, n in collections.Counter(r["verdict"] for r in rows).most_common():
            print(f"  {k:14} {n}")
        causes = collections.defaultdict(collections.Counter)
        for r in rows:
            if r["verdict"] not in ("match", "refused-cff"):
                causes[r["verdict"]][str(r["why"][0])[:110]] += 1
        for k, cc in causes.items():
            print(f"  -- {k}")
            for w, n in cc.most_common(10):
                print(f"     {n:5}  {w}")


if __name__ == "__main__":
    m, rest = sys.argv[1], sys.argv[2:]
    if m == "ref":
        try:
            print(json.dumps(ref_font(rest[0])))
        except Exception as e:
            print(json.dumps({"err": f"{type(e).__name__}: {str(e)[:150]}"}))
    elif m == "refshx":
        try:
            print(json.dumps(ref_shx(rest[0])))
        except Exception as e:
            print(json.dumps({"err": f"{type(e).__name__}: {str(e)[:150]}"}))
    elif m in ("run", "shx", "report"):
        {"run": run, "shx": shx, "report": report}[m](rest)


def ref_glyf(path, cps):
    """A second reading for adjudication: the glyf outlines as stored, decoded here from
    fontTools' coordinates (components resolved): no hmtx side-bearing shift (fontTools' glyph
    set, like FreeType, moves a glyph by lsb - xMin where the two disagree), glyf even where CFF
    is present, and a contour drawn only if it has two points or more."""
    from fontTools.ttLib import TTFont

    f = TTFont(path, fontNumber=0)
    glyf, cmap = f["glyf"], f.getBestCmap()
    out = {}
    for cp in cps:
        name = cmap.get(int(cp))
        g = glyf[name]
        co, ends, fl = g.getCoordinates(glyf)
        segs, s = [], 0
        for e in ends:
            pts = [(float(co[i][0]), float(co[i][1]), fl[i] & 1) for i in range(s, e + 1)]
            s = e + 1
            if len(pts) < 2:
                continue
            # start at an on-curve point (or the midpoint of two off-curve ones)
            k = next((i for i, p in enumerate(pts) if p[2]), None)
            if k is None:
                a, b = pts[0], pts[1]
                pts = [((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, 1)] + pts[1:] + pts[:1]
            else:
                pts = pts[k:] + pts[:k]
            c, cur, ctrl = [], pts[0][:2], None
            for p in pts[1:] + pts[:1]:
                if p[2]:
                    c.append([*cur, *ctrl, *p[:2]] if ctrl else [*cur, *p[:2]])
                    cur, ctrl = p[:2], None
                elif ctrl:
                    m = ((ctrl[0] + p[0]) / 2, (ctrl[1] + p[1]) / 2)
                    c.append([*cur, *ctrl, *m])
                    cur, ctrl = m, p[:2]
                else:
                    ctrl = p[:2]
            if ctrl:
                c.append([*cur, *ctrl, *pts[0][:2]])
            segs.append(c)
        a, box = our_measures(segs)
        lsb_shift = f["hmtx"][name][1] - g.xMin if hasattr(g, "xMin") else 0
        out[cp] = {"area": a, "box": box, "contours": len(segs), "lsb_shift": lsb_shift}
    return out


def adjudicate(argv):
    """For each font the rule calls a mismatch: each code point it named, against ref_glyf.
    'stored': ours equals the glyf outline as stored (the difference was fontTools' placement
    by hmtx, its CFF preference, or lone points in its bounds); else 'real'."""
    probe, rows = argv[0], argv[1:]
    for p in rows:
        for l in open(p):
            r = json.loads(l)
            if r["verdict"] != "mismatch" or not r["why"][0][0].isdigit():
                continue
            ours = child([probe, "font", r["file"]], 120)
            t = ref_font(r["file"])
            bad = []
            scale = t["upem"]
            for cp, b in t["cp"].items():
                a = ours["cp"].get(cp)
                if not a or "c" not in a:
                    continue
                area, box = our_measures(a["c"])
                n = sum(1 for c in a["c"] if c)
                if n != b["contours"] or abs(area - b["area"]) > 1e-9 * scale * scale or (box and b["box"] and any(abs(x - y) > 1e-6 for x, y in zip(box, b["box"]))):
                    bad.append(cp)
            g = ref_glyf(r["file"], bad)
            kinds = collections.Counter()
            first = {}
            for cp in bad:
                a = ours["cp"][cp]
                area, box = our_measures(a["c"])
                n = sum(1 for c in a["c"] if c)
                x = g[cp]
                same = n == x["contours"] and abs(area - x["area"]) <= 1e-9 * scale * scale and (box is None) == (x["box"] is None) and (box is None or all(abs(u - v) <= 1e-6 for u, v in zip(box, x["box"])))
                k = "stored" if same else "real"
                if same and x["lsb_shift"]:
                    k = "stored (lsb shift)"
                kinds[k] += 1
                if k == "real":
                    first.setdefault(k, f"U+{int(cp):04X}: ours {n} contours area {area:.6g} box {box} vs glyf {x['contours']} {x['area']:.6g} {x['box']}"[:200])
            print(json.dumps({"file": r["file"], "kinds": dict(kinds), "first": first}))


if __name__ == "__main__" and sys.argv[1] == "adjudicate":
    adjudicate(sys.argv[2:])
