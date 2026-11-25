"""Small Parasolid transmit files written from the XT Format Reference, for the reader's tests.

    python3 research/parasolid/fixtures.py kernel/forge-step/tests/data/xt

Each model is built as nodes (type, fields in the base schema SCH_13006's order) and written as
text and binary: plain (schema SCH_1300000_13006, the base itself), or with the schema carried as
edits of the base (every type written unchanged, the 255 flag), as files from Parasolid V14 on.
Lengths in metres, as Parasolid writes them.
"""
import math
import struct
import sys
from pathlib import Path

# the base schema's field types, by node type, as the reader's `schema.rs` has them
G = "d p p p p p c"
BASE = {
    10: "d p p p p p p p f f p p p u p u p",
    11: "d p u p p p p p p p",
    12: "d p p p p p p f f p p p u p u u p p p p p p p",
    13: "d p p p p p p p p",
    14: "d p f p p p p p c p p p p p",
    15: "d p p p p",
    16: "d p f p p p p p p p",
    17: "p p p p p p p p p c",
    18: "d p p p p p f p",
    19: "d p p p p p c",
    29: "d p p p p v",
    30: G + " v v",
    31: G + " v v v f",
    50: G + " v v v",
    51: G + " v v f v",
    100: "d p p p f9 v f d v",
    # lists and attributes: `*` marks the variable field, its length written before the index
    74: "d p p*",
    79: "c*",
    80: "p p d u8 p l14 u*",
    81: "d p p p p p p p*",
    84: "c*",
    98: "w*",
}
VARIABLE = set()


class Model:
    def __init__(self):
        self.nodes = {}  # index -> (type, [values])
        self.next = 1

    def add(self, ty, vals, at=None):
        i = at if at is not None else self.next
        self.nodes[i] = [ty, vals]
        self.next = max(self.next, i + 1)
        return i

    def reserve(self):
        i = self.next
        self.next += 1
        return i

    def set(self, i, ty, vals):
        self.nodes[i] = [ty, vals]


def types_of(ty):
    """(code, count) per field: count 0 for a scalar, n for an array, -1 for the variable field
    (its values one list)."""
    out = []
    for t in BASE[ty].split():
        if t.endswith("*"):
            out.append((t[0], -1))
        elif len(t) > 1:
            out.append((t[0], int(t[1:])))
        else:
            out.append((t, 0))
    return out


def var_len(ty, vals):
    """The variable field's length (the last field's values), or None for a fixed node."""
    ts = types_of(ty)
    if ts and ts[-1][1] == -1:
        return len(vals[-1])
    return None


def text(m, embedded):
    hdr = (
        "**ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz**************************\n"
        "**PARASOLID !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~0123456789**************************\n"
        "**PART1;\nMC=unknown;\nAPPL=forgecad-fixture;\nFORMAT=text;\nGUISE=transmit;\n"
        "**PART2;\nSCH=SCH_1300000_13006;\nUSFLD_SIZE=0;\n**PART3;\n"
        "**END_OF_HEADER*****************************************************************\n"
    )
    ver = ": TRANSMIT FILE created by modeller version 1300000"
    sch = "SCH_1300000_13006_13006" if embedded else "SCH_1300000_13006"
    out = ["T", f"{len(ver)} {ver}", f"{len(sch)} {sch}"]
    if embedded:
        out.append("230 ")
    out.append("0 ")
    seen = set()

    def num(x):
        if x is None:
            return "?"
        if isinstance(x, float):
            s = repr(x)
            if s.endswith(".0"):
                s = s[:-2]
            return s + " "
        return f"{x} "

    for i in sorted(m.nodes, key=lambda k: (k != 1, k)):
        ty, vals = m.nodes[i]
        s = f"{ty} "
        if embedded and ty not in seen:
            s += "255 "
        seen.add(ty)
        vl = var_len(ty, vals)
        if vl is not None:
            s += f"{vl} "
        s += f"{i} "
        it = iter(vals)
        for t, n in types_of(ty):
            if n and t in "cl":
                s += "".join(next(it))
            elif n and t in "updw":
                s += "".join(num(x) for x in next(it))
            elif t in "cl":
                s += next(it)
            elif t == "v":
                v = next(it)
                s += "?" if v is None else "".join(num(float(x)) for x in v)
            elif n:
                s += "".join(num(float(next(it))) for _ in range(n))
            elif t == "f":
                v = next(it)
                s += num(None if v is None else float(v))
            else:
                s += num(next(it))
        out.append(s)
    out.append("1 0 ")
    body = "".join(out)
    # records of 80 characters; a record may not end in a space, so one moves to the next
    res = []
    while body:
        n = 80
        while n > 1 and body[n - 1 : n] == " ":
            n -= 1
        res.append(body[:n])
        body = body[n:]
    return hdr + "\n".join(res) + "\n"


def binary(m, little):
    e = "<" if little else ">"
    flag = b"PS\x00\x01" + bytes([1 if little else 0, 0, 0]) if little else b"PS\x00\x00"
    ver = b": TRANSMIT FILE created by modeller version 1300000"
    sch = b"SCH_1300000_13006"
    out = bytearray(flag)
    out += struct.pack(e + "h", len(ver)) + ver
    out += struct.pack(e + "i", len(sch)) + sch
    out += struct.pack(e + "i", 0)

    def ptr(i):
        if i is None:
            i = 0
        if i < 32767:
            return struct.pack(e + "h", i + 1)
        return struct.pack(e + "h", -(i % 32767 + 1)) + struct.pack(e + "h", i // 32767)

    null_real = -3.14158e13
    for i in sorted(m.nodes, key=lambda k: (k != 1, k)):
        ty, vals = m.nodes[i]
        out += struct.pack(e + "h", ty)
        vl = var_len(ty, vals)
        if vl is not None:
            out += struct.pack(e + "i", vl)
        out += ptr(i)
        it = iter(vals)
        for t, n in types_of(ty):
            if n and t != "f":
                xs = next(it)
                if t == "c":
                    out += "".join(xs).encode()
                elif t == "l":
                    out += bytes(1 if x == "T" else 0 for x in xs)
                elif t == "u":
                    out += bytes(xs)
                elif t in "nw":
                    out += b"".join(struct.pack(e + "h", x) for x in xs)
                elif t == "d":
                    out += b"".join(struct.pack(e + "i", x) for x in xs)
                elif t == "p":
                    out += b"".join(ptr(x) for x in xs)
                continue
            if t == "c":
                out += next(it).encode()
            elif t == "l":
                out += bytes([1 if next(it) == "T" else 0])
            elif t == "u":
                out += bytes([next(it)])
            elif t == "n":
                out += struct.pack(e + "h", next(it))
            elif t == "d":
                v = next(it)
                out += struct.pack(e + "i", -32764 if v is None else v)
            elif t == "p":
                out += ptr(next(it))
            elif t == "v":
                v = next(it)
                for x in v if v is not None else (null_real,) * 3:
                    out += struct.pack(e + "d", float(x))
            elif t == "f":
                for _ in range(max(n, 1)):
                    v = next(it)
                    out += struct.pack(e + "d", null_real if v is None else float(v))
    out += struct.pack(e + "h", 1) + ptr(0)
    return bytes(out)


def solid(m, faces, edges, verts, owner=0):
    """A body: faces as (surface ty, surface fields, sense, loops), each loop a list of
    (edge, fin sense); edges as (curve ty, curve fields, start vertex, end vertex) with 0 for a
    ring; verts as points. Returns the body's index."""
    body = m.reserve()
    void, solid_r = m.reserve(), m.reserve()
    shell = m.reserve()
    vx = []
    for p in verts:
        v, pt = m.reserve(), m.reserve()
        m.set(pt, 29, [m.next, 0, v, 0, 0, p])
        vx.append((v, pt))
    for (v, pt) in vx:
        m.set(v, 18, [m.next, 0, 0, 0, 0, pt, None, body])
    # edges with their curves; fins made per use
    ed = []
    for (cty, cvals, a, b) in edges:
        e, c = m.reserve(), m.reserve()
        m.set(c, cty, [m.next, 0, e, 0, 0, 0, "+"] + cvals)
        ed.append([e, c, a, b, []])
    face_ids = []
    for (sty, svals, fsense, loops) in faces:
        f, s = m.reserve(), m.reserve()
        m.set(s, sty, [m.next, 0, f, 0, 0, 0, "+"] + svals)
        lps = []
        for lp in loops:
            l = m.reserve()
            fins = [m.reserve() for _ in lp]
            for k, ((ei, sense), fin) in enumerate(zip(lp, fins)):
                ed[ei][4].append((fin, sense, l))
            lps.append((l, fins, lp))
        face_ids.append((f, s, fsense, lps))
    # fins: forward/backward round the loop, other round the edge, vertex the fin's head
    fin_of = {}
    for ei, (e, c, a, b, uses) in enumerate(ed):
        for k, (fin, sense, l) in enumerate(uses):
            head = (b if sense == "+" else a)
            other = uses[(k + 1) % len(uses)][0]
            fin_of[fin] = [l, head, other, e, sense]
    for (f, s, fsense, lps) in face_ids:
        for (l, fins, lp) in lps:
            n = len(fins)
            for k, fin in enumerate(fins):
                l_, head, other, e, sense = fin_of[fin]
                hv = vx[head - 1][0] if head else 0
                m.set(fin, 17, [0, l, fins[(k + 1) % n], fins[(k - 1) % n], hv, other, e, 0, 0, sense])
    for (e, c, a, b, uses) in ed:
        m.set(e, 16, [m.next, 0, None, uses[0][0] if uses[0][1] == "+" else uses[-1][0], 0, 0, c, 0, 0, body])
    first_face = face_ids[0][0]
    for k, (f, s, fsense, lps) in enumerate(face_ids):
        nxt = face_ids[k + 1][0] if k + 1 < len(face_ids) else 0
        prv = face_ids[k - 1][0] if k > 0 else 0
        loop0 = lps[0][0] if lps else 0
        for j, (l, fins, lp) in enumerate(lps):
            m.set(l, 15, [m.next, 0, fins[0], f, lps[j + 1][0] if j + 1 < len(lps) else 0])
        m.set(f, 14, [m.next, 0, None, nxt, prv, loop0, shell, s, fsense, 0, 0, 0, 0, 0])
    m.set(shell, 13, [m.next, 0, body, 0, first_face, 0, 0, solid_r, 0])
    m.set(void, 19, [m.next, 0, body, solid_r, 0, 0, "V"])
    m.set(solid_r, 19, [m.next, 0, body, 0, void, shell, "S"])
    m.set(body, 12, [m.next + 10, 0, 0, 0, 0, 0, 0, 1000.0, 1e-8, 0, 0, 0, 1, owner, 1, 1, shell, 0, 0, 0, void, ed[0][0], vx[0][0] if vx else 0])
    return body


def box(m, a, b, c, owner=0):
    """A block [0,a] x [0,b] x [0,c]."""
    P = [(0, 0, 0), (a, 0, 0), (a, b, 0), (0, b, 0), (0, 0, c), (a, 0, c), (a, b, c), (0, b, c)]
    verts = list(P)
    # faces as vertex cycles, counter-clockwise seen from outside, with an outward normal
    F = [
        ((0, 0, -1), (1, 0, 0), [0, 3, 2, 1]),
        ((0, 0, 1), (1, 0, 0), [4, 5, 6, 7]),
        ((0, -1, 0), (1, 0, 0), [0, 1, 5, 4]),
        ((1, 0, 0), (0, 1, 0), [1, 2, 6, 5]),
        ((0, 1, 0), (-1, 0, 0), [2, 3, 7, 6]),
        ((-1, 0, 0), (0, -1, 0), [3, 0, 4, 7]),
    ]
    emap, edges = {}, []
    faces = []
    for (nrm, xax, cyc) in F:
        loop = []
        for k in range(4):
            i, j = cyc[k], cyc[(k + 1) % 4]
            key = (min(i, j), max(i, j))
            if key not in emap:
                lo, hi = key
                d = [P[hi][q] - P[lo][q] for q in range(3)]
                L = math.sqrt(sum(x * x for x in d))
                emap[key] = len(edges)
                edges.append((30, [P[lo], tuple(x / L for x in d)], lo + 1, hi + 1))
            loop.append((emap[key], "+" if i < j else "-"))
        p0 = P[cyc[0]]
        faces.append((50, [p0, nrm, xax], "+", [loop]))
    return solid(m, faces, edges, verts, owner)


def cylinder(m, r, h, at=(0, 0)):
    """A cylinder of radius r about the z line through `at` (x, y), from 0 to h: two rims with
    no vertex."""
    x, y = at
    edges = [
        (31, [(x, y, 0), (0, 0, 1), (1, 0, 0), r], 0, 0),
        (31, [(x, y, h), (0, 0, 1), (1, 0, 0), r], 0, 0),
    ]
    faces = [
        (51, [(x, y, 0), (0, 0, 1), r, (1, 0, 0)], "+", [[(0, "+")], [(1, "-")]]),
        (50, [(x, y, 0), (0, 0, -1), (1, 0, 0)], "+", [[(0, "-")]]),
        (50, [(x, y, h), (0, 0, 1), (1, 0, 0)], "+", [[(1, "+")]]),
    ]
    return solid(m, faces, edges, [])


def assembly():
    """Two instances of one block: one where it stands, one turned a quarter about z and moved."""
    m = Model()
    asm = m.add(10, None)
    i1, i2, t1, t2 = m.reserve(), m.reserve(), m.reserve(), m.reserve()
    body = box(m, 0.02, 0.01, 0.005, owner=0)
    # p' = R p + t; R turns x to y
    m.set(t1, 100, [m.next, i1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, (0, 0, 0), 1.0, 0, (0, 0, 0)])
    m.set(t2, 100, [m.next, i2, 0, 0, 0, -1, 0, 1, 0, 0, 0, 0, 1, (0.1, 0, 0), 1.0, 3, (0, 0, 0)])
    m.set(i1, 11, [m.next, 0, 1, body, t1, asm, i2, 0, i2, 0])
    m.set(i2, 11, [m.next, 0, 1, body, t2, asm, 0, i1, 0, i1])
    m.set(asm, 10, [m.next + 10, 0, 0, 0, 0, 0, 0, 0, 1000.0, 1e-8, 0, 0, 0, 1, 0, 1, i1])
    return m


def named(m, owner, name):
    """A name attribute on `owner` (a body or an instance), as SolidWorks and Onshape write one:
    SDL/TYSA_NAME with its characters, or SDL/TYSA_UNAME with UTF-16 units when not ASCII."""
    uni = any(ord(c) > 126 for c in name)
    key = "SDL/TYSA_UNAME" if uni else "SDL/TYSA_NAME"
    ident = m.add(79, [list(key)])
    actions = [0] * 8
    legal = ["F"] * 14
    d = m.add(80, [0, ident, 8001 if uni else 8000, actions, 0, legal, [5 if uni else 2]])
    if uni:
        units = list(struct.unpack("<%dH" % (len(name.encode("utf-16-le")) // 2), name.encode("utf-16-le")))
        v = m.add(98, [units])
    else:
        v = m.add(84, [list(name)])
    a = m.add(81, [m.next, d, owner, 0, 0, 0, 0, [v]])
    m.nodes[owner][1][1] = a
    return a


def multipart():
    """A multi-part file, as a Part Studio of several parts exports: a list of bodies at the
    root, each named, one name not ASCII."""
    m = Model()
    lst = m.add(74, None)
    b1 = box(m, 0.03, 0.02, 0.01)
    named(m, b1, "Plate")
    b2 = cylinder(m, 0.01, 0.025, at=(0.05, 0.0))
    named(m, b2, "Stift Ø20")
    m.set(lst, 74, [2, 0, [b1, b2]])
    return m


def nested():
    """An assembly of two instances of a sub-assembly (a block and a pin beside it), the second
    moved 100 mm along y; the top instances named, the parts named."""
    m = Model()
    top = m.add(10, None)
    t1, t2 = m.reserve(), m.reserve()
    sub = m.reserve()
    s1, s2 = m.reserve(), m.reserve()
    st2 = m.reserve()
    blk = box(m, 0.02, 0.01, 0.005)
    named(m, blk, "Block")
    pin = cylinder(m, 0.002, 0.01, at=(0.0, 0.0))
    named(m, pin, "Pin")
    tr1, tr2 = m.reserve(), m.reserve()
    I = [1, 0, 0, 0, 1, 0, 0, 0, 1]
    m.set(tr1, 100, [m.next, t1, 0, 0] + I + [(0, 0, 0), 1.0, 0, (0, 0, 0)])
    m.set(tr2, 100, [m.next, t2, 0, 0] + I + [(0, 0.1, 0), 1.0, 1, (0, 0, 0)])
    m.set(st2, 100, [m.next, s2, 0, 0] + I + [(0.05, 0, 0), 1.0, 1, (0, 0, 0)])
    m.set(t1, 11, [m.next, 0, 1, sub, tr1, top, t2, 0, t2, 0])
    m.set(t2, 11, [m.next, 0, 1, sub, tr2, top, 0, t1, 0, t1])
    m.set(s1, 11, [m.next, 0, 1, blk, 0, sub, s2, 0, 0, 0])
    m.set(s2, 11, [m.next, 0, 1, pin, st2, sub, 0, s1, 0, 0])
    m.set(sub, 10, [m.next + 10, 0, 0, 0, 0, 0, 0, 0, 1000.0, 1e-8, 0, 0, 0, 1, 0, 1, s1])
    m.set(top, 10, [m.next + 10, 0, 0, 0, 0, 0, 0, 0, 1000.0, 1e-8, 0, 0, 0, 1, 0, 1, t1])
    named(m, t1, "Left")
    named(m, t2, "Right")
    return m


def main(out):
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    m = Model()
    box(m, 0.03, 0.02, 0.01)
    (out / "block.x_t").write_text(text(m, False))
    (out / "block_embedded.x_t").write_text(text(m, True))
    (out / "block.x_b").write_bytes(binary(m, False))
    (out / "block_little.x_b").write_bytes(binary(m, True))
    m = Model()
    cylinder(m, 0.01, 0.025)
    (out / "cylinder.x_t").write_text(text(m, True))
    (out / "cylinder.x_b").write_bytes(binary(m, False))
    m = assembly()
    (out / "assembly.x_t").write_text(text(m, True))
    m = multipart()
    (out / "multipart.x_t").write_text(text(m, True))
    (out / "multipart.x_b").write_bytes(binary(m, False))
    m = nested()
    (out / "nested.x_t").write_text(text(m, True))
    (out / "nested.x_b").write_bytes(binary(m, False))


if __name__ == "__main__":
    main(sys.argv[1])
