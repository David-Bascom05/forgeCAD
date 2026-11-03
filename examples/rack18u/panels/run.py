#!/usr/bin/env python3
"""Subsystem B part checks (SPEC 8, per-part DFM): build, expects, lints, typed violations, DXF,
folded and flat renders, and the shop limits Forge's shop does not hold (flat 762 x 914.4, bend
length 1117.6, bend angle 130deg), plus flat area x t against the flat body's volume.
Usage: run.py part.fg [..]   (no args: every part).  Results in out/<part>.json."""
import json, math, os, re, subprocess, sys, importlib.util

HERE = os.path.dirname(os.path.abspath(__file__))
FG = os.path.expanduser("~/projects/ForgeCAD/.claude/worktrees/forge-lang/target/release/fg")
SCRATCH = os.environ.get("SCRATCH", "/tmp/claude-1000/rack18u-panels")
os.makedirs(SCRATCH, exist_ok=True)
os.makedirs(os.path.join(HERE, "out"), exist_ok=True)
PARTS = ["side_panel.fg", "door.fg", "hinge_bracket.fg", "strike.fg", "cam.fg", "latch_handle.fg", "latch_base.fg"]
spec = importlib.util.spec_from_file_location("cp", os.path.expanduser(
    "~/projects/forgecad-gate/sheet-metal/complex-parts/run.py"))
cp = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cp)   # dxf_loops, flat_area, flat_png


def fg(*args):
    p = subprocess.run(["fbuild", FG, *args], capture_output=True, text=True)
    try:
        return json.loads(p.stdout.strip().splitlines()[-1])
    except Exception:
        return {"ok": False, "raw": p.stdout[-2000:] + p.stderr[-2000:]}


def strip(x):
    return {k: v for k, v in x.items() if k not in ("span", "fixes")}


def one(path):
    name = os.path.splitext(os.path.basename(path))[0]
    src = open(path).read()
    res = {"part": name}
    cj = fg("check", path, "--json")
    res["lints"] = [w["kind"] + ": " + w.get("text", "") for w in cj.get("warnings", []) if w.get("severity") == "lint"]
    dxf = os.path.join(HERE, "dxf", name + ".dxf")
    if os.path.exists(dxf):
        os.remove(dxf)
    j = fg("run", path, "--json", "--dxf", dxf)
    res["ok"] = j.get("ok")
    res["errors"] = [strip(e) for e in j.get("errors", [])]
    res["violations"] = [strip(w) for w in j.get("warnings", []) if w.get("severity") == "violation"]
    res["warnings"] = [strip(w) for w in j.get("warnings", []) if w.get("severity") not in ("lint", "violation")]
    res["expects"] = j.get("expects")
    sheets = j.get("sheets") or []
    res["bends"] = [{k: b.get(k) for k in ("flange", "angle", "radius", "allowance")}
                    for s in sheets for b in s.get("bends", [])]
    folded = "\n".join(l for l in src.splitlines() if not re.match(r"\s*(flat\s*=\s*unfold|expect)\b", l)) + "\n"
    fv = os.path.join(SCRATCH, name + "-folded.fg")
    open(fv, "w").write(folded)
    fj = fg("run", fv, "--json", "--render", os.path.join(HERE, "renders", name + ".png"), "--px", "900")
    lim = {}
    if os.path.exists(dxf):
        a, open_loops, nb, nent = cp.flat_area(dxf)
        circles, loops, bends = cp.dxf_loops(dxf)
        xs = [p[0] for c, _ in loops for p in c] + [x + s * r for x, _, r in circles for s in (-1, 1)]
        ys = [p[1] for c, _ in loops for p in c] + [y + s * r for _, y, r in circles for s in (-1, 1)]
        bw, bh = max(xs) - min(xs), max(ys) - min(ys)
        lo, hi = sorted((bw, bh))
        lim["flat_bbox"] = [round(lo, 3), round(hi, 3)]
        lim["flat_fits_762x914.4"] = lo <= 762.0 and hi <= 914.4
        bl = [math.dist(p, q) for p, q in bends]
        lim["max_bend_line"] = round(max(bl), 3) if bl else 0
        lim["bend_lines_ok"] = all(x <= 1117.6 for x in bl)
        lim["bend_angles_ok"] = all((b.get("angle") or 0) <= 130 for b in res["bends"])
        lim["open_loops"] = open_loops
        if j.get("part") and fj.get("part") and a:
            vflat = j["part"]["volume"] - fj["part"]["volume"]
            t = sheets[0]["thickness"]
            lim["flat_area_x_t"] = a * t
            lim["flat_body_volume"] = vflat
            lim["rel_err"] = (a * t - vflat) / vflat
        cp.flat_png(dxf, os.path.join(HERE, "renders", name + "-flat.png"), name + " flat (CUT black, BEND red dashed)")
    res["shop_limits"] = lim
    if j.get("part"):
        res["volume"] = j["part"].get("volume")
    json.dump(res, open(os.path.join(HERE, "out", name + ".json"), "w"), indent=1, default=str)
    bad = [e["kind"] for e in res["errors"]] + [e["metric"] for e in (res["expects"] or []) if not e.get("ok")]
    print(f"{name:16} ok={res['ok']} errors/expects-failed={bad or '-'} lints={len(res['lints'])} "
          f"violations={[v['kind'] for v in res['violations']] or '-'} limits={lim}")
    for l in res["lints"]:
        print("   lint", l)
    for v in res["violations"] + res["warnings"]:
        print("   ", v.get("kind"), v.get("text", "")[:200])


if __name__ == "__main__":
    for f in sys.argv[1:] or [os.path.join(HERE, p) for p in PARTS]:
        one(os.path.abspath(f))
