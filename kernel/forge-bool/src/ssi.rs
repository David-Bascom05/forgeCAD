use forge_geom::intersect::{Contact, SurfaceSurface, plane_surface};
use forge_geom::surface::Proc;
use forge_geom::{Circle, Curve, Cylinder, Ellipse, Line, Sphere, Surface};
use forge_math::{Box3, Frame, Point3, tol};

/// Intersection of two surfaces within `region`: closed form where the pair
/// allows, otherwise traced (an exact curve defined by both surfaces).
pub fn surfaces_in(a: &Surface, b: &Surface, region: Box3) -> SurfaceSurface {
    match closed_or_flush(a, b, &region) {
        SurfaceSurface::NotAnalytic => searched(a, b, region),
        r => r,
    }
}

/// Where two surfaces meet, when that has a closed form (NotAnalytic otherwise): cheap, unlike
/// `searched`.
pub fn closed_form(a: &Surface, b: &Surface) -> SurfaceSurface {
    let closed = match (a, b) {
        (Surface::Plane(p), s) => plane_surface(p, s),
        (s, Surface::Plane(p)) => plane_surface(p, s),
        (Surface::Cylinder(c), Surface::Cylinder(d)) => cylinder_cylinder(c, d),
        (Surface::Sphere(s), Surface::Sphere(t)) => sphere_sphere(s, t),
        _ => SurfaceSurface::NotAnalytic,
    };
    let r = match closed {
        SurfaceSurface::NotAnalytic => forge_geom::screw::meet_coaxial(a, b)
            .or_else(|| forge_geom::ruled::meet_ruled(a, b))
            .or_else(|| forge_geom::spline::meet::meet_spline(a, b))
            .unwrap_or(SurfaceSurface::NotAnalytic),
        r => r,
    };
    // a spindle's equations hold its other sheet too
    if forge_geom::sheet::spindle_in(a, b) {
        return forge_geom::sheet::used_by(a, b, r);
    }
    r
}

/// `closed_form`, except surfaces it has crossing that are `nearly_flush` over `region`.
#[inline]
pub(crate) fn closed_or_flush(a: &Surface, b: &Surface, region: &Box3) -> SurfaceSurface {
    let r = closed_form(a, b);
    let near = match (a, b, &r) {
        // a plane all but tangent to a cylinder crosses it in a long thin ellipse
        (Surface::Plane(_), Surface::Cylinder(_), SurfaceSurface::Curves(cs))
        | (Surface::Cylinder(_), Surface::Plane(_), SurfaceSurface::Curves(cs)) => {
            matches!(cs[..], [(Curve::Ellipse(_), _)])
        }
        (Surface::Plane(p), Surface::Plane(q), SurfaceSurface::Curves(_)) => {
            p.normal_vec().dot(q.normal_vec()).abs() > 0.5
        }
        _ => false,
    };
    if near && let Some(f) = nearly_flush(a, b, region) {
        return f;
    }
    r
}

/// Surfaces too nearly flush for an angle test to see, as a model rounded to 9 digits has them: a
/// plane on a plane, or tangent to a cylinder along its length, to within LINEAR all over `region`.
/// Left to the closed forms, the plane crossing the other in a line or a cylinder in two hairs
/// apart gives edges the tolerance cannot tell apart.
#[cold]
fn nearly_flush(a: &Surface, b: &Surface, region: &Box3) -> Option<SurfaceSurface> {
    const NEAR: f64 = 1e-4;
    let corners = || {
        [
            (region.x.lo, region.y.lo, region.z.lo),
            (region.x.hi, region.y.lo, region.z.lo),
            (region.x.lo, region.y.hi, region.z.lo),
            (region.x.hi, region.y.hi, region.z.lo),
            (region.x.lo, region.y.lo, region.z.hi),
            (region.x.hi, region.y.lo, region.z.hi),
            (region.x.lo, region.y.hi, region.z.hi),
            (region.x.hi, region.y.hi, region.z.hi),
        ]
        .map(|(x, y, z)| Point3::new(x, y, z))
    };
    match (a, b) {
        (Surface::Plane(p), Surface::Plane(q)) => {
            let (n, m) = (p.normal_vec(), q.normal_vec());
            let d = n.dot(m);
            let flip = if d < 0.0 { -1.0 } else { 1.0 };
            let c = n.cross(m).length();
            (c > tol::ANGULAR
                && c <= NEAR
                && corners()
                    .iter()
                    .all(|&k| (p.signed_distance(k) - flip * q.signed_distance(k)).abs() <= tol::LINEAR))
            .then_some(SurfaceSurface::Coincident)
        }
        (Surface::Plane(p), Surface::Cylinder(c)) | (Surface::Cylinder(c), Surface::Plane(p)) => {
            let (n, ax) = (p.normal_vec(), c.frame.z());
            let tilt = n.dot(ax).abs();
            if tilt <= tol::ANGULAR || tilt > NEAR {
                return None;
            }
            // the plane's distance from the axis, where the region lies along the axis
            let along = corners().map(|k| ax.dot(k - c.frame.origin));
            let (lo, hi) = along
                .iter()
                .fold((f64::MAX, f64::MIN), |m, &t| (m.0.min(t), m.1.max(t)));
            let at = |t: f64| p.signed_distance(c.frame.origin + ax * t);
            let touching = (at(lo).abs() - c.radius).abs() <= tol::LINEAR
                && (at(hi).abs() - c.radius).abs() <= tol::LINEAR;
            touching.then(|| {
                let foot = c.frame.origin + ax * lo - n * at(lo);
                SurfaceSurface::Curves(vec![(Curve::Line(Line::new(foot, ax)), Contact::Touching)])
            })
        }
        _ => None,
    }
}

/// Pairs searched: a boolean meets one pair several times over the same region, and a drag meets over again every
/// pair it did not move. `search` is a function of the two surfaces and the region and of nothing else, and a key
/// holds them whole, to the bit, so an answer found stands (research/drag-speed).
#[derive(Clone, PartialEq, Eq, Hash)]
struct Met([forge_geom::surface::Bits; 2], [u64; 6]);
fn met(a: &Surface, b: &Surface, r: &Box3) -> Met {
    Met(
        [a.bits(), b.bits()],
        [r.x.lo, r.x.hi, r.y.lo, r.y.hi, r.z.lo, r.z.hi].map(f64::to_bits),
    )
}

/// A quick mix: the map compares whole keys, so all a hash does is pick where to look.
#[derive(Default)]
struct Mix(u64);
impl std::hash::Hasher for Mix {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(b as u64);
        }
    }
    fn write_u64(&mut self, x: u64) {
        self.0 = (self.0.rotate_left(5) ^ x).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    fn write_usize(&mut self, x: usize) {
        self.write_u64(x as u64);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}
type Mets = (
    std::collections::HashMap<
        Met,
        (Option<Proc>, Option<Proc>, SurfaceSurface),
        std::hash::BuildHasherDefault<Mix>,
    >,
    usize,
);
thread_local! {
    static MET: std::cell::RefCell<Mets> = std::cell::RefCell::new(Default::default());
    static MET_MOST: std::cell::Cell<usize> = const { std::cell::Cell::new(4096) };
}

/// For tests: how many answers this thread keeps (0: none), and forgets those it has.
pub fn searched_keeps(most: usize) {
    MET_MOST.with(|m| m.set(most));
    MET.with(|m| *m.borrow_mut() = Default::default());
}

/// Where two surfaces with no closed form meet in `region`: by marching or tracing.
pub fn searched(a: &Surface, b: &Surface, region: Box3) -> SurfaceSurface {
    // an apex in the region: a branch may change sheets there, and curves cannot end at one yet
    if forge_geom::sheet::apex_met(a, b, &region) {
        return SurfaceSurface::NotAnalytic;
    }
    if forge_geom::screw::misses(a, b, region) {
        return SurfaceSurface::Empty;
    }
    let key = met(a, b, &region);
    if let Some(r) = MET.with(|m| m.borrow().0.get(&key).map(|e| e.2.clone())) {
        return r;
    }
    let r = search(a, b, region);
    let most = MET_MOST.with(|m| m.get());
    MET.with(|m| {
        let mut m = m.borrow_mut();
        if m.1 >= most {
            *m = Default::default();
        }
        if most > 0 {
            if m.0.capacity() == 0 {
                m.0.reserve(most.min(4096));
            }
            m.1 += 1;
            // a procedural surface kept, so that its address stays its own
            let held = |s: &Surface| match s {
                Surface::Procedural(p) => Some(p.clone()),
                _ => None,
            };
            m.0.insert(key, (held(a), held(b), r.clone()));
        }
    });
    r
}

/// Where two surfaces the search gave up on meet, when every point it stalled at is `off` the faces in
/// question: the surfaces go on past the faces, and a near-touch there is none of a boolean's business.
#[cold]
pub(crate) fn past_stalls(
    a: &Surface,
    b: &Surface,
    region: Box3,
    off: impl Fn(Point3) -> bool,
) -> Option<Vec<(Curve, Contact)>> {
    if forge_geom::sheet::apex_met(a, b, &region) {
        return None;
    }
    let (curves, stalls) = forge_geom::intcurve::trace_past_stalls(a, b, region)?;
    if stalls.is_empty() || !stalls.into_iter().all(off) {
        return None;
    }
    let curve = |c| (Curve::Intersection(std::sync::Arc::new(c)), Contact::Crossing);
    Some(curves.into_iter().map(curve).collect())
}

fn search(a: &Surface, b: &Surface, region: Box3) -> SurfaceSurface {
    // a spindle's algebra holds its other sheet too; the tracer's implicit, a distance with the
    // axis distance not negative, vanishes on the used sheet only
    let met = forge_geom::meet::intersect(a, b, region)
        .map(|r| forge_geom::sheet::used_by(a, b, r))
        .or_else(|| forge_geom::loftmeet::meet(a, b, region));
    match met {
        Some(r) if r != SurfaceSurface::NotAnalytic => r,
        _ => match forge_geom::intcurve::trace(a, b, region) {
            Some(cs) if cs.is_empty() => SurfaceSurface::Empty,
            Some(cs) => SurfaceSurface::Curves(
                cs.into_iter()
                    .map(|c| (Curve::Intersection(std::sync::Arc::new(c)), Contact::Crossing))
                    .collect(),
            ),
            // a screw face the tracer cannot follow across its carried track's corners
            None => forge_geom::screw::meet_general(a, b, region).unwrap_or(SurfaceSurface::NotAnalytic),
        },
    }
}

/// Whether two surfaces are the same surface.
pub fn coincident(a: &Surface, b: &Surface) -> bool {
    match (a, b) {
        (Surface::Plane(p), Surface::Plane(q)) => forge_geom::intersect::planes_coincident(p, q),
        (Surface::Plane(p), s) | (s, Surface::Plane(p)) => plane_surface(p, s) == SurfaceSurface::Coincident,
        (Surface::Cylinder(c), Surface::Cylinder(d)) => cylinder_cylinder(c, d) == SurfaceSurface::Coincident,
        (Surface::Sphere(s), Surface::Sphere(t)) => sphere_sphere(s, t) == SurfaceSurface::Coincident,
        _ => forge_geom::meet::coaxial(a, b) == Some(SurfaceSurface::Coincident),
    }
}

/// Parallel axes give lines (or the same cylinder), equal ones with crossing axes two
/// ellipses; anything else is a quartic space curve, left to the general path.
fn cylinder_cylinder(c: &Cylinder, d: &Cylinder) -> SurfaceSurface {
    let a = c.frame.z();
    if a.cross(d.frame.z()).length() > tol::ANGULAR {
        return equal_crossing(c, d).unwrap_or(SurfaceSurface::NotAnalytic);
    }
    // work in c's cross-section plane
    let off = d.frame.origin - c.frame.origin;
    let off = off - a * a.dot(off);
    let dist = off.length();
    let (r1, r2) = (c.radius, d.radius);
    if dist <= tol::LINEAR {
        return if (r1 - r2).abs() <= tol::LINEAR {
            SurfaceSurface::Coincident
        } else {
            SurfaceSurface::Empty
        };
    }
    if dist > r1 + r2 + tol::LINEAR || dist < (r1 - r2).abs() - tol::LINEAR {
        return SurfaceSurface::Empty;
    }
    let ex = off / dist;
    let ey = a.cross(ex);
    // circle-circle in the section: x along the centre line
    let x = (dist * dist + r1 * r1 - r2 * r2) / (2.0 * dist);
    let h2 = r1 * r1 - x * x;
    let at = |y: f64| c.frame.origin + ex * x + ey * y;
    let dir = a;
    if h2 <= (2.0 * tol::LINEAR * r1).max(0.0) {
        return SurfaceSurface::Curves(vec![(Curve::Line(Line::new(at(0.0), dir)), Contact::Touching)]);
    }
    let h = h2.sqrt();
    SurfaceSurface::Curves(vec![
        (Curve::Line(Line::new(at(-h), dir)), Contact::Crossing),
        (Curve::Line(Line::new(at(h), dir)), Contact::Crossing),
    ])
}

/// Equal cylinders whose axes cross (a T or + drilling) meet in two ellipses, in the planes
/// through the crossing that bisect the axes, centred on it: each plane holds the common
/// perpendicular, along which both ellipses reach r, where they cross and the cylinders touch.
fn equal_crossing(c: &Cylinder, d: &Cylinder) -> Option<SurfaceSurface> {
    let r = c.radius;
    let (a, b) = (c.frame.z().get(), d.frame.z().get());
    let n = a.cross(b);
    let w = d.frame.origin - c.frame.origin;
    let len = n.length();
    if (r - d.radius).abs() > tol::LINEAR || (w.dot(n) / len).abs() > tol::LINEAR {
        return None;
    }
    let o = c.frame.origin + a * (w.cross(b).dot(n) / (len * len));
    let m = n / len;
    let mut out = Vec::with_capacity(2);
    for u in [a + b, a - b] {
        let x = u.normalize()?.get();
        // a point x t from the crossing is t |x × a| from either axis
        let frame = Frame::from_axes(o, x, m, x.cross(m))?;
        let major = r / x.cross(a).length();
        out.push((
            Curve::Ellipse(Ellipse {
                frame,
                major,
                minor: r,
            }),
            Contact::Crossing,
        ));
    }
    Some(SurfaceSurface::Curves(out))
}

fn sphere_sphere(s: &Sphere, t: &Sphere) -> SurfaceSurface {
    let d = t.frame.origin - s.frame.origin;
    let dist = d.length();
    let (r1, r2) = (s.radius, t.radius);
    if dist <= tol::LINEAR {
        return if (r1 - r2).abs() <= tol::LINEAR {
            SurfaceSurface::Coincident
        } else {
            SurfaceSurface::Empty
        };
    }
    if dist > r1 + r2 + tol::LINEAR || dist < (r1 - r2).abs() - tol::LINEAR {
        return SurfaceSurface::Empty;
    }
    let n = d.normalize().expect("distinct centres");
    let x = (dist * dist + r1 * r1 - r2 * r2) / (2.0 * dist);
    let centre = s.frame.origin + n * x;
    let h2 = r1 * r1 - x * x;
    if h2 <= 2.0 * tol::LINEAR * r1 {
        return SurfaceSurface::Points(vec![centre]);
    }
    SurfaceSurface::Curves(vec![(
        Curve::Circle(Circle {
            frame: Frame::from_z(centre, n),
            radius: h2.sqrt(),
        }),
        Contact::Crossing,
    )])
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::testing::{rand_frame, rand_unit};
    use forge_geom::{CurveGeom, SurfaceGeom};
    use forge_math::rng::Rng;

    fn surfaces(a: &Surface, b: &Surface) -> SurfaceSurface {
        let far = forge_math::Point3::new(100.0, 100.0, 100.0);
        surfaces_in(
            a,
            b,
            Box3::from_points([far, forge_math::Point3::ORIGIN - far.to_vec()]),
        )
    }

    fn on_both(a: &Surface, b: &Surface) {
        if let SurfaceSurface::Curves(cs) = surfaces(a, b) {
            for (c, contact) in cs {
                let tolerance = if contact == Contact::Touching {
                    1.01 * tol::LINEAR
                } else {
                    1e-7
                };
                let range = c.full_range(20.0);
                for i in 0..=20 {
                    let p = c.eval((range.lo + range.width() * i as f64 / 20.0).min(range.hi));
                    assert!(
                        a.distance(p) <= tolerance && b.distance(p) <= tolerance,
                        "{a:?} {b:?} {c:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn parallel_cylinders_and_spheres() {
        let mut r = Rng::new(900);
        let mut curves = 0;
        for _ in 0..2000 {
            let f = rand_frame(&mut r);
            let g = f.with_origin(f.origin + f.x() * r.range(0.0, 8.0) + f.z() * r.range(-5.0, 5.0));
            let c = Surface::Cylinder(Cylinder {
                frame: f,
                radius: r.range(1.0, 5.0),
            });
            let d = Surface::Cylinder(Cylinder {
                frame: g,
                radius: r.range(1.0, 5.0),
            });
            on_both(&c, &d);
            let s = Surface::Sphere(Sphere {
                frame: f,
                radius: r.range(1.0, 5.0),
            });
            let t = Surface::Sphere(Sphere {
                frame: g.with_origin(g.origin + rand_unit(&mut r) * 1.0),
                radius: r.range(1.0, 5.0),
            });
            on_both(&s, &t);
            if matches!(surfaces(&c, &d), SurfaceSurface::Curves(_)) {
                curves += 1;
            }
        }
        assert!(curves > 500);
        let c = Surface::Cylinder(Cylinder {
            frame: Frame::WORLD,
            radius: 2.0,
        });
        assert_eq!(surfaces(&c, &c), SurfaceSurface::Coincident);
    }

    /// Memory on and off: a pair the tracer takes, to compare by all it returns.
    fn pair() -> (Surface, Surface, Box3) {
        let cyl = Surface::Cylinder(Cylinder {
            frame: Frame::WORLD,
            radius: 5.0,
        });
        let tor = Surface::Torus(forge_geom::Torus {
            frame: Frame::from_z(forge_math::Point3::new(5.0, 0.0, 0.0), forge_math::UnitVec3::X),
            major: 6.0,
            minor: 1.5,
        });
        let r = Box3::from_point(forge_math::Point3::ORIGIN).grow(30.0);
        (cyl, tor, r)
    }

    fn text(r: &SurfaceSurface) -> String {
        format!("{r:?}")
    }

    /// Asked on a new thread, so with nothing remembered.
    fn cold(a: &Surface, b: &Surface, r: Box3) -> String {
        let (a, b) = (a.clone(), b.clone());
        std::thread::spawn(move || text(&searched(&a, &b, r)))
            .join()
            .unwrap()
    }

    fn moved_digit(x: f64) -> f64 {
        f64::from_bits(x.to_bits() + 1)
    }

    #[test]
    #[cfg_attr(target_family = "wasm", ignore = "needs a thread")]
    fn a_surface_a_last_digit_off_is_another_question() {
        searched_keeps(4096);
        let (a, b, r) = pair();
        let first = text(&searched(&a, &b, r));
        assert!(first.contains("Intersection"), "{first}");
        assert_eq!(first, text(&searched(&a, &b, r)));
        assert_eq!(first, cold(&a, &b, r));
        let Surface::Cylinder(c) = &a else { unreachable!() };
        let other = Surface::Cylinder(Cylinder {
            frame: c.frame,
            radius: moved_digit(c.radius),
        });
        let second = text(&searched(&other, &b, r));
        assert_ne!(first, second);
        assert_eq!(second, cold(&other, &b, r));
        // and the first still answers as it did
        assert_eq!(first, text(&searched(&a, &b, r)));
        // the other way round is its own question
        assert_eq!(cold(&b, &a, r), text(&searched(&b, &a, r)));
    }

    #[test]
    fn every_field_is_in_the_bits() {
        use forge_geom::{Cone, Plane, Torus};
        let f = Frame::WORLD;
        let g = f.with_origin(forge_math::Point3::new(moved_digit(1.0), 0.0, 0.0));
        let h = f.with_origin(forge_math::Point3::new(1.0, 0.0, 0.0));
        let turned = Frame::from_z(f.origin, forge_math::UnitVec3::X);
        assert_ne!(
            Surface::Plane(Plane { frame: g }).bits(),
            Surface::Plane(Plane { frame: h }).bits()
        );
        assert_ne!(
            Surface::Plane(Plane { frame: f }).bits(),
            Surface::Plane(Plane { frame: turned }).bits()
        );
        let cyl = |frame, radius| Surface::Cylinder(Cylinder { frame, radius }).bits();
        assert_ne!(cyl(f, 1.0), cyl(f, moved_digit(1.0)));
        assert_ne!(cyl(g, 1.0), cyl(h, 1.0));
        let sph = |frame, radius| Surface::Sphere(Sphere { frame, radius }).bits();
        assert_ne!(sph(f, 1.0), sph(f, moved_digit(1.0)));
        assert_ne!(sph(g, 1.0), sph(h, 1.0));
        let tor = |frame, major, minor| Surface::Torus(Torus { frame, major, minor }).bits();
        assert_ne!(tor(f, 3.0, 1.0), tor(f, moved_digit(3.0), 1.0));
        assert_ne!(tor(f, 3.0, 1.0), tor(f, 3.0, moved_digit(1.0)));
        assert_ne!(tor(g, 3.0, 1.0), tor(h, 3.0, 1.0));
        let cone = |a: f64, r: f64, frame| Surface::Cone(Cone::new(frame, r, a).expect("a cone")).bits();
        assert_ne!(cone(0.5, 1.0, f), cone(moved_digit(0.5), 1.0, f));
        assert_ne!(cone(0.5, 1.0, f), cone(0.5, moved_digit(1.0), f));
        assert_ne!(cone(0.5, 1.0, g), cone(0.5, 1.0, h));
        // a zero's sign: `==` calls them equal, the key does not
        let zero = f.with_origin(forge_math::Point3::new(0.0, 0.0, 0.0));
        let minus = f.with_origin(forge_math::Point3::new(-0.0, 0.0, 0.0));
        assert_ne!(cyl(zero, 1.0), cyl(minus, 1.0));
        // kinds apart
        assert_ne!(cyl(f, 1.0), sph(f, 1.0));
    }

    #[test]
    fn a_hash_that_collides_finds_nothing_false() {
        // every key on one list: only comparing keys can tell them apart
        #[derive(Default)]
        struct Same;
        impl std::hash::Hasher for Same {
            fn write(&mut self, _: &[u8]) {}
            fn finish(&self) -> u64 {
                7
            }
        }
        let (a, b, r) = pair();
        use forge_math::Interval;
        let (k0, k1) = (met(&a, &b, &r), met(&b, &a, &r));
        let mut m: std::collections::HashMap<Met, u32, std::hash::BuildHasherDefault<Same>> =
            Default::default();
        m.insert(k0.clone(), 0);
        assert_eq!(m.get(&k1), None);
        m.insert(k1.clone(), 1);
        assert_eq!((m[&k0], m[&k1]), (0, 1));
        let r2 = Box3 {
            x: Interval::new(r.x.lo, moved_digit(r.x.hi)),
            ..r
        };
        assert_eq!(m.get(&met(&a, &b, &r2)), None);
    }

    #[test]
    #[cfg_attr(target_family = "wasm", ignore = "needs a thread")]
    fn keeping_little_or_nothing_answers_the_same() {
        let (a, b, r) = pair();
        let want = cold(&a, &b, r);
        for most in [0, 1, 2, 5] {
            searched_keeps(most);
            for k in 0..4 {
                // something else asked in between, to push it out
                let other = Surface::Cylinder(Cylinder {
                    frame: Frame::WORLD,
                    radius: 5.0 + 0.1 * k as f64,
                });
                searched(&other, &b, r);
                assert_eq!(want, text(&searched(&a, &b, r)), "keeping {most}");
            }
        }
        searched_keeps(4096);
    }

    /// The answer depends on the box (at 1e-13 here, not at 1e-14), so a box near one known must not take its answer:
    /// that made an n-ary regen's result hang on which thread took the pair, so on load. Fails with a near match.
    #[test]
    #[cfg_attr(target_family = "wasm", ignore = "needs a thread")]
    fn a_box_a_little_off_is_another_question() {
        use forge_math::Interval;
        searched_keeps(4096);
        let (a, b, r) = pair();
        let first = text(&searched(&a, &b, r));
        let mut told = false;
        for off in [1e-15, 1e-14, 1e-13, 1e-12, 5e-12] {
            let near = Box3 {
                x: Interval::new(r.x.lo, r.x.hi + off),
                ..r
            };
            let alone = cold(&a, &b, near);
            told |= alone != first;
            assert_eq!(text(&searched(&a, &b, near)), alone, "{off:e}");
        }
        assert!(told, "no box told apart: this no longer tests anything");
    }

    /// A procedural surface goes by its address: a key must hold the surface, or the address could be another's.
    #[test]
    fn a_new_surface_at_a_freed_address_is_not_taken_for_the_old() {
        use forge_geom::ruled::{Rail, Ruled};
        use forge_math::Point3;
        searched_keeps(4096);
        let plane = Surface::Plane(forge_geom::Plane {
            frame: Frame::from_z(Point3::new(0.0, 0.0, 0.5), forge_math::UnitVec3::Z),
        });
        let region = Box3::from_point(Point3::ORIGIN).grow(20.0);
        let wall = |x: f64| {
            let line = |z: f64| Rail::Line {
                a: Point3::new(x, -3.0, z),
                b: Point3::new(x, 3.0 + x * 0.1, z),
            };
            Ruled::new(line(0.0), line(1.0)).expect("a wall").surface()
        };
        for k in 0..200 {
            let x = 1.0 + k as f64 * 0.01;
            let s = wall(x);
            if let SurfaceSurface::Curves(cs) = searched(&s, &plane, region) {
                for (c, _) in &cs {
                    if let Curve::Intersection(ic) = c {
                        assert!(ic.a == s, "an answer for another surface");
                    }
                }
            }
        }
    }

    /// A rod lying on a rounded plate's top plane touches the rim's torus where its bottom line crosses
    /// the rim's crown. Off the faces that stops nothing; on them it is still a refusal.
    #[test]
    fn a_stall_counts_only_on_the_faces() {
        use forge_geom::Torus;
        use forge_math::Vec3;
        let (o, y) = (
            Point3::new(-197.12941588811063, -6.0, 45.512016370977626),
            -Vec3::Y,
        );
        let frame = Frame::from_axes(o, Vec3::X, Vec3::Z, y).unwrap();
        let torus = Surface::Torus(Torus::new(frame, 49.0, 6.0).unwrap());
        let along = Vec3::new(-0.9841236656984402, 0.0, -0.1774841136898307);
        let rod = Surface::Cylinder(Cylinder {
            frame: Frame::from_z(
                Point3::new(-21.65393134863068, 10.000000000000002, 120.06790834721227),
                along.normalize().unwrap(),
            ),
            radius: 10.0,
        });
        let region = Box3::from_points([
            Point3::new(-206.39862274227644, -0.22540519861131517, 87.06967569277985),
            Point3::new(-142.12940588811063, 9.994697390738665, 100.51202637097762),
        ]);
        assert!(forge_geom::intcurve::trace(&rod, &torus, region).is_none());
        let got = past_stalls(&rod, &torus, region, |_| true);
        assert!(past_stalls(&rod, &torus, region, |_| false).is_none());
        for (c, _) in got.expect("the branches off the touch") {
            let d = c.domain();
            for i in 0..=20 {
                let p = c.eval(d.lo + (d.hi - d.lo) * i as f64 / 20.0);
                assert!(rod.distance(p) <= 1e-4 && torus.distance(p) <= 1e-4);
            }
        }
    }

    /// A rod crossing a torus's axis meets the tube in two lobes, one each side, and on the rod's rulings
    /// the tube's two sides come and go together: the quartic's discriminant only touches zero there.
    #[test]
    fn a_rod_across_a_torus_axis_meets_it() {
        use forge_geom::Torus;
        use forge_math::Vec3;
        let torus = Surface::Torus(Torus::new(Frame::WORLD, 34.0, 3.0).unwrap());
        for (z, r) in [(4.0, 9.0), (4.0, 8.0), (2.0, 9.0)] {
            let rod = Surface::Cylinder(Cylinder {
                frame: Frame::from_axes(Point3::new(0.0, 22.0, z), Vec3::X, -Vec3::Z, Vec3::Y).unwrap(),
                radius: r,
            });
            let region = Box3::from_points([Point3::new(-40.0, 15.0, -5.0), Point3::new(40.0, 45.0, 15.0)]);
            let SurfaceSurface::Curves(cs) = surfaces_in(&rod, &torus, region) else {
                panic!("{z} {r}: they meet");
            };
            assert_eq!(cs.len(), 2, "{z} {r}");
            let sides: Vec<f64> = cs
                .iter()
                .map(|(c, _)| {
                    let d = c.domain();
                    let lo = if d.lo.is_finite() { d.lo } else { 0.0 };
                    c.eval(lo + 0.3 * (d.hi - lo)).x
                })
                .collect();
            assert!(sides[0] * sides[1] < 0.0, "{sides:?}");
        }
    }
}
