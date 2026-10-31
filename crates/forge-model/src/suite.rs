//! A set of benchmark parts unlike one another, so no single shape decides
//! what "fast" and "robust" mean, plus a random history generator.

use crate::{
    Coil, Combine, End, Extent, Feature, Head, Hole, Layout, Loft, Part, Row, Source, Sweep, Target, Thread, Turn,
    torture,
};
use forge_geom::Circle;
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, Point3, Vec2, Vec3, dmath};
use forge_ops::{Axis2, PathSeg, Profile, ProfileLoop, Segment};
use forge_topo::Name;

fn frame(o: Point3, z: Vec3, x: Vec3) -> Frame {
    let z = z.normalize().expect("nonzero axis");
    Frame::from_zx(o, z, x).unwrap_or_else(|| Frame::from_z(o, z))
}

fn up(o: Point3) -> Frame {
    Frame::WORLD.with_origin(o)
}

/// Rotated about world z by `a`.
fn turned(o: Point3, a: f64) -> Frame {
    let (s, c) = dmath::sin_cos(a);
    frame(o, Vec3::Z, Vec3::new(c, s, 0.0))
}

fn block(plane: Frame, corner: Point2, w: f64, h: f64, d: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane,
            loops: vec![Profile::rectangle(corner, w, h, 0)],
        },
        extent: Extent::Distance(d),
        op,
    }
}

fn rounded(plane: Frame, corner: Point2, w: f64, h: f64, r: f64, d: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane,
            loops: vec![Profile::rounded_rectangle(corner, w, h, r, 0)],
        },
        extent: Extent::Distance(d),
        op,
    }
}

fn rod(plane: Frame, r: f64, len: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane,
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        },
        extent: Extent::Distance(len),
        op,
    }
}

fn ball(c: Point3, r: f64, op: Combine) -> Feature {
    Feature::Ball {
        frame: frame(c, Vec3::Y, Vec3::X),
        radius: r,
        op,
    }
}

/// A closed polygon profile from points (segment ids are positions).
fn polygon(plane: Frame, pts: &[Point2]) -> Profile {
    let n = pts.len();
    let segs = (0..n)
        .map(|i| {
            (
                Segment::Line {
                    a: pts[i],
                    b: pts[(i + 1) % n],
                },
                i as u64,
            )
        })
        .collect();
    Profile {
        plane,
        loops: vec![ProfileLoop::new(segs)],
    }
}

fn revolve(profile: Profile, axis: Axis2, op: Combine) -> Feature {
    Feature::Revolve {
        profile,
        axis,
        extent: Turn::Angle(dmath::TAU),
        op,
    }
}

/// Sketch plane standing on (cx, cy, z0): sketch x is world x, sketch y is world -z,
/// so the sketch's y axis through (0, 0) is the vertical line through (cx, cy).
fn standing(cx: f64, cy: f64, z0: f64) -> Frame {
    frame(Point3::new(cx, cy, z0), Vec3::Y, Vec3::X)
}

const VERTICAL: Axis2 = Axis2 {
    origin: Point2::ORIGIN,
    dir: Vec2::Y,
};

/// Every named benchmark part.
pub fn parts() -> Vec<(&'static str, Part)> {
    NAMES
        .iter()
        .map(|&n| (n, part(n).expect("a named part")))
        .collect()
}

/// The names of the benchmark parts.
pub const NAMES: [&str; 13] = [
    "plate",
    "shaft",
    "housing",
    "coplanar",
    "patterns",
    "mixed",
    "tour",
    "hole-grid",
    "upto",
    "upto-flat",
    "sweep",
    "coil",
    "loft",
];

/// The benchmark part of that name, built alone.
pub fn part(name: &str) -> Option<Part> {
    Some(match name {
        "plate" => torture::part(usize::MAX),
        "shaft" => shaft(),
        "housing" => housing(),
        "coplanar" => coplanar(),
        "patterns" => patterns(),
        "mixed" => mixed(),
        "tour" => tour(),
        "hole-grid" => hole_grid(),
        "upto" => upto(false),
        "upto-flat" => upto(true),
        "sweep" => sweep(),
        "coil" => coil(),
        "loft" => loft(),
        _ => return None,
    })
}

/// A stepped shaft along world x: grooves, cross holes, flats, keyways and
/// rings of radial pin holes.
pub fn shaft() -> Part {
    let mut r = Rng::new(0x5_4af7);
    let steps = 10;
    let len = 30.0;
    let radii: Vec<f64> = (0..steps).map(|_| r.range(12.0, 25.0)).collect();
    let mut pts = vec![Point2::ORIGIN];
    for (i, &rad) in radii.iter().enumerate() {
        pts.push(Point2::new(i as f64 * len, rad));
        pts.push(Point2::new((i + 1) as f64 * len, rad));
    }
    pts.push(Point2::new(steps as f64 * len, 0.0));
    let axis = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::X,
    };
    let mut f = vec![revolve(polygon(Frame::WORLD, &pts), axis, Combine::New)];
    let total = steps as f64 * len;
    let radius_at = |x: f64| radii[((x / len) as usize).min(steps - 1)];
    for _ in 0..25 {
        let x = r.range(3.0, total - 6.0);
        let (w, d) = (r.range(1.0, 3.0), r.range(0.8, 3.0));
        let rad = radius_at(x).min(radius_at(x + w));
        let groove = [
            Point2::new(x, rad - d),
            Point2::new(x + w, rad - d),
            Point2::new(x + w, rad + 3.0),
            Point2::new(x, rad + 3.0),
        ];
        f.push(revolve(polygon(Frame::WORLD, &groove), axis, Combine::Cut));
    }
    for _ in 0..40 {
        let x = r.range(5.0, total - 5.0);
        let a = r.range(0.0, dmath::PI);
        let (s, c) = dmath::sin_cos(a);
        let dir = Vec3::new(0.0, c, s);
        f.push(rod(
            frame(Point3::new(x, 0.0, 0.0) - dir * 40.0, dir, Vec3::X),
            r.range(1.0, 4.0),
            80.0,
            Combine::Cut,
        ));
    }
    for _ in 0..20 {
        let x = r.range(0.0, total - 20.0);
        let depth = radius_at(x) - r.range(1.0, 4.0);
        let a = r.range(0.0, dmath::TAU);
        let (s, c) = dmath::sin_cos(a);
        let n = Vec3::new(0.0, c, s);
        let plane = frame(Point3::new(x, 0.0, 0.0) + n * depth, n, Vec3::X);
        f.push(block(
            plane,
            Point2::new(0.0, -30.0),
            r.range(8.0, 20.0),
            60.0,
            30.0,
            Combine::Cut,
        ));
    }
    for _ in 0..25 {
        let x = r.range(2.0, total - 25.0);
        let a = r.range(0.0, dmath::TAU);
        let (s, c) = dmath::sin_cos(a);
        let n = Vec3::new(0.0, c, s);
        let d = r.range(1.5, 4.0);
        let plane = frame(Point3::new(x, 0.0, 0.0) + n * (radius_at(x) - d), n, Vec3::X);
        let (kl, kw) = (r.range(8.0, 20.0), r.range(3.0, 6.0));
        f.push(rounded(
            plane,
            Point2::new(0.0, -kw / 2.0),
            kl,
            kw,
            kw * 0.49,
            d + 6.0,
            Combine::Cut,
        ));
    }
    for station in 0..8 {
        let x = 15.0 + station as f64 * 36.0 + r.range(-3.0, 3.0);
        let rad = radius_at(x);
        let phase = r.range(0.0, 1.0);
        for k in 0..8 {
            let a = phase + k as f64 * dmath::TAU / 8.0;
            let (s, c) = dmath::sin_cos(a);
            let n = Vec3::new(0.0, c, s);
            f.push(rod(
                frame(Point3::new(x, 0.0, 0.0) + n * (rad + 2.0), -n, Vec3::X),
                r.range(0.8, 1.8),
                r.range(4.0, 9.0),
                Combine::Cut,
            ));
        }
    }
    Part { features: f }
}

/// A hollow open box with bosses on floor and walls, screw holes in them,
/// ribs, and windows through the walls.
pub fn housing() -> Part {
    let mut r = Rng::new(0x40_051);
    let (w, h, d, t) = (200.0, 150.0, 80.0, 4.0);
    let mut f = vec![
        block(Frame::WORLD, Point2::ORIGIN, w, h, d, Combine::New),
        block(
            up(Point3::new(0.0, 0.0, t)),
            Point2::new(t, t),
            w - 2.0 * t,
            h - 2.0 * t,
            d,
            Combine::Cut,
        ),
    ];
    for _ in 0..40 {
        let (x, y) = (r.range(15.0, w - 15.0), r.range(15.0, h - 15.0));
        let (br, bh) = (r.range(3.0, 7.0), r.range(8.0, 30.0));
        f.push(rod(up(Point3::new(x, y, t - 1.0)), br, bh + 1.0, Combine::Add));
        f.push(rod(
            up(Point3::new(x, y, t + bh * 0.4)),
            br * 0.45,
            bh,
            Combine::Cut,
        ));
    }
    for _ in 0..30 {
        let (y, z) = (r.range(12.0, h - 12.0), r.range(12.0, d - 12.0));
        let (br, bl) = (r.range(3.0, 6.0), r.range(8.0, 20.0));
        f.push(rod(
            frame(Point3::new(t - 1.0, y, z), Vec3::X, Vec3::Y),
            br,
            bl + 1.0,
            Combine::Add,
        ));
        f.push(rod(
            frame(Point3::new(t + 2.0, y, z), Vec3::X, Vec3::Y),
            br * 0.45,
            bl,
            Combine::Cut,
        ));
    }
    for _ in 0..30 {
        let (x, y) = (r.range(20.0, w - 20.0), r.range(10.0, h - 10.0));
        let rh = r.range(5.0, 15.0);
        f.push(block(
            up(Point3::new(0.0, y - 0.9, t - 0.5)),
            Point2::new(t - 0.5, 0.0),
            x - t,
            1.8,
            rh,
            Combine::Add,
        ));
    }
    for _ in 0..15 {
        let (x, z) = (r.range(15.0, w - 40.0), r.range(12.0, d - 30.0));
        let wall = frame(Point3::new(x, -1.0, z), Vec3::Y, Vec3::X);
        f.push(block(
            wall,
            Point2::ORIGIN,
            r.range(8.0, 25.0),
            r.range(5.0, 15.0),
            t + 2.0,
            Combine::Cut,
        ));
    }
    Part { features: f }
}

/// Features that share planes on purpose: blocks stacked on the top face,
/// slots and pockets of equal depth, bosses standing exactly on the top face,
/// and a regular grid of equal holes.
pub fn coplanar() -> Part {
    let mut r = Rng::new(0xc0_91a);
    let (w, h, t) = (300.0, 200.0, 20.0);
    let mut f = vec![block(Frame::WORLD, Point2::ORIGIN, w, h, t, Combine::New)];
    for _ in 0..30 {
        let (x, y) = (r.range(5.0, w - 45.0), r.range(5.0, h - 45.0));
        f.push(block(
            up(Point3::new(x, y, t)),
            Point2::ORIGIN,
            r.range(10.0, 40.0),
            r.range(10.0, 40.0),
            10.0,
            Combine::Add,
        ));
    }
    for _ in 0..40 {
        let (x, y) = (r.range(15.0, w - 15.0), r.range(15.0, h - 15.0));
        let (sw, sh) = (r.range(10.0, 30.0), r.range(3.0, 7.0));
        f.push(block(
            turned(Point3::new(x, y, 12.0), r.range(0.0, 3.1)),
            Point2::new(-sw / 2.0, -sh / 2.0),
            sw,
            sh,
            20.0,
            Combine::Cut,
        ));
    }
    for _ in 0..40 {
        let (x, y) = (r.range(10.0, w - 10.0), r.range(10.0, h - 10.0));
        f.push(rod(
            up(Point3::new(x, y, t)),
            r.range(2.0, 6.0),
            r.range(4.0, 15.0),
            Combine::Add,
        ));
    }
    for _ in 0..40 {
        let (x, y) = (r.range(10.0, w - 10.0), r.range(10.0, h - 10.0));
        f.push(rod(
            up(Point3::new(x, y, t - 5.0)),
            r.range(2.0, 6.0),
            20.0,
            Combine::Cut,
        ));
    }
    for i in 0..10 {
        for j in 0..6 {
            f.push(rod(
                up(Point3::new(20.0 + i as f64 * 28.0, 20.0 + j as f64 * 32.0, -1.0)),
                2.0,
                t + 30.0,
                Combine::Cut,
            ));
        }
    }
    Part { features: f }
}

/// Rectangular and circular arrays of identical features, some overlapping
/// their neighbours, some rotated.
pub fn patterns() -> Part {
    let (w, t) = (250.0, 15.0);
    let mut f = vec![block(Frame::WORLD, Point2::ORIGIN, w, w, t, Combine::New)];
    for i in 0..10 {
        for j in 0..8 {
            f.push(rod(
                up(Point3::new(12.0 + i as f64 * 7.0, 12.0 + j as f64 * 7.0, -1.0)),
                3.0,
                t + 2.0,
                Combine::Cut,
            ));
        }
    }
    // overlapping neighbours: spacing below the diameter
    for i in 0..8 {
        f.push(rod(
            up(Point3::new(150.0 + i as f64 * 9.0, 30.0, t - 4.0)),
            4.8,
            10.0,
            Combine::Cut,
        ));
    }
    let centre = Point3::new(125.0, 150.0, 0.0);
    for k in 0..24 {
        let a = k as f64 * dmath::TAU / 24.0;
        let (s, c) = dmath::sin_cos(a);
        f.push(rod(
            up(centre + Vec3::new(80.0 * c, 80.0 * s, t - 1.0)),
            6.0,
            8.0,
            Combine::Add,
        ));
    }
    // radial slots meeting near the centre
    for k in 0..36 {
        let a = k as f64 * dmath::TAU / 36.0 + 0.013;
        f.push(rounded(
            turned(centre + Vec3::new(0.0, 0.0, t - 3.0), a),
            Point2::new(12.0, -2.0),
            40.0,
            4.0,
            1.9,
            10.0,
            Combine::Cut,
        ));
    }
    // a tight ring of small bosses that overlap each other
    for k in 0..30 {
        let a = k as f64 * dmath::TAU / 30.0 + 0.007;
        let (s, c) = dmath::sin_cos(a);
        f.push(rod(
            up(centre + Vec3::new(62.0 * c, 62.0 * s, t - 1.0)),
            7.0,
            5.0,
            Combine::Add,
        ));
    }
    // rotated slot grid
    for i in 0..6 {
        for j in 0..3 {
            let o = Point3::new(30.0 + i as f64 * 30.0, 210.0 + j as f64 * 12.0, t - 2.5);
            f.push(block(
                turned(o, 0.3 + 0.05 * i as f64),
                Point2::new(-10.0, -2.0),
                20.0,
                4.0,
                10.0,
                Combine::Cut,
            ));
        }
    }
    Part { features: f }
}

/// Revolved and spherical shapes mixed with extrudes: domes, spherical cuts,
/// rings, cones cut in, stepped revolved bosses, holes.
pub fn mixed() -> Part {
    let mut r = Rng::new(0x31_a3d);
    let (w, t) = (200.0, 20.0);
    let mut f = vec![block(Frame::WORLD, Point2::ORIGIN, w, w, t, Combine::New)];
    let spot = |r: &mut Rng, m: f64| (r.range(m, w - m), r.range(m, w - m));
    for _ in 0..40 {
        let (x, y) = spot(&mut r, 15.0);
        let rad = r.range(4.0, 12.0);
        f.push(ball(
            Point3::new(x, y, t - rad * r.range(0.1, 0.5)),
            rad,
            Combine::Add,
        ));
    }
    for _ in 0..40 {
        let (x, y) = spot(&mut r, 10.0);
        let rad = r.range(3.0, 8.0);
        f.push(ball(
            Point3::new(x, y, t + rad * r.range(0.2, 0.8)),
            rad,
            Combine::Cut,
        ));
    }
    for _ in 0..30 {
        let (x, y) = spot(&mut r, 25.0);
        let (big, tube) = (r.range(6.0, 15.0), r.range(1.0, 3.0));
        let ring = Profile {
            plane: standing(x, y, t),
            loops: vec![Profile::circle(Point2::new(big, -tube * 0.5), tube, 0)],
        };
        f.push(revolve(ring, VERTICAL, Combine::Add));
    }
    for _ in 0..40 {
        let (x, y) = spot(&mut r, 12.0);
        let (rb, hd) = (r.range(2.0, 7.0), r.range(4.0, 15.0));
        let tri = polygon(
            standing(x, y, t),
            &[
                Point2::new(0.0, -2.0),
                Point2::new(rb, -2.0),
                Point2::new(0.0, hd),
            ],
        );
        f.push(revolve(tri, VERTICAL, Combine::Cut));
    }
    for _ in 0..30 {
        let (x, y) = spot(&mut r, 15.0);
        let (r1, r2) = (r.range(5.0, 9.0), r.range(2.0, 4.5));
        let (h1, h2) = (r.range(3.0, 8.0), r.range(3.0, 10.0));
        let steps = [
            Point2::new(0.0, 1.0),
            Point2::new(r1, 1.0),
            Point2::new(r1, -h1),
            Point2::new(r2, -h1),
            Point2::new(r2, -h1 - h2),
            Point2::new(0.0, -h1 - h2),
        ];
        f.push(revolve(
            polygon(standing(x, y, t), &steps),
            VERTICAL,
            Combine::Add,
        ));
    }
    for _ in 0..20 {
        let (x, y) = spot(&mut r, 8.0);
        f.push(rod(
            up(Point3::new(x, y, -1.0)),
            r.range(1.5, 4.0),
            t + 30.0,
            Combine::Cut,
        ));
    }
    Part { features: f }
}

fn hole(at: Point3, diameter: f64, head: Head, end: End) -> Hole {
    Hole {
        at: frame(at, -Vec3::Z, Vec3::X),
        diameter,
        head,
        end,
        point: None,
        thread: None,
    }
}

/// Every pattern and hole kind: tapped blind holes in bosses, patterned in a row and mirrored,
/// a ring of countersunk holes, then the whole part mirrored and drilled across the seam.
pub fn tour() -> Part {
    let (w, h, t) = (140.0, 90.0, 15.0);
    let row = |dir: Vec3, count: usize, spacing: f64| Row { dir, count, spacing };
    let features = vec![
        block(Frame::WORLD, Point2::ORIGIN, w, h, t, Combine::New),
        rod(up(Point3::new(20.0, 20.0, t - 1.0)), 6.0, 11.0, Combine::Add),
        Feature::Hole(Box::new(Hole {
            point: Some(118.0 / 180.0 * dmath::PI),
            thread: Some(Thread {
                size: 5.0,
                pitch: 0.8,
            }),
            ..hole(
                Point3::new(20.0, 20.0, t + 10.0),
                4.2,
                Head::Plain,
                End::Depth(12.0),
            )
        })),
        Feature::Pattern {
            of: Source::Features(vec![1, 2]),
            layout: Layout::Linear {
                first: row(Vec3::X, 3, 25.0),
                second: None,
            },
        },
        Feature::Hole(Box::new(hole(
            Point3::new(123.0, 55.0, t),
            5.0,
            Head::Countersink {
                diameter: 10.0,
                angle: dmath::FRAC_PI_2,
            },
            End::Through,
        ))),
        Feature::Pattern {
            of: Source::Features(vec![4]),
            layout: Layout::Circular {
                origin: Point3::new(105.0, 55.0, 0.0),
                axis: Vec3::Z,
                count: 6,
                angle: dmath::TAU,
                incremental: false,
            },
        },
        Feature::Pattern {
            of: Source::Features(vec![3]),
            layout: Layout::Mirror {
                plane: frame(Point3::new(0.0, h / 2.0, 0.0), Vec3::Y, Vec3::X),
            },
        },
        Feature::Pattern {
            of: Source::Body,
            layout: Layout::Mirror {
                plane: frame(Point3::new(w, 0.0, 0.0), Vec3::X, Vec3::Y),
            },
        },
        Feature::Hole(Box::new(hole(
            Point3::new(w, h / 2.0, t),
            8.0,
            Head::Counterbore {
                diameter: 14.0,
                depth: 5.0,
            },
            End::Through,
        ))),
    ];
    Part { features }
}

/// A plate drilled with a 20 x 20 grid of counterbored through holes: one hole, patterned.
pub fn hole_grid() -> Part {
    let (n, pitch, t) = (20, 20.0, 12.0);
    let w = n as f64 * pitch;
    let row = |dir: Vec3| Row {
        dir,
        count: n,
        spacing: pitch,
    };
    let counterbore = Head::Counterbore {
        diameter: 11.0,
        depth: 6.5,
    };
    let features = vec![
        block(Frame::WORLD, Point2::ORIGIN, w, w, t, Combine::New),
        Feature::Hole(Box::new(hole(
            Point3::new(pitch / 2.0, pitch / 2.0, t),
            6.6,
            counterbore,
            End::Through,
        ))),
        Feature::Pattern {
            of: Source::Features(vec![1]),
            layout: Layout::Linear {
                first: row(Vec3::X),
                second: Some(row(Vec3::Y)),
            },
        },
    ];
    Part { features }
}

/// Posts up to a dome and up to the underside of a rod lying over a plate, pockets down to a
/// tilted plane, holes down to the next face; or (`flat`) the same, each ending flat at about
/// the same place, to price ends on a surface.
pub fn upto(flat: bool) -> Part {
    let (w, h, t) = (120.0, 80.0, 10.0);
    let mut f = vec![
        block(Frame::WORLD, Point2::ORIGIN, w, h, t, Combine::New),
        ball(Point3::new(30.0, 40.0, t), 15.0, Combine::Add),
        rod(
            frame(Point3::new(60.0, 10.0, 35.0), Vec3::Y, Vec3::X),
            8.0,
            60.0,
            Combine::Add,
        ),
        Feature::Plane(frame(
            Point3::new(0.0, 0.0, 6.0),
            Vec3::new(0.05, 0.03, 1.0),
            Vec3::X,
        )),
    ];
    let dome = Name::feature(2).with_str("side").with(0).with(0);
    let under = Name::feature(3).with_str("side").with(0);
    let end = |e: Extent, d: f64| if flat { Extent::Distance(d) } else { e };
    let post = |x: f64, y: f64, e: Extent| Feature::Extrude {
        profile: Profile {
            plane: up(Point3::new(x, y, t)),
            loops: vec![Profile::circle(Point2::ORIGIN, 1.5, 0)],
        },
        extent: e,
        op: Combine::Add,
    };
    for k in 0..4 {
        let a = k as f64 * dmath::FRAC_PI_2 + 0.3;
        let (s, c) = dmath::sin_cos(a);
        let e = Extent::To {
            target: Target::Face {
                feature: 1,
                name: dome,
            },
            offset: 2.0,
        };
        f.push(post(30.0 + 8.0 * c, 40.0 + 8.0 * s, end(e, 14.0)));
    }
    for k in 0..4 {
        let e = Extent::To {
            target: Target::Face {
                feature: 2,
                name: under,
            },
            offset: 1.0,
        };
        f.push(post(
            60.0 + 2.0 * k as f64 - 3.0,
            20.0 + 12.0 * k as f64,
            end(e, 18.5),
        ));
    }
    for k in 0..4 {
        f.push(Feature::Extrude {
            profile: Profile {
                plane: up(Point3::new(80.0 + 8.0 * k as f64, 60.0, t)),
                loops: vec![Profile::rectangle(Point2::ORIGIN, 5.0, 8.0, 0)],
            },
            extent: end(
                Extent::To {
                    target: Target::Plane(3),
                    offset: 0.0,
                },
                -1.0,
            ),
            op: Combine::Cut,
        });
    }
    for k in 0..4 {
        f.push(Feature::Hole(Box::new(Hole {
            end: if flat { End::Through } else { End::ToNext },
            ..hole(
                Point3::new(90.0 + 8.0 * k as f64, 20.0, t),
                3.0,
                Head::Plain,
                End::Through,
            )
        })));
    }
    Part { features: f }
}

/// A sweep along `path`, its profile square to the path's start.
fn swept(profile_loops: Vec<ProfileLoop>, x: Vec3, path: Vec<PathSeg>, op: Combine) -> Feature {
    let (o, t) = match path[0] {
        PathSeg::Line { a, b } => (a, b - a),
        PathSeg::Arc { .. } => unreachable!("the suite's paths start straight"),
    };
    Feature::Sweep(Box::new(Sweep {
        profile: Profile {
            plane: frame(o, t, x),
            loops: profile_loops,
        },
        path,
        op,
    }))
}

/// A quarter arc from `a`, leaving along unit `t`, bending towards unit `n` with radius `r`.
fn bend(a: Point3, t: Vec3, n: Vec3, r: f64) -> (PathSeg, Point3) {
    let c = a + n * r;
    let circle = Circle {
        frame: frame(c, (-n).cross(t), -n),
        radius: r,
    };
    (
        PathSeg::Arc {
            circle,
            start: 0.0,
            sweep: dmath::FRAC_PI_2,
        },
        c + t * r,
    )
}

/// A block with a pipe swept through a bend out of its side and top, a square channel swept
/// round a corner across its top, and a lens-shaped groove round a bend too tight for a torus.
pub fn sweep() -> Part {
    let mut f = vec![block(
        Frame::WORLD,
        Point2::ORIGIN,
        120.0,
        80.0,
        20.0,
        Combine::New,
    )];
    // the pipe: in from the side, up through the top
    let a = Point3::new(-20.0, 20.0, 10.0);
    let b = Point3::new(30.0, 20.0, 10.0);
    let (arc, c) = bend(b, Vec3::X, Vec3::Z, 20.0);
    f.push(swept(
        vec![Profile::circle(Point2::ORIGIN, 6.0, 0)],
        Vec3::Y,
        vec![
            PathSeg::Line { a, b },
            arc,
            PathSeg::Line {
                a: c,
                b: c + Vec3::Z * 25.0,
            },
        ],
        Combine::Add,
    ));
    // the channel: along y, round a corner towards -x, half sunk in the top
    let a = Point3::new(90.0, -10.0, 20.0);
    let b = Point3::new(90.0, 30.0, 20.0);
    let (arc, c) = bend(b, Vec3::Y, -Vec3::X, 15.0);
    f.push(swept(
        vec![Profile::rectangle(Point2::new(-2.0, -2.0), 4.0, 4.0, 0)],
        Vec3::X,
        vec![
            PathSeg::Line { a, b },
            arc,
            PathSeg::Line {
                a: c,
                b: c - Vec3::X * 20.0,
            },
        ],
        Combine::Cut,
    ));
    // the lens: the far arc's circle reaches past the bend's axis
    let half = dmath::atan(0.5);
    let (d, rad) = (2.0, 5f64.sqrt());
    let lens = ProfileLoop::new(vec![
        (
            Segment::Arc {
                center: Point2::new(-d, 0.0),
                radius: rad,
                start: -half,
                sweep: 2.0 * half,
            },
            0,
        ),
        (
            Segment::Arc {
                center: Point2::new(d, 0.0),
                radius: rad,
                start: dmath::PI - half,
                sweep: 2.0 * half,
            },
            1,
        ),
    ]);
    let a = Point3::new(40.0, 90.0, 20.0);
    let b = Point3::new(40.0, 60.0, 20.0);
    let (arc, c) = bend(b, -Vec3::Y, Vec3::X, 2.5);
    f.push(swept(
        vec![lens],
        Vec3::X,
        vec![
            PathSeg::Line { a, b },
            arc,
            PathSeg::Line {
                a: c,
                b: c + Vec3::X * 30.0,
            },
        ],
        Combine::Cut,
    ));
    Part { features: f }
}

/// A rod on a plate with a thread cut round it and a spring wound about it, its first turn sunk
/// in the plate, and a tapered spring beside it.
pub fn coil() -> Part {
    let up_axis = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::new(0.0, -1.0),
    };
    let wound = |plane: Frame, loops: Vec<ProfileLoop>, pitch: f64, turns: f64, taper: f64, op: Combine| {
        Feature::Coil(Box::new(Coil {
            profile: Profile { plane, loops },
            axis: up_axis,
            pitch,
            turns,
            taper,
            op,
        }))
    };
    Part {
        features: vec![
            block(
                Frame::WORLD,
                Point2::new(-30.0, -30.0),
                100.0,
                60.0,
                5.0,
                Combine::New,
            ),
            rod(up(Point3::new(0.0, 0.0, 4.0)), 8.0, 56.0, Combine::Add),
            wound(
                standing(0.0, 0.0, 12.0),
                vec![Profile::rectangle(Point2::new(7.0, -0.8), 2.0, 1.6, 0)],
                4.0,
                8.0,
                0.0,
                Combine::Cut,
            ),
            wound(
                standing(0.0, 0.0, 4.0),
                vec![Profile::circle(Point2::new(15.0, 0.0), 1.5, 0)],
                6.0,
                5.5,
                0.0,
                Combine::Add,
            ),
            wound(
                standing(45.0, 0.0, 4.0),
                vec![Profile::rectangle(Point2::new(10.0, -0.5), 2.0, 2.0, 0)],
                5.0,
                4.0,
                -0.15,
                Combine::Add,
            ),
        ],
    }
}

fn lofted(profiles: Vec<Profile>, op: Combine) -> Feature {
    Feature::Loft(Box::new(Loft { profiles, op }))
}

/// A block with a twisted square loft standing out of it, an offset round loft cut through it,
/// and a three-section frustum.
pub fn loft() -> Part {
    let square = |o: Point3, s: f64, turn: f64| Profile {
        plane: turned(o, turn),
        loops: vec![Profile::rectangle(Point2::new(-s, -s), 2.0 * s, 2.0 * s, 0)],
    };
    let round = |o: Point3, r: f64| Profile {
        plane: up(o),
        loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
    };
    Part {
        features: vec![
            block(Frame::WORLD, Point2::ORIGIN, 100.0, 60.0, 15.0, Combine::New),
            lofted(
                vec![
                    square(Point3::new(30.0, 30.0, 10.0), 15.0, 0.0),
                    square(Point3::new(30.0, 30.0, 40.0), 8.0, 0.5),
                ],
                Combine::Add,
            ),
            lofted(
                vec![
                    round(Point3::new(75.0, 30.0, -1.0), 8.0),
                    round(Point3::new(80.0, 30.0, 16.0), 4.0),
                ],
                Combine::Cut,
            ),
            lofted(
                vec![
                    square(Point3::new(75.0, 50.0, 10.0), 4.0, 0.0),
                    square(Point3::new(75.0, 50.0, 25.0), 6.0, 0.0),
                    square(Point3::new(75.0, 50.0, 35.0), 2.5, 0.0),
                ],
                Combine::Add,
            ),
        ],
    }
}

/// A random history: a random block, then `count` features of random kind,
/// placement and orientation (sometimes tilted). Nothing is coincident on purpose.
pub fn fuzz(seed: u64, count: usize) -> Part {
    let mut r = Rng::new(seed.wrapping_mul(0x9e37_79b9).wrapping_add(17));
    let (w, h, t) = (r.range(60.0, 200.0), r.range(60.0, 200.0), r.range(10.0, 40.0));
    let mut f = vec![block(Frame::WORLD, Point2::ORIGIN, w, h, t, Combine::New)];
    for _ in 0..count {
        let (x, y) = (r.range(0.0, w), r.range(0.0, h));
        let z = r.range(-0.3 * t, 1.1 * t);
        let o = Point3::new(x, y, z);
        let op = if r.bool() { Combine::Cut } else { Combine::Add };
        let plane = if r.below(5) == 0 {
            let n = Vec3::new(r.range(-0.5, 0.5), r.range(-0.5, 0.5), 1.0);
            frame(o, n, Vec3::new(r.range(-1.0, 1.0), 1.0, 0.0))
        } else {
            turned(o, r.range(0.0, dmath::TAU))
        };
        let size = r.range(2.0, 0.25 * w.min(h));
        let depth = r.range(1.0, t);
        f.push(match r.below(10) {
            0..=2 => rod(plane, size * 0.5, depth, op),
            3..=4 => block(
                plane,
                Point2::new(-size / 2.0, -size / 3.0),
                size,
                size * 0.66,
                depth,
                op,
            ),
            5..=6 => {
                let (a, b) = (size, size * r.range(0.3, 1.0));
                rounded(
                    plane,
                    Point2::new(-a / 2.0, -b / 2.0),
                    a,
                    b,
                    b * r.range(0.1, 0.45),
                    depth,
                    op,
                )
            }
            7..=8 => ball(o, size * 0.4, op),
            _ => {
                let (big, tube) = (size * 0.5, size * r.range(0.05, 0.2));
                let ring = Profile {
                    plane: standing(x, y, z),
                    loops: vec![Profile::circle(Point2::new(big, 0.0), tube, 0)],
                };
                revolve(ring, VERTICAL, op)
            }
        });
    }
    Part { features: f }
}
