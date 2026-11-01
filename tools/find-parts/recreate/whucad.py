#!/usr/bin/env python3
"""WHUCAD vector sequences to .fc. Run with the venv that has h5py (a safe reader), -I; data stays outside the repo.

Every value is quantised to 256 levels of a normalised box, so coincidences and tangencies in the source are only
nearly kept. It is a robustness smoke test: compare to the STEP only coarsely, by shape after scaling.

A selection is CATIA's name for a face: Pad K's side from curve c of sketch S, or its caps (no 1, 2); an edge is the
faces that meet in it. Those resolve like CADFS's queries, against the part as built; faces of later features do not.

  whucad.py FILE.h5 --topo EXE --out OUT.fc       prints the outcome as JSON
"""
import json
import math
import os
import sys

import h5py

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from cadfs import Any, Both, KernelStop, OnPlane, Swept, Turned, Unresolved, Unsupported, add, dot, num, vtext
from freecad import Tr as FT, along_tangents, even_regions
from sketch2d import Arrangement, Piece

CMD = ["Line", "Arc", "Circle", "Spline", "SCP", "EOS", "SOL", "Ext", "Rev", "Pocket", "Groove", "Shell", "Chamfer", "Fillet", "Draft", "Mirror", "Hole", "Topo", "Select", "MirrorStart", "NSI", "NSIE", "AOI1", "AOI2", "AOIE", "API", "APIE"]
SEL = ["Wire", "Face", "Edge", "Multiply_Face", "Sub_Face"]
BODY = ["None", "OriginElements", "Sketch", "Pad", "Shaft", "Pocket", "Add", "Remove", "Intersect", "Shell", "Chamfer", "EdgeFillet"]
TOL = 2e-3
N = 256.0


def unq(v):
    return v / N * 2 - 1


class Frame:
    def __init__(self, r):
        th, ph, ga = (unq(r[6]) * math.pi, unq(r[7]) * math.pi, unq(r[8]) * math.pi)
        self.n = (math.sin(th) * math.cos(ph), math.sin(th) * math.sin(ph), math.cos(th))
        rx = (math.cos(th) * math.cos(ph), math.cos(th) * math.sin(ph), -math.sin(th))
        ry = (self.n[1] * rx[2] - self.n[2] * rx[1], self.n[2] * rx[0] - self.n[0] * rx[2], self.n[0] * rx[1] - self.n[1] * rx[0])
        self.x = tuple(a * math.cos(ga) + b * math.sin(ga) for a, b in zip(rx, ry))
        self.y = (self.n[1] * self.x[2] - self.n[2] * self.x[1], self.n[2] * self.x[0] - self.n[0] * self.x[2], self.n[0] * self.x[1] - self.n[1] * self.x[0])
        self.o = (unq(r[9]), unq(r[10]), unq(r[11]))
        self.size = r[12] / N * 2

    def uvw(self, p):
        d = add(p, self.o, -1)
        return dot(d, self.x), dot(d, self.y), dot(d, self.n)

    def world(self, u, v, w=0.0):
        return add(add(add(self.o, self.x, u), self.y, v), self.n, w)


def curves(rows, scale):
    """Sketch rows as entities, numbered from 1 as CATIA's selections count them."""
    f = lambda v: (v - 128.0) * scale
    ents, k, i = [], 0, 0
    while i < len(rows):
        if CMD[int(rows[i][0])] != "SOL":
            i += 1
            continue
        j = i + 1
        while j < len(rows) and CMD[int(rows[j][0])] != "SOL":
            j += 1
        lp = [r for r in rows[i + 1 : j] if CMD[int(r[0])] in ("Line", "Arc", "Circle", "Spline", "SCP")]
        if not lp:
            i = j
            continue
        if any(CMD[int(r[0])] in ("Spline", "SCP") for r in lp):
            raise Unsupported("a spline in a sketch")
        last = lp[-1]
        at = (f(last[1]), f(last[2]))
        for r in lp:
            c = CMD[int(r[0])]
            k += 1
            end = (f(r[1]), f(r[2]))
            if c == "Line":
                ents.append({"kind": "skLineSegment", "id": f"c{k}", "start": list(at), "end": list(end)})
                at = end
            elif c == "Circle":
                ents.append({"kind": "skCircle", "id": f"c{k}", "center": list(end), "radius": r[5] * scale})
            elif c == "Arc":
                sw = r[3] / N * 2 * math.pi
                ch = math.dist(at, end)
                if ch == 0 or sw == 0:
                    raise Unresolved("an arc quantised away")
                rad = (ch / 2) / math.sin(sw / 2)
                m = ((at[0] + end[0]) / 2, (at[1] + end[1]) / 2)
                ve = ((end[1] - at[1]) / ch, -(end[0] - at[0]) / ch)
                s = 1.0 if r[4] else -1.0
                ce = (m[0] - s * ve[0] * rad * math.cos(sw / 2), m[1] - s * ve[1] * rad * math.cos(sw / 2))
                a0 = math.atan2(at[1] - ce[1], at[0] - ce[0]) + (sw / 2 if r[4] else -sw / 2)
                mid = (ce[0] + rad * math.cos(a0), ce[1] + rad * math.sin(a0))
                ents.append({"kind": "skArc", "id": f"c{k}", "start": list(at), "mid": list(mid), "end": list(end)})
                at = end
        i = j
    return ents


class Tr(FT):
    def __init__(self, v, topo):
        self.lines, self.topo, self.topo_exe = [], None, topo
        self.bodies = 0
        self.v = v
        self.sk = {}  # sketch number: (Arrangement, Frame)
        self.solid = {}  # ("Pad", k): record
        self.count = {"Pad": 0, "Pocket": 0, "Shaft": 0, "Sketch": 0}
        self.kinds = []

    def groups(self, rows):
        """A finishing feature's selection rows, split at each Topo; NoShared and the like are left out."""
        out, cur, skip = [], None, False
        for r in rows:
            c = CMD[int(r[0])]
            if c == "Topo":
                cur = []
                out.append(cur)
            elif c in ("NSI", "AOI1", "AOI2", "API", "MirrorStart"):
                skip = True
            elif c in ("NSIE", "AOIE", "APIE"):
                skip = False
            elif c == "Select" and not skip and cur is not None:
                cur.append((SEL[int(r[29])], BODY[int(r[30])] if int(r[30]) < len(BODY) else "other", int(r[31]), int(r[32])))
        return out

    def face_carrier(self, wires, b, k, no):
        rec = self.solid.get((b, k))
        if rec is None:
            raise Unresolved(f"a face of {b}")
        fr = rec["frame"]
        if "a2" in rec:
            if no != 0 or not wires:
                raise Unresolved("a cap of a revolve")
            s, c = wires[-1]
            e = self.sk[s][0].every.get(f"c{c}")
            if e is None:
                raise Unresolved("a curve the sketch does not have")
            return Turned(fr, e, rec["a2"], rec["d2"])
        if no in (1, 2):
            # no 2 is the limit along the direction, no 1 the one behind it, whichever way they lie: on a pad reversed, no 1 is still at the sketch
            z = rec["first"] if no == 2 else rec["second"]
            if b == "Pocket":  # no 2 is the floor, the limit farther from the sketch, whichever way it lies
                far, near = (rec["lo"], rec["hi"]) if abs(rec["lo"]) >= abs(rec["hi"]) else (rec["hi"], rec["lo"])
                z = far if no == 2 else near
            return OnPlane(fr.world(0, 0, z), fr.n)
        if no == 0 and wires:
            s, c = wires[-1]
            a = self.sk[s][0]
            e = a.every.get(f"c{c}")
            if e is None:
                raise Unresolved("a curve the sketch does not have")
            return Swept(fr, e, min(rec["lo"], rec["hi"]), max(rec["lo"], rec["hi"]))
        raise Unresolved("a face form")

    def items(self, group):
        stack, subs, wires, out = [], [], [], []
        for t, b, k, no in group:
            if t == "Wire":
                if b != "Sketch":
                    raise Unresolved("a wire of " + b)
                wires.append((k, no))
            elif t in ("Face", "Sub_Face"):
                if b not in ("Pad", "Pocket", "Shaft"):
                    raise Unresolved(f"a face of {b}")
                (subs if t == "Sub_Face" else stack).append(self.face_carrier(wires, b, k, no))
                wires = []
            elif t == "Multiply_Face":
                if subs:
                    stack.append(Any(subs))
                subs = []
            elif t == "Edge":
                if len(stack) < 2:
                    raise Unresolved("an edge of fewer than two faces")
                c = stack[0]
                for x in stack[1:]:
                    c = Both(c, x)
                out.append(("edge", c))
                stack = []
        out += [("face", c) for c in stack]
        return out

    def finish(self, rows, head, faces_only=False):
        st = self.state()
        edges, faces = st.get("edges", []), st.get("faces", [])
        picked = {}
        for g in self.groups(rows):
            for kind, c in self.items(g):
                if kind == "edge":
                    hit = [e for e in edges if all(c.dist(p) <= TOL for p in e["p"])]
                    if not hit:
                        raise Unresolved("no edge where the faces meet")
                    picked.update({e["h"]: e for e in hit})
                else:
                    hit = [f for f in faces if all(c.dist(p) <= TOL for p in f["p"])]
                    if not hit:
                        raise Unresolved("no such face")
                    if faces_only:
                        picked.update({f["h"]: f for f in hit})
                    else:
                        for f in hit:
                            picked.update({e["h"]: e for e in edges if f["h"] in e["f"] and e["c"] != "smooth"})
        if not picked:
            raise Unresolved("nothing selected")
        if not faces_only:
            along_tangents(edges, picked)  # CATIA's fillets and chamfers run on along tangents
        self.lines.append(head + " " + " ".join(sorted(picked)))

    def extrude(self, rows, r, kind):
        fr = Frame(r)
        self.count["Sketch"] += 1
        s = self.count["Sketch"]
        a = Arrangement(curves(rows, fr.size / 95.0))
        self.sk[s] = (a, fr)
        regs = even_regions(a)
        if not regs:
            raise Unresolved("no closed region")
        if r[15] != 0 or r[16] != 0:
            raise Unsupported("an up-to limit")
        e1, e2 = unq(r[13]), unq(r[14])
        cut = kind == "Pocket" or r[19] == 1
        if r[19] == 2:
            raise Unsupported("an intersect")
        # limits are signed: first along the direction, second against it; a pocket's direction is against the normal
        ends = (-e1, e2) if kind == "Pocket" else (e1, -e2)
        lo, hi = min(ends), max(ends)
        if hi - lo <= 1e-9:
            raise Unresolved("an extent quantised to nothing")
        name = "Pocket" if kind == "Pocket" else "Pad"
        self.count[name] += 1
        self.solid[(name, self.count[name])] = {"frame": fr, "lo": lo, "hi": hi, "first": ends[0], "second": ends[1]}
        head = num(hi - lo)
        flip = False
        frame = f"z {vtext(fr.n)} x {vtext(fr.x)} at {vtext(fr.world(0, 0, lo))}"
        from cadfs import Tr as C
        for prof in a.merged(regs):
            loops = " ".join(C.loop_text(self, a, lp, flip) for lp in prof)
            op = "cut" if cut else "add"
            if self.bodies == 0:
                if cut:
                    raise Unsupported("first feature cuts")
                op = ""
                self.bodies = 1
            self.lines.append(f"extrude {op + ' ' if op else ''}{head} {frame} {loops}")

    def mirror(self, rows):
        g = self.groups(rows)
        if len(g) != 1 or len(g[0]) < 1:
            raise Unresolved("a mirror's plane")
        t, b, k, no = g[0][-1]
        if b == "OriginElements":
            n = {1: (0.0, 0.0, 1.0), 2: (1.0, 0.0, 0.0), 3: (0.0, 1.0, 0.0)}.get(no)
            if n is None:
                raise Unresolved("a mirror's plane")
            o = (0.0, 0.0, 0.0)
        else:
            cs = self.items(g[0])
            c = cs[0][1] if cs else None
            if isinstance(c, Any):
                c = c.cs[0]
            if isinstance(c, Swept) and c.piece.kind == "line":
                # a flat side: the plane through its sketch line and the extrude's direction
                fr, e = c.fr, c.piece
                p, q = fr.world(*e.a), fr.world(*e.b)
                d = add(q, p, -1)
                k = (d[1] * fr.n[2] - d[2] * fr.n[1], d[2] * fr.n[0] - d[0] * fr.n[2], d[0] * fr.n[1] - d[1] * fr.n[0])
                l = math.sqrt(dot(k, k))
                c = OnPlane(p, tuple(x / l for x in k))
            if not isinstance(c, OnPlane):
                raise Unsupported("a mirror in a curved face")
            n, o = c.n, c.o
        self.lines.append(f"pattern body mirror z {vtext(n)} at {vtext(o)}")

    def revolve(self, rows, r, cut):
        fr = Frame(r)
        self.count["Sketch"] += 1
        s = self.count["Sketch"]
        a = Arrangement(curves(rows, fr.size / 95.0))
        self.sk[s] = (a, fr)
        regs = even_regions(a)
        if not regs:
            raise Unresolved("no closed region")
        sel = [g for g in self.groups(rows)]
        if not sel or not sel[0]:
            raise Unresolved("a revolve with no axis")
        t, b, k, no = sel[0][0]
        if b == "OriginElements":
            o2, d2 = (0.0, 0.0), ((1.0, 0.0) if no == 1 else (0.0, 1.0))
        elif b == "Sketch" and k == s and a.every.get(f"c{no}") is not None and a.every[f"c{no}"].kind == "line":
            e = a.every[f"c{no}"]
            l = math.dist(e.a, e.b)
            o2, d2 = e.a, ((e.b[0] - e.a[0]) / l, (e.b[1] - e.a[1]) / l)
        else:
            raise Unresolved("a revolve axis on " + b)
        ang = r[17] / 255.0 * 360
        back = r[18] / 255.0 * 360
        if not (ang >= 359.9 and back == 0):
            if back != 0:
                raise Unsupported("a revolve both ways")
        head = "" if ang >= 359.9 else num(ang)
        name = "Groove" if cut else "Shaft"
        self.count.setdefault(name, 0)
        self.count[name] += 1
        self.solid[(name, self.count[name])] = {"frame": fr, "a2": o2, "d2": d2}
        frame = f"z {vtext(fr.n)} x {vtext(fr.x)} at {vtext(fr.o)} axis {num(o2[0])} {num(o2[1])} {num(d2[0])} {num(d2[1])}"
        from cadfs import Tr as C
        for prof in a.merged(regs):
            loops = " ".join(C.loop_text(self, a, lp, False) for lp in prof)
            op = "cut" if cut or r[19] == 1 else "add"
            if self.bodies == 0:
                if op == "cut":
                    raise Unsupported("first feature cuts")
                op = ""
                self.bodies = 1
            self.lines.append(f"revolve {op + ' ' if op else ''}{head} {frame} {loops}")

    def run(self):
        pending = []
        for r in self.v:
            c = CMD[int(r[0])]
            if c == "EOS":
                break
            if c in ("Ext", "Pocket"):
                self.kinds.append("extrude" if c == "Ext" else "pocket")
                self.extrude(pending, r, c)
                pending = []
            elif c in ("Rev", "Groove"):
                self.kinds.append("revolve" if c == "Rev" else "groove")
                self.revolve(pending, r, c == "Groove")
                pending = []
            elif c == "Mirror":
                self.kinds.append("mirror")
                self.mirror(pending)
                pending = []
            elif c in ("Draft", "Hole"):
                raise Unsupported("feature " + c)
            elif c == "Fillet":
                self.kinds.append("fillet")
                self.finish(pending, f"fillet {num(unq(r[24]))}")
                pending = []
            elif c == "Chamfer":
                self.kinds.append("chamfer")
                if abs(r[22] - r[23]) > 1:
                    raise Unsupported("an unequal chamfer")
                # CATIA runs three chamfers of one feature on to a point at a corner
                self.finish(pending, f"chamfer {num(unq(r[22]))} corner point")
                pending = []
            elif c == "Shell":
                self.kinds.append("shell")
                if r[21] > 128:
                    raise Unsupported("a shell with outer thickness")
                self.finish(pending, f"shell {num(unq(r[20]))}", True)
                pending = []
            else:
                pending.append(r)


def main():
    a = sys.argv[1:]
    opt = lambda k: a[a.index(k) + 1]
    with h5py.File(a[0], "r") as h:
        v = h["vec"][()]
    v = [[float(x) for x in row] for row in v]
    r = {"status": "translated", "why": "", "notes": ["quantised"]}
    t = None
    try:
        t = Tr(v, opt("--topo"))
        t.run()
        if not t.lines:
            raise Unsupported("no solid")
    except Unsupported as x:
        r.update(status="untranslated", cls="form", why=str(x))
    except Unresolved as x:
        r.update(status="untranslated", cls="reference", why=str(x))
    except KernelStop as x:
        r.update(status="stopped", why=str(x)[:200])
    except (KeyError, IndexError, TypeError, ValueError, ZeroDivisionError, AttributeError) as x:
        r.update(status="untranslated", cls="importer", why="bug: " + repr(x)[:70])
    if t is not None:
        r["features"] = t.kinds
        if t.topo is not None:
            t.topo.kill()
            t.topo.wait()
        if r["status"] != "untranslated":
            with open(opt("--out"), "w") as f:
                f.write("\n".join(t.lines) + "\n")
    print(json.dumps(r))


if __name__ == "__main__":
    main()
