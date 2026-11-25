#!/usr/bin/env python3
"""Corpus equality for the assembly overlap check: every example assembly and its edited
variants (each of the first four `shared` numbers grown 5%, plus the two thread repros), built
by BIN_A (with ENV_A) and BIN_B (with ENV_B), each in a fresh process with FORGECAD_CACHE=off.
The whole `build --json` output must match with timings removed: every verdict, interference,
volume, clearance and gap point, bit for bit (floats compare by their printed digits, which
round-trip). Prints one line a job and a summary; exits 1 on any difference.
usage: corpus_equal.py BIN_A ENV_A BIN_B ENV_B OUT.jsonl [JOBS]
ENV_x: comma-separated NAME=VALUE, or - for none."""
import json, os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
BIN_A, ENV_A, BIN_B, ENV_B, OUT = sys.argv[1:6]
J = int(sys.argv[6]) if len(sys.argv) > 6 else 2
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../..")

def envs(spec):
    e = dict(os.environ, FORGECAD_CACHE="off")
    if spec != "-":
        for kv in spec.split(","):
            k, v = kv.split("=", 1); e[k] = v
    return e

files = sorted(os.path.join(d, f) for d, _, fs in os.walk(os.path.join(ROOT, "examples")) for f in fs if f.endswith(".fg") and not f.startswith("zz_tmp_"))
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

def strip(x):
    if isinstance(x, dict):
        return {k: strip(v) for k, v in x.items() if k not in ("timings_ms",) and not k.endswith("_ms")}
    if isinstance(x, list):
        return [strip(v) for v in x]
    return x

def run(k):
    f, var, text = jobs[k]
    d = os.path.dirname(f); tmp = os.path.join(d, f"zz_tmp_eq_{k}.fg")
    open(tmp, "w").write(text)
    out = []
    try:
        for b, e in ((BIN_A, ENV_A), (BIN_B, ENV_B)):
            p = subprocess.run([b, "build", os.path.basename(tmp), "--json"], cwd=d, capture_output=True, text=True, env=envs(e), timeout=3600)
            try: j = strip(json.loads(p.stdout))
            except Exception: j = {"unparsed": p.stdout[-2000:], "stderr": p.stderr[-2000:]}
            out.append((p.returncode, j))
    finally:
        os.remove(tmp)
    a = out[0][1].get("assembly") or {} if isinstance(out[0][1], dict) else {}
    return {"doc": os.path.relpath(f, ROOT), "variant": var, "same": out[0] == out[1],
            "rc": [out[0][0], out[1][0]], "candidates": a.get("overlap", {}).get("candidates") if isinstance(a.get("overlap"), dict) else None,
            "diff": None if out[0] == out[1] else [json.dumps(out[0])[:4000], json.dumps(out[1])[:4000]]}

bad = 0
with open(OUT, "w") as o, ThreadPoolExecutor(J) as ex:
    for r in ex.map(run, range(len(jobs))):
        bad += not r["same"]
        o.write(json.dumps(r) + "\n"); o.flush()
        print(("same " if r["same"] else "DIFF ") + r["doc"] + " " + r["variant"] + " rc=" + str(r["rc"]), flush=True)
print(f"{len(jobs)} builds, {len(jobs) - bad} the same, {bad} different")
sys.exit(1 if bad else 0)
