//! Coverage group 5's families (research/coverage-2026-11-23): surface pairs the corpus never ran,
//! kept out of forge-model's library (its code and layout as they were): this crate's
//! exact-curved and forge-model's mesh_valid test (by path) use them.
use forge_math::rng::Rng;
use forge_math::{Frame, Vec2};
use forge_math::{Point2, Point3, Vec3, dmath};
use forge_model::adversarial::fr;
use forge_model::{Combine, Extent, Feature, Part, Turn};
use forge_ops::Profile;
use forge_ops::{Axis2, ProfileLoop, Segment};

/// The families, apart from `adversarial::families` (whose list, and the code built from it, stay
/// as they were): harnesses chain both.
pub fn families() -> Vec<(&'static str, forge_model::adversarial::Family)> {
    vec![
        ("apex_planes", apex_planes),
        ("apex_cones", apex_cones),
        ("near_parallel", near_parallel),
        ("spindle_tori", spindle_tori),
    ]
}

fn op3(r: &mut Rng) -> Combine {
    [Combine::Add, Combine::Cut, Combine::Keep][r.below(3)]
}

/// A slab 60 across whose face through `o` has outward normal `n`, the solid on the -n side.
fn slab(o: Point3, n: Vec3, op: Combine) -> Feature {
    let z = unit(-n);
    ext(
        frame(o, z, perp(z)),
        vec![Profile::rectangle(Point2::new(-30.0, -30.0), 60.0, 60.0, 0)],
        30.0,
        op,
    )
}

/// Offsets in LINEARs near a coincidence: on it, inside and outside the tolerance, and clear.
const NEAR: [f64; 7] = [0.0, 0.5, -0.5, 3.0, -3.0, 60.0, -60.0];

/// A cone (apex at the origin, axis +z, height 20, random half-angle) cut by a plane through or
/// near its apex: perpendicular (a point), at the generator's angle (a parabola, or one line
/// through the apex), steeper (two lines or a hyperbola), and a hair either side of each.
pub fn apex_planes(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0xA9E1);
    let r0 = r.range(3.0, 15.0);
    let half = dmath::atan2(r0, 20.0);
    let lin = forge_math::tol::LINEAR;
    // the angle of the plane's normal from the axis
    let theta = match r.below(6) {
        0 => 0.0,
        1 => dmath::FRAC_PI_2 - half,
        2 => dmath::FRAC_PI_2 - half + [1e-9, -1e-9, 1e-7, -1e-7, 1e-4][r.below(5)],
        3 => dmath::FRAC_PI_2,
        4 => r.range(0.0, dmath::FRAC_PI_2 - half),
        _ => r.range(dmath::FRAC_PI_2 - half, dmath::FRAC_PI_2),
    };
    let phi = r.range(0.0, dmath::TAU);
    let (st, ct) = dmath::sin_cos(theta);
    let (sp, cp) = dmath::sin_cos(phi);
    let n = Vec3::new(st * cp, st * sp, ct);
    let off = if r.below(4) == 0 {
        r.range(-10.0, 10.0)
    } else {
        NEAR[r.below(7)] * lin
    };
    Part {
        features: vec![
            cone(p3(0.0, 0.0, -20.0), Z, r0, 0.0, 20.0, Combine::New),
            slab(Point3::ORIGIN + n * off, n, op3(&mut r)),
        ],
    }
}

/// Two cones with one apex (or apexes a hair apart): coaxial, opposed, along a generator of the
/// first, perpendicular, at random.
pub fn apex_cones(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0xA9E2);
    let r0 = r.range(3.0, 15.0);
    let half = dmath::atan2(r0, 20.0);
    let lin = forge_math::tol::LINEAR;
    let phi = r.range(0.0, dmath::TAU);
    let (sp, cp) = dmath::sin_cos(phi);
    let (sh, ch) = dmath::sin_cos(half);
    let a = match r.below(6) {
        0 => Z,
        1 => -Z,
        // down a generator of the first, into it
        2 => Vec3::new(-sh * cp, -sh * sp, -ch),
        // up the mirror of a generator
        3 => Vec3::new(sh * cp, sh * sp, ch),
        4 => Vec3::new(cp, sp, 0.0),
        _ => {
            let t = r.range(0.0, dmath::PI);
            let (st, ct) = dmath::sin_cos(t);
            Vec3::new(st * cp, st * sp, ct)
        }
    };
    let d = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0));
    let apex = Point3::ORIGIN + d * (NEAR[r.below(5)] * lin);
    let r1 = [r0, r.range(2.0, 15.0)][r.below(2)];
    Part {
        features: vec![
            cone(p3(0.0, 0.0, -20.0), Z, r0, 0.0, 20.0, Combine::New),
            cone(apex, a, 0.0, r1, 20.0, op3(&mut r)),
        ],
    }
}

/// A cylinder (axis z, r 10) and a plane a hair off parallel to its axis, at distances from the
/// axis through it, tangent to it and clear of it.
pub fn near_parallel(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0xA9E3);
    let lin = forge_math::tol::LINEAR;
    let tilt = [0.0, 1e-12, 1e-9, 1e-7, 1e-5, 1e-3][r.below(6)] * [1.0, -1.0][r.below(2)];
    let d = match r.below(4) {
        0 => 0.0,
        1 => r.range(0.0, 10.0),
        _ => 10.0 + NEAR[r.below(7)] * lin,
    };
    let phi = r.range(0.0, dmath::TAU);
    let (sp, cp) = dmath::sin_cos(phi);
    let (st, ct) = dmath::sin_cos(tilt);
    let n = Vec3::new(ct * cp, ct * sp, st);
    Part {
        features: vec![
            cyl(p3(0.0, 0.0, -20.0), Z, 10.0, 20.0, Combine::New),
            slab(Point3::new(d * cp, d * sp, -10.0), n, op3(&mut r)),
        ],
    }
}

/// Horn and spindle tori (tube >= the centre radius), with a plane through the centre, a ball
/// and a cylinder on the axis.
pub fn spindle_tori(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0xA9E4);
    let big = r.range(3.0, 8.0);
    let tube = big * [1.0, 1.2, 1.5, 1.0 + 1e-6][r.below(4)];
    let tool = match r.below(4) {
        0 => slab(Point3::ORIGIN, Z, op3(&mut r)),
        1 => ball_at(p3(0.0, 0.0, r.range(-2.0, 2.0)), r.range(1.0, 6.0), op3(&mut r)),
        2 => cyl(p3(0.0, 0.0, -20.0), Z, r.range(0.5, 4.0), 40.0, op3(&mut r)),
        _ => slab(
            Point3::new(0.0, 0.0, r.range(-tube, tube)),
            Vec3::new(r.range(-0.3, 0.3), 0.0, 1.0),
            op3(&mut r),
        ),
    };
    Part {
        features: vec![torus(Point3::ORIGIN, Z, big, tube, Combine::New), tool],
    }
}

// (private copies of adversarial.rs's helpers: that file and its code stay as they were)
const Z: Vec3 = Vec3::Z;

fn frame(o: Point3, z: Vec3, x: Vec3) -> Frame {
    let z = z.normalize().expect("nonzero axis").get();
    fr([o.x, o.y, o.z], [z.x, z.y, z.z], [x.x, x.y, x.z])
}

fn ext(plane: Frame, loops: Vec<ProfileLoop>, d: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile { plane, loops },
        extent: Extent::Distance(d),
        op,
    }
}

fn ball_at(c: Point3, r: f64, op: Combine) -> Feature {
    Feature::Ball {
        frame: frame(c, Vec3::Y, Vec3::X),
        radius: r,
        op,
    }
}

fn perp(a: Vec3) -> Vec3 {
    if a.z.abs() < 0.9 {
        Vec3::new(-a.y, a.x, 0.0)
    } else {
        Vec3::new(a.z, 0.0, -a.x)
    }
}

fn unit(a: Vec3) -> Vec3 {
    a.normalize().expect("nonzero axis").get()
}

/// An extruded cylinder: axis from `o` along `a`.
fn cyl(o: Point3, a: Vec3, r: f64, h: f64, op: Combine) -> Feature {
    let a = unit(a);
    ext(
        frame(o, a, perp(a)),
        vec![Profile::circle(Point2::ORIGIN, r, 0)],
        h,
        op,
    )
}

/// A solid of revolution about the axis from `o` along `a`; points are (radius, height).
fn spun(o: Point3, a: Vec3, pts: &[(f64, f64)], op: Combine) -> Feature {
    let a = unit(a);
    let n = pts.len();
    let p = |i: usize| Point2::new(pts[i % n].0, pts[i % n].1);
    let segs = (0..n)
        .map(|i| (Segment::Line { a: p(i), b: p(i + 1) }, i as u64))
        .collect();
    Feature::Revolve {
        profile: Profile {
            plane: axis_frame(o, a, perp(a)),
            loops: vec![ProfileLoop::new(segs)],
        },
        axis: Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::Y,
        },
        extent: Turn::Angle(dmath::TAU),
        op,
    }
}

/// A cone or frustum: radius `r0` at `o`, `r1` at `o + a h` (either may be 0, an apex).
fn cone(o: Point3, a: Vec3, r0: f64, r1: f64, h: f64, op: Combine) -> Feature {
    let mut pts = vec![(0.0, 0.0)];
    if r0 > 0.0 {
        pts.push((r0, 0.0));
    }
    if r1 > 0.0 {
        pts.push((r1, h));
    }
    pts.push((0.0, h));
    spun(o, a, &pts, op)
}

/// A torus about the axis through `o` along `a`.
fn torus(o: Point3, a: Vec3, big: f64, tube: f64, op: Combine) -> Feature {
    let a = unit(a);
    Feature::Revolve {
        profile: Profile {
            plane: axis_frame(o, a, perp(a)),
            loops: vec![Profile::circle(Point2::new(big, 0.0), tube, 0)],
        },
        axis: Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::Y,
        },
        extent: Turn::Angle(dmath::TAU),
        op,
    }
}

fn p3(x: f64, y: f64, z: f64) -> Point3 {
    Point3::new(x, y, z)
}

fn axis_frame(o: Point3, a: Vec3, u: Vec3) -> Frame {
    frame(o, u.cross(a), u)
}
