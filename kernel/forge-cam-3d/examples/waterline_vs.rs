//! CAM-3AXIS's surf::waterline against this crate's waterline (through SurfDrop) on the mould
//! core: the same heights, tolerance and tool; each path certified by certify_with (column
//! route). Prints generation and certification time, moves, and the worst xy distance of each
//! path's vertices outside the true contour (from the drop: CL there must be ≤ z, and a point
//! a little further in must be blocked).
use forge_cam::ops::{self, Cut, Heights, ToolUse};
use forge_cam::surf::{self, Surfacer};
use forge_cam_3d::drop::DropCutter;
use forge_cam_3d::link;
use forge_cam_3d::path::Emit;
use forge_cam_3d::surfdrop::SurfDrop;
use forge_cam_3d::waterline;
use forge_tooling::Library;
use std::time::Instant;

fn main() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined/mold_core.fg");
    let src = std::fs::read_to_string(p).unwrap();
    let (run, _) = forge_lang::run(&src, forge_lang::Options::default(), Default::default());
    let faces = forge_cam::lower3d::lower_surfacing(&run.body.unwrap()).unwrap().0;
    let l = Library::builtin();
    let t = l.tool("ball-6").unwrap().clone();
    let shape = t.shape(40.0, Some(l.holder("bt40-er32").unwrap())).unwrap();
    let tool = ToolUse {
        t: 2,
        tool: t,
        holder: Some("bt40-er32".into()),
        stickout: 40.0,
        shape: shape.clone(),
    };
    let sf = Surfacer::new(&faces, &shape.bodies[0], 0.0, 0.01, 10.0).unwrap();
    let dc = SurfDrop::new(&sf).unwrap();
    let h = Heights {
        clear: 50.0,
        retract: 40.0,
    };
    let zs = [22.0, 18.0, 14.0];
    let area = [-30.0, -24.0, 30.0, 24.0];
    let stages = [(0usize, Vec::new())];

    // theirs
    let t0 = Instant::now();
    let c = Cut {
        tool: &tool,
        tidx: 0,
        h,
        feed: (1500.0, 500.0),
    };
    let theirs = surf::waterline(&sf, c, area, &zs, 0.5).unwrap();
    let g1 = t0.elapsed().as_secs_f64();
    let t0 = Instant::now();
    let s1 = ops::certify_with(&theirs, &tool, &faces, &stages, &[], 0.0, 0.01, 1.0, true);
    let c1 = t0.elapsed().as_secs_f64();
    println!(
        "CAM-3AXIS waterline: {} moves, generated {g1:.2} s, certified {c1:.2} s, clear {}",
        theirs.moves.len(),
        s1.clear
    );

    // mine: the same heights, grid 0.5, tol_xy = tol / 2, linked
    let t0 = Instant::now();
    let mut passes = Vec::new();
    for &z in &zs {
        for lp in waterline::level(
            &dc,
            z,
            waterline::Params {
                grid: 0.5,
                tol_xy: 0.005,
            },
        )
        .unwrap()
        {
            let mut v: Vec<[f64; 3]> = lp.pts.iter().map(|q| [q[0], q[1], lp.z]).collect();
            v.push(v[0]);
            passes.push(v);
        }
    }
    let passes = link::order(vec![passes], [0.0, 0.0, 50.0], false);
    let e = Emit {
        tool: 0,
        h,
        feed: (1500.0, 500.0),
        rpm: 12000.0,
        coolant: forge_post::Coolant::Flood,
    };
    let (mine, count) = link::path(
        &dc,
        &passes,
        link::Rules {
            max_stay: 2.0,
            lift: 0.5,
            slack: 0.0,
        },
        e,
    );
    let g2 = t0.elapsed().as_secs_f64();
    let t0 = Instant::now();
    let s2 = ops::certify_with(&mine, &tool, &faces, &stages, &[], 0.0, 0.01, 1.0, true);
    let c2 = t0.elapsed().as_secs_f64();
    println!(
        "CAM-3D-FINISH waterline: {} moves, links {count:?}, generated {g2:.2} s, certified {c2:.2} s, clear {}",
        mine.moves.len(),
        s2.clear
    );

    // how far out of the contour each path's cutting vertices are: at each vertex at height z,
    // walk towards the part (decreasing free) in 1 µm steps up to 0.1 mm until the CL exceeds z
    let gap_of = |p: &ops::OpPath| {
        let mut worst: f64 = 0.0;
        let mut n = 0;
        for m in &p.moves {
            if let ops::PMove::Line {
                to,
                phase: ops::Phase::Cut,
                ..
            } = m
            {
                if !zs.iter().any(|z| (to[2] - z).abs() < 1e-9) {
                    continue;
                }
                n += 1;
                // the nearest blocked point within 0.2 mm, on 64 directions
                let mut best = f64::INFINITY;
                for k in 0..64 {
                    let (s, c) = forge_math::dmath::sin_cos(k as f64 * std::f64::consts::TAU / 64.0);
                    let mut r = 0.0;
                    while r < 0.2 {
                        r += 0.002;
                        if dc
                            .drop(to[0] + r * c, to[1] + r * s)
                            .is_some_and(|b| b.lo > to[2])
                        {
                            best = best.min(r);
                            break;
                        }
                    }
                }
                if best.is_finite() {
                    worst = worst.max(best);
                }
            }
        }
        (worst, n)
    };
    println!(
        "worst xy gap to the contour (to 2 µm): theirs {:?}, mine {:?}",
        gap_of(&theirs),
        gap_of(&mine)
    );
}
