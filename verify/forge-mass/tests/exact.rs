//! `forge_topo::mass` on solids with closed-form mass properties, in random poses. On these faces
//! (planes, cylinders, cones, spheres, tori, bounded by lines and circles) every rule the kernel picks
//! is exact over the reals (lean/ForgeMass), so what is left is rounding.
//!
//! - `exact_up_to_rounding`: volume, area and principal moments agree with the closed forms to a
//!   few parts in 1e13.
//! - `reported_error_covers_the_difference`: `err` (volume, area) holds the difference from the
//!   closed form.

use forge_math::rng::Rng;
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec3, dmath};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment, extrude, revolve};
use forge_topo::Body;
use forge_topo::mass::{MassProps, mass};

const PI: f64 = dmath::PI;

struct Case {
    name: String,
    body: Body,
    volume: f64,
    area: f64,
    /// principal moments, smallest first (none where not worked out)
    principal: Option<[f64; 3]>,
}

fn frame(r: &mut Rng) -> Frame {
    let z = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0));
    let z = z.normalize().unwrap_or(UnitVec3::Z);
    let origin = Point3::new(
        r.range(-100.0, 100.0),
        r.range(-100.0, 100.0),
        r.range(-100.0, 100.0),
    );
    Frame::from_zx(origin, z, Vec3::new(r.range(-1.0, 1.0), 1.0, r.range(-1.0, 1.0))).unwrap_or(Frame::WORLD)
}

fn sorted(mut p: [f64; 3]) -> [f64; 3] {
    p.sort_by(f64::total_cmp);
    p
}

const Y: Axis2 = Axis2 {
    origin: Point2::ORIGIN,
    dir: forge_math::Vec2 { x: 0.0, y: 1.0 },
};

fn cases(seed: u64) -> Vec<Case> {
    let mut r = Rng::new(seed);
    let mut out = Vec::new();
    // a box
    let (w, h, t) = (r.range(1.0, 50.0), r.range(1.0, 50.0), r.range(1.0, 50.0));
    let p = Profile {
        plane: frame(&mut r),
        loops: vec![Profile::rectangle(
            Point2::new(r.range(-5.0, 5.0), r.range(-5.0, 5.0)),
            w,
            h,
            0,
        )],
    };
    let m = w * h * t;
    out.push(Case {
        name: format!("box {w} {h} {t}"),
        body: extrude(&p, t, 1).unwrap(),
        volume: m,
        area: 2.0 * (w * h + h * t + t * w),
        principal: Some(sorted([
            m * (h * h + t * t) / 12.0,
            m * (w * w + t * t) / 12.0,
            m * (w * w + h * h) / 12.0,
        ])),
    });
    // a cylinder
    let (rad, h) = (r.range(1.0, 30.0), r.range(1.0, 50.0));
    let p = Profile {
        plane: frame(&mut r),
        loops: vec![Profile::circle(
            Point2::new(r.range(-5.0, 5.0), r.range(-5.0, 5.0)),
            rad,
            0,
        )],
    };
    let m = PI * rad * rad * h;
    let side = m * (3.0 * rad * rad + h * h) / 12.0;
    out.push(Case {
        name: format!("cylinder {rad} {h}"),
        body: extrude(&p, h, 1).unwrap(),
        volume: m,
        area: 2.0 * PI * rad * rad + 2.0 * PI * rad * h,
        principal: Some(sorted([m * rad * rad / 2.0, side, side])),
    });
    // a cone frustum: a trapezoid round the y axis
    let (r0, r1, h) = (r.range(1.0, 30.0), r.range(1.0, 30.0), r.range(1.0, 50.0));
    let pts = [
        Point2::new(0.0, 0.0),
        Point2::new(r0, 0.0),
        Point2::new(r1, h),
        Point2::new(0.0, h),
    ];
    let segs = (0..4)
        .map(|i| {
            (
                Segment::Line {
                    a: pts[i],
                    b: pts[(i + 1) % 4],
                },
                i as u64,
            )
        })
        .collect();
    let p = Profile {
        plane: frame(&mut r),
        loops: vec![ProfileLoop::new(segs)],
    };
    let slant = ((r0 - r1) * (r0 - r1) + h * h).sqrt();
    out.push(Case {
        name: format!("frustum {r0} {r1} {h}"),
        body: revolve(&p, Y, dmath::TAU, 1).unwrap(),
        volume: PI * h * (r0 * r0 + r0 * r1 + r1 * r1) / 3.0,
        area: PI * (r0 * r0 + r1 * r1) + PI * (r0 + r1) * slant,
        principal: None,
    });
    // a sphere: a half disc round the y axis
    let big = r.range(1.0, 30.0);
    let p = Profile {
        plane: frame(&mut r),
        loops: vec![ProfileLoop::new(vec![
            (
                Segment::Arc {
                    center: Point2::ORIGIN,
                    radius: big,
                    start: -PI / 2.0,
                    sweep: PI,
                },
                0,
            ),
            (
                Segment::Line {
                    a: Point2::new(0.0, big),
                    b: Point2::new(0.0, -big),
                },
                1,
            ),
        ])],
    };
    let m = 4.0 / 3.0 * PI * big * big * big;
    let i = 0.4 * m * big * big;
    out.push(Case {
        name: format!("sphere {big}"),
        body: revolve(&p, Y, dmath::TAU, 1).unwrap(),
        volume: m,
        area: 4.0 * PI * big * big,
        principal: Some([i, i, i]),
    });
    // a ring torus: a circle round the y axis
    let (tube, ring) = (r.range(1.0, 10.0), r.range(0.0, 30.0));
    let ring = tube + 0.5 + ring;
    let p = Profile {
        plane: frame(&mut r),
        loops: vec![Profile::circle(Point2::new(ring, r.range(-5.0, 5.0)), tube, 0)],
    };
    let m = 2.0 * PI * PI * ring * tube * tube;
    let perp = m * (ring * ring / 2.0 + 5.0 / 8.0 * tube * tube);
    out.push(Case {
        name: format!("torus {ring} {tube}"),
        body: revolve(&p, Y, dmath::TAU, 1).unwrap(),
        volume: m,
        area: 4.0 * PI * PI * ring * tube,
        principal: Some(sorted([perp, perp, m * (ring * ring + 0.75 * tube * tube)])),
    });
    out
}

fn rel(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs()
}

fn all() -> Vec<(Case, MassProps)> {
    (0..40u64)
        .flat_map(cases)
        .map(|c| {
            let m = mass(&c.body).unwrap_or_else(|e| panic!("{}: {e:?}", c.name));
            (c, m)
        })
        .collect()
}

#[test]
fn exact_up_to_rounding() {
    let mut worst = (0.0f64, String::new());
    for (c, m) in all() {
        let scale = c.volume.abs();
        let dv = rel(m.volume, c.volume);
        let da = rel(m.area, c.area);
        let dp = c.principal.map_or(0.0, |p| {
            let big = p[2];
            (0..3)
                .map(|k| (m.principal[k] - p[k]).abs() / big)
                .fold(0.0, f64::max)
        });
        for (what, d) in [("volume", dv), ("area", da), ("principal", dp)] {
            assert!(
                d < 1e-12,
                "{}: {what} off by {d:e} (volume {} vs {scale})",
                c.name,
                m.volume
            );
            if d > worst.0 {
                worst = (d, format!("{} {what}", c.name));
            }
        }
    }
    eprintln!("worst relative difference: {:e} ({})", worst.0, worst.1);
}

#[test]
fn reported_error_covers_the_difference() {
    let mut slack = f64::INFINITY;
    for (c, m) in all() {
        for (what, got, want, err) in [
            ("volume", m.volume, c.volume, m.err[0]),
            ("area", m.area, c.area, m.err[1]),
        ] {
            let off = (got - want).abs();
            // the closed form's own rounding: a few ulps of the value
            let own = 8.0 * f64::EPSILON * want.abs();
            assert!(off <= err + own, "{}: {what} off {off:e}, err {err:e}", c.name);
            slack = slack.min(err / off.max(f64::MIN_POSITIVE));
        }
    }
    eprintln!("smallest err / difference: {slack:e}");
}
