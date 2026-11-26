"""P5 measurement: build every example assembly with a forgecad built from main plus
p5_hull_instrument.patch; each tested pair appends a JSON line to OUT/<doc>.jsonl.
usage: p5_run.py OUTDIR [BIN]"""
import os, subprocess, sys, json
ROOT=os.path.join(os.path.dirname(os.path.abspath(__file__)),'../..'); BIN=sys.argv[2] if len(sys.argv)>2 else ROOT+'/target/release/forgecad'; OUT=sys.argv[1]
files = sorted(os.path.join(d, f) for d, _, fs in os.walk(os.path.join(ROOT, "examples")) for f in fs if f.endswith(".fg"))
asms = [f for f in files if any(l.strip() == "assembly" for l in open(f))]
os.makedirs(OUT, exist_ok=True)
for f in asms:
    rel=os.path.relpath(f,ROOT); out=os.path.join(OUT, rel.replace('/','__')+'.jsonl')
    if os.path.exists(out): os.remove(out)
    env=dict(os.environ, FORGECAD_CACHE="off", FORGEASM_P5=out)
    p=subprocess.run([BIN,"build",os.path.basename(f),"--json"],cwd=os.path.dirname(f),capture_output=True,text=True,env=env,timeout=3600)
    print(rel, p.returncode, flush=True)
