#!/usr/bin/env python3
"""Random parts whose fillets have traced spines, cut through: gen.py DIR SEED0 COUNT. A round or
square body with an oblique boss, the concave edge filleted, a second fillet where it applies, then
holes, slots and flats through the fillet, many of them lying just along a contact or at a flat box. Each a
.fg part, its frames written as tools/find-parts/fgsrc writes them."""
import math, os, random, sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../tools/find-parts"))
from fgsrc import Part, loops, num


def unit(v):
    n = math.sqrt(sum(c * c for c in v))
    return tuple(c / n for c in v)


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def square(z):
    h = (1.0, 0.0, 0.0) if abs(z[0]) < 0.9 else (0.0, 1.0, 0.0)
    return unit(cross(z, h))


def g(v):
    return float(f"{v:.12g}")


def extrude(P, name, op, d, z, x, at, loop):
    """`extrude OP D` on the frame (normal z, x along x, origin at): a cut runs along z too."""
    pl = P.plane(tuple(map(g, at)), tuple(map(g, z)), tuple(map(g, x)))
    sk = P.sketch_on(pl, loops(loop))
    call = {"add": "extrude", "cut": "cut"}[op]
    P.add(f"{name} = {call}({sk}, {num(-d if op == 'cut' else d)})")


NUDGE = [0, 0, 0, 0, 1e-7, 1e-6, 1e-5, 2e-5, 1e-3]


def part(seed):
    r = random.Random(seed)
    nudge = lambda: r.choice(NUDGE) * r.choice((-1, 1))
    f6 = lambda v: float(f"{v:.6g}")
    f9 = lambda v: float(f"{v:.9g}")
    R = r.uniform(8, 25)
    L = r.uniform(30, 80)
    P = Part()
    P.add(f"# seed {seed}")
    Z, X, O = (0, 0, 1), (1, 0, 0), (0, 0, 0)
    if r.random() < 0.7:
        extrude(P, "body", "add", f6(L), Z, X, O, ("circle", 0, 0, f6(R)))
    else:
        extrude(P, "body", "add", f6(L), Z, X, O, ("rect", f6(-R), f6(-R), f6(2 * R), f6(2 * R)))
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
    extrude(P, "boss", "add", f6(len_b), z, x, at, ("circle", 0, 0, f6(rb)))
    rf = rb * r.uniform(0.08, 0.4) + nudge()
    P.add(f'round = fillet(edges(between: (body, boss), crease: "concave"), {num(f9(rf))})')
    if r.random() < 0.3:
        extrude(P, "bore", "cut", f6(len_b + R), z, x, at, ("circle", 0, 0, f6(rb * r.uniform(0.3, 0.7))))
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
            cx = f6(r.uniform(-1, 1) * rf)
            cy = f6(r.uniform(-1, 1) * rf)
            extrude(P, f"c{i}", "cut", f6(d), n, q, o, ("circle", cx, cy, f9(s)))
        elif w == "slot":
            extrude(P, f"c{i}", "cut", f6(d), n, q, o, ("rect", f9(-s), f6(-2 * s), f9(2 * s), f6(4 * s)))
        else:
            # a flat through the foot: the plane stands where the region's box goes flat
            extrude(P, f"c{i}", "cut", f6(d / 2), n, q, foot, ("rect", f6(-d), f6(-d), f6(2 * d), f6(2 * d)))
    return P.text()


if __name__ == "__main__":
    d, s0, n = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
    for s in range(s0, s0 + n):
        open(f"{d}/p{s:05d}.fg", "w").write(part(s))
