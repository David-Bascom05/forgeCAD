#!/usr/bin/env python3
"""A third opinion when ForgeCAD and OCCT disagree: the part's volume by Monte Carlo, classifying
random points straight against the history's analytic tools (no B-rep at all). Whole circles are
tested exactly; arcs are walked as 2048-gons (sag under 1.2e-6 of the radius).

  tools/occt/montecarlo.py part.json [--samples 2e7]    (a part as the oracle example exports it)
"""
import argparse
import json
import math
import os
import sys

os.environ.setdefault("OPENBLAS_NUM_THREADS", "1")
os.environ.setdefault("OMP_NUM_THREADS", "1")
import numpy as np  # noqa: E402


def loops2d(f):
    """The profile's loops as closed polylines (n x 2 arrays), or ("circle", cx, cy, r)."""
    out = []
    for lp in f["loops"]:
        if len(lp) == 1 and "circle" in lp[0]:
            out.append(("circle", *lp[0]["circle"][:3]))
            continue
        pts = []
        for s in lp:
            if "line" in s:
                a, b, _, _ = s["line"]
                pts.append((a, b))
            elif "arc" in s:
                cx, cy, r, st, sw = s["arc"]
                n = max(4, int(abs(sw) / (2 * math.pi) * 2048))
                for i in range(n):
                    t = st + sw * i / n
                    pts.append((cx + r * math.cos(t), cy + r * math.sin(t)))
        out.append(np.array(pts))
    return out


def points(loops):
    """Points on the loops (circles by eight points round them) for boxes and sides."""
    pts = []
    for p in loops:
        if isinstance(p, tuple):
            _, cx, cy, r = p
            pts += [(cx + r * math.cos(k * math.pi / 4), cy + r * math.sin(k * math.pi / 4)) for k in range(8)]
        else:
            pts += list(map(tuple, p))
    return np.array(pts)


def in_profile(loops, u, v):
    """Even-odd rule over every loop (holes included)."""
    inside = np.zeros(u.shape, bool)
    for p in loops:
        if isinstance(p, tuple):
            _, cx, cy, r = p
            inside ^= (u - cx) ** 2 + (v - cy) ** 2 < r * r
            continue
        q = np.roll(p, -1, axis=0)
        for (x1, y1), (x2, y2) in zip(p, q):
            if y1 == y2:
                continue
            c = (y1 > v) != (y2 > v)
            x = x1 + (v - y1) * (x2 - x1) / (y2 - y1)
            inside ^= c & (u < x)
    return inside


def member(f, P):
    if f["kind"] == "ball":
        return ((P - np.array(f["center"])) ** 2).sum(1) <= f["radius"] ** 2
    pl = f["plane"]
    o, x, y, z = (np.array(pl[k]) for k in ("o", "x", "y", "z"))
    loops = loops2d(f)
    if f["kind"] == "extrude":
        d = P - o
        w = d @ z
        D = f["distance"]
        ok = (w >= min(0, D)) & (w <= max(0, D))
        return ok & in_profile(loops, d @ x, d @ y)
    ox, oy, dx, dy = f["axis"]
    n = math.hypot(dx, dy)
    dx, dy = dx / n, dy / n
    px, py = -dy, dx  # perpendicular in the sketch
    pts2 = points(loops)
    side = 1.0 if ((pts2 - (ox, oy)) @ (px, py)).max() > 0 else -1.0
    A = o + x * ox + y * oy
    d3 = x * dx + y * dy
    p3 = (x * px + y * py) * side
    q = P - A
    h = q @ d3
    w = q - np.outer(h, d3)
    r = np.sqrt((w ** 2).sum(1))
    ok = in_profile(loops, ox + dx * h + px * side * r, oy + dy * h + py * side * r)
    if f["angle"] < 2 * math.pi - 1e-9:
        t3 = np.cross(d3, p3)
        ang = np.mod(np.arctan2(w @ t3, w @ p3), 2 * math.pi)
        ok &= ang <= f["angle"]
    return ok


def bbox(f):
    if f["kind"] == "ball":
        c, r = np.array(f["center"]), f["radius"]
        return c - r, c + r
    pl = f["plane"]
    o, x, y, z = (np.array(pl[k]) for k in ("o", "x", "y", "z"))
    pts2 = points(loops2d(f))
    if f["kind"] == "extrude":
        base = o + np.outer(pts2[:, 0], x) + np.outer(pts2[:, 1], y)
        allp = np.vstack([base, base + z * f["distance"]])
        return allp.min(0), allp.max(0)
    ox, oy, dx, dy = f["axis"]
    n = math.hypot(dx, dy)
    a2 = np.array([dx, dy]) / n
    A = o + x * ox + y * oy
    d3 = x * a2[0] + y * a2[1]
    rel = pts2 - (ox, oy)
    h = rel @ a2
    reach = np.sqrt((rel ** 2).sum(1) - h ** 2).max()
    ends = np.array([A + d3 * h.min(), A + d3 * h.max()])
    return ends.min(0) - reach, ends.max(0) + reach


def volume(part, samples=2e7, seed=1):
    feats = [f for f in part["features"] if f["kind"] != "fillet"]
    if len(feats) != len(part["features"]):
        raise ValueError("fillets have no membership test")
    start = max(i for i, f in enumerate(feats) if f["op"] == "new")
    feats = feats[start:]
    boxes = [bbox(f) for f in feats if f["op"] in ("new", "add")]
    lo = np.min([b[0] for b in boxes], 0) - 1e-6
    hi = np.max([b[1] for b in boxes], 0) + 1e-6
    rng = np.random.default_rng(seed)
    hit, n, chunk = 0, 0, 500_000
    while n < samples:
        P = lo + (hi - lo) * rng.random((chunk, 3))
        inside = member(feats[0], P)
        for f in feats[1:]:
            m = member(f, P)
            inside = inside | m if f["op"] == "add" else inside & ~m if f["op"] == "cut" else inside & m
        hit += int(inside.sum())
        n += chunk
    vb = float(np.prod(hi - lo))
    p = hit / n
    return vb * p, vb * math.sqrt(p * (1 - p) / n)


def judge(part, ours, theirs, samples=2e7):
    """Which of two volumes the Monte Carlo estimate backs: "ours", "occt" or "unsure"."""
    v, s = volume(part, samples)
    a, b = abs(ours - v), abs(theirs - v)
    if a < 3 * s and b > 5 * s:
        return "ours", v, s
    if b < 3 * s and a > 5 * s:
        return "occt", v, s
    return "unsure", v, s


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("part")
    ap.add_argument("--samples", type=float, default=2e7)
    a = ap.parse_args()
    d = json.load(open(a.part))
    part = d["parts"][0] if "parts" in d else d
    v, s = volume(part, a.samples)
    print(f"monte carlo volume {v:.1f} +- {s:.1f}")


if __name__ == "__main__":
    sys.exit(main())
