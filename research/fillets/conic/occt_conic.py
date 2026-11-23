#!/usr/bin/env python3
"""OCCT reference for conic fillets: each examples/conic part built again in OpenCascade from exact
pieces, the conic section as a rational quadratic B-spline (feet, apex; middle weight rho/(1-rho)),
the fill region swept straight (BRepPrimAPI_MakePrism) or turned (MakeRevol) and taken off or added
by a boolean. OCCT's own fillet makes circular sections only, so this is the exact construction it
can make. Prints volume and area per part; the mitre of rim.fg is the extended prisms cut each one.

  $VENV/bin/python -I occt_conic.py
"""
import json
import math

from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut, BRepAlgoAPI_Fuse
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeEdge, BRepBuilderAPI_MakeFace, BRepBuilderAPI_MakeWire
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.BRepOffsetAPI import BRepOffsetAPI_ThruSections
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox, BRepPrimAPI_MakeCylinder, BRepPrimAPI_MakePrism, BRepPrimAPI_MakeRevol
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakePolygon
from OCP.Geom import Geom_BezierCurve
from OCP.GeomAPI import GeomAPI_Interpolate

from OCP.GProp import GProp_GProps
from OCP.gp import gp_Ax1, gp_Ax2, gp_Dir, gp_Pnt, gp_Vec
from OCP.OCP.collections import Array1_double, Array1_gp_Pnt, HArray1_gp_Pnt


def P(p):
    return gp_Pnt(*p)


def region(fa, apex, fb, rho):
    """The face between the conic (fa, apex, fb) and the two legs to the apex."""
    w = rho / (1 - rho)
    poles = Array1_gp_Pnt(1, 3)
    weights = Array1_double(1, 3)
    for i, (p, wi) in enumerate([(fa, 1.0), (apex, w), (fb, 1.0)]):
        poles.SetValue(i + 1, P(p))
        weights.SetValue(i + 1, wi)
    curve = Geom_BezierCurve(poles, weights)
    wire = BRepBuilderAPI_MakeWire()
    wire.Add(BRepBuilderAPI_MakeEdge(curve).Edge())
    wire.Add(BRepBuilderAPI_MakeEdge(P(fb), P(apex)).Edge())
    wire.Add(BRepBuilderAPI_MakeEdge(P(apex), P(fa)).Edge())
    return BRepBuilderAPI_MakeFace(wire.Wire(), True).Face()


def prism(fa, apex, fb, rho, along):
    return BRepPrimAPI_MakePrism(region(fa, apex, fb, rho), gp_Vec(*along)).Shape()


def turned(fa, apex, fb, rho, origin, axis, angle=2 * math.pi):
    return BRepPrimAPI_MakeRevol(region(fa, apex, fb, rho), gp_Ax1(P(origin), gp_Dir(*axis)), angle).Shape()


def box(x0, y0, z0, dx, dy, dz):
    return BRepPrimAPI_MakeBox(P((x0, y0, z0)), dx, dy, dz).Shape()


def cut(a, b):
    return BRepAlgoAPI_Cut(a, b).Shape()


def fuse(a, b):
    return BRepAlgoAPI_Fuse(a, b).Shape()


def prism_from(points, along):
    poly = BRepBuilderAPI_MakePolygon()
    for p in points:
        poly.Add(P(p))
    poly.Close()
    return BRepPrimAPI_MakePrism(BRepBuilderAPI_MakeFace(poly.Wire(), True).Face(), gp_Vec(*along)).Shape()


def edge_fg():
    L, W, H, r, rho = 40, 30, 20, 5, 0.5
    return cut(box(0, 0, 0, L, W, H), prism((L, W - r, -1), (L, W, -1), (L - r, W, -1), rho, (0, 0, H + 2)))


def rim_fg():
    L, W, H, r, rho = 40, 30, 20, 6, 0.7
    m = 10
    s = box(0, 0, 0, L, W, H)
    # along x at y = 0 and y = W, along y at x = 0 and x = L, each run on past the box
    s = cut(s, prism((-m, r, H), (-m, 0, H), (-m, 0, H - r), rho, (L + 2 * m, 0, 0)))
    s = cut(s, prism((-m, W - r, H), (-m, W, H), (-m, W, H - r), rho, (L + 2 * m, 0, 0)))
    s = cut(s, prism((r, -m, H), (0, -m, H), (0, -m, H - r), rho, (0, W + 2 * m, 0)))
    s = cut(s, prism((L - r, -m, H), (L, -m, H), (L, -m, H - r), rho, (0, W + 2 * m, 0)))
    return s


def step_fg():
    L, W, T, S, r, rho = 40, 30, 8, 12, 5, 0.3
    pts = [(0, 0, 0), (L, 0, 0), (L, 0, T), (S, 0, T), (S, 0, 3 * T), (0, 0, 3 * T)]
    # xz's sketch extrudes along -y
    s = prism_from(pts, (0, -W, 0))
    # a fill is added: exactly the part's width, not run on past it
    return fuse(s, prism((S + r, 0, T), (S, 0, T), (S, 0, T + r), rho, (0, -W, 0)))


def wedge_fg():
    W, B, H, r, rho = 30, 40, 30, 4, 0.6
    cx = B - H / math.tan(math.radians(60))
    pts = [(0, 0, 0), (B, 0, 0), (cx, 0, H), (0, 0, H)]
    s = prism_from(pts, (0, -W, 0))
    # the faces turn by 60 degrees: each foot r tan(30deg) from the edge
    d = r * math.tan(math.radians(30))
    down = ((B - cx) / H, -1.0)
    n = math.hypot(*down)
    fa = (cx - d, 1, H)
    fb = (cx + d * down[0] / n, 1, H + d * down[1] / n)
    return cut(s, prism(fa, (cx, 1, H), fb, rho, (0, -W - 2, 0)))


def boss_fg():
    Pl, T, R, H = 60, 8, 10, 12
    s = fuse(box(0, 0, 0, Pl, Pl, T), BRepPrimAPI_MakeCylinder(gp_Ax2(P((30, 30, T)), gp_Dir(0, 0, 1)), R, H).Shape())
    c = (30, 30, 0)
    s = fuse(s, turned((30 + R + 3, 30, T), (30 + R, 30, T), (30 + R, 30, T + 3), 0.5, c, (0, 0, 1)))
    s = cut(s, turned((30 + R - 3, 30, T + H), (30 + R, 30, T + H), (30 + R, 30, T + H - 3), 0.8, c, (0, 0, 1)))
    return s


def slot_fg():
    L, R, H, r, rho = 40, 8, 10, 3, 0.5
    # the slot: a box between the arcs' centres and a cylinder at each end
    s = box(0, -R, 0, L, 2 * R, H)
    for x in (0, L):
        s = fuse(s, BRepPrimAPI_MakeCylinder(gp_Ax2(P((x, 0, 0)), gp_Dir(0, 0, 1)), R, H).Shape())
    take = prism((0, -R + r, H), (0, -R, H), (0, -R, H - r), rho, (L, 0, 0))
    take = fuse(take, prism((0, R - r, H), (0, R, H), (0, R, H - r), rho, (L, 0, 0)))
    # each end's half turn, from the line's end round the far side
    take = fuse(take, turned((L, -R + r, H), (L, -R, H), (L, -R, H - r), rho, (L, 0, 0), (0, 0, 1), math.pi))
    take = fuse(take, turned((0, R - r, H), (0, R, H), (0, R, H - r), rho, (0, 0, 0), (0, 0, 1), math.pi))
    return cut(s, take)


def _sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def _dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def _cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def _unit(a):
    n = math.sqrt(_dot(a, a))
    return tuple(x / n for x in a)


def post_section(phi, R, Pr, r, w, m=24):
    """The fill's section at angle phi round the post, a closed wire in the plane square to the
    ball centre's path: the conic from the foot on the post through the apex to the foot on the
    bar, then the bar's and the post's own traces in that plane to the root between them (each
    solved for by Newton, point by point, and interpolated)."""
    a, b = Pr + r, R + r
    sn, cs = math.sin(phi), math.cos(phi)
    c = (a * cs, a * sn, math.sqrt(b * b - a * a * sn * sn))
    t = _unit((-a * sn, a * cs, -a * a * sn * cs / c[2]))
    na = (cs, sn, 0.0)  # out of the post
    nb = _unit((0.0, c[1], c[2]))  # out of the bar
    fa = tuple(c[k] - r * na[k] for k in range(3))
    fb = tuple(c[k] - r * nb[k] for k in range(3))
    ua, ub = _sub(fa, c), _sub(fb, c)
    cos = _dot(ua, ub) / (r * r)
    apex = tuple(c[k] + (ua[k] + ub[k]) / (1 + cos) for k in range(3))
    # the root in this plane: the post's and bar's meeting curve q(al), (q - c) . t = 0
    q = lambda al: (Pr * math.cos(al), Pr * math.sin(al), math.sqrt(R * R - Pr * Pr * math.sin(al) ** 2))
    al = phi
    for _ in range(60):
        h = 1e-7
        f0 = _dot(_sub(q(al), c), t)
        f1 = (_dot(_sub(q(al + h), c), t) - _dot(_sub(q(al - h), c), t)) / (2 * h)
        step = f0 / f1
        al -= step
        if abs(step) < 1e-15:
            break
    corner = q(al)
    post_f = lambda p: p[0] ** 2 + p[1] ** 2 - Pr * Pr
    bar_f = lambda p: p[1] ** 2 + p[2] ** 2 - R * R

    def trace(p0, p1, f):
        # points of {plane} and {f = 0} from p0 to p1: along the chord, Newton across it in the plane
        d = _sub(p1, p0)
        across = _unit(_cross(t, d))
        pts = []
        for i in range(m + 1):
            base = tuple(p0[k] + d[k] * i / m for k in range(3))
            s = 0.0
            for _ in range(60):
                p = tuple(base[k] + across[k] * s for k in range(3))
                h = 1e-7
                g = (f(tuple(base[k] + across[k] * (s + h) for k in range(3))) - f(tuple(base[k] + across[k] * (s - h) for k in range(3)))) / (2 * h)
                step = f(p) / g
                s -= step
                if abs(step) < 1e-15:
                    break
            pts.append(tuple(base[k] + across[k] * s for k in range(3)))
        arr = HArray1_gp_Pnt(1, len(pts))
        for i, p in enumerate(pts):
            arr.SetValue(i + 1, P(p))
        it = GeomAPI_Interpolate(arr, False, 1e-12)
        it.Perform()
        return BRepBuilderAPI_MakeEdge(it.Curve()).Edge()

    poles = Array1_gp_Pnt(1, 3)
    weights = Array1_double(1, 3)
    for j, (p, wj) in enumerate([(fa, 1.0), (apex, w), (fb, 1.0)]):
        poles.SetValue(j + 1, P(p))
        weights.SetValue(j + 1, wj)
    wire = BRepBuilderAPI_MakeWire()
    wire.Add(BRepBuilderAPI_MakeEdge(Geom_BezierCurve(poles, weights)).Edge())
    wire.Add(trace(fb, corner, bar_f))
    wire.Add(trace(corner, fa, post_f))
    return wire.Wire()


def post_fill(n=360):
    """The post's root on the bar, which has no closed form: the conic's fill alone, lofted (smooth,
    not ruled) through n exact sections worked out here from the definition: the ball's centre where
    the bar and post offset by r meet, its feet below it, the apex. Its volume converges to the
    exact fill's as n grows (the loft is a fit between the sections)."""
    R, Pr, r, rho = 20, 8, 2, 0.5
    w = rho / (1 - rho)
    loft = BRepOffsetAPI_ThruSections(True, False, 1e-9)
    for i in range(n + 1):
        loft.AddWire(post_section(2 * math.pi * (i % n) / n, R, Pr, r, w))
    loft.Build()
    return loft.Shape()


def measure(s):
    # adaptive, to a relative error of 1e-12 (the default rule is good to about 1e-7 here)
    g = GProp_GProps()
    BRepGProp.VolumeProperties_s(s, g, 1e-12, True)
    a = GProp_GProps()
    BRepGProp.SurfaceProperties_s(s, a, 1e-12)
    return {"valid": BRepCheck_Analyzer(s).IsValid(), "vol": g.Mass(), "area": a.Mass()}


if __name__ == "__main__":
    for name, f in [("boss", boss_fg), ("edge", edge_fg), ("rim", rim_fg), ("slot", slot_fg), ("step", step_fg), ("wedge", wedge_fg)]:
        print(json.dumps({"part": name, **measure(f())}))
    # the post's fill alone, its volume to set against the part's minus the part without the round
    for n in (90, 180, 360):
        m = measure(post_fill(n))
        print(json.dumps({"part": "post fill", "sections": n, "valid": m["valid"], "vol": m["vol"]}))


def post_fill_quadrature(n_phi=256, n_s=64):
    """The post's fill by quadrature alone, no OCCT: each section square to the ball centre's path
    sweeps V = integral of (A |c'| - m . c'' / |c'|) dphi, A its area and m its first moment about c
    (from Green's theorem on its exact boundary: the conic, and the two faces' traces solved by
    Newton), the trapezoid rule in phi (exact to rounding for a smooth periodic integrand)."""
    R, Pr, r, rho = 20, 8, 2, 0.5
    w = rho / (1 - rho)
    a, b = Pr + r, R + r
    xs, ws = gauss_legendre(n_s)
    total = 0.0
    for i in range(n_phi):
        phi = 2 * math.pi * i / n_phi
        sn, cs = math.sin(phi), math.cos(phi)
        cz = math.sqrt(b * b - a * a * sn * sn)
        c = (a * cs, a * sn, cz)
        d1 = (-a * sn, a * cs, -a * a * sn * cs / cz)
        # d2 of cz: d/dphi (-a^2 sn cs / cz)
        dcz = d1[2]
        d2 = (-a * cs, -a * sn, -(a * a * (cs * cs - sn * sn) * cz - a * a * sn * cs * dcz) / (cz * cz))
        speed = math.sqrt(_dot(d1, d1))
        t = tuple(x / speed for x in d1)
        e1 = _unit(_cross(t, (0.0, 0.0, 1.0)) if abs(t[2]) < 0.9 else _cross(t, (1.0, 0.0, 0.0)))
        e2 = _cross(t, e1)
        loc = lambda p: (_dot(_sub(p, c), e1), _dot(_sub(p, c), e2))
        na, nb = (cs, sn, 0.0), _unit((0.0, c[1], c[2]))
        fa = tuple(c[k] - r * na[k] for k in range(3))
        fb = tuple(c[k] - r * nb[k] for k in range(3))
        ua, ub = _sub(fa, c), _sub(fb, c)
        cos = _dot(ua, ub) / (r * r)
        apex = tuple(c[k] + (ua[k] + ub[k]) / (1 + cos) for k in range(3))
        q = lambda al: (Pr * math.cos(al), Pr * math.sin(al), math.sqrt(R * R - Pr * Pr * math.sin(al) ** 2))
        al = phi
        for _ in range(80):
            h = 1e-6
            f0 = _dot(_sub(q(al), c), t)
            f1 = (_dot(_sub(q(al + h), c), t) - _dot(_sub(q(al - h), c), t)) / (2 * h)
            al -= f0 / f1
        corner = q(al)
        # pieces of the boundary as (x(s), y(s)) in the section's frame, s in [0, 1], with derivatives
        A0, A1, A2 = loc(fa), loc(apex), loc(fb)

        def conic(s):
            b0, b1, b2 = (1 - s) ** 2, 2 * s * (1 - s) * w, s * s
            g0, g1, g2 = -2 * (1 - s), 2 * (1 - 2 * s) * w, 2 * s
            D, dD = b0 + b1 + b2, g0 + g1 + g2
            X = [(b0 * A0[k] + b1 * A1[k] + b2 * A2[k]) for k in (0, 1)]
            dX = [(g0 * A0[k] + g1 * A1[k] + g2 * A2[k]) for k in (0, 1)]
            return [X[k] / D for k in (0, 1)], [(dX[k] * D - X[k] * dD) / (D * D) for k in (0, 1)]

        def trace(p0, p1, rad, axis):
            # the face's trace from p0 to p1 in the section plane: base on the chord, Newton across
            P0, P1 = loc(p0), loc(p1)
            dx, dy = P1[0] - P0[0], P1[1] - P0[1]
            L = math.hypot(dx, dy)
            mx, my = -dy / L, dx / L

            def f(x, y):
                p = tuple(c[k] + x * e1[k] + y * e2[k] for k in range(3))
                return (p[0] ** 2 + p[1] ** 2 if axis == "z" else p[1] ** 2 + p[2] ** 2) - rad * rad

            def at(s):
                bx, by = P0[0] + dx * s, P0[1] + dy * s
                u = 0.0
                for _ in range(60):
                    h = 1e-7
                    g = (f(bx + mx * (u + h), by + my * (u + h)) - f(bx + mx * (u - h), by + my * (u - h))) / (2 * h)
                    step = f(bx + mx * u, by + my * u) / g
                    u -= step
                    if abs(step) < 1e-16:
                        break
                return bx + mx * u, by + my * u

            def piece(s):
                h = 1e-6
                (x0, y0), (xa, ya), (xb, yb) = at(s), at(s - h), at(s + h)
                return [x0, y0], [(xb - xa) / (2 * h), (yb - ya) / (2 * h)]

            return piece

        area, mx_, my_ = 0.0, 0.0, 0.0
        for piece in (conic, trace(fb, corner, R, "x"), trace(corner, fa, Pr, "z")):
            for x, wt in zip(xs, ws):
                s = 0.5 * (1 + x)
                (X, Y), (dX, dY) = piece(s)
                k = 0.5 * wt
                area += k * 0.5 * (X * dY - Y * dX)
                mx_ += k * 0.5 * X * X * dY
                my_ += -k * 0.5 * Y * Y * dX
        m = tuple(mx_ * e1[k] + my_ * e2[k] for k in range(3))
        total += (abs(area) * speed - math.copysign(1.0, area) * _dot(m, d2) / speed) * 2 * math.pi / n_phi
    return total


def gauss_legendre(n):
    """Nodes and weights on [-1, 1], by Newton on the Legendre polynomial."""
    xs, ws = [], []
    for i in range(n):
        x = math.cos(math.pi * (i + 0.75) / (n + 0.5))
        for _ in range(100):
            p0, p1 = 1.0, x
            for k in range(2, n + 1):
                p0, p1 = p1, ((2 * k - 1) * x * p1 - (k - 1) * p0) / k
            dp = n * (x * p1 - p0) / (x * x - 1)
            dx = p1 / dp
            x -= dx
            if abs(dx) < 1e-16:
                break
        xs.append(x)
        ws.append(2 / ((1 - x * x) * dp * dp))
    return xs, ws
