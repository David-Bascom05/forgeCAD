#!/usr/bin/env python3
"""The fillet and boolean hard-case corpus: writes tools/corpus/fillet.json.

Each case is a part history in the oracle's JSON schema (examples/oracle.rs, tools/occt/build.py),
its fillets as {"kind": "fillet", "edges": [{"pts": [[x, y, z]], "law": [[[x, y, z], r]]}, ...]}:
each point lies on an edge to round (the edge nearest the point, so it survives an edge being
split into pieces), with its radius. A fillet rounds every edge tangent-continuous with a
picked one, as every rolling-ball engine does; edges meeting at an angle are picked one by one.

A case's "expect":
  status       "ok": a valid solid; "ok_or_reject": a valid solid or a clear error naming why
               (e.g. the radius is too large for an adjacent face); "reject": must be refused
  volume/area  analytic values, with "exact": true where closed form (Pappus, Minkowski sums,
               the corner overlaps worked out below); otherwise left to the oracle's OCCT run
  surfaces     distinct surfaces by kind (face counts differ between kernels with seams, so the
               corpus counts surfaces: a cylinder split into two half faces is one cylinder)
  faces/edges/vertices  where the topology is unambiguous

  python3 tools/corpus/fillet_corpus.py            # rewrite fillet.json
"""
import json
import math
import os

PI = math.pi
HERE = os.path.dirname(os.path.abspath(__file__))

WORLD = {"o": [0.0, 0.0, 0.0], "x": [1.0, 0.0, 0.0], "y": [0.0, 1.0, 0.0], "z": [0.0, 0.0, 1.0]}


def frame(o, z, x):
    def unit(v):
        n = math.sqrt(sum(c * c for c in v))
        return [c / n for c in v]

    z = unit(z)
    d = sum(x[i] * z[i] for i in range(3))
    x = unit([x[i] - z[i] * d for i in range(3)])
    y = [z[1] * x[2] - z[2] * x[1], z[2] * x[0] - z[0] * x[2], z[0] * x[1] - z[1] * x[0]]
    return {"o": list(map(float, o)), "x": x, "y": y, "z": z}


def rect(x0, y0, w, h):
    p = [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]
    return [{"line": [*p[i], *p[(i + 1) % 4]]} for i in range(4)]


def poly(pts):
    n = len(pts)
    return [{"line": [*pts[i], *pts[(i + 1) % n]]} for i in range(n)]


def rounded_rect(x0, y0, w, h, r):
    """Lines and quarter arcs, counter-clockwise from the bottom edge."""
    a, b, c, d = x0 + r, x0 + w - r, y0 + r, y0 + h - r
    q = PI / 2
    return [
        {"line": [a, y0, b, y0]},
        {"arc": [b, c, r, -q, q]},
        {"line": [x0 + w, c, x0 + w, d]},
        {"arc": [b, d, r, 0.0, q]},
        {"line": [b, y0 + h, a, y0 + h]},
        {"arc": [a, d, r, q, q]},
        {"line": [x0, d, x0, c]},
        {"arc": [a, c, r, PI, q]},
    ]


def box(x, y, z, w, h, d, op="new"):
    return {"kind": "extrude", "op": op, "plane": frame([x, y, z], [0, 0, 1], [1, 0, 0]),
            "loops": [rect(0.0, 0.0, w, h)], "extent": {"d": float(d)}}


def extrude(plane, loops, d, op):
    return {"kind": "extrude", "op": op, "plane": plane, "loops": loops, "extent": {"d": float(d)}}


def cyl(o, a, r, h, op="new", x=None):
    x = x or ([1, 0, 0] if abs(a[0]) < 0.9 else [0, 1, 0])
    return extrude(frame(o, a, x), [[{"circle": [0.0, 0.0, float(r), True]}]], h, op)


def spun(o, a, pts, op="new"):
    """Revolve a (radius, height) polygon about the axis from o along a, a whole turn."""
    u = [1, 0, 0] if abs(a[0]) < 0.9 else [0, 1, 0]
    # sketch x along u, sketch y along the axis: normal u x a
    n = [u[1] * a[2] - u[2] * a[1], u[2] * a[0] - u[0] * a[2], u[0] * a[1] - u[1] * a[0]]
    return {"kind": "revolve", "op": op, "plane": frame(o, n, u), "loops": [poly(pts)],
            "axis": [0.0, 0.0, 0.0, 1.0], "turn": {"a": 2 * PI}}


def ball(c, r, op="new"):
    return {"kind": "ball", "op": op, "center": list(map(float, c)), "radius": float(r)}


def fillet(r, *at, radii=None):
    rs = list(map(float, radii)) if radii else [float(r)] * len(at)
    pts = [list(map(float, p)) for p in at]
    return {"kind": "fillet", "edges": [{"pts": [p], "law": [[p, q]]} for p, q in zip(pts, rs)]}


def rim(cx, cy, r, z):
    """A point on a horizontal circle, one radian round from its start (off any seam)."""
    return [cx + r * math.cos(1.0), cy + r * math.sin(1.0), z]


def box_edges(a, b, c):
    """A point on each of the twelve edges of the box [0, a] x [0, b] x [0, c]."""
    pts = []
    for z in (0, c):
        pts += [[a / 2, 0, z], [a, b / 2, z], [a / 2, b, z], [0, b / 2, z]]
    return pts + [[0, 0, c / 2], [a, 0, c / 2], [a, b, c / 2], [0, b, c / 2]]


def corner_area(a, r):
    """Cross-section a rolling ball of radius r removes from (or adds to) a straight corner of
    interior angle a: the region between the two sides and the arc tangent to both."""
    return r * r * (1.0 / math.tan(a / 2) - (PI - a) / 2)


def pappus(a, r, R, outward):
    """Volume swept by a right-angle corner's region revolved about an axis at radius R from the
    corner; `outward` is +1 when the region lies farther from the axis than the corner, -1 nearer.
    The region (an r x r square less a quarter disc) has its centroid r (5/6 - pi/4) / (1 - pi/4)
    from each side."""
    if abs(a - PI / 2) > 1e-12:
        raise ValueError("only right-angle corners revolve here")
    d = r * (5 / 6 - PI / 4) / (1 - PI / 4)
    return 2 * PI * (R + outward * d) * corner_area(a, r)


def cases():
    out = []

    def case(id, cls, description, why, features, expect, refs=()):
        features = [dict(f, i=i) for i, f in enumerate(features)]
        out.append({"id": id, "class": cls, "description": description, "why": why,
                    "features": features, "expect": expect, "refs": list(refs)})

    q = PI / 2
    # --- baselines with closed forms ---
    a, b, c, r = 40.0, 30.0, 20.0, 5.0
    case("box_one_edge", "baseline",
         "40x30x20 box, the top front edge (along x) rounded r 5.",
         "The simplest blend: one convex edge between two planes, ending on two planar faces.",
         [box(0, 0, 0, a, b, c), fillet(r, [a / 2, 0, c])],
         {"status": "ok", "volume": a * b * c - corner_area(q, r) * a, "exact": True,
          "faces": 7, "edges": 15, "vertices": 10, "surfaces": {"plane": 6, "cylinder": 1}})
    case("box_top_loop", "baseline",
         "40x30x20 box, the four top edges rounded r 5; the vertical edges stay sharp.",
         "Two blends meet at each top corner with a sharp third edge: the blend cylinders end on "
         "each other along a miter curve instead of a vertex blend.",
         [box(0, 0, 0, a, b, c), fillet(r, [a / 2, 0, c], [a, b / 2, c], [a / 2, b, c], [0, b / 2, c])],
         {"status": "ok", "exact": True,
          # each corner's two removed prisms overlap in r^3 (5/3 - pi/2)
          "volume": a * b * c - corner_area(q, r) * 2 * (a + b) + 4 * r ** 3 * (5 / 3 - PI / 2),
          "faces": 10, "edges": 20, "vertices": 12, "surfaces": {"plane": 6, "cylinder": 4}})
    case("box_corner_three", "vertex",
         "40x30x20 box, the three edges at the corner (40, 30, 20) rounded r 5.",
         "Three equal-radius convex blends meeting at a vertex: the vertex blend is a sphere octant.",
         [box(0, 0, 0, a, b, c), fillet(r, [a / 2, b, c], [a, b / 2, c], [a, b, c / 2])],
         {"status": "ok", "exact": True,
          "volume": a * b * c - corner_area(q, r) * (a + b + c - 3 * r) - (r ** 3 - PI * r ** 3 / 6),
          "faces": 10, "edges": 21, "vertices": 13, "surfaces": {"plane": 6, "cylinder": 3, "sphere": 1}})
    ia, ib, ic = a - 2 * r, b - 2 * r, c - 2 * r
    case("box_all_edges", "vertex",
         "40x30x20 box, all twelve edges rounded r 5.",
         "Eight sphere-octant vertex blends; every blend ends on vertex blends, none on a face.",
         [box(0, 0, 0, a, b, c), fillet(r, *box_edges(a, b, c))],
         {"status": "ok", "exact": True,
          "volume": ia * ib * ic + 2 * r * (ia * ib + ib * ic + ia * ic) + PI * r * r * (ia + ib + ic)
          + 4 / 3 * PI * r ** 3,
          "area": 2 * (ia * ib + ib * ic + ia * ic) + 2 * PI * r * (ia + ib + ic) + 4 * PI * r * r,
          "faces": 26, "edges": 48, "vertices": 24,
          "surfaces": {"plane": 6, "cylinder": 12, "sphere": 8}},
         [])
    R, h, rf = 10.0, 20.0, 3.0
    case("cylinder_cap_edge", "seam",
         "Cylinder r 10, height 20; the top circular edge rounded r 3.",
         "A closed edge crossing the cylinder's seam; the blend is a torus that closes on itself.",
         [cyl([0, 0, 0], [0, 0, 1], R, h), fillet(rf, rim(0, 0, R, h))],
         {"status": "ok", "exact": True, "volume": PI * R * R * h - pappus(q, rf, R, -1),
          "surfaces": {"plane": 2, "cylinder": 1, "torus": 1}})
    case("hole_rim", "seam",
         "60x60x10 plate with a through hole r 5 at its centre; the hole's top rim rounded r 2.",
         "A concave-side closed edge: the torus blend lies outside the hole's radius.",
         [box(0, 0, 0, 60, 60, 10), cyl([30, 30, -1], [0, 0, 1], 5, 12, "cut"), fillet(2, rim(30, 30, 5, 10))],
         {"status": "ok", "exact": True,
          "volume": 3600 * 10 - PI * 25 * 10 - pappus(q, 2, 5, +1),
          "surfaces": {"plane": 6, "cylinder": 1, "torus": 1}})
    case("boss_base", "boolean_then_fillet",
         "60x60x10 plate with a boss r 8, 10 high, added on top; the concave edge round the boss's "
         "base rounded r 2.",
         "A concave blend (adds material) on an edge a boolean made.",
         [box(0, 0, 0, 60, 60, 10), cyl([30, 30, 10], [0, 0, 1], 8, 10, "add"), fillet(2, rim(30, 30, 8, 10))],
         {"status": "ok", "exact": True,
          "volume": 36000 + PI * 64 * 10 + pappus(q, 2, 8, +1),
          "surfaces": {"plane": 7, "cylinder": 1, "torus": 1}})
    case("boss_base_and_top", "boolean_then_fillet",
         "The boss_base part with the boss's top rim rounded r 2 as well (two fillet steps).",
         "Two blends on one cylindrical face, from both ends: the face shrinks to a band.",
         [box(0, 0, 0, 60, 60, 10), cyl([30, 30, 10], [0, 0, 1], 8, 10, "add"), fillet(2, rim(30, 30, 8, 10)),
          fillet(2, rim(30, 30, 8, 20))],
         {"status": "ok", "exact": True,
          "volume": 36000 + PI * 64 * 10 + pappus(q, 2, 8, +1) - pappus(q, 2, 8, -1),
          "surfaces": {"plane": 7, "cylinder": 1, "torus": 2}})
    case("long_tangent_chain", "tangent_chain",
         "Rounded rectangle 60x40 with corner radius 8, extruded 20; the top edge (four lines and four "
         "arcs, tangent-continuous) rounded r 3 from one picked point.",
         "Chain propagation across G1 joints, alternating cylinder and torus blends with no vertex "
         "blends: the blend's own seams must line up exactly where the arcs meet the lines.",
         [extrude(WORLD, [rounded_rect(0, 0, 60, 40, 8)], 20, "new"), fillet(3, [30, 0, 20])],
         {"status": "ok", "exact": True,
          "volume": (60 * 40 - (4 - PI) * 64) * 20 - corner_area(q, 3) * (2 * (60 - 16) + 2 * (40 - 16))
          - 4 * pappus(q, 3, 8, -1) / 4,
          "surfaces": {"plane": 6, "cylinder": 8, "torus": 4}})
    # --- consumed faces ---
    case("full_round_rib", "consumed_face",
         "60x40x10 plate with a rib 10 wide, 10 high along y on top; both top edges of the rib rounded "
         "r 5, exactly half the rib's width.",
         "The blends consume the rib's top face exactly: the two blend cylinders meet tangentially "
         "and the face between them vanishes (the 'full round').",
         [box(0, 0, 0, 60, 40, 10), box(25, 0, 10, 10, 40, 10, "add"), fillet(5, [25, 20, 20], [35, 20, 20])],
         {"status": "ok", "exact": True, "volume": 24000 + 4000 - 2 * corner_area(q, 5) * 40})
    case("full_round_rib_sliver", "consumed_face",
         "As full_round_rib with r 4.999: a 0.002 wide strip of the top face survives.",
         "A sliver face just above tolerance; the blend boundaries nearly coincide.",
         [box(0, 0, 0, 60, 40, 10), box(25, 0, 10, 10, 40, 10, "add"),
          fillet(4.999, [25, 20, 20], [35, 20, 20])],
         {"status": "ok", "exact": True, "volume": 28000 - 2 * corner_area(q, 4.999) * 40})
    case("full_round_rib_over", "overflow",
         "As full_round_rib with r 5.001: the two blends would cross.",
         "The top face is narrower than the two blends need: they must merge into one blend or be "
         "refused clearly, never produce a self-intersecting solid.",
         [box(0, 0, 0, 60, 40, 10), box(25, 0, 10, 10, 40, 10, "add"),
          fillet(5.001, [25, 20, 20], [35, 20, 20])],
         {"status": "ok_or_reject"})
    case("hole_wall_consumed", "consumed_face",
         "60x60x10 plate with a through hole r 5; both rims rounded r 5 (half the plate's thickness).",
         "The hole's cylindrical wall is consumed exactly: two tori meet tangentially at mid-plane.",
         [box(0, 0, 0, 60, 60, 10), cyl([30, 30, -1], [0, 0, 1], 5, 12, "cut"),
          fillet(5, rim(30, 30, 5, 10), rim(30, 30, 5, 0))],
         {"status": "ok", "exact": True, "volume": 36000 - PI * 25 * 10 - 2 * pappus(q, 5, 5, +1),
          "surfaces": {"plane": 6, "torus": 2}})
    # --- overflow / cliff edges ---
    case("cliff_ridge", "overflow",
         "40x40x20 box with the top 2 mm cut away except a 2 mm wide ridge along x = 0..2; the ridge's "
         "top inner edge (x = 2, z = 20) rounded r 5.",
         "Blend overflow: the ball is larger than both adjacent faces (2 wide, 2 high), so it rolls "
         "off them onto the faces beyond (a 'cliff edge').",
         [box(0, 0, 0, 40, 40, 20), box(2, -1, 18, 40, 42, 3, "cut"), fillet(5, [2, 20, 20])],
         {"status": "ok_or_reject"})
    case("short_edge_big_radius", "overflow",
         "40x2x20 slab; the top edge along y (length 2) at x = 40 rounded r 5.",
         "The radius exceeds the slab's thickness: the blend runs off both end faces.",
         [box(0, 0, 0, 40, 2, 20), fillet(5, [40, 1, 20])],
         {"status": "ok_or_reject"})
    case("radius_exceeds_face", "overflow",
         "40x40x4 plate; one top edge rounded r 5 (the side face is only 4 high).",
         "The blend's lower boundary would fall below the bottom face: overflow onto the bottom "
         "face's edge, or a clear refusal.",
         [box(0, 0, 0, 40, 40, 4), fillet(5, [20, 0, 4])],
         {"status": "ok_or_reject"})
    # --- vertices: setbacks, mixed convexity, mixed radii ---
    case("corner_mixed_radii", "vertex",
         "40x30x20 box, the three edges at (40, 30, 20) rounded r 2, 4 and 6.",
         "Unequal radii at a vertex: no sphere fits; needs a setback vertex blend (an n-sided patch) "
         "joining three blends of different size.",
         [box(0, 0, 0, a, b, c), fillet(4, [a / 2, b, c], [a, b / 2, c], [a, b, c / 2], radii=[2, 4, 6])],
         {"status": "ok"})
    case("corner_two_of_three", "vertex",
         "40x30x20 box, two of the three edges at (40, 30, 20) rounded r 5 (the vertical edge stays).",
         "The two blends end at the sharp edge's vertex; their ends must be trimmed against each other "
         "and the vertical edge's faces.",
         [box(0, 0, 0, a, b, c), fillet(5, [a / 2, b, c], [a, b / 2, c])],
         {"status": "ok", "exact": True,
          "volume": a * b * c - corner_area(q, 5) * (a + b) + r ** 3 * (5 / 3 - PI / 2)})
    case("step_mixed_convexity", "vertex",
         "An L step: 40x40x10 block with a 20x40x10 block on its left half. At the vertex (20, 0, 10) "
         "the concave edge along the step's foot (x = 20, z = 10), the convex step front edge (x = 20, y = 0, vertical) and the base's front "
         "top edge (y = 0, z = 10) meet; all three rounded r 3.",
         "A vertex where convex and concave blends meet (mixed convexity): the classic setback case; "
         "the rolling ball cannot touch all three faces at once.",
         [box(0, 0, 0, 40, 40, 10), box(0, 0, 10, 20, 40, 10, "add"),
          fillet(3, [20, 20, 10], [20, 0, 15], [30, 0, 10])],
         {"status": "ok"})
    case("pyramid_apex", "vertex",
         "A square pyramid (base 40, height 20) made by cutting a box with four tilted half-spaces; its "
         "four apex edges rounded r 2.",
         "A four-edge vertex: the vertex blend is a four-sided patch.",
         [box(-20, -20, 0, 40, 40, 20)] + [
             extrude(frame([20 * cx, 20 * cy, 0], [cx, cy, 1], [-cy, cx, 0]), [rect(-40, -40, 80, 80)], 40, "cut")
             for cx, cy in ((1, 0), (-1, 0), (0, 1), (0, -1))]
         + [fillet(2, [5, 5, 15], [-5, 5, 15], [-5, -5, 15], [5, -5, 15])],
         {"status": "ok"})
    # --- near tangency and vanishing dihedral ---
    th = math.radians(1.0)
    case("near_tangent_faces", "near_tangent",
         "40x40x20 box whose top is cut by a plane tilted 1 degree about the y axis through (20, 0, 20); "
         "the ridge edge at x = 20 (dihedral 179 degrees) rounded r 5.",
         "Two nearly coplanar faces: the blend is 5 * pi/180 wide and its cross-section area tiny; "
         "contact curves are ill-conditioned.",
         [box(0, 0, 0, 40, 40, 20),
          extrude(frame([20, -1, 20], [math.sin(th), 0, math.cos(th)], [math.cos(th), 0, -math.sin(th)]),
                  [rect(0, -1, 40, 44)], 10, "cut"),
          fillet(5, [20, 20, 20])],
         {"status": "ok", "exact": True,
          "volume": 40 * 40 * 20 - 0.5 * 20 * 20 * math.tan(th) * 40 - corner_area(PI - th, 5) * 40})
    case("equal_radius_tee", "near_tangent",
         "Two pipes of equal radius 10 crossing at right angles (union of cylinders along x and y, both "
         "40 long); the intersection edges rounded r 2.",
         "The intersection curves cross at two points where the dihedral reaches 180 degrees: the blend "
         "width falls to zero there (vanishing dihedral).",
         [cyl([-20, 0, 0], [1, 0, 0], 10, 40), cyl([0, -20, 0], [0, 1, 0], 10, 40, "add"),
          fillet(2, [5, 5, math.sqrt(75)])],
         {"status": "ok_or_reject"})
    case("pipe_tee", "varying_dihedral",
         "A pipe r 10 along x (40 long) with a branch r 6 along z from its axis up to z = 20; the "
         "saddle edge where they meet rounded r 2.",
         "The dihedral varies continuously round a closed saddle curve (4th degree); the blend's "
         "radius of curvature varies with it.",
         [cyl([-20, 0, 0], [1, 0, 0], 10, 40), cyl([0, 0, 0], [0, 0, 1], 6, 20, "add"),
          fillet(2, [6, 0, 10])],
         {"status": "ok"})
    # --- poles and seams ---
    case("sphere_boss_at_pole", "pole",
         "A ball r 20 with a boss r 5 standing on its north pole (to z = 30); the junction rounded r 2.",
         "The edge circles the sphere's pole: the blend's spine winds round a parametric singularity.",
         [ball([0, 0, 0], 20), cyl([0, 0, 10], [0, 0, 1], 5, 20, "add"), fillet(2, rim(0, 0, 5, math.sqrt(400 - 25)))],
         {"status": "ok"})
    case("half_ball_rim", "pole",
         "A ball r 20 cut in half by the plane x = 0; the great circle where the plane meets the sphere "
         "rounded r 2.",
         "The edge passes through both of the sphere's poles (as the kernels parametrise it).",
         [ball([0, 0, 0], 20), box(0, -30, -30, 30, 60, 60, "cut"), fillet(2, [0, 20, 0])],
         {"status": "ok"})
    case("cone_base_rim", "seam",
         "A cone (base r 10, height 20) from a revolved triangle; its base rim rounded r 2.",
         "A blend between a cone and a plane: the interior angle is 63.4 degrees, not 90.",
         [spun([0, 0, 0], [0, 0, 1], [(0, 0), (10, 0), (0, 20)]), fillet(2, [10, 0, 0])],
         {"status": "ok"})
    # --- tiny edges left by booleans ---
    case("boolean_sliver_edge", "tiny_edge",
         "40x40x20 box with a boss r 10, 20 high, whose wall passes 0.01 inside the side x = 40 "
         "(centre at x = 49.99); then the box's top edge along x = 40 rounded r 3.",
         "The boolean leaves 0.01-long edges and a sliver face where the boss grazes the side; the edge "
         "to blend is in pieces, one of them tiny.",
         [box(0, 0, 0, 40, 40, 20), cyl([49.99, 20, 0], [0, 0, 1], 10, 20, "add"), fillet(3, [40, 5, 20])],
         {"status": "ok_or_reject"})
    case("boolean_tiny_step", "tiny_edge",
         "40x40x20 box plus a 20x40x20.001 box on its left half: a 0.001 step in the top face; the box's "
         "front top edge (y = 0, z = 20) rounded r 3.",
         "The chain to blend crosses a 0.001 step: two blends 0.001 apart must join smoothly or the "
         "step be absorbed.",
         [box(0, 0, 0, 40, 40, 20), box(0, 0, 0, 20, 40, 20.001, "add"), fillet(3, [30, 0, 20])],
         {"status": "ok_or_reject"})
    # --- fillet then boolean ---
    case("hole_through_blend", "fillet_then_boolean",
         "box_top_loop (40x30x20, top edges r 5) then a hole r 3 drilled along y through the front "
         "blend at x = 20, z = 17.",
         "A boolean against a blend surface (a procedural surface): exact intersection curves on it.",
         [box(0, 0, 0, a, b, c), fillet(5, [a / 2, 0, c], [a, b / 2, c], [a / 2, b, c], [0, b / 2, c]), cyl([20, -5, 17], [0, 1, 0], 3, 40, "cut")],
         {"status": "ok"})
    case("boss_on_vertex_blend", "fillet_then_boolean",
         "box_all_edges (r 5) then a boss r 3 standing on the vertex blend at the corner (40, 30, 20), "
         "along the corner's diagonal.",
         "A boolean against the sphere-octant vertex blend, near its three boundary curves.",
         [box(0, 0, 0, a, b, c), fillet(5, *box_edges(a, b, c)),
          cyl([35 + 5 / math.sqrt(3) * 0.5, 25 + 5 / math.sqrt(3) * 0.5, 15 + 5 / math.sqrt(3) * 0.5],
              [1, 1, 1], 1.5, 10, "add")],
         {"status": "ok"})
    case("blend_then_refillet", "fillet_then_boolean",
         "boss_base then a slot cut across the boss and its base blend, then the slot's top edges "
         "rounded r 1.",
         "Blend, boolean, blend: the second fillet's edges lie on the first fillet's torus.",
         [box(0, 0, 0, 60, 60, 10), cyl([30, 30, 10], [0, 0, 1], 8, 10, "add"), fillet(2, rim(30, 30, 8, 10)),
          box(28, 15, 6, 4, 30, 20, "cut"), fillet(1, [28, 30, 20], [32, 30, 20])],
         {"status": "ok_or_reject"})
    return out


def main():
    data = {"version": 1, "cases": cases()}
    path = os.path.join(HERE, "fillet.json")
    with open(path, "w") as f:
        json.dump(data, f, indent=1)
    print(f"wrote {len(data['cases'])} cases to {path}")


if __name__ == "__main__":
    main()
