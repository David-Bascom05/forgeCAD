#!/usr/bin/env python3
"""What real STEP files use: entities, what faces and edges stand on, units, tolerances, systems.
Reads text only and runs nothing. Run with python3 -I.

  survey.py OUT.json --zip Z --prefix P --sample N --seed S    (a sample of a zip's .step/.stp)
  survey.py OUT.json --dir D [--sample N]                      (a tree)

The sources are downloads: no source file enters the repo, only the counts."""
import argparse, collections, json, multiprocessing as mp, os, random, re, zipfile

SPLIT = re.compile(r"'(?:[^']|'')*'|;")
HEAD = re.compile(r"\s*#(\d+)\s*=\s*(\(?)\s*([A-Za-z_0-9]*)")
WORD = re.compile(r"([A-Z_][A-Z_0-9]*)\s*\(")
REF = re.compile(r"#(\d+)")
MAX_FILE = 400 << 20
ZIPS = {}  # one open zip per worker: opening a 450k-entry directory per file costs more than the read


def statements(t):
    at = 0
    for m in SPLIT.finditer(t):
        if m.group() == ";":
            yield t[at : m.end() - 1]
            at = m.end()


def top_args(s):
    # the argument list of a simple instance, split at depth 1
    i = s.find("(", s.find("="))
    depth, out, cur, q = 0, [], i + 1, False
    for j in range(i, len(s)):
        c = s[j]
        if c == "'":
            q = not q
        elif q:
            continue
        elif c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                out.append(s[cur:j].strip())
                break
        elif c == "," and depth == 1:
            out.append(s[cur:j].strip())
            cur = j + 1
    return out


def scan(t):
    r = dict(n=collections.Counter(), faces=collections.Counter(), edges=collections.Counter(),
             sor=collections.Counter(), extr=collections.Counter(), bs=collections.Counter(),
             bc=collections.Counter(), pc=collections.Counter(), solid_refs=collections.Counter(),
             unit=collections.Counter(), unc=collections.Counter(), roots=collections.Counter(), offset=collections.Counter(), iso=collections.Counter())
    ents, kinds, faces, ecs = {}, {}, [], []
    schema, system = "?", "?"
    i = t.find("DATA")
    hdr = t[: i if i > 0 else 4096][:8192]
    m = re.search(r"FILE_SCHEMA\s*\(\s*\(\s*'([^']*)'", hdr)
    if m:
        schema = m.group(1).split()[0].split("{")[0].upper()[:40]
    m = re.search(r"FILE_NAME\s*\((.*?)\)\s*;", hdr, re.S)
    if m:
        s = re.findall(r"'((?:[^']|'')*)'", m.group(1))
        system = " | ".join(x for x in s[-3:-1] if x)[:60]
    for s in statements(t):
        m = HEAD.match(s)
        if not m:
            continue
        idn, cplx, kw = int(m.group(1)), m.group(2), m.group(3).upper()
        if cplx:
            ws = tuple(sorted(w for w in WORD.findall(s.upper()) if w not in ("LENGTH_MEASURE", "PLANE_ANGLE_MEASURE", "RATIO_MEASURE", "POSITIVE_LENGTH_MEASURE", "PARAMETER_VALUE")))
            kinds[idn] = "+".join(ws)[:120]
            r["n"]["(" + kinds[idn] + ")"] += 1
            for w in ws:
                r["n"][w] += 0  # known, but counted under its complex form
            if "B_SPLINE_SURFACE_WITH_KNOTS" in ws or "B_SPLINE_SURFACE" in ws:
                kinds[idn] = "(rational) B_SPLINE_SURFACE" if "RATIONAL_B_SPLINE_SURFACE" in ws else "(complex) B_SPLINE_SURFACE"
            elif "B_SPLINE_CURVE" in ws:
                kinds[idn] = "(rational) B_SPLINE_CURVE" if "RATIONAL_B_SPLINE_CURVE" in ws else "(complex) B_SPLINE_CURVE"
            if "LENGTH_UNIT" in ws or "PLANE_ANGLE_UNIT" in ws:
                r["unit"][re.sub(r"\s+", "", s[s.find("(") :])[:90]] += 1
            if "SURFACE_CURVE" in ws or "SEAM_CURVE" in ws:
                ents[idn] = s
            continue
        kinds[idn] = kw
        r["n"][kw] += 1
        if kw == "ADVANCED_FACE" or kw == "FACE_SURFACE":
            a = top_args(s)
            if len(a) >= 3:
                x = REF.match(a[2])
                faces.append(int(x.group(1)) if x else -1)
        elif kw == "EDGE_CURVE":
            a = top_args(s)
            if len(a) >= 4:
                x = REF.match(a[3])
                ecs.append(int(x.group(1)) if x else -1)
        elif kw in ("SURFACE_CURVE", "SEAM_CURVE", "BOUNDED_SURFACE_CURVE", "SURFACE_OF_REVOLUTION", "SURFACE_OF_LINEAR_EXTRUSION",
                    "OFFSET_SURFACE", "RECTANGULAR_TRIMMED_SURFACE", "CURVE_BOUNDED_SURFACE", "OFFSET_CURVE_3D", "TRIMMED_CURVE", "COMPOSITE_CURVE", "PCURVE", "INTERSECTION_CURVE", "DEGENERATE_TOROIDAL_SURFACE"):
            ents[idn] = s
        elif kw == "B_SPLINE_SURFACE_WITH_KNOTS":
            a = top_args(s)
            if len(a) > 4:
                r["bs"][f"p{a[1]}q{a[2]}"[:12]] += 1
                r["bs"]["form " + a[4].strip(".")] += 1
                r["bs"]["closed " + a[5] + a[6]] += 1
                r["bs"]["knots " + a[-2].strip(".")] += 1
                ctrl = len(REF.findall(a[3]))
                r["bs"]["ctrl<=16" if ctrl <= 16 else "ctrl<=100" if ctrl <= 100 else "ctrl<=1000" if ctrl <= 1000 else "ctrl>1000"] += 1
        elif kw == "B_SPLINE_CURVE_WITH_KNOTS":
            a = top_args(s)
            if len(a) > 1:
                r["bc"]["p" + a[1]] += 1
                r["bc"]["form " + a[3].strip(".")] += 1
                r["bc"]["closed " + a[4]] += 1
        elif kw == "UNCERTAINTY_MEASURE_WITH_UNIT":
            x = re.search(r"\(\s*([-+0-9.Ee]+)\s*\)", s)
            if x:
                try:
                    r["unc"]["1e%d" % round(__import__("math").log10(abs(float(x.group(1))) or 1e-300))] += 1
                except ValueError:
                    pass
        if kw in ("MANIFOLD_SOLID_BREP", "BREP_WITH_VOIDS", "FACETED_BREP", "SHELL_BASED_SURFACE_MODEL", "GEOMETRIC_SET", "GEOMETRIC_CURVE_SET", "NON_MANIFOLD_SURFACE_SHAPE_REPRESENTATION", "MANIFOLD_SURFACE_SHAPE_REPRESENTATION", "ADVANCED_BREP_SHAPE_REPRESENTATION", "NEXT_ASSEMBLY_USAGE_OCCURRENCE", "MAPPED_ITEM", "TESSELLATED_SOLID", "TESSELLATED_SHELL", "TRIANGULATED_FACE", "COMPLEX_TRIANGULATED_FACE"):
            r["roots"][kw] += 1
    def inner(k, seen=0):
        # what a wrapper stands on
        kk = kinds.get(k, "?")
        if kk in ("SURFACE_CURVE", "SEAM_CURVE", "BOUNDED_SURFACE_CURVE", "INTERSECTION_CURVE") and k in ents and seen < 4:
            a = top_args(ents[k])
            x = REF.match(a[1]) if len(a) > 1 else None
            if x:
                return inner(int(x.group(1)), seen + 1)
        return kk
    for f in faces:
        r["faces"][kinds.get(f, "?")] += 1
    for e in ecs:
        k = kinds.get(e, "?")
        r["edges"][("[%s] " % k if k in ("SURFACE_CURVE", "SEAM_CURVE", "BOUNDED_SURFACE_CURVE", "INTERSECTION_CURVE") else "") + inner(e)] += 1
    for k, s in ents.items():
        kk = kinds.get(k)
        a = top_args(s) if kk else []
        if kk == "SURFACE_OF_REVOLUTION" and len(a) > 1:
            x = REF.match(a[1])
            r["sor"][inner(int(x.group(1))) if x else "?"] += 1
        elif kk == "SURFACE_OF_LINEAR_EXTRUSION" and len(a) > 1:
            x = REF.match(a[1])
            r["extr"][inner(int(x.group(1))) if x else "?"] += 1
        elif kk == "OFFSET_SURFACE" and len(a) > 1:
            x = REF.match(a[1])
            r["offset"][kinds.get(int(x.group(1)), "?") if x else "?"] += 1
        elif kk == "PCURVE" and len(a) > 2:
            x = REF.match(a[2])
            r["pc"][kinds.get(int(x.group(1)), "?") if x else "?"] += 1
    return schema, system, r


def merge(tot, schema, system, r):
    tot["files"] += 1
    tot["schema"][schema] += 1
    tot["system"][system] += 1
    for k, c in r.items():
        tot[k].update(c)
    for k in r["n"]:
        tot["files_with"][k] += 1
    kinds = set(r["n"])
    solids = r["roots"].get("MANIFOLD_SOLID_BREP", 0) + r["roots"].get("BREP_WITH_VOIDS", 0)
    tot["solids_per_file"]["0" if solids == 0 else "1" if solids == 1 else "2-10" if solids <= 10 else ">10"] += 1
    if r["roots"].get("NEXT_ASSEMBLY_USAGE_OCCURRENCE"):
        tot["with_assembly"] += 1
    n = sum(r["n"].values())
    tot["ents_per_file"]["<1k" if n < 1000 else "<10k" if n < 10000 else "<100k" if n < 100000 else ">=100k"] += 1
    if not kinds:
        tot["empty"] += 1


def one(job):
    try:
        src, name = job
        if src:
            if src not in ZIPS:
                ZIPS[src] = zipfile.ZipFile(src)
            b = ZIPS[src].read(name)
        else:
            if os.path.getsize(name) > MAX_FILE:
                return None
            with open(name, "rb") as f:
                b = f.read()
        return scan(b.decode("latin-1"))
    except Exception as e:
        return ("error", repr(e)[:80], None)


def main():
    p = argparse.ArgumentParser()
    p.add_argument("out")
    p.add_argument("--zip")
    p.add_argument("--dir")
    p.add_argument("--prefix", default="")
    p.add_argument("--sample", type=int, default=0)
    p.add_argument("--seed", type=int, default=1)
    p.add_argument("--jobs", type=int, default=4)
    a = p.parse_args()
    if a.zip:
        with zipfile.ZipFile(a.zip) as z:
            names = sorted(i.filename for i in z.infolist() if i.filename.lower().endswith((".step", ".stp")) and i.filename.startswith(a.prefix) and i.file_size < MAX_FILE)
        jobs = [(a.zip, n) for n in names]
    else:
        jobs = sorted((None, os.path.join(d, f)) for d, _, fs in os.walk(a.dir) for f in fs if f.lower().endswith((".step", ".stp")))
    total = len(jobs)
    if a.sample and a.sample < total:
        random.Random(a.seed).shuffle(jobs)
        jobs = jobs[: a.sample]
    tot = collections.defaultdict(collections.Counter)
    tot["files"] = tot["with_assembly"] = tot["empty"] = 0
    tot["errors"] = 0
    with mp.Pool(a.jobs) as pool:
        for i, res in enumerate(pool.imap_unordered(one, jobs, chunksize=8)):
            if res is None or res[0] == "error":
                tot["errors"] += 1
                continue
            merge(tot, *res)
            if i % 500 == 0:
                print(i, len(jobs), flush=True)
    tot["population"] = total
    with open(a.out, "w") as f:
        json.dump(tot, f, indent=0, sort_keys=True)
    print("done", a.out, tot["files"], "errors", tot["errors"])


if __name__ == "__main__":
    main()
