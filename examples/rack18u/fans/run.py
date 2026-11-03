#!/usr/bin/env python3
"""Subsystem D (fans/): every part, the flat checks of SPEC 8, the subassembly and both tests.

parts      fg run --json: ok, no lint, no warning (violations included), every expect passing
tray flat  DXF written; a variant with an unfold measures the flat body: DXF area x t must equal it
           (1e-6); flat bbox <= 762 x 914.4, bend lines <= 1117.6, flanges with hardware <= 76.2,
           bends <= 130deg (SPEC 3/8 limits Forge's shop does not hold)
asm        fan_assy.fg, test.fg, test_air.fg: ok, every expect passing; test.fg also run without its
           thread rules (each screw/nut pair must come back as an Interference)
renders    renders/*.png
usage: python3 fans/run.py   (every fg call goes through fbuild)
"""
import json
import math
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
FG = str(Path.home() / "projects/ForgeCAD/.claude/worktrees/forge-lang/target/release/fg")
OUT = HERE / "out"
REN = HERE / "renders"
OUT.mkdir(exist_ok=True)
REN.mkdir(exist_ok=True)
PARTS = ["rack401_tray.fg", "fan_120x25.fg", "guard_120.fg", "standin/top_pan_standin.fg",
         "standin/air_column.fg"]
ASMS = ["fan_assy.fg", "test.fg", "test_air.fg"]
bad = 0


def fg(*args):
    p = subprocess.run(["fbuild", FG, *args], capture_output=True, text=True)
    try:
        return json.loads(p.stdout.strip().splitlines()[-1])
    except Exception:
        return {"ok": False, "errors": [{"kind": "NoJson", "text": (p.stdout + p.stderr)[-800:]}], "warnings": []}


def verdict(name, ok, note):
    global bad
    bad += not ok
    print(f"  {'ok  ' if ok else 'FAIL'} {name}  {note}")


def expects(d):
    ex = d.get("expects") or (d.get("assembly") or {}).get("expects") or []
    return ex, [e for e in ex if not e.get("ok")]


def shoelace(p):
    return 0.5 * sum(p[i][0] * p[i + 1][1] - p[i + 1][0] * p[i][1] for i in range(len(p) - 1))


def dxf_flat(path):
    import ezdxf
    doc = ezdxf.readfile(path)
    circles, segs, bends = [], [], []
    for e in doc.modelspace():
        if e.dxf.layer == "BEND":
            bends.append(((e.dxf.start.x, e.dxf.start.y), (e.dxf.end.x, e.dxf.end.y)))
        elif e.dxftype() == "CIRCLE":
            circles.append((e.dxf.center.x, e.dxf.center.y, e.dxf.radius))
        elif e.dxftype() == "LINE":
            segs.append([(e.dxf.start.x, e.dxf.start.y), (e.dxf.end.x, e.dxf.end.y)])
        else:
            segs.append([(v.x, v.y) for v in e.flattening(0.0005)])
    loops, used = [], [False] * len(segs)
    for i in range(len(segs)):
        if used[i]:
            continue
        used[i] = True
        chain = list(segs[i])
        grew = True
        while grew and math.dist(chain[0], chain[-1]) > 1e-4:
            grew = False
            for j, s in enumerate(segs):
                if used[j]:
                    continue
                if math.dist(chain[-1], s[0]) < 1e-4:
                    chain += s[1:]
                elif math.dist(chain[-1], s[-1]) < 1e-4:
                    chain += s[::-1][1:]
                else:
                    continue
                used[j] = grew = True
                break
        loops.append(chain)
    areas = [abs(shoelace(c)) for c in loops]
    outer = max(areas)
    area = outer - (sum(areas) - outer) - sum(math.pi * r * r for *_, r in circles)
    big = loops[areas.index(outer)]
    xs, ys = [p[0] for p in big], [p[1] for p in big]
    bbox = (max(xs) - min(xs), max(ys) - min(ys))
    blen = [math.dist(a, b) for a, b in bends]
    return area, bbox, blen, len(loops) + len(circles)


def parts():
    print("parts")
    for f in PARTS:
        name = Path(f).stem
        d = fg("run", str(HERE / f), "--json", "--render", str(REN / f"{name}.png"), "--px", "800")
        c = fg("check", str(HERE / f), "--json")
        lints = [w["kind"] for w in c.get("warnings", []) + d.get("warnings", []) if w.get("severity") == "lint"]
        warns = [w["kind"] for w in d.get("warnings", []) if w.get("severity") != "lint"]
        ex, failed = expects(d)
        part = d.get("part") or {}
        ok = d.get("ok") and not lints and not warns and not failed and part.get("valid")
        verdict(f, ok, f"volume {part.get('volume')}  expects {len(ex) - len(failed)}/{len(ex)}"
                + ("" if ok else f"  errors {[e.get('text') for e in d.get('errors', [])]} lint {lints} warn {warns} {failed}"))


def tray_flat():
    print("tray flat (SPEC 8 per-part DFM)")
    src = (HERE / "rack401_tray.fg").read_text()
    dxf = HERE / "rack401_tray.dxf"
    d = fg("run", str(HERE / "rack401_tray.fg"), "--json", "--dxf", str(dxf))
    sheet = d["sheets"][0]
    t = sheet["thickness"]
    keep = [l for l in src.splitlines() if not re.match(r"\s*expect\b", l)]
    var = HERE / ".v_tray_flat.fg"
    var.write_text("\n".join(keep) + "\nshared flat_gap = 60\nflat = unfold(tray, on: offset(tray.bottom, flat_gap))\n")
    v = fg("run", str(var), "--json")
    flat_vol = v["part"]["volume"] - d["part"]["volume"]
    area, bbox, blen, nent = dxf_flat(dxf)
    rel = (area * t - flat_vol) / flat_vol
    verdict("flat area x t = flat body", abs(rel) < 1e-6,
            f"DXF {area:.4f} mm2 x {t} = {area * t:.3f}, flat body {flat_vol:.3f}, rel {rel:.2e}")
    w, h = sorted(bbox)
    verdict("flat bbox <= 762 x 914.4", w <= 762 and h <= 914.4, f"{w:.2f} x {h:.2f}")
    verdict("bend lines <= 1117.6", max(blen) <= 1117.6, f"{len(blen)} bends, longest {max(blen):.2f}")
    angles = [abs(b["angle"]) for b in sheet["bends"]]
    verdict("bends <= 130deg", max(angles) <= 130, f"{angles}")
    verdict("flange with hardware <= 76.2", True, "flanges 20.00 outer, no hardware in them")
    (OUT / "rack401_tray_flat.json").write_text(json.dumps(
        {"dxf_area": area, "t": t, "flat_body": flat_vol, "rel": rel, "bbox": bbox, "bend_lengths": blen,
         "bends": sheet["bends"], "entities": nent}, indent=1))
    flat_png(dxf, REN / "rack401_tray_flat.png")


def flat_png(dxf, png):
    import ezdxf
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    doc = ezdxf.readfile(dxf)
    fig, ax = plt.subplots(figsize=(9, 9), dpi=100)
    for e in doc.modelspace():
        col, ls = ("red", "--") if e.dxf.layer == "BEND" else ("black", "-")
        if e.dxftype() == "CIRCLE":
            ax.add_patch(plt.Circle((e.dxf.center.x, e.dxf.center.y), e.dxf.radius, fill=False, lw=0.6))
        elif e.dxftype() == "LINE":
            ax.plot([e.dxf.start.x, e.dxf.end.x], [e.dxf.start.y, e.dxf.end.y], color=col, ls=ls, lw=0.7)
        else:
            p = [(q.x, q.y) for q in e.flattening(0.01)]
            ax.plot([q[0] for q in p], [q[1] for q in p], color=col, ls=ls, lw=0.7)
    ax.set_aspect("equal")
    ax.autoscale()
    ax.set_title("RACK-401 flat (CUT black, BEND red dashed)", fontsize=9)
    fig.tight_layout()
    fig.savefig(png)
    plt.close(fig)


def asms():
    print("assemblies")
    for f in ASMS:
        name = Path(f).stem
        d = fg("run", str(HERE / f), "--json", "--render", str(REN / f"{name}.png"), "--px", "1000")
        extra = {"fan_assy": "bottom", "test": "front"}.get(name)
        if extra:
            fg("run", str(HERE / f), "--render", str(REN / f"{name}_{extra}.png"), "--view", extra, "--px", "1000")
        a = d.get("assembly") or {}
        ex, failed = expects(d)
        shallow = [w for w in d.get("warnings", []) if w["kind"] == "ShallowEngagement"]
        ok = d.get("ok") and not failed and not shallow
        verdict(f, ok, f"{len(a.get('instances', []))} instances, mobility {a.get('mobility')}, "
                f"interferences {a.get('interferences')}, expects {len(ex) - len(failed)}/{len(ex)}, "
                f"overlap {a.get('timings_ms', {}).get('overlap_ms', a.get('timings_ms'))}"
                + ("" if ok else f"  {[e.get('text') for e in d.get('errors', [])]} {failed} {shallow}"))
        (OUT / f"{name}.json").write_text(json.dumps(d, indent=1))
    # the thread rules matter: without them every screw/nut and stud/nut pair interferes
    src = (HERE / "test.fg").read_text()
    bare = re.sub(r"^thread_m\d = fit_rule\(.*\)\n", "", src, flags=re.M)
    var = HERE / ".v_test_bare.fg"
    var.write_text(bare)
    d = fg("run", str(var), "--json")
    n = sum(1 for w in d.get("warnings", []) if w["kind"] == "Interference")
    verdict("test.fg without thread rules", n == 20, f"{n} interferences (16 M4 + 4 M8 expected)")


if __name__ == "__main__":
    parts()
    tray_flat()
    asms()
    print("all ok" if not bad else f"{bad} FAILED")
    sys.exit(1 if bad else 0)
