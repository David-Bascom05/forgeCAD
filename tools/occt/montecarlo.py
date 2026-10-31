#!/usr/bin/env python3
"""A third opinion when ForgeCAD and OCCT disagree: the part's volume by Monte Carlo, classifying
random points straight against the history's analytic tools (no B-rep at all). Whole circles are
tested exactly; arcs are walked as 2048-gons (sag under 1.2e-6 of the radius).

  tools/occt/montecarlo.py part.json [--samples 2e7] [--budget 60]    (a part as the oracle example exports it)
"""
import argparse
import json
import math
import os
import sys
import time

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


def span(f):
    """An extrude's (lo, hi) along its normal or a revolve's (start, sweep), for the extents
    that need no part to work out."""
    if f["kind"] == "extrude":
        e = f["extent"]
        if "d" in e:
            return min(0, e["d"]), max(0, e["d"])
        if "sym" in e:
            return -e["sym"] / 2, e["sym"] / 2
        if "two" in e:
            return -e["two"][1], e["two"][0]
        raise ValueError("an extent that needs the part")
    t = f["turn"]
    if "a" in t:
        return 0.0, t["a"]
    if "sym" in t:
        return -t["sym"] / 2, t["sym"]
    if "two" in t:
        return -t["two"][1], t["two"][0] + t["two"][1]
    raise ValueError("a turn that needs the part")


def hole_section(f):
    """A hole's section as (radius, depth down the drill) points (a through hole: 1e6 deep)."""
    r, head, end = f["d"] / 2, f["head"], f["end"]
    if "depth" in end:
        depth = end["depth"]
    elif "through" in end:
        depth = 1e6
    else:
        raise ValueError("a hole end that needs the part")
    pts, mouth = [], r
    if head and "cbore" in head:
        D, d = head["cbore"]
        mouth = D / 2
        pts += [(D / 2, d), (r, d)]
    elif head and "csink" in head:
        D, a = head["csink"]
        mouth = D / 2
        pts.append((r, (D / 2 - r) / math.tan(a / 2)))
    pts.append((r, depth))
    tip = depth + (r / math.tan(f["point"] / 2) if f["point"] and "depth" in end else 0)
    return np.array([(0, 0), (mouth, 0)] + pts + [(0, tip)])


def member(f, P):
    if f["kind"] == "shape":
        return shape_member(f["shape"], P)
    if f["kind"] == "hole":
        at = f["at"]
        o, z = np.array(at["o"]), np.array(at["z"])
        q = P - o
        h = q @ z
        rho = np.sqrt(np.maximum((q ** 2).sum(1) - h * h, 0))
        return in_profile([hole_section(f)], rho, h)
    if f["kind"] == "ball":
        return ((P - np.array(f["center"])) ** 2).sum(1) <= f["radius"] ** 2
    pl = f["plane"]
    o, x, y, z = (np.array(pl[k]) for k in ("o", "x", "y", "z"))
    loops = loops2d(f)
    if f["kind"] == "extrude":
        d = P - o
        w = d @ z
        lo, hi = span(f)
        ok = (w >= lo) & (w <= hi)
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
    start, sweep = span(f)
    if sweep < 2 * math.pi - 1e-9:
        t3 = np.cross(d3, p3)
        ang = np.mod(np.arctan2(w @ t3, w @ p3) - start, 2 * math.pi)
        ok &= ang <= sweep
    return ok


def bbox(f):
    if f["kind"] == "shape":
        return shape_box(f["shape"])
    if f["kind"] == "ball":
        c, r = np.array(f["center"]), f["radius"]
        return c - r, c + r
    pl = f["plane"]
    o, x, y, z = (np.array(pl[k]) for k in ("o", "x", "y", "z"))
    pts2 = points(loops2d(f))
    if f["kind"] == "extrude":
        base = o + np.outer(pts2[:, 0], x) + np.outer(pts2[:, 1], y)
        lo, hi = span(f)
        allp = np.vstack([base + z * lo, base + z * hi])
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


def seg_poly(segs):
    """Analytic-shape segments (lines and arcs, end to end) as one closed polyline."""
    pts = []
    for s in segs:
        if "line" in s:
            pts.append(tuple(s["line"][:2]))
        else:
            cx, cy, r, st, sw = s["arc"]
            n = max(4, int(abs(sw) / (2 * math.pi) * 2048))
            pts += [(cx + r * math.cos(st + sw * i / n), cy + r * math.sin(st + sw * i / n)) for i in range(n)]
    return np.array(pts)


def local(fr, P):
    o, x, y, z = (np.array(fr[k]) for k in ("o", "x", "y", "z"))
    d = P - o
    return d @ x, d @ y, d @ z


def shape_member(s, P):
    """Membership in a tool's analytic solid (as the exporter writes `shapes`)."""
    if "ball" in s:
        return ((P - np.array(s["ball"])) ** 2).sum(1) <= s["r"] ** 2
    if "csg" in s:
        inside = shape_member(s["csg"], P)
        for r in s["rest"]:
            m = shape_member(r["shape"], P)
            inside = inside | m if r["op"] == "add" else inside & ~m if r["op"] == "cut" else inside & m
        return inside
    if "prism" in s:
        u, v, w = local(s["prism"], P)
        return (w >= s["lo"]) & (w <= s["hi"]) & in_profile([seg_poly(s["segs"])], u, v)
    if "revolved" in s:
        x, y, z = local(s["revolved"], P)
        r = np.hypot(x, y)
        (ox, oy), (dx, dy), (px, py) = s["origin"], s["dir"], s["perp"]
        ok = in_profile([seg_poly(s["segs"])], ox + dx * z + px * r, oy + dy * z + py * r)
        if s["angle"] is not None:
            ok &= np.mod(np.arctan2(y, x), 2 * math.pi) <= s["angle"]
        return ok
    # a coil: (r, h) at angle u lies at ((r + a u) cos u, (r + a u) sin u, h + b u)
    x, y, z = local(s["coil"], P)
    r, t = np.hypot(x, y), np.mod(np.arctan2(y, x), 2 * math.pi)
    poly = [seg_poly(s["segs"])]
    inside = np.zeros(len(P), bool)
    for k in range(int(s["angle"] / (2 * math.pi)) + 2):
        u = t + 2 * math.pi * k
        ok = u <= s["angle"]
        inside |= ok & in_profile(poly, r - s["a"] * u, z - s["b"] * u)
    return inside


def shape_box(s):
    """A box holding a tool's analytic solid (corners of the frame-aligned box, in world)."""
    if "ball" in s:
        c = np.array(s["ball"])
        return c - s["r"], c + s["r"]
    if "csg" in s:
        return shape_box(s["csg"])
    key = next(k for k in ("prism", "revolved", "coil") if k in s)
    fr = s[key]
    o, x, y, z = (np.array(fr[k]) for k in ("o", "x", "y", "z"))
    p = seg_poly(s["segs"])
    if key == "prism":
        lo, hi = p.min(0), p.max(0)
        box = [(lo[0], lo[1], s["lo"]), (hi[0], hi[1], s["hi"])]
    else:
        q = p - np.array(s["origin"]) if key == "revolved" else p
        if key == "revolved":
            (dx, dy), (px, py) = s["dir"], s["perp"]
            r, h = q @ np.array([px, py]), q @ np.array([dx, dy])
        else:
            r, h = q[:, 0], q[:, 1]
        R = np.abs(r).max() + abs(s.get("a", 0)) * s.get("angle", 0)
        hs = [h.min(), h.max()]
        if key == "coil":
            hs += [h.min() + s["b"] * s["angle"], h.max() + s["b"] * s["angle"]]
        box = [(-R, -R, min(hs)), (R, R, max(hs))]
    cs = np.array([o + x * a + y * b + z * c for a in (box[0][0], box[1][0]) for b in (box[0][1], box[1][1])
                   for c in (box[0][2], box[1][2])])
    return cs.min(0), cs.max(0)


def volume(part, samples=2e7, seed=1, budget=60.0, target=0.0):
    """Stops after `budget` seconds or once the standard error is under `target`, whichever
    comes first, with what it has by then."""
    feats = []
    for f in part["features"]:
        if f["kind"] == "shapes":
            feats += [{"kind": "shape", "op": t["op"], "shape": t["shape"]} for t in f["tools"]]
        else:
            feats.append(f)
    if any(f["kind"] not in ("extrude", "revolve", "ball", "hole", "shape") for f in feats):
        raise ValueError("only extrudes, revolves, balls, holes and analytic tools have a membership test here")
    for f in feats:
        if f["kind"] in ("extrude", "revolve"):
            span(f)
        if f["kind"] == "hole":
            hole_section(f)
    start = max(i for i, f in enumerate(feats) if f["op"] == "new")
    feats = feats[start:]
    boxes = [bbox(f) for f in feats if f["op"] in ("new", "add")]
    lo = np.min([b[0] for b in boxes], 0) - 1e-6
    hi = np.max([b[1] for b in boxes], 0) + 1e-6
    rng = np.random.default_rng(seed)
    hit, n, chunk = 0, 0, 500_000
    t0 = time.monotonic()
    vb = float(np.prod(hi - lo))
    while n < samples:
        if n and (time.monotonic() - t0 > budget or vb * math.sqrt(hit / n * (1 - hit / n) / n) < target):
            break
        P = lo + (hi - lo) * rng.random((chunk, 3))
        inside = member(feats[0], P)
        for f in feats[1:]:
            m = member(f, P)
            inside = inside | m if f["op"] == "add" else inside & ~m if f["op"] == "cut" else inside & m
        hit += int(inside.sum())
        n += chunk
    p = hit / n
    return vb * p, vb * math.sqrt(p * (1 - p) / n)


def judge(part, ours, theirs, samples=2e7, budget=60.0):
    """Which of two volumes the Monte Carlo estimate backs: "ours", "occt" or "unsure" (within
    `budget` seconds: sampling stops once the error is a tenth of the gap)."""
    v, s = volume(part, samples, budget=budget, target=abs(ours - theirs) / 10)
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
    ap.add_argument("--budget", type=float, default=60.0, help="seconds")
    a = ap.parse_args()
    d = json.load(open(a.part))
    part = d["parts"][0] if "parts" in d else d
    v, s = volume(part, a.samples, budget=a.budget)
    print(f"monte carlo volume {v:.1f} +- {s:.1f}")


if __name__ == "__main__":
    sys.exit(main())
