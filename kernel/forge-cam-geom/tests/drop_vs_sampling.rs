//! L6 for the drop-cutter (drop.rs): its certified CL against dense sampling of the face and
//! against the branch and bound alone (the closed forms' reference); the vertical gap of a move
//! against sampling; the column's containment of each standard cutter; the grown profile.
use forge_cam_geom::check::{Face, Move, Surf};
use forge_cam_geom::drop::{Column, Gap, drop, drop_general, gap_move};
use forge_cam_geom::ival::Ival;
use forge_cam_geom::tool::{Cutter, assemble};

struct Rng(u64);
impl Rng {
    fn f(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn r(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f()
    }
}

fn cutters() -> Vec<(&'static str, Cutter)> {
    vec![
        ("flat6", Cutter::Flat { d: 6.0 }),
        ("ball6", Cutter::Ball { d: 6.0 }),
        ("bull8r1.5", Cutter::Bull { d: 8.0, re: 1.5 }),
        ("chamfer90", Cutter::Chamfer { d_tip: 1.0, angle: 90.0, d: 10.0 }),
        ("drill118", Cutter::Drill { d: 6.0, angle: 118.0 }),
    ]
}

fn column(c: &Cutter) -> (Column, forge_cam_geom::tool::Body) {
    let sh = assemble(c, 20.0, 2.0 * c.radius(), 30.0, None).unwrap();
    (Column::of(&sh.bodies[0]).unwrap(), sh.bodies[0].clone())
}

fn unit(v: [f64; 3]) -> [f64; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / l, v[1] / l, v[2] / l]
}
fn crs(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// A random face of each kind about the origin, within reach of tools near it.
fn face(rng: &mut Rng, kind: usize) -> Face {
    let ax = unit([rng.r(-0.6, 0.6), rng.r(-0.6, 0.6), 1.0]);
    let x = unit(crs([0.3, 1.0, 0.1], ax));
    let y = crs(ax, x);
    let o = [rng.r(-2.0, 2.0), rng.r(-2.0, 2.0), rng.r(-3.0, 0.0)];
    match kind {
        0 => Face::new(Surf::Plane { o, u: x, v: y }, Ival::new(-6.0, 6.0), Ival::new(-6.0, 6.0), None, 1e-9, 0),
        1 => Face::new(Surf::Sphere { o, x, y, ax, r: rng.r(2.0, 12.0) }, Ival::new(-1.0, 1.0), Ival::new(rng.r(-0.6, 0.3), 1.0), None, 1e-9, 0),
        2 => {
            // a cylinder with a horizontal axis (closed form) or a tilted one (branch and bound)
            let a = if rng.f() < 0.5 { unit([1.0, rng.r(-1.0, 1.0), 0.0]) } else { unit([1.0, 0.2, 0.4]) };
            let xx = unit(crs([0.0, 0.0, 1.0], a));
            let yy = crs(a, xx);
            Face::new(Surf::Cyl { o, x: xx, y: yy, ax: a, r: rng.r(1.0, 8.0) }, Ival::new(-1.0, 1.0), Ival::new(-8.0, 8.0), None, 1e-9, 0)
        }
        3 => Face::new(
            Surf::Torus { o, x, y, ax, big: rng.r(4.0, 10.0), r: rng.r(1.0, 3.0), cm: 0.0, sm: 1.0 },
            Ival::new(-1.0, 1.0),
            Ival::new(-1.0, 1.0),
            None,
            1e-9,
            0,
        ),
        5 => {
            // a bowl: a sphere's lower part (axis up), or a horizontal cylinder's lower half
            if rng.f() < 0.5 {
                let up = [0.0, 0.0, 1.0];
                let xx = unit(crs([0.3, 1.0, 0.1], up));
                let yy = crs(up, xx);
                Face::new(Surf::Sphere { o, x: xx, y: yy, ax: up, r: rng.r(3.5, 12.0) }, Ival::new(-1.0, 1.0), Ival::new(-1.0, rng.r(-0.5, 0.0)), None, 1e-9, 0)
            } else {
                let a = unit([1.0, rng.r(-1.0, 1.0), 0.0]);
                let xx = [0.0, 0.0, -1.0];
                let yy = crs(a, xx);
                let th = (std::f64::consts::PI / 4.0 * rng.r(0.3, 1.0)).tan();
                Face::new(Surf::Cyl { o, x: xx, y: yy, ax: a, r: rng.r(2.0, 8.0) }, Ival::new(-th, th), Ival::new(-8.0, 8.0), None, 1e-9, 0)
            }
        }
        6 => {
            // a torus about a vertical axis, its tube arc anywhere (a convex round or a foot)
            let up = [0.0, 0.0, if rng.f() < 0.8 { 1.0 } else { -1.0 }];
            let xx = unit(crs([0.3, 1.0, 0.1], up));
            let yy = crs(up, xx);
            let pm = rng.r(-std::f64::consts::PI, std::f64::consts::PI);
            Face::new(
                Surf::Torus { o, x: xx, y: yy, ax: up, big: rng.r(4.0, 10.0), r: rng.r(1.0, 4.0), cm: pm.cos(), sm: pm.sin() },
                Ival::new(-1.0, 1.0),
                Ival::new(-rng.r(0.1, 0.9), rng.r(0.1, 0.9)),
                None,
                1e-9,
                0,
            )
        }
        _ => Face::new(
            Surf::Cone { o, x, y, ax, r0: rng.r(1.0, 5.0), k: rng.r(-0.8, 0.8) },
            Ival::new(-1.0, 1.0),
            Ival::new(-2.0, 3.0),
            None,
            1e-9,
            0,
        ),
    }
}

/// The largest y.z − b(ρ) over a grid of the face's parameters (a lower bound on CL, up to f64
/// rounding), or −∞.
fn sampled(col: &Column, f: &Face, x: f64, y: f64, n: usize) -> f64 {
    let mut m = f64::NEG_INFINITY;
    for i in 0..=n {
        for j in 0..=n {
            let p = f.p.lo + (f.p.hi - f.p.lo) * i as f64 / n as f64;
            let q = f.q.lo + (f.q.hi - f.q.lo) * j as f64 / n as f64;
            let pt = f.surf.eval(Ival::pt(p), Ival::pt(q));
            let (px, py, pz) = (pt[0].mid(), pt[1].mid(), pt[2].mid());
            let rho = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            if rho > col.r {
                continue;
            }
            m = m.max(pz - col.b_at(rho).mid());
        }
    }
    m
}

/// The least over a grid of the face of the column's signed distance (its solid's, at the
/// interval's middle), over points within R.
fn sampled_gap(col: &Column, f: &Face, x: f64, y: f64, z: f64, n: usize) -> f64 {
    let mut m = f64::INFINITY;
    for i in 0..=n {
        for j in 0..=n {
            let p = f.p.lo + (f.p.hi - f.p.lo) * i as f64 / n as f64;
            let q = f.q.lo + (f.q.hi - f.q.lo) * j as f64 / n as f64;
            let pt = f.surf.eval(Ival::pt(p), Ival::pt(q));
            let (px, py, pz) = (pt[0].mid(), pt[1].mid(), pt[2].mid());
            let rho = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            if rho > col.r {
                continue;
            }
            m = m.min(col.solid.sd_rz(Ival::pt(rho), Ival::pt(pz - z)).mid());
        }
    }
    m
}

#[test]
fn drop_holds_every_sample_and_agrees_with_the_general_search() {
    let mut rng = Rng(0x9e3779b97f4a7c15);
    let (mut n, mut closed_cells, mut bb_cells) = (0, 0usize, 0usize);
    for (name, c) in cutters() {
        let (col, _) = column(&c);
        for kind in 0..7 {
            for _ in 0..3 {
                let f = face(&mut rng, kind);
                let (x, y) = (rng.r(-5.0, 5.0), rng.r(-5.0, 5.0));
                let res = 1e-3;
                let a = drop(&col, std::slice::from_ref(&f), &[0], x, y, res, 400_000);
                let b = drop_general(&col, std::slice::from_ref(&f), &[0], x, y, res, 400_000);
                closed_cells += a.cells;
                bb_cells += b.cells;
                let s = sampled(&col, &f, x, y, 120);
                let eps = if s.is_finite() { 1e-9 * (1.0 + s.abs()) } else { 0.0 };
                if s == f64::NEG_INFINITY { n += 1; continue; }
                // every sample is a real point: CL ≥ it, so hi ≥ it
                assert!(s <= a.hi + eps && s <= b.hi + eps, "{name} kind {kind}: sample {s} above {a:?} / {b:?}");
                // the two certified answers enclose the same number
                if a.hi > f64::NEG_INFINITY || b.hi > f64::NEG_INFINITY {
                    assert!(a.lo <= b.hi + eps && b.lo <= a.hi + eps, "{name} kind {kind}: {a:?} vs {b:?}");
                }
                // resolved, unless the budget ran out
                if a.cells < 400_000 && a.hi > f64::NEG_INFINITY {
                    assert!(a.hi - a.lo <= res * 1.0001, "{name} kind {kind}: {a:?}");
                    // and sampling comes near it from below
                    assert!(s >= a.lo - 0.25, "{name} kind {kind}: sample {s} far below {a:?}");
                }
                n += 1;
            }
        }
    }
    println!("{n} drops: closed forms + B&B {closed_cells} cells, B&B alone {bb_cells} cells");
}

#[test]
fn a_cleared_move_is_clear_at_every_sample() {
    let mut rng = Rng(0x2545f4914f6cdd1d);
    let (mut clear, mut over, mut unknown) = (0, 0, 0);
    for (name, c) in cutters() {
        let (col, _) = column(&c);
        for kind in 0..7 {
            for _ in 0..4 {
                let f = face(&mut rng, kind);
                let a = [rng.r(-4.0, 4.0), rng.r(-4.0, 4.0)];
                let b = [a[0] + rng.r(-3.0, 3.0), a[1] + rng.r(-3.0, 3.0)];
                // heights near the CL at the ends, a little above or below
                let za = drop(&col, std::slice::from_ref(&f), &[0], a[0], a[1], 1e-3, 400_000).hi;
                let zb = drop(&col, std::slice::from_ref(&f), &[0], b[0], b[1], 1e-3, 400_000).hi;
                if !(za.is_finite() && zb.is_finite()) {
                    continue;
                }
                let lift = rng.r(-0.05, 0.3);
                let mv = Move::Line { a: [a[0], a[1], za + lift], b: [b[0], b[1], zb + lift] };
                let limit = -0.01;
                let (g, _) = gap_move(&col, &f, &mv, limit, 1e-4, 400_000);
                match g {
                    Gap::Clear { lower } => {
                        clear += 1;
                        for k in 0..=20 {
                            let t = k as f64 / 20.0;
                            let (x, y) = (a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]));
                            let z = za + lift + t * (zb - za);
                            // the column's signed distance ≥ lower at every sampled face point
                            let v = sampled_gap(&col, &f, x, y, z, 30);
                            assert!(v >= lower - 1e-9, "{name} kind {kind}: v {v} < {lower}");
                        }
                    }
                    Gap::Over { s, by, .. } => {
                        over += 1;
                        // a real point: the drop there is at least by above the move
                        let (x, y) = (a[0] + s * (b[0] - a[0]), a[1] + s * (b[1] - a[1]));
                        let z = za + lift + s * (zb - za);
                        let cl = drop(&col, std::slice::from_ref(&f), &[0], x, y, 1e-5, 400_000);
                        assert!(cl.hi - z >= by - 1e-5, "{name} kind {kind}: over by {by} but CL {cl:?} at z {z}");
                    }
                    Gap::Unknown => unknown += 1,
                }
            }
        }
    }
    println!("moves: {clear} clear, {over} over, {unknown} unknown");
    assert!(clear > 25 && over > 3);
}

#[test]
fn each_cutter_lies_in_its_column() {
    let mut rng = Rng(7);
    for (name, c) in cutters() {
        let (col, body) = column(&c);
        assert!(col.holds_or_is(&body), "{name}");
        let mut inside = 0;
        for _ in 0..20_000 {
            let (r, z) = (rng.r(0.0, col.r + 1.0), rng.r(-1.0, 25.0));
            let sd = body.sd_rz(Ival::pt(r), Ival::pt(z));
            if sd.hi < 0.0 {
                inside += 1;
                // a point of the body is in the column: within R and on or above the profile
                assert!(r <= col.r && z >= col.b_at(r.min(col.r)).lo, "{name}: ({r}, {z})");
            }
        }
        assert!(inside > 1000, "{name}");
    }
}

#[test]
fn a_grown_ball_is_a_bigger_ball() {
    let (ball, _) = column(&Cutter::Ball { d: 6.0 });
    let g = ball.grow(0.5).unwrap();
    assert_eq!(g.r, 3.5);
    for k in 0..=20 {
        let rho = 3.5 * k as f64 / 20.0;
        // centre (0, 3) radius 3.5
        let want = 3.0 - (3.5f64 * 3.5 - rho * rho).sqrt();
        let b = g.b_at(rho);
        assert!(b.lo <= want + 1e-12 && b.hi >= want - 1e-12, "{rho}: {b:?} {want}");
    }
    let (flat, _) = column(&Cutter::Flat { d: 6.0 });
    let g = flat.grow(1.0).unwrap();
    assert_eq!(g.r, 4.0);
    assert!(g.b_at(2.0).lo <= -1.0 && g.b_at(2.0).hi >= -1.0);
    let b = g.b_at(4.0);
    assert!(b.lo <= 0.0 && b.hi >= 0.0);
}
