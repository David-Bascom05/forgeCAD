#!/usr/bin/env python3
"""Rates the reader over a dataset against OCCT's reading of the same files: reads, valid, volume,
area and box match, crashes and timeouts. One child per file, memory and time capped; files go to
it on stdin as data. Run with python3 -I.

  rate.py BIN OUT.jsonl --zip Z --ids IDS --truth TRUTH.jsonl --name 'step/{id:.4}/{id}.step' [--jobs 4] [--timeout 60]
  rate.py BIN OUT.jsonl --dir D [--truth ..]
  rate.py --report OUT.jsonl..
"""
import argparse, collections, concurrent.futures as cf, json, os, resource, subprocess, sys, threading, zipfile

MEM = 3 << 30
lock = threading.Lock()


def limit():
    resource.setrlimit(resource.RLIMIT_AS, (MEM, MEM))
    os.nice(15)


def run(bin_, data, timeout, flags):
    try:
        p = subprocess.run([bin_] + flags, input=data, capture_output=True, timeout=timeout, preexec_fn=limit)
    except subprocess.TimeoutExpired:
        return {"crash": "timeout"}
    last = p.stdout.decode("utf8", "replace").strip().splitlines()[-1:] if p.stdout else []
    try:
        d = json.loads(last[0])
    except (IndexError, json.JSONDecodeError):
        err = p.stderr.decode("utf8", "replace")
        at = [l for l in err.splitlines() if "panicked" in l or "memory allocation" in l]
        return {"crash": f"exit {p.returncode}", "panic": (at[0] if at else err[-200:])[:200]}
    return d


def verdict(d, t):
    """Whether ours matches OCCT's truth, summing solids."""
    if "crash" in d or "err" in d or not d.get("solids"):
        return {}
    s = d["solids"]
    bad = any(x["problems"] for x in s)
    seen = None if any(x["vol"] is None or x["area"] is None for x in s) else s
    out = {"valid": not bad, "massok": seen is not None}
    if seen is None or not t:
        return out
    vol = sum(abs(x["vol"]) for x in seen)
    area = sum(x["area"] for x in seen)
    out["dvol"] = (vol - t["vol"]) / t["vol"] if t.get("vol") else None
    out["darea"] = (area - t["area"]) / t["area"] if t.get("area") else None
    boxes = [x["box"] for x in seen if x["box"]]
    tb = t.get("bbox") or t.get("box")
    if boxes and tb and len(boxes) == len(seen):
        lo = [min(b[k] for b in boxes) for k in range(3)]
        hi = [max(b[k + 3] for b in boxes) for k in range(3)]
        diag = max(1e-9, sum((tb[k + 3] - tb[k]) ** 2 for k in range(3)) ** 0.5)
        out["dbox"] = max(abs(a - b) for a, b in zip(lo + hi, tb)) / diag
    out["match"] = bool(out.get("dvol") is not None and abs(out["dvol"]) < 1e-3 and abs(out["darea"]) < 1e-3)
    out["match1"] = bool(out.get("dvol") is not None and abs(out["dvol"]) < 1e-2 and abs(out["darea"]) < 1e-2)
    return out


def report(paths):
    for path in paths:
        rows = [json.loads(l) for l in open(path)]
        c = collections.Counter()
        probs, refs, panics, syst = collections.Counter(), collections.Counter(), collections.Counter(), collections.defaultdict(collections.Counter)
        for r in rows:
            sys_ = (r.get("system") or "?")[:24]
            c["files"] += 1
            syst[sys_]["files"] += 1
            if "crash" in r:
                k = "timeout" if r["crash"] == "timeout" else "crash"
                c[k] += 1
                syst[sys_][k] += 1
                panics[r.get("panic", r["crash"])[:90]] += 1
                continue
            if "err" in r:
                c["parse_err"] += 1
                continue
            v = r.get("v", {})
            if r.get("refusals"):
                c["refused"] += 1
                for k, n in r["refusals"].items():
                    refs[k] += 1
            if not r.get("solids"):
                continue
            c["read"] += 1
            syst[sys_]["read"] += 1
            if v.get("valid"):
                c["valid"] += 1
                syst[sys_]["valid"] += 1
            for x in r["solids"]:
                for k in x["problems"]:
                    probs[k] += 1
            if v.get("match"):
                c["match"] += 1
                syst[sys_]["match"] += 1
                if v.get("valid"):
                    c["valid+match"] += 1
            if v.get("match1"):
                c["match1"] += 1
            if "dvol" in v:
                c["compared"] += 1
            if v.get("dbox") is not None and v["dbox"] < 1e-3:
                c["box"] += 1
        n = c["files"] or 1
        print(f"== {path}")
        print("  ", " ".join(f"{k}={c[k]} ({100*c[k]/n:.1f}%)" for k in ["files", "read", "valid", "compared", "match", "match1", "valid+match", "box", "refused", "parse_err", "crash", "timeout"]))
        print("   problems(files):", probs.most_common(14))
        print("   refused(files):", refs.most_common(14))
        print("   crash:", panics.most_common(6))
        for s, k in sorted(syst.items(), key=lambda x: -x[1]["files"])[:8]:
            print("   system", s, dict(k))
        ed = collections.defaultdict(collections.Counter)
        for r in rows:
            for x in r.get("solids", []):
                for k, v in x.get("edit", {}).items():
                    ed[k][v.split(":")[0] if v.startswith(("err", "volume")) else v] += 1
        for k in sorted(ed):
            print(f"   edit {k}:", ed[k].most_common(8))
        q = sorted(r.get("coff", 0) for r in rows if r.get("solids"))
        if q:
            pick = lambda f: q[min(len(q) - 1, int(f * len(q)))]
            print("   curve off surface mm p50/p90/p99/max:", pick(0.5), pick(0.9), pick(0.99), q[-1])
        q = sorted(r.get("voff", 0) for r in rows if r.get("solids"))
        if q:
            pick = lambda f: q[min(len(q) - 1, int(f * len(q)))]
            print("   vertex off curve mm p50/p90/p99/max:", pick(0.5), pick(0.9), pick(0.99), q[-1])
        pairs = collections.defaultdict(lambda: [0, 0, 0.0])
        for r in rows:
            for k, v in r.get("pairs", {}).items():
                pairs[k][0] += v[0]
                pairs[k][1] += v[1]
                pairs[k][2] = max(pairs[k][2], v[2])
        print("   edges on surfaces (kind: uses, off > 1e-5, worst):")
        for k, v in sorted(pairs.items(), key=lambda x: -x[1][1])[:14]:
            print(f"     {k:34s} {v[0]:8d} {v[1]:8d} {v[2]:.2e}")
        ms = sorted(r.get("read_ms", 0) for r in rows if r.get("solids"))
        if ms:
            print("   read ms p50/p99/max:", ms[len(ms) // 2], ms[int(0.99 * len(ms))], ms[-1])


def main():
    a = argparse.ArgumentParser()
    a.add_argument("bin", nargs="?")
    a.add_argument("out", nargs="?")
    a.add_argument("--zip")
    a.add_argument("--dir")
    a.add_argument("--ids")
    a.add_argument("--truth")
    a.add_argument("--name", default="{id}")
    a.add_argument("--stem", action="store_true", help="find entries by file stem, wherever in the zip")
    a.add_argument("--jobs", type=int, default=4)
    a.add_argument("--timeout", type=int, default=60)
    a.add_argument("--limit", type=int, default=0)
    a.add_argument("--flags", default="")
    a.add_argument("--report", nargs="*")
    o = a.parse_args()
    if o.report is not None:
        return report(o.report)
    truth = {}
    if o.truth:
        for l in open(o.truth):
            try:
                d = json.loads(l)
            except json.JSONDecodeError:
                continue
            truth[str(d.get("id"))] = d
    done = set()
    if os.path.exists(o.out):
        done = {json.loads(l)["id"] for l in open(o.out)}
    z = zipfile.ZipFile(o.zip) if o.zip else None
    if o.dir:
        ids = sorted(os.path.join(p, f) for p, _, fs in os.walk(o.dir) for f in fs if f.lower().endswith((".step", ".stp")))
    elif o.ids:
        ids = [l.strip() for l in open(o.ids) if l.strip()]
    else:
        ids = sorted(truth)
    stems = {}
    if o.stem and z:
        stems = {os.path.splitext(os.path.basename(n))[0]: n for n in z.namelist()}
    ids = [i for i in ids if i not in done]
    if o.limit:
        ids = ids[: o.limit]
    flags = o.flags.split()
    out = open(o.out, "a")

    def job(i):
        try:
            if z:
                with lock:
                    data = z.read(stems[i] if stems else o.name.format(id=i))
            else:
                with open(i, "rb") as f:
                    data = f.read()
        except (KeyError, OSError) as e:
            return {"id": i, "missing": str(e)[:80]}
        d = run(o.bin, data, o.timeout, flags)
        d["id"] = i
        d["v"] = verdict(d, truth.get(i))
        return d

    with cf.ThreadPoolExecutor(o.jobs) as ex:
        for n, d in enumerate(ex.map(job, ids)):
            if "missing" in d:
                continue
            out.write(json.dumps(d) + "\n")
            out.flush()
            if n % 200 == 0:
                print(n, len(ids), flush=True)
    print("done", o.out)


if __name__ == "__main__":
    main()
