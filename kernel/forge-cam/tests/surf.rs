//! 3-axis surfacing on a dome (a sphere cap on a plate): every op's path certified by the
//! checker's column route; a path moved into the part is refused; the z-map stock holds the
//! true stock (sampled).
use forge_cam::ops::{self, Cut, Heights, OpPath, PMove, ToolUse};
use forge_cam::surf::{self, Surfacer, ZMap};
use forge_cam_geom::check::{Face, Surf, TSeg, Trim};
use forge_cam_geom::drop::Column;
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

fn tool(id: &str, t: u32) -> ToolUse {
    let lib = Library::builtin();
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
    }
}

const TOL: f64 = 0.01;
const H: Heights = Heights {
    clear: 40.0,
    retract: 25.0,
};

fn certified(p: &OpPath, t: &ToolUse, faces: &[Face]) -> ops::CheckSummary {
    ops::certify_with(p, t, faces, &[(0, vec![])], &[], 0.0, TOL, 0.1, true)
}

#[test]
fn finishing_ops_certify_on_the_dome() {
    let faces = dome();
    let ball = tool("ball-6", 1);
    let sf = Surfacer::new(&faces, &ball.shape.bodies[0], 0.0, TOL, 0.0).unwrap();
    let c = Cut {
        tool: &ball,
        tidx: 0,
        h: H,
        feed: (1000.0, 300.0),
    };
    let area = [10.0, 10.0, 50.0, 50.0];
    let mut paths = vec![("parallel", surf::parallel(&sf, c, area, 2.0, 30.0, 1.0).unwrap())];
    paths.push((
        "waterline",
        surf::waterline(&sf, c, area, &[2.0, 6.0], 1.0).unwrap(),
    ));
    paths.push(("scallop", surf::scallop(&sf, c, area, 2.5, 1.0).unwrap()));
    paths.push(("pencil", surf::pencil(&sf, c, area, 0.5, 0.02).unwrap()));
    let bull = tool("bull-10-r1", 2);
    let sfb = Surfacer::new(&faces, &bull.shape.bodies[0], 0.0, TOL, 0.0).unwrap();
    paths.push((
        "rest",
        surf::rest(&sf, &sfb, c, area, 2.0, 1.0, 1.0, 0.05).unwrap(),
    ));
    for (name, p) in &paths {
        let s = certified(p, &ball, &faces);
        assert!(s.clear, "{name}: {:?}", s.checks);
        assert!(p.moves.len() > 10, "{name}");
    }
    // the parallel path moved 0.05 into the part is refused
    let mut low = paths[0].1.clone();
    for m in low.moves.iter_mut() {
        if let PMove::Line {
            to,
            phase: ops::Phase::Cut,
            ..
        } = m
        {
            to[2] -= 0.05;
        }
    }
    let s = certified(&low, &ball, &faces);
    assert!(!s.clear);
    assert!(s.checks.iter().any(|c| c.rule == "gouge"), "{:?}", s.checks);
}

#[test]
fn zlevel_roughing_certifies_and_the_map_holds_the_stock() {
    let faces = dome();
    let flat = tool("flat-10", 1);
    let sf = Surfacer::new(&faces, &flat.shape.bodies[0], 0.5, TOL, 0.0).unwrap();
    let c = Cut {
        tool: &flat,
        tidx: 0,
        h: H,
        feed: (1500.0, 300.0),
    };
    let p = surf::zlevel(&sf, c, [-5.0, -5.0, 65.0, 65.0], 12.0, 0.5, 4.0, 4.0, 2.0).unwrap();
    let s = certified(&p, &flat, &faces);
    assert!(s.clear, "{:?}", s.checks);
    // a stepover wider than the tool's radius leaves posts between passes: refused
    assert!(surf::zlevel(&sf, c, [-5.0, -5.0, 65.0, 65.0], 12.0, 0.5, 4.0, 6.0, 2.0).is_err());
    let mut zm = ZMap::new([0.0, 0.0, 0.0], [60.0, 60.0, 12.0], 2.0);
    zm.cut_path(&sf.check, &p);
    // every cell lowered is cut: at random points of it the tool's lowest reach over the path
    // (sampled along each move) is at or below the cell's top
    let col: &Column = &sf.check;
    let mut st = 12345u64;
    let mut rnd = || {
        st ^= st << 13;
        st ^= st >> 7;
        st ^= st << 17;
        (st >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut segs = Vec::new();
    let mut at = p.start;
    for m in &p.moves {
        match *m {
            PMove::Line { to, .. } => {
                segs.push((at, to));
                at = to;
            }
            PMove::Rapid { to } | PMove::Arc { to, .. } => at = to,
            PMove::Cycle { .. } => {}
        }
    }
    let mut lowered = 0;
    for iy in 0..zm.ny {
        for ix in 0..zm.nx {
            let t = zm.top[iy * zm.nx + ix];
            if t >= 12.0 {
                continue;
            }
            lowered += 1;
            for _ in 0..4 {
                let (x, y) = (
                    zm.x0 + (ix as f64 + rnd()) * zm.h,
                    zm.y0 + (iy as f64 + rnd()) * zm.h,
                );
                // the tool at the instant nearest (x, y) on some move reaches down to t there
                let mut reach = f64::INFINITY;
                for (a, b) in &segs {
                    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                    let dd = dx * dx + dy * dy;
                    let s = if dd == 0.0 {
                        0.0
                    } else {
                        (((x - a[0]) * dx + (y - a[1]) * dy) / dd).clamp(0.0, 1.0)
                    };
                    let q = [a[0] + s * dx, a[1] + s * dy, a[2] + s * (b[2] - a[2])];
                    let rho = ((x - q[0]).powi(2) + (y - q[1]).powi(2)).sqrt();
                    if rho <= col.r {
                        reach = reach.min(q[2] + col.b_at(rho).hi);
                    }
                }
                assert!(reach <= t + 1e-9, "cell ({ix}, {iy}) top {t}, reach {reach}");
            }
        }
    }
    assert!(lowered > 100);
    assert!(!zm.faces(9000).is_empty());
}

/// Native = wasm, bit for bit: a digest of the dome's 3D paths (every coordinate's bits), pinned;
/// the same constant must hold on wasm32-wasip1 (tools/test-wasm.sh).
#[test]
fn surfacing_paths_are_the_same_bits_everywhere() {
    let faces = dome();
    let ball = tool("ball-6", 1);
    let sf = Surfacer::new(&faces, &ball.shape.bodies[0], 0.0, TOL, 0.0).unwrap();
    let c = Cut {
        tool: &ball,
        tidx: 0,
        h: H,
        feed: (1000.0, 300.0),
    };
    let area = [10.0, 10.0, 50.0, 50.0];
    let flat = tool("flat-10", 2);
    let sfr = Surfacer::new(&faces, &flat.shape.bodies[0], 0.5, TOL, 0.0).unwrap();
    let cr = Cut {
        tool: &flat,
        tidx: 0,
        h: H,
        feed: (1500.0, 300.0),
    };
    let paths = [
        surf::parallel(&sf, c, area, 3.0, 30.0, 1.0).unwrap(),
        surf::waterline(&sf, c, area, &[3.0], 1.0).unwrap(),
        surf::zlevel(&sfr, cr, [-5.0, -5.0, 65.0, 65.0], 12.0, 4.0, 4.0, 4.0, 2.0).unwrap(),
    ];
    let mut h: u64 = 0xcbf29ce484222325;
    let mut eat = |x: f64| {
        for b in x.to_bits().to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    };
    for p in &paths {
        for m in &p.moves {
            match *m {
                PMove::Rapid { to } | PMove::Line { to, .. } => to.iter().for_each(|&x| eat(x)),
                PMove::Arc { to, c, .. } => {
                    to.iter().for_each(|&x| eat(x));
                    c.iter().for_each(|&x| eat(x));
                }
                PMove::Cycle { .. } => {}
            }
        }
    }
    assert_eq!(h, 0x423e3078ebeba40d, "digest {h:#018x}");
}
