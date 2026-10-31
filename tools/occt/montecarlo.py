#!/usr/bin/env python3
"""A third opinion when ForgeCAD and OCCT disagree: the part's volume by Monte Carlo, classifying
random points straight against the history's analytic tools (no B-rep at all). Whole circles are
tested exactly; arcs are walked as 2048-gons (sag under 1.2e-6 of the radius). Ruled lofts blend
matched section points; smooth lofts through circles follow our definition (natural spline over
chord-length knots), so against OCCT's own spline they show which definition a volume belongs to.

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


def loop_samples(loop, n=64):
    """A profile loop at matched points: each segment by the same number of steps of its own
    parameter, as a loft's walls match them. Loops of one loft have alike segments."""
    pts = []
    for s in loop:
        if "line" in s:
            a, b, c, d = s["line"]
            pts.append((a, b))
        elif "arc" in s:
            cx, cy, r, st, sw = s["arc"]
            pts += [(cx + r * math.cos(st + sw * i / n), cy + r * math.sin(st + sw * i / n)) for i in range(n)]
        else:
            cx, cy, r = s["circle"][:3]
            pts += [(cx + r * math.cos(2 * math.pi * i / (4 * n)), cy + r * math.sin(2 * math.pi * i / (4 * n)))
                    for i in range(4 * n)]
    return np.array(pts)


def ruled_member(f, P):
    """Spans between sections on parallel planes: at each height the section is the matched
    points' straight blend, tested by even-odd rays."""
    pr = f["profiles"]
    o0, x, y, z = (np.array(pr[0]["plane"][k]) for k in ("o", "x", "y", "z"))
    if any(abs(np.array(q["plane"]["z"]) @ z) < 1 - 1e-12 for q in pr):
        raise ValueError("a ruled loft's sections must be parallel here")
    q = P - o0
    u, v, h = q @ x, q @ y, q @ z
    secs = []
    for p in pr:
        o, xk, yk = (np.array(p["plane"][k]) for k in ("o", "x", "y"))
        w = loop_samples(p["loops"][0])
        world = o + np.outer(w[:, 0], xk) + np.outer(w[:, 1], yk) - o0
        secs.append((world @ z, np.stack([world @ x, world @ y], 1)))
    inside = np.zeros(len(P), bool)
    for (h0, a), (h1, b) in zip(secs, secs[1:]):
        h0, h1 = h0.mean(), h1.mean()
        mask = (h >= min(h0, h1)) & (h <= max(h0, h1))
        idx = np.nonzero(mask)[0]
        if not len(idx):
            continue
        t = ((h[idx] - h0) / (h1 - h0))[:, None]
        pu, pv = u[idx], v[idx]
        odd = np.zeros(len(idx), bool)
        c0, c1 = a, b
        for i in range(len(a)):
            j = (i + 1) % len(a)
            pi = c0[i] * (1 - t) + c1[i] * t
            pj = c0[j] * (1 - t) + c1[j] * t
            cross = (pi[:, 1] > pv) != (pj[:, 1] > pv)
            with np.errstate(divide="ignore", invalid="ignore"):
                at = pi[:, 0] + (pv - pi[:, 1]) * (pj[:, 0] - pi[:, 0]) / (pj[:, 1] - pi[:, 1])
            odd ^= cross & (pu < at)
        inside[idx] = odd
    return inside


def natural_spline(knots):
    """The natural cubic spline's second derivatives for each unit value, one row per value."""
    n = len(knots)
    h = np.diff(knots)
    a = np.zeros((n, n))
    a[0, 0] = a[-1, -1] = 1.0
    r = np.zeros((n, n))
    for i in range(1, n - 1):
        a[i, i - 1], a[i, i], a[i, i + 1] = h[i - 1], 2 * (h[i - 1] + h[i]), h[i]
        for d in range(n):
            y = lambda k: 1.0 if k == d else 0.0
            r[i, d] = 6 * ((y(i + 1) - y(i)) / h[i] - (y(i) - y(i - 1)) / h[i - 1])
    return np.linalg.solve(a, r)


def spline_weights(knots, m2, s):
    i = min(np.searchsorted(knots, s, side="right") - 1, len(knots) - 2)
    h = knots[i + 1] - knots[i]
    a, b = (knots[i + 1] - s) / h, (s - knots[i]) / h
    w = np.zeros(len(knots))
    w[i], w[i + 1] = a, b
    return w + ((a ** 3 - a) * m2[i] + (b ** 3 - b) * m2[i + 1]) * h * h / 6


def smooth_sections(f, k=2000):
    """A smooth loft through whole circles: S(t, s) = sum w_k(s) C_k(t) (natural spline in s over
    chord-length knots between the centres, forge-geom skinned.rs), whose section at each s is the
    ellipse o(s) + cos t X(s) + sin t Y(s). Sampled at k values of s."""
    pr = f["profiles"]
    cs, xs, ys = [], [], []
    for p in pr:
        lp = p["loops"][0]
        if len(p["loops"]) != 1 or len(lp) != 1 or "circle" not in lp[0]:
            raise ValueError("a smooth loft here is through whole circles")
        o, x, y = (np.array(p["plane"][q]) for q in ("o", "x", "y"))
        cx, cy, r = lp[0]["circle"][:3]
        cs.append(o + cx * x + cy * y)
        xs.append(r * x)
        ys.append(r * y)
    cs, xs, ys = np.array(cs), np.array(xs), np.array(ys)
    knots = np.concatenate([[0.0], np.cumsum(np.linalg.norm(np.diff(cs, axis=0), axis=1))])
    m2 = natural_spline(knots)
    w = np.array([spline_weights(knots, m2, s) for s in np.linspace(0, knots[-1], k)])
    return w @ cs, w @ xs, w @ ys


def smooth_member(f, P):
    """Inside if some section's plane through p holds p inside its ellipse: where the distance to
    that plane changes sign between samples, the solved (a, b) on the ellipse's axes, interpolated."""
    o, x, y = smooth_sections(f)
    n = np.cross(x, y)
    inside = np.zeros(len(P), bool)
    prev = None
    for k in range(len(o)):
        q = P - o[k]
        g = q @ n[k]
        xx, xy, yy = x[k] @ x[k], x[k] @ y[k], y[k] @ y[k]
        det = xx * yy - xy * xy
        qx, qy = q @ x[k], q @ y[k]
        a, b = (yy * qx - xy * qy) / det, (xx * qy - xy * qx) / det
        rr = a * a + b * b
        if prev is not None:
            g0, r0 = prev
            hit = (g0 * g <= 0) & (g0 != g)
            t = g0[hit] / (g0[hit] - g[hit])
            inside[np.nonzero(hit)[0][(r0[hit] * (1 - t) + rr[hit] * t) <= 1.0]] = True
        prev = (g, rr)
    return inside


def moved_member(f, P):
    m = np.array(f["m"])
    return member(f["src"], np.linalg.solve(m[:3].T, (P - m[3]).T).T)


def member(f, P):
    if f["kind"] == "moved":
        return moved_member(f, P)
    if f["kind"] == "loft":
        return ruled_member(f, P) if f["ruled"] else smooth_member(f, P)
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
    if f["kind"] == "moved":
        lo, hi = bbox(f["src"])
        m = np.array(f["m"])
        c = np.array([[a, b, d] for a in (lo[0], hi[0]) for b in (lo[1], hi[1]) for d in (lo[2], hi[2])])
        w = c @ m[:3] + m[3]
        return w.min(0), w.max(0)
    if f["kind"] == "loft":
        if f["ruled"]:
            pts = []
            for p in f["profiles"]:
                o, x, y = (np.array(p["plane"][k]) for k in ("o", "x", "y"))
                w = loop_samples(p["loops"][0])
                pts.append(o + np.outer(w[:, 0], x) + np.outer(w[:, 1], y))
            pts = np.vstack(pts)
        else:
            o, x, y = smooth_sections(f, 400)
            pts = np.vstack([o + x * math.cos(t) + y * math.sin(t) for t in np.linspace(0, 2 * math.pi, 64)])
        return pts.min(0) - 1e-3, pts.max(0) + 1e-3
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
    by_i = {f.get("i"): f for f in part["features"]}
    for f in part["features"]:
        if f["kind"] == "shapes":
            feats += [{"kind": "shape", "op": t["op"], "shape": t["shape"]} for t in f["tools"]]
        elif f["kind"] == "pattern" and "src" in f:
            for m in f["m"]:
                feats += [{"kind": "moved", "op": "add" if by_i[i]["op"] == "new" else by_i[i]["op"], "src": by_i[i],
                           "m": m} for i in f["src"]]
        else:
            feats.append(f)
    if any(f["kind"] not in ("extrude", "revolve", "ball", "hole", "shape", "moved", "loft") for f in feats):
        raise ValueError("only extrudes, revolves, balls, holes, ruled and smooth lofts, patterns of them "
                         "and analytic tools have a membership test here")
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
    hit, n, chunk = 0, 0, 100_000
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
