#!/usr/bin/env python3
"""Random parts that go through the checked operations (chamfer, draft, shell, thicken, offset,
replace), a few with sizes too large for their part, so that a check can refuse them: gen.py DIR SEED0 COUNT."""
import random, sys

NUDGE = [0, 0, 0, 1e-7, 1e-6, 1e-5, 2e-5, 1e-3]


def part(seed):
    r = random.Random(seed)
    nudge = lambda: r.choice(NUDGE) * r.choice((-1, 1))
    L, W, H = r.uniform(30, 90), r.uniform(25, 70), r.uniform(6, 30)
    out = [f"# seed {seed}", f"base: extrude {H:.6g} rect {-L / 2:.6g} {-W / 2:.6g} {L:.6g} {W:.6g}"]
    if r.random() < 0.3:
        out.append(f"draft {r.uniform(-6, 6):.6g} xy at 0 0 0 (faces of base normal {r.choice('+-')}{r.choice('xy')})")
    if r.random() < 0.5:
        out.append(f"fillet {r.uniform(0.5, 4) + nudge():.9g} (edges of base along +z)")
    ops = r.randint(2, 5)
    size = lambda s: (s * r.uniform(2, 40) if r.random() < 0.15 else s * r.uniform(0.2, 1.5)) + nudge()
    for i in range(ops):
        x, y = r.uniform(-L / 3, L / 3), r.uniform(-W / 3, W / 3)
        s = r.uniform(2, 8)
        w = r.choice(["pad", "pocket", "hole", "boss", "chamfer", "chamfer", "offset", "cut"])
        if w == "pad":
            out.append(f"p{i}: extrude cut {r.uniform(0.5, H / 2):.6g} rrect {x - s:.6g} {y - s:.6g} {2 * s:.6g} {2 * s:.6g} {s * r.uniform(0.1, 0.45):.6g}")
            out.append(f"chamfer {size(0.5):.9g} (edges of p{i} concave)")
        elif w == "pocket":
            out.append(f"p{i}: extrude cut {H:.6g} at 0 0 {H / 2:.6g} rect {x - s:.6g} {y - s:.6g} {2 * s:.6g} {2 * s:.6g}")
            out.append(f"chamfer {size(0.4):.9g} (edges of p{i})")
        elif w == "hole":
            out.append(f"hole {s * 0.6:.6g} at {x:.6g} {y:.6g} 0 thru")
        elif w == "boss":
            out.append(f"p{i}: extrude add {r.uniform(3, 12):.6g} at 0 0 {H:.6g} circle {x:.6g} {y:.6g} {s:.6g}")
            out.append(f"chamfer {size(0.5):.9g} (edges of p{i} top)")
        elif w == "chamfer":
            out.append(f"chamfer {size(0.6):.9g} (edges of base top)")
        elif w == "offset":
            out.append(f"offset {size(0.5) * r.choice((-1, 1)):.9g} (faces of base normal {r.choice('+-')}{r.choice('xy')})")
        else:
            out.append(f"p{i}: extrude cut {H:.6g} at 0 0 {H / 2:.6g} rect {x:.6g} {-W:.6g} {r.uniform(0.5, 4):.6g} {2 * W:.6g}")
    f = r.random()
    if f < 0.3:
        out.append(f"shell {size(1):.9g} (faces plane top)")
    elif f < 0.5:
        out.append(f"thicken {size(1):.9g} (faces of base normal {r.choice('+-')}{r.choice('xy')})")
    return "\n".join(out) + "\n"


d, s0, n = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
for s in range(s0, s0 + n):
    open(f"{d}/p{s:05d}.fc", "w").write(part(s))
