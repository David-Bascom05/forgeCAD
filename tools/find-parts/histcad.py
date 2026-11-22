#!/usr/bin/env python3
"""HistCAD construction sequences as .fg part files: extrudes and revolves. Lengths in mm; a frame's Euler
angles are intrinsic XYZ, degrees. The data stays out of the repo; run with python3 -I. No volume is recorded:
replay in OpenCascade with occt_diff.sh for a truth.

HistCAD's fillets and chamfers pick their edges by 3D points; a .fg selection names edges by what made them
and how they lie, never by where (`help forge`: Selections), so a sequence with one is not translated
("edges by a point").

  histcad.py FILE.json .. --out DIR
"""
import json
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fusion

fusion.MM = 1.0


def rotation(deg):
    """The frame's axes (columns of Rx Ry Rz)."""
    a, b, c = (math.radians(d) for d in deg)
    ca, sa, cb, sb, cc, sc = math.cos(a), math.sin(a), math.cos(b), math.sin(b), math.cos(c), math.sin(c)
    return (
        (cb * cc, ca * sc + sa * sb * cc, sa * sc - ca * sb * cc),
        (-cb * sc, ca * cc - sa * sb * sc, sa * cc + ca * sb * sc),
        (sb, -sa * cb, ca * cb),
    )


def xyz(p):
    return {"x": p[0], "y": p[1], "z": 0.0}


def circle_through(s, m, e):
    ax, ay, bx, by, cx, cy = s[0], s[1], m[0], m[1], e[0], e[1]
    d = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by))
    if abs(d) < 1e-12:
        raise fusion.Unsupported("arc through three points in a line")
    a2, b2, c2 = ax * ax + ay * ay, bx * bx + by * by, cx * cx + cy * cy
    return ((a2 * (by - cy) + b2 * (cy - ay) + c2 * (ay - by)) / d, (a2 * (cx - bx) + b2 * (ax - cx) + c2 * (bx - ax)) / d)


def curves(sketch):
    """A flat sketch's primitives in the form fusion.py reads."""
    out = []
    for name, c in sketch.items():
        kind = name.rsplit("_", 1)[0]
        if kind == "line":
            out.append({"type": "Line3D", "start_point": xyz(c["start"]), "end_point": xyz(c["end"])})
        elif kind == "circle":
            out.append({"type": "Circle3D", "center_point": xyz(c["center"]), "radius": c["radius"], "normal": {"x": 0, "y": 0, "z": 1}})
        elif kind == "arc":
            s, m, e = c["start"], c["middle"], c["end"]
            ce = circle_through(s, m, e)
            r = math.dist(s, ce)
            a0 = math.atan2(s[1] - ce[1], s[0] - ce[0])
            ccw, tom = (math.atan2(e[1] - ce[1], e[0] - ce[0]) - a0) % (2 * math.pi), (math.atan2(m[1] - ce[1], m[0] - ce[0]) - a0) % (2 * math.pi)
            sweep, nz = (ccw, 1) if tom < ccw else (2 * math.pi - ccw, -1)
            ref = {"x": (s[0] - ce[0]) / r, "y": (s[1] - ce[1]) / r, "z": 0.0}
            out.append({"type": "Arc3D", "start_point": xyz(s), "end_point": xyz(e), "center_point": xyz(ce), "radius": r, "start_angle": 0.0, "end_angle": sweep, "reference_vector": ref, "normal": {"x": 0, "y": 0, "z": nz}})
        else:
            raise fusion.Unsupported("curve " + kind)
    return out


def translate(seq):
    """fusion.write's steps for a sequence; raises fusion.Unsupported."""
    steps, bodies = [], 0
    for s in seq:
        op = s["operation"]
        if op in ("Fillet", "Chamfer"):
            raise fusion.Unsupported("edges by a point")
        if "sketch" not in s:
            raise fusion.Unsupported("operation " + op)
        want = {"NewBody": "new", "Join": "add", "Cut": "cut", "Intersect": "keep"}.get(op)
        if want is None:
            raise fusion.Unsupported("operation " + op)
        cs = s["coordinate_system"]
        X, Y, Z = rotation(cs["Euler Angles"])
        O = tuple(cs["Translation Vector"])
        regs = fusion.regions([{"loops": [{"profile_curves": curves(s["sketch"])}]}])
        this = want
        if bodies == 0:
            if want in ("cut", "keep"):
                raise fusion.Unsupported("first feature cuts")
            this = ""
        label = None
        if this in ("", "new"):
            label = f"b{bodies}"
            bodies += 1
        into = "all" if bodies > 1 and want in ("cut", "keep") else None
        if "axis" in s:
            if s.get("start", 0.0) != 0.0:
                raise fusion.Unsupported("revolve not starting at its profile")
            if len(regs) > 1:
                raise fusion.Unsupported("a revolve of several regions")
            a, b = s["axis"]
            rel = (a[0] - O[0], a[1] - O[1], a[2] - O[2])
            u, v = fusion.dot(rel, X), fusion.dot(rel, Y)
            d = (b[0] - a[0], b[1] - a[1], b[2] - a[2])
            du, dv = fusion.dot(d, X), fusion.dot(d, Y)
            loops = [fusion.emit_loop(lp, fusion.Frame(X, Y, Y), "o" if j == 0 else f"i{j}") for r in regs for j, lp in enumerate(r)]
            steps.append({"kind": "revolve", "op": this, "label": label, "into": into, "z": Z, "x": X, "at": O, "axis": (u, v, du, dv),
                          "turn": s.get("end", 360.0), "rows": [x[0] for x in loops], "loops": [x[1] for x in loops]})
            continue
        up, down = float(s.get("towards") or 0.0), float(s.get("opposite") or 0.0)
        lo, hi = -down, up
        if lo > hi:
            lo, hi = hi, lo
        if hi <= 0:
            z, a, b, shift = tuple(-x for x in Z), hi - lo, 0.0, hi
        elif lo >= 0:
            z, a, b, shift = Z, hi - lo, 0.0, lo
        else:
            z, a, b, shift = Z, hi, -lo, 0.0
        org = fusion.add(O, Z, shift)
        y = fusion.cross(z, X)
        f = fusion.Frame(X, Y, y)
        for k, r in enumerate(regs):
            loops = [fusion.emit_loop(lp, f, "o" if j == 0 else f"i{j}") for j, lp in enumerate(r)]
            # each region its own extrude, as drawn: after the first, a first feature's regions join it
            steps.append({"kind": "extrude", "op": this if k == 0 or this else "add", "label": label if k == 0 else None,
                          "into": into, "z": z, "x": X, "at": org, "up": a, "down": b, "taper": 0.0,
                          "rows": [x[0] for x in loops], "loops": [x[1] for x in loops]})
    return steps


def main():
    a = sys.argv[1:]
    out = a[a.index("--out") + 1]
    os.makedirs(out, exist_ok=True)
    for fn in a:
        if fn.endswith(".json"):
            stem = os.path.basename(fn)[:-5]
            with open(fn) as f:
                seq = json.load(f)
            try:
                text = fusion.write(translate(seq))
            except (fusion.Unsupported, KeyError) as x:
                print(stem, "not translated:", repr(x)[:80])
                continue
            with open(os.path.join(out, stem + ".fg"), "w") as f:
                f.write(text)
            print(stem, "translated")


if __name__ == "__main__":
    main()
