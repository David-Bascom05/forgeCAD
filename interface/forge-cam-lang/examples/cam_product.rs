//! The product path's certification cost: the go/no-go's contours (cam_gonogo.rs) as op paths,
//! certified by ops::certify_fixtured, so nothing outside the product is measured.
//!
//! cargo run --release -p forge-cam-lang --example cam_product [N]
use forge_cam::ops::{self, OpPath, PMove, Phase, ToolUse};
use forge_cam_geom::check::Face;
use forge_cam_geom::offset::offset;
use forge_cam_geom::region::{Loop, Region, Seg, p2};
use forge_cam_geom::tool::{Cutter, Holder, assemble};
use forge_tooling::Library;

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

/// A loop as one op path at height z, each line cut into k pieces and each arc into 4k.
fn path(segs: &[Seg], z: f64, k: usize) -> OpPath {
    let mut moves = Vec::new();
    let s0 = match segs[0] {
        Seg::Line { a, .. } | Seg::Arc { a, .. } => a,
    };
    for s in segs {
        match *s {
            Seg::Line { a, b } => {
                for i in 0..k {
                    let t = (i + 1) as f64 / k as f64;
                    moves.push(PMove::Line {
                        to: [a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t, z],
                        f: 1000.0,
                        phase: Phase::Cut,
                    });
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
                    let u = t0 + (t1 - t0) * (i + 1) as f64 / n as f64;
                    let (s, co) = forge_math::dmath::sin_cos(u);
                    moves.push(PMove::Arc {
                        to: [c.x + r * co, c.y + r * s, z],
                        c: [c.x, c.y],
                        cw: !ccw,
                        f: 1000.0,
                    });
                }
            }
        }
    }
    OpPath {
        tool: 0,
        start: [s0.x, s0.y, z],
        moves,
        rpm: 8000.0,
        coolant: forge_post::Coolant::Flood,
    }
}

fn main() {
    let src = std::fs::read_to_string("examples/machined/flange.fg").unwrap();
    let (run, _) = forge_lang::run(&src, forge_lang::Options::default(), Default::default());
    let body = run.body.expect("flange builds");
    let faces: Vec<Face> = forge_cam::lower::lower(&body).expect("lowers");
    let holder = Holder {
        profile: vec![[0.0, 25.0], [20.0, 25.0], [25.0, 40.0], [60.0, 40.0]],
    };
    let lib = Library::builtin();
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
    let n: usize = std::env::args()
        .nth(1)
        .map(|a| a.parse().unwrap())
        .unwrap_or(5000);
    let k = (n / 28).max(1);
    let paths = [
        path(&plate[0].segs, 2.0, k),
        path(&boss[0].segs, 13.0, k),
        path(&bore[0].segs, 1.0, k),
    ];
    for (tname, c) in [
        ("flat 6", Cutter::Flat { d: 6.0 }),
        ("bull 6 r1", Cutter::Bull { d: 6.0, re: 1.0 }),
    ] {
        let tu = ToolUse {
            t: 1,
            tool: lib.tool("flat-6").unwrap().clone(),
            holder: None,
            stickout: 0.0,
            shape: assemble(&c, 25.0, 6.0, 35.0, Some(&holder)).unwrap(),
        };
        let t0 = std::time::Instant::now();
        let (mut mv, mut pairs, mut cells, mut bad) = (0, 0, 0, 0);
        for p in &paths {
            mv += ops::checker_moves(p).len();
            let s = ops::certify_fixtured(p, &tu, &faces, &[(0, Vec::new())], &[], 0.0, 0.01, 0.1);
            pairs += s.pairs;
            cells += s.cells;
            bad += s.checks.len();
        }
        let dt = t0.elapsed().as_secs_f64();
        println!(
            "{tname}: {mv} checker moves, {pairs} pairs, {cells} cells, {bad} checks reported, {dt:.3} s"
        );
    }
}
