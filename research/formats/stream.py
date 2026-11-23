#!/usr/bin/env python3
"""Streaming, resumable, sharded runs of the judges over corpora too large to stage whole: fetch a
batch, judge it by the same rule (mesh_judge, font_judge), append each file's verdict, delete the
batch. A file already in OUT is skipped, so a run killed anywhere resumes where it stopped.
Shards are deterministic: shard K of N takes items K, K+N, K+2N.. of the sorted list.

  $VENV/bin/python -I stream.py thingi PROBE OUT.jsonl META_DIR [--shard K/N] [--batch 100] [--jobs 3]
      every Thingi10K file (Hugging Face raw_meshes), META_DIR holding metadata/input_summary.csv
  $VENV/bin/python -I stream.py abc PROBE OUT.jsonl CHUNK.7z [--jobs 3] [--every 1]
      an ABC OBJ chunk, read once in order: each mesh judged as it comes out, then deleted
  $VENV/bin/python -I stream.py files KIND PROBE OUT.jsonl @LIST [--jobs 3]
      local files (KIND mesh or font), judged and kept
  $VENV/bin/python -I stream.py urls KIND PROBE OUT.jsonl BASE_URL @PATHS [--shard K/N] [--batch 100] [--jobs 3]
      BASE_URL + each path fetched, judged (KIND mesh or font), deleted; the path is the id
  $VENV/bin/python -I stream.py report OUT.jsonl..

Rows carry the rule's verdict and, where the rule's mismatch has a declared adjudication
(mesh_judge: ASCII STL against trimesh's f64 reading; OBJ open edges from the file's indices;
font_judge: the stored glyf outline), its result as `adjudicated`.
"""
import collections, concurrent.futures as cf, csv, hashlib, json, os, shutil, sys, tempfile, threading, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import mesh_judge  # noqa: E402

THINGI = "https://huggingface.co/datasets/Thingi10K/Thingi10K/resolve/main/raw_meshes/"
lock = threading.Lock()


def opt(args, name, default):
    if name in args:
        i = args.index(name)
        v = args[i + 1]
        del args[i : i + 2]
        return type(default)(v)
    return default


def done_ids(out):
    if not os.path.exists(out):
        return set()
    ids = set()
    for l in open(out):
        try:
            ids.add(json.loads(l)["id"])
        except Exception:
            pass
    return ids


def judge_mesh(probe, path, timeout=60):
    """mesh_judge's rule on one file, then its declared adjudication where the rule says mismatch."""
    me = os.path.join(HERE, "mesh_judge.py")
    ours = mesh_judge.child([probe, "mesh", path], timeout)
    theirs = mesh_judge.child([sys.executable, "-I", me, "ref", path], timeout * 4)
    v, why = mesh_judge.verdict(path, ours, theirs)
    row = {"verdict": v, "why": [str(w)[:200] for w in why][:3]}
    low = path.lower()
    if v == "mismatch" and low.endswith(".stl") and open(path, "rb").read(5) == b"solid":
        try:
            import numpy as np
            import trimesh

            m = trimesh.load(path, force="mesh", process=False)
            V, T = mesh_judge.weld(np.asarray(m.triangles, dtype=np.float64))
            v2, why2 = mesh_judge.verdict(path, ours, mesh_judge.measures(V, T))
            row["adjudicated"], row["why2"] = v2, why2[:2]
        except Exception as e:
            row["adjudicated"] = f"error: {e}"[:120]
    elif v == "mismatch" and low.endswith(".obj") and all(w.startswith("open edges") for w in why):
        try:
            n = mesh_judge.obj_indices(path)
            row["adjudicated"] = "match" if n == ours["open_edges"] else "mismatch"
            row["why2"] = [f"file indices give {n} open edges"]
        except Exception as e:
            row["adjudicated"] = f"error: {e}"[:120]
    if "crash" not in ours and "err" not in ours:
        row["triangles"] = ours.get("triangles")
    return row


def write_row(o, row):
    with lock:
        o.write(json.dumps(row) + "\n")
        o.flush()


def thingi(argv):
    shard, batch, jobs = opt(argv, "--shard", "0/1"), opt(argv, "--batch", 100), opt(argv, "--jobs", 3)
    probe, out, meta = argv[0], argv[1], argv[2]
    k, n = (int(x) for x in shard.split("/"))
    rows = sorted(csv.DictReader(open(os.path.join(meta, "input_summary.csv"))), key=lambda r: int(r["ID"]))[k::n]
    seen = done_ids(out)
    todo = [r for r in rows if r["ID"] not in seen]
    print(f"shard {shard}: {len(rows)} files, {len(todo)} to do", flush=True)
    with open(out, "a") as o:
        for b in range(0, len(todo), batch):
            tmp = tempfile.mkdtemp(prefix="thingi-")
            try:
                def fetch(r):
                    ext = r["Link"].rsplit(".", 1)[-1].lower()
                    p = os.path.join(tmp, f"{r['ID']}.{ext}")
                    try:
                        urllib.request.urlretrieve(THINGI + f"{r['ID']}.{ext}", p)
                        return r, p, None
                    except Exception as e:
                        return r, p, str(e)[:120]

                with cf.ThreadPoolExecutor(8) as ex:
                    got = list(ex.map(fetch, todo[b : b + batch]))

                def one(g):
                    r, p, err = g
                    row = {"id": r["ID"], "thing": r["Thing ID"], "licence": r["License"], "ext": p.rsplit(".", 1)[-1]}
                    if err:
                        row.update(verdict="not-fetched", why=[err])
                    else:
                        d = open(p, "rb").read()
                        row.update(bytes=len(d), sha256=hashlib.sha256(d).hexdigest())
                        if row["ext"] in ("stl", "obj", "3mf"):
                            row.update(judge_mesh(probe, p))
                        else:
                            row.update(verdict="not-a-format-we-read", why=[row["ext"]])
                    write_row(o, row)

                with cf.ThreadPoolExecutor(jobs) as ex:
                    list(ex.map(one, got))
            finally:
                shutil.rmtree(tmp, ignore_errors=True)
            print(f"  {min(b + batch, len(todo))}/{len(todo)}", flush=True)


def abc(argv):
    """One pass over the solid archive (py7zr's extract with a writer factory): each member is
    written to a temporary file; when the factory is asked for the next, the last is complete and
    goes to be judged, then deleted. A semaphore holds at most jobs * 4 files on disk."""
    import py7zr
    from py7zr.io import Py7zIO, WriterFactory

    jobs, every = opt(argv, "--jobs", 3), opt(argv, "--every", 1)
    probe, out, chunk = argv[0], argv[1], argv[2]
    seen = done_ids(out)
    tmp = tempfile.mkdtemp(prefix="abc-")
    pool = cf.ThreadPoolExecutor(jobs)
    o = open(out, "a")
    sem = threading.Semaphore(jobs * 4)
    last = []

    def judge(name, path):
        try:
            d = open(path, "rb").read()
            row = {"id": name, "bytes": len(d), "sha256": hashlib.sha256(d).hexdigest()}
            row.update(judge_mesh(probe, path, timeout=120))
            write_row(o, row)
        except Exception as e:
            write_row(o, {"id": name, "verdict": "judge-error", "why": [str(e)[:120]]})
        finally:
            os.remove(path)
            sem.release()

    def hand_on():
        while last:
            w = last.pop()
            w.f.close()
            pool.submit(judge, w.name, w.path)

    class W(Py7zIO):
        def __init__(self, name):
            self.name, self.n = name, 0
            self.path = os.path.join(tmp, hashlib.sha1(name.encode()).hexdigest() + ".obj")
            self.f = open(self.path, "wb")

        def write(self, s):
            self.f.write(s)
            self.n += len(s)
            return len(s)

        def read(self, size=None):
            return b""

        def seek(self, offset, whence=0):
            return 0

        def flush(self):
            pass

        def size(self):
            return self.n

    class F(WriterFactory):
        def create(self, filename):
            hand_on()
            sem.acquire()
            w = W(filename)
            last.append(w)
            return w

    with py7zr.SevenZipFile(chunk) as z:
        allnames = sorted(n for n in z.getnames() if n.lower().endswith(".obj"))
        pick = [n for i, n in enumerate(allnames) if i % every == 0 and n not in seen]
        print(f"{len(allnames)} meshes, {len(pick)} to do", flush=True)
        z.extract(targets=pick, factory=F())
    hand_on()
    pool.shutdown(wait=True)
    o.close()
    shutil.rmtree(tmp, ignore_errors=True)


def judge_font(probe, path):
    import font_judge

    me = os.path.join(HERE, "font_judge.py")
    ours = font_judge.child([probe, "font", path], 120)
    t = font_judge.child([sys.executable, "-I", me, "ref", path], 360)
    v, why, n = font_judge.verdict(ours, t)
    row = {"verdict": v, "why": why[:3], "cps": n}
    if v == "mismatch" and why and why[0][:1].isdigit():
        # font_judge.adjudicate's test, on this font
        try:
            import io, contextlib

            buf = io.StringIO()
            with open(os.devnull, "w"):
                tmp = tempfile.NamedTemporaryFile("w", suffix=".jsonl", delete=False)
                tmp.write(json.dumps({"file": path, "verdict": v, "why": why}) + "\n")
                tmp.close()
                with contextlib.redirect_stdout(buf):
                    font_judge.adjudicate([probe, tmp.name])
                os.remove(tmp.name)
            k = json.loads(buf.getvalue().strip().splitlines()[-1])["kinds"]
            row["adjudicated"] = "match" if not k.get("real") else "mismatch"
            row["why2"] = [json.dumps(k)]
        except Exception as e:
            row["adjudicated"] = f"error: {e}"[:120]
    return row


def urls(argv):
    shard, batch, jobs = opt(argv, "--shard", "0/1"), opt(argv, "--batch", 100), opt(argv, "--jobs", 3)
    kind, probe, out, base = argv[0], argv[1], argv[2], argv[3]
    paths = []
    for a in argv[4:]:
        paths += [l.strip() for l in open(a[1:]) if l.strip()] if a.startswith("@") else [a]
    k, n = (int(x) for x in shard.split("/"))
    paths = sorted(paths)[k::n]
    seen = done_ids(out)
    todo = [p for p in paths if p not in seen]
    print(f"shard {shard}: {len(paths)} files, {len(todo)} to do", flush=True)
    with open(out, "a") as o:
        for b in range(0, len(todo), batch):
            tmp = tempfile.mkdtemp(prefix="urls-")
            try:
                def fetch(i_p):
                    i, p = i_p
                    local = os.path.join(tmp, f"{i}." + p.rsplit(".", 1)[-1])
                    try:
                        urllib.request.urlretrieve(base + urllib.request.quote(p), local)
                        return p, local, None
                    except Exception as e:
                        return p, local, str(e)[:120]

                with cf.ThreadPoolExecutor(8) as ex:
                    got = list(ex.map(fetch, enumerate(todo[b : b + batch])))

                def one(g):
                    p, local, err = g
                    row = {"id": p}
                    if err:
                        row.update(verdict="not-fetched", why=[err])
                    else:
                        d = open(local, "rb").read()
                        row.update(bytes=len(d), sha256=hashlib.sha256(d).hexdigest())
                        row.update(judge_mesh(probe, local) if kind == "mesh" else judge_font(probe, local))
                    write_row(o, row)

                with cf.ThreadPoolExecutor(jobs) as ex:
                    list(ex.map(one, got))
            finally:
                shutil.rmtree(tmp, ignore_errors=True)
            print(f"  {min(b + batch, len(todo))}/{len(todo)}", flush=True)


def files(argv):
    jobs = opt(argv, "--jobs", 3)
    kind, probe, out = argv[0], argv[1], argv[2]
    fs = []
    for a in argv[3:]:
        fs += [l.strip() for l in open(a[1:]) if l.strip()] if a.startswith("@") else [a]
    seen = done_ids(out)
    todo = [f for f in fs if f not in seen]
    print(f"{len(fs)} files, {len(todo)} to do", flush=True)
    if kind == "font":
        import font_judge

    with open(out, "a") as o:
        def one(f):
            row = {"id": f}
            if kind == "mesh":
                row.update(judge_mesh(probe, f))
            else:
                me = os.path.join(HERE, "font_judge.py")
                ours = font_judge.child([probe, "font", f], 120)
                t = font_judge.child([sys.executable, "-I", me, "ref", f], 360)
                v, why, n = font_judge.verdict(ours, t)
                row.update(verdict=v, why=why[:3], cps=n)
            write_row(o, row)

        with cf.ThreadPoolExecutor(jobs) as ex:
            list(ex.map(one, todo))


def report(paths):
    for p in paths:
        rows = [json.loads(l) for l in open(p)]
        final = collections.Counter(r.get("adjudicated") if r.get("adjudicated") in ("match", "mismatch") else r["verdict"] for r in rows)
        print(f"== {p}: {len(rows)} files")
        print("  rule:       ", dict(collections.Counter(r["verdict"] for r in rows).most_common()))
        print("  adjudicated:", dict(final.most_common()))
        causes = collections.defaultdict(collections.Counter)
        for r in rows:
            v = r.get("adjudicated") if r.get("adjudicated") in ("match", "mismatch") else r["verdict"]
            if v not in ("match",):
                causes[v][str((r.get("why2") or r.get("why") or [""])[0])[:100]] += 1
        for k, cc in causes.items():
            print(f"  -- {k}")
            for w, n in cc.most_common(8):
                print(f"     {n:6}  {w}")


if __name__ == "__main__":
    m, rest = sys.argv[1], sys.argv[2:]
    {"thingi": thingi, "abc": abc, "files": files, "urls": urls, "report": report}[m](rest)
