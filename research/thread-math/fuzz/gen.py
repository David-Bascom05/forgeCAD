#!/usr/bin/env python3
"""Random threaded parts as .fg part files: gen.py DIR SEED0 COUNT. Thread size, pitch, length, hand,
chamfer, run-out; coils with taper and several starts; cross holes, flats and keyways before and
after; oblique axes; lengths and diameters nudged around a tolerance. Frames are written as
tools/find-parts/fgsrc writes them: a plane through helper points."""
import math, os, random, sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../tools/find-parts"))
from fgsrc import Part, line, loops, num

COARSE = [(3, .5), (4, .7), (5, .8), (6, 1), (8, 1.25), (10, 1.5), (12, 1.75), (16, 2), (20, 2.5), (24, 3)]
FINE = [(8, 1), (10, 1.25), (12, 1), (16, 1.5), (20, 1), (24, 2), (20, .5), (30, 1.5)]
NUDGE = [0, 0, 0, 1e-7, 1e-6, 5e-6, 1e-5, 1.5e-5, 2e-5, 1e-4, 1e-3]


def g12(v):
    return float(f"{v:.12g}")


def plane(P, z, x, at):
    """The plane of the frame (normal z, x along x, origin at), as the retired frames printed it."""
    return P.plane(tuple(map(g12, at)), tuple(map(g12, z)), tuple(map(g12, x)))


def g6(v):
    return float(f"{v:.6g}")


def g9(v):
    return float(f"{v:.9g}")


def axes(r):
    if r.random() < 0.7:
        return (0.0, 0.0, 1.0), (1.0, 0.0, 0.0)
    th = r.uniform(0.1, 1.2)
    ph = r.uniform(0, 6.28)
    z = (math.sin(th) * math.cos(ph), math.sin(th) * math.sin(ph), math.cos(th))
    x = (math.cos(th) * math.cos(ph), math.cos(th) * math.sin(ph), -math.sin(th))
    return z, x


def neg(v):
    return tuple(-c for c in v)


def along(o, z, t):
    return tuple(o[i] + z[i] * t for i in range(3))


def part(seed):
    r = random.Random(seed)
    d, p = r.choice(COARSE + FINE)
    z, x = axes(r)
    y = (z[1] * x[2] - z[2] * x[1], z[2] * x[0] - z[0] * x[2], z[0] * x[1] - z[1] * x[0])
    o = (0.0, 0.0, 0.0)
    nudge = lambda: r.choice(NUDGE) * r.choice((-1, 1))
    L = round(r.uniform(4 * p, 25 * p) + 2, 2)
    length = round(r.uniform(2 * p, L - p), 3) + nudge()
    opts = ""
    if r.random() < 0.3:
        opts += ", left: true"
    if r.random() < 0.5:
        opts += ", chamfer: true"
    runout = r.random() < 0.2
    if runout:
        opts += ", runout: true"
    H = 5 / 16 * math.sqrt(3) * p
    kind = r.choice(["ext", "ext", "int", "int", "coil"])
    P = Part()
    P.add(f"# seed {seed} {kind} {d}x{p}")

    def cut(name, ext, fz, fx, at, loop):
        sk = P.sketch_on(plane(P, fz, fx, at), loops(loop))
        return f"{name} = cut({sk}, {ext})"

    # things crossing the thread: a hole square to it, an oblique hole, a flat, a keyway
    def crossing(i):
        w = r.choice(["hole", "oblique", "flat", "key", "axial"])
        h = r.uniform(0.0, L)
        at = along(o, z, h)
        rad = r.uniform(0.1, 0.4) * d / 2
        if w == "hole":
            off = r.choice([0.0, 0.0, r.uniform(-.3, .3) * d])
            return lambda: cut(f"cross{i}", f"sym({num(g6(4 * d))})", x, y, tuple(at[k] + y[k] * off for k in range(3)), ("circle", 0, 0, g6(rad)))
        if w == "oblique":
            c, s = math.cos(.7), math.sin(.7)
            a = tuple(x[k] * c + z[k] * s for k in range(3))
            b = tuple(-x[k] * s + z[k] * c for k in range(3))
            return lambda: cut(f"cross{i}", f"sym({num(g6(4 * d))})", a, b, at, ("circle", 0, 0, g6(rad)))
        if w == "flat":
            f = along(tuple(o[k] + x[k] * (d / 2 - r.uniform(0.3, 0.7) * H) for k in range(3)), z, h)
            return lambda: cut(f"cross{i}", "-2", y, z, f, ("rect", g6(-d), g6(-L / 8), g6(2 * d), g6(L / 4)))
        if w == "key":
            f = tuple(at[k] + y[k] * (-2 * d) for k in range(3))
            return lambda: cut(f"cross{i}", num(-g6(4 * d)), y, z, f, ("rect", g6(d / 2 - 2 * H), g6(-L / 10), g6(d), g6(L / 5)))
        cx = g9(d / 2 + nudge())
        return lambda: cut(f"cross{i}", num(-g6(L + 2)), z, x, along(o, z, -1), ("circle", cx, 0, g6(rad / 3)))

    before = [crossing(i) for i in range(r.choice([0, 0, 0, 1, 2]))]
    after = [crossing(10 + i) for i in range(r.choice([0, 0, 0, 1, 2]))]
    top = along(o, z, L)

    def solid(name, dist, fz, fx, at, loop, op=""):
        sk = P.sketch_on(plane(P, fz, fx, at), loops(loop))
        P.add(f"{name} = extrude({sk}, {num(dist)}{op})")

    if kind == "ext":
        solid("rod", g6(L), z, x, o, ("circle", 0, 0, g9(d / 2 + nudge())))
        if r.random() < 0.3:
            solid("shoulder", g6(2 + p), neg(z), x, o, ("circle", 0, 0, g6(d / 2 + 3 * p)))
        for c in before:
            P.add(c())
        # the thread from the rod's top end, into it
        P.add(f'th = thread(on: {plane(P, z, x, top)}, spec: "M{d}x{p}", length: {num(g9(length))}{opts})')
    elif kind == "int":
        w = d + 6 * p
        solid("blk", g6(L), z, x, o, ("rect", g6(-w), g6(-w), g6(2 * w), g6(2 * w)))
        for c in before:
            P.add(c())
        tap = d - 2 * H + nudge()
        # drilled from the top down: against the plane's normal
        hs = P.sketch_on(plane(P, z, x, top), [lambda q, s: "m = pt(0, 0)"])
        if r.random() < 0.3:
            P.add(f"h = hole(at: {hs}.m, d: {num(g9(tap))}, depth: {num(g9(length + p))}, thread: ({d}, {p}))")
            P.add(f"th = thread(hole: h, length: {num(g9(length))}{opts})")
        else:
            P.add(f"h = hole(at: {hs}.m, d: {num(g9(tap))}, depth: thru, thread: ({d}, {p}))")
            P.add(f'th = thread(hole: h, length: "full"{opts})')
    else:
        pitch = r.choice([2 * p, 3 * p, 4 * p])
        starts = r.choice([1, 2, 2, 3, 4]) if pitch >= 2 * p else 1
        rr = d / 2
        solid("rod", g6(L + 2 * pitch), z, x, o, ("circle", 0, 0, g6(rr)))
        for c in before:
            P.add(c())
        turns = round(r.uniform(1.2, (L + pitch) / pitch), 3)
        taper = r.choice([0, 0, 0, 0.02, -0.02, 0.05])
        hand = ", left: true" if r.random() < 0.3 else ""
        w = 0.8 * p / starts
        for s in range(starts):
            h0 = -pitch / 2 + s * pitch / starts
            prof = [(g6(rr - H), g6(-w / 3)), (g6(rr + 1), g6(-w)), (g6(rr + 1), g6(w)), (g6(rr - H), g6(w / 3))]
            tp = f", taper: {taper}deg" if taper else ""
            # the section in the plane through the axis: x out, y along the axis
            at = along(o, z, h0)
            sk = P.sketch_on(plane(P, neg(y), x, at), loops(("poly", prof)) + [line(0, 0, 0, 1, nm="ax", construction=True)])
            P.add(f"c{s} = coil({sk}.o, axis: {sk}.ax, pitch: {num(g6(pitch))}, turns: {turns}{tp}{hand}, cut: true)")
    for c in after:
        P.add(c())
    return P.text()


if __name__ == "__main__":
    d, s0, n = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
    for s in range(s0, s0 + n):
        open(f"{d}/t{s:05d}.fg", "w").write(part(s))
