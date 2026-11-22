"""The certified tracer against OpenCascade: for each pair the certify_sweep example dumped (its
surfaces and certified chains), GeomAPI_IntSS's curves sampled inside the region, and the distance
from each sample to the nearest chord of the chains, and from each chain point to the nearest
OCCT sample. Both small: the same branches, none missed either way.
usage: <venv python> tools/occt/certify_check.py DUMP [half_size]"""
import math
import sys

from OCP.gp import gp_Ax3, gp_Dir, gp_Pnt
from OCP.Geom import (Geom_ConicalSurface, Geom_CylindricalSurface, Geom_Plane,
                      Geom_SphericalSurface, Geom_ToroidalSurface)
from OCP.GeomAPI import GeomAPI_IntSS


def surface(f):
    kind = f[1]
    o, x, z = [float(v) for v in f[2:5]], [float(v) for v in f[5:8]], [float(v) for v in f[8:11]]
    r1, r2, r3 = float(f[11]), float(f[12]), float(f[13])
    ax = gp_Ax3(gp_Pnt(*o), gp_Dir(*z), gp_Dir(*x))
    if kind == "plane":
        return Geom_Plane(ax)
    if kind == "sphere":
        return Geom_SphericalSurface(ax, r1)
    if kind == "cylinder":
        return Geom_CylindricalSurface(ax, r1)
    if kind == "cone":
        return Geom_ConicalSurface(ax, math.atan2(r2, r3), r1)
    return Geom_ToroidalSurface(ax, r1, r2)


def nappe(f):
    """The kernel's cone stops at its apex (r + tan * lz >= 0); OpenCascade's runs on through it."""
    if f[1] != "cone":
        return lambda p: True
    o, z = [float(v) for v in f[2:5]], [float(v) for v in f[8:11]]
    r, sn, cs = float(f[11]), float(f[12]), float(f[13])
    return lambda p: r * cs + sn * sum((p[i] - o[i]) * z[i] for i in range(3)) >= -1e-9


def seg_dist(p, a, b):
    d = [b[i] - a[i] for i in range(3)]
    w = [p[i] - a[i] for i in range(3)]
    dd = sum(c * c for c in d)
    t = 0.0 if dd == 0 else max(0.0, min(1.0, sum(w[i] * d[i] for i in range(3)) / dd))
    return math.dist(p, [a[i] + t * d[i] for i in range(3)])


def pt(c, t):
    p = c.Value(t)
    return (p.X(), p.Y(), p.Z())


def main():
    path = sys.argv[1]
    half = float(sys.argv[2]) if len(sys.argv) > 2 else 8.0
    inside = lambda p, m=0.0: all(abs(c) <= half - m for c in p)
    blocks = open(path).read().strip().split("\n\n")
    worst_occ, worst_ours, n, skipped, bad = 0.0, 0.0, 0, 0, []
    for blk in blocks:
        lines = blk.split("\n")
        sa, sb = surface(lines[0].split()), surface(lines[1].split())
        chains = []
        for ln in lines[2:]:
            v = ln.split()
            closed = v[1] == "1"
            pts = [tuple(float(c) for c in v[2 + 3 * i:5 + 3 * i]) for i in range((len(v) - 2) // 3)]
            chains.append((closed, pts))
        inter = GeomAPI_IntSS(sa, sb, 1e-7)
        if not inter.IsDone():
            skipped += 1
            continue
        on = [nappe(lines[0].split()), nappe(lines[1].split())]
        occ = []
        for k in range(1, inter.NbLines() + 1):
            c = inter.Line(k)
            t0, t1 = max(c.FirstParameter(), -1e3), min(c.LastParameter(), 1e3)
            # coarse, then fine over the stretches inside the region
            m = 4000
            ts = [t0 + (t1 - t0) * i / m for i in range(m + 1)]
            keep = [i for i, t in enumerate(ts) if inside(pt(c, t), -1.0)]
            for i in keep:
                a, b = ts[max(i - 1, 0)], ts[min(i + 1, m)]
                for j in range(41):
                    q = pt(c, a + (b - a) * j / 40)
                    if on[0](q) and on[1](q):
                        occ.append(q)
        pair_occ = pair_ours = 0.0
        segs = []
        for closed, pts in chains:
            m = len(pts)
            for i in range(m if closed else m - 1):
                segs.append((pts[i], pts[(i + 1) % m]))
        # OCCT's points well inside the region lie on our chains
        for p in occ:
            if inside(p, 0.05):
                d = min((seg_dist(p, a, b) for a, b in segs), default=float("inf"))
                pair_occ = max(pair_occ, d)
        # our points inside the region lie on OCCT's curves
        for _, pts in chains:
            for p in pts:
                if inside(p) and occ:
                    pair_ours = max(pair_ours, min(math.dist(p, q) for q in occ))
        n += 1
        h = max((math.dist(a, b) for a, b in segs), default=0.0)
        if pair_occ > h or pair_ours > h:
            bad.append(f"  {lines[0].split()[1]}-{lines[1].split()[1]}: {len(chains)} chains, "
                       f"{inter.NbLines()} OCCT lines, {pair_occ:.2e} {pair_ours:.2e} (step {h:.2e})")
        worst_occ, worst_ours = max(worst_occ, pair_occ), max(worst_ours, pair_ours)
    print(f"{n} pairs ({skipped} OCCT could not intersect): OCCT point to our chain at most "
          f"{worst_occ:.2e}, our point to OCCT's samples at most {worst_ours:.2e}; "
          f"{len(bad)} apart by more than a step:")
    print("\n".join(bad))


main()
