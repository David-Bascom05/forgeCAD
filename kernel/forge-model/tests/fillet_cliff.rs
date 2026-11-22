//! Rounds whose ball runs over a sharp rim and back (a cliff): a pipe let into a round or square
//! body, its round's spring running off the pipe's end or the body's bottom for
//! a stretch. Before, refused; now the ball rests on the rim there, held against the two solids'
//! own distances: every ball the round's faces say they hold touches the solids at its radius and
//! lies clear of them, and the volume it adds is the morphological closing's.

use forge_bool::{Op, boolean};
use forge_geom::{Surface, SurfaceGeom};
use forge_math::{Frame, Point2, Point3, Vec3};
use forge_ops::{FilletEdge, FilletOptions, Profile, Radius, extrude, fillet_with};
use forge_topo::{Body, Name};
use std::collections::BTreeSet;

/// A solid of the scene by its distance: a capped cylinder (axis origin, unit direction, length,
/// radius) or a box (corner, its unit x, y, z, and sizes).
#[derive(Clone, Copy)]
enum Solid {
    Pipe(Point3, Vec3, f64, f64),
    Block(Point3, [Vec3; 3], [f64; 3]),
}

impl Solid {
    /// Distance from p outside, minus the depth inside.
    fn distance(&self, p: Point3) -> f64 {
        match *self {
            Solid::Pipe(o, d, h, r) => {
                let v = p - o;
                let a = v.dot(d);
                let rad = (v - d * a).length();
                let e = if a < 0.0 {
                    -a
                } else if a > h {
                    a - h
                } else {
                    0.0
                };
                let q = (rad - r).max(0.0);
                if e == 0.0 && q == 0.0 {
                    -(r - rad).min(a).min(h - a)
                } else {
                    (e * e + q * q).sqrt()
                }
            }
            Solid::Block(o, axes, size) => {
                let v = p - o;
                let mut out = 0.0f64;
                let mut depth = f64::INFINITY;
                for k in 0..3 {
                    let t = v.dot(axes[k]);
                    let e = if t < 0.0 {
                        -t
                    } else if t > size[k] {
                        t - size[k]
                    } else {
                        0.0
                    };
                    out += e * e;
                    depth = depth.min(t).min(size[k] - t);
                }
                if out > 0.0 { out.sqrt() } else { -depth }
            }
        }
    }

    /// The solid made, a pipe's seam where its frame's x (the world's x off the axis) points.
    fn body(&self, id: u64) -> Body {
        match *self {
            Solid::Pipe(o, d, h, r) => {
                let z = d.normalize().expect("axis");
                let frame = Frame::from_zx(o, z, Vec3::X).expect("frame");
                extrude(
                    &Profile {
                        plane: frame,
                        loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
                    },
                    h,
                    id,
                )
                .expect("pipe")
            }
            Solid::Block(o, axes, size) => {
                let z = axes[2].normalize().expect("axis");
                extrude(
                    &Profile {
                        plane: Frame::from_zx(o, z, axes[0]).expect("frame"),
                        loops: vec![Profile::rectangle(Point2::ORIGIN, size[0], size[1], 0)],
                    },
                    size[2],
                    id,
                )
                .expect("block")
            }
        }
    }
}

/// The body, the pipe let into it with its seam turned `turn` radians from where it starts.
fn scene(body: Solid, pipe: Solid, turn: f64) -> Body {
    let Solid::Pipe(o, d, h, r) = pipe else {
        panic!("a pipe")
    };
    let z = d.normalize().expect("axis");
    let base = Frame::from_zx(o, z, Vec3::new(1.0, 0.3, 0.2)).expect("frame");
    let (s, c) = forge_math::dmath::sin_cos(turn);
    let x = base.x().get() * c + base.y().get() * s;
    let boss = extrude(
        &Profile {
            plane: Frame::from_zx(o, z, x).expect("frame"),
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        },
        h,
        2,
    )
    .expect("pipe");
    boolean(&body.body(1), &boss, Op::Union, 3).expect("union")
}

/// The concave edges between the pipe's wall (a cylinder of radius `r`) and the body.
fn foot(b: &Body, r: f64) -> Vec<FilletEdge> {
    b.edge_ids()
        .filter(|&e| {
            let faces: Vec<_> = b.edge(e).coedges.iter().map(|&c| b.coedge_face(c)).collect();
            let wall = |f| matches!(b.face(f).surface, Surface::Cylinder(k) if (k.radius - r).abs() < 1e-9);
            faces.len() == 2
                && faces[0] != faces[1]
                && faces.iter().any(|&f| wall(f))
                && !faces.iter().all(|&f| wall(f))
                && forge_ops::fillet_kind(b, e) == Some(false)
        })
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(RADIUS.with(|r| r.get())),
        })
        .collect()
}

thread_local! {
    static RADIUS: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
}

/// The round of the pipe's foot at radius `r`: valid, and its volume.
fn rounded(body: Solid, pipe: Solid, r: f64, turn: f64) -> (Body, Body) {
    let Solid::Pipe(.., rp) = pipe else {
        panic!("a pipe")
    };
    RADIUS.with(|c| c.set(r));
    let before = scene(body, pipe, turn);
    let picks = foot(&before, rp);
    assert!(!picks.is_empty(), "the pipe's foot");
    let after = fillet_with(&before, &picks, &FilletOptions::default(), 9).expect("rounded");
    assert!(
        forge_topo::check(&after).is_empty(),
        "{:?}",
        forge_topo::check(&after)
    );
    (before, after)
}

fn volume(b: &Body) -> f64 {
    forge_topo::mass::mass(b).expect("mass").volume
}

/// Every new face's points: a ball of radius r on one side of each touches the solids (its centre
/// r from their union) and holds nothing of them. Returns the number of points held.
fn balls_hold(before: &Body, after: &Body, solids: &[Solid], r: f64) -> usize {
    let old: BTreeSet<Name> = before.face_ids().map(|f| before.face(f).name).collect();
    let d = |c: Point3| solids.iter().map(|s| s.distance(c)).fold(f64::INFINITY, f64::min);
    let mut n = 0;
    for f in after.face_ids().filter(|&f| !old.contains(&after.face(f).name)) {
        let loops = forge_topo::uv::face_uv_loops(after, f);
        let pts: Vec<Point2> = loops.iter().flat_map(|l| l.points.iter().copied()).collect();
        let (mut lo, mut hi) = (pts[0], pts[0]);
        for p in &pts {
            lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
        }
        let s = &after.face(f).surface;
        let mut here = 0;
        let mut worst = (0.0f64, Point3::ORIGIN);
        for i in 1..40 {
            for j in 1..12 {
                let uv = Point2::new(
                    lo.x + (hi.x - lo.x) * i as f64 / 40.0,
                    lo.y + (hi.y - lo.y) * j as f64 / 12.0,
                );
                if !forge_topo::uv::point_in_loops(uv, &loops) {
                    continue;
                }
                let (p, nrm) = (s.eval(uv), after.face_normal(f, uv).expect("normal").get());
                let best = [1.0, -1.0]
                    .iter()
                    .map(|&k| d(p + nrm * (k * r)))
                    .fold(
                        f64::INFINITY,
                        |a, x| if (x - r).abs() < (a - r).abs() { x } else { a },
                    );
                if (best - r).abs() > worst.0 {
                    worst = ((best - r).abs(), p);
                }
                here += 1;
            }
        }
        assert!(here >= 20, "face {f:?}: {here} points");
        assert!(
            worst.0 <= 1e-6,
            "a ball on a {} at {:?} off the solids by {:e}",
            s.kind(),
            worst.1,
            worst.0
        );
        n += here;
    }
    n
}

fn unit(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3::new(x, y, z).normalize().expect("unit").get()
}

/// The pipe runs out of a round body low down, its end just clear of the body's wall: the
/// round's spring on the pipe runs off over the rim of its end for a stretch.
fn over_its_end() -> (Solid, Solid, f64) {
    (
        Solid::Pipe(Point3::new(10.0, -7.0, 0.0), Vec3::Z, 71.1, 21.49),
        Solid::Pipe(
            Point3::new(7.089, -7.141, 34.8),
            unit(-0.0591, 0.8173, -0.5731),
            35.82,
            12.13,
        ),
        3.96,
    )
}

/// The pipe runs out of a round body and on below its bottom: the spring on the body runs off
/// over the bottom's rim.
fn over_the_bottom() -> (Solid, Solid, f64) {
    (
        Solid::Pipe(Point3::new(10.0, -7.0, 0.0), Vec3::Z, 36.2, 17.3),
        Solid::Pipe(
            Point3::new(10.515, -6.69, 24.15),
            unit(-0.7506, 0.3504, -0.5602),
            43.15,
            8.2,
        ),
        3.17,
    )
}

/// The volume a ball rolling in the corner adds: the solids' closing, on a grid of step 0.05,
/// worked out apart from the kernel (research/fillets closing-style: a point is added where no ball
/// clear of the solids holds it). Points whose clear ball it nearly finds make it high by up to
/// about half a percent.
fn closing_added(case: &str) -> f64 {
    match case {
        "end" => CLOSING_END,
        _ => CLOSING_BOTTOM,
    }
}

const CLOSING_END: f64 = 135.94;
const CLOSING_BOTTOM: f64 = 83.77;

fn holds(case: &str, (body, pipe, r): (Solid, Solid, f64)) {
    let (before, after) = rounded(body, pipe, r, 0.0);
    assert!(balls_hold(&before, &after, &[body, pipe], r) > 60);
    let added = volume(&after) - volume(&before);
    // (the exact bits, to hold the native build to the wasm one: `-- --nocapture`)
    eprintln!("{case} cliff volume bits {:016x}", volume(&after).to_bits());
    let want = closing_added(case);
    assert!(
        (added - want).abs() <= 0.01 * want,
        "{case}: added {added}, the closing {want}"
    );
}

#[test]
fn round_over_the_pipes_end() {
    holds("end", over_its_end());
}

#[test]
fn round_over_the_bottoms_rim() {
    holds("bottom", over_the_bottom());
}

/// Where the pipe's seam lies is no part of the shape: the round is the same with it turned.
#[test]
fn round_over_the_end_whatever_the_seam() {
    let (body, pipe, r) = over_its_end();
    let base = {
        let (b, a) = rounded(body, pipe, r, 0.0);
        volume(&a) - volume(&b)
    };
    for turn in [0.6, 2.0, 4.2] {
        let (b, a) = rounded(body, pipe, r, turn);
        let added = volume(&a) - volume(&b);
        assert!(
            (added - base).abs() <= 2e-5 * base,
            "turned {turn}: {added} against {base}"
        );
    }
}

#[test]
fn round_over_the_bottom_whatever_the_seam() {
    let (body, pipe, r) = over_the_bottom();
    let base = {
        let (b, a) = rounded(body, pipe, r, 0.0);
        volume(&a) - volume(&b)
    };
    for turn in [0.6, 2.0, 4.2] {
        let (b, a) = rounded(body, pipe, r, turn);
        let added = volume(&a) - volume(&b);
        assert!(
            (added - base).abs() <= 2e-5 * base,
            "turned {turn}: {added} against {base}"
        );
    }
}

/// A pipe out of a square body's side: there the ball over the pipe's end would lean back into the
/// pipe's wall for a stretch (it should touch the wall, the end's rim and the side at once, which
/// nothing builds yet): refused, never a round with balls cutting the pipe.
#[test]
fn round_over_the_end_beside_a_square_side_refuses() {
    let body = Solid::Block(
        Point3::new(-6.87, -23.87, 0.0),
        [Vec3::X, Vec3::Y, Vec3::Z],
        [33.75, 33.75, 70.33],
    );
    let pipe = Solid::Pipe(
        Point3::new(13.17, -6.3, 39.98),
        unit(-0.2301, 0.9084, 0.3491),
        22.55,
        9.91,
    );
    RADIUS.with(|c| c.set(2.01));
    let before = scene(body, pipe, 0.0);
    let picks = foot(&before, 9.91);
    assert!(fillet_with(&before, &picks, &FilletOptions::default(), 9).is_err());
}
