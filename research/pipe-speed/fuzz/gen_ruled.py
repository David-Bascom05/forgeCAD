#!/usr/bin/env python3
"""Random ruled lofts cut by holes, oblique cylinders, flats and small posts, some just catching a corner so the
curve on the wall is short: gen_ruled.py DIR SEED0 COUNT."""
import math, random, sys


def unit(v):
    n = math.sqrt(sum(c * c for c in v))
    return tuple(c / n for c in v)


def square(z):
    h = (1.0, 0.0, 0.0) if abs(z[0]) < 0.9 else (0.0, 1.0, 0.0)
    x = (z[1] * h[2] - z[2] * h[1], z[2] * h[0] - z[0] * h[2], z[0] * h[1] - z[1] * h[0])
    return unit(x)


def fr(z, x, at):
    return f"z {z[0]:.12g} {z[1]:.12g} {z[2]:.12g} x {x[0]:.12g} {x[1]:.12g} {x[2]:.12g} at {at[0]:.12g} {at[1]:.12g} {at[2]:.12g}"


def section(r, kind, w, h):
    if kind == "circle":
        return f"circle 0 0 {w:.6g}"
    rad = min(w, h) * r.uniform(0.1, 0.6)
    return f"rrect {-w:.6g} {-h:.6g} {2 * w:.6g} {2 * h:.6g} {rad:.6g}"


def part(seed):
    r = random.Random(seed)
    kind = r.choice(["circle", "rrect", "rrect"])
    n = r.choice([2, 2, 3])
    L = r.uniform(30, 120)
    w, h = r.uniform(8, 25), r.uniform(8, 25)
    profs = []
    for i in range(n):
        z = L * i / (n - 1)
        f = r.uniform(0.5, 1.4)
        profs.append(f"profile at {r.uniform(-3, 3):.6g} {r.uniform(-3, 3):.6g} {z:.6g} " + section(r, kind, w * f, h * f))
    out = [f"# seed {seed}", "arm: loft " + " ".join(profs)]
    for i in range(r.choice([1, 2, 3])):
        what = r.choice(["hole", "oblique", "flat", "post", "corner", "cross"])
        a = r.uniform(0, 6.28)
        at = (0.0, 0.0, r.uniform(0.05, 0.95) * L)
        d = 4 * (w + h) + L
        if what == "hole":
            n_ = (0.0, 0.0, 1.0)
            o = (r.uniform(-w, w), r.uniform(-h, h), -5.0)
            out.append(f"c{i}: extrude cut {L + 10:.6g} {fr(n_, (1, 0, 0), o)} circle 0 0 {r.uniform(0.5, 4):.6g}")
        elif what in ("oblique", "cross"):
            n_ = unit((math.cos(a), math.sin(a), r.uniform(-0.8, 0.8) if what == "oblique" else 0.0))
            o = tuple(at[k] - n_[k] * d / 2 for k in range(3))
            out.append(f"c{i}: extrude cut {d:.6g} {fr(n_, square(n_), o)} circle 0 0 {r.uniform(0.5, 6):.6g}")
        elif what == "flat":
            n_ = unit((math.cos(a), math.sin(a), r.uniform(-0.4, 0.4)))
            o = tuple(at[k] + n_[k] * r.uniform(0.5, 1.0) * min(w, h) for k in range(3))
            out.append(f"c{i}: extrude cut {d:.6g} {fr(n_, square(n_), o)} rect {-d:.6g} {-d:.6g} {2 * d:.6g} {2 * d:.6g}")
        else:
            # a cylinder whose side just reaches the wall, so the curve there is short
            rad = r.uniform(0.3, 3.0)
            reach = (w * 1.0 + rad * r.uniform(0.7, 1.1)) if what == "post" else (math.hypot(w, h) * 0.75 + rad * r.uniform(0.5, 1.2))
            o = (reach * math.cos(a), reach * math.sin(a), -5.0)
            n_ = (0.0, 0.0, 1.0)
            out.append(f"c{i}: extrude {r.choice(['cut', 'add'])} {L + 10:.6g} {fr(n_, (1, 0, 0), o)} circle 0 0 {rad:.6g}")
    return "\n".join(out) + "\n"


d, s0, n = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
for s in range(s0, s0 + n):
    open(f"{d}/p{s:05d}.fc", "w").write(part(s))
