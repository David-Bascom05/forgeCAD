//! L6 for the drop-cutter (drop.rs): its certified CL against dense sampling of the face and
//! against the branch and bound alone (the closed forms' reference); the vertical gap of a move
//! against sampling; the column's containment of each standard cutter; the grown profile.
use forge_cam_geom::check::{Face, Move, Surf};
use forge_cam_geom::drop::{Column, Gap, drop, drop_decide, drop_general, gap_clear, gap_move};
use forge_cam_geom::ival::Ival;
use forge_cam_geom::tool::{Cutter, assemble};
use forge_math::ipow::Powi as _;

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
        (
            "chamfer90",
            Cutter::Chamfer {
                d_tip: 1.0,
                angle: 90.0,
                d: 10.0,
            },
        ),
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
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// A random face of each kind about the origin, within reach of tools near it.
fn face(rng: &mut Rng, kind: usize) -> Face {
    let ax = unit([rng.r(-0.6, 0.6), rng.r(-0.6, 0.6), 1.0]);
    let x = unit(crs([0.3, 1.0, 0.1], ax));
    let y = crs(ax, x);
    let o = [rng.r(-2.0, 2.0), rng.r(-2.0, 2.0), rng.r(-3.0, 0.0)];
    match kind {
        0 => Face::new(
            Surf::Plane { o, u: x, v: y },
            Ival::new(-6.0, 6.0),
            Ival::new(-6.0, 6.0),
            None,
            1e-9,
            0,
        ),
        1 => Face::new(
            Surf::Sphere {
                o,
                x,
                y,
                ax,
                r: rng.r(2.0, 12.0),
            },
            Ival::new(-1.0, 1.0),
            Ival::new(rng.r(-0.6, 0.3), 1.0),
            None,
            1e-9,
            0,
        ),
        2 => {
            // a cylinder with a horizontal axis (closed form) or a tilted one (branch and bound)
            let a = if rng.f() < 0.5 {
                unit([1.0, rng.r(-1.0, 1.0), 0.0])
            } else {
                unit([1.0, 0.2, 0.4])
            };
            let xx = unit(crs([0.0, 0.0, 1.0], a));
            let yy = crs(a, xx);
            Face::new(
                Surf::Cyl {
                    o,
                    x: xx,
                    y: yy,
                    ax: a,
                    r: rng.r(1.0, 8.0),
                },
                Ival::new(-1.0, 1.0),
                Ival::new(-8.0, 8.0),
                None,
                1e-9,
                0,
            )
        }
        3 => Face::new(
            Surf::Torus {
                o,
                x,
                y,
                ax,
                big: rng.r(4.0, 10.0),
                r: rng.r(1.0, 3.0),
                cm: 0.0,
                sm: 1.0,
            },
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
                Face::new(
                    Surf::Sphere {
                        o,
                        x: xx,
                        y: yy,
                        ax: up,
                        r: rng.r(3.5, 12.0),
                    },
                    Ival::new(-1.0, 1.0),
                    Ival::new(-1.0, rng.r(-0.5, 0.0)),
                    None,
                    1e-9,
                    0,
                )
            } else {
                let a = unit([1.0, rng.r(-1.0, 1.0), 0.0]);
                let xx = [0.0, 0.0, -1.0];
                let yy = crs(a, xx);
                // the arc's half-angle tangent, up to tan(π/8)·… (under a quarter turn either side)
                let th = rng.r(0.15, 0.41);
                Face::new(
                    Surf::Cyl {
                        o,
                        x: xx,
                        y: yy,
                        ax: a,
                        r: rng.r(2.0, 8.0),
                    },
                    Ival::new(-th, th),
                    Ival::new(-8.0, 8.0),
                    None,
                    1e-9,
                    0,
                )
            }
        }
        6 => {
            // a torus about a vertical axis, its tube arc anywhere (a convex round or a foot)
            let up = [0.0, 0.0, if rng.f() < 0.8 { 1.0 } else { -1.0 }];
            let xx = unit(crs([0.3, 1.0, 0.1], up));
            let yy = crs(up, xx);
            // the tube's middle angle by the rational map (no platform trig)
            let t = rng.r(-3.0, 3.0);
            let (cpm, spm) = ((1.0 - t * t) / (1.0 + t * t), 2.0 * t / (1.0 + t * t));
            Face::new(
                Surf::Torus {
                    o,
                    x: xx,
                    y: yy,
                    ax: up,
                    big: rng.r(4.0, 10.0),
                    r: rng.r(1.0, 4.0),
                    cm: cpm,
                    sm: spm,
                },
                Ival::new(-1.0, 1.0),
                Ival::new(-rng.r(0.1, 0.9), rng.r(0.1, 0.9)),
                None,
                1e-9,
                0,
            )
        }
        _ => Face::new(
            Surf::Cone {
                o,
                x,
                y,
                ax,
                r0: rng.r(1.0, 5.0),
                k: rng.r(-0.8, 0.8),
            },
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
            let rho = ((px - x).ipow(2) + (py - y).ipow(2)).sqrt();
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
            let rho = ((px - x).ipow(2) + (py - y).ipow(2)).sqrt();
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
            for _ in 0..2 {
                let f = face(&mut rng, kind);
                let (x, y) = (rng.r(-5.0, 5.0), rng.r(-5.0, 5.0));
                let res = 1e-3;
                let a = drop(&col, std::slice::from_ref(&f), &[0], x, y, res, 60_000);
                let b = drop_general(&col, std::slice::from_ref(&f), &[0], x, y, res, 60_000);
                closed_cells += a.cells;
                bb_cells += b.cells;
                let s = sampled(&col, &f, x, y, 120);
                let eps = if s.is_finite() {
                    1e-9 * (1.0 + s.abs())
                } else {
                    0.0
                };
                if s == f64::NEG_INFINITY {
                    n += 1;
                    continue;
                }
                // every sample is a real point: CL ≥ it, so hi ≥ it
                assert!(
                    s <= a.hi + eps && s <= b.hi + eps,
                    "{name} kind {kind}: sample {s} above {a:?} / {b:?}"
                );
                // the two certified answers enclose the same number
                if a.hi > f64::NEG_INFINITY || b.hi > f64::NEG_INFINITY {
                    assert!(
                        a.lo <= b.hi + eps && b.lo <= a.hi + eps,
                        "{name} kind {kind}: {a:?} vs {b:?}"
                    );
                }
                // resolved, unless the budget ran out
                if a.cells < 60_000 && a.hi > f64::NEG_INFINITY {
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
            for _ in 0..3 {
                let f = face(&mut rng, kind);
                let a = [rng.r(-4.0, 4.0), rng.r(-4.0, 4.0)];
                let b = [a[0] + rng.r(-3.0, 3.0), a[1] + rng.r(-3.0, 3.0)];
                // heights near the CL at the ends, a little above or below
                let za = drop(&col, std::slice::from_ref(&f), &[0], a[0], a[1], 1e-3, 60_000).hi;
                let zb = drop(&col, std::slice::from_ref(&f), &[0], b[0], b[1], 1e-3, 60_000).hi;
                if !(za.is_finite() && zb.is_finite()) {
                    continue;
                }
                let lift = rng.r(-0.05, 0.3);
                let mv = Move::Line {
                    a: [a[0], a[1], za + lift],
                    b: [b[0], b[1], zb + lift],
                };
                let limit = -0.01;
                let (g, _) = gap_move(&col, &f, &mv, limit, 1e-4, 60_000);
                match g {
                    Gap::Clear { lower } => {
                        clear += 1;
                        for k in 0..=12 {
                            let t = k as f64 / 12.0;
                            let (x, y) = (a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]));
                            let z = za + lift + t * (zb - za);
                            // the column's signed distance ≥ lower at every sampled face point
                            let v = sampled_gap(&col, &f, x, y, z, 24);
                            assert!(v >= lower - 1e-9, "{name} kind {kind}: v {v} < {lower}");
                        }
                    }
                    Gap::Over { s, by, .. } => {
                        over += 1;
                        // a real point: the drop there is at least by above the move
                        let (x, y) = (a[0] + s * (b[0] - a[0]), a[1] + s * (b[1] - a[1]));
                        let z = za + lift + s * (zb - za);
                        let cl = drop(&col, std::slice::from_ref(&f), &[0], x, y, 1e-5, 60_000);
                        assert!(
                            cl.hi - z >= by - 1e-5,
                            "{name} kind {kind}: over by {by} but CL {cl:?} at z {z}"
                        );
                    }
                    Gap::Unknown => unknown += 1,
                }
            }
        }
    }
    println!("moves: {clear} clear, {over} over, {unknown} unknown");
    assert!(clear > 20 && over > 3);
}

/// `gap_clear` is gap_move's "Clear?" exactly: over moves near the CL, budgets small enough to
/// run out and resolutions coarse enough to leave cells unsplit, both say Clear or both don't.
#[test]
fn gap_clear_is_gap_moves_clear() {
    let mut rng = Rng(0x9e3779b97f4a7c15);
    let (mut clear, mut over, mut unknown) = (0, 0, 0);
    for (_, c) in cutters() {
        let (col, _) = column(&c);
        for kind in 0..7 {
            for _ in 0..6 {
                let f = face(&mut rng, kind);
                let a = [rng.r(-4.0, 4.0), rng.r(-4.0, 4.0)];
                let b = [a[0] + rng.r(-3.0, 3.0), a[1] + rng.r(-3.0, 3.0)];
                let za = drop(&col, std::slice::from_ref(&f), &[0], a[0], a[1], 1e-3, 60_000).hi;
                let zb = drop(&col, std::slice::from_ref(&f), &[0], b[0], b[1], 1e-3, 60_000).hi;
                if !(za.is_finite() && zb.is_finite()) {
                    continue;
                }
                let lift = rng.r(-0.02, 0.05);
                let mv = Move::Line {
                    a: [a[0], a[1], za + lift],
                    b: [b[0], b[1], zb + lift],
                };
                let mut runs = vec![
                    (1e-4, 40),
                    (1e-4, 400),
                    (1e-4, 4_000),
                    (1e-2, 4_000),
                    (1e-6, 60_000),
                ];
                // and each Clear search's own spend as its budget, and one less: the tightest
                // budgets it can and cannot end Clear within
                for (res, budget) in runs.clone() {
                    if let (Gap::Clear { .. }, n) = gap_move(&col, &f, &mv, -0.005, res, budget)
                        && n > 1
                    {
                        runs.push((res, n));
                        runs.push((res, n - 1));
                    }
                }
                for (res, budget) in runs {
                    let g = gap_move(&col, &f, &mv, -0.005, res, budget).0;
                    match g {
                        Gap::Clear { .. } => clear += 1,
                        Gap::Over { .. } => over += 1,
                        Gap::Unknown => unknown += 1,
                    }
                    assert_eq!(
                        gap_clear(&col, &f, &mv, -0.005, res, budget),
                        matches!(g, Gap::Clear { .. }),
                        "kind {kind} res {res} budget {budget}: {g:?}"
                    );
                }
            }
        }
    }
    println!("verdicts: {clear} clear, {over} over, {unknown} unknown");
    assert!(clear > 50 && over > 20 && unknown > 20);
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
    for k in 0..=12 {
        let rho = 3.5 * k as f64 / 20.0;
        // centre (0, 3) radius 3.5
        let want = 3.0 - (3.5f64 * 3.5 - rho * rho).sqrt();
        let b = g.b_at(rho);
        assert!(
            b.lo <= want + 1e-12 && b.hi >= want - 1e-12,
            "{rho}: {b:?} {want}"
        );
    }
    let (flat, _) = column(&Cutter::Flat { d: 6.0 });
    let g = flat.grow(1.0).unwrap();
    assert_eq!(g.r, 4.0);
    assert!(g.b_at(2.0).lo <= -1.0 && g.b_at(2.0).hi >= -1.0);
    let b = g.b_at(4.0);
    assert!(b.lo <= 0.0 && b.hi >= 0.0);
}

/// φ(d) of a horizontal cylinder of radius r (whole, its axis at height 0), sampled: the most of
/// √(r² − e²) − b(|u|) over e − u = d, u on a grid of [−R, R].
fn phi_sampled(col: &Column, r: f64, d: f64, n: usize) -> f64 {
    let mut m = f64::NEG_INFINITY;
    for i in 0..=n {
        let u = -col.r + 2.0 * col.r * i as f64 / n as f64;
        let e = d + u;
        if e.abs() > r {
            continue;
        }
        m = m.max((r * r - e * e).sqrt() - col.b_at(u.abs()).mid());
    }
    m
}

#[test]
fn a_dome_cylinders_chord_bound_is_its_sampled_maximum() {
    // the exact route (cyl_chord, Concave.lean): every move's bound holds the sampled maximum of
    // CL − z along it, and is within the sampling's error of it (exact, not a branch and bound)
    let mut rng = Rng(0x51ed_2701_aa55_9e11);
    let (mut ends, mut inner, mut n) = (0, 0, 0);
    for (name, c) in cutters() {
        let (col, _) = column(&c);
        assert!(col.convex(), "{name}");
        for _ in 0..40 {
            let a = unit([1.0, rng.r(-1.0, 1.0), 0.0]);
            let xx = unit(crs([0.0, 0.0, 1.0], a));
            let yy = crs(a, xx);
            let o = [rng.r(-2.0, 2.0), rng.r(-2.0, 2.0), rng.r(-3.0, 0.0)];
            let r = rng.r(1.0, 8.0);
            // the arc's upper half (y up or down: p's range covers the top either way)
            let f = Face::new(
                Surf::Cyl {
                    o,
                    x: xx,
                    y: yy,
                    ax: a,
                    r,
                },
                Ival::new(-1.0, 1.0),
                Ival::new(-8.0, 8.0),
                None,
                1e-9,
                0,
            );
            let lat = |p: [f64; 2]| {
                ((p[0] - o[0]) * a[1] - (p[1] - o[1]) * a[0]) / (a[0] * a[0] + a[1] * a[1]).sqrt()
            };
            let pa = [rng.r(-6.0, 6.0), rng.r(-6.0, 6.0)];
            let pb = [pa[0] + rng.r(-4.0, 4.0), pa[1] + rng.r(-4.0, 4.0)];
            let za = o[2] + phi_sampled(&col, r, lat(pa), 4000) + rng.r(-0.3, 0.6);
            let zb = o[2] + phi_sampled(&col, r, lat(pb), 4000) + rng.r(-0.3, 0.6);
            if !(za.is_finite() && zb.is_finite()) {
                continue;
            }
            let mv = Move::Line {
                a: [pa[0], pa[1], za],
                b: [pb[0], pb[1], zb],
            };
            let Some(bound) = forge_cam_geom::drop::cyl_chord(&col, &f, &mv) else {
                continue;
            };
            let mut m = f64::NEG_INFINITY;
            let mut arg = 0.0;
            for k in 0..=300 {
                let t = k as f64 / 300.0;
                let p = [pa[0] + t * (pb[0] - pa[0]), pa[1] + t * (pb[1] - pa[1])];
                let u = o[2] + phi_sampled(&col, r, lat(p), 600) - (za + t * (zb - za));
                if u > m {
                    m = u;
                    arg = t;
                }
            }
            n += 1;
            if arg == 0.0 || arg == 1.0 {
                ends += 1;
            } else {
                inner += 1;
            }
            assert!(bound >= m - 1e-9, "{name}: bound {bound} under the sampled {m}");
            assert!(
                bound <= m + 1e-3,
                "{name}: bound {bound} far over the sampled {m} (at {arg})"
            );
        }
    }
    println!("{n} moves: worst at an end {ends}, inside {inner}");
    assert!(ends > 20 && inner > 20);
}

#[test]
fn a_move_by_a_dome_cylinders_end_is_cleared_only_where_it_is() {
    // gap_move's exact route past a dome cylinder's end (the column's section for a ball or a
    // flat end; b(√·) superadditivity for a bull nose): moves along and across the axis near
    // the face's ends, at heights about the sampled CL; every Clear holds at each sampled face
    // point and instant, every Over is real
    let mut rng = Rng(0x0dd5_eed5_1234_9876);
    let (mut clear, mut over) = (0, 0);
    for (name, c) in cutters() {
        let (col, _) = column(&c);
        for _ in 0..80 {
            let a = unit([1.0, rng.r(-0.5, 0.5), 0.0]);
            let xx = unit(crs([0.0, 0.0, 1.0], a));
            let yy = crs(a, xx);
            let o = [rng.r(-1.0, 1.0), rng.r(-1.0, 1.0), rng.r(-2.0, 0.0)];
            let r = rng.r(1.0, 6.0);
            let half = rng.r(0.5, 3.0);
            let f = Face::new(
                Surf::Cyl {
                    o,
                    x: xx,
                    y: yy,
                    ax: a,
                    r,
                },
                Ival::new(-1.0, 1.0),
                Ival::new(-half, half),
                None,
                1e-9,
                0,
            );
            // near an end: along the axis (with a little across), or across it
            let n = [-a[1], a[0]];
            let end = if rng.f() < 0.5 { half } else { -half };
            let t0 = end + rng.r(-1.5, 1.5) * col.r;
            let l0 = rng.r(-(r + col.r), r + col.r);
            let along = rng.f() < 0.6;
            let (dt, dl) = if along {
                (rng.r(-2.0, 2.0), rng.r(-0.05, 0.05))
            } else {
                (rng.r(-0.3, 0.3), rng.r(-3.0, 3.0))
            };
            let pa = [o[0] + t0 * a[0] + l0 * n[0], o[1] + t0 * a[1] + l0 * n[1]];
            let pb = [pa[0] + dt * a[0] + dl * n[0], pa[1] + dt * a[1] + dl * n[1]];
            let za = sampled(&col, &f, pa[0], pa[1], 40);
            let zb = sampled(&col, &f, pb[0], pb[1], 40);
            if !(za.is_finite() && zb.is_finite()) {
                continue;
            }
            let lift = rng.r(-0.02, 0.1);
            let mv = Move::Line {
                a: [pa[0], pa[1], za + lift],
                b: [pb[0], pb[1], zb + lift],
            };
            // (a tangent move may stay unknown: a small budget)
            let (g, _) = gap_move(&col, &f, &mv, -0.005, 1e-4, 4_000);
            match g {
                Gap::Clear { lower } => {
                    clear += 1;
                    for k in 0..=10 {
                        let t = k as f64 / 10.0;
                        let (x, y) = (pa[0] + t * (pb[0] - pa[0]), pa[1] + t * (pb[1] - pa[1]));
                        let z = za + lift + t * (zb - za);
                        let v = sampled_gap(&col, &f, x, y, z, 24);
                        assert!(v >= lower - 1e-9, "{name}: v {v} < {lower} at {t} on {mv:?}");
                    }
                }
                Gap::Over { s, by, .. } => {
                    over += 1;
                    let (x, y) = (pa[0] + s * (pb[0] - pa[0]), pa[1] + s * (pb[1] - pa[1]));
                    let z = za + lift + s * (zb - za);
                    let cl = drop(&col, std::slice::from_ref(&f), &[0], x, y, 1e-5, 60_000);
                    assert!(
                        cl.hi - z >= by - 1e-5,
                        "{name}: over by {by} but CL {cl:?} at z {z}"
                    );
                }
                Gap::Unknown => {}
            }
        }
    }
    println!("moves: {clear} clear, {over} over");
    assert!(clear > 60 && over > 10);
}

/// `sampled_gap` over the face's trimmed part only (points its trim test proves inside).
fn sampled_gap_in(col: &Column, f: &Face, x: f64, y: f64, z: f64, n: usize) -> f64 {
    let mut m = f64::INFINITY;
    for i in 0..=n {
        for j in 0..=n {
            let p = f.p.lo + (f.p.hi - f.p.lo) * i as f64 / n as f64;
            let q = f.q.lo + (f.q.hi - f.q.lo) * j as f64 / n as f64;
            if f.holds(Ival::pt(p), Ival::pt(q)) != 1 {
                continue;
            }
            let pt = f.surf.eval(Ival::pt(p), Ival::pt(q));
            let (px, py, pz) = (pt[0].mid(), pt[1].mid(), pt[2].mid());
            let rho = ((px - x).ipow(2) + (py - y).ipow(2)).sqrt();
            if rho > col.r {
                continue;
            }
            m = m.min(col.solid.sd_rz(Ival::pt(rho), Ival::pt(pz - z)).mid());
        }
    }
    m
}

/// gap_move's quick routes: a level flat passed beside (its region's distance), a vertical
/// plane with an L-shaped trim (the wall over its box), and a face under the move (its height):
/// every Clear holds at every sampled face point and instant, and the quick routes do decide
/// most moves (one cell or none).
#[test]
fn quick_routes_clear_only_what_sampling_holds() {
    use forge_cam_geom::check::{TSeg, Trim};
    let mut rng = Rng(0x7a11_f1a7_0bad_cafe);
    let (mut clear, mut quick) = (0, 0);
    for (name, c) in cutters() {
        let (col, _) = column(&c);
        for k in 0..30 {
            let rr = rng.r(3.0, 10.0);
            let (cx, cy, zf) = (rng.r(-2.0, 2.0), rng.r(-2.0, 2.0), rng.r(-2.0, 2.0));
            // a disk flat (four ccw quarter arcs), or an L-shaped vertical wall in x-z
            let f = if k % 2 == 0 {
                let q = [[cx + rr, cy], [cx, cy + rr], [cx - rr, cy], [cx, cy - rr]];
                let segs = (0..4)
                    .map(|i| TSeg::Arc {
                        a: [q[i][0] - cx + rr, q[i][1] - cy + rr],
                        b: [q[(i + 1) % 4][0] - cx + rr, q[(i + 1) % 4][1] - cy + rr],
                        c: [rr, rr],
                        r: rr,
                        ccw: true,
                    })
                    .collect();
                Face::new(
                    Surf::Plane {
                        o: [cx - rr, cy - rr, zf],
                        u: [1.0, 0.0, 0.0],
                        v: [0.0, 1.0, 0.0],
                    },
                    Ival::new(0.0, 2.0 * rr),
                    Ival::new(0.0, 2.0 * rr),
                    Some(Trim { segs }),
                    1e-9,
                    0,
                )
            } else {
                let (w, h) = (2.0 * rr, rr);
                let l = [
                    [0.0, 0.0],
                    [w, 0.0],
                    [w, 0.5 * h],
                    [0.5 * w, 0.5 * h],
                    [0.5 * w, h],
                    [0.0, h],
                ];
                let segs = (0..6)
                    .map(|i| TSeg::Line {
                        a: l[i],
                        b: l[(i + 1) % 6],
                    })
                    .collect();
                Face::new(
                    Surf::Plane {
                        o: [cx - rr, cy, zf - 0.5 * h],
                        u: [1.0, 0.0, 0.0],
                        v: [0.0, 0.0, 1.0],
                    },
                    Ival::new(0.0, w),
                    Ival::new(0.0, h),
                    Some(Trim { segs }),
                    1e-9,
                    0,
                )
            };
            assert!(
                k % 2 == 0 || f.prism.is_none(),
                "the L-shaped wall has no prism of its own"
            );
            // moves beside or over it, at heights about its own
            // a direction by the rational map (no platform trig)
            let t = rng.r(-3.0, 3.0);
            let (dc, ds) = ((1.0 - t * t) / (1.0 + t * t), 2.0 * t / (1.0 + t * t));
            // (some from well inside its region: a flat passed over, not beside)
            let d = if rng.f() < 0.3 {
                rng.r(0.0, 0.3 * rr)
            } else {
                rr + rng.r(-1.0, 1.5) * col.r
            };
            let (sx, sy) = (cx + d * dc, cy + d * ds);
            let e = [sx + rng.r(-3.0, 3.0), sy + rng.r(-3.0, 3.0)];
            let (za, zb) = (zf + rng.r(-2.0, 3.0), zf + rng.r(-2.0, 3.0));
            let mv = Move::Line {
                a: [sx, sy, za],
                b: [e[0], e[1], zb],
            };
            let (g, cells) = gap_move(&col, &f, &mv, -0.005, 1e-4, 4_000);
            if let Gap::Clear { lower } = g {
                clear += 1;
                if cells <= 1 {
                    quick += 1;
                }
                for s in 0..=10 {
                    let t = s as f64 / 10.0;
                    let (x, y) = (sx + t * (e[0] - sx), sy + t * (e[1] - sy));
                    let z = za + t * (zb - za);
                    let v = sampled_gap_in(&col, &f, x, y, z, 40);
                    assert!(v >= lower - 1e-9, "{name} {k}: v {v} < {lower} at {t} on {mv:?}");
                }
            }
        }
    }
    println!("moves: {clear} clear, {quick} by a quick route");
    assert!(clear > 40 && quick > 20);
}

/// Mutation for the dome cylinder's exact route (`cyl_chord`): a move at the cylinder's CL near
/// its top, lowered 0.05 into it (a real violation at its ends, the contact's normal within 60°
/// of up), and a chord across the top with its ends on the CL (a violation in its middle), are
/// never cleared at −0.005.
#[test]
fn a_move_pushed_into_a_dome_cylinder_is_not_cleared() {
    let mut rng = Rng(0xbad0_c11d_0000_0001);
    let mut n = 0;
    for (name, c) in cutters() {
        let (col, _) = column(&c);
        for _ in 0..30 {
            let a = unit([1.0, rng.r(-1.0, 1.0), 0.0]);
            let up = [0.0, 0.0, 1.0];
            let yy = crs(a, up);
            let o = [rng.r(-2.0, 2.0), rng.r(-2.0, 2.0), rng.r(-3.0, 0.0)];
            let r = rng.r(2.0, 8.0);
            // the upper half (x up: a quarter turn either way)
            let f = Face::new(
                Surf::Cyl {
                    o,
                    x: up,
                    y: yy,
                    ax: a,
                    r,
                },
                Ival::new(-1.0, 1.0),
                Ival::new(-8.0, 8.0),
                None,
                1e-9,
                0,
            );
            let lat = |p: [f64; 2]| {
                ((p[0] - o[0]) * a[1] - (p[1] - o[1]) * a[0]) / (a[0] * a[0] + a[1] * a[1]).sqrt()
            };
            let n2 = [a[1], -a[0]];
            let at = |d: f64, t: f64| [o[0] + d * n2[0] + t * a[0], o[1] + d * n2[1] + t * a[1]];
            let (pa, pb) = (
                at(rng.r(-0.4, 0.4) * r, rng.r(-3.0, 3.0)),
                at(rng.r(-0.4, 0.4) * r, rng.r(-3.0, 3.0)),
            );
            let za = o[2] + phi_sampled(&col, r, lat(pa), 2000) - 0.05;
            let zb = o[2] + phi_sampled(&col, r, lat(pb), 2000) - 0.05;
            let mv = Move::Line {
                a: [pa[0], pa[1], za],
                b: [pb[0], pb[1], zb],
            };
            let (g, _) = gap_move(&col, &f, &mv, -0.005, 1e-4, 20_000);
            assert!(
                !matches!(g, Gap::Clear { .. }),
                "{name}: a move 0.05 into the cylinder cleared: {mv:?}"
            );
            // and a chord across the top, its ends on the CL, its middle sagging into it (the
            // worst point inside the move: d*'s place decides it)
            // (rising: from low on one side to just past the top, so λ > 0 and the worst point
            // lies inside, toward the low end)
            let dq = col.r + r;
            let (qa, qb) = (at(-0.9 * dq, rng.r(-3.0, 3.0)), at(0.1 * dq, rng.r(-3.0, 3.0)));
            let mv = Move::Line {
                a: [qa[0], qa[1], o[2] + phi_sampled(&col, r, lat(qa), 2000)],
                b: [qb[0], qb[1], o[2] + phi_sampled(&col, r, lat(qb), 2000)],
            };
            let (g, _) = gap_move(&col, &f, &mv, -0.005, 1e-4, 20_000);
            assert!(
                !matches!(g, Gap::Clear { .. }),
                "{name}: a chord across the top cleared: {mv:?}"
            );
            n += 1;
        }
    }
    assert!(n >= 100);
}

/// Mutation for the quick routes: a move over a disk flat's inside and under it, or through an
/// L-shaped wall's lower part, is never cleared.
#[test]
fn a_move_into_a_flat_or_an_l_wall_is_not_cleared() {
    use forge_cam_geom::check::{TSeg, Trim};
    let mut rng = Rng(0xbad0_f1a7_0000_0002);
    for (name, c) in cutters() {
        let (col, _) = column(&c);
        for _ in 0..20 {
            let rr = rng.r(6.0, 10.0);
            let zf = rng.r(-1.0, 1.0);
            let q = [[2.0 * rr, rr], [rr, 2.0 * rr], [0.0, rr], [rr, 0.0]];
            let segs = (0..4)
                .map(|i| TSeg::Arc {
                    a: q[i],
                    b: q[(i + 1) % 4],
                    c: [rr, rr],
                    r: rr,
                    ccw: true,
                })
                .collect();
            let disk = Face::new(
                Surf::Plane {
                    o: [-rr, -rr, zf],
                    u: [1.0, 0.0, 0.0],
                    v: [0.0, 1.0, 0.0],
                },
                Ival::new(0.0, 2.0 * rr),
                Ival::new(0.0, 2.0 * rr),
                Some(Trim { segs }),
                1e-9,
                0,
            );
            // from near the centre, the tip 0.1 under the flat
            let s = [rng.r(-1.0, 1.0), rng.r(-1.0, 1.0)];
            let e = [s[0] + rng.r(-2.0, 2.0), s[1] + rng.r(-2.0, 2.0)];
            let z = zf - 0.1;
            let mv = Move::Line {
                a: [s[0], s[1], z],
                b: [e[0], e[1], z],
            };
            let (g, _) = gap_move(&col, &disk, &mv, -0.005, 1e-4, 20_000);
            assert!(
                !matches!(g, Gap::Clear { .. }),
                "{name}: a move under the flat cleared: {mv:?}"
            );
            // an L-shaped wall in x-z (its lower half full width), the move across it low down
            let (w, h) = (2.0 * rr, rr);
            let l = [
                [0.0, 0.0],
                [w, 0.0],
                [w, 0.5 * h],
                [0.5 * w, 0.5 * h],
                [0.5 * w, h],
                [0.0, h],
            ];
            let lsegs = (0..6)
                .map(|i| TSeg::Line {
                    a: l[i],
                    b: l[(i + 1) % 6],
                })
                .collect();
            let wall = Face::new(
                Surf::Plane {
                    o: [-rr, 0.0, zf - 0.5 * h],
                    u: [1.0, 0.0, 0.0],
                    v: [0.0, 0.0, 1.0],
                },
                Ival::new(0.0, w),
                Ival::new(0.0, h),
                Some(Trim { segs: lsegs }),
                1e-9,
                0,
            );
            let x = rng.r(-0.8, 0.8) * rr;
            let zl = zf - 0.5 * h + rng.r(0.1, 0.3) * h;
            let mv = Move::Line {
                a: [x, -3.0, zl],
                b: [x, 3.0, zl],
            };
            let (g, _) = gap_move(&col, &wall, &mv, -0.005, 1e-4, 20_000);
            assert!(
                !matches!(g, Gap::Clear { .. }),
                "{name}: a move through the wall cleared: {mv:?}"
            );
        }
    }
}

/// `drop_decide` (the push-cutter's probe) against the full drop: a decided answer is the full
/// drop's (decided free: the full lower bound is not above z; decided blocked: the full upper
/// bound is above z), over random faces, places and heights about the CL.
#[test]
fn drop_decide_agrees_with_the_full_drop() {
    let mut rng = Rng(0xdec1_de00_0000_0003);
    let (mut free, mut blocked) = (0, 0);
    for (name, c) in cutters() {
        let (col, _) = column(&c);
        for kind in 0..7 {
            for _ in 0..4 {
                let f = face(&mut rng, kind);
                let (x, y) = (rng.r(-4.0, 4.0), rng.r(-4.0, 4.0));
                let full = drop(&col, std::slice::from_ref(&f), &[0], x, y, 1e-4, 60_000);
                if !full.hi.is_finite() {
                    continue;
                }
                let z = full.hi + rng.r(-0.2, 0.2);
                let d = drop_decide(
                    &col,
                    std::slice::from_ref(&f),
                    &[0],
                    x,
                    y,
                    z,
                    f64::NEG_INFINITY,
                    1e-4,
                    60_000,
                );
                if d.hi <= z {
                    free += 1;
                    assert!(
                        full.lo <= z,
                        "{name} kind {kind}: decided free, the drop says {full:?} > {z}"
                    );
                } else if d.lo > z {
                    blocked += 1;
                    assert!(
                        full.hi > z,
                        "{name} kind {kind}: decided blocked, the drop says {full:?} ≤ {z}"
                    );
                }
            }
        }
    }
    println!("decided: {free} free, {blocked} blocked");
    assert!(free > 20 && blocked > 20);
}

/// Mutation for `drop_decide`'s early stop: far from the CL the answer must be decided, and
/// rightly (0.5 above the full drop: free; 0.5 under it: blocked). A stop that fires on the
/// wrong side (`lo > stop` turned round) leaves these undecided or wrong, and fails here.
#[test]
fn drop_decide_decides_far_from_the_cl() {
    let mut rng = Rng(0xdec1_de00_0000_0004);
    let mut n = 0;
    for (name, c) in cutters() {
        let (col, _) = column(&c);
        for kind in 0..7 {
            for _ in 0..3 {
                let f = face(&mut rng, kind);
                let (x, y) = (rng.r(-3.0, 3.0), rng.r(-3.0, 3.0));
                let full = drop(&col, std::slice::from_ref(&f), &[0], x, y, 1e-4, 60_000);
                if !(full.hi.is_finite() && full.hi - full.lo <= 1e-3) {
                    continue;
                }
                let one = std::slice::from_ref(&f);
                let above = drop_decide(
                    &col,
                    one,
                    &[0],
                    x,
                    y,
                    full.hi + 0.5,
                    f64::NEG_INFINITY,
                    1e-4,
                    60_000,
                );
                assert!(
                    above.hi <= full.hi + 0.5,
                    "{name} kind {kind}: 0.5 over the CL not decided free: {above:?} vs {full:?}"
                );
                let below = drop_decide(
                    &col,
                    one,
                    &[0],
                    x,
                    y,
                    full.lo - 0.5,
                    f64::NEG_INFINITY,
                    1e-4,
                    60_000,
                );
                assert!(
                    below.lo > full.lo - 0.5,
                    "{name} kind {kind}: 0.5 under the CL not decided blocked: {below:?} vs {full:?}"
                );
                n += 1;
            }
        }
    }
    assert!(n > 40, "{n}");
}
