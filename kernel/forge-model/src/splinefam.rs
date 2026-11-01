//! Fuzz families of profiles with real curves: walls cut square, askew, by parallel holes and by
//! other curves, tangent and within a hair of tangent, and turned about an axis.

use crate::adversarial::fr;
use crate::{Combine, Extent, Feature, Part, Turn};
use forge_geom::spline::{PlaneSpline, uniform_knots};
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, Point3, Vec2, dmath};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment};
use std::sync::Arc;

fn seg(c: PlaneSpline) -> Segment {
    Segment::Spline(Arc::new(c))
}

fn ext(plane: Frame, loops: Vec<ProfileLoop>, d: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile { plane, loops },
        extent: Extent::Distance(d),
        op,
    }
}

/// A curve from left to right above y = 0, of a kind picked from what real files hold.
fn top_curve(r: &mut Rng, w: f64) -> PlaneSpline {
    let kind = r.below(4);
    let n = 4 + r.below(5);
    let pts: Vec<Point2> = (0..n)
        .map(|i| {
            let x = w * i as f64 / (n - 1) as f64;
            let edge = i == 0 || i == n - 1;
            Point2::new(x, if edge { 2.0 } else { r.range(3.0, 14.0) })
        })
        .collect();
    let c = match kind {
        0 => PlaneSpline::through(pts, None, None),
        1 => PlaneSpline::control(3, pts, None, None),
        2 => {
            let d = if n > 5 { 5 } else { 2 };
            let k = uniform_knots(d, n);
            let weights: Vec<f64> = (0..n).map(|_| r.range(0.5, 2.0)).collect();
            PlaneSpline::control(d, pts, Some(k), Some(weights))
        }
        _ => PlaneSpline::control(2, pts, None, None),
    }
    .expect("the points make a curve");
    // kept clear above y = 0, where `under` draws its floor
    let low = c.c.bbox(c.c.domain()).y.lo;
    if low < 1.0 {
        return c.remade(&|p| Point2::new(p.x, p.y + 1.0 - low), &|l| l, &|a| a, false);
    }
    c
}

/// The region under `c`, down to y = 0.
fn under(c: PlaneSpline) -> ProfileLoop {
    under_to(c, 0.0)
}

fn under_to(c: PlaneSpline, floor: f64) -> ProfileLoop {
    let (a, b) = (c.start(), c.end());
    let (p, q) = (Point2::new(b.x, floor), Point2::new(a.x, floor));
    ProfileLoop::new(vec![
        (seg(c), 0),
        (Segment::Line { a: b, b: p }, 1),
        (Segment::Line { a: p, b: q }, 2),
        (Segment::Line { a: q, b: a }, 3),
    ])
}

/// A frame with z along `z` (not unit) and x as near `x` as it can be.
fn tilted(o: [f64; 3], z: [f64; 3], x: [f64; 3]) -> Frame {
    let l = (z[0] * z[0] + z[1] * z[1] + z[2] * z[2]).sqrt();
    fr(o, [z[0] / l, z[1] / l, z[2] / l], x)
}

fn up_at(x: f64, y: f64, z: f64) -> Frame {
    fr([x, y, z], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0])
}

/// A plate under a curve, cut and built on in a few ways.
pub fn spline_walls(seed: u64) -> Part {
    let mut r = Rng::new(seed.wrapping_mul(2654435761).wrapping_add(11));
    let (w, h) = (r.range(20.0, 40.0), r.range(5.0, 15.0));
    let mut f = vec![ext(
        Frame::WORLD,
        vec![under(top_curve(&mut r, w))],
        h,
        Combine::New,
    )];
    for _ in 0..1 + r.below(3) {
        let (x, y) = (r.range(0.0, w), r.range(0.0, 12.0));
        f.push(match r.below(6) {
            // a round hole straight down: parallel to the rulings
            0 => ext(
                up_at(x, y, -1.0),
                vec![Profile::circle(Point2::ORIGIN, r.range(1.0, 5.0), 0)],
                h + 2.0,
                Combine::Cut,
            ),
            // a round bar across: a cylinder at right angles
            1 => {
                let z = r.range(1.0, h - 1.0);
                ext(
                    fr([x, -5.0, z], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
                    vec![Profile::circle(Point2::ORIGIN, r.range(0.5, 2.0), 0)],
                    40.0,
                    Combine::Cut,
                )
            }
            // cut away above a tilted plane
            2 => {
                let zn = [r.range(-0.6, 0.6), r.range(-0.6, 0.6), 1.0];
                ext(
                    tilted([w / 2.0, 6.0, r.range(1.0, h)], zn, [1.0, 0.0, 0.0]),
                    vec![Profile::rectangle(Point2::new(-60.0, -60.0), 120.0, 120.0, 0)],
                    60.0,
                    Combine::Cut,
                )
            }
            // a slot with square walls along the rulings
            3 => ext(
                up_at(x, 0.0, -1.0),
                vec![Profile::rectangle(
                    Point2::new(0.0, -2.0),
                    r.range(0.5, 6.0),
                    30.0,
                    0,
                )],
                h + 2.0,
                Combine::Cut,
            ),
            // another curve, its wall parallel: two sections crossing
            4 => {
                let dy = r.range(-6.0, 2.0);
                let c = top_curve(&mut r, w);
                let c = c.remade(&|p| Point2::new(p.x, p.y + dy), &|l| l, &|a| a, false);
                ext(
                    Frame::WORLD.with_origin(Point3::new(0.0, 0.0, -1.0)),
                    vec![under_to(c, -20.0)],
                    h + 2.0,
                    Combine::Cut,
                )
            }
            // a boss on top
            _ => ext(
                up_at(x, y.min(1.5), h),
                vec![Profile::circle(Point2::ORIGIN, r.range(0.5, 1.5), 0)],
                r.range(1.0, 5.0),
                Combine::Add,
            ),
        });
    }
    Part { features: f }
}

/// A plate under a curve with a tool laid tangent to the curve, or a small gap or overlap off
/// tangent, across the sizes round the tolerance.
pub fn spline_tangent(seed: u64) -> Part {
    let mut r = Rng::new(seed.wrapping_mul(40503).wrapping_add(7));
    let (w, h) = (r.range(20.0, 40.0), r.range(5.0, 15.0));
    let c = top_curve(&mut r, w);
    let gap = [
        0.0, 1e-9, -1e-9, 1e-7, -1e-7, 1e-6, -1e-6, 1e-5, -1e-5, 1e-4, -1e-4, 1e-3, -1e-3, 0.05, -0.05,
    ][r.below(15)];
    let dom = c.c.domain();
    let t = dom.lo + dom.width() * r.range(0.2, 0.8);
    let d = c.c.deriv(t);
    let Some(tan) = Vec2::new(d.d1.x, d.d1.y).normalize() else {
        return Part {
            features: vec![ext(Frame::WORLD, vec![under(c)], h, Combine::New)],
        };
    };
    let up = Vec2::new(-tan.y, tan.x);
    let on = Point2::new(d.p.x, d.p.y);
    let base = ext(Frame::WORLD, vec![under(c)], h, Combine::New);
    let tool = match r.below(3) {
        // a block whose flat side lies along the tangent
        0 => {
            let a = dmath::atan2(tan.y, tan.x);
            let o = on + up * gap;
            let (sn, cs) = dmath::sin_cos(a);
            ext(
                fr([o.x, o.y, -1.0], [0.0, 0.0, 1.0], [cs, sn, 0.0]),
                vec![Profile::rectangle(Point2::new(-8.0, 0.0), 16.0, 8.0, 0)],
                h + 2.0,
                if r.bool() { Combine::Cut } else { Combine::Add },
            )
        }
        // a round bar outside touching the wall
        1 => {
            let rad = r.range(0.5, 4.0);
            let o = on + up * (rad + gap);
            ext(
                up_at(o.x, o.y, -1.0),
                vec![Profile::circle(Point2::ORIGIN, rad, 0)],
                h + 2.0,
                if r.bool() { Combine::Cut } else { Combine::Add },
            )
        }
        // a plane through a ruling and tangent there, as a tilted cut would be: flat on the wall
        _ => {
            let a = dmath::atan2(tan.y, tan.x);
            let o = on + up * gap;
            let (sn, cs) = dmath::sin_cos(a);
            ext(
                fr([o.x, o.y, h * 0.5], [0.0, 0.0, 1.0], [cs, sn, 0.0]),
                vec![Profile::rectangle(Point2::new(-30.0, 0.0), 60.0, 20.0, 0)],
                h,
                Combine::Cut,
            )
        }
    };
    Part {
        features: vec![base, tool],
    }
}

/// A spline turned about an axis, cut square to it, through it, along it and across it.
pub fn spline_revolve(seed: u64) -> Part {
    let mut r = Rng::new(seed.wrapping_mul(69069).wrapping_add(3));
    let n = 4 + r.below(4);
    let hgt = r.range(15.0, 30.0);
    let rin = r.range(1.0, 4.0);
    let pts: Vec<Point2> = (0..n)
        .map(|i| {
            let y = hgt * i as f64 / (n - 1) as f64;
            let edge = i == 0 || i == n - 1;
            Point2::new(if edge { rin + 2.0 } else { rin + r.range(3.0, 12.0) }, y)
        })
        .collect();
    let c = if r.bool() {
        PlaneSpline::through(pts, None, None)
    } else {
        PlaneSpline::control(3, pts, None, None)
    }
    .expect("a curve");
    let (a, b) = (c.start(), c.end());
    let inner_a = Point2::new(rin, a.y);
    let inner_b = Point2::new(rin, b.y);
    let l = ProfileLoop::new(vec![
        (seg(c), 0),
        (Segment::Line { a: b, b: inner_b }, 1),
        (
            Segment::Line {
                a: inner_b,
                b: inner_a,
            },
            2,
        ),
        (Segment::Line { a: inner_a, b: a }, 3),
    ]);
    let stand = fr([0.0, 0.0, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]);
    let angle = if r.below(3) == 0 {
        r.range(1.0, 5.0)
    } else {
        dmath::TAU
    };
    let mut f = vec![Feature::Revolve {
        profile: Profile {
            plane: stand,
            loops: vec![l],
        },
        axis: Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::Y,
        },
        extent: Turn::Angle(angle),
        op: Combine::New,
    }];
    for _ in 0..r.below(3) {
        f.push(match r.below(4) {
            // square to the axis
            0 => ext(
                up_at(0.0, 0.0, r.range(2.0, hgt - 2.0)),
                vec![Profile::rectangle(Point2::new(-40.0, -40.0), 80.0, 80.0, 0)],
                50.0,
                Combine::Cut,
            ),
            // through the axis
            1 => ext(
                fr([0.0, 0.0, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                vec![Profile::rectangle(Point2::new(-40.0, 0.0), 80.0, 40.0, 0)],
                hgt + 2.0,
                Combine::Cut,
            ),
            // about the axis
            2 => ext(
                up_at(0.0, 0.0, -1.0),
                vec![Profile::circle(Point2::ORIGIN, rin + r.range(0.5, 4.0), 0)],
                hgt + 2.0,
                Combine::Cut,
            ),
            // across
            _ => ext(
                fr(
                    [-30.0, 0.0, r.range(3.0, hgt - 3.0)],
                    [1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                ),
                vec![Profile::circle(Point2::ORIGIN, r.range(0.5, 2.0), 0)],
                60.0,
                Combine::Cut,
            ),
        });
    }
    Part { features: f }
}

/// Ellipses whole and in part, cut by planes and round holes.
pub fn ellipse_walls(seed: u64) -> Part {
    let mut r = Rng::new(seed.wrapping_mul(1000003).wrapping_add(19));
    let (a, b) = (r.range(5.0, 20.0), r.range(2.0, 5.0));
    let rot = r.range(0.0, 3.0);
    let h = r.range(4.0, 12.0);
    let e = PlaneSpline::ellipse(Point2::ORIGIN, a, b, rot, 0.0, dmath::TAU).expect("axes positive");
    let mut f = vec![ext(
        Frame::WORLD,
        vec![ProfileLoop::new(vec![(seg(e), 0)])],
        h,
        Combine::New,
    )];
    for _ in 0..r.below(3) {
        f.push(match r.below(3) {
            0 => ext(
                up_at(r.range(-b, b), r.range(-b, b), -1.0),
                vec![Profile::circle(Point2::ORIGIN, r.range(0.3, 1.5), 0)],
                h + 2.0,
                Combine::Cut,
            ),
            1 => ext(
                tilted(
                    [0.0, 0.0, r.range(1.0, h)],
                    [r.range(-0.5, 0.5), r.range(-0.5, 0.5), 1.0],
                    [1.0, 0.0, 0.0],
                ),
                vec![Profile::rectangle(Point2::new(-40.0, -40.0), 80.0, 80.0, 0)],
                40.0,
                Combine::Cut,
            ),
            _ => {
                let inner = PlaneSpline::ellipse(
                    Point2::ORIGIN,
                    0.5 * a,
                    0.5 * b,
                    rot + r.range(0.0, 0.6),
                    0.0,
                    dmath::TAU,
                )
                .expect("axes positive");
                ext(
                    Frame::WORLD.with_origin(Point3::new(0.0, 0.0, -1.0)),
                    vec![ProfileLoop::new(vec![(seg(inner), 0)])],
                    h + 2.0,
                    Combine::Cut,
                )
            }
        });
    }
    Part { features: f }
}

/// A profile along a curve, round a bend and wound in a coil, some of it a curve itself, then cut.
pub fn spline_sweep(seed: u64) -> Part {
    use forge_geom::spline::Spline;
    use forge_ops::{Handed, PathSeg};
    let mut r = Rng::new(seed.wrapping_mul(16807).wrapping_add(5));
    let s = r.range(1.0, 2.0);
    let round = r.bool();
    let section = |r: &mut Rng| -> ProfileLoop {
        let (a, b) = (Point2::new(-s, 0.0), Point2::new(s, 0.0));
        match r.below(3) {
            0 => Profile::circle(Point2::ORIGIN, s, 0),
            1 => Profile::rectangle(Point2::new(-s, -s), 2.0 * s, 2.0 * s, 0),
            _ => {
                let c =
                    PlaneSpline::through(vec![a, Point2::new(0.0, s), b], None, None).expect("three points");
                ProfileLoop::new(vec![(seg(c), 0), (Segment::Line { a: b, b: a }, 1)])
            }
        }
    };
    let l = section(&mut r);
    let mut f = Vec::new();
    match r.below(3) {
        0 => {
            // a curve in the xy plane, starting along x
            let n = 4 + r.below(3);
            let mut pts = vec![Point3::ORIGIN, Point3::new(6.0, 0.0, 0.0)];
            let mut x = 6.0;
            for _ in 0..n - 2 {
                x += r.range(8.0, 14.0);
                pts.push(Point3::new(x, r.range(-4.0, 4.0), 0.0));
            }
            let k = uniform_knots(3, pts.len());
            let c = Spline::from_nurbs(3, &k, &pts, None).expect("a curve");
            let plane = fr([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
            let mut path = vec![PathSeg::Spline(Arc::new(c.clone()))];
            if round {
                let (e, d) = (c.ends().1, c.deriv(c.domain().hi).d1);
                path.push(PathSeg::Line {
                    a: e,
                    b: e + d.normalize().expect("a tangent").get() * 5.0,
                });
            }
            f.push(Feature::Sweep(Box::new(crate::Sweep {
                profile: Profile {
                    plane,
                    loops: vec![l],
                },
                path,
                ends: crate::PathEnds::Path,
                op: Combine::New,
            })));
        }
        1 => {
            let plane = fr([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
            let circle = forge_geom::Circle {
                frame: fr([0.0, 15.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]),
                radius: 15.0,
            };
            f.push(Feature::Sweep(Box::new(crate::Sweep {
                profile: Profile {
                    plane,
                    loops: vec![l],
                },
                path: vec![PathSeg::Arc {
                    circle,
                    start: 0.0,
                    sweep: r.range(0.5, 3.0),
                }],
                ends: crate::PathEnds::Path,
                op: Combine::New,
            })));
        }
        _ => {
            // a coil of a curved section, out from the axis
            let mid = r.range(8.0, 14.0);
            let (a, b) = (Point2::new(mid - s, 0.0), Point2::new(mid + s, 0.0));
            let c = PlaneSpline::through(vec![a, Point2::new(mid, s), b], None, None).expect("three points");
            let sec = ProfileLoop::new(vec![(seg(c), 0), (Segment::Line { a: b, b: a }, 1)]);
            f.push(Feature::Coil(Box::new(crate::Coil {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]),
                    loops: vec![sec],
                },
                axis: Axis2 {
                    origin: Point2::ORIGIN,
                    dir: Vec2::Y,
                },
                size: crate::CoilSize::PitchTurns {
                    pitch: s + r.range(1.0, 4.0),
                    turns: r.range(0.7, 3.0),
                },
                taper: 0.0,
                handed: if r.bool() { Handed::Right } else { Handed::Left },
                op: Combine::New,
            })));
        }
    }
    let (x, y) = (r.range(2.0, 25.0), r.range(-8.0, 12.0));
    if r.below(3) > 0 {
        f.push(ext(
            up_at(x, y, -10.0),
            vec![Profile::rectangle(
                Point2::new(-20.0, -20.0),
                40.0,
                r.range(5.0, 40.0),
                0,
            )],
            20.0,
            Combine::Cut,
        ));
    }
    Part { features: f }
}
