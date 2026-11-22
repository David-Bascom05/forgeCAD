#!/usr/bin/env python3
"""Random parts that go through the checked operations (chamfer, draft, shell, thicken, offset,
replace), a few with sizes too large for their part, so that a check can refuse them: gen.py DIR SEED0 COUNT.
Each a .fg part, its placements written as tools/find-parts/fgsrc writes them."""
import os, random, sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../tools/find-parts"))
from fgsrc import Frame, Part, extrude, hole, loops, num

NUDGE = [0, 0, 0, 1e-7, 1e-6, 1e-5, 2e-5, 1e-3]
DIRS = {"+x": "X", "-x": "-X", "+y": "Y", "-y": "-Y"}


def part(seed):
    r = random.Random(seed)
    nudge = lambda: r.choice(NUDGE) * r.choice((-1, 1))
    L, W, H = r.uniform(30, 90), r.uniform(25, 70), r.uniform(6, 30)
    g = lambda v: float(f"{v:.6g}")
    P = Part()
    P.add(f"# seed {seed}")
    extrude(P, "add", g(H), Frame(), loops(("rect", g(-L / 2), g(-W / 2), g(L), g(W))), label="base")
    side = lambda: DIRS[r.choice("+-") + r.choice("xy")]
    if r.random() < 0.3:
        a = num(g(r.uniform(-6, 6)))
        P.add(f"{P.name('dr')} = draft(faces(of: base, normal: {side()}), angle: {a}deg, neutral: xy)")
    if r.random() < 0.5:
        P.add(f"{P.name('fi')} = fillet(edges(of: base, along: Z), {num(float(f'{r.uniform(0.5, 4) + nudge():.9g}'))})")
    ops = r.randint(2, 5)
    size = lambda s: num(float(f"{(s * r.uniform(2, 40) if r.random() < 0.15 else s * r.uniform(0.2, 1.5)) + nudge():.9g}"))
    for i in range(ops):
        x, y = r.uniform(-L / 3, L / 3), r.uniform(-W / 3, W / 3)
        s = r.uniform(2, 8)
        w = r.choice(["pad", "pocket", "hole", "boss", "chamfer", "chamfer", "offset", "cut"])
        if w == "pad":
            d = g(r.uniform(0.5, H / 2))
            extrude(P, "cut", d, Frame(), loops(("rrect", g(x - s), g(y - s), g(2 * s), g(2 * s), g(s * r.uniform(0.1, 0.45)))), label=f"p{i}")
            P.add(f"{P.name('ch')} = chamfer(edges(of: p{i}, crease: \"concave\"), {size(0.5)})")
        elif w == "pocket":
            extrude(P, "cut", g(H), Frame("xy", (0, 0, g(H / 2))), loops(("rect", g(x - s), g(y - s), g(2 * s), g(2 * s))), label=f"p{i}")
            P.add(f"{P.name('ch')} = chamfer(edges(of: p{i}), {size(0.4)})")
        elif w == "hole":
            hole(P, g(s * 0.6), Frame("xy", (g(x), g(y), 0)), "thru")
        elif w == "boss":
            d = g(r.uniform(3, 12))
            extrude(P, "add", d, Frame("xy", (0, 0, g(H))), loops(("circle", g(x), g(y), g(s))), label=f"p{i}")
            P.add(f"{P.name('ch')} = chamfer(edges(of: p{i}, extreme: Z), {size(0.5)})")
        elif w == "chamfer":
            P.add(f"{P.name('ch')} = chamfer(edges(of: base, extreme: Z), {size(0.6)})")
        elif w == "offset":
            d = size(0.5)
            sign = r.choice((-1, 1))
            P.add(f"{P.name('of')} = offset_faces(faces(of: base, normal: {side()}), {num(float(d) * sign)})")
        else:
            extrude(P, "cut", g(H), Frame("xy", (0, 0, g(H / 2))), loops(("rect", g(x), g(-W), g(r.uniform(0.5, 4)), g(2 * W))), label=f"p{i}")
    f = r.random()
    if f < 0.3:
        P.add(f"{P.name('sh')} = shell({size(1)}, open: faces(shape: \"plane\", extreme: Z))")
    elif f < 0.5:
        t = size(1)
        P.add(f"{P.name('th')} = thicken(faces(of: base, normal: {side()}), {t})")
    return P.text()


if __name__ == "__main__":
    d, s0, n = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
    for s in range(s0, s0 + n):
        open(f"{d}/p{s:05d}.fg", "w").write(part(s))
