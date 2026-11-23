//! Kernel regressions on single parts, as Forge parts: a smooth loft through a cone's other nappe,
//! surface intersections that must not jump, a rounded blade on a spline hub, a face's fit past
//! its surface's domain, a narrow tip of an intersection curve, a round touching a flush torus,
//! curves in sketches, fillets past slight kinks, topology that must not follow ids, and real
//! parts that must mesh.
mod common;
mod kernel_util;

use forge_bool::{Index, Op, Where, boolean};
use forge_math::{Point3, Vec3, dmath};
use forge_topo::Body;
use kernel_util::built;

/// The part's body and its exact volume; it builds valid.
fn body(src: &str) -> (Body, f64) {
    let b = built(src);
    let v = b.volume();
    (b.body, v)
}

// ---- a smooth loft through a cone's apex (was forge-agent/tests/cone_other_nappe.rs): the
// cone's equation holds its other nappe too, and the wall's curve on that nappe is not on the cone.

const CONE_BASE: &str = "sketch s1(on: xz) {
  ax = line((0, 0), (0, 1), construction: true)
  l1 = line((0, -20), (60, -20))
  l2 = line((60, -20), (60, 2))
  l3 = line((60, 2), (0, 14))
  l4 = line((0, 14), (0, -20))
}
base = revolve(s1, axis: s1.ax)
";

/// A blade lofted through three copies of a section, at heights 0, 25 and 50, turned 0, half
/// `twist` and `twist` about z.
fn twisted_blade(section: &str, twist: f64) -> String {
    let mut src = String::new();
    for (k, (z, deg)) in [(0.0, 0.0), (25.0, 0.5 * twist), (50.0, twist)]
        .into_iter()
        .enumerate()
    {
        let (s, c) = dmath::sin_cos(deg.to_radians());
        src += &format!(
            "sketch x{k}(on: xy) {{
  p = pt(0, 0)
  q = pt({c:?}, {s:?})
}}
sketch p{k}(on: offset(plane(at: x{k}.p, normal: Z, x: axis(x{k}.p, x{k}.q)), {z:?})) {{
  {section}
}}
"
        );
    }
    src + "blade = loft([p0, p1, p2], smooth: true)\n"
}

/// Where an edge of `b` running up from z = 0 is at height z, for the four that do.
fn corners(b: &Body, z: f64) -> Vec<Point3> {
    let mut out = Vec::new();
    for e in b.edge_ids() {
        let pts = b.edge_points(e, 400);
        let (zs, ze) = (pts[0].z, pts[pts.len() - 1].z);
        if zs.min(ze) > z || zs.max(ze) < z || (zs - ze).abs() < 1.0 {
            continue;
        }
        let i = pts
            .windows(2)
            .position(|w| w[0].z.min(w[1].z) <= z && w[0].z.max(w[1].z) >= z);
        let w = &pts[i.expect("it crosses z")..];
        out.push(w[0].lerp(w[1], (z - w[0].z) / (w[1].z - w[0].z)));
    }
    // round the section from the first
    let f = out[0];
    let turn = |p: &Point3| {
        dmath::atan2(f.x * p.y - f.y * p.x, f.x * p.x + f.y * p.y).rem_euclid(std::f64::consts::TAU)
    };
    out.sort_by(|p, q| turn(p).total_cmp(&turn(q)));
    out
}

/// The blade's volume inside the cone, by slices: its walls are straight across each height.
fn inside_cone(b: &Body) -> f64 {
    let (nz, nu, nv) = (60, 300, 300);
    let dz = 15.0 / nz as f64;
    let mut v = 0.0;
    for k in 0..nz {
        let z = (k as f64 + 0.5) * dz;
        let c = corners(b, z);
        assert_eq!(c.len(), 4);
        let at = |s: f64, t: f64| {
            let (p, q) = (c[0].lerp(c[1], s), c[3].lerp(c[2], s));
            p.lerp(q, t)
        };
        for i in 0..nu {
            for j in 0..nv {
                let (s, t) = ((i as f64 + 0.5) / nu as f64, (j as f64 + 0.5) / nv as f64);
                let p = at(s, t);
                let h = 1e-4;
                let (a, b) = (at(s + h, t) - at(s - h, t), at(s, t + h) - at(s, t - h));
                let area = (a.x * b.y - a.y * b.x).abs() / (4.0 * h * h);
                if z < 14.0 - 0.2 * (p.x * p.x + p.y * p.y).sqrt() {
                    v += area * dz / (nu * nv) as f64;
                }
            }
        }
    }
    v
}

#[test]
fn a_twisted_wall_cuts_only_the_cones_own_nappe() {
    let base = dmath::PI * 3600.0 * (22.0 + 4.0);
    for twist in [15.0, 30.0, 60.0] {
        let loft = twisted_blade("r = rect(corner: (-25, -2), size: (50, 4))", twist);
        let alone = built(&loft);
        let blade_volume = alone.volume();
        let cut = inside_cone(&alone.body);
        let b = built(&format!("{CONE_BASE}{loft}"));
        let problems = forge_topo::check(&b.body);
        assert!(problems.is_empty(), "{twist}: {problems:?}");
        let volume = b.volume();
        let want = base + blade_volume - cut;
        assert!((volume - want).abs() < 2.0, "{twist}: {volume} against {want}");
    }
}

#[test]
fn rounded_sections_too() {
    for twist in [15.0, 30.0, 60.0] {
        let section = "r = hull(disc((-23.5, -0.5), 1.5), disc((23.5, -0.5), 1.5), disc((23.5, 0.5), 1.5), disc((-23.5, 0.5), 1.5))";
        let b = built(&format!("{CONE_BASE}{}", twisted_blade(section, twist)));
        let problems = forge_topo::check(&b.body);
        assert!(problems.is_empty(), "{twist}: {problems:?}");
        assert!(b.volume() > 3e5, "{twist}");
    }
}

// ---- a tube square to a torus's axis, the axes not meeting (was forge-agent/tests/meet_no_jump.rs).
// The roots of a mirror pair meet at one u, and the discriminant lies flat there: the closed form
// must not link a root to its mirror image, a chain on both surfaces at every point but jumping
// across the torus between two of them.

#[test]
fn a_tube_square_to_a_torus_axis_has_no_chain_across_the_torus() {
    use forge_geom::intersect::SurfaceSurface;
    use forge_geom::{Curve, Cylinder, Surface, Torus, meet};
    use forge_math::{Box3, Frame};
    let torus = Surface::Torus(
        Torus::new(
            Frame::from_axes(Point3::new(150.5, 20.0, -13.0), Vec3::Z, -Vec3::Y, Vec3::X).unwrap(),
            5.5,
            2.5,
        )
        .unwrap(),
    );
    // (the rounding in the axes is what puts the event on the wrong side)
    let tube = Surface::Cylinder(Cylinder {
        frame: Frame::from_axes(
            Point3::new(143.0, -18.0, -12.000000000000002),
            Vec3::X,
            Vec3::new(0.0, 6.123233995736766e-17, -1.0),
            Vec3::new(0.0, 1.0, 6.123233995736766e-17),
        )
        .unwrap(),
        radius: 8.25,
    });
    let region = Box3::from_points([
        Point3::new(142.49999, 11.99999, -20.25002000000001),
        Point3::new(151.25002000000006, 14.500020000000008, -4.99999),
    ]);
    // the tracer's to answer if the closed form will not
    let Some(SurfaceSurface::Curves(cs)) = meet::intersect(&tube, &torus, region) else {
        return;
    };
    for (c, _) in &cs {
        let Curve::Intersection(ic) = c else {
            panic!("a chain")
        };
        for w in ic.pts.windows(2) {
            assert!(w[0].distance(w[1]) < 3.0, "{:?} to {:?}", w[0], w[1]);
        }
    }
}

/// A cylinder lying across a torus's flat top: going round the cylinder the root count runs 0, 4, 2
/// in a band a few hundredths of a turn wide, which the quarter-point count checks do not see. The
/// two outer roots must not be joined as a turn over the stretch of curve between them.
#[test]
fn a_band_of_roots_too_thin_to_see_does_not_lose_its_stretch_of_curve() {
    use forge_geom::intersect::SurfaceSurface;
    use forge_geom::meet::intersect;
    use forge_geom::{Curve, Cylinder, Surface, Torus};
    use forge_math::{Box3, Frame};
    let torus = Surface::Torus(Torus {
        frame: Frame::from_axes(Point3::ORIGIN, Vec3::X, -Vec3::Z, Vec3::Y).unwrap(),
        major: 56.0,
        minor: 8.0,
    });
    let cylinder = Surface::Cylinder(Cylinder {
        frame: Frame::from_axes(Point3::new(0.0, -4.0, -64.35), Vec3::Z, Vec3::Y, -Vec3::X).unwrap(),
        radius: 11.0,
    });
    let region = Box3::from_points([Point3::new(-40.0, -8.0, -70.0), Point3::new(40.0, 7.0, -40.0)]);
    let step = 0.5 * region.diagonal() / 64.0;
    if let Some(SurfaceSurface::Curves(curves)) = intersect(&torus, &cylinder, region) {
        for (c, _) in &curves {
            let Curve::Intersection(c) = c else { continue };
            let n = c.pts.len();
            for i in 0..c.segments() {
                let (p, q) = (c.pts[i], c.pts[(i + 1) % n]);
                if region.contains(p) || region.contains(q) {
                    assert!(p.distance(q) <= 3.0 * step, "{p:?} {q:?}");
                }
            }
        }
    }
}

// ---- a lofted blade, its long edges rounded, joined to a hub revolved from a spline (was
// forge-agent/tests/rounded_blade_on_spline_hub.rs): every curve where they meet ends on one of
// the round's springs, and a spring is not in a plane.

/// Two rectangles turned apart on a stack leaning out from the axis (at 292 degrees round it), the
/// four long walls' edges rounded; a point of the blade clear of the hub.
fn rounded_blade() -> (&'static str, Point3) {
    let az = 292f64;
    let (s, c) = dmath::sin_cos(az.to_radians());
    let (st, ct) = dmath::sin_cos(36f64.to_radians());
    let origin = Vec3::new(37.0 * c, 37.0 * s, 30.0);
    let z = Vec3::new(c * st, s * st, ct);
    let p = origin + z * 15.0;
    (
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch d1(on: offset(xy, 17.864745084375787)) {
  p = pt(10.557620491067807, -26.131027680800432)
}
sketch d2(on: offset(xy, 0.8090169943749475)) {
  p = pt(0.22018823102139573, -0.5449849958780468)
}
sketch d3(on: offset(xy, -0.5877852522924731)) {
  p = pt(0.30306310027837907, -0.7501074952546007)
}
sketch s4(on: plane(at: d1.p, normal: axis(o.p, d2.p), x: axis(o.p, d3.p))) {
  r1 = rect(corner: (-22, -2.5), size: (44, 5))
}
sketch d5(on: offset(xy, 46.989356881873896)) {
  p = pt(18.484396807838053, -45.75048753241012)
}
sketch d6(on: offset(xy, -0.393305102275257)) {
  p = pt(0.8918206798417602, -0.22353293164728627)
}
sketch s7(on: plane(at: d5.p, normal: axis(o.p, d2.p), x: axis(o.p, d6.p))) {
  r1 = rect(corner: (-22, -2.5), size: (44, 5))
}
blade = loft([s4, s7], smooth: true)
round = fillet(edges(between: (blade.side(s4.r1[0]), blade.side(s4.r1[1]))) + edges(between: (blade.side(s4.r1[1]), blade.side(s4.r1[2]))) + edges(between: (blade.side(s4.r1[2]), blade.side(s4.r1[3]))) + edges(between: (blade.side(s4.r1[3]), blade.side(s4.r1[0]))), 1.5)
"#,
        Point3::new(p.x, p.y, p.z),
    )
}

#[test]
fn a_rounded_blade_stands_on_a_spline_hub() {
    let (h, vh) = body(
        r#"sketch sec(on: xz) {
  s = spline([(55.0, 10.0), (47.19638711935487, 10.614871027096626), (39.69266270539641, 12.435854959638824), (32.777190679215906, 15.392972406318552), (26.715728752538098, 19.37258300203048), (21.74121550789819, 24.22175254337273), (18.04481869954853, 29.754130164317125), (15.768588783870783, 35.7571096954839), (15.0, 42.0)], cv: true, closed: false)
  path rest(start: (15, 42)) {
    line(to: (15, 50))
    line(to: (0, 50))
    line(to: (0, 0))
    line(to: (55, 0))
    line(to: (55, 10))
  }
  ax = line((0, 0), (0, 1), construction: true)
}
hub = revolve(sec, axis: sec.ax)
"#,
    );
    let (src, p) = rounded_blade();
    let (b, vb) = body(src);
    let u = boolean(&h, &b, Op::Union, 11).expect("the union");
    let problems = forge_topo::check(&u);
    assert!(problems.is_empty(), "{problems:?}");
    let vu = forge_topo::mass::mass(&u).expect("mass").volume;
    assert!(
        vu > vh + 0.2 * vb && vu < vh + vb,
        "the blade has gone: {vu}, hub {vh}, blade {vb}"
    );
    // a point of the blade clear of the hub is in the union
    assert_eq!(Index::new(&b).classify(p), Where::Inside);
    assert_eq!(Index::new(&u).classify(p), Where::Inside);
}

// ---- a face may run past the domain its procedural surface states, onto the part carried on
// beyond it (a mitre takes a loft's wall there): its fit must cover it, or the file's edges lie
// off the fit (was forge-agent/tests/fit_past_domain.rs).

#[test]
fn a_faces_fit_box_covers_it_past_the_stated_domain() {
    use forge_geom::{Curve, Line, SurfaceGeom};
    use forge_math::{Interval, Point2};
    use forge_topo::Name;
    let loft = built(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Z, x: X)) {
  r1 = rect(corner: (-10, -5), size: (20, 10))
}
sketch d2(on: offset(xy, 20)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {
  r1 = rect(corner: (-10, -5), size: (20, 10))
}
sketch d4(on: offset(xy, 45)) {
  p = pt(0, 0)
}
sketch s5(on: plane(at: d4.p, normal: Z, x: X)) {
  r1 = rect(corner: (-10, -5), size: (20, 10))
}
loft1 = loft([s1, s3, s5], smooth: true)
"#,
    )
    .body;
    let wall = loft
        .face_ids()
        .find(|&g| matches!(loft.face(g).surface, forge_geom::Surface::Procedural(_)))
        .expect("a loft wall");
    let surface = loft.face(wall).surface.clone();
    let d = surface.domain();
    assert!(d.v.period.is_none());

    // a patch of the wall from below the domain's start to its middle
    let span = d.v.hi - d.v.lo;
    let (v0, v1) = (d.v.lo - 0.1 * span, d.v.lo + 0.5 * span);
    let (u0, u1) = (
        d.u.lo + 0.25 * (d.u.hi - d.u.lo),
        d.u.lo + 0.75 * (d.u.hi - d.u.lo),
    );
    let corners = [(u0, v0), (u1, v0), (u1, v1), (u0, v1)].map(|(u, v)| surface.eval(Point2::new(u, v)));
    let mut b = Body::new();
    let shell = b.add_shell();
    let face = b.add_face(shell, surface.clone(), true, Name::feature(1));
    let names = |k: u64| Name::feature(100 + k);
    let vs: Vec<_> = corners
        .iter()
        .enumerate()
        .map(|(k, &p): (usize, &Point3)| b.add_vertex(p, names(k as u64)))
        .collect();
    let uses: Vec<_> = (0..4)
        .map(|k| {
            let (p, q) = (corners[k], corners[(k + 1) % 4]);
            let line = Line::new(p, (q - p).normalize().expect("apart"));
            let range = Interval::new(0.0, p.distance(q));
            let e = b.add_edge(
                Curve::Line(line),
                range,
                vs[k],
                vs[(k + 1) % 4],
                names(10 + k as u64),
            );
            (e, true)
        })
        .collect();
    b.add_loop(face, &uses);

    let got = forge_step::write::face_box(&b, face);
    assert!(
        got.v.lo <= v0 + 1e-6,
        "the fit stops at {} short of {v0}",
        got.v.lo
    );
    assert!(
        got.v.hi >= v1 - 1e-6 && got.v.hi <= d.v.hi,
        "{} {}",
        got.v.hi,
        d.v.hi
    );
}

// ---- a hole all but tangent to a rim whose wall goes vertical at the top (was
// forge-agent/tests/narrow_tip.rs): the curve where they meet climbs to a tip far narrower than
// the marching step. Judged against slices, each a disk less a lens.

/// How much of a disk radius `a` a circle radius `r` at `c` from its centre covers.
fn lens(a: f64, r: f64, c: f64) -> f64 {
    if c >= a + r {
        return 0.0;
    }
    if c + r <= a {
        return dmath::PI * r * r;
    }
    let u = dmath::acos((c * c + a * a - r * r) / (2.0 * c * a));
    let v = dmath::acos((c * c + r * r - a * a) / (2.0 * c * r));
    let k = (-c + a + r) * (c + a - r) * (c - a + r) * (c + a + r);
    a * a * u + r * r * v - 0.5 * k.max(0.0).sqrt()
}

/// The volume by slices, spaced closely towards the top where the wall is steep.
fn sliced(rim: f64, big: f64, r: f64, c: f64) -> f64 {
    let n = 400_000;
    let (mut v, h) = (0.0, 24.0);
    for k in 0..n {
        // z = 40 - h t^3, t running from 1 to 0
        let t = (k as f64 + 0.5) / n as f64;
        let d = h * t * t * t;
        let dz = h * 3.0 * t * t / n as f64;
        let a = rim - big + (big * big - d * d).sqrt();
        v += (dmath::PI * a * a - lens(a, r, c)) * dz;
    }
    v
}

#[test]
fn a_hole_all_but_tangent_to_a_vertical_rim_has_the_volume_of_its_slices() {
    let (rim, big, r) = (42.0, 30.0, 2.5);
    let foot = rim - big + (big * big - 24.0f64 * 24.0).sqrt();
    for gap in [1e-3, 1e-4, 3e-5, 1e-5] {
        for turn in [0.0, 37.0, 100.0, 200.0, 300.0] {
            let c = rim - gap - r;
            let (s, co) = dmath::sin_cos(f64::to_radians(turn));
            let src = format!(
                "sketch s1(on: xz) {{
  ax = line((0, 0), (0, 1), construction: true)
  l1 = line((0, 16), ({foot:?}, 16))
  a2 = arc(center: ({:?}, 40), from: ({foot:?}, 16), to: ({rim:?}, 40))
  l3 = line(({rim:?}, 40), (0, 40))
  l4 = line((0, 40), (0, 16))
}}
revolve1 = revolve(s1, axis: s1.ax)
sketch d2(on: offset(xy, 14)) {{
  p = pt(0, 0)
}}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {{
  c1 = circle(({:?}, {:?}), {r:?})
}}
cut1 = cut(s3, -30)
",
                rim - big,
                c * co,
                c * s
            );
            let b = built(&src);
            let problems = forge_topo::check(&b.body);
            assert!(problems.is_empty(), "{gap} {turn}: {problems:?}");
            let p = forge_topo::mass::mass(&b.body).expect("mass");
            let want = sliced(rim, big, r, c);
            assert!(
                (p.volume - want).abs() < 1e-2 && (p.volume - want).abs() <= p.err[0] + 1e-3,
                "{gap} {turn}: {} against {want}, reported {}",
                p.volume,
                p.err[0]
            );
        }
    }
}

// ---- a rounded boss on a part of a revolve, joined to the whole revolve (was
// forge-agent/tests/blend_on_flush_torus.rs): the part's curved face lies on the whole's, and the
// round only touches that surface, along its rim.

const TORUS_PROFILE: &str = "  ax = line((0, 0), (0, 1), construction: true)
  l1 = line((0, 30), (0, -100))
  l2 = line((0, -100), (50, -100))
  a3 = arc(center: (150, -100), from: (50, -100), to: (150, 0), cw: true)
  l4 = line((150, 0), (150, 30))
  l5 = line((150, 30), (0, 30))";

/// The k'th sixth of the ring, turned 120 k degrees about its axis; and that part with a box on
/// its bottom, rounded into it. Its features named for k, so the copies' names differ.
fn torus_copy(k: usize) -> (String, String) {
    let (s, c) = dmath::sin_cos((120.0 * k as f64).to_radians());
    let part = format!(
        "sketch d{k}(on: xy) {{
  p = pt(0, 0)
  n = pt({s:?}, {:?})
  x = pt({c:?}, {s:?})
}}
sketch sec{k}(on: plane(at: d{k}.p, normal: axis(d{k}.p, d{k}.n), x: axis(d{k}.p, d{k}.x))) {{
{TORUS_PROFILE}
}}
wedge{k} = revolve(sec{k}, axis: sec{k}.ax, angle: 60deg)
",
        -c
    );
    let boss = format!(
        "{part}sketch top{k}(on: offset(plane(at: d{k}.p, normal: Z, x: axis(d{k}.p, d{k}.x)), -70)) {{
  r = rect(corner: (95, 45), size: (20, 6))
}}
box{k} = extrude(top{k}, 80)
round{k} = fillet(edges(between: (wedge{k}, box{k}), crease: \"concave\"), 2)
"
    );
    (part, boss)
}

#[test]
fn a_round_touches_the_flush_torus_and_does_not_cut_it() {
    let (ring, whole) = body(&format!(
        "sketch sec(on: xz) {{
{TORUS_PROFILE}
}}
ring = revolve(sec, axis: sec.ax)
"
    ));
    let (_, wedge) = body(&torus_copy(0).0);
    assert!((wedge - whole / 6.0).abs() < 1e-3, "{wedge} {whole}");
    let added = body(&torus_copy(0).1).1 - wedge;
    assert!(added > 1000.0, "{added}");

    let mut out = ring;
    for k in 0..3 {
        // the boss stands out of the torus inside its wedge, away from the wedge's ends
        let boss = body(&torus_copy(k).1).0;
        out = boolean(&out, &boss, Op::Union, 7 + k as u64).expect("the union");
        let problems = forge_topo::check(&out);
        assert!(problems.is_empty(), "copy {k}: {problems:?}");
        let v = forge_topo::mass::mass(&out).expect("mass").volume;
        let want = whole + (k + 1) as f64 * added;
        assert!((v - want).abs() < 1e-2, "copy {k}: {v} against {want}");
    }
}

// ---- curves in sketches (was forge-agent/tests/spline.rs)

fn volume(src: &str) -> f64 {
    let b = built(src);
    assert!(forge_topo::check(&b.body).is_empty());
    b.volume()
}

#[test]
fn loops_of_curves() {
    let v = volume(
        r#"sketch s(on: xy) {
  e = ellipse(center: (0, 0), major: (6.5778483455013586, 2.394141003279681), b: 3)
}
slab = extrude(s, 4)
"#,
    );
    assert!((v - dmath::PI * 21.0 * 4.0).abs() < 1e-6);
    let v = volume(
        r#"sketch s(on: xy) {
  c = spline([(0, 0), (10, 8), (20, 8), (30, 0)], closed: false)
  l = line((30, 0), (0, 0))
}
arch = extrude(s, 5)
"#,
    );
    assert!(v > 0.0);
    // a parabolic arch: 2/3 of the chord times half the control height, 5 high
    let v = volume(
        r#"sketch s(on: xy) {
  c = spline([(0, 0), (5, 8), (10, 0)], cv: true, closed: false)
  l = line((10, 0), (0, 0))
}
arch = extrude(s, 5)
"#,
    );
    assert!((v - 5.0 * (2.0 / 3.0) * 10.0 * 4.0).abs() < 1e-9, "{v}");
}

/// Half an ellipse closed by its major axis. The .fc part:
///   extrude 3 loop ell 0 0 6 2 0 0 180 line -6 0 6 0
#[test]
fn half_an_ellipse() {
    let v = volume(
        "sketch s(on: xy) {
  e = earc(center: (0, 0), major: (6, 0), b: 2, from: (6, 0), to: (-6, 0))
  l = line((-6, 0), (6, 0))
}
half = extrude(s, 3)
",
    );
    assert!((v - 3.0 * 0.5 * dmath::PI * 12.0).abs() < 1e-6);
}

/// A degree-1 B-spline on knots: a square 4 across, as one curve. The .fc part:
///   extrude 2 loop spline 1 0 0 4 0 4 4 0 4 0 0 knots 0 0 1 2 3 4 4
#[test]
fn a_degree_one_spline_on_knots() {
    let v = volume(
        "sketch s(on: xy) {
  c = spline([(0, 0), (4, 0), (4, 4), (0, 4), (0, 0)], cv: true, degree: 1, knots: [0, 0, 1, 2, 3, 4, 4])
}
square = extrude(s, 2)
",
    );
    assert!((v - 32.0).abs() < 1e-9, "{v}");
}

/// Every free curve a .fc loop wrote, the same solid in Forge (each volume the .fc part's):
///   extrude 2 loop spline 2 0 0 5 8 10 0 weights 1 3 1 line 10 0 0 0                       71.303242796
///   extrude 2 loop spline 2 0 0 3 6 7 6 10 0 knots 0 0 0 1 3 3 3 weights 1 2 0.5 1 line 10 0 0 0   70.997473014
///   extrude 2 loop thru 0 0 5 4 10 0 from 1 2 to 1 -2 line 10 0 0 0                          58.575207876
///   extrude 2 loop thru 0 0 5 4 10 0 from 3 0 line 10 0 0 0                                  27.851074305
///   extrude 3 loop ell 0 0 6 2 30 0 -180 line -5.196152422706632 -3 5.196152422706632 3      56.548667765
///   extrude 5 rrect -20 -10 40 20 4                                                         3931.327412287
/// each 4 faces and 6 edges but the rounded rectangle's 10 and 24.
#[test]
fn every_fc_loop_curve() {
    let arch = |curve: &str| {
        format!("sketch s(on: xy) {{\n  c = {curve}\n  l = line((10, 0), (0, 0))\n}}\narch = extrude(s, 2)\n")
    };
    let x = 5.196152422706632;
    for (src, want, faces) in [
        (
            arch("spline([(0, 0), (5, 8), (10, 0)], degree: 2, weights: [1, 3, 1])"),
            71.303242796,
            4,
        ),
        (
            arch(
                "spline([(0, 0), (3, 6), (7, 6), (10, 0)], degree: 2, knots: [0, 0, 0, 1, 3, 3, 3], weights: [1, 2, 0.5, 1])",
            ),
            70.997473014,
            4,
        ),
        (
            arch("spline([(0, 0), (5, 4), (10, 0)], start_slope: (1, 2), end_slope: (1, -2))"),
            58.575207876,
            4,
        ),
        (
            arch("spline([(0, 0), (5, 4), (10, 0)], start_slope: (3, 0))"),
            27.851074305,
            4,
        ),
        (
            format!(
                "sketch s(on: xy) {{
  e = earc(center: (0, 0), major: ({x}, 3), b: 2, from: ({x}, 3), to: (-{x}, -3), cw: true)
  l = line((-{x}, -3), ({x}, 3))
}}
half = extrude(s, 3)
"
            ),
            56.548667765,
            4,
        ),
        (
            "sketch s(on: xy) {
  h = hull(disc((-16, -6), 4), disc((16, -6), 4), disc((16, 6), 4), disc((-16, 6), 4))
}
e = extrude(s, 5)
"
            .to_string(),
            3931.327412287,
            10,
        ),
    ] {
        let b = built(&src);
        assert!(forge_topo::check(&b.body).is_empty(), "{src}");
        let v = b.volume();
        assert!((v - want).abs() < 1e-6 * want, "{src}: {v}, .fc {want}");
        assert_eq!(b.body.compact().faces.len(), faces, "{src}");
    }
}

/// A spline's degree, knots and weights are checked where they are written, each a typed error.
#[test]
fn spline_forms_refused() {
    for (curve, kind) in [
        (
            "spline([(0, 0), (5, 8), (10, 0)], degree: 2, knots: [0, 0, 1, 1])",
            "BadValue",
        ),
        (
            "spline([(0, 0), (5, 8), (10, 0)], degree: 2, knots: [0, 0, 0, 2, 1, 1])",
            "BadValue",
        ),
        ("spline([(0, 0), (5, 8), (10, 0)], degree: 3)", "BadValue"),
        (
            "spline([(0, 0), (5, 8), (10, 0)], weights: [1, 0, 1])",
            "BadValue",
        ),
        ("spline([(0, 0), (5, 8), (10, 0)], weights: [1, 1])", "BadValue"),
        (
            "spline([(0, 0), (5, 8), (10, 0)], degree: 2, closed: true)",
            "NotWith",
        ),
        (
            "spline([(0, 0), (5, 8), (10, 0)], degree: 2, cv: false)",
            "NotWith",
        ),
        (
            "spline([(0, 0), (5, 8), (10, 0)], start_slope: (1, 0), cv: true)",
            "NotWith",
        ),
        (
            "spline([(0, 0), (5, 8), (10, 0)], start_slope: (1, 0), degree: 2)",
            "NotWith",
        ),
        (
            "spline([(0, 0), (5, 8), (10, 0)], start_slope: (0, 0))",
            "BadValue",
        ),
    ] {
        let src = format!(
            "sketch s(on: xy) {{\n  c = {curve}\n  l = line((10, 0), (0, 0))\n}}\na = extrude(s, 2)\n"
        );
        let msg = common::refused(&src);
        assert!(msg.contains(kind), "{curve}: {msg}");
    }
}

/// Loops that cross are refused at the sketch (the .fc part
/// `extrude 5 circle 0 0 40 circle 20 0 30` was the kernel's HoleOutside); loops that touch at a
/// corner go on to the kernel, which says where the solid pinches (as the .fc part
/// `extrude 5 rect 0 0 20 20 poly 10 0 15 10 5 10` did: ZeroThickness at (10, 0)).
#[test]
fn crossing_loops_refused_touching_judged() {
    let msg = common::refused(
        "sketch s(on: xy) {
  c1 = circle((0, 0), 40)
  c2 = circle((20, 0), 30)
}
e = extrude(s, 5)
",
    );
    assert!(
        msg.contains("SketchProfile") && msg.contains("self intersection"),
        "{msg}"
    );
    let (run, _) = common::run(
        "sketch s(on: xy) {
  r = rect(corner: (0, 0), size: (20, 20))
  t = poly([(10, 0), (15, 10), (5, 10)])
}
e = extrude(s, 5)
",
    );
    let e = format!("{:?}", run.errors.first().expect("refused"));
    assert!(
        e.contains("ZeroThickness") && e.contains("x: 10.0, y: 0.0"),
        "{e}"
    );
}

/// Sketches solved before they are extruded: a spline's end held 40 along from its start, an
/// ellipse's minor radius set, a spline tangent to the line it leaves, a B-spline closed by a line.
#[test]
fn sketch_rows_solve_and_extrude() {
    let v = volume(
        "sketch s(on: xy) {
  a = pt(0, 0)
  b = near(30, 0)
  c = spline([a, (10, 8), (20, 8), b], closed: false)
  l = line(b, a)
  horizontal(l)
  distance(a, b, 40)
}
arch = extrude(s, 5)
",
    );
    assert!(v > 0.0);
    let v = volume(
        "sketch s(on: xy) {
  e = ellipse(center: (0, 0), major: (8, 0), b: 2)
}
flat = extrude(s, 1)
",
    );
    assert!((v - dmath::PI * 16.0).abs() < 1e-6, "{v}");
    let v = volume(
        "sketch s(on: xy) {
  l1 = line((0, 0), (10, 0))
  c = spline([l1.b, near(15, 5), (10, 10)], closed: false)
  l2 = line((10, 10), (0, 10))
  l3 = line((0, 10), l1.a)
  tangent(l1, c)
}
lobe = extrude(s, 1)
",
    );
    assert!(v > 100.0);
    let v = volume(
        "sketch s(on: xy) {
  c = spline([(0, 0), (5, 8), (10, 8), (15, 0)], cv: true, closed: false)
  l = line((15, 0), (0, 0))
}
arch = extrude(s, 1)
",
    );
    assert!(v > 0.0);
}

// ---- a fillet on one edge where the rim carries on past a slight kink (was
// forge-agent/tests/fillet_kinks.rs): within half a degree the next edges round with it, as a
// tangent chain; past that the round ends on the next wall.

/// A 30 x 20 x 10 block whose back edge is a line meeting a quarter arc of radius 10, turned
/// `kink` degrees outward where they meet; fillet 2 on the line's top edge only.
fn kinked_block(kink: f64) -> (f64, usize) {
    let t = kink.to_radians();
    let y0 = 20.0 + 20.0 * dmath::sin(t) / dmath::cos(t);
    let b = built(&format!(
        "sketch s(on: xy) {{
  back = line((0, {y0:?}), (20, 20))
  bend = arc(center: (20, 10), from: (20, 20), to: (30, 10), cw: true)
  side = line((30, 10), (30, 0))
  front = line((30, 0), (0, 0))
  left = line((0, 0), (0, {y0:?}))
}}
b = extrude(s, 10)
round = fillet(edges(between: (b.side(s.back), b.end)), 2)
"
    ));
    assert!(forge_topo::check(&b.body).is_empty());
    (b.volume(), b.body.face_ids().count())
}

#[test]
fn a_kink_under_half_a_degree_rounds_on() {
    // OpenCascade's fillet of the same block, which chains these edges too (research/blend-ends)
    let (v, faces) = kinked_block(0.03);
    assert_eq!(faces, 10, "the arc and the line after it round too");
    assert!((v - 5747.809067323634).abs() < 1e-4 * v, "{v}");
}

#[test]
fn a_kink_over_half_a_degree_ends_the_round() {
    let (_, faces) = kinked_block(1.0);
    assert_eq!(faces, 8, "only the picked edge rounds");
}

// ---- a part's topology must not follow its feature ids (was forge-agent/tests/ids_topology.rs):
// the same features under other ids build the same faces, edges and vertices. The frame's axle
// slot once kept the bore's seam where the least name put it: 77 edges or 76 by id.

/// The ids, other ones for each k.
fn other_ids(n: usize, k: u64) -> Vec<u64> {
    (0..n as u64).map(|i| 1 + i * (k + 1) + 7 * k).collect()
}

#[test]
fn the_slot_cut_keeps_one_seam_whatever_the_ids() {
    let b = built(include_str!("kernel_parts/frame_slot_ids.fg"));
    let all: Vec<_> = (0..4)
        .map(|k| {
            let solids = b
                .part()
                .regenerate_solids_ids(Some(&other_ids(b.features.len(), k)))
                .expect("builds");
            let s = &solids[0].body;
            (s.face_ids().count(), s.edge_ids().count(), s.vertex_ids().count())
        })
        .collect();
    assert!(all.iter().all(|c| *c == all[0]), "{all:?}");
    assert_eq!(all[0], (30, 76, 48));
}

/// Two names of one edge (pieces a seam split under other ids, one edge here) in one fillet with
/// one radius: the edge is rounded once, not refused as listed twice.
#[test]
fn a_fillet_naming_one_edge_twice_rounds_it_once() {
    use forge_model::Feature;
    use forge_ops::Radius;
    let b = built(
        "sketch s(on: xy) {
  c = circle((0, 0), 5)
}
rod = extrude(s, 10)
",
    );
    let mut part = b.part();
    let body = b.solids[0].body.clone();
    let rim = body
        .edge_ids()
        .find(|&e| matches!(body.edge(e).curve, forge_geom::Curve::Circle(_)))
        .expect("a rim");
    let n = body.edge(rim).name;
    let r = Radius::Constant(1.0);
    part.features.push(Feature::Fillet {
        edges: vec![(n, r.clone()), (n, r)],
        starts: vec![],
        setbacks: vec![],
        only: false,
    });
    let mut ids = b.ids.clone();
    ids.push(9999);
    let once = part
        .regenerate_solids_ids(Some(&ids))
        .expect("rounds")
        .remove(0)
        .body;
    part.features.pop();
    part.features.push(Feature::Fillet {
        edges: vec![(n, Radius::Constant(1.0))],
        starts: vec![],
        setbacks: vec![],
        only: false,
    });
    let single = part
        .regenerate_solids_ids(Some(&ids))
        .expect("rounds")
        .remove(0)
        .body;
    assert_eq!(once.face_ids().count(), single.face_ids().count());
}

/// A cylinder notched below and whole above: its pieces join into one face, and the seam it keeps
/// is a ruling, under any ids; never an arc left across the face (a slit a fillet along the notch
/// would run into).
#[test]
fn a_rejoined_cylinder_keeps_a_ruling_as_its_seam() {
    let b = built(
        "sketch s1(on: xy) {
  c = circle((0, 0), 6)
}
low = extrude(s1, 3)
sketch s2(on: offset(xy, 3)) {
  c = circle((0, 0), 6)
}
high = extrude(s2, 5)
sketch s3(on: xy) {
  r = rect(corner: (-2, -7), size: (4, 3))
}
notch = cut(s3, -3)
",
    );
    let mut seen = vec![];
    for k in 0..6u64 {
        let ids = other_ids(b.features.len(), k);
        let s = b
            .part()
            .regenerate_solids_ids(Some(&ids))
            .expect("builds")
            .remove(0)
            .body;
        for e in s.edge_ids() {
            let x = s.edge(e);
            let faces: Vec<_> = x.coedges.iter().map(|&c| s.coedge_face(c)).collect();
            if faces.len() == 2 && faces[0] == faces[1] {
                assert!(
                    matches!(x.curve, forge_geom::Curve::Line(_)),
                    "ids {ids:?}: a seam that is no ruling"
                );
            }
        }
        seen.push((s.face_ids().count(), s.edge_ids().count()));
    }
    assert!(seen.iter().all(|s| *s == seen[0]), "{seen:?}");
}

// ---- real parts mesh closed, and within the chord at triangles' centroids and edge midpoints, at
// chords from coarse to fine (was forge-agent/tests/mesh_parts.rs). A crank's and a frame's cone
// faces once failed: unrolled flat, two coarse chords of curves meeting at a sharp corner crossed.

fn meshes(src: &str) {
    use forge_geom::SurfaceGeom;
    use forge_mesh::{Quality, tessellate};
    // the mesh's face indices are the compacted body's
    let b = built(src).body.compact();
    let bb = forge_math::Box3::from_points(b.vertices.iter().map(|v| v.point));
    let diag = (bb.x.width().powi(2) + bb.y.width().powi(2) + bb.z.width().powi(2)).sqrt();
    for chord in [0.25, 4e-4 * diag, 0.05] {
        let q = Quality {
            chord,
            ..Quality::default()
        };
        let mesh = tessellate(&b, &q).unwrap_or_else(|e| panic!("chord {chord}: {e:?}"));
        assert!(mesh.open_edges().is_empty(), "chord {chord}: open");
        assert_eq!(mesh.degenerate_triangles(1e-12), 0, "chord {chord}");
        for (t, &f) in mesh.triangles.iter().zip(&mesh.face_of) {
            let s = &b.face(forge_topo::FaceId(f)).surface;
            let p = t.map(|i| mesh.positions[i as usize]);
            for x in [
                p[0].lerp(p[1], 0.5).lerp(p[2], 1.0 / 3.0),
                p[0].lerp(p[1], 0.5),
                p[1].lerp(p[2], 0.5),
                p[2].lerp(p[0], 0.5),
            ] {
                let d = s.distance(x);
                assert!(d <= chord * (1.0 + 1e-6), "chord {chord}: face {f} {d} off");
            }
        }
    }
}

#[test]
fn crank() {
    meshes(include_str!("kernel_parts/crank.fg"));
}

#[test]
fn frame() {
    meshes(include_str!("kernel_parts/frame.fg"));
}

#[test]
fn balls_at_pole() {
    meshes(include_str!("kernel_parts/balls_at_pole.fg"));
}

/// A hole tangent inside a rim with no vertex there, and rims tangent at a shared vertex: the chords
/// cross at any fineness unless each edge has a sample at the point, or the tighter one stays coarse
/// (WHUCAD 100002382, 100002741).
#[test]
fn circles_touching_inside_a_face() {
    use forge_mesh::{Quality, tessellate};
    for src in [
        r#"sketch d1(on: offset(xy, 0)) {
  p = pt(-1, 0)
}
sketch s2(on: plane(at: d1.p, normal: Z, x: X)) {
  c1 = circle((0.95, 0), 0.95)
}
extrude1 = extrude(s2, 9)
sketch d3(on: offset(xy, 9)) {
  p = pt(0, 0)
}
sketch s4(on: plane(at: d3.p, normal: X, x: Y)) {
  ax = line((0, 1), (0, 0), construction: true)
  l1 = line((0, 0), (1, 0))
  l2 = line((1, 0), (0, 1))
  l3 = line((0, 1), (0, 0))
}
revolve1 = revolve(s4, axis: s4.ax)
"#,
        r#"sketch d1(on: offset(xy, 0)) {
  p = pt(-2, 0)
}
sketch s2(on: plane(at: d1.p, normal: Z, x: X)) {
  c1 = circle((2, 0), 2)
}
extrude1 = extrude(s2, 7)
round = fillet(edges(of: extrude1.end), 0.5)
sketch d3(on: offset(xy, 7)) {
  p = pt(-1.5, 0)
}
sketch s4(on: plane(at: d3.p, normal: Z, x: X)) {
  c1 = circle((1.45, 0), 1.45)
}
extrude2 = extrude(s4, 2)
"#,
    ] {
        let b = built(src).body.compact();
        let mesh = tessellate(&b, &Quality::default()).expect("meshes");
        assert!(mesh.open_edges().is_empty());
    }
}

/// A tapered pocket's cone and a bore's cylinder 0.15 apart: the boolean stops on
/// InconsistentClassification (a CADFS part fails so; see the fixture). Open: ignored until the
/// boolean classifies near-coincident cone and cylinder faces alike.
#[test]
#[ignore = "open: InconsistentClassification, cone against plane"]
fn a_bore_beside_a_tapered_lobe_builds() {
    use forge_lang::fuzz::{Verdict, judge_built};
    let (v, _) = judge_built(include_str!("kernel_parts/tapered_lobe_bore.fg"));
    assert!(matches!(v, Verdict::Ok), "{v:?}");
}

/// A tapered cut whose walls meet before its depth: today an invalid body (EdgeDegenerate), which
/// the judge calls a fail. Due: a clean refusal. Open: ignored until the draft says the tool
/// collapses.
#[test]
#[ignore = "open: the collapsed taper builds an invalid body"]
fn a_taper_that_collapses_is_refused_cleanly() {
    use forge_lang::fuzz::{Verdict, judge_built};
    let (v, _) = judge_built(include_str!("kernel_parts/taper_collapses.fg"));
    assert!(matches!(v, Verdict::Refused(_)), "{v:?}");
}

/// A ball cut centred on the top face over a 1 deg tapered rounded-rectangle pocket: the boolean
/// stops on SplitCrossed (the sphere). Open: ignored until the boolean splits the sphere cleanly.
#[test]
#[ignore = "open: SplitCrossed on the sphere"]
fn a_ball_over_a_tapered_pocket_builds() {
    use forge_lang::fuzz::{Verdict, judge_built};
    let (v, _) = judge_built(include_str!("kernel_parts/ball_over_tapered_pocket.fg"));
    assert!(matches!(v, Verdict::Ok), "{v:?}");
}

/// A revolved cup whose wall is a spline, shelled: the shell stops on Rebuild (an edge); a straight
/// wall shells. Open: ignored until the shell rebuilds offset spline walls.
#[test]
#[ignore = "open: the shell's Rebuild on a revolved spline wall"]
fn a_revolved_spline_cup_shells() {
    use forge_lang::fuzz::{Verdict, judge_built};
    let (v, _) = judge_built(include_str!("kernel_parts/shell_revolved_spline.fg"));
    assert!(matches!(v, Verdict::Ok), "{v:?}");
}

/// A plate whose wall is a wavy spline (crossing nowhere) builds, then its face fails to mesh
/// (ConstraintThroughPoint). Open: ignored until the mesher takes the face.
#[test]
#[ignore = "open: ConstraintThroughPoint meshing a wavy spline wall"]
fn a_wavy_spline_wall_meshes() {
    use forge_lang::fuzz::{Verdict, judge_built};
    let (v, _) = judge_built(include_str!("kernel_parts/mesh_wavy_spline.fg"));
    assert!(matches!(v, Verdict::Ok), "{v:?}");
}

/// A fillet carried along a tangent chain (a concave arc, a straight side, a top arc) into an end
/// face a scoop has trimmed: Overflow, where FreeCAD rounds it. Open: ignored until the fillet
/// builds there.
#[test]
#[ignore = "open: Fillet Overflow along a tangent chain into a trimmed face"]
fn a_fillet_along_a_tangent_chain_past_a_scoop_builds() {
    use forge_lang::fuzz::{Verdict, judge_built};
    let (v, _) = judge_built(include_str!("kernel_parts/fillet_tangent_chain_scoop.fg"));
    assert!(matches!(v, Verdict::Ok), "{v:?}");
}

/// A drill started on a plane tangent to a round crown, cut up to a bore: NotClosed (a judge fail).
/// Open: ignored until the cut builds.
#[test]
#[ignore = "open: NotClosed drilling from a plane tangent to the crown"]
fn a_drill_from_a_tangent_plane_to_a_bore_builds() {
    use forge_lang::fuzz::{Verdict, judge_built};
    let (v, _) = judge_built(include_str!("kernel_parts/drill_from_tangent_plane.fg"));
    assert!(matches!(v, Verdict::Ok), "{v:?}");
}

/// A ring cut whose outer circle is tangent to the block's top and bottom: the two sides it leaves
/// meet only along lines, a non-manifold result, which the kernel refuses as ZeroThickness at the
/// tangent lines (the owner's decision, as mainstream mechanical CAD; Onshape builds it).
#[test]
fn a_ring_cut_tangent_to_two_faces_is_refused_at_the_tangent_lines() {
    use forge_lang::fuzz::{Verdict, judge_built};
    let (v, _) = judge_built(include_str!("kernel_parts/ring_cut_tangent.fg"));
    let Verdict::Refused(why) = &v else {
        panic!("{v:?}")
    };
    assert!(why.contains("ZeroThickness"), "{why}");
    // the tangent lines: the outer circle's top and bottom, z = 0 and z = 10
    assert!(
        why.contains("z: 0") || why.contains("z: 10") || why.contains("z: 9"),
        "{why}"
    );
}

/// Three fillets on exactly written spline walls that the kernel refuses where Onshape rounds them
/// (CADFS, fit splines written exactly). Open: ignored until the fillet spec says what each is due
/// (a round, or a refusal with its reason); each test expects a build.
#[test]
#[ignore = "open: fillet NoBlend where spline walls meet (for the fillet spec)"]
fn a_fillet_where_two_spline_walls_meet_builds() {
    use forge_lang::fuzz::{Verdict, judge_built};
    let (v, _) = judge_built(include_str!("kernel_parts/fillet_spline_walls_noblend.fg"));
    assert!(matches!(v, Verdict::Ok), "{v:?}");
}

#[test]
#[ignore = "open: fillet Overflow between a wavy spline wall and a notch (for the fillet spec)"]
fn a_fillet_on_a_notched_wavy_plate_builds() {
    use forge_lang::fuzz::{Verdict, judge_built};
    let (v, _) = judge_built(include_str!("kernel_parts/fillet_wavy_plate_notch.fg"));
    assert!(matches!(v, Verdict::Ok), "{v:?}");
}

#[test]
#[ignore = "open: fillet Overflow on both edges of a narrow crescent (for the fillet spec)"]
fn a_fillet_on_both_edges_of_a_crescent_builds() {
    use forge_lang::fuzz::{Verdict, judge_built};
    let (v, _) = judge_built(include_str!("kernel_parts/fillet_crescent_both_edges.fg"));
    assert!(matches!(v, Verdict::Ok), "{v:?}");
}
