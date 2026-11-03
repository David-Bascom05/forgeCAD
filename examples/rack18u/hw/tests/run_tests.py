#!/usr/bin/env python3
"""Runs every hardware part and stack test through fg and prints the stack table.

Each stack is run four ways:
  as written            must be ok: no errors, 0 interferences, no ShallowEngagement;
  without fit rules     each threaded pair must come back as an Interference (the rules matter);
  min_engagement 99     every covered pair comes back ShallowEngagement with Forge's measured
                        engagement (Forge reports the number only when it is short);
  SPEC section 8 minima the rules as SPEC rev B writes them (m4 2.77, m5 2.77, m6 4.78, m8 8.0).
usage: python3 hardware/tests/run_tests.py [--fg PATH]
"""
import json
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
HW = HERE.parent
FG = str(Path.home() / "projects/ForgeCAD/.claude/worktrees/forge-lang/target/release/fg")
if "--fg" in sys.argv:
    FG = sys.argv[sys.argv.index("--fg") + 1]

SPEC_MIN = {"m4": 2.77, "m5": 2.77, "m6": 4.78, "m8": 8.0}   # SPEC rev B section 8

# where each fastener ends (from its frame origin along +z) and where each nut's thread ends
TIP = {"iso7380_m6x10": 10, "iso7380_m5x10": 10, "iso7380_m5x12": 12, "iso4017_m8x20": 20,
       "iso7379_8x12_m6": 23, "iso7045_m4x35": 35, "rack_screw_m6x16": 16,
       "pem_fh_m6_20": 20, "pem_fh_m8_25": 25,
       "iso7380_m4x25": 25, "iso7380_m6x12": 12, "iso7045_m4x40": 40, "iso7379_8x16_m6": 27, "pem_fh_m4_12": 12}
END = {"pem_s_m6_0_t2": 1.88 + 4.08, "pem_s_m6_0_t3": 3.02 + 4.08, "pem_s_m6_0_t1": 1.52 + 4.08,
       "pem_ss_m5_0_t3": 3.02 + 2.0, "pem_s_m4_0_t2": 1.88 + 2.0, "iso10511_m5": 5.0,
       "iso10511_m6": 6.0, "iso10511_m8": 8.0, "cage_nut_m6": 8.03, "iso10511_m4": 5.0}
PITCH = {"m4": 0.7, "m5": 0.8, "m6": 1.0, "m8": 1.25}


def run(path, text=None):
    if text is not None:
        tmp = path.with_name(f".variant_{path.name}")
        tmp.write_text(text)
        path = tmp
    r = subprocess.run([FG, "run", str(path), "--json"], capture_output=True, text=True)
    if text is not None:
        path.unlink()
    return json.loads(r.stdout)


def kinds(d, kind):
    return [w for w in d["warnings"] if w["kind"] == kind]


def main():
    bad = 0
    print("parts")
    for f in sorted(HW.glob("*.fg")) + sorted((HERE / "plates").glob("*.fg")):
        d = run(f)
        lints = [w for w in d["warnings"] if w.get("severity") in ("lint", "warning")]
        exp = [e for e in d["expects"] if not e["ok"]]
        ok = d["ok"] and not lints and not exp
        bad += not ok
        vol = d.get("part", {}).get("volume")
        print(f"  {'ok  ' if ok else 'FAIL'} {f.relative_to(HW)}  volume {vol}"
              + ("" if ok else f"  {d['errors']} {lints} {exp}"))
    print("\nstacks")
    rows = []
    for f in sorted(HERE.glob("*.fg")):
        if f.name.startswith("."):
            continue
        src = f.read_text()
        d = run(f)
        a = d.get("assembly") or {}
        shallow = kinds(d, "ShallowEngagement")
        ok = d["ok"] and a.get("interferences") == 0 and not shallow
        bad += not ok
        bare = re.sub(r"^\w+ = fit_rule\(.*\)\n", "", src, flags=re.M)
        bare = bare.replace("expect interferences = 0\n", "")
        nb = len(kinds(run(f, bare), "Interference"))
        threaded = sum(1 for r in a.get("fit_rules", []) if r["rule"] in PITCH for _ in range(r["pairs"]))
        probe = re.sub(r"^(shared min_\w+) = [0-9.]+", r"\1 = 99", src, flags=re.M)
        eng = {(w["a"], w["b"]): w["engaged"] for w in kinds(run(f, probe), "ShallowEngagement")}
        spec = src
        for rule, v in SPEC_MIN.items():
            spec = re.sub(rf"^shared min_{rule} = [0-9.]+", f"shared min_{rule} = {v}", spec, flags=re.M)
        sp = kinds(run(f, spec), "ShallowEngagement")
        poses = {i["id"]: i["pose"]["origin"][2] for i in a.get("instances", [])}
        files = dict(re.findall(r'^(\w+) = part\("(?:\.\./)?(?:plates/)?([\w]+)\.fg"', src, flags=re.M))
        rule = next((r for r in PITCH if f"min_{r}" in src), None)
        fast = next(i for i, fl in files.items() if fl in TIP)
        nut = next(i for i, fl in files.items() if fl in END)
        prot = poses[fast] + TIP[files[fast]] - (poses[nut] + END[files[nut]])
        e = next(iter(eng.values()), float("nan"))
        need = float(re.search(rf"^shared min_{rule} = ([0-9.]+)", src, flags=re.M).group(1))
        prot_ok = prot >= PITCH[rule] - 1e-9
        bad += not prot_ok or nb < threaded
        rows.append((f.stem, files[fast], files[nut], e, need, prot, PITCH[rule], ok, nb, threaded,
                     [round(w["engaged"], 3) for w in sp]))
        print(f"  {'ok  ' if ok else 'FAIL'} {f.name}: {a.get('interferences')} interferences, "
              f"{a.get('intended_contacts')} intended, rules {a.get('fit_rules')}; "
              f"without rules {nb} interferences; engaged {eng}; protrusion {prot:.2f}; "
              f"SPEC minima: {[(w['a'], w['b'], round(w['engaged'], 3), w['min']) for w in sp]}"
              + ("" if ok else f" {d['errors']}"))
    print("\n| stack | fastener | nut | engaged (Forge) | nut thread | past the nut end | 1 pitch | "
          "stack ok | without rules | SPEC 8 minimum |")
    print("|---|---|---|---|---|---|---|---|---|---|")
    for s, fa, nu, e, need, prot, p, ok, nb, th, sp in rows:
        spec = "short: " + ", ".join(map(str, sp)) if sp else "passes"
        print(f"| {s} | {fa} | {nu} | {e:.2f} | {need:.2f} | {prot:.2f} | {p} | {'yes' if ok else 'NO'} | "
              f"{nb} interference(s) | {spec} |")
    print(f"\n{'all passed' if not bad else f'{bad} FAILED'}")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
