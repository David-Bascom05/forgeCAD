#!/usr/bin/env python3
"""The order package, from the final assembly (out/rack.json, written by run.py) and the part files.

order/<RACK-nnn>_<name>.dxf   flat pattern per fabricated part (fg run --dxf; bend lines on layer BEND)
order/<RACK-nnn>_<name>.step  the folded part (forgecad export ap214, from a copy with the flat and
                              the expects dropped), read back by OpenCascade (step_check.py): one valid
                              solid whose volume matches Forge's folded volume
order/<RACK-nnn>_<name>.json  the part's run (sheets: thickness, radius, K, bends)
order/parts.md                per part: qty (from the assembly BOM, checked against SPEC 4), material,
                              flat bbox, bend table (flange, angle, inside radius, length, bend line in the
                              DXF), PEM list for SendCutSend's hardware step
order/purchased.md            purchased BOM from the assembly, checked against SPEC 4, 5.12 and 7
SendCutSend takes STEP for bent parts; the DXF plus bend table is the second route (SPEC 10 #9, #10).
Run: python3 order.py   (each fg call capped like run.py)"""
import collections
import json
import math
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
OUT = os.path.join(HERE, "order")
BIN = os.path.expanduser("~/projects/ForgeCAD/.claude/worktrees/forge-lang/target/release")
FG = os.path.join(BIN, "fg")
FORGECAD = os.path.join(BIN, "forgecad")
OCP_PY = os.path.expanduser("~/.local/share/forgecad-occt/venv/bin/python")
CAP = ["systemd-run", "--user", "--scope", "-q", "-p", "MemoryMax=6G", "-p", "MemorySwapMax=0", "nice"]

# fabricated parts (SPEC 4): number, file, SPEC qty
FAB = [
    ("RACK-101", "frame/post.fg", "post", 4),
    ("RACK-111", "frame/carrier.fg", "rail carrier", 4),
    ("RACK-121", "frame/base_pan.fg", "base pan", 1),
    ("RACK-131", "frame/top_pan.fg", "top pan", 1),
    ("RACK-201", "frame/rail.fg", "EIA rail", 4),
    ("RACK-141", "panels/side_panel.fg", "side half-panel", 4),
    ("RACK-301", "panels/door.fg", "door", 2),
    ("RACK-311", "panels/hinge_bracket.fg", "hinge bracket", 4),
    ("RACK-312", "panels/strike.fg", "strike", 2),
    ("RACK-313", "panels/cam.fg", "latch cam", 2),
    ("RACK-321", "panels/latch_handle.fg", "latch handle", 2),
    ("RACK-322", "panels/latch_base.fg", "latch base", 2),
    ("RACK-401", "fans/rack401_tray.fg", "fan tray", 1),
]
SPEC_PEM_TOTAL = 270          # SPEC 4, R12 (no S-M6-0 in the latch bases)
# pem(kind, size) -> SendCutSend / PennEngineering part (SPEC 3, 7)
PEM_PN = {("stud", "M4"): "FH-M4-12", ("stud", "M6"): "FH-M6-20", ("stud", "M8"): "FH-M8-25",
          ("nut", "M4"): "S-M4-0", ("nut", "M5"): "SS-M5-0", ("nut", "M6"): "S-M6-0"}
# hw part files that are PEM bodies (pressed by SCS, not bought by us)
PEM_FILES = re.compile(r"hw/pem_")
# purchased parts, SPEC 5.12 / 7 counts for the whole rack; per-U items for the three dummies (7 U)
SPEC_BUY = {
    "iso7380_m5x10.fg": 24, "iso7380_m5x12.fg": 6, "iso7380_m6x10.fg": 16, "iso7380_m6x12.fg": 24,
    "iso4017_m8x20.fg": 16, "iso10511_m5.fg": 6, "iso10511_m6.fg": 18, "iso10511_m8.fg": 20,
    "iso7089_5.fg": 6, "iso7089_6.fg": 16, "iso7089_8.fg": 24, "iso7379_8x12_m6.fg": 4,
    "iso7379_8x16_m6.fg": 2, "iso7380_m4x25.fg": 4, "iso7089_4.fg": 6, "iso10511_m4.fg": 6,
    "nylon_spacer_8x14x8.fg": 8, "latch_spacer_6x15.fg": 4, "ball_4.fg": 4, "iso7045_m4x40.fg": 16,
    "fan_120x25.fg": 4, "guard_120.fg": 4, "cage_nut_m6.fg": 12, "rack_screw_m6x16.fg": 12,
    "plastic_washer_m6.fg": 12,
}
SPRINGS = re.compile(r"latch_spring_")


def fg(args):
    p = subprocess.run(CAP + [FG] + args, capture_output=True, text=True)
    return json.loads(p.stdout)


def folded_step(part_file, base):
    """STEP of the folded body alone: a copy without the unfold, its params and the expects."""
    os.makedirs(os.path.join(OUT, "build"), exist_ok=True)
    src = open(os.path.join(ROOT, part_file)).read().splitlines()
    keep = [l for l in src if not (l.startswith("param flat.") or re.match(r"flat = unfold\(", l)
                                   or l.startswith("expect "))]
    fold = os.path.join(OUT, "build", base + ".fg")
    open(fold, "w").write("\n".join(keep) + "\n")
    run = fg(["run", fold, "--json"])
    step = os.path.join(OUT, base + ".step")
    p = subprocess.run(CAP + [FORGECAD, "export", fold, "ap214", "-o", step], capture_output=True, text=True)
    back = subprocess.run([OCP_PY, "-I", os.path.join(HERE, "step_check.py"), step], capture_output=True, text=True)
    info = json.loads(back.stdout.strip().splitlines()[-1]) if back.returncode == 0 else {"read": False}
    info["fg_volume"] = run.get("part", {}).get("volume")
    info["export"] = (p.stdout + p.stderr).strip().splitlines()[-1:] 
    return info


def dxf_info(path):
    import ezdxf
    doc = ezdxf.readfile(path)
    xs, ys, bends = [], [], []
    for e in doc.modelspace():
        if e.dxf.layer == "BEND":
            a, b = e.dxf.start, e.dxf.end
            bends.append(((a.x, a.y), (b.x, b.y), math.dist((a.x, a.y), (b.x, b.y))))
            continue
        try:
            pts = [(v.x, v.y) for v in e.flattening(0.01)] if e.dxftype() != "LINE" else \
                [(e.dxf.start.x, e.dxf.start.y), (e.dxf.end.x, e.dxf.end.y)]
        except Exception:
            continue
        xs += [p[0] for p in pts]
        ys += [p[1] for p in pts]
    return (min(xs), min(ys), max(xs), max(ys)), bends


def pems(part_file, run):
    src = open(os.path.join(ROOT, part_file)).read().splitlines()
    out = collections.Counter()
    for f in run["features"]:
        if f["kind"] != "pem":
            continue
        m = re.search(r'pem\("(\w+)",\s*"(M\d+)".*wall:\s*(\w+)', src[f["line"] - 1])
        out[(PEM_PN[(m.group(1), m.group(2))], m.group(3))] += 1
    return out


def main():
    os.makedirs(OUT, exist_ok=True)
    rack = json.load(open(os.path.join(HERE, "out", "rack.json")))
    asm = rack["assembly"]
    open_items = ["rack.fg line %d: %s measured %s, limit %s" % (e["line"], e["metric"], e["measured"], e["limit"])
                  for e in asm["expects"] if not e["ok"]]
    open_items += [w["text"] for w in rack["warnings"] if w["kind"] == "Interference"]
    bom = {os.path.relpath(b["file"], ROOT): b["qty"] for b in asm["bom"]}
    lines = ["# RACK-18U order package: fabricated parts", "",
             "From asm/rack.fg's run (out/rack.json). SendCutSend, cold rolled 1008 mild steel, black textured",
             "powder coat (SPEC 12). Each part: STEP of the folded part (SendCutSend's route for bent parts,",
             "read back by OpenCascade), DXF of the flat, and the bend table for entering the bends by hand",
             "if the STEP is refused. Bend angle sign is Forge's (negative: the flange turns to the DXF's back",
             "face). Open items in the assembly at the time of this package are listed at the end.", ""]
    problems = ["open in the assembly: " + x for x in open_items]
    pem_total = collections.Counter()
    for num, f, name, qty in FAB:
        base = "%s_%s" % (num, os.path.splitext(os.path.basename(f))[0])
        dxf = os.path.join(OUT, base + ".dxf")
        run = fg(["run", os.path.join(ROOT, f), "--json", "--dxf", dxf])
        json.dump(run, open(os.path.join(OUT, base + ".json"), "w"), indent=1)
        if not run["ok"] or run["errors"]:
            problems.append("%s: run not ok %s" % (num, run["errors"][:2]))
        viol = [w for w in run["warnings"] if w.get("severity") != "lint"]
        n_asm = bom.get(f, 0)
        if n_asm != qty:
            problems.append("%s: assembly has %d, SPEC 4 says %d" % (num, n_asm, qty))
        sheet = run["sheets"][0] if run.get("sheets") else None
        (x0, y0, x1, y1), blines = dxf_info(dxf)
        w, h = x1 - x0, y1 - y0
        if max(w, h) > 914.4 or min(w, h) > 762.0:
            problems.append("%s: flat %.1f x %.1f exceeds 762 x 914.4" % (num, w, h))
        st = folded_step(f, base)
        ok_step = (st.get("read") and st.get("solids") == 1 and st.get("valid") and st.get("fg_volume")
                   and abs(st["volume"] - st["fg_volume"]) <= 1e-4 * st["fg_volume"])
        if not ok_step:
            problems.append("%s: STEP read-back %s" % (num, st))
        lines += ["## %s %s, qty %d" % (num, name, qty), "",
                  "- file: `%s`, STEP `order/%s.step`, DXF `order/%s.dxf`" % (f, base, base),
                  "- STEP read back by OpenCascade: %s solid, valid %s, volume %.1f (Forge %.1f)"
                  % (st.get("solids"), st.get("valid"), st.get("volume", 0), st.get("fg_volume") or 0),
                  "- thickness %.2f, flat bbox %.2f x %.2f" % (sheet["thickness"] if sheet else 0, w, h)]
        lines.append("- shop and DFM warnings: %d" % len(viol))
        if viol:
            lines.append("  " + ", ".join(sorted({v["kind"] for v in viol})))
            problems.append("%s: %d warnings %s" % (num, len(viol), sorted({v["kind"] for v in viol})))
        bends = sheet["bends"] if sheet else []
        lines.append("- bends: %d" % len(bends))
        if bends:
            lines += ["", "```", "flange              angle   inside r  length    bend line in the DXF"]
            used = set()
            for b in bends:
                match = ""
                for i, (a, c, L) in enumerate(blines):
                    if i not in used and abs(L - b["length"]) < 0.05:
                        used.add(i)
                        match = "(%.2f, %.2f) - (%.2f, %.2f)" % (a[0], a[1], c[0], c[1])
                        break
                lines.append("%-18s %7.1f %9.2f %8.2f    %s" % (b["flange"], b["angle"], b["radius"], b["length"], match))
                if b["length"] > 1117.6 or abs(b["angle"]) > 130:
                    problems.append("%s: bend %s beyond SPEC 3 limits" % (num, b["flange"]))
            lines.append("```")
            if len(blines) != len(bends):
                problems.append("%s: %d bend lines in the DXF, %d bends" % (num, len(blines), len(bends)))
        pl = pems(f, run)
        if pl:
            lines += ["", "PEM hardware, per part (pressed by SendCutSend):", "```"]
            for (pn, wall), n in sorted(pl.items()):
                lines.append("%-10s x%-3d in %s" % (pn, n, wall))
                pem_total[pn] += n * qty
            lines.append("```")
        lines.append("")
    total = sum(pem_total.values())
    lines += ["## PEM totals (whole rack)", "", "```"]
    lines += ["%-10s %d" % (pn, n) for pn, n in sorted(pem_total.items())]
    lines += ["total      %d   (SPEC 4: %d)" % (total, SPEC_PEM_TOTAL), "```", ""]
    lines += ["## Open items in the assembly", ""] + ["- " + x for x in open_items or ["none"]] + [""]
    if total != SPEC_PEM_TOTAL:
        problems.append("PEM total %d, SPEC 4 says %d" % (total, SPEC_PEM_TOTAL))
    asm_pem = sum(n for f, n in bom.items() if PEM_FILES.search(f))
    if asm_pem != total:
        problems.append("PEM bodies in the assembly %d, part files %d" % (asm_pem, total))
    open(os.path.join(OUT, "parts.md"), "w").write("\n".join(lines))

    # purchased
    fab_files = {f for _, f, _, _ in FAB}
    buy = collections.Counter()
    casters = collections.Counter()
    other = collections.Counter()
    for f, n in bom.items():
        if f in fab_files or PEM_FILES.search(f):
            continue
        if f.startswith("casters/"):
            casters[f] += n
        elif f.startswith("hw/") or f.startswith("fans/"):
            buy[os.path.basename(f)] += n
        else:
            other[f] += n
    # the hardware inside each caster comes with the caster (Blickle), not from us
    for name, n_corner in (("caster_lock.fg", 2), ("caster_plain.fg", 2)):
        src = open(os.path.join(ROOT, "casters", name)).read()
        assert "for" not in "".join(l for l in src.splitlines() if "../hw/" in l)
        for hw in re.findall(r'^\w+ = part\("\.\./hw/([\w.]+)"', src, re.M):
            buy[hw] -= n_corner
            casters["hw/" + hw + " (inside the caster)"] += n_corner
    plines = ["# RACK-18U purchased BOM", "", "From the assembly's BOM; checked against SPEC 5.12 and 7. Order one spare",
              "of each small part (SPEC 4).", "", "```", "%-28s %5s %5s" % ("part", "asm", "SPEC")]
    for f in sorted(f for f in set(buy) | set(SPEC_BUY) if buy.get(f, 0) or f in SPEC_BUY):
        if SPRINGS.search(f):
            continue
        s = SPEC_BUY.get(f)
        plines.append("%-28s %5d %5s%s" % (f, buy.get(f, 0), s if s is not None else "-",
                                           "" if s == buy.get(f, 0) else "   MISMATCH"))
        if s != buy.get(f, 0):
            problems.append("purchased %s: assembly %d, SPEC %s" % (f, buy.get(f, 0), s))
    springs = sum(n for f, n in buy.items() if SPRINGS.search(f))
    plines.append("%-28s %5d %5s%s" % ("latch spring (any height)", springs, 2, "" if springs == 2 else "   MISMATCH"))
    plines += ["```", "", "Casters (Blickle, SPEC 7): 2 x LKPA-PATH 80KF-FI (ID 755344, front), 2 x LKPA-PATH 80KF",
               "(ID 755346, rear); their modelled internals are not ordered:", "```"]
    plines += ["%-40s %d" % (f, n) for f, n in sorted(casters.items())]
    plines += ["```", ""]
    if other:
        plines += ["Not classified:", "```"] + ["%-40s %d" % (f, n) for f, n in sorted(other.items())] + ["```"]
        problems += ["unclassified BOM line %s" % f for f in other]
    open(os.path.join(OUT, "purchased.md"), "w").write("\n".join(plines))
    print("\n".join(problems) if problems else "order package: no problems")


if __name__ == "__main__":
    main()
