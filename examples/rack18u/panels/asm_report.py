#!/usr/bin/env python3
"""Runs an assembly, prints its instances (optional -p), every non-info diagnostic, engagements and
expects. Usage: asm_report.py FILE [-p]"""
import json, os, subprocess, sys
FG = os.path.expanduser("~/projects/ForgeCAD/.claude/worktrees/forge-lang/target/release/fg")
p = subprocess.run(["fbuild", FG, "run", sys.argv[1], "--json"], capture_output=True, text=True)
d = json.loads(p.stdout.strip().splitlines()[-1])
a = d.get("assembly") or {}
r = lambda v: [round(x, 3) for x in v]
if "-p" in sys.argv:
    for i in a.get("instances", []):
        q = i["pose"]
        print(f"  {i['id']:14s} o={r(q['origin'])} x={r(q['x'])} z={r(q['z'])}")
for e in d.get("errors", []):
    print("ERROR", e["kind"], e.get("text", ""))
for w in d.get("warnings", []):
    if w.get("severity") not in ("info", "lint") and w["kind"] != "UnusedParam":
        print("WARN ", w["kind"], w.get("text", ""))
for e in a.get("engagements", []):
    print(f"engage {e['a']}/{e['b']} {e['rule']}: {e['engaged']} (min {e['min']}){' SHORT' if e['short'] else ''}")
print("mobility", a.get("mobility"), "interferences", a.get("interferences"), "intended", a.get("intended_contacts"))
src = open(sys.argv[1]).read().splitlines()
for x in a.get("expects", []):
    print(f"{'ok  ' if x['ok'] else 'FAIL'} L{x['line']:<4} {x['metric']:13s} measured {x.get('measured')}  | {src[x['line']-1].strip()[:110]}")
print("ok" if d.get("ok") else "NOT OK", d.get("timings_ms", {}).get("total"))
