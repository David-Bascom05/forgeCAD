//! 3-axis surfacing on a dome: a sphere cap (r 20, centre 10 below the plate) on a 60 x 60 plate.
//! Parallel finishing with a 6 mm ball, every move certified (column route).
use forge_cam::ops::{self, Cut, Heights, ToolUse};
use forge_cam::surf::{self, Surfacer};
use forge_cam_geom::check::{Face, Surf, TSeg, Trim};
use forge_cam_geom::ival::Ival;
use forge_tooling::Library;

pub fn dome() -> Vec<Face> {
    let (cx, cy) = (30.0, 30.0);
    let rr = (400.0f64 - 100.0).sqrt();
    // the plate's top with the cap's footprint taken out (four clockwise quarter arcs)
    let mut segs = vec![
        TSeg::Line {
            a: [0.0, 0.0],
            b: [60.0, 0.0],
        },
        TSeg::Line {
            a: [60.0, 0.0],
            b: [60.0, 60.0],
        },
        TSeg::Line {
            a: [60.0, 60.0],
            b: [0.0, 60.0],
        },
        TSeg::Line {
            a: [0.0, 60.0],
            b: [0.0, 0.0],
        },
    ];
    let q = [[cx + rr, cy], [cx, cy - rr], [cx - rr, cy], [cx, cy + rr]];
    for i in 0..4 {
        segs.push(TSeg::Arc {
            a: q[i],
            b: q[(i + 1) % 4],
            c: [cx, cy],
            r: rr,
            ccw: false,
        });
    }
    let plate = Face::new(
        Surf::Plane {
            o: [0.0, 0.0, 0.0],
            u: [1.0, 0.0, 0.0],
            v: [0.0, 1.0, 0.0],
        },
        Ival::new(0.0, 60.0),
        Ival::new(0.0, 60.0),
        Some(Trim { segs }),
        1e-7,
        0,
    );
    let q0 = 2.0 - 3f64.sqrt(); // tan(15°): the cap from 30° of elevation
    let half = |sgn: f64, id: u32| {
        Face::new(
            Surf::Sphere {
                o: [cx, cy, -10.0],
                x: [sgn, 0.0, 0.0],
                y: [0.0, sgn, 0.0],
                ax: [0.0, 0.0, 1.0],
                r: 20.0,
            },
            Ival::new(-1.0, 1.0),
            Ival::new(q0, 1.0),
            None,
            1e-7,
            id,
        )
    };
    vec![plate, half(1.0, 1), half(-1.0, 2)]
}

fn tool(lib: &Library, id: &str, t: u32) -> ToolUse {
    let tl = lib.tool(id).unwrap().clone();
    let shape = tl
        .shape(tl.lcf + 15.0, Some(lib.holder("bt40-er32").unwrap()))
        .unwrap();
    ToolUse {
        t,
        tool: tl,
        holder: Some("bt40-er32".into()),
        stickout: 0.0,
        shape,
        probe: false,
    }
}

fn report(
    name: &str,
    p: &forge_cam::ops::OpPath,
    made: std::time::Duration,
    tool: &ToolUse,
    faces: &[Face],
    stock: Vec<Face>,
    tol: f64,
) {
    let t1 = std::time::Instant::now();
    let sum = ops::certify_with(p, tool, faces, &[(0, stock)], &[], 0.0, tol, 0.1, true);
    println!(
        "{name}: {} moves, made {:?}; certify {:?}: clear {} pairs {} cells {} least {:.4} {:?}",
        p.moves.len(),
        made,
        t1.elapsed(),
        sum.clear,
        sum.pairs,
        sum.cells,
        sum.least,
        sum.checks.iter().take(2).collect::<Vec<_>>()
    );
}

fn main() {
    let faces = dome();
    let lib = Library::builtin();
    let tol = 0.01;
    let h = Heights {
        clear: 40.0,
        retract: 25.0,
    };
    let area = [2.0, 2.0, 58.0, 58.0];
    let step: f64 = std::env::args().nth(1).map(|s| s.parse().unwrap()).unwrap_or(1.0);
    // roughing: flat 10, 0.5 left, from the stock's top (12) down, then the stock map
    let rough_t = tool(&lib, "flat-10", 1);
    let sfr = Surfacer::new(&faces, &rough_t.shape.bodies[0], 0.5, tol, 0.0).unwrap();
    let c = Cut {
        tool: &rough_t,
        tidx: 0,
        h,
        feed: (1500.0, 300.0),
        ramp_feed: None,
        lead_feed: None,
    };
    let t0 = std::time::Instant::now();
    let rough = surf::zlevel(&sfr, c, [-5.0, -5.0, 65.0, 65.0], 12.0, 0.5, 2.0, 4.0, 1.0).unwrap();
    report("zlevel", &rough, t0.elapsed(), &rough_t, &faces, vec![], tol);
    let mut zm = surf::ZMap::new([0.0, 0.0, 0.0], [60.0, 60.0, 12.0], 1.0);
    let t0 = std::time::Instant::now();
    zm.cut_path(&sfr.check, &rough);
    let top = zm.top.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let stock = zm.faces(5000);
    let hi: Vec<(usize, usize, f64)> = (0..zm.top.len())
        .filter(|&k| zm.top[k] > 11.0)
        .map(|k| (k % zm.nx, k / zm.nx, zm.top[k]))
        .take(10)
        .collect();
    println!(
        "cells above 11: {} e.g. {hi:?}",
        zm.top.iter().filter(|&&t| t > 11.0).count()
    );
    println!(
        "zmap: {} cells cut in {:?}, highest {top:.3}, {} faces",
        zm.top.len(),
        t0.elapsed(),
        stock.len()
    );
    // finishing: ball 6, parallel
    let ball = tool(&lib, "ball-6", 2);
    let sf = Surfacer::new(&faces, &ball.shape.bodies[0], 0.0, tol, 0.0).unwrap();
    let c = Cut {
        tool: &ball,
        tidx: 0,
        h,
        feed: (1000.0, 300.0),
        ramp_feed: None,
        lead_feed: None,
    };
    let t0 = std::time::Instant::now();
    let p = surf::parallel(&sf, c, area, step, 0.0, 1.0).unwrap();
    report("parallel", &p, t0.elapsed(), &ball, &faces, stock.clone(), tol);
    let t0 = std::time::Instant::now();
    let zs: Vec<f64> = (1..=9).map(|k| k as f64).collect();
    let w = surf::waterline(&sf, c, area, &zs, 0.5).unwrap();
    report("waterline", &w, t0.elapsed(), &ball, &faces, stock.clone(), tol);
    let t0 = std::time::Instant::now();
    match surf::pencil(&sf, c, area, 0.25, 0.02) {
        Ok(pc) => report("pencil", &pc, t0.elapsed(), &ball, &faces, stock.clone(), tol),
        Err(e) => println!("pencil: {e:?}"),
    }
    let t0 = std::time::Instant::now();
    match surf::scallop(&sf, c, area, step, 0.5) {
        Ok(r) => report("scallop", &r, t0.elapsed(), &ball, &faces, stock.clone(), tol),
        Err(e) => println!("scallop: {e:?}"),
    }
    // rest: ball 6 after the bull 10 r1
    let bull = tool(&lib, "bull-10-r1", 3);
    let sfb = Surfacer::new(&faces, &bull.shape.bodies[0], 0.0, tol, 0.0).unwrap();
    let t0 = std::time::Instant::now();
    match surf::rest(&sf, &sfb, c, area, step, 0.5, 0.5, 0.05) {
        Ok(r) => report("rest", &r, t0.elapsed(), &ball, &faces, stock.clone(), tol),
        Err(e) => println!("rest: {e:?}"),
    }
}
