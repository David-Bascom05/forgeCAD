#!/usr/bin/env python3
"""Rates the STEP reader against OCCT over whole datasets, streamed: a shard of files is fetched, judged and
deleted before the next, so disk holds one shard at a time. Resumable and deterministic: a corpus's ids are
sorted and cut into fixed shards of SHARD ids, a shard's results are one file, and a file id judged is never
judged again. Another session can take shard K..K2 of the same manifest.

  scale.py index CORPUS                     # writes scale/CORPUS.ids.gz: id, bytes, crc32, where (the manifest)
  scale.py run CORPUS K [K2] [--stride S] [--jobs 4]
                                            # judges shards K, K+S, .. < K2 into scale/CORPUS/K.jsonl.gz
  scale.py report [CORPUS..] [--causes N]   # per corpus: match/mismatch/uncertain/refused/crash/timeout, causes

Every file goes to its own process twice, each capped in time and memory: OCCT's reading under the truth rule
(occt_truth.py, research/import/truth.md) and ours (kernel/forge-step/examples/stepread.rs, as rate.py runs it).
Files are data: they are written to a scratch directory of their own, nothing in them runs, and the scripts run
with python -I from here. No dataset file is kept; the manifest keeps ids, sizes, CRCs and where each came from.

Classes, for each file:
- match: our solids valid, none warned, none refused, and volume and area within TOL of the truth;
- mismatch: we read a solid, and it is invalid, warned (`Body::unverified`) or off;
- uncertain: the truth is: OCCT's measures disagree (truth.md), OCCT's shape is invalid, OCCT failed or
  timed out, or OCCT finds no solid (then nothing to judge: `empty` if we find none either);
- refused: we read no solid, or refused one of the solids OCCT has, with a typed error;
- crash, timeout: ours.
"""
import argparse
import collections
import concurrent.futures as cf
import gzip
import json
import os
import resource
import shutil
import struct
import subprocess
import sys
import tempfile
import time
import urllib.request
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
OUT = os.path.join(HERE, "scale")
SHARD = 500
TOL = 1e-3
OURS_MEM, OURS_TIME = 3 << 30, 60
OCCT_TIME = 120
VENV = os.environ.get("VENV", os.path.expanduser("~/.local/share/forgecad-occt/venv")) + "/bin/python"
BIN = os.environ.get("STEPREAD", os.path.join(ROOT, "target/release/examples/stepread"))
SCRATCH = os.environ.get("SCALE_SCRATCH", os.path.join(tempfile.gettempdir(), "scale-step"))
UA = {"User-Agent": "forgecad-step-rate"}

CORPORA = {
    # licence: CC-BY-4.0 (Hugging Face card). One zip; members are read by HTTP range.
    "cadfs": {
        "zip": "https://huggingface.co/datasets/VladPyatov/CADFS/resolve/main/cadfs/dataset/step.zip",
        "licence": "CC-BY-4.0",
    },
    # the ABC dataset's STEP (Koch et al. 2019), as CADFS mirrors it in one zip readable by range (NYU's 7z
    # chunks take no range). Models' rights stay their creators' under Onshape's terms of use 1.g.ii; read here
    # for benchmarking only, nothing redistributed.
    "abc": {
        "zip": "https://huggingface.co/datasets/VladPyatov/CADFS/resolve/main/cadfs/raw/step_abc.zip",
        "licence": "Onshape Terms of Use 1.g.ii (ABC); mirror CC-BY-4.0",
    },
    # AutoMate (Jones et al. 2021): parts of real Onshape assemblies. Zenodo record 7776208.
    "automate": {
        "zip": "https://zenodo.org/api/records/7776208/files/step.zip/content",
        "licence": "CC0-1.0",
    },
    # NIST MBE PMI validation and conformance test models: "can be used without any restrictions".
    "nist": {
        "zip": "https://www.nist.gov/system/files/documents/noindex/2024/06/19/NIST-PMI-STEP-Files.zip",
        "licence": "NIST: no restrictions",
    },
}


def get(url, a=None, b=None, tries=6):
    h = dict(UA)
    if a is not None:
        h["Range"] = f"bytes={a}-{b}"
    for k in range(tries):
        try:
            with urllib.request.urlopen(urllib.request.Request(url, headers=h), timeout=300) as r:
                return r.read()
        except Exception as e:  # network: retried with backoff, then given up
            if k == tries - 1:
                raise
            print("retry", url[:60], e, file=sys.stderr, flush=True)
            time.sleep(2 ** k)


# ---- zips read by range


def zip_index(url):
    """Every member of a remote zip: (name, method, csize, usize, crc, local header offset)."""
    req = urllib.request.Request(url, method="HEAD", headers=UA)
    size = int(urllib.request.urlopen(req, timeout=60).headers["Content-Length"])
    tail = get(url, size - 66000, size - 1)
    e = tail.rfind(b"PK\x05\x06")
    cd_n, cd_size, cd_off = struct.unpack("<HII", tail[e + 10 : e + 20])
    z64 = tail.rfind(b"PK\x06\x07")
    if z64 >= 0:
        (e64,) = struct.unpack("<Q", tail[z64 + 8 : z64 + 16])
        cd_n, cd_size, cd_off = struct.unpack("<QQQ", get(url, e64, e64 + 55)[32:56])
    cd = get(url, cd_off, cd_off + cd_size - 1)
    out, i = [], 0
    while i < len(cd) and cd[i : i + 4] == b"PK\x01\x02":
        (meth,) = struct.unpack("<H", cd[i + 10 : i + 12])
        crc, csize, usize = struct.unpack("<III", cd[i + 16 : i + 28])
        nl, xl, cl = struct.unpack("<HHH", cd[i + 28 : i + 34])
        (lho,) = struct.unpack("<I", cd[i + 42 : i + 46])
        name = cd[i + 46 : i + 46 + nl].decode("utf-8", "replace")
        extra = cd[i + 46 + nl : i + 46 + nl + xl]
        j = 0
        while j + 4 <= len(extra):
            hid, hl = struct.unpack("<HH", extra[j : j + 4])
            if hid == 1:
                vals, k = extra[j + 4 : j + 4 + hl], 0
                if usize == 0xFFFFFFFF:
                    (usize,) = struct.unpack("<Q", vals[k : k + 8])
                    k += 8
                if csize == 0xFFFFFFFF:
                    (csize,) = struct.unpack("<Q", vals[k : k + 8])
                    k += 8
                if lho == 0xFFFFFFFF:
                    (lho,) = struct.unpack("<Q", vals[k : k + 8])
            j += 4 + hl
        out.append((name, meth, csize, usize, crc, lho))
        i += 46 + nl + xl + cl
    return out


def zip_fetch(url, rows, into):
    """Writes the members rows name into the directory, fetched in spans of neighbouring members. Returns
    {id: path}. A member whose CRC disagrees is left out."""
    rows = sorted(rows, key=lambda r: r["off"])
    spans, cur = [], []
    for r in rows:
        if cur and (r["off"] - (cur[-1]["off"] + cur[-1]["csize"] + 1024) > (1 << 20) or
                    r["off"] + r["csize"] - cur[0]["off"] > (64 << 20)):
            spans.append(cur)
            cur = []
        cur.append(r)
    if cur:
        spans.append(cur)
    got = {}

    def fetch(sp):
        a = sp[0]["off"]
        b = sp[-1]["off"] + 30 + 1024 + sp[-1]["csize"] + 65535
        blob = get(url, a, b)
        for r in sp:
            o = r["off"] - a
            nl, xl = struct.unpack("<HH", blob[o + 26 : o + 30])
            d = blob[o + 30 + nl + xl : o + 30 + nl + xl + r["csize"]]
            raw = zlib.decompress(d, -15) if r["meth"] == 8 else d
            if zlib.crc32(raw) != r["crc"]:
                print("crc", r["id"], file=sys.stderr)
                continue
            p = os.path.join(into, r["id"] + ".step")
            with open(p, "wb") as f:
                f.write(raw)
            got[r["id"]] = p

    with cf.ThreadPoolExecutor(8) as ex:
        list(ex.map(fetch, spans))
    return got


# ---- manifest


def manifest_path(c):
    """The full index (where each member is) is cached outside the repository and rebuilt by `index`; the
    repository keeps CORPUS.manifest.json (source, licence, count, a hash of the sorted ids) and, per file
    judged, its id, size and CRC in the results."""
    return os.path.join(SCRATCH, f"{c}.ids.gz")


def index(c):
    cfg = CORPORA[c]
    rows = []
    for name, meth, csize, usize, crc, lho in zip_index(cfg["zip"]):
        if not name.lower().endswith((".step", ".stp")) or usize == 0:
            continue
        rows.append({"id": os.path.splitext(os.path.basename(name))[0], "name": name, "bytes": usize,
                     "crc": crc, "meth": meth, "csize": csize, "off": lho})
    if len({r["id"] for r in rows}) != len(rows):  # stems repeat: the member's path is the id
        for r in rows:
            r["id"] = os.path.splitext(r["name"])[0].replace("/", "~")
    rows.sort(key=lambda r: r["id"])
    ids = [r["id"] for r in rows]
    assert len(set(ids)) == len(ids), "ids not unique"
    os.makedirs(OUT, exist_ok=True)
    os.makedirs(SCRATCH, exist_ok=True)
    import hashlib
    summary = {"corpus": c, "source": cfg.get("zip"), "licence": cfg["licence"], "files": len(rows),
               "bytes": sum(r["bytes"] for r in rows), "shard": SHARD, "order": "ids sorted as strings",
               "ids_sha256": hashlib.sha256("\n".join(ids).encode()).hexdigest(),
               "indexed": time.strftime("%Y-%m-%d")}
    with open(os.path.join(OUT, f"{c}.manifest.json"), "w") as f:
        json.dump(summary, f, indent=1)
        f.write("\n")
    with gzip.open(manifest_path(c), "wt") as f:
        f.write(f"# {c}: {cfg.get('zip')} licence {cfg['licence']}; id bytes crc32 method csize offset member\n")
        for r in rows:
            f.write(f"{r['id']}\t{r['bytes']}\t{r['crc']:08x}\t{r['meth']}\t{r['csize']}\t{r['off']}\t{r['name']}\n")
    print(c, len(rows), "files", sum(r["bytes"] for r in rows) / 1e9, "GB", file=sys.stderr)


def manifest(c):
    rows = []
    with gzip.open(manifest_path(c), "rt") as f:
        for l in f:
            if l.startswith("#"):
                continue
            i, b, crc, meth, cs, off, name = l.rstrip("\n").split("\t")
            rows.append({"id": i, "bytes": int(b), "crc": int(crc, 16), "meth": int(meth), "csize": int(cs),
                         "off": int(off), "name": name})
    return rows


# ---- judging one file


def cap(mem):
    def f():
        resource.setrlimit(resource.RLIMIT_AS, (mem, mem))
        os.nice(10)
    return f


def child(cmd, timeout, mem, stdin=None):
    t = time.time()
    try:
        p = subprocess.run(cmd, input=stdin, capture_output=True, timeout=timeout, preexec_fn=cap(mem), cwd=HERE)
    except subprocess.TimeoutExpired:
        return {"crash": "timeout", "s": round(time.time() - t, 1)}
    last = p.stdout.decode("utf8", "replace").strip().splitlines()[-1:] if p.stdout else []
    try:
        return json.loads(last[0])
    except (IndexError, json.JSONDecodeError):
        err = p.stderr.decode("utf8", "replace")
        at = [l for l in err.splitlines() if "panicked" in l or "memory allocation" in l or "Error" in l]
        return {"crash": f"exit {p.returncode}", "panic": (at[0] if at else err[-200:]).strip()[:200]}


def truth_of(o, ours_n):
    """(volume, area) to judge by, or (None, why) where the truth is uncertain: the truth rule (truth.md) at
    TOL, on all solids, or on the distinct ones where we read as many solids as OCCT has distinct."""
    t = o["dist"] if ours_n == o.get("distinct") and ours_n != o.get("solids") else o["all"]
    v, near = t["vol_gk"], TOL / 10
    agree = lambda x: abs(x - v) <= near * max(v, 1e-12)
    cand = [a for a, w in ((t["area_eps"], t["vol_eps"]), (t["area"], t["vol"])) if agree(w)]
    if not cand:
        return None, "occt-volumes-disagree"
    if len(cand) == 2 and abs(cand[0] - cand[1]) > near * max(abs(cand[0]), 1e-12):
        return None, "occt-areas-disagree"
    return (v, cand[0]), None


def kind(s):
    return s.split(":")[0].split(" ")[0][:40]


def why(d):
    """A refusal's kind and the first words of its reason: '#12 KIND: words..' to 'KIND: words'."""
    w = d.get("why", "")
    k, _, rest = w.partition(": ")
    k = k.split(" ")[-1] if k else max(d["refusals"], key=d["refusals"].get)
    words = [x for x in rest.split()[:6] if not any(ch.isdigit() for ch in x)]
    return (k + ": " + " ".join(words[:5])).strip(": ")


def judge(o, d):
    """The class and the cause of one file, from OCCT's reading o and ours d."""
    if "crash" in d:
        if d["crash"] == "timeout":
            return "timeout", "timeout"
        p = d.get("panic", "")
        site = p.split("panicked at ")[-1].split(":")[0] if "panicked" in p else p[:60]
        return "crash", "crash:" + site
    oerr = o.get("crash") or o.get("err")
    if oerr:
        return "uncertain", "occt-" + ("timeout" if oerr == "timeout" else "failed")
    sol = d.get("solids") or []
    if not o.get("solids"):
        return ("empty", "no-solid-either") if not sol else ("uncertain", "occt-no-solid")
    if "err" in d:
        return "refused", "error:" + kind(d["err"])
    if not sol:
        r = d.get("refusals") or {}
        return "refused", ("refused:" + why(d)) if r else "no-solid"
    if not o.get("valid"):
        return "uncertain", "occt-invalid"
    probs = collections.Counter()
    for x in sol:
        probs.update(x.get("problems") or {})
    if probs:
        return "mismatch", "invalid:" + probs.most_common(1)[0][0]
    if any(x.get("warn") for x in sol):
        return "mismatch", "warned:" + " ".join(next(x["warn"] for x in sol if x.get("warn")).split()[:4])
    if any(x.get("vol") is None or x.get("area") is None for x in sol):
        return "mismatch", "no-mass:" + kind(next(x.get("merr", "") for x in sol if x.get("vol") is None or x.get("area") is None) or "?")
    if d.get("refusals"):
        return "refused", "refused-some:" + why(d)
    t, unsure = truth_of(o, len(sol))
    if t is None:
        return "uncertain", unsure
    vol = sum(abs(x["vol"]) for x in sol)
    area = sum(x["area"] for x in sol)
    dv = (vol - t[0]) / t[0] if t[0] else float("inf")
    da = (area - t[1]) / t[1] if t[1] else float("inf")
    if abs(dv) < TOL and abs(da) < TOL:
        return "match", ""
    if len(sol) != o["solids"] and len(sol) != o["distinct"]:
        return "mismatch", "off:solid-count"
    return "mismatch", "off:" + ("volume" if abs(dv) >= TOL else "area") + (" 1e-2+" if max(abs(dv), abs(da)) >= 1e-2 else "")


def compact(o, d, cls, cause):
    """The row kept for a file: enough to judge it again under another rule without re-reading."""
    sol = d.get("solids") or []
    row = {"cls": cls, "cause": cause}
    if "crash" not in o and "err" not in o:
        row["occt"] = {k: o.get(k) for k in ("valid", "solids", "distinct", "faces", "all", "dist", "secs")}
    else:
        row["occt"] = {"err": o.get("crash") or o.get("err")}
    if "crash" in d:
        row["ours"] = {"crash": d["crash"], "panic": d.get("panic", "")}
    elif "err" in d:
        row["ours"] = {"err": d["err"][:160]}
    else:
        row["ours"] = {
            "system": d.get("system", "")[:40], "schema": d.get("schema", "")[:30], "ms": d.get("all_ms"),
            "unc": d.get("unc"), "mm": d.get("mm"), "faces": d.get("faces"), "tolerant": d.get("tolerant"),
            "refusals": d.get("refusals") or {}, "why": d.get("why", "")[:160],
            "solids": [{k: x.get(k) for k in ("faces", "vol", "area", "problems", "first", "warn", "merr", "mesh")}
                       for x in sol[:50]],
            "n": len(sol),
        }
    return row


def one(path):
    o = child([VENV, "-I", os.path.join(HERE, "occt_truth.py"), path], OCCT_TIME, 5 << 30)
    with open(path, "rb") as f:
        data = f.read()
    d = child([BIN], OURS_TIME, OURS_MEM, stdin=data)
    cls, cause = judge(o, d)
    return compact(o, d, cls, cause)


# ---- shards


def shard_path(c, k):
    return os.path.join(OUT, c, f"{k:05d}.jsonl.gz")


def run(c, k0, k1, jobs, stride=1):
    rows = manifest(c)
    url = CORPORA[c]["zip"]
    nshards = (len(rows) + SHARD - 1) // SHARD
    os.makedirs(os.path.join(OUT, c), exist_ok=True)
    os.makedirs(SCRATCH, exist_ok=True)
    for k in range(k0, min(k1, nshards), stride):
        final = shard_path(c, k)
        if os.path.exists(final):
            continue
        part = os.path.join(SCRATCH, f"{c}-{k:05d}.partial.jsonl")
        done = set()
        if os.path.exists(part):
            for l in open(part):
                try:
                    done.add(json.loads(l)["id"])
                except (json.JSONDecodeError, KeyError):
                    pass
        todo = [r for r in rows[k * SHARD : (k + 1) * SHARD] if r["id"] not in done]
        t0 = time.time()
        d = tempfile.mkdtemp(prefix=f"{c}-{k}-", dir=SCRATCH)
        try:
            got = zip_fetch(url, todo, d) if todo else {}
            with open(part, "a") as out, cf.ThreadPoolExecutor(jobs) as ex:
                futs = {ex.submit(one, got[r["id"]]): r for r in todo if r["id"] in got}
                for r in todo:
                    if r["id"] not in got:
                        out.write(json.dumps({"id": r["id"], "bytes": r["bytes"], "cls": "fetch-failed", "cause": "crc"}) + "\n")
                for f in cf.as_completed(futs):
                    r = futs[f]
                    row = {"id": r["id"], "bytes": r["bytes"], "crc": f"{r['crc']:08x}", **f.result()}
                    out.write(json.dumps(row, separators=(",", ":")) + "\n")
                    out.flush()
                    os.remove(got[r["id"]])
        finally:
            shutil.rmtree(d, ignore_errors=True)
        lines = sorted(open(part).read().splitlines(), key=lambda l: json.loads(l)["id"])
        with gzip.open(final + ".tmp", "wt") as f:
            f.write("\n".join(lines) + "\n")
        os.replace(final + ".tmp", final)
        os.remove(part)
        cl = collections.Counter(json.loads(l)["cls"] for l in lines)
        print(time.strftime("%H:%M:%S"), c, k, f"{time.time() - t0:.0f}s", dict(cl), flush=True)


# ---- report


CLASSES = ["match", "mismatch", "uncertain", "refused", "crash", "timeout", "empty", "fetch-failed"]


def rejudge(r):
    """The class and cause of a stored row under the rule as it is now (the row keeps what judge reads)."""
    if r["cls"] == "fetch-failed":
        return r
    r["cls"], r["cause"] = judge(r["occt"], r["ours"])
    return r


def load(c):
    d = os.path.join(OUT, c)
    out = []
    for f in sorted(os.listdir(d)) if os.path.isdir(d) else []:
        if f.endswith(".jsonl.gz"):
            with gzip.open(os.path.join(d, f), "rt") as g:
                out += [rejudge(json.loads(l)) for l in g if l.strip()]
    return out


def report(cs, ncause):
    cs = cs or sorted(x for x in os.listdir(OUT) if os.path.isdir(os.path.join(OUT, x)))
    for c in cs:
        rows = load(c)
        n = len(rows) or 1
        cl = collections.Counter(r["cls"] for r in rows)
        print(f"== {c}: {len(rows)} files")
        print("  ", "  ".join(f"{k} {cl[k]} ({100 * cl[k] / n:.1f}%)" for k in CLASSES if cl[k]))
        judged = cl["match"] + cl["mismatch"] + cl["refused"] + cl["crash"] + cl["timeout"]
        if judged:
            print(f"   of the {judged} with a certain truth: match {100 * cl['match'] / judged:.1f}%")
        cause = collections.Counter(r["cause"] for r in rows if r["cls"] not in ("match", "empty"))
        print("   causes:")
        for k, m in cause.most_common(ncause):
            print(f"     {m:6d}  {k}")
        mesh = collections.Counter()
        for r in rows:
            for x in (r.get("ours") or {}).get("solids") or []:
                mesh[kind(x.get("mesh") or "none") if not (x.get("mesh") or "").startswith("tolerant") else "tolerant"] += 1
        print("   mesh (solids):", dict(mesh))
        sysc = collections.defaultdict(collections.Counter)
        for r in rows:
            s = ((r.get("ours") or {}).get("system") or "?")[:20]
            sysc[s][r["cls"]] += 1
        for s, k in sorted(sysc.items(), key=lambda x: -sum(x[1].values()))[:5]:
            print("   system", repr(s), dict(k))


def main():
    a = argparse.ArgumentParser()
    a.add_argument("cmd", choices=["index", "run", "report"])
    a.add_argument("args", nargs="*")
    a.add_argument("--jobs", type=int, default=4)
    a.add_argument("--causes", type=int, default=15)
    a.add_argument("--stride", type=int, default=1, help="every STRIDE-th shard from K, so a sample spans the corpus")
    o = a.parse_args()
    if o.cmd == "index":
        for c in o.args:
            index(c)
    elif o.cmd == "run":
        c, k0 = o.args[0], int(o.args[1])
        k1 = int(o.args[2]) if len(o.args) > 2 else k0 + 1
        run(c, k0, k1, o.jobs, o.stride)
    else:
        report(o.args, o.causes)


if __name__ == "__main__":
    main()
