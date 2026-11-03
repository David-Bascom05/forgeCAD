#!/usr/bin/env python3
"""Runs every caster part and assembly through fg, writes the renders and prints a summary.

usage: systemd-run --user --scope -p MemoryMax=8G -p MemorySwapMax=0 nice python3 casters/run.py [--no-render] [--only NAME]
Each file must come back ok: no errors, no lints or warnings other than IntendedContact, every expect met.
"""
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
FG = str(Path.home() / "projects/ForgeCAD/.claude/worktrees/forge-lang/target/release/fg")
PARTS = ["plate", "kingpin", "lower_race", "yoke", "ball_4763", "pedal", "pedal_pin", "lock_pin", "spacer", "hub",
         "tread", "bearing_inner", "bearing_outer", "ball_3969", "cage_half", "seal",
         "pan_standin", "floor_ref", "envelope_ref"]
ASMS = ["bearing_608", "caster_plain", "caster_lock", "test"]
VIEWS = {"caster_lock": ["iso", "right", "front"], "caster_plain": ["iso"], "bearing_608": ["iso", "front"],
         "test": ["iso"]}


def run(name, render):
    args = [FG, "run", str(HERE / f"{name}.fg"), "--json"]
    out = HERE / "renders"
    out.mkdir(exist_ok=True)
    if render:
        args += ["--render", str(out / f"{name}.png"), "--view", (VIEWS.get(name) or ["iso"])[0], "--px", "1200"]
    r = subprocess.run(args, capture_output=True, text=True, cwd=HERE)
    d = json.loads(r.stdout)
    if render:
        for v in (VIEWS.get(name) or ["iso"])[1:]:
            subprocess.run([FG, "run", str(HERE / f"{name}.fg"), "--render", str(out / f"{name}-{v}.png"),
                            "--view", v, "--px", "1200"], capture_output=True, cwd=HERE)
    return d


def main():
    render = "--no-render" not in sys.argv
    only = sys.argv[sys.argv.index("--only") + 1] if "--only" in sys.argv else None
    bad = 0
    for name in PARTS + ASMS:
        if only and name != only:
            continue
        d = run(name, render)
        warn = [w for w in d["warnings"] if w["kind"] != "IntendedContact"]
        a = d.get("assembly")
        exps = a["expects"] if a else d["expects"]
        failed = [e for e in exps if not e["ok"]]
        ok = d["ok"] and not warn and not failed
        bad += not ok
        if a:
            info = (f"{len(a['instances'])} instances, mobility {a['mobility']}, {a['interferences']} interferences, "
                    f"{a['intended_contacts']} intended contacts, {len(a.get('engagements', []))} threads engaged "
                    f"(short {sum(e['short'] for e in a.get('engagements', []))}), {sum(e['ok'] for e in exps)}/{len(exps)} expects")
        else:
            info = f"volume {d['part']['volume']}, {sum(e['ok'] for e in exps)}/{len(exps)} expects"
        print(f"{'ok  ' if ok else 'FAIL'} {name:14s} {info}")
        for e in d["errors"][:5]:
            print("     error", e["kind"], e.get("text"))
        for w in warn[:5]:
            print("     warning", w["kind"], w.get("text"))
        for e in failed[:10]:
            print("     expect", e)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
