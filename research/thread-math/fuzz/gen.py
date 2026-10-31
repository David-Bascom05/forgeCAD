#!/usr/bin/env python3
"""Random threaded parts as part files: gen.py DIR SEED0 COUNT. Thread size, pitch, length, hand,
chamfer, run-out; coils with taper and several starts; cross holes, flats and keyways before and
after; oblique axes; lengths and diameters nudged around a tolerance."""
import math, random, sys

COARSE = [(3, .5), (4, .7), (5, .8), (6, 1), (8, 1.25), (10, 1.5), (12, 1.75), (16, 2), (20, 2.5), (24, 3)]
FINE = [(8, 1), (10, 1.25), (12, 1), (16, 1.5), (20, 1), (24, 2), (20, .5), (30, 1.5)]
NUDGE = [0, 0, 0, 1e-7, 1e-6, 5e-6, 1e-5, 1.5e-5, 2e-5, 1e-4, 1e-3]


def fr(z, x, at):
    return f"z {z[0]:.12g} {z[1]:.12g} {z[2]:.12g} x {x[0]:.12g} {x[1]:.12g} {x[2]:.12g} at {at[0]:.12g} {at[1]:.12g} {at[2]:.12g}"


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
        opts += " left"
    if r.random() < 0.5:
        opts += " chamfer"
    runout = r.random() < 0.2
    if runout:
        opts += " runout"
    H = 5 / 16 * math.sqrt(3) * p
    kind = r.choice(["ext", "ext", "int", "int", "coil"])
    out = [f"# seed {seed} {kind} {d}x{p}"]
    # things crossing the thread: a hole square to it, an oblique hole, a flat, a keyway
    def crossing(i):
        w = r.choice(["hole", "oblique", "flat", "key", "axial"])
        h = r.uniform(0.0, L)
        at = along(o, z, h)
        rad = r.uniform(0.1, 0.4) * d / 2
        if w == "hole":
            off = r.choice([0.0, 0.0, r.uniform(-.3, .3) * d])
            return f"cross{i}: extrude cut sym {4 * d:.6g} {fr(x, y, tuple(at[k] + y[k] * off for k in range(3)))} circle 0 0 {rad:.6g}"
        if w == "oblique":
            c, s = math.cos(.7), math.sin(.7)
            a = tuple(x[k] * c + z[k] * s for k in range(3))
            b = tuple(-x[k] * s + z[k] * c for k in range(3))
            return f"cross{i}: extrude cut sym {4 * d:.6g} {fr(a, b, at)} circle 0 0 {rad:.6g}"
        if w == "flat":
            f = along(tuple(o[k] + x[k] * (d / 2 - r.uniform(0.3, 0.7) * H) for k in range(3)), z, h)
            return f"cross{i}: extrude cut 2 {fr(y, z, f)} rect {-d:.6g} {-L / 8:.6g} {2 * d:.6g} {L / 4:.6g}"
        if w == "key":
            f = tuple(at[k] + y[k] * (-2 * d) for k in range(3))
            return f"cross{i}: extrude cut {4 * d:.6g} {fr(y, z, f)} rect {d / 2 - 2 * H:.6g} {-L / 10:.6g} {d:.6g} {L / 5:.6g}"
        return f"cross{i}: extrude cut {L + 2:.6g} {fr(z, x, along(o, z, -1))} circle {d / 2 + nudge():.9g} 0 {rad / 3:.6g}"

    before = [crossing(i) for i in range(r.choice([0, 0, 0, 1, 2]))]
    after = [crossing(10 + i) for i in range(r.choice([0, 0, 0, 1, 2]))]
    top = along(o, z, L)
    if kind == "ext":
        out.append(f"rod: extrude {L:.6g} {fr(z, x, o)} circle 0 0 {d / 2 + nudge():.9g}")
        if r.random() < 0.3:
            out.append(f"shoulder: extrude add {2 + p:.6g} {fr(neg(z), x, o)} circle 0 0 {d / 2 + 3 * p:.6g}")
        out += before
        out.append(f"thread M{d}x{p} {fr(neg(z), x, top)} ext {length:.9g}{opts}")
    elif kind == "int":
        w = d + 6 * p
        out.append(f"blk: extrude {L:.6g} {fr(z, x, o)} rect {-w:.6g} {-w:.6g} {2 * w:.6g} {2 * w:.6g}")
        out += before
        tap = d - 2 * H + nudge()
        if r.random() < 0.3:
            out.append(f"h: hole {tap:.9g} {fr(neg(z), x, top)} {length + p:.9g} thread {d} {p}")
            out.append(f"thread hole h {length:.9g}{opts}")
        else:
            out.append(f"h: hole {tap:.9g} {fr(neg(z), x, top)} thru thread {d} {p}")
            out.append(f"thread hole h full{opts}")
    else:
        pitch = r.choice([2 * p, 3 * p, 4 * p])
        starts = r.choice([1, 2, 2, 3, 4]) if pitch >= 2 * p else 1
        rr = d / 2
        out.append(f"rod: extrude {L + 2 * pitch:.6g} {fr(z, x, o)} circle 0 0 {rr:.6g}")
        out += before
        turns = round(r.uniform(1.2, (L + pitch) / pitch), 3)
        taper = r.choice([0, 0, 0, 0.02, -0.02, 0.05])
        hand = " left" if r.random() < 0.3 else ""
        w = 0.8 * p / starts
        for s in range(starts):
            h0 = -pitch / 2 + s * pitch / starts
            prof = f"poly {rr - H:.6g} {-w / 3:.6g} {rr + 1:.6g} {-w:.6g} {rr + 1:.6g} {w:.6g} {rr - H:.6g} {w / 3:.6g}"
            tp = f" taper {taper}" if taper else ""
            # the section in the plane through the axis: x out, y along the axis
            at = along(o, z, h0)
            out.append(f"c{s}: coil cut {fr(neg(y), x, at)} {prof} pitch {pitch:.6g} turns {turns}{tp}{hand}")
    out += after
    return "\n".join(out) + "\n"


d, s0, n = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
for s in range(s0, s0 + n):
    open(f"{d}/t{s:05d}.fc", "w").write(part(s))
