#!/usr/bin/env python3
"""Overlap check over the example assemblies and edited variants, for one forgecad binary.
For each document: overlap ms, exact runs, certified, and each interference (pair, confirmed,
volume, volume_at_least, boolean_failed). Variants: each of the first four `shared` numbers
grown 5% (as tests/asm_certify.rs), plus the two thread repros (6 -> 6.3). Variant texts are
written beside their original (relative paths) as zz_tmp_*.fg and removed.
usage: overlap_corpus.py BIN CPU OUT.jsonl"""
import json, os, re, subprocess, sys
BIN, CPU, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../..")
env = dict(os.environ, FORGECAD_CACHE="off")
files = sorted(os.path.join(d, f) for d, _, fs in os.walk(os.path.join(ROOT, "examples")) for f in fs if f.endswith(".fg"))
asms = [f for f in files if any(l.strip() == "assembly" for l in open(f))]
jobs = []
for f in asms:
    text = open(f).read()
    jobs.append((f, "base", text))
    n = 0
    for l in text.splitlines():
        m = re.match(r"shared (\S+) = ([-0-9.eE]+)(.*)$", l)
        if not m or n >= 4: continue
        try: v = float(m.group(2))
        except ValueError: continue
        if v == 0: continue
        n += 1
        jobs.append((f, f"{m.group(1)}*1.05", text.replace(l, f"shared {m.group(1)} = {v * 1.05}{m.group(3)}", 1)))
for rel, name in [("examples/rack18u/hw/tests/hinge_pin.fg", "size_m6"), ("examples/rack18u/asm/post_kit.fg", "m6")]:
    f = os.path.join(ROOT, rel); t = open(f).read()
    jobs.append((f, f"{name}=6.3", t.replace(f"shared {name} = 6\n", f"shared {name} = 6.3\n", 1)))
with open(OUT, "w") as out:
    for k, (f, var, text) in enumerate(jobs):
        d = os.path.dirname(f); tmp = os.path.join(d, f"zz_tmp_{CPU}_{k}.fg")
        open(tmp, "w").write(text)
        try:
            p = subprocess.run(["taskset", "-c", CPU, BIN, "build", os.path.basename(tmp), "--json"], cwd=d, capture_output=True, text=True, env=env, timeout=600)
            j = json.loads(p.stdout)
        except Exception as e:
            j = {"error": str(e)}
        finally:
            os.remove(tmp)
        a = j.get("assembly", {}) or {}
        inter = [{k2: w.get(k2) for k2 in ("a", "b", "confirmed", "volume", "volume_at_least", "boolean_failed")} for w in j.get("warnings", []) if w.get("kind") == "Interference"]
        out.write(json.dumps({"doc": os.path.relpath(f, ROOT), "variant": var, "overlap_ms": (a.get("timings_ms") or {}).get("overlap"),
                              "total_ms": (j.get("timings_ms") or {}).get("total"), "overlap": a.get("overlap"), "interferences": inter}) + "\n")
        out.flush()
