#!/usr/bin/env python3
"""Writes the hand-made STEP repros of scale.md's causes: tiny solids, every entity written here, none taken from a
dataset. `mk.py DIR` writes them all; each is checked by `check.sh` against OCCT and our reader.

A repro is a solid of revolution about z, given as its profile's faces from the bottom up, so one helper covers
cones, tori and spheres: a tube (outer and inner cylinder, a plane below) whose top face stands on the surface under
test, bounded by circles on it.
"""
import math
import os
import sys


class Step:
    def __init__(self):
        self.lines = []

    def add(self, s):
        self.lines.append(s)
        return f"#{len(self.lines)}"

    def pt(self, p):
        return self.add("CARTESIAN_POINT('',(%r,%r,%r))" % tuple(float(x) for x in p))

    def dir(self, d):
        return self.add("DIRECTION('',(%r,%r,%r))" % tuple(float(x) for x in d))

    def ax(self, o, z=(0, 0, 1), x=(1, 0, 0)):
        return self.add(f"AXIS2_PLACEMENT_3D('',{self.pt(o)},{self.dir(z)},{self.dir(x)})")

    def circle(self, z, r):
        """A circle about the z axis at height z, its vertex at +x, the one edge's loop."""
        c = self.add(f"CIRCLE('',{self.ax((0, 0, z))},{float(r)!r})")
        v = self.add(f"VERTEX_POINT('',{self.pt((r, 0, z))})")
        return self.add(f"EDGE_CURVE('',{v},{v},{c},.T.)")

    def loop(self, edge, same, outer=False):
        oe = self.add(f"ORIENTED_EDGE('',*,*,{edge},{'.T.' if same else '.F.'})")
        el = self.add(f"EDGE_LOOP('',({oe}))")
        return self.add(f"{'FACE_OUTER_BOUND' if outer else 'FACE_BOUND'}('',{el},.T.)")

    def face(self, bounds, surf, same=True):
        return self.add(f"ADVANCED_FACE('',({','.join(bounds)}),{surf},{'.T.' if same else '.F.'})")

    def solid(self, faces, name="repro"):
        sh = self.add(f"CLOSED_SHELL('',({','.join(faces)}))")
        return self.add(f"MANIFOLD_SOLID_BREP('{name}',{sh})")

    def text(self, solid, name):
        ctx = (
            "( GEOMETRIC_REPRESENTATION_CONTEXT(3) GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT(({u})) "
            "GLOBAL_UNIT_ASSIGNED_CONTEXT(({l},{a},{s})) REPRESENTATION_CONTEXT('',''))"
        )
        l = self.add("( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) )")
        a = self.add("( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) )")
        s = self.add("( NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT() )")
        u = self.add(f"UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.E-05),{l},'distance_accuracy_value','')")
        c = self.add(ctx.format(u=u, l=l, a=a, s=s))
        app = self.add("APPLICATION_CONTEXT('core data for automotive mechanical design processes')")
        self.add(f"APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2000,{app})")
        mc = self.add(f"PRODUCT_CONTEXT('',{app},'mechanical')")
        p = self.add(f"PRODUCT('{name}','{name}','',({mc}))")
        pf = self.add(f"PRODUCT_DEFINITION_FORMATION('','',{p})")
        pdc = self.add(f"PRODUCT_DEFINITION_CONTEXT('part definition',{app},'design')")
        pd = self.add(f"PRODUCT_DEFINITION('design','',{pf},{pdc})")
        pds = self.add(f"PRODUCT_DEFINITION_SHAPE('','',{pd})")
        r = self.add(f"ADVANCED_BREP_SHAPE_REPRESENTATION('',({self.ax((0, 0, 0))},{solid}),{c})")
        self.add(f"SHAPE_DEFINITION_REPRESENTATION({pds},{r})")
        body = "\n".join(f"#{i + 1}={x};" for i, x in enumerate(self.lines))
        return (
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('forgecad repro'),'2;1');\n"
            f"FILE_NAME('{name}.step','2026-11-23T00:00:00',(''),(''),'','forgecad research/step/repro/mk.py','');\n"
            "FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }'));\nENDSEC;\nDATA;\n"
            f"{body}\nENDSEC;\nEND-ISO-10303-21;\n"
        )


def tube(name, top, r_in=4.0, r_out=10.0, h=10.0):
    """A tube r_in..r_out, z from 0 to its top, whose top face stands on the surface `top(st)` makes, which returns
    (surface, z at r_in, z at r_out, face sense)."""
    st = Step()
    surf, z_in, z_out, sense = top(st)
    b_out, b_in = st.circle(0, r_out), st.circle(0, r_in)
    t_out, t_in = st.circle(z_out, r_out), st.circle(z_in, r_in)
    plane = st.add(f"PLANE('',{st.ax((0, 0, 0))})")
    cyl_o = st.add(f"CYLINDRICAL_SURFACE('',{st.ax((0, 0, 0))},{r_out!r})")
    cyl_i = st.add(f"CYLINDRICAL_SURFACE('',{st.ax((0, 0, 0))},{r_in!r})")
    # circles run counter-clockwise seen from +z; a face's outer loop runs counter-clockwise about its normal
    faces = [
        st.face([st.loop(b_out, False, True), st.loop(b_in, True)], plane, False),
        st.face([st.loop(b_out, True, True), st.loop(t_out, False)], cyl_o, True),
        st.face([st.loop(b_in, False, True), st.loop(t_in, True)], cyl_i, False),
        st.face([st.loop(t_out, True, True), st.loop(t_in, False)], surf, sense),
    ]
    return st.text(st.solid(faces), name)


def near_flat_cone(st):
    """A cone of half angle a hair under 90 degrees: nearly a plane, as some CATIA and Parasolid files write a
    face's flat ring (00012071 of ABC: 1.5707956859 rad)."""
    a = 1.5707956859
    z0, r0 = 10.0, 10.0
    surf = st.add(f"CONICAL_SURFACE('',{st.ax((0, 0, z0))},{r0!r},{a!r})")
    # a point at radius r lies at z0 + (r - r0) / tan(a)
    zr = lambda r: z0 + (r - r0) / math.tan(a)
    return surf, zr(4.0), zr(10.0), True


def whole_torus():
    """A torus whose one face has no loops: a ring, closed both ways (CADFS 00000550 and others)."""
    st = Step()
    t = st.add(f"TOROIDAL_SURFACE('',{st.ax((0, 0, 0))},10.0,3.0)")
    f = st.add(f"ADVANCED_FACE('',(),{t},.T.)")
    return st.text(st.solid([f]), "whole_torus")


def lemon_ring(st):
    """The top face on a degenerate torus (major radius under the minor, the inner part: a lemon), a ring of it, as
    Onshape writes a near-spherical cap (CADFS 00000723: R 1e-4, r 0.01 m). Here R 0.1, r 12, centre at z 0."""
    R, r = 0.1, 12.0
    surf = st.add(f"DEGENERATE_TOROIDAL_SURFACE('',{st.ax((0, 0, 0))},{R!r},{r!r},.F.)")
    # the lemon: points at distance r from the circle of radius R, on the far side of the axis: rho = r cos v - R
    z = lambda rho: math.sqrt(r * r - (rho + R) ** 2)
    return surf, z(4.0), z(10.0), True


def elbow(name="elbow", turn=90.0, R=63.5, r=27.94, inner=None):
    """A pipe elbow: a torus face between two of its meridian circles, `turn` degrees apart, capped by discs
    (a ring if `inner`, then also the inner torus). As CADFS 00022673's bent tube (R 0.0635, r 0.02794 m)."""
    st = Step()
    a = math.radians(turn)

    def meridian(u, rr):
        c, s_ = math.cos(u), math.sin(u)
        centre = (R * c, R * s_, 0.0)
        # the circle's plane holds z and the radial direction; its normal is the torus's direction of travel
        ax = st.ax(centre, (-s_, c, 0.0), (c, s_, 0.0))
        circ = st.add(f"CIRCLE('',{ax},{rr!r})")
        v = st.add(f"VERTEX_POINT('',{st.pt((centre[0] + rr * c, centre[1] + rr * s_, 0.0))})")
        return st.add(f"EDGE_CURVE('',{v},{v},{circ},.T.)")

    def cap(u, edges_senses, flip):
        c, s_ = math.cos(u), math.sin(u)
        n = (-s_, c, 0.0) if not flip else (s_, -c, 0.0)
        pl = st.add(f"PLANE('',{st.ax((R * c, R * s_, 0.0), n, (c, s_, 0.0))})")
        bounds = [st.loop(e, sg, i == 0) for i, (e, sg) in enumerate(edges_senses)]
        return st.face(bounds, pl, True)

    e0, e1 = meridian(0.0, r), meridian(a, r)
    tor = st.add(f"TOROIDAL_SURFACE('',{st.ax((0, 0, 0))},{R!r},{r!r})")
    faces = [st.face([st.loop(e0, True, True), st.loop(e1, False)], tor, True)]
    if inner:
        i0, i1 = meridian(0.0, inner), meridian(a, inner)
        tor_i = st.add(f"TOROIDAL_SURFACE('',{st.ax((0, 0, 0))},{R!r},{inner!r})")
        faces.append(st.face([st.loop(i0, False, True), st.loop(i1, True)], tor_i, False))
        faces += [cap(0.0, [(e0, False), (i0, True)], True), cap(a, [(e1, True), (i1, False)], False)]
    else:
        faces += [cap(0.0, [(e0, False)], True), cap(a, [(e1, True)], False)]
    return st.text(st.solid(faces), name)


def seamed_cylinder(name, seam="bspline", r=10.0, h=20.0):
    """A plain cylinder whose side is one loop: bottom rim, seam up, top rim back, seam down, the seam one edge used
    both ways, as Parasolid and Onshape write it (AutoMate parts). The seam a degree-1 B-spline (`bspline`, as
    those files have it) or a LINE (`line`)."""
    st = Step()
    bot, top = st.circle(0, r), st.circle(h, r)
    # the circles' vertices: the second entity each circle() made after its CIRCLE
    vb = st.lines.index(next(l for l in st.lines if l.startswith("VERTEX_POINT"))) + 1
    vt = [i + 1 for i, l in enumerate(st.lines) if l.startswith("VERTEX_POINT")][1]
    if seam == "line":
        d = st.add(f"VECTOR('',{st.dir((0, 0, 1))},{h!r})")
        c = st.add(f"LINE('',{st.pt((r, 0, 0))},{d})")
    else:
        c = st.add(f"B_SPLINE_CURVE_WITH_KNOTS('',1,({st.pt((r, 0, 0))},{st.pt((r, 0, h))}),.UNSPECIFIED.,.F.,.U.,(2,2),(0.,{h!r}),.PIECEWISE_BEZIER_KNOTS.)")
    se = st.add(f"EDGE_CURVE('',#{vb},#{vt},{c},.T.)")
    oes = [st.add(f"ORIENTED_EDGE('',*,*,{e},{sg})") for e, sg in ((bot, ".T."), (se, ".T."), (top, ".F."), (se, ".F."))]
    el = st.add(f"EDGE_LOOP('',({','.join(oes)}))")
    cyl = st.add(f"CYLINDRICAL_SURFACE('',{st.ax((0, 0, 0))},{r!r})")
    side = st.add(f"ADVANCED_FACE('',({st.add(f'FACE_OUTER_BOUND({chr(39)}{chr(39)},{el},.T.)')}),{cyl},.T.)")
    pb = st.add(f"PLANE('',{st.ax((0, 0, 0))})")
    pt = st.add(f"PLANE('',{st.ax((0, 0, h))})")
    faces = [side, st.face([st.loop(bot, False, True)], pb, False), st.face([st.loop(top, True, True)], pt, True)]
    return st.text(st.solid(faces), name)


def split_rim_cylinder(name, rim="bspline", sense=False, r=10.0, h=20.0):
    """seamed_cylinder with its top rim in two edges, A to B and B to A, on one closed curve: a CIRCLE (`circle`) or
    an exact rational B-spline circle (`bspline`), each edge with EDGE_CURVE sense `sense`. As AutoMate parts write a
    cross hole's rim (two edges on one closed B_SPLINE_CURVE, both .F.)."""
    st = Step()
    bot = st.circle(0, r)
    vb = [i + 1 for i, l in enumerate(st.lines) if l.startswith("VERTEX_POINT")][0]
    va = st.add(f"VERTEX_POINT('',{st.pt((r, 0, h))})")
    vc = st.add(f"VERTEX_POINT('',{st.pt((-r, 0, h))})")
    if rim == "circle":
        # sense .F. runs the curve backwards: clockwise seen from +z
        c = st.add(f"CIRCLE('',{st.ax((0, 0, h))},{r!r})")
    else:
        w = math.sqrt(0.5)
        P = [(r, 0), (r, r), (0, r), (-r, r), (-r, 0), (-r, -r), (0, -r), (r, -r), (r, 0)]
        ws = [1, w, 1, w, 1, w, 1, w, 1]
        pts = ",".join(st.pt((x, y, h)) for x, y in P)
        c = st.add(
            "( BOUNDED_CURVE() B_SPLINE_CURVE(2,(%s),.CIRCULAR_ARC.,.T.,.F.) B_SPLINE_CURVE_WITH_KNOTS((3,2,2,2,3),"
            "(0.,0.25,0.5,0.75,1.),.UNSPECIFIED.) CURVE() GEOMETRIC_REPRESENTATION_ITEM() RATIONAL_B_SPLINE_CURVE((%s)) "
            "REPRESENTATION_ITEM('') )" % (pts, ",".join("%r" % float(x) for x in ws))
        )
    sg = ".T." if sense else ".F."
    # with the curve run backwards (.F.), A to C goes clockwise; the loop walks the top rim clockwise seen from +z
    e1 = st.add(f"EDGE_CURVE('',{va if sense else vc},{vc if sense else va},{c},{sg})")
    e2 = st.add(f"EDGE_CURVE('',{vc if sense else va},{va if sense else vc},{c},{sg})")
    se = st.add(f"EDGE_CURVE('',#{vb},{va},{st.add(f'LINE({chr(39)}{chr(39)},{st.pt((r, 0, 0))},{st.add(f"VECTOR({chr(39)}{chr(39)},{st.dir((0, 0, 1))},{h!r})")})')},.T.)")
    # bottom rim counter-clockwise, seam up, top rim clockwise (A..C..A), seam down
    top = [(e2, ".F."), (e1, ".F.")] if sense else [(e1, ".T."), (e2, ".T.")]
    if not sense:
        # e1 runs C to A along the reversed curve; the loop needs A to C first
        top = [(e2, ".T."), (e1, ".T.")]
    oes = [st.add(f"ORIENTED_EDGE('',*,*,{e},{o})") for e, o in [(bot, ".T."), (se, ".T.")] + top + [(se, ".F.")]]
    el = st.add(f"EDGE_LOOP('',({','.join(oes)}))")
    cyl = st.add(f"CYLINDRICAL_SURFACE('',{st.ax((0, 0, 0))},{r!r})")
    side = st.add(f"ADVANCED_FACE('',({st.add(f'FACE_OUTER_BOUND({chr(39)}{chr(39)},{el},.T.)')}),{cyl},.T.)")
    pb = st.add(f"PLANE('',{st.ax((0, 0, 0))})")
    pt = st.add(f"PLANE('',{st.ax((0, 0, h))})")
    # the top disc: the rim counter-clockwise seen from +z, so each edge the other way from the side's use
    tl = [st.add(f"ORIENTED_EDGE('',*,*,{e},{'.F.' if o == '.T.' else '.T.'})") for e, o in reversed(top)]
    tel = st.add(f"EDGE_LOOP('',({','.join(tl)}))")
    topf = st.add(f"ADVANCED_FACE('',({st.add(f'FACE_OUTER_BOUND({chr(39)}{chr(39)},{tel},.T.)')}),{pt},.T.)")
    faces = [side, st.face([st.loop(bot, False, True)], pb, False), topf]
    return st.text(st.solid(faces), name)


def main(out):
    os.makedirs(out, exist_ok=True)
    made = {
        "near_flat_cone": tube("near_flat_cone", near_flat_cone),
        "whole_torus": whole_torus(),
        "lemon_ring": tube("lemon_ring", lemon_ring),
        "elbow": elbow(),
        "elbow_tube": elbow("elbow_tube", inner=19.05),
        "seamed_cylinder": seamed_cylinder("seamed_cylinder"),
        "seamed_cylinder_line": seamed_cylinder("seamed_cylinder_line", "line"),
        "split_rim_circle_t": split_rim_cylinder("split_rim_circle_t", "circle", True),
        "split_rim_circle_f": split_rim_cylinder("split_rim_circle_f", "circle", False),
        "split_rim_bspline_t": split_rim_cylinder("split_rim_bspline_t", "bspline", True),
        "split_rim_bspline_f": split_rim_cylinder("split_rim_bspline_f", "bspline", False),
    }
    for k, v in made.items():
        with open(os.path.join(out, k + ".step"), "w") as f:
            f.write(v)
        print(k)


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else os.path.dirname(os.path.abspath(__file__)))
