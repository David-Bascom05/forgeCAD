#!/usr/bin/env python3
"""Haas's solid models into forgecad's machine library, reproducibly.

    python3 tools/machines/import_haas.py ZIP_DIR [MACHINE ..] [--forgecad BIN] [--cell MM]

For each machine in MACHINES (or those named): its zip from ZIP_DIR (Haas's download, as on the
branch haas-3d-models: research/haas-models/*.zip) unpacked to a temporary folder, its STEP
attached to the record and split onto the bodies by the parts' names
(`forgecad machine model attach` / `split ... PART=BODY`), placed in the record's frame, and
shipped (`forgecad machine model ship`, decimated) into
kernel/forge-machine/data/models/haas/<machine>. All of it in a temporary config folder
(XDG_CONFIG_HOME): a user's own library is never touched, and nothing of the zip is kept.

Placements (each machine's `place`) put the record's frame origin (the table or platter top
centre at home) where Haas's model has it, measured on the model: the model's own frame is Y up,
its origin at the Y and Z ball screws (Haas's tutorial). A model not at home (the UMC-750's
head stands 319 mm under its Z home) has its moving bodies moved to the record's home.

`cut`: a part of a body's mesh that belongs to another body (the UMC-750's trunnion is one solid:
its B drive housing, beyond y = 370 mm in the record's frame, does not tilt) moved to that body,
triangle by triangle on their centres in the record's frame.
"""
import math
import os
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = os.path.join(ROOT, "kernel/forge-machine/data/models/haas")

MACHINES = {
    "haas-vf-2": dict(
        zip="vf-2_solid_models_1_2023.zip",
        step=r"VF-2_STEP__1_2023\.step$",
        source="Haas Automation solid model VF-2_STEP__1_2023.step (haascnc.com download)",
        rules=["TABLE_=table", "SADDLE_=saddle", "COLUMN_=column", "BASE_=base", "PADS_=base",
               "SPINDLE HEAD_=head", "P-COOL=head", "20-PKT UTC=enclosure", "SMTC=-", "ENCLOSURE=enclosure",
               "DOORS=enclosure", "EES_=enclosure", "PENDANT=enclosure", "CHIP CHUTE=enclosure",
               "CONV PKG=enclosure"],
        # the table top centre at home: x -381, y (up) 347.72, z 692.247 in the model
        rotate=[90, 0, 0],
        place={"*": [381, 692.247, -347.72]},
    ),
    "haas-umc-750": dict(
        zip="rpm_modular_umc_750_solid_models_09_2026.zip",
        step=r"umc-750.*\.STEP$",
        source="Haas Automation solid model rpl_modular_umc-750_09-2026 1.STEP (haascnc.com download)",
        # a robot cell around the machine: the machine kept, its options' alternatives dropped
        rules=["Trunion=cradle", "T Slot Platter=platter", "Base Casting=base", "UMC-750- Leveling=base",
               "X Axis RPL=column", "Z Axis RPL=head", "40T Spindle=head", "UMC-750- Enclosure=enclosure",
               "Auto Window RPL config=enclosure", "Control Panel=enclosure", "WayCover=enclosure",
               "Wips=enclosure", "50 PKT TC=enclosure"],
        # the platter top centre: x 938.05, y (up) 840.105, z 645.7 in the model; x mirrored (Haas's
        # +x is the record's -x for this model's view); the head to the published 610 mm and the
        # record's spindle x, y; the X carriage with it in x
        rotate=[90, 0, 180],
        place={"*": [938.05, -645.7, -840.105], "head": [989.2, -677.3, -520.705], "column": [989.2, -645.7, -840.105]},
        cut=[("cradle", "support-r", 1, 370.0)],
    ),
}


def run(cmd, env):
    print("+", " ".join(cmd), flush=True)
    r = subprocess.run(cmd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    print(r.stdout, end="")
    if r.returncode:
        sys.exit(f"failed: {' '.join(cmd)}")
    return r.stdout


def rotation(deg):
    def r(axis, a):
        c, s = math.cos(math.radians(a)), math.sin(math.radians(a))
        return [[1, 0, 0], [0, c, -s], [0, s, c]] if axis == 0 else (
            [[c, 0, s], [0, 1, 0], [-s, 0, c]] if axis == 1 else [[c, -s, 0], [s, c, 0], [0, 0, 1]])

    def mm(a, b):
        return [[sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]

    return mm(r(2, deg[2]), mm(r(1, deg[1]), r(0, deg[0])))


def read_stl(p):
    d = open(p, "rb").read()
    n = struct.unpack("<I", d[80:84])[0]
    return d[:80], [d[84 + 50 * i: 84 + 50 * (i + 1)] for i in range(n)]


def write_stl(p, head, tris):
    with open(p, "wb") as f:
        f.write(head + struct.pack("<I", len(tris)) + b"".join(tris))


def models(dir_):
    t = open(os.path.join(dir_, "models.toml")).read()
    out = {}
    for blk in t.split("[[model]]")[1:]:
        out[re.search(r'body = "(.*)"', blk).group(1)] = dict(
            file=re.search(r'file = "(.*)"', blk).group(1),
            rotate=[float(x) for x in re.search(r"rotate = \[(.*)\]", blk).group(1).split(",")],
            offset=[float(x) for x in re.search(r"offset = \[(.*)\]", blk).group(1).split(",")])
    return t, out


def cut(dir_, frm, to, axis, at):
    """Triangles of body `frm`'s mesh whose centre lies at or past `at` on record axis `axis`
    moved to body `to` (a model of its own, with the same placement)."""
    text, ms = models(dir_)
    m = ms[frm]
    rot = rotation(m["rotate"])
    head, tris = read_stl(os.path.join(dir_, m["file"]))
    keep, move = [], []
    for t in tris:
        v = struct.unpack("<12f", t[:48])
        c = [(v[3 + k] + v[6 + k] + v[9 + k]) / 3 for k in range(3)]
        w = sum(rot[axis][k] * c[k] for k in range(3)) + m["offset"][axis]
        (move if w >= at else keep).append(t)
    write_stl(os.path.join(dir_, m["file"]), head, keep)
    write_stl(os.path.join(dir_, f"{to}.stl"), head, move)
    blk = text.split("[[model]]")
    mine = next(b for b in blk if f'body = "{frm}"' in b)
    new = mine.replace(f'body = "{frm}"', f'body = "{to}"').replace(f'file = "{m["file"]}"', f'file = "{to}.stl"')
    with open(os.path.join(dir_, "models.toml"), "w") as f:
        f.write(text.rstrip("\n") + "\n\n[[model]]" + new.rstrip("\n") + "\n")
    print(f"cut: {len(move)} of {len(tris)} triangles of {frm} to {to}")


def main(argv):
    args, bin_, cell, names = list(argv), os.path.join(ROOT, "target/release/forgecad"), "3", []
    while args:
        a = args.pop(0)
        if a == "--forgecad":
            bin_ = args.pop(0)
        elif a == "--cell":
            cell = args.pop(0)
        else:
            names.append(a)
    if not names:
        sys.exit(__doc__)
    zips, names = names[0], names[1:] or list(MACHINES)
    for name in names:
        m = MACHINES[name]
        with tempfile.TemporaryDirectory() as tmp:
            env = dict(os.environ, XDG_CONFIG_HOME=os.path.join(tmp, "config"))
            with zipfile.ZipFile(os.path.join(zips, m["zip"])) as z:
                step = next(n for n in z.namelist() if re.search(m["step"], n) and "__MACOSX" not in n)
                src = z.extract(step, os.path.join(tmp, "zip"))
            out = run([bin_, "machine", "model", "attach", name, src, "--from", m["source"]], env)
            file = re.search(r"^(\S+) attached", out, re.M).group(1)
            run([bin_, "machine", "model", "split", name, file] + m["rules"], env)
            lib = os.path.join(tmp, "config/forgecad/cam/machines/models", name)
            text = open(os.path.join(lib, "models.toml")).read()
            blocks = text.split("[[model]]")
            for i, b in enumerate(blocks[1:], 1):
                body = re.search(r'body = "(.*)"', b).group(1)
                off = m["place"].get(body, m["place"]["*"])
                b = re.sub(r"rotate = .*", f"rotate = {m['rotate']}", b)
                blocks[i] = re.sub(r"offset = .*", f"offset = {off}", b)
            open(os.path.join(lib, "models.toml"), "w").write("[[model]]".join(blocks))
            dst = os.path.join(OUT, name)
            shutil.rmtree(dst, ignore_errors=True)
            run([bin_, "machine", "model", "ship", name, dst, "--cell", cell, "--source", m["source"]], env)
            for (frm, to, axis, at) in m.get("cut", []):
                cut(dst, frm, to, axis, at)
            size = sum(os.path.getsize(os.path.join(dst, f)) for f in os.listdir(dst))
            print(f"{name}: {size / 1e6:.2f} MB in {os.path.relpath(dst, ROOT)}")


if __name__ == "__main__":
    main(sys.argv[1:])
