#!/usr/bin/env python3
"""Random parts whose fillets have traced spines, cut through: gen.py DIR SEED0 COUNT. A round or
square body with an oblique boss, the concave edge filleted, a second fillet where it applies, then
holes, slots and flats through the fillet, many of them lying just along a contact or at a flat box."""
import math, random, sys


def unit(v):
    n = math.sqrt(sum(c * c for c in v))
    return tuple(c / n for c in v)


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def square(z):
    h = (1.0, 0.0, 0.0) if abs(z[0]) < 0.9 else (0.0, 1.0, 0.0)
    return unit(cross(z, h))


def fr(z, x, at):
    return f"z {z[0]:.12g} {z[1]:.12g} {z[2]:.12g} x {x[0]:.12g} {x[1]:.12g} {x[2]:.12g} at {at[0]:.12g} {at[1]:.12g} {at[2]:.12g}"


NUDGE = [0, 0, 0, 0, 1e-7, 1e-6, 1e-5, 2e-5, 1e-3]


def part(seed):
    r = random.Random(seed)
    nudge = lambda: r.choice(NUDGE) * r.choice((-1, 1))
    R = r.uniform(8, 25)
    L = r.uniform(30, 80)
    out = [f"# seed {seed}"]
    if r.random() < 0.7:
        out.append(f"body: extrude {L:.6g} {fr((0, 0, 1), (1, 0, 0), (0, 0, 0))} circle 0 0 {R:.6g}")
    else:
        out.append(f"body: extrude {L:.6g} {fr((0, 0, 1), (1, 0, 0), (0, 0, 0))} rect {-R:.6g} {-R:.6g} {2 * R:.6g} {2 * R:.6g}")
    # the boss leaves the body's side, leaning and offset, so the edge is no circle
    th = r.uniform(0, 6.28)
    lean = r.uniform(-0.7, 0.7)
    z = unit((math.cos(th), math.sin(th), lean))
    x = square(z)
    rb = R * r.uniform(0.2, 0.75)
    h = r.uniform(0.3, 0.7) * L
    off = (R - rb) * r.uniform(-0.6, 0.6)
    y = cross(z, x)
    at = tuple((0, 0, h)[k] + y[k] * off for k in range(3))
    len_b = R + r.uniform(5, 30)
    out.append(f"boss: extrude add {len_b:.6g} {fr(z, x, at)} circle 0 0 {rb:.6g}")
    rf = rb * r.uniform(0.08, 0.4) + nudge()
    out.append(f"fillet {rf:.9g} (edges between body boss concave)")
    if r.random() < 0.3:
        out.append(f"bore: extrude cut {len_b + R:.6g} {fr(z, x, at)} circle 0 0 {rb * r.uniform(0.3, 0.7):.6g}")
    # where the fillet is, roughly: where the boss's side meets the body's
    foot = tuple(at[k] + z[k] * (R - 0.2 * rb) + (x[k] * math.cos(a) + y[k] * math.sin(a)) * (rb + rf)
                 for a in [r.uniform(0, 6.28)] for k in range(3))
    for i in range(r.choice([1, 1, 2, 3])):
        w = r.choice(["hole", "hole", "oblique", "flat", "slot", "along"])
        a = r.uniform(0, 6.28)
        n = unit((math.cos(a), math.sin(a), r.uniform(-0.8, 0.8)))
        if w == "hole":
            n = z
        if w == "along":
            n = y
        q = square(n)
        d = 3 * (R + rb)
        o = tuple(foot[k] - n[k] * d / 2 for k in range(3))
        s = rf * r.uniform(0.3, 3) + nudge()
        if w in ("hole", "oblique", "along"):
            out.append(f"c{i}: extrude cut {d:.6g} {fr(n, q, o)} circle {r.uniform(-1, 1) * rf:.6g} {r.uniform(-1, 1) * rf:.6g} {s:.9g}")
        elif w == "slot":
            out.append(f"c{i}: extrude cut {d:.6g} {fr(n, q, o)} rect {-s:.9g} {-2 * s:.6g} {2 * s:.9g} {4 * s:.6g}")
        else:
            # a flat through the foot: the plane stands where the region's box goes flat
            out.append(f"c{i}: extrude cut {d / 2:.6g} {fr(n, q, foot)} rect {-d:.6g} {-d:.6g} {2 * d:.6g} {2 * d:.6g}")
    return "\n".join(out) + "\n"


d, s0, n = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
for s in range(s0, s0 + n):
    open(f"{d}/p{s:05d}.fc", "w").write(part(s))
