//! Parts built to break the kernel: exact coincidences, tangencies, features
//! through vertices, slivers and near-tolerance offsets. Each family is a
//! deterministic generator; `outcome` classifies a regeneration (error,
//! invalid solid, or panic) and `shrink` reduces a failing part to a minimal
//! history that fails the same way.

use crate::{Combine, Feature, Part};
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec2, Vec3, dmath};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment};

/// A frame rebuilt the way the literal printer writes it, so a printed repro
/// reproduces bit for bit.
pub fn fr(o: [f64; 3], z: [f64; 3], x: [f64; 3]) -> Frame {
    let zv = Vec3::new(z[0], z[1], z[2]);
    let zu = UnitVec3::new_unchecked(zv);
    let o = Point3::new(o[0], o[1], o[2]);
    Frame::from_zx(o, zu, Vec3::new(x[0], x[1], x[2])).unwrap_or_else(|| Frame::from_z(o, zu))
}

fn frame(o: Point3, z: Vec3, x: Vec3) -> Frame {
    let z = z.normalize().expect("nonzero axis").get();
    fr([o.x, o.y, o.z], [z.x, z.y, z.z], [x.x, x.y, x.z])
}

fn up(o: Point3) -> Frame {
    frame(o, Vec3::Z, Vec3::X)
}

fn turned(o: Point3, a: f64) -> Frame {
    let (s, c) = dmath::sin_cos(a);
    frame(o, Vec3::Z, Vec3::new(c, s, 0.0))
}

fn ext(plane: Frame, loops: Vec<ProfileLoop>, d: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile { plane, loops },
        distance: d,
        op,
    }
}

fn block_at(o: Point3, w: f64, h: f64, d: f64, op: Combine) -> Feature {
    ext(up(o), vec![Profile::rectangle(Point2::ORIGIN, w, h, 0)], d, op)
}

fn rod_at(o: Point3, r: f64, d: f64, op: Combine) -> Feature {
    ext(up(o), vec![Profile::circle(Point2::ORIGIN, r, 0)], d, op)
}

fn ball_at(c: Point3, r: f64, op: Combine) -> Feature {
    Feature::Ball {
        frame: frame(c, Vec3::Y, Vec3::X),
        radius: r,
        op,
    }
}

fn base(w: f64, h: f64, t: f64) -> Feature {
    block_at(Point3::ORIGIN, w, h, t, Combine::New)
}

const VERTICAL: Axis2 = Axis2 {
    origin: Point2::ORIGIN,
    dir: Vec2::Y,
};

/// Sketch plane standing at (cx, cy, z0) whose y axis is the world vertical (downwards).
fn standing(cx: f64, cy: f64, z0: f64) -> Frame {
    frame(Point3::new(cx, cy, z0), Vec3::Y, Vec3::X)
}

fn op_of(r: &mut Rng) -> Combine {
    if r.bool() { Combine::Cut } else { Combine::Add }
}

pub type Family = fn(u64) -> Part;

pub fn families() -> Vec<(&'static str, Family)> {
    vec![
        ("coplanar_stack", coplanar_stack),
        ("through_vertex", through_vertex),
        ("tangent", tangent),
        ("coincident_walls", coincident_walls),
        ("coaxial", coaxial),
        ("slivers", slivers),
        ("near_tolerance", near_tolerance),
        ("axis_revolve", axis_revolve),
        ("ball_poles", ball_poles),
        ("exact_patterns", exact_patterns),
        ("tilted", tilted),
        ("repeated", repeated),
        ("remove_all", remove_all),
        ("disjoint", disjoint),
        ("enclosing", enclosing),
        ("rounded_everywhere", rounded_everywhere),
    ]
}

/// Blocks and rods standing exactly on the top face, overlapping one another;
/// pockets all to one depth.
fn coplanar_stack(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0xC0);
    let t = 10.0;
    let mut f = vec![base(80.0, 60.0, t)];
    for _ in 0..12 {
        let (x, y) = (r.range(5.0, 60.0), r.range(5.0, 45.0));
        f.push(match r.below(4) {
            0 => block_at(
                Point3::new(x, y, t),
                r.range(5.0, 20.0),
                r.range(5.0, 15.0),
                r.range(2.0, 8.0),
                Combine::Add,
            ),
            1 => rod_at(
                Point3::new(x, y, t),
                r.range(2.0, 8.0),
                r.range(2.0, 8.0),
                Combine::Add,
            ),
            2 => block_at(
                Point3::new(x, y, t - 4.0),
                r.range(5.0, 20.0),
                r.range(5.0, 15.0),
                5.0,
                Combine::Cut,
            ),
            _ => rod_at(Point3::new(x, y, t - 4.0), r.range(2.0, 6.0), 5.0, Combine::Cut),
        });
    }
    Part { features: f }
}

/// Cuts whose corners and centres sit exactly on existing vertices and edges.
fn through_vertex(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0x7E);
    let (w, h, t) = (60.0, 40.0, 10.0);
    let mut f = vec![base(w, h, t)];
    let corners = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)];
    for _ in 0..8 {
        let (cx, cy) = corners[r.below(4)];
        f.push(match r.below(4) {
            0 => rod_at(
                Point3::new(cx, cy, -1.0),
                r.range(2.0, 8.0),
                t + 2.0,
                Combine::Cut,
            ),
            1 => block_at(
                Point3::new(cx - 5.0, cy - 5.0, -1.0),
                10.0,
                10.0,
                t + 2.0,
                Combine::Cut,
            ),
            // a hole centred on a top edge
            2 => rod_at(
                Point3::new(r.range(5.0, w - 5.0), 0.0, -1.0),
                r.range(2.0, 5.0),
                t + 2.0,
                Combine::Cut,
            ),
            // a slot ending exactly at the far face
            _ => block_at(
                Point3::new(r.range(5.0, w - 15.0), r.range(5.0, h - 10.0), t - 3.0),
                w,
                4.0,
                3.0,
                Combine::Cut,
            ),
        });
    }
    Part { features: f }
}

/// Cylinders tangent to faces, balls tangent to the top, tangent holes.
fn tangent(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0x7A);
    let (w, h, t) = (60.0, 40.0, 10.0);
    let mut f = vec![base(w, h, t)];
    for _ in 0..8 {
        let rad = r.range(2.0, 6.0);
        let k = r.below(5);
        if k == 4 {
            // two rods tangent to each other
            let (x, y) = (r.range(10.0, w - 20.0), r.range(10.0, h - 10.0));
            f.push(rod_at(Point3::new(x, y, -1.0), rad, t + 2.0, Combine::Cut));
            f.push(rod_at(
                Point3::new(x + 2.0 * rad, y, -1.0),
                rad,
                t + 2.0,
                Combine::Cut,
            ));
            continue;
        }
        f.push(match k {
            // rod tangent to the side face x = w, from outside
            0 => rod_at(
                Point3::new(w + rad, r.range(8.0, h - 8.0), -1.0),
                rad,
                t + 2.0,
                op_of(&mut r),
            ),
            // rod tangent to the side face from inside
            1 => rod_at(
                Point3::new(w - rad, r.range(8.0, h - 8.0), -1.0),
                rad,
                t + 2.0,
                Combine::Cut,
            ),
            // ball resting on the top face
            2 => ball_at(
                Point3::new(r.range(8.0, w - 8.0), r.range(8.0, h - 8.0), t + rad),
                rad,
                Combine::Add,
            ),
            // ball tangent to the top from inside
            3 => ball_at(
                Point3::new(r.range(8.0, w - 8.0), r.range(8.0, h - 8.0), t - rad),
                rad,
                Combine::Cut,
            ),
            _ => ball_at(
                Point3::new(r.range(8.0, w - 8.0), r.range(8.0, h - 8.0), t - rad),
                rad,
                Combine::Cut,
            ),
        });
    }
    Part { features: f }
}

/// Second features exactly as wide as the first: shared walls.
fn coincident_walls(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0xC1);
    let (w, h, t) = (60.0, 40.0, 10.0);
    let mut f = vec![base(w, h, t)];
    for _ in 0..5 {
        let (x, y) = (r.range(5.0, w - 25.0), r.range(5.0, h - 15.0));
        let (sw, sh) = (r.range(5.0, 15.0), r.range(3.0, 8.0));
        f.push(block_at(Point3::new(x, y, t - 3.0), sw, sh, 4.0, Combine::Cut));
        // same width, shifted along: walls coincide
        f.push(block_at(
            Point3::new(x + sw * 0.5, y, t - 5.0),
            sw,
            sh,
            6.0,
            Combine::Cut,
        ));
        if r.bool() {
            // refill exactly
            f.push(block_at(Point3::new(x, y, t - 3.0), sw, sh, 3.0, Combine::Add));
        }
    }
    let (x, y) = (r.range(10.0, w - 10.0), r.range(10.0, h - 10.0));
    let rad = r.range(2.0, 5.0);
    f.push(rod_at(Point3::new(x, y, -1.0), rad, t + 2.0, Combine::Cut));
    f.push(rod_at(Point3::new(x, y, 0.0), rad, t, Combine::Add));
    Part { features: f }
}

/// Holes, bosses and counterbores sharing axes.
fn coaxial(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0xCA);
    let (w, h, t) = (60.0, 40.0, 10.0);
    let mut f = vec![base(w, h, t)];
    for _ in 0..4 {
        let (x, y) = (r.range(10.0, w - 10.0), r.range(10.0, h - 10.0));
        let rad = r.range(2.0, 4.0);
        f.push(rod_at(
            Point3::new(x, y, t - 1.0),
            rad * 2.0,
            r.range(3.0, 8.0),
            Combine::Add,
        ));
        f.push(rod_at(Point3::new(x, y, -1.0), rad, t + 12.0, Combine::Cut));
        f.push(rod_at(Point3::new(x, y, t + 2.0), rad * 1.5, 20.0, Combine::Cut));
        if r.bool() {
            f.push(rod_at(Point3::new(x, y, -2.0), rad * 0.5, t, Combine::Add));
        }
    }
    Part { features: f }
}

/// Features offset from existing faces by 0.01 to 0.1 mm.
fn slivers(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0x51);
    let (w, h, t) = (60.0, 40.0, 10.0);
    let mut f = vec![base(w, h, t)];
    for _ in 0..8 {
        let e = r.range(0.01, 0.1) * if r.bool() { 1.0 } else { -1.0 };
        f.push(match r.below(4) {
            0 => block_at(
                Point3::new(w - 10.0 + e, r.range(5.0, h - 15.0), -1.0),
                10.0,
                8.0,
                t + 2.0,
                op_of(&mut r),
            ),
            1 => block_at(
                Point3::new(r.range(5.0, w - 15.0), r.range(5.0, h - 15.0), t + e),
                10.0,
                8.0,
                3.0,
                op_of(&mut r),
            ),
            2 => rod_at(
                Point3::new(r.range(5.0, w - 5.0), h + e - 3.0, -1.0),
                3.0,
                t + 2.0,
                op_of(&mut r),
            ),
            _ => block_at(
                Point3::new(r.range(5.0, w - 15.0), r.range(5.0, h - 15.0), t - 2.0 + e),
                10.0,
                8.0,
                2.0 - e * 0.5,
                Combine::Cut,
            ),
        });
    }
    Part { features: f }
}

/// Offsets near the 1e-5 mm tolerance.
fn near_tolerance(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0x1E);
    let (w, h, t) = (60.0, 40.0, 10.0);
    let mut f = vec![base(w, h, t)];
    for _ in 0..8 {
        let mag = [1e-6, 5e-6, 1e-5, 2e-5, 1e-4][r.below(5)];
        let e = mag * if r.bool() { 1.0 } else { -1.0 };
        f.push(match r.below(3) {
            0 => block_at(
                Point3::new(r.range(5.0, w - 15.0), r.range(5.0, h - 15.0), t + e),
                10.0,
                8.0,
                3.0,
                Combine::Add,
            ),
            1 => block_at(
                Point3::new(w - 10.0 + e, r.range(5.0, h - 15.0), -1.0),
                10.0 - e,
                8.0,
                t + 2.0,
                Combine::Cut,
            ),
            _ => rod_at(
                Point3::new(r.range(8.0, w - 8.0), r.range(8.0, h - 8.0), t - 3.0 + e),
                3.0,
                3.0 + e,
                Combine::Cut,
            ),
        });
    }
    Part { features: f }
}

/// Revolves whose profiles touch the axis (cones, domes, solid spindles).
fn axis_revolve(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0xA1);
    let (w, h, t) = (60.0, 40.0, 10.0);
    let mut f = vec![base(w, h, t)];
    for _ in 0..5 {
        let (x, y) = (r.range(10.0, w - 10.0), r.range(10.0, h - 10.0));
        let rad = r.range(2.0, 6.0);
        let height = r.range(3.0, 12.0);
        let z0 = r.range(-2.0, t + 2.0);
        let loops = match r.below(3) {
            // cone: triangle with one side on the axis
            0 => vec![ProfileLoop::new(vec![
                (
                    Segment::Line {
                        a: Point2::ORIGIN,
                        b: Point2::new(rad, 0.0),
                    },
                    0,
                ),
                (
                    Segment::Line {
                        a: Point2::new(rad, 0.0),
                        b: Point2::new(0.0, -height),
                    },
                    1,
                ),
                (
                    Segment::Line {
                        a: Point2::new(0.0, -height),
                        b: Point2::ORIGIN,
                    },
                    2,
                ),
            ])],
            // rectangle from the axis: a cylinder with discs
            1 => vec![Profile::rectangle(Point2::new(0.0, -height), rad, height, 0)],
            // quarter disc: a dome
            _ => vec![ProfileLoop::new(vec![
                (
                    Segment::Line {
                        a: Point2::ORIGIN,
                        b: Point2::new(rad, 0.0),
                    },
                    0,
                ),
                (
                    Segment::Arc {
                        center: Point2::ORIGIN,
                        radius: rad,
                        start: 0.0,
                        sweep: -dmath::FRAC_PI_2,
                    },
                    1,
                ),
                (
                    Segment::Line {
                        a: Point2::new(0.0, -rad),
                        b: Point2::ORIGIN,
                    },
                    2,
                ),
            ])],
        };
        f.push(Feature::Revolve {
            profile: Profile {
                plane: standing(x, y, z0),
                loops,
            },
            axis: VERTICAL,
            angle: dmath::TAU,
            op: op_of(&mut r),
        });
    }
    Part { features: f }
}

/// Balls whose poles touch other balls or faces.
fn ball_poles(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0xB0);
    let (w, h, t) = (60.0, 40.0, 10.0);
    let mut f = vec![base(w, h, t)];
    for _ in 0..4 {
        let (x, y) = (r.range(12.0, w - 12.0), r.range(12.0, h - 12.0));
        let rad = r.range(3.0, 6.0);
        // ball frames have their axis along world y: poles at y +- r
        f.push(ball_at(Point3::new(x, y, t), rad, op_of(&mut r)));
        f.push(ball_at(Point3::new(x, y + 2.0 * rad, t), rad, op_of(&mut r)));
        f.push(ball_at(Point3::new(x, y + rad, t), rad * 0.5, op_of(&mut r)));
        f.push(rod_at(Point3::new(x, y + rad, -1.0), 1.0, t + 10.0, Combine::Cut));
    }
    Part { features: f }
}

/// Grids whose pitch equals the feature size: neighbours share walls exactly.
fn exact_patterns(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0xE9);
    let (w, h, t) = (80.0, 60.0, 10.0);
    let mut f = vec![base(w, h, t)];
    let s = r.range(4.0, 8.0);
    let (x0, y0) = (r.range(2.0, 10.0), r.range(2.0, 10.0));
    for i in 0..4 {
        for j in 0..3 {
            let o = Point3::new(x0 + i as f64 * s, y0 + j as f64 * s, t - 3.0);
            f.push(if (i + j) % 2 == 0 {
                block_at(o, s, s, 4.0, Combine::Cut)
            } else {
                rod_at(
                    Point3::new(o.x + s / 2.0, o.y + s / 2.0, o.z),
                    s / 2.0,
                    4.0,
                    Combine::Cut,
                )
            });
        }
    }
    Part { features: f }
}

/// Tilted cuts and adds.
fn tilted(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0x71);
    let (w, h, t) = (60.0, 40.0, 10.0);
    let mut f = vec![base(w, h, t)];
    for _ in 0..8 {
        let o = Point3::new(
            r.range(5.0, w - 5.0),
            r.range(5.0, h - 5.0),
            r.range(-2.0, t + 2.0),
        );
        let z = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(0.2, 1.0));
        let plane = frame(o, z, Vec3::new(r.range(-1.0, 1.0), 1.0, 0.0));
        f.push(if r.bool() {
            ext(
                plane,
                vec![Profile::circle(Point2::ORIGIN, r.range(1.0, 5.0), 0)],
                r.range(2.0, 15.0),
                op_of(&mut r),
            )
        } else {
            ext(
                plane,
                vec![Profile::rectangle(Point2::new(-3.0, -2.0), 6.0, 4.0, 0)],
                r.range(2.0, 15.0),
                op_of(&mut r),
            )
        });
    }
    Part { features: f }
}

/// The same feature twice.
fn repeated(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0x2E);
    let (w, h, t) = (60.0, 40.0, 10.0);
    let mut f = vec![base(w, h, t)];
    for _ in 0..4 {
        let x = r.range(8.0, w - 8.0);
        let y = r.range(8.0, h - 8.0);
        let feat = match r.below(3) {
            0 => rod_at(Point3::new(x, y, -1.0), 3.0, t + 2.0, Combine::Cut),
            1 => rod_at(Point3::new(x, y, t - 1.0), 3.0, 5.0, Combine::Add),
            _ => ball_at(Point3::new(x, y, t), 3.0, op_of(&mut r)),
        };
        f.push(feat.clone());
        f.push(feat);
    }
    Part { features: f }
}

/// A cut that removes everything, then adds after it.
fn remove_all(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0x8A);
    let (w, h, t) = (60.0, 40.0, 10.0);
    let mut f = vec![base(w, h, t)];
    f.push(rod_at(Point3::new(10.0, 10.0, -1.0), 3.0, t + 2.0, Combine::Cut));
    let m = r.range(0.0, 5.0);
    f.push(block_at(
        Point3::new(-m, -m, -m),
        w + 2.0 * m,
        h + 2.0 * m,
        t + 2.0 * m,
        Combine::Cut,
    ));
    f.push(block_at(
        Point3::new(5.0, 5.0, 0.0),
        10.0,
        10.0,
        5.0,
        Combine::Add,
    ));
    Part { features: f }
}

/// Adds that touch nothing, then cuts spanning several pieces.
fn disjoint(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0xD1);
    let (w, h, t) = (40.0, 30.0, 10.0);
    let mut f = vec![base(w, h, t)];
    for k in 0..3 {
        let x = w + 10.0 + k as f64 * 20.0;
        f.push(block_at(
            Point3::new(x, r.range(0.0, 10.0), 0.0),
            10.0,
            10.0,
            t,
            Combine::Add,
        ));
    }
    f.push(block_at(
        Point3::new(-5.0, r.range(2.0, 8.0), t - 3.0),
        200.0,
        4.0,
        5.0,
        Combine::Cut,
    ));
    f.push(rod_at(
        Point3::new(w + 15.0, 5.0, -1.0),
        2.0,
        t + 2.0,
        Combine::Cut,
    ));
    Part { features: f }
}

/// Tools wholly inside the part (voids) or wholly enclosing it.
fn enclosing(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0xE1);
    let (w, h, t) = (60.0, 40.0, 20.0);
    let mut f = vec![base(w, h, t)];
    f.push(block_at(
        Point3::new(10.0, 10.0, 5.0),
        r.range(5.0, 20.0),
        r.range(5.0, 15.0),
        5.0,
        Combine::Cut,
    ));
    f.push(ball_at(
        Point3::new(40.0, 20.0, 10.0),
        r.range(2.0, 5.0),
        Combine::Cut,
    ));
    f.push(block_at(
        Point3::new(-5.0, -5.0, -5.0),
        w + 10.0,
        h + 10.0,
        t + 10.0,
        Combine::Keep,
    ));
    f.push(rod_at(Point3::new(30.0, 20.0, -1.0), 2.0, t + 2.0, Combine::Cut));
    Part { features: f }
}

/// Rounded profiles everywhere: every wall junction is a smooth edge.
fn rounded_everywhere(seed: u64) -> Part {
    let mut r = Rng::new(seed ^ 0x2D);
    let (w, h, t) = (80.0, 60.0, 12.0);
    let mut f = vec![ext(
        up(Point3::ORIGIN),
        vec![Profile::rounded_rectangle(Point2::ORIGIN, w, h, 6.0, 0)],
        t,
        Combine::New,
    )];
    for _ in 0..10 {
        let o = Point3::new(
            r.range(5.0, w - 20.0),
            r.range(5.0, h - 15.0),
            r.range(-2.0, t + 1.0),
        );
        let (a, b) = (r.range(6.0, 20.0), r.range(4.0, 12.0));
        // a corner radius over half the shorter side turns the outline inside out
        let rad = a.min(b) * r.range(0.1, 0.49);
        f.push(ext(
            turned(o, r.range(0.0, dmath::TAU)),
            vec![Profile::rounded_rectangle(
                Point2::new(-a / 2.0, -b / 2.0),
                a,
                b,
                rad,
                0,
            )],
            r.range(2.0, 15.0),
            op_of(&mut r),
        ));
    }
    Part { features: f }
}

/// What happened when a part was regenerated.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Ok,
    /// Regeneration returned an error: kind (without the feature index) and feature index.
    Error {
        kind: String,
        feature: usize,
    },
    /// Regenerated, but the solid fails the checker.
    Invalid {
        kind: String,
    },
    Panic {
        message: String,
    },
    /// Refused with a clear error: the solids touch exactly, leaving zero-thickness material.
    Rejected {
        feature: usize,
    },
}

impl Outcome {
    pub fn kind(&self) -> Option<String> {
        match self {
            Outcome::Ok => None,
            Outcome::Error { kind, .. } => Some(format!("error: {kind}")),
            Outcome::Invalid { kind } => Some(format!("invalid: {kind}")),
            Outcome::Panic { message } => Some(format!("panic: {message}")),
            Outcome::Rejected { .. } => Some("rejected: zero thickness".to_string()),
        }
    }
}

fn error_kind(e: &crate::RegenError) -> (String, usize) {
    use crate::RegenError::*;
    match e {
        Extrude { feature, error } => (format!("Extrude({error:?})"), *feature),
        Revolve { feature, error } => (format!("Revolve({error:?})"), *feature),
        Boolean { feature, error } => (format!("Boolean({error:?})"), *feature),
        NothingToCombine { feature } => ("NothingToCombine".to_string(), *feature),
    }
}

/// Families whose solids touch exactly by design (tangent rods and balls, grids sharing walls
/// and corners): refusing them with a zero-thickness error is correct handling.
pub fn touches_by_design(family: &str) -> bool {
    matches!(family, "tangent" | "ball_poles" | "exact_patterns")
}

/// Whether a family's part was handled correctly: a valid solid, or a zero-thickness refusal
/// where the family touches by design.
pub fn handled(family: &str, o: &Outcome) -> bool {
    match o {
        Outcome::Ok => true,
        Outcome::Rejected { .. } => touches_by_design(family),
        _ => false,
    }
}

pub fn outcome(part: &Part) -> Outcome {
    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match part.regenerate() {
        Ok(b) => {
            let p = forge_topo::check(&b);
            match p.first() {
                None => Outcome::Ok,
                Some(first) => {
                    let s = format!("{first:?}");
                    Outcome::Invalid {
                        kind: s.split([' ', '{', '(']).next().unwrap_or("").to_string(),
                    }
                }
            }
        }
        Err(crate::RegenError::Boolean {
            feature,
            error: forge_bool::BoolError::ZeroThickness { .. },
        }) => Outcome::Rejected { feature },
        Err(e) => {
            let (kind, feature) = error_kind(&e);
            Outcome::Error { kind, feature }
        }
    }));
    run.unwrap_or_else(|p| {
        let message = p
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| p.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "?".to_string());
        // keep the location-free part so equal panics dedup
        Outcome::Panic {
            message: message.chars().take(80).collect(),
        }
    })
}

/// Rounds origins, distances and radii to `digits` decimals (frames are
/// rebuilt so they stay orthonormal).
fn rounded_feature(f: &Feature, digits: i32) -> Feature {
    let k = [1.0, 10.0, 100.0, 1000.0][digits as usize];
    let rd = |v: f64| (v * k).round() / k;
    let rf = |fr0: &Frame| {
        let o = fr0.origin;
        let z = fr0.z().get();
        let x = fr0.x().get();
        fr([rd(o.x), rd(o.y), rd(o.z)], [z.x, z.y, z.z], [x.x, x.y, x.z])
    };
    match f {
        Feature::Extrude {
            profile,
            distance,
            op,
        } => Feature::Extrude {
            profile: Profile {
                plane: rf(&profile.plane),
                loops: profile.loops.clone(),
            },
            distance: rd(*distance),
            op: *op,
        },
        Feature::Revolve {
            profile,
            axis,
            angle,
            op,
        } => Feature::Revolve {
            profile: Profile {
                plane: rf(&profile.plane),
                loops: profile.loops.clone(),
            },
            axis: *axis,
            angle: *angle,
            op: *op,
        },
        Feature::Ball { frame, radius, op } => Feature::Ball {
            frame: rf(frame),
            radius: rd(*radius),
            op: *op,
        },
    }
}

fn reframe(f: &Frame) -> Frame {
    let (o, z, x) = (f.origin, f.z().get(), f.x().get());
    fr([o.x, o.y, o.z], [z.x, z.y, z.z], [x.x, x.y, x.z])
}

/// The part with every frame rebuilt through `fr` until it no longer
/// changes, so `literal` reproduces it bit for bit.
pub fn canonical(part: &Part) -> Part {
    let mut cur = part.clone();
    for _ in 0..8 {
        let next = Part {
            features: cur
                .features
                .iter()
                .map(|f| match f {
                    Feature::Extrude {
                        profile,
                        distance,
                        op,
                    } => Feature::Extrude {
                        profile: Profile {
                            plane: reframe(&profile.plane),
                            loops: profile.loops.clone(),
                        },
                        distance: *distance,
                        op: *op,
                    },
                    Feature::Revolve {
                        profile,
                        axis,
                        angle,
                        op,
                    } => Feature::Revolve {
                        profile: Profile {
                            plane: reframe(&profile.plane),
                            loops: profile.loops.clone(),
                        },
                        axis: *axis,
                        angle: *angle,
                        op: *op,
                    },
                    Feature::Ball { frame, radius, op } => Feature::Ball {
                        frame: reframe(frame),
                        radius: *radius,
                        op: *op,
                    },
                })
                .collect(),
        };
        if next == cur {
            break;
        }
        cur = next;
    }
    cur
}

/// Smallest history found that still fails with the same kind of failure.
pub fn shrink(part: &Part) -> Part {
    let Some(kind) = outcome(part).kind() else {
        return part.clone();
    };
    let same = |p: &Part| outcome(p).kind().as_deref() == Some(kind.as_str());
    let mut cur = part.clone();
    // a regeneration error at feature k: everything after k is irrelevant
    if let Outcome::Error { feature, .. } = outcome(&cur) {
        cur.features.truncate(feature + 1);
    }
    loop {
        let mut changed = false;
        let mut i = 1;
        while i < cur.features.len() {
            let mut t = cur.clone();
            t.features.remove(i);
            if same(&t) {
                cur = t;
                changed = true;
            } else {
                i += 1;
            }
        }
        if !changed {
            break;
        }
    }
    for digits in [0, 1, 2, 3] {
        let t = canonical(&Part {
            features: cur.features.iter().map(|f| rounded_feature(f, digits)).collect(),
        });
        if same(&t) {
            cur = t;
            break;
        }
    }
    cur
}

fn lit_f(v: f64) -> String {
    let s = format!("{v:?}");
    if s.contains('.') || s.contains('e') || s.contains("inf") || s.contains("NaN") {
        s
    } else {
        format!("{s}.0")
    }
}

fn lit_frame(f: &Frame) -> String {
    let (o, z, x) = (f.origin, f.z().get(), f.x().get());
    format!(
        "fr([{}, {}, {}], [{}, {}, {}], [{}, {}, {}])",
        lit_f(o.x),
        lit_f(o.y),
        lit_f(o.z),
        lit_f(z.x),
        lit_f(z.y),
        lit_f(z.z),
        lit_f(x.x),
        lit_f(x.y),
        lit_f(x.z)
    )
}

fn lit_p2(p: Point2) -> String {
    format!("Point2::new({}, {})", lit_f(p.x), lit_f(p.y))
}

fn lit_loops(loops: &[ProfileLoop]) -> String {
    let ls: Vec<String> = loops
        .iter()
        .map(|l| {
            let segs: Vec<String> = l
                .segments
                .iter()
                .map(|(s, id)| {
                    let s = match *s {
                        Segment::Line { a, b } => {
                            format!("Segment::Line {{ a: {}, b: {} }}", lit_p2(a), lit_p2(b))
                        }
                        Segment::Arc {
                            center,
                            radius,
                            start,
                            sweep,
                        } => format!(
                            "Segment::Arc {{ center: {}, radius: {}, start: {}, sweep: {} }}",
                            lit_p2(center),
                            lit_f(radius),
                            lit_f(start),
                            lit_f(sweep)
                        ),
                        Segment::Circle { center, radius, ccw } => {
                            format!(
                                "Segment::Circle {{ center: {}, radius: {}, ccw: {ccw} }}",
                                lit_p2(center),
                                lit_f(radius)
                            )
                        }
                    };
                    format!("({s}, {id})")
                })
                .collect();
            format!("ProfileLoop::new(vec![{}])", segs.join(", "))
        })
        .collect();
    format!("vec![{}]", ls.join(", "))
}

/// The part as Rust source building it (uses `fr` from this module).
pub fn literal(part: &Part) -> String {
    let fs: Vec<String> = part
        .features
        .iter()
        .map(|f| match f {
            Feature::Extrude { profile, distance, op } => format!(
                "        Feature::Extrude {{ profile: Profile {{ plane: {}, loops: {} }}, distance: {}, op: Combine::{op:?} }},",
                lit_frame(&profile.plane),
                lit_loops(&profile.loops),
                lit_f(*distance)
            ),
            Feature::Revolve { profile, axis, angle, op } => format!(
                "        Feature::Revolve {{ profile: Profile {{ plane: {}, loops: {} }}, axis: Axis2 {{ origin: {}, dir: Vec2::new({}, {}) }}, angle: {}, op: Combine::{op:?} }},",
                lit_frame(&profile.plane),
                lit_loops(&profile.loops),
                lit_p2(axis.origin),
                lit_f(axis.dir.x),
                lit_f(axis.dir.y),
                lit_f(*angle)
            ),
            Feature::Ball { frame, radius, op } => format!(
                "        Feature::Ball {{ frame: {}, radius: {}, op: Combine::{op:?} }},",
                lit_frame(frame),
                lit_f(*radius)
            ),
        })
        .collect();
    format!("Part {{\n    features: vec![\n{}\n    ],\n}}", fs.join("\n"))
}
