#!/usr/bin/env python3
"""DWG: forge-dwg's reader and writer against LibreDWG's dwgread (0.13.3), file by file, by a rule
fixed before any result was looked at (research/formats/real-files.md, "DWG").

  python3 -I dwg_judge.py run PROBE DWGREAD OUT.jsonl FILE.. [@LIST] [--jobs 3] [--timeout 120]
  python3 -I dwg_judge.py write PROBE DWGREAD OUT.jsonl FILE..   read, write back, both read it
  python3 -I dwg_judge.py report OUT.jsonl..

Objects are keyed as the file types them: a fixed type's number (below 500), a class's DXF name.
Compared: the count of every key over all objects; of entities alone; the layers' names (as a
multiset). Match is all three equal. One process per file each side, memory capped.
"""
import collections, concurrent.futures as cf, json, os, resource, subprocess, sys, tempfile

MEM = 4 << 30
# types the DWG spec gives a fixed number that a file may also list as a class
DUAL = {"LWPOLYLINE": 77, "HATCH": 78, "XRECORD": 79, "ACDBPLACEHOLDER": 80, "PLACEHOLDER": 80, "LAYOUT": 82}


def limit():
    resource.setrlimit(resource.RLIMIT_AS, (MEM, MEM))


def ours(probe, f, timeout):
    try:
        p = subprocess.run([probe, "dwg", f], capture_output=True, timeout=timeout, preexec_fn=limit)
    except subprocess.TimeoutExpired:
        return {"crash": "timeout"}
    for l in reversed(p.stdout.decode("utf8", "replace").splitlines()):
        try:
            d = json.loads(l)
            return {"crash": "panic", "panic": d["panic"]} if "panic" in d else d
        except json.JSONDecodeError:
            pass
    err = p.stderr.decode("utf8", "replace")
    at = [l for l in err.splitlines() if "panicked" in l or "memory allocation" in l or "overflow" in l]
    return {"crash": f"exit {p.returncode}", "panic": (at[0] if at else err[-200:])[:200]}


def theirs(dwgread, f, timeout):
    with tempfile.TemporaryDirectory() as t:
        out = os.path.join(t, "o.json")
        try:
            p = subprocess.run([dwgread, "-O", "json", "-o", out, f], capture_output=True, timeout=timeout, preexec_fn=limit)
        except subprocess.TimeoutExpired:
            return {"err": "dwgread timeout"}
        try:
            d = json.load(open(out, encoding="utf8", errors="replace"))
        except Exception as e:
            return {"err": f"dwgread exit {p.returncode}: {p.stderr.decode('utf8', 'replace')[-120:]}"}
    classes = {c["number"]: c.get("dxfname", "") for c in d.get("CLASSES", [])}
    names = set(classes.values())
    kinds, ents, layers = collections.Counter(), collections.Counter(), []
    for o in d.get("OBJECTS", []):
        t = o.get("type", -1)
        k = str(t) if t < 500 else classes.get(t, f"class {t}")
        # dwgread's JSON gives some class objects a fixed number (MLEADERSTYLE as 1, TEXT's):
        # an object named as one of the file's classes is keyed by that class
        # (the types with both a fixed number and a class keep the number when given it)
        name = o.get("dxfname") or o.get("object") or o.get("entity")
        if name in names and DUAL.get(name) != t:
            k = name
        kinds[k] += 1
        if "entity" in o:
            ents[k] += 1
        if o.get("object") == "LAYER":
            layers.append(o.get("name", ""))
    return {"rc": p.returncode, "version": d.get("FILEHEADER", {}).get("version", ""), "objects": sum(kinds.values()),
            "kinds": dict(kinds), "entities": dict(ents), "layers": layers}


def verdict(a, b):
    if "crash" in a:
        return ("timeout" if a["crash"] == "timeout" else "crash"), [a.get("panic", a["crash"])]
    if "err" in a and "err" in b:
        return "both-refused", [a["err"]]
    if "err" in a:
        return "refused", [a["err"]]
    if "err" in b:
        return "ref-refused", [b["err"]]
    why = []
    for name in ("kinds", "entities"):
        x, y = a[name], b[name]
        d = {k: (x.get(k, 0), y.get(k, 0)) for k in set(x) | set(y) if x.get(k, 0) != y.get(k, 0)}
        if d:
            why.append(f"{name} {dict(sorted(d.items())[:6])}")
    if sorted(a["layers"]) != sorted(b["layers"]):
        why.append(f"layers {sorted(a['layers'])[:5]} vs {sorted(b['layers'])[:5]}")
    return ("match" if not why else "mismatch"), why


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
    probe, dwgread, out, fs = argv[0], argv[1], argv[2], files(argv[3:])

    def one(f):
        a, b = ours(probe, f, timeout), theirs(dwgread, f, timeout)
        v, why = verdict(a, b)
        return {"file": f, "verdict": v, "why": why, "ours": a, "ref": b}

    with open(out, "w") as o, cf.ThreadPoolExecutor(jobs) as ex:
        for r in ex.map(one, fs):
            o.write(json.dumps(r) + "\n")
            o.flush()


def write(argv):
    """The writer: each file ours reads is written back in its version (forge_dwg::file::write)
    and the written file must read, by dwgread, as the original reads by dwgread (the same rule),
    and by ours as the original reads by ours."""
    jobs, timeout = opt(argv, "--jobs", 3), opt(argv, "--timeout", 120)
    probe, dwgread, out, fs = argv[0], argv[1], argv[2], files(argv[3:])
    tmp = out + ".d"
    os.makedirs(tmp, exist_ok=True)

    def one(k_f):
        k, f = k_f
        w = os.path.join(tmp, f"{k}.dwg")
        try:
            p = subprocess.run([probe, "dwg-rewrite", f, w], capture_output=True, timeout=timeout, preexec_fn=limit)
            r = json.loads(p.stdout.decode().strip().splitlines()[-1])
        except Exception as e:
            return {"file": f, "verdict": "crash", "why": [str(e)[:120]]}
        if "panic" in r:
            return {"file": f, "verdict": "crash", "why": [r["panic"]]}
        if "err" in r:
            return {"file": f, "verdict": "not-read" if r.get("stage") != "write" else "not-written", "why": [r["err"]]}
        b0, b1 = theirs(dwgread, f, timeout), theirs(dwgread, w, timeout)
        a0, a1 = ours(probe, f, timeout), ours(probe, w, timeout)
        os.remove(w)
        if "err" in b0:
            v, why = "ref-refused-source", [b0["err"]]
        elif "err" in b1:
            v, why = "ref-refused-written", [b1["err"]]
        else:
            v, why = verdict(b1, b0)
        v2, why2 = verdict(a1, a0)
        return {"file": f, "verdict": v, "why": why, "ours_again": v2, "ours_why": why2}

    with open(out, "w") as o, cf.ThreadPoolExecutor(jobs) as ex:
        for r in ex.map(one, enumerate(fs)):
            o.write(json.dumps(r) + "\n")
            o.flush()


def report(paths):
    for p in paths:
        rows = [json.loads(l) for l in open(p)]
        print(f"== {p}: {len(rows)}")
        for k, n in collections.Counter(r["verdict"] for r in rows).most_common():
            print(f"  {k:20} {n}")
        if "ours_again" in rows[0]:
            print("  ours again:", dict(collections.Counter(r.get("ours_again") for r in rows)))
        causes = collections.defaultdict(collections.Counter)
        for r in rows:
            if r["verdict"] != "match":
                causes[r["verdict"]][str(r["why"][0])[:100] if r["why"] else ""] += 1
        for k, cc in causes.items():
            print(f"  -- {k}")
            for w, n in cc.most_common(8):
                print(f"     {n:5}  {w}")


if __name__ == "__main__":
    {"run": run, "write": write, "report": report}[sys.argv[1]](sys.argv[2:])
