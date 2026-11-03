#!/usr/bin/env python3
"""Builds every frame part: fg run --json (ok, expects, warnings), the DXF flat, a folded render, a flat
render from the DXF, and SPEC 8's scripted flat checks:
  flat bbox <= 762.0 x 914.4 (powder coat), bend lines <= 1117.6, every bend <= 130deg,
  box flanges with hardware <= 76.2, flat area x t = flat body volume (to 1e-6 relative).
Writes out/<part>.dxf, out/<part>.json, renders/<part>.png, renders/<part>-flat.png; prints a table.
usage: python3 frame/run.py [part.fg ..]   (every fg call through fbuild)"""
import json, math, re, subprocess, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
FG = str(Path.home() / "projects/ForgeCAD/.claude/worktrees/forge-lang/target/release/fg")
PARTS = {"post.fg": 3.02, "carrier.fg": 1.88, "rail.fg": 1.88, "base_pan.fg": 3.02, "top_pan.fg": 3.02}
BOX_FLANGE = {"base_pan.fg": 30.0, "top_pan.fg": 30.0}     # box parts: their flange height (outer)
(HERE / "out").mkdir(exist_ok=True)
(HERE / "renders").mkdir(exist_ok=True)


def fg(*args):
    p = subprocess.run(["fbuild", "nice", FG, *args], capture_output=True, text=True)
    return json.loads(p.stdout.strip().splitlines()[-1])


def dxf_loops(path):
    import ezdxf
    doc = ezdxf.readfile(path)
    circles, segs, bends = [], [], []
    for e in doc.modelspace():
        if e.dxf.layer == "BEND":
            bends.append(((e.dxf.start.x, e.dxf.start.y), (e.dxf.end.x, e.dxf.end.y)))
            continue
        t = e.dxftype()
        if t == "CIRCLE":
            circles.append((e.dxf.center.x, e.dxf.center.y, e.dxf.radius))
        elif t == "LINE":
            segs.append([(e.dxf.start.x, e.dxf.start.y), (e.dxf.end.x, e.dxf.end.y)])
        else:
            segs.append([(v.x, v.y) for v in e.flattening(0.0005)])
    tol, loops, used = 1e-4, [], [False] * len(segs)
    for i in range(len(segs)):
        if used[i]:
            continue
        used[i] = True
        chain, progress = list(segs[i]), True
        while progress and math.dist(chain[0], chain[-1]) > tol:
            progress = False
            for j in range(len(segs)):
                if used[j]:
                    continue
                s = segs[j]
                if math.dist(chain[-1], s[0]) < tol:
                    chain += s[1:]
                elif math.dist(chain[-1], s[-1]) < tol:
                    chain += s[::-1][1:]
                else:
                    continue
                used[j] = progress = True
                break
        loops.append((chain, math.dist(chain[0], chain[-1]) <= tol))
    return circles, loops, bends


def shoelace(pts):
    return 0.5 * sum(pts[i][0] * pts[i + 1][1] - pts[i + 1][0] * pts[i][1] for i in range(len(pts) - 1))


def flat_png(circles, loops, bends, png, title):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    fig, ax = plt.subplots(figsize=(12, 8), dpi=110)
    for c, _ in loops:
        ax.plot([p[0] for p in c], [p[1] for p in c], color="black", lw=0.6)
    for x, y, r in circles:
        ax.add_patch(plt.Circle((x, y), r, fill=False, color="black", lw=0.6))
    for a, b in bends:
        ax.plot([a[0], b[0]], [a[1], b[1]], color="red", lw=0.9, ls=(0, (6, 3)))
    ax.set_aspect("equal")
    ax.set_title(title, fontsize=9)
    ax.autoscale()
    fig.tight_layout()
    fig.savefig(png)
    plt.close(fig)


def one(name, t):
    f = HERE / name
    stem = f.stem
    dxf, png = HERE / "out" / f"{stem}.dxf", HERE / "renders" / f"{stem}.png"
    d = fg("run", str(f), "--json", "--dxf", str(dxf), "--render", str(png), "--view", "iso")
    (HERE / "out" / f"{stem}.json").write_text(json.dumps(d, indent=1))
    # the folded body alone: the part without its unfold and expects
    src = [l for l in f.read_text().splitlines() if not re.match(r"\s*(flat\s*=\s*unfold|expect)\b", l)]
    tmp = HERE / f".folded_{name}"
    tmp.write_text("\n".join(src) + "\n")
    folded = fg("run", str(tmp), "--json")
    tmp.unlink()
    vol = next(e["measured"] for e in d["expects"] if e["metric"] == "volume")
    fv = folded.get("part", {}).get("volume")
    circles, loops, bends = dxf_loops(str(dxf))
    areas = [abs(shoelace(c)) for c, closed in loops if closed]
    outer = max(areas)
    area = outer - (sum(areas) - outer) - sum(math.pi * r * r for _, _, r in circles)
    xs = [p[0] for c, _ in loops for p in c]
    ys = [p[1] for c, _ in loops for p in c]
    w, h = sorted([max(xs) - min(xs), max(ys) - min(ys)])
    bend_max = max(math.dist(a, b) for a, b in bends)
    flat_png(circles, loops, bends, HERE / "renders" / f"{stem}-flat.png", f"{stem}: flat {w:.2f} x {h:.2f}")
    angles = [b.get("angle") for s in d.get("sheets", []) for b in s.get("bends", [])]
    viol = [x for x in d["warnings"] if x.get("severity") == "violation"]
    lints = [x for x in d["warnings"] if x.get("severity") in ("lint", "warning")]
    row = {
        "part": stem, "ok": d["ok"], "ms": d["timings_ms"]["total"],
        "expects": all(e["ok"] for e in d["expects"]), "violations": len(viol), "lints": len(lints),
        "flat": f"{w:.2f} x {h:.2f}", "flat_ok": w <= 762.0 and h <= 914.4,
        "bend_max": round(bend_max, 2), "bend_ok": bend_max <= 1117.6,
        "angles_ok": all(a is None or a <= 130 for a in angles),
        "box_flange_ok": BOX_FLANGE.get(name, 0) <= 76.2,
        "flat_vol_rel": None if fv is None else abs(area * t - (vol - fv)) / (area * t),
        "bends": len(bends), "holes": len(circles) + len(areas) - 1,
    }
    for x in viol + lints:
        print(f"  {stem}: {x.get('kind')} {x.get('text', '')}")
    return row


def main():
    names = [a for a in sys.argv[1:] if a.endswith(".fg")] or list(PARTS)
    rows = [one(n, PARTS[n]) for n in names]
    keys = list(rows[0])
    print(" | ".join(keys))
    for r in rows:
        print(" | ".join(f"{r[k]:.1e}" if k == "flat_vol_rel" and r[k] is not None else str(r[k]) for k in keys))
    (HERE / "out" / "parts.json").write_text(json.dumps(rows, indent=1))


main()
