//! CAM Phase 0 go/no-go: the certified clearance checker's cost on a real part.
//!
//! cargo run --release -p forge-lang --example cam_gonogo [N...]
//!
//! examples/machined/flange.fg, built and lowered; three finishing contours made by the exact
//! offset (the plate's outline outside, the boss outside, the bore inside), every move subdivided
//! until the op has N moves; each move certified for the cutter (limit −tol) and the shank and
//! holder (limit +clearance) against every face whose box is near.
use forge_cam_geom::check::{Face, Move, Verdict, apart, swept_box};
use forge_cam_geom::offset::offset;
use forge_cam_geom::region::{Loop, Region, Seg, p2};
use forge_cam_geom::tool::{Cutter, Holder, assemble};

fn circle(r: f64, ccw: bool) -> Loop {
    let a = p2(r, 0.0);
    Loop {
        segs: vec![Seg::Arc {
            a,
            b: a,
            c: p2(0.0, 0.0),
            ccw,
        }],
    }
}
fn square(h: f64) -> Loop {
    let (a, b, c, d) = (p2(-h, -h), p2(h, -h), p2(h, h), p2(-h, h));
    Loop {
        segs: vec![
            Seg::Line { a, b },
            Seg::Line { a: b, b: c },
            Seg::Line { a: c, b: d },
            Seg::Line { a: d, b: a },
        ],
    }
}

/// A loop's segments as moves at height z, each line and arc cut into `k` pieces (arcs first
/// into pieces of under a half turn).
fn moves(segs: &[Seg], z: f64, k: usize, out: &mut Vec<Move>) {
    for s in segs {
        match *s {
            Seg::Line { a, b } => {
                for i in 0..k {
                    let (t0, t1) = (i as f64 / k as f64, (i + 1) as f64 / k as f64);
                    let p = |t: f64| [a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t, z];
                    out.push(Move::Line { a: p(t0), b: p(t1) });
                }
            }
            Seg::Arc { a, b, c, ccw } => {
                let r = ((a.x - c.x).powi(2) + (a.y - c.y).powi(2)).sqrt();
                let ang = |p: forge_cam_geom::region::P2| forge_math::dmath::atan2(p.y - c.y, p.x - c.x);
                let t0 = ang(a);
                let mut t1 = ang(b);
                let tau = std::f64::consts::TAU;
                if ccw {
                    while t1 <= t0 {
                        t1 += tau;
                    }
                } else {
                    while t1 >= t0 {
                        t1 -= tau;
                    }
                }
                let n = 4 * k;
                for i in 0..n {
                    let (u0, u1) = (
                        t0 + (t1 - t0) * i as f64 / n as f64,
                        t0 + (t1 - t0) * (i + 1) as f64 / n as f64,
                    );
                    let um = 0.5 * (u0 + u1);
                    let (sm, cm) = forge_math::dmath::sin_cos(um);
                    let h = 0.5 * (u1 - u0);
                    let th = forge_math::dmath::sin(0.5 * h) / forge_math::dmath::cos(0.5 * h);
                    out.push(Move::Arc {
                        c: [c.x, c.y, z],
                        x: [cm, sm, 0.0],
                        y: [-sm, cm, 0.0],
                        r,
                        t0: -th,
                        t1: th,
                    });
                }
            }
        }
    }
}

fn main() {
    let src = std::fs::read_to_string("examples/machined/flange.fg").unwrap();
    let (run, _) = forge_lang::run(&src, forge_lang::Options::default(), Default::default());
    let body = run.body.expect("flange builds");
    let faces: Vec<Face> = forge_cam::lower::lower(&body).expect("lowers");
    println!(
        "flange: {} B-rep faces, {} checker faces",
        body.face_ids().count(),
        faces.len()
    );
    if std::env::var("DUMP").is_ok() {
        for f in &faces {
            if let Some(forge_cam_geom::slab::Prism::Flat { region, .. }) = &f.prism {
                let tr = forge_cam_geom::check::Trim { segs: region.clone() };
                use forge_cam_geom::ival::Ival;
                println!(
                    "classify face {} (7, 0.0196): {} ; (0,0): {}",
                    f.id,
                    tr.classify(Ival::pt(6.99997246585088), Ival::pt(0.01963357658604995)),
                    tr.classify(Ival::pt(0.0), Ival::pt(0.0))
                );
                let m = Move::Arc {
                    c: [0.0, 0.0, 1.0],
                    x: [0.9997482349425065, 0.022438064295804937, 0.0],
                    y: [-0.022438064295804937, 0.9997482349425065, 0.0],
                    r: 7.0,
                    t0: -0.011220444605843115,
                    t1: 0.011220444605843115,
                };
                if let Some(forge_cam_geom::check::Planar::Arc { c, r, a, b, ccw, .. }) = m.planar() {
                    let ps = forge_cam_geom::slab::Sec::Arc { c, r, a, b, ccw };
                    println!("  path {ps:?}");
                    for sg in region {
                        let sc = match *sg {
                            forge_cam_geom::check::TSeg::Line { a, b } => {
                                forge_cam_geom::slab::Sec::Seg { a, b }
                            }
                            forge_cam_geom::check::TSeg::Arc { a, b, c, r, ccw } => {
                                forge_cam_geom::slab::Sec::Arc { c, r, a, b, ccw }
                            }
                        };
                        let d = forge_cam_geom::slab::dist(&ps, &sc);
                        if d.hi < 1.0 {
                            println!("   ZERO-ish {d:?} vs {sg:?}");
                        }
                    }
                }
            }
        }
        for f in &faces {
            println!(
                "face {} {:?} p {:?} q {:?} prism {:?}",
                f.id,
                f.surf,
                f.p,
                f.q,
                f.prism.as_ref().map(|p| match p {
                    forge_cam_geom::slab::Prism::Flat { z, region, .. } =>
                        format!("flat z {z} {} segs", region.len()),
                    w => format!("{w:?}"),
                })
            );
        }
    }
    let holder = Holder {
        profile: vec![[0.0, 25.0], [20.0, 25.0], [25.0, 40.0], [60.0, 40.0]],
    };
    let tool = assemble(&Cutter::Flat { d: 6.0 }, 25.0, 6.0, 35.0, Some(&holder)).unwrap();
    let bull = assemble(&Cutter::Bull { d: 6.0, re: 1.0 }, 25.0, 6.0, 35.0, Some(&holder)).unwrap();
    let r = 3.0;
    let plate = offset(
        &Region {
            loops: vec![square(40.0)],
        },
        -r,
    )
    .unwrap()
    .0;
    let boss = offset(
        &Region {
            loops: vec![circle(20.0, true)],
        },
        -r,
    )
    .unwrap()
    .0;
    let bore = offset(
        &Region {
            loops: vec![circle(10.0, true)],
        },
        r,
    )
    .unwrap()
    .0;
    let ns: Vec<usize> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let ns = if ns.is_empty() {
        vec![1_000, 10_000, 100_000]
    } else {
        ns
    };
    let (tol, clear): (f64, f64) = (0.01, 0.1);
    for (tname, t) in [("flat 6", &tool), ("bull 6 r1", &bull)] {
        for &n in &ns {
            // base moves: plate 8 (4 lines + 4 arcs), boss 1 arc, bore 1 arc → ~ k * (4 + 4*4 + 4 + 4)
            let k = (n / 28).max(1);
            let mut mv = Vec::new();
            moves(&plate[0].segs, 2.0, k, &mut mv);
            moves(&boss[0].segs, 13.0, k, &mut mv);
            moves(&bore[0].segs, 1.0, k, &mut mv);
            let grid = forge_cam_geom::check::Grid::new(&faces, clear + 1e-6);
            let t0 = std::time::Instant::now();
            // the moves are independent: chunks on all cores (forge_math::par, FORGECAD_THREADS)
            const CHUNK: usize = 64;
            let parts = forge_math::par::map(mv.len().div_ceil(CHUNK), |c| {
                let mut cand: Vec<u32> = Vec::new();
                let mut seen = forge_cam_geom::check::Seen::default();
                let (mut cells, mut pairs, mut near, mut bad) = (0usize, 0usize, 0usize, 0usize);
                let mut lowest = f64::INFINITY;
                for m in &mv[c * CHUNK..((c + 1) * CHUNK).min(mv.len())] {
                    let sb = swept_box(m, t.rmax(), t.top());
                    grid.near(&sb, &mut cand, &mut seen);
                    for &fi in &cand {
                        let f = &faces[fi as usize];
                        pairs += 1;
                        if apart(&sb, &f.bbox, clear.max(0.0) + f.mu) {
                            continue;
                        }
                        let mut shared = None;
                        for b in &t.bodies {
                            let lim = if b.part == forge_cam_geom::tool::Part::Cutter { -tol } else { clear };
                            if apart(&forge_cam_geom::check::swept_box_body(m, b), &f.bbox, lim.max(0.0) + f.mu) {
                                continue;
                            }
                            near += 1;
                            let (v, c) = forge_cam_geom::check::certify_shared(b, f, m, lim, tol / 4.0, 200_000, &mut shared);
                            cells += c;
                            match v {
                                Verdict::Clear { lower } => lowest = lowest.min(lower),
                                _ => bad += 1,
                            }
                        }
                    }
                }
                (cells, pairs, near, bad, lowest)
            });
            let (mut cells, mut pairs, mut near, mut bad) = (0usize, 0usize, 0usize, 0usize);
            let mut lowest = f64::INFINITY;
            for (c, p, n, b, l) in parts {
                cells += c;
                pairs += p;
                near += n;
                bad += b;
                lowest = lowest.min(l);
            }
            let _ = lowest;
            let dt = t0.elapsed().as_secs_f64();
            println!(
                "{tname}: {} moves, {pairs} move-face pairs, {near} near body-face checks, {cells} cells ({:.2}/move), {bad} not clear, {:.3} s ({:.1} us/move)",
                mv.len(),
                cells as f64 / mv.len() as f64,
                dt,
                1e6 * dt / mv.len() as f64
            );
        }
    }
}
