#!/usr/bin/env python3
"""STL, OBJ and 3MF: our readers against independent ones, file by file, by a rule fixed before
any result was looked at (research/formats/real-files.md, "Meshes").

  $VENV/bin/python -I mesh_judge.py run PROBE OUT.jsonl FILE.. [@LIST] [--jobs 4] [--timeout 60]
  $VENV/bin/python -I mesh_judge.py ref FILE            the reference's reading of one file
  $VENV/bin/python -I mesh_judge.py rewrite PROBE OUT.jsonl FILE..   the STL writer's round trip
  $VENV/bin/python -I mesh_judge.py report OUT.jsonl..

References: numpy-stl for STL (raw facets), trimesh for OBJ (process=False), lib3mf for 3MF
(strict mode off, the build walked through components). Every measure the judge compares is
worked out here in numpy from the reference's own arrays: area, box, signed volume, and the open
edges by the reader's documented definition (a directed edge repeated, or with no reverse). STL
corners are welded where equal to the bit (-0 = 0), as STL has no indices; OBJ and 3MF keep the
file's indices. One process per file each side, memory capped, so a file costs only itself.
"""
import collections, concurrent.futures as cf, json, os, resource, subprocess, sys

MEM = 4 << 30
TOL = 1e-9  # relative: both sides sum the same f64 products, in different orders


def limit():
    resource.setrlimit(resource.RLIMIT_AS, (MEM, MEM))


def measures(V, T):
    """V (n,3) float64, T (m,3) int: the same measures our summary gives."""
    import numpy as np

    out = {"triangles": int(len(T)), "vertices": int(len(V))}
    if len(T) == 0:
        return out
    a, b, c = V[T[:, 0]], V[T[:, 1]], V[T[:, 2]]
    out["area"] = float(np.linalg.norm(np.cross(b - a, c - a), axis=1).sum() * 0.5)
    used = V[np.unique(T)]
    lo, hi = used.min(axis=0), used.max(axis=0)
    out["box"] = [float(x) for x in list(lo) + list(hi)]
    out["degenerate"] = int(((T[:, 0] == T[:, 1]) | (T[:, 1] == T[:, 2]) | (T[:, 0] == T[:, 2])).sum())
    d = np.concatenate([T[:, [0, 1]], T[:, [1, 2]], T[:, [2, 0]]]).astype(np.int64)
    n = int(len(V)) + 1
    key = d[:, 0] * n + d[:, 1]
    rev = d[:, 1] * n + d[:, 0]
    u, cnt = np.unique(key, return_counts=True)
    bad = set(u[cnt > 1].tolist())
    # a directed edge with no reverse among the directed edges
    noreverse = ~np.isin(key, rev)
    bad |= set(key[noreverse].tolist())
    out["open_edges"] = len(bad)
    out["closed"] = len(bad) == 0
    out["volume"] = float(np.einsum("ij,ij->i", a, np.cross(b, c)).sum() / 6.0) if not bad else None
    return out


def weld(P):
    """(m,3,3) corners: welded where equal to the bit, -0 being 0."""
    import numpy as np

    flat = P.reshape(-1, 3) + 0.0
    V, inv = np.unique(flat, axis=0, return_inverse=True)
    return V, inv.reshape(-1, 3)


def ref_stl(path):
    import numpy as np
    from stl import mesh as npstl

    m = npstl.Mesh.from_file(path, calculate_normals=False)
    P = np.asarray(m.vectors, dtype=np.float32).astype(np.float64)
    if not np.isfinite(P).all():
        return {"err": "non-finite corner"}
    if len(P) == 0:
        return {"err": "no triangles"}
    V, T = weld(P)
    return measures(V, T)


def ref_obj(path):
    import numpy as np
    import trimesh

    m = trimesh.load(path, force="mesh", process=False, maintain_order=True, skip_materials=True)
    if not hasattr(m, "faces") or len(m.faces) == 0:
        return {"err": "no faces"}
    return measures(np.asarray(m.vertices, dtype=np.float64), np.asarray(m.faces, dtype=np.int64))


def ref_3mf(path):
    import numpy as np
    import lib3mf

    w = lib3mf.get_wrapper()
    model = w.CreateModel()
    r = model.QueryReader("3mf")
    r.SetStrictModeActive(False)
    try:
        r.ReadFromFile(path)
    except Exception as e:
        return {"err": str(e)[:200]}
    scale = {"micron": 1e-3, "millimeter": 1.0, "centimeter": 10.0, "inch": 25.4, "foot": 304.8, "meter": 1000.0}
    unit = model.GetUnit()
    names = {0: "micron", 1: "millimeter", 2: "centimeter", 3: "inch", 4: "foot", 5: "meter"}
    s = scale[names.get(int(unit), "millimeter")]

    def mat(t):
        f = t.Fields
        M = np.eye(4)
        for i in range(4):
            for j in range(3):
                M[i, j] = f[i][j]
        return M  # row vector convention: p' = [p 1] M

    Vs, Ts, flips = [], [], []
    count = [0]

    def walk(obj_id, M, depth):
        if depth > 32:
            raise RuntimeError("components nested too deep")
        try:
            mo = model.GetMeshObjectByID(obj_id)
        except Exception:
            mo = None
        if mo is not None:
            V = np.array([v.Coordinates[0:3] for v in mo.GetVertices()], dtype=np.float64).reshape(-1, 3)
            T = np.array([t.Indices[0:3] for t in mo.GetTriangleIndices()], dtype=np.int64).reshape(-1, 3)
            if len(T) == 0:
                return
            Vh = np.hstack([V, np.ones((len(V), 1))]) @ M
            if np.linalg.det(M[:3, :3]) < 0:
                T = T[:, [0, 2, 1]]
            Vs.append(Vh[:, :3] * s)
            Ts.append(T + count[0])
            count[0] += len(V)
        else:
            co = model.GetComponentsObjectByID(obj_id)
            for k in range(co.GetComponentCount()):
                c = co.GetComponent(k)
                CM = mat(c.GetTransform()) if c.HasTransform() else np.eye(4)
                walk(c.GetObjectResourceID(), CM @ M, depth + 1)

    it = model.GetBuildItems()
    while it.MoveNext():
        b = it.GetCurrent()
        M = mat(b.GetObjectTransform()) if b.HasObjectTransform() else np.eye(4)
        walk(b.GetObjectResource().GetResourceID(), M, 0)
    if not Ts:
        return {"err": "no triangles in the build"}
    out = measures(np.concatenate(Vs), np.concatenate(Ts))
    out["warnings"] = r.GetWarningCount()
    return out


def ref(path):
    ext = path.rsplit(".", 1)[-1].lower()
    try:
        return {"stl": ref_stl, "obj": ref_obj, "3mf": ref_3mf}[ext](path)
    except Exception as e:
        return {"err": f"{type(e).__name__}: {str(e)[:200]}"}


def child(cmd, timeout):
    try:
        p = subprocess.run(cmd, capture_output=True, timeout=timeout, preexec_fn=limit)
    except subprocess.TimeoutExpired:
        return {"crash": "timeout"}
    lines = p.stdout.decode("utf8", "replace").strip().splitlines()
    for l in reversed(lines):
        try:
            d = json.loads(l)
            if "panic" in d:
                return {"crash": "panic", "panic": d["panic"]}
            return d
        except json.JSONDecodeError:
            continue
    err = p.stderr.decode("utf8", "replace")
    at = [l for l in err.splitlines() if "panicked" in l or "memory allocation" in l or "overflow" in l]
    return {"crash": f"exit {p.returncode}", "panic": (at[0] if at else err[-200:])[:200]}


def close(a, b, rel, scale=1.0):
    return abs(a - b) <= rel * max(abs(a), abs(b), scale)


def verdict(path, ours, theirs):
    """The rule. Fields compared: triangles, vertices (STL only: trimesh re-indexes an OBJ's), area,
    box of the vertices the triangles use, open edges, closed,
    volume where closed. Order of outcomes: crash/timeout, then refusals, then the fields."""
    if "crash" in ours:
        return "timeout" if ours["crash"] == "timeout" else "crash", [ours.get("panic", ours["crash"])]
    if "crash" in theirs:
        theirs = {"err": "reference " + theirs["crash"]}
    if "err" in ours and "err" in theirs:
        return "both-refused", [ours["err"]]
    if "err" in ours:
        return "refused", [ours["err"]]
    if "err" in theirs:
        return "ref-refused", [theirs["err"]]
    ext = path.rsplit(".", 1)[-1].lower()
    bad = []
    if ours["triangles"] != theirs["triangles"]:
        bad.append(f"triangles {ours['triangles']} vs {theirs['triangles']}")
    if ext == "stl" and ours["vertices"] != theirs["vertices"]:
        bad.append(f"vertices {ours['vertices']} vs {theirs['vertices']}")
    diag = sum((theirs["box"][k + 3] - theirs["box"][k]) ** 2 for k in range(3)) ** 0.5 or 1.0
    tol = TOL
    if ext == "3mf":
        # lib3mf keeps coordinates as f32 (its API's sLib3MFPosition), where ours reads the text
        # as f64: each coordinate may be off by half an f32 ulp of its size, 6e-8 relative
        far = max(abs(x) for x in theirs["box"]) / diag
        tol = 2e-6 * (1.0 + far)
    if not close(ours["area"], theirs["area"], tol, 1e-12 * diag * diag):
        bad.append(f"area {ours['area']} vs {theirs['area']}")
    if any(abs(x - y) > tol * diag for x, y in zip(ours["rbox"], theirs["box"])):
        bad.append(f"box {ours['rbox']} vs {theirs['box']}")
    if ours["open_edges"] != theirs["open_edges"]:
        bad.append(f"open edges {ours['open_edges']} vs {theirs['open_edges']}")
    if ours["closed"] and theirs["closed"] and not close(ours["volume"], theirs["volume"], tol, 1e-12 * diag ** 3):
        bad.append(f"volume {ours['volume']} vs {theirs['volume']}")
    return ("match" if not bad else "mismatch"), bad


def files(args):
    out = []
    for a in args:
        if a.startswith("@"):
            out += [l.strip() for l in open(a[1:]) if l.strip()]
        else:
            out.append(a)
    return out


def opt(args, name, default):
    if name in args:
        i = args.index(name)
        v = args[i + 1]
        del args[i : i + 2]
        return type(default)(v)
    return default


def run(argv):
    jobs, timeout = opt(argv, "--jobs", 4), opt(argv, "--timeout", 60)
    probe, out, fs = argv[0], argv[1], files(argv[2:])
    me = os.path.abspath(__file__)

    def one(f):
        ours = child([probe, "mesh", f], timeout)
        theirs = child([sys.executable, "-I", me, "ref", f], timeout * 4)
        v, why = verdict(f, ours, theirs)
        return {"file": f, "verdict": v, "why": why, "ours": ours, "ref": theirs}

    with open(out, "w") as o, cf.ThreadPoolExecutor(jobs) as ex:
        for r in ex.map(one, fs):
            o.write(json.dumps(r) + "\n")
            o.flush()


def rewrite(argv):
    """The STL writer: our reading of FILE written by to_binary_stl and read back by numpy-stl
    must measure what the reference measured of FILE itself, by the same rule. STL holds f32:
    an STL source's reference is numpy-stl's f32 too (trimesh's, every solid, for an ASCII one
    numpy-stl reads in part), so the two agree to the bit; an OBJ source's is trimesh's f64,
    held at f32's rounding (2e-6 of the size and distance from the origin, as for 3MF).
    (The first version held our own f64 reading of the source against the f32 file: wrong for
    every ASCII and OBJ source, corrected before its results were counted.)"""
    import numpy as np
    import trimesh

    jobs, timeout = opt(argv, "--jobs", 4), opt(argv, "--timeout", 60)
    probe, out, fs = argv[0], argv[1], files(argv[2:])
    me = os.path.abspath(__file__)
    tmp = out + ".d"
    os.makedirs(tmp, exist_ok=True)

    def one(k_f):
        k, f = k_f
        w = os.path.join(tmp, f"{k}.stl")
        wr = child([probe, "mesh-rewrite", f, w], timeout)
        if "err" in wr or "crash" in wr:
            return {"file": f, "verdict": "not-read", "why": [wr.get("err") or wr.get("crash")]}
        back = child([sys.executable, "-I", me, "ref", w], timeout * 4)
        os.remove(w)
        if "err" in back or "crash" in back:
            return {"file": f, "verdict": "ref-refused-written", "why": [back.get("err") or back.get("crash")]}
        src = child([sys.executable, "-I", me, "ref", f], timeout * 4)
        ascii_stl = f.lower().endswith(".stl") and open(f, "rb").read(5) == b"solid"
        if ascii_stl:
            m = trimesh.load(f, force="mesh", process=False)
            V, T = weld(np.asarray(m.triangles, dtype=np.float32).astype(np.float64))
            src = measures(V, T)
        if "err" in src:
            return {"file": f, "verdict": "ref-refused", "why": [src["err"]]}
        src["rbox"] = src["box"]
        v, why = verdict(w if not f.lower().endswith(".obj") else "x.3mf", src, back)
        return {"file": f, "verdict": v, "why": why, "src": src, "ref": back}

    with open(out, "w") as o, cf.ThreadPoolExecutor(jobs) as ex:
        for r in ex.map(one, enumerate(fs)):
            o.write(json.dumps(r) + "\n")
            o.flush()


def report(paths):
    for p in paths:
        rows = [json.loads(l) for l in open(p)]
        c = collections.Counter(r["verdict"] for r in rows)
        print(f"== {p}: {len(rows)} files")
        for k, n in c.most_common():
            print(f"  {k:14} {n}")
        causes = collections.defaultdict(collections.Counter)
        for r in rows:
            if r["verdict"] != "match":
                for w in r["why"][:1]:
                    causes[r["verdict"]][str(w)[:90]] += 1
        for k, cc in causes.items():
            print(f"  -- {k}")
            for w, n in cc.most_common(8):
                print(f"     {n:5}  {w}")


if __name__ == "__main__":
    m, rest = sys.argv[1], sys.argv[2:]
    if m == "ref":
        print(json.dumps(ref(rest[0])))
    elif m == "run":
        run(rest)
    elif m == "rewrite":
        rewrite(rest)
    elif m == "report":
        report(rest)


def adjudicate(argv):
    """A second reference for the mismatches the rule finds in ASCII STL: numpy-stl keeps an
    ASCII file's numbers as f32 and reads only its first solid, where ours reads f64 and every
    solid. trimesh parses ASCII STL as f64, every solid: welded the same way, held to the same
    rule. Run after `run`, on its output; added once the first run showed the cause."""
    import numpy as np
    import trimesh

    for p in argv:
        for l in open(p):
            r = json.loads(l)
            if r["verdict"] != "mismatch" or not r["file"].lower().endswith(".stl"):
                continue
            if not open(r["file"], "rb").read(5) == b"solid":
                continue
            m = trimesh.load(r["file"], force="mesh", process=False)
            P = np.asarray(m.triangles, dtype=np.float64)
            V, T = weld(P)
            v, why = verdict(r["file"], r["ours"], measures(V, T))
            print(json.dumps({"file": r["file"], "verdict2": v, "why2": why}))


if __name__ == "__main__" and sys.argv[1] == "adjudicate":
    adjudicate(sys.argv[2:])


def obj_indices(path):
    """An OBJ's triangles by the file's own position indices (fans, as the format means), for
    adjudicating open-edge counts where trimesh re-indexes by texture and material."""
    import numpy as np

    nv, T = 0, []
    for raw in open(path, "rb").read().decode("utf8", "replace").splitlines():
        w = raw.split("#")[0].split()
        if not w:
            continue
        if w[0] == "v":
            nv += 1
        elif w[0] == "f":
            ix = [int(x.split("/")[0]) for x in w[1:]]
            ix = [i - 1 if i > 0 else nv + i for i in ix]
            T += [[ix[0], ix[j], ix[j + 1]] for j in range(1, len(ix) - 1)]
    V = np.zeros((nv, 3))
    return measures(V, np.array(T, dtype=np.int64))["open_edges"]


if __name__ == "__main__" and sys.argv[1] == "obj-edges":
    for p in sys.argv[2:]:
        for l in open(p):
            r = json.loads(l)
            if r["verdict"] == "mismatch" and r["file"].lower().endswith(".obj"):
                print(json.dumps({"file": r["file"], "ours": r["ours"]["open_edges"], "trimesh": r["ref"]["open_edges"], "file_indices": obj_indices(r["file"])}))
