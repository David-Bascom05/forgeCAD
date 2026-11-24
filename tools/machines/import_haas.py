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


# Automatic mode (VMCs): a model split by keywords in its parts' names onto the record's table and
# head and the display-only saddle, base and enclosure (tools/machines/haas.py AUTO_MODELS), placed by
# measuring it: the table top centre is the origin, the castings behind it are +y, Y up in the file.
# The spindle nose face over the table against the record's published height says whether the model
# stands at Z home; one that does is taken as at home in X and Y too (Haas's tutorial; the VF-2's
# agrees to 0.4 mm), its spindle position measured; one that does not has its head moved to the
# record's home. The measures go to research/cam/machines/haas/model-measures.json for haas.py.
AUTO = {
    "haas-vf-1": "vf-1_11_2021.zip",
    "haas-tm-1": "tm_1_1p_10_2021.zip",
    "haas-dm-1": "dm-1_solid_models_01_2026.zip",
    "haas-vf-3": "vf-3_ss_solid_models_2_2023.zip",
    "haas-vf-4": "vf-4_4ss_solid_models_1_2022.zip",
    "haas-mini-mill": "mini_mill_solid_models_2_2022.zip",
    "haas-tm-2": "tm_2_2p_10_2021-.zip",
    "haas-dm-2": "dm-2_solid_models_04_2025.zip",
    "haas-dt-2": "dt-2_solid_models_04_2025.zip",
    "haas-vf-2yt": "vf-2yt_2ssyt_solid_models_11_2021.zip",
    "haas-vf-5-40": "vf-5_40_ss_tr_solid_models_3_2022.zip",
}
VMC_RULES = ["anchor=-", "waycover=-", "table=table", "saddle=saddle", "spindle=head", "z axis=head", "head=head",
             "casting=base", "base=base", "column=base", "pads=base", "leveling=base", "enclosure=enclosure",
             "door=enclosure", "window=enclosure", "control panel=enclosure", "pendant=enclosure", "chute=enclosure",
             "*=-"]
MEASURES = os.path.join(ROOT, "research/cam/machines/haas/model-measures.json")


def stl_points(p):
    _, tris = read_stl(p)
    out = []
    for t in tris:
        v = struct.unpack("<12f", t[:48])
        out += [v[3:6], v[6:9], v[9:12]]
    return out


def nose_piece(p):
    """The spindle nose in a head mesh: of its connected pieces (triangles sharing corners), the
    lowest one round in plan, 90 to 230 mm across (a 40- or 50-taper nose, its drive keys on it).
    Its plan centre is the spindle axis; its bottom the nose (the keys' faces, 9 mm under the nose
    face on the VF-2). None where no piece is."""
    d = open(p, "rb").read()
    n = struct.unpack("<I", d[80:84])[0]
    idx, up = {}, []

    def root(i):
        while up[i] != i:
            up[i] = up[up[i]]
            i = up[i]
        return i

    for k in range(n):
        v = struct.unpack("<12f", d[84 + 50 * k: 84 + 50 * k + 48])
        ids = []
        for j in (3, 6, 9):
            key = v[j:j + 3]
            if key not in idx:
                idx[key] = len(up)
                up.append(len(up))
            ids.append(idx[key])
        a = root(ids[0])
        for b in ids[1:]:
            b = root(b)
            if a != b:
                up[b] = a
    groups = {}
    for key, i in idx.items():
        groups.setdefault(root(i), []).append(key)
    best = None
    for g in groups.values():
        lo = [min(q[k] for q in g) for k in range(3)]
        hi = [max(q[k] for q in g) for k in range(3)]
        ex, ey, ez = hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]
        if 90 <= ex <= 230 and 90 <= ez <= 230 and max(ex, ez) / min(ex, ez) < 1.15 and ey >= 15:
            if best is None or lo[1] < best[0][1]:
                best = (lo, hi)
    return best


def record(name):
    import tomllib
    d = tomllib.load(open(os.path.join(ROOT, "kernel/forge-machine/data/machines/haas.toml"), "rb"))
    return next(m for m in d["machine"] if m["name"] == name)


def auto(name, zips, bin_, cell):
    import json
    rec = record(name)
    nose = rec["spindle"][0]["nose"]
    with tempfile.TemporaryDirectory() as tmp:
        env = dict(os.environ, XDG_CONFIG_HOME=os.path.join(tmp, "config"))
        with zipfile.ZipFile(os.path.join(zips, AUTO[name])) as z:
            # the whole machine: the zip's largest STEP (a Mini Mill's also holds its Z axis alone)
            step = max((i for i in z.infolist() if re.search(r"\.ste?p$", i.filename, re.I) and "__MACOSX" not in i.filename),
                       key=lambda i: i.file_size).filename
            src = z.extract(step, os.path.join(tmp, "zip"))
        source = f"Haas Automation solid model {os.path.basename(step)} (haascnc.com download)"
        out = run([bin_, "machine", "model", "attach", name, src, "--from", source], env)
        file = re.search(r"^(\S+) attached", out, re.M).group(1)
        run([bin_, "machine", "model", "split", name, file] + VMC_RULES, env)
        lib = os.path.join(tmp, "config/forgecad/cam/machines/models", name)
        _, ms = models(lib)
        pts = {b: stl_points(os.path.join(lib, m["file"])) for b, m in ms.items()}
        tp = pts["table"]
        top = max(p[1] for p in tp)
        face = [p for p in tp if p[1] > top - 0.5]
        cx = (min(p[0] for p in face) + max(p[0] for p in face)) / 2
        cz = (min(p[2] for p in face) + max(p[2] for p in face)) / 2
        bz = sum(p[2] for p in pts["base"]) / len(pts["base"])
        front_z = bz < cz  # the castings (the column) at smaller z: the front is +z, Haas's VF-2 way
        rot = [90, 0, 0] if front_z else [90, 0, 180]

        def ours(p):
            return (p[0] - cx, -(p[2] - cz), p[1] - top) if front_z else (-(p[0] - cx), p[2] - cz, p[1] - top)

        off = [-cx, cz, -top] if front_z else [cx, -cz, -top]
        hp = pts["head"]
        low = min(p[1] for p in hp)
        bottom = [p for p in hp if p[1] < low + 15]
        ext = [max(p[k] for p in bottom) - min(p[k] for p in bottom) for k in (0, 2)]
        if all(60 <= e <= 230 for e in ext) and max(ext) / min(ext) < 1.3:
            # the head's lowest 15 mm a nose in plan: the nose (with its keys) is the bottom
            np_ = ([min(p[k] for p in bottom) for k in range(3)], [max(p[k] for p in bottom) for k in range(3)])
        else:
            np_ = nose_piece(os.path.join(lib, ms["head"]["file"]))
        if np_ is None:
            sys.exit(f"{name}: no spindle nose found in the head")
        hx, hz, face_y = (np_[0][0] + np_[1][0]) / 2, (np_[0][2] + np_[1][2]) / 2, np_[0][1]
        sx, sy, sh = *ours((hx, face_y, hz))[:2], face_y - top
        dz = nose[2] - sh
        # at home: the nose within 15 mm of the published height (the keys stand 9 mm under the
        # nose face), and the spindle within the travels' reach of the record's estimate
        at_home = abs(dz) < 15 and abs(sx - nose[0]) < 150 and abs(sy - nose[1]) < 150
        place = {"*": off}
        if not at_home:
            # the head to the record's home: its published height, and its x, y (the model's unknown)
            d = [nose[0] - sx, nose[1] - sy, dz]
            place["head"] = [off[k] + d[k] for k in range(3)]
        text, _ = models(lib)
        blocks = text.split("[[model]]")
        for i, b in enumerate(blocks[1:], 1):
            body = re.search(r'body = "(.*)"', b).group(1)
            b = re.sub(r"rotate = .*", f"rotate = {rot}", b)
            blocks[i] = re.sub(r"offset = .*", f"offset = {place.get(body, place['*'])}", b)
        open(os.path.join(lib, "models.toml"), "w").write("[[model]]".join(blocks))
        boxes = {}
        for b, ps in pts.items():
            q = [ours(p) for p in ps]
            if b == "head" and not at_home:
                q = [(x + place["head"][0] - off[0], y + place["head"][1] - off[1], z + dz) for x, y, z in q]
            boxes[b] = [[round(min(v[k] for v in q), 1) for k in range(3)], [round(max(v[k] for v in q), 1) for k in range(3)]]
        dst = os.path.join(OUT, name)
        shutil.rmtree(dst, ignore_errors=True)
        run([bin_, "machine", "model", "ship", name, dst, "--cell", cell, "--source", source], env)
        size = sum(os.path.getsize(os.path.join(dst, f)) for f in os.listdir(dst))
        meas = json.load(open(MEASURES)) if os.path.exists(MEASURES) else {}
        meas[name] = dict(file=os.path.basename(step), at_home=at_home, nose_over_table=round(sh, 1),
                          published_nose=nose[2], spindle_xy=[round(sx, 1), round(sy, 1)], front_z=front_z,
                          bodies=boxes, mb=round(size / 1e6, 2))
        os.makedirs(os.path.dirname(MEASURES), exist_ok=True)
        with open(MEASURES, "w") as f:
            json.dump(dict(sorted(meas.items())), f, indent=1)
            f.write("\n")
        print(f"{name}: {size / 1e6:.2f} MB; nose {sh:.1f} over the table (published {nose[2]}), "
              f"{'at home' if at_home else 'head moved to home'}; spindle ({sx:.1f}, {sy:.1f}) vs record ({nose[0]}, {nose[1]})")


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
    zips, names = names[0], names[1:] or list(MACHINES) + list(AUTO)
    for name in names:
        if name in AUTO:
            auto(name, zips, bin_, cell)
            continue
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
