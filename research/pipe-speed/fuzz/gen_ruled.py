#!/usr/bin/env python3
"""Random ruled lofts cut by holes, oblique cylinders, flats and small posts, some just catching a corner so the
curve on the wall is short: gen_ruled.py DIR SEED0 COUNT. Each a .fg part (fgsrc frames, gen.py's extrude)."""
import math, os, random, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../tools/find-parts"))
from fgsrc import Frame, Part, loops
from gen import extrude


def unit(v):
    n = math.sqrt(sum(c * c for c in v))
    return tuple(c / n for c in v)


def square(z):
    h = (1.0, 0.0, 0.0) if abs(z[0]) < 0.9 else (0.0, 1.0, 0.0)
    x = (z[1] * h[2] - z[2] * h[1], z[2] * h[0] - z[0] * h[2], z[0] * h[1] - z[1] * h[0])
    return unit(x)


def section(r, kind, w, h):
    f6 = lambda v: float(f"{v:.6g}")
    if kind == "circle":
        return ("circle", 0, 0, f6(w))
    rad = min(w, h) * r.uniform(0.1, 0.6)
    return ("rrect", f6(-w), f6(-h), f6(2 * w), f6(2 * h), f6(rad))


def part(seed):
    r = random.Random(seed)
    f6 = lambda v: float(f"{v:.6g}")
    kind = r.choice(["circle", "rrect", "rrect"])
    n = r.choice([2, 2, 3])
    L = r.uniform(30, 120)
    w, h = r.uniform(8, 25), r.uniform(8, 25)
    P = Part()
    P.add(f"# seed {seed}")
    sks = []
    for i in range(n):
        z = L * i / (n - 1)
        f = r.uniform(0.5, 1.4)
        at = (f6(r.uniform(-3, 3)), f6(r.uniform(-3, 3)), f6(z))
        sks.append(P.sketch(Frame("xy", at), loops(section(r, kind, w * f, h * f))))
    P.add(f"arm = loft([{', '.join(sks)}])")
    for i in range(r.choice([1, 2, 3])):
        what = r.choice(["hole", "oblique", "flat", "post", "corner", "cross"])
        a = r.uniform(0, 6.28)
        at = (0.0, 0.0, r.uniform(0.05, 0.95) * L)
        d = 4 * (w + h) + L
        if what == "hole":
            n_ = (0.0, 0.0, 1.0)
            o = (r.uniform(-w, w), r.uniform(-h, h), -5.0)
            extrude(P, f"c{i}", "cut", f6(L + 10), n_, (1, 0, 0), o, ("circle", 0, 0, f6(r.uniform(0.5, 4))))
        elif what in ("oblique", "cross"):
            n_ = unit((math.cos(a), math.sin(a), r.uniform(-0.8, 0.8) if what == "oblique" else 0.0))
            o = tuple(at[k] - n_[k] * d / 2 for k in range(3))
            extrude(P, f"c{i}", "cut", f6(d), n_, square(n_), o, ("circle", 0, 0, f6(r.uniform(0.5, 6))))
        elif what == "flat":
            n_ = unit((math.cos(a), math.sin(a), r.uniform(-0.4, 0.4)))
            o = tuple(at[k] + n_[k] * r.uniform(0.5, 1.0) * min(w, h) for k in range(3))
            extrude(P, f"c{i}", "cut", f6(d), n_, square(n_), o, ("rect", f6(-d), f6(-d), f6(2 * d), f6(2 * d)))
        else:
            # a cylinder whose side just reaches the wall, so the curve there is short
            rad = r.uniform(0.3, 3.0)
            reach = (w * 1.0 + rad * r.uniform(0.7, 1.1)) if what == "post" else (math.hypot(w, h) * 0.75 + rad * r.uniform(0.5, 1.2))
            o = (reach * math.cos(a), reach * math.sin(a), -5.0)
            n_ = (0.0, 0.0, 1.0)
            extrude(P, f"c{i}", r.choice(["cut", "add"]), f6(L + 10), n_, (1, 0, 0), o, ("circle", 0, 0, f6(rad)))
    return P.text()


if __name__ == "__main__":
    d, s0, n = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
    for s in range(s0, s0 + n):
        open(f"{d}/p{s:05d}.fg", "w").write(part(s))
