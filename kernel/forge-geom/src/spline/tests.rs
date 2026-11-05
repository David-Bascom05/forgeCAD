use super::*;
use crate::Curve;
use crate::surface::{Procedural, Surface};
use forge_math::rng::Rng;
use forge_math::{Box2, Point2};

fn pts(v: &[(f64, f64)]) -> Vec<Point3> {
    v.iter().map(|&(x, y)| Point3::new(x, y, 0.0)).collect()
}

fn cubic() -> Spline {
    let c = pts(&[
        (0.0, 0.0),
        (2.0, 5.0),
        (6.0, -3.0),
        (9.0, 4.0),
        (12.0, 1.0),
        (15.0, 2.0),
    ]);
    let k = [0.0, 0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 3.0, 3.0, 3.0];
    Spline::from_nurbs(3, &k, &c, None).unwrap()
}

/// de Boor against the Bezier pieces.
fn de_boor(p: usize, k: &[f64], c: &[Point3], w: &[f64], t: f64) -> Point3 {
    let s = (p..c.len() - 1).find(|&i| t < k[i + 1]).unwrap_or(c.len() - 1);
    let mut d: Vec<[f64; 4]> = (0..=p)
        .map(|j| {
            let q = c[j + s - p];
            let m = w[j + s - p];
            [q.x * m, q.y * m, q.z * m, m]
        })
        .collect();
    for r in 1..=p {
        for j in (r..=p).rev() {
            let i = j + s - p;
            let a = (t - k[i]) / (k[i + 1 + p - r] - k[i]);
            d[j] = lerp(&d[j - 1], &d[j], a);
        }
    }
    point_of(&d[p])
}

#[test]
fn pieces_match_de_boor() {
    let mut r = Rng::new(7);
    for _ in 0..200 {
        let p = 1 + r.below(5);
        let n = p + 1 + r.below(9);
        let c: Vec<Point3> = (0..n)
            .map(|_| Point3::new(r.range(-9.0, 9.0), r.range(-9.0, 9.0), r.range(-9.0, 9.0)))
            .collect();
        let w: Vec<f64> = (0..n).map(|_| r.range(0.3, 3.0)).collect();
        let mut k = vec![0.0; p + 1];
        let mut x = 0.0;
        for _ in 0..n - p - 1 {
            if r.below(4) != 0 {
                x += r.range(0.2, 2.0);
            }
            k.push(x);
        }
        // at most p repeats inside
        let mut ok = true;
        for i in p + 1..n {
            if k[i] == 0.0 || (i >= p && k[i - p..=i].iter().all(|&y| y == k[i])) {
                ok = false;
            }
        }
        if !ok {
            continue;
        }
        k.extend(std::iter::repeat_n(x + 1.0, p + 1));
        let s = Spline::from_nurbs(p, &k, &c, Some(&w)).unwrap();
        for i in 0..=40 {
            let t = (x + 1.0) * i as f64 / 40.0;
            let (a, b) = (s.point(t), de_boor(p, &k, &c, &w, t));
            assert!(a.distance(b) < 1e-9, "{p} {t} {a:?} {b:?}");
        }
    }
}

#[test]
fn unclamped_knots_match_de_boor_to_the_domain_end() {
    // a periodic cubic's control polygon wrapped, uniform knots: it closes on itself
    let mut r = Rng::new(11);
    for p in 1..=4 {
        let n = p + 2 + r.below(6);
        let ring: Vec<Point3> = (0..n)
            .map(|_| Point3::new(r.range(-9.0, 9.0), r.range(-9.0, 9.0), 0.0))
            .collect();
        let c: Vec<Point3> = (0..n + p).map(|i| ring[i % n]).collect();
        let k: Vec<f64> = (0..n + 2 * p + 1).map(|i| i as f64).collect();
        let w = vec![1.0; c.len()];
        let s = Spline::from_nurbs(p, &k, &c, None).unwrap();
        let (a, b) = s.ends();
        assert!(a.distance(b) < 1e-9, "{p} {n}: {a:?} {b:?}");
        for i in 0..=40 {
            let t = p as f64 + n as f64 * i as f64 / 40.0;
            let (x, y) = (s.point(t), de_boor(p, &k, &c, &w, t.min(k[c.len()] - 1e-12)));
            assert!(x.distance(y) < 1e-6, "{p} {t} {x:?} {y:?}");
        }
    }
}

#[test]
fn derivatives_match_differences() {
    let mut w = cubic();
    let e = Spline::ellipse(Point3::ORIGIN, Vec3::X * 5.0, Vec3::Y * 2.0, 0.3, 4.0).unwrap();
    for s in [&mut w, &mut e.clone()] {
        let d = s.domain();
        for i in 1..50 {
            let t = d.lo + d.width() * (i as f64 + 0.37) / 51.0;
            let h = 1e-5;
            let q = s.deriv(t);
            let (a, b) = (s.deriv(t - h), s.deriv(t + h));
            let d1 = (b.p - a.p) / (2.0 * h);
            let d2 = (b.d1 - a.d1) / (2.0 * h);
            assert!((d1 - q.d1).length() < 1e-5, "{t}");
            // past a joint the second derivative jumps; only smooth stretches are compared
            if s.breaks().iter().all(|&k| (k - t).abs() > 2.0 * h) {
                assert!((d2 - q.d2).length() < 1e-4, "{t} {d2:?} {:?}", q.d2);
            }
        }
    }
}

#[test]
fn bounds_hold() {
    let s = Spline::ellipse(Point3::ORIGIN, Vec3::X * 5.0, Vec3::Y * 2.0, 0.3, 4.0).unwrap();
    let c = cubic();
    for x in [&s, &c] {
        let dom = x.domain();
        let mut r = Rng::new(3);
        for _ in 0..200 {
            let (a, b) = (r.range(dom.lo, dom.hi), r.range(dom.lo, dom.hi));
            let i = Interval::new(a.min(b), a.max(b));
            let box_ = x.bbox(i);
            let [m1, m2, _] = x.derivative_bounds(i);
            for k in 0..=16 {
                let t = i.lo + i.width() * k as f64 / 16.0;
                let d = x.deriv(t);
                assert!(box_.grow(1e-9).contains(d.p));
                assert!(d.d1.length() <= m1 * (1.0 + 1e-9) + 1e-9);
                assert!(d.d2.length() <= m2 * (1.0 + 1e-9) + 1e-9);
            }
        }
    }
}

#[test]
fn nearest_is_the_nearest() {
    let c = cubic();
    let e = Spline::ellipse(
        Point3::new(1.0, 2.0, 0.0),
        Vec3::new(5.0, 1.0, 0.0),
        Vec3::new(-0.4, 2.0, 0.0),
        0.0,
        6.0,
    )
    .unwrap();
    let mut r = Rng::new(11);
    for s in [&c, &e] {
        let d = s.domain();
        for _ in 0..500 {
            let p = Point3::new(r.range(-5.0, 20.0), r.range(-8.0, 8.0), 0.0);
            let t = s.closest(p, d);
            let dt = s.point(t).distance(p);
            for i in 0..=400 {
                let u = d.lo + d.width() * i as f64 / 400.0;
                assert!(dt <= s.point(u).distance(p) + 1e-9, "{p:?} {t} {u}");
            }
        }
    }
}

#[test]
fn bern_roots_count_and_touch() {
    // (s - 0.25)(s - 0.75) and its square, in Bernstein form over [0, 1]
    let q = [0.1875, -0.3125, 0.1875];
    let mut o = Vec::new();
    bern_roots(&q, 0.0, &mut o);
    assert_eq!(o.len(), 2);
    assert!((o[0].0 - 0.25).abs() < 1e-12 && (o[1].0 - 0.75).abs() < 1e-12);
    assert!(!o[0].1 && !o[1].1);
    // (s - 0.5)^2
    let t = [0.25, -0.25, 0.25];
    o.clear();
    bern_roots(&t, 1e-12, &mut o);
    assert_eq!(o.len(), 1);
    assert!(o[0].1 && (o[0].0 - 0.5).abs() < 1e-9);
    // a triple root crosses: (s - 0.5)^3
    let c = [-0.125, -0.125 / 3.0, 0.125 / 3.0, 0.125];
    o.clear();
    bern_roots(&c, 1e-9, &mut o);
    assert_eq!(o.len(), 1, "{o:?}");
    assert!(!o[0].1);
}

#[test]
fn through_passes_its_points_and_is_c2() {
    let p = pts(&[(0.0, 0.0), (3.0, 4.0), (7.0, 1.0), (8.0, -2.0), (12.0, 0.0)]);
    let s = Spline::through(&p, None, None).unwrap();
    for (i, q) in p.iter().enumerate() {
        assert!(s.point(s.breaks()[i]).distance(*q) < 1e-12);
    }
    for &b in &s.breaks()[1..4] {
        let (a, c) = (s.deriv(b - 1e-9), s.deriv(b + 1e-9));
        assert!((a.d1 - c.d1).length() < 1e-6);
        assert!((a.d2 - c.d2).length() < 1e-5);
    }
    // natural ends
    let d = s.domain();
    assert!(s.deriv(d.lo).d2.length() < 1e-9 && s.deriv(d.hi).d2.length() < 1e-9);
    // closed: runs on past the joint smoothly
    let mut q = p.clone();
    q.push(p[0]);
    let cl = Spline::through(&q, None, None).unwrap();
    let dm = cl.domain();
    let (a, b) = (cl.deriv(dm.lo), cl.deriv(dm.hi));
    assert!(a.p.distance(b.p) < 1e-12);
    assert!((a.d1 - b.d1).length() < 1e-9, "{:?} {:?}", a.d1, b.d1);
    assert!((a.d2 - b.d2).length() < 1e-7);
}

#[test]
fn an_ellipse_lies_on_its_equation() {
    let (a, b) = (7.0, 3.0);
    let s = Spline::ellipse(Point3::ORIGIN, Vec3::X * a, Vec3::Y * b, 0.0, dmath::TAU).unwrap();
    for i in 0..=999 {
        let d = s.domain();
        let q = s.point(d.lo + d.width() * i as f64 / 999.0);
        let e = (q.x / a) * (q.x / a) + (q.y / b) * (q.y / b) - 1.0;
        assert!(e.abs() < 1e-13);
    }
    let area = s.area_term();
    assert!((area - dmath::PI * a * b).abs() < 1e-9, "{area}");
}

fn wall() -> Extruded {
    let mut f = Frame::from_z(
        Point3::new(1.0, -2.0, 3.0),
        Vec3::new(0.3, 0.4, 1.0).normalize().unwrap(),
    );
    f = f.rotated_about_z(0.7);
    Extruded::new(f, Arc::new(cubic()), Interval::new(0.0, 7.0)).unwrap()
}

#[test]
fn extruded_derivatives_and_implicit() {
    let e = wall();
    let dom = e.domain();
    let mut r = Rng::new(5);
    for _ in 0..300 {
        let uv = Point2::new(r.range(dom.u.lo, dom.u.hi), r.range(0.0, 7.0));
        let d = e.deriv(uv);
        // on it: zero and back to the same parameters
        assert!(e.implicit(d.p).abs() < 1e-9);
        let back = e.project(d.p);
        assert!(e.deriv(back).p.distance(d.p) < 1e-9);
        // off it along the natural normal
        let n = e.normal(uv).unwrap();
        let h = 1e-3;
        assert!((e.implicit(d.p + n * h) - h).abs() < 1e-5);
        assert!((e.gradient(d.p + n * h) - n.get()).length() < 1e-2);
        let b = e.bbox(Box2::new(
            Interval::new(uv.x - 0.1, uv.x + 0.1),
            Interval::new(uv.y - 0.1, uv.y + 0.1),
        ));
        assert!(b.grow(1e-9).contains(d.p));
    }
}

#[test]
fn spun_derivatives_and_implicit() {
    let c = cubic().mapped(|p| Point3::new(p.y + 10.0, p.x, 0.0));
    let f = Frame::from_z(
        Point3::new(1.0, -2.0, 3.0),
        Vec3::new(0.3, 0.4, 1.0).normalize().unwrap(),
    );
    let s = Spun::new(f, Arc::new(c)).unwrap();
    let dom = s.domain();
    let mut r = Rng::new(5);
    for _ in 0..300 {
        let uv = Point2::new(r.range(0.0, 6.2), r.range(dom.v.lo, dom.v.hi));
        let d = s.deriv(uv);
        assert!(s.implicit(d.p).abs() < 1e-9);
        let back = s.project(d.p);
        assert!(s.deriv(back).p.distance(d.p) < 1e-9);
        let n = s.normal(uv).unwrap();
        assert!((s.implicit(d.p + n * 1e-3) - 1e-3).abs() < 1e-5);
        let h = 1e-5;
        let du =
            (s.deriv(Point2::new(uv.x + h, uv.y)).p - s.deriv(Point2::new(uv.x - h, uv.y)).p) / (2.0 * h);
        let dv =
            (s.deriv(Point2::new(uv.x, uv.y + h)).p - s.deriv(Point2::new(uv.x, uv.y - h)).p) / (2.0 * h);
        assert!((du - d.du).length() < 1e-6);
        assert!((dv - d.dv).length() < 1e-5);
        let b = s.bbox(Box2::new(
            Interval::new(uv.x - 0.1, uv.x + 0.1),
            Interval::new(uv.y - 0.1, uv.y + 0.1),
        ));
        assert!(b.contains(d.p));
    }
}

#[test]
fn a_plane_cuts_a_wall_in_exactly_the_section() {
    use crate::Plane;
    use crate::intersect::SurfaceSurface;
    let e = wall();
    let s = e.clone().surface();
    let mut r = Rng::new(9);
    let mut splines = 0;
    let mut rulings = 0;
    for k in 0..300 {
        let f = *e.frame();
        let n = if k % 3 == 0 {
            // along the rulings
            let a = r.range(0.0, dmath::TAU);
            f.x() * dmath::cos(a) + f.y() * dmath::sin(a)
        } else {
            Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0))
        };
        let Some(n) = n.normalize() else { continue };
        let o =
            f.origin + f.x() * r.range(0.0, 15.0) + f.y() * r.range(-2.0, 2.0) + f.z() * r.range(0.0, 7.0);
        let pl = Surface::Plane(Plane::new(Frame::from_z(o, n)));
        let Some(SurfaceSurface::Curves(cs)) = meet::meet_spline(&pl, &s) else {
            continue;
        };
        for (c, _) in &cs {
            match c {
                Curve::Line(l) => {
                    rulings += 1;
                    for t in [-3.0, 0.0, 5.0] {
                        let p = l.origin + l.dir * t;
                        assert!(s.implicit(p).abs() < 1e-10, "ruling off the wall");
                        assert!(pl.implicit(p).abs() < 1e-10, "ruling off the plane");
                    }
                }
                _ => {
                    splines += 1;
                    let sc = spline_of(c).expect("a spline");
                    for i in 0..=64 {
                        let t = sc.dom.lo + sc.dom.width() * i as f64 / 64.0;
                        let p = sc.c.point(t);
                        assert!(s.implicit(p).abs() < 1e-10);
                        assert!(pl.implicit(p).abs() < 1e-10);
                    }
                }
            }
        }
    }
    assert!(splines > 50 && rulings > 20, "{splines} {rulings}");
}

#[test]
fn patches_hold_their_walls_and_quadric_signs_agree() {
    use crate::Sphere;
    use crate::loftmeet::{Patch, Span};
    use crate::meet::Alg;
    let e = wall();
    let c = cubic().mapped(|p| Point3::new(p.y + 10.0, p.x, 0.0));
    let f = Frame::from_z(
        Point3::new(1.0, -2.0, 3.0),
        Vec3::new(0.3, 0.4, 1.0).normalize().unwrap(),
    );
    let s = Spun::new(f, Arc::new(c)).unwrap();
    let mut r = Rng::new(21);
    let spans: [&dyn Span; 2] = [&e, &s];
    let (mut said, mut holds) = (0, 0);
    for sp in spans {
        for piece in sp.pieces() {
            for _ in 0..30 {
                let (a, b) = (r.range(0.0, 1.0), r.range(0.0, 1.0));
                let (c0, d0) = (r.range(0.0, 1.0), r.range(0.0, 1.0));
                let sub = |i: Interval, x: f64, y: f64| {
                    Interval::new(i.lo + i.width() * x.min(y), i.lo + i.width() * x.max(y))
                };
                let uv = Box2::new(sub(piece.u, a, b), sub(piece.v, c0, d0));
                if uv.u.width() <= 0.0 || uv.v.width() <= 0.0 {
                    continue;
                }
                let Some(Patch::Curve(h)) = sp.patch(uv) else {
                    panic!("a patch")
                };
                let bx = h.bbox().grow(1e-9);
                let o = Point3::new(r.range(-20.0, 20.0), r.range(-20.0, 20.0), r.range(-20.0, 20.0));
                let q = Alg::of(&Surface::Sphere(Sphere {
                    frame: Frame::WORLD.with_origin(o),
                    radius: r.range(1.0, 30.0),
                }))
                .unwrap();
                let misses = h.misses(&q);
                said += usize::from(misses);
                let mut first: Option<f64> = None;
                let mut same = true;
                for i in 0..=8 {
                    for j in 0..=8 {
                        let w = Point2::new(
                            uv.u.lo + uv.u.width() * i as f64 / 8.0,
                            uv.v.lo + uv.v.width() * j as f64 / 8.0,
                        );
                        let p = sp.at(w).p;
                        assert!(bx.contains(p), "{w:?} {p:?} {bx:?}");
                        let v = q.grad(p).0;
                        same &= first.is_none_or(|x| x * v > 0.0);
                        first = first.or(Some(v));
                    }
                }
                if misses {
                    assert!(same, "a patch that misses the quadric has one sign on it");
                    holds += 1;
                }
            }
        }
    }
    assert!(said > 20 && said == holds, "{said} {holds}");
}

#[test]
fn walls_meet_a_cylinder_without_tracing() {
    use crate::Cylinder;
    use crate::intersect::SurfaceSurface;
    let e = wall().surface();
    let f = *wall().frame();
    // across the wall
    let cy = Surface::Cylinder(Cylinder {
        frame: Frame::from_zx(
            f.origin + f.x() * 6.0 + f.y() * -20.0 + f.z() * 3.0,
            f.y(),
            f.x().get(),
        )
        .unwrap(),
        radius: 1.5,
    });
    let region = forge_math::Box3 {
        x: Interval::new(-40.0, 40.0),
        y: Interval::new(-40.0, 40.0),
        z: Interval::new(-40.0, 40.0),
    };
    let Some(SurfaceSurface::Curves(cs)) = crate::loftmeet::meet(&e, &cy, region) else {
        panic!("the hull path takes this pair");
    };
    assert!(!cs.is_empty());
    for (c, _) in &cs {
        let Curve::Intersection(ic) = c else { panic!() };
        for i in 0..=64 {
            let p = ic.eval(ic.domain_hi() * i as f64 / 64.0);
            assert!(e.implicit(p).abs() < 1e-8 && cy.implicit(p).abs() < 1e-8);
        }
    }
}

#[test]
fn a_swept_wall_stays_on_its_definition() {
    use crate::spline::swept::{Spine, plane_of};
    // a planar spine in the xy plane: its section square to it, x along z
    let spine = Arc::new(cubic());
    let b = plane_of(&spine).unwrap();
    let sp = Spine::planar(spine.clone(), b).unwrap();
    let start = spine.deriv(spine.domain().lo);
    let t = start.d1.normalize().unwrap();
    let plane = Frame::from_zx(start.p, t, b.get()).unwrap();
    let section = Spline::ellipse(
        Point3::new(0.3, 0.1, 0.0),
        Vec3::X * 0.4,
        Vec3::Y * 0.25,
        0.2,
        2.0,
    )
    .unwrap();
    let w = Swept::new(sp, Arc::new(section.clone()), &plane).unwrap();
    let dom = w.domain();
    let mut r = Rng::new(31);
    for _ in 0..300 {
        let uv = Point2::new(r.range(dom.u.lo, dom.u.hi), r.range(dom.v.lo, dom.v.hi));
        let d = w.deriv(uv);
        // differences match
        let h = 1e-5;
        let at = |u: f64, v: f64| w.deriv(Point2::new(u, v));
        let (u0, u1) = (at(uv.x - h, uv.y), at(uv.x + h, uv.y));
        let (v0, v1) = (at(uv.x, uv.y - h), at(uv.x, uv.y + h));
        assert!(((u1.p - u0.p) / (2.0 * h) - d.du).length() < 1e-6);
        assert!(((v1.p - v0.p) / (2.0 * h) - d.dv).length() < 1e-4);
        let kink = spine.breaks().iter().any(|&k| (k - uv.y).abs() < 2.0 * h);
        if !kink {
            assert!(((v1.dv - v0.dv) / (2.0 * h) - d.dvv).length() < 2e-3, "{:?}", uv);
            assert!(((u1.dv - u0.dv) / (2.0 * h) - d.duv).length() < 1e-3);
        }
        // the section stays square to the spine and the same size: its distance from the
        // spine's point is the start's
        let m = spine.deriv(uv.y);
        let e = d.p - m.p;
        assert!(
            e.dot(m.d1.normalize().unwrap().get()).abs() < 1e-10,
            "off the normal plane"
        );
        let q = section.point(uv.x);
        let want = (q.x * q.x + q.y * q.y).sqrt();
        assert!((e.length() - want).abs() < 1e-10);
        assert!(w.implicit(d.p).abs() < 1e-8, "{:?}", uv);
        assert!(w.deriv(w.project(d.p)).p.distance(d.p) < 1e-8);
    }
}

#[test]
fn a_coil_on_a_section_keeps_its_helix() {
    use crate::spline::swept::Spine;
    let axis = Frame::WORLD;
    let sp = Spine::Helix {
        axis,
        a: 0.0,
        b: 2.0,
        range: Interval::new(0.0, 12.0),
    };
    // the section in the axis' xz plane, 10 out
    let plane = Frame::from_zx(
        Point3::new(10.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0).normalize().unwrap(),
        Vec3::X,
    )
    .unwrap();
    let section = Spline::ellipse(Point3::ORIGIN, Vec3::X * 1.0, Vec3::Y * 0.5, 0.0, 3.0).unwrap();
    let w = Swept::new(sp, Arc::new(section.clone()), &plane).unwrap();
    let mut r = Rng::new(32);
    let dom = w.domain();
    for _ in 0..200 {
        let uv = Point2::new(r.range(dom.u.lo, dom.u.hi), r.range(dom.v.lo, dom.v.hi));
        let d = w.deriv(uv);
        let q = section.point(uv.x);
        // radius and height by the section point, turned and raised
        let rad = (d.p.x * d.p.x + d.p.y * d.p.y).sqrt();
        assert!((rad - (10.0 + q.x)).abs() < 1e-9, "{rad} {q:?}");
        assert!(w.implicit(d.p).abs() < 1e-8);
    }
}

#[test]
fn a_spatial_spine_turns_without_twist() {
    use crate::spline::swept::Spine;
    // a helix-like cubic chain: the frame must keep the tangent, and its rate have no part along it
    let pts: Vec<Point3> = (0..9)
        .map(|i| {
            let a = i as f64 * 0.9;
            Point3::new(10.0 * dmath::cos(a), 10.0 * dmath::sin(a), 3.0 * i as f64)
        })
        .collect();
    let k = uniform_knots(3, pts.len());
    let c = Arc::new(Spline::from_nurbs(3, &k, &pts, None).unwrap());
    let sp = Spine::spatial(c.clone()).unwrap();
    let d = c.domain();
    let t0 = c.deriv(d.lo).d1.normalize().unwrap();
    let plane = Frame::from_z(c.point(d.lo), t0);
    let section = Spline::ellipse(Point3::ORIGIN, Vec3::X * 0.8, Vec3::Y * 0.4, 0.0, 2.0).unwrap();
    let w = Swept::new(sp.clone(), Arc::new(section), &plane).unwrap();
    let mut r = Rng::new(40);
    for _ in 0..200 {
        let v = r.range(d.lo, d.hi);
        let m = c.deriv(v);
        let t = m.d1.normalize().unwrap().get();
        // the start's frame, carried: tangent onto tangent
        let u = r.range(0.1, 1.9);
        let s = w.deriv(Point2::new(u, v));
        assert!((s.p - m.p).dot(t).abs() < 1e-9, "out of the normal plane at {v}");
        assert!(w.implicit(s.p).abs() < 1e-7, "{v}");
        // derivatives against differences
        let h = 1e-5;
        if !c.breaks().iter().any(|&b| (b - v).abs() < 3.0 * h) {
            let (a, b) = (w.deriv(Point2::new(u, v - h)), w.deriv(Point2::new(u, v + h)));
            assert!(((b.p - a.p) / (2.0 * h) - s.dv).length() < 1e-5);
            assert!(((b.dv - a.dv) / (2.0 * h) - s.dvv).length() < 1e-3);
        }
    }
    // the far end's frame by the table, against a fine fixed march of the defining equation
    let probe = Vec3::new(t0.get().y, -t0.get().x, 0.0).normalize().unwrap().get();
    let mut q = probe;
    let n = 200_000;
    let hh = d.width() / n as f64;
    for i in 0..n {
        let dd = c.deriv(d.lo + (i as f64 + 0.5) * hh);
        let wr = dd.d1.cross(dd.d2) / dd.d1.length_sq();
        q = q + (wr * hh).cross(q + (wr * hh).cross(q) * 0.5);
        q = q / q.length();
    }
    assert!(
        (sp.end_turn(probe) - q).length() < 1e-6,
        "{:?} {q:?}",
        sp.end_turn(probe)
    );
}

type Bez = [(f64, f64); 4];

/// A cubic's Bezier pieces, from its control form (each break to the degree).
fn beziers(s: &Spline) -> Vec<Bez> {
    let (_, _, p, _) = s.control_form();
    (0..s.pieces())
        .map(|i| std::array::from_fn(|k| (p[3 * i + k].x, p[3 * i + k].y)))
        .collect()
}

fn halves(b: &Bez) -> (Bez, Bez) {
    let m = |p: (f64, f64), q: (f64, f64)| ((p.0 + q.0) / 2.0, (p.1 + q.1) / 2.0);
    let (a, c, e) = (m(b[0], b[1]), m(b[1], b[2]), m(b[2], b[3]));
    let (f, g) = (m(a, c), m(c, e));
    let h = m(f, g);
    ([b[0], a, f, h], [h, g, e, b[3]])
}

fn boxes_apart(a: &Bez, b: &Bez) -> bool {
    let bx = |z: &Bez, k: fn(&(f64, f64)) -> f64| {
        z.iter()
            .map(k)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            })
    };
    let apart = |k: fn(&(f64, f64)) -> f64| {
        let (x, y) = (bx(a, k), bx(b, k));
        x.1 < y.0 || y.1 < x.0
    };
    apart(|p| p.0) || apart(|p| p.1)
}

/// Every hodograph control point of the pieces strictly on one side of a line through the
/// origin: together they run one way along its normal, so they can't meet themselves again.
fn one_way(pieces: &[&Bez]) -> bool {
    let d: Vec<(f64, f64)> = pieces
        .iter()
        .flat_map(|b| (0..3).map(move |k| (b[k + 1].0 - b[k].0, b[k + 1].1 - b[k].1)))
        .collect();
    let s = d.iter().fold((0.0, 0.0), |a, v| {
        let l = (v.0 * v.0 + v.1 * v.1).sqrt().max(1e-300);
        (a.0 + v.0 / l, a.1 + v.1 / l)
    });
    d.iter().all(|v| v.0 * s.0 + v.1 * s.1 > 0.0)
}

/// Proven not to meet: boxes apart, or (`touching` at a's end and b's start) one way together.
fn apart(a: &Bez, b: &Bez, touching: bool, depth: u32) -> bool {
    if boxes_apart(a, b) || (touching && one_way(&[a, b])) {
        return true;
    }
    if depth == 0 {
        return false;
    }
    let ((a0, a1), (b0, b1)) = (halves(a), halves(b));
    apart(&a0, &b0, false, depth - 1)
        && apart(&a0, &b1, false, depth - 1)
        && apart(&a1, &b1, false, depth - 1)
        && apart(&a1, &b0, touching, depth - 1)
}

/// An open cubic proven never to cross itself: split into arcs that each run one way, then
/// every pair of arcs apart.
fn simple(s: &Spline) -> bool {
    let mut arcs = Vec::new();
    let mut todo: Vec<(Bez, u32)> = beziers(s).into_iter().rev().map(|b| (b, 20)).collect();
    while let Some((b, d)) = todo.pop() {
        if one_way(&[&b]) {
            arcs.push(b);
        } else if d == 0 {
            return false;
        } else {
            let (x, y) = halves(&b);
            todo.push((y, d - 1));
            todo.push((x, d - 1));
        }
    }
    (0..arcs.len()).all(|i| (i + 1..arcs.len()).all(|j| apart(&arcs[i], &arcs[j], j == i + 1, 24)))
}

/// What `through` gave before it was centripetal: the knots spaced by the distances themselves.
fn by_the_chord(p: &[Point3]) -> Spline {
    let n = p.len();
    let mut t = vec![0.0; n];
    for i in 1..n {
        t[i] = t[i - 1] + p[i].distance(p[i - 1]);
    }
    let h = |i: usize| t[i + 1] - t[i];
    let dp = |i: usize| (p[i + 1] - p[i]) / h(i);
    let mut rows = vec![(0.0, 2.0, 1.0, dp(0) * 3.0)];
    for i in 1..n - 1 {
        rows.push((
            h(i),
            2.0 * (h(i - 1) + h(i)),
            h(i - 1),
            (dp(i - 1) * h(i) + dp(i) * h(i - 1)) * 3.0,
        ));
    }
    rows.push((1.0, 2.0, 0.0, dp(n - 2) * 3.0));
    let m = tridiagonal(&rows);
    let mut out = Vec::new();
    for i in 0..n - 1 {
        let (a, b) = (p[i], p[i + 1]);
        for q in [a, a + m[i] * (h(i) / 3.0), b - m[i + 1] * (h(i) / 3.0), b] {
            out.push([q.x, q.y, q.z, 1.0]);
        }
    }
    Spline::from_pieces(3, t, out).unwrap()
}

#[test]
fn through_a_sharp_turn_the_fit_never_crosses_itself_where_the_chord_fit_did() {
    // out 8 along, back past the start, then down: the turn at (8, 3) is sharp
    let p = pts(&[(0.0, 0.0), (8.0, 3.0), (-2.0, -3.0), (-2.0, -10.0)]);
    let s = Spline::through(&p, None, None).unwrap();
    assert!(simple(&s), "the fit crosses itself");
    // the chord fit's first two pieces cross: the crossing found and refined, inside both
    let c = by_the_chord(&p);
    let b = beziers(&c);
    let at = |z: &Bez, u: f64| {
        let w = [
            (1.0 - u).powi(3),
            3.0 * (1.0 - u).powi(2) * u,
            3.0 * (1.0 - u) * u * u,
            u.powi(3),
        ];
        (0..4).fold((0.0, 0.0), |a, k| (a.0 + w[k] * z[k].0, a.1 + w[k] * z[k].1))
    };
    let d = |z: &Bez, u: f64| {
        let w = [(1.0 - u).powi(2), 2.0 * (1.0 - u) * u, u * u];
        (0..3).fold((0.0, 0.0), |a, k| {
            (
                a.0 + 3.0 * w[k] * (z[k + 1].0 - z[k].0),
                a.1 + 3.0 * w[k] * (z[k + 1].1 - z[k].1),
            )
        })
    };
    let n = 400;
    let mut best = (f64::INFINITY, 0.0, 0.0);
    for i in 1..n {
        for j in 1..n {
            let (u, v) = (i as f64 / n as f64, j as f64 / n as f64);
            let (x, y) = (at(&b[0], u), at(&b[1], v));
            let g = ((x.0 - y.0).powi(2) + (x.1 - y.1).powi(2)).sqrt();
            // away from where the two pieces join
            if g < best.0 && u < 0.9 && v > 0.1 {
                best = (g, u, v);
            }
        }
    }
    let (_, mut u, mut v) = best;
    for _ in 0..30 {
        let (x, y) = (at(&b[0], u), at(&b[1], v));
        let (f, g) = (x.0 - y.0, x.1 - y.1);
        let (da, db) = (d(&b[0], u), d(&b[1], v));
        let det = -da.0 * db.1 + da.1 * db.0;
        u -= (-db.1 * f + db.0 * g) / det;
        v -= (-da.1 * f + da.0 * g) / det;
    }
    let (x, y) = (at(&b[0], u), at(&b[1], v));
    let (da, db) = (d(&b[0], u), d(&b[1], v));
    assert!(
        ((x.0 - y.0).powi(2) + (x.1 - y.1).powi(2)).sqrt() < 1e-12,
        "{x:?} {y:?}"
    );
    assert!(0.0 < u && u < 1.0 && 0.0 < v && v < 1.0, "{u} {v}");
    assert!((da.0 * db.1 - da.1 * db.0).abs() > 1.0, "a crossing, not a touch");
    assert!(!simple(&c));
}
