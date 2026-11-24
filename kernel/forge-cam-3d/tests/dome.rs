//! End to end on a dome (sphere r = 30 about (0, 0, −10) on the floor z = 0) with a 6 mm ball:
//! waterline passes from the analytic drop-cutter, then forge-cam's certifier on the exact
//! sphere and plane faces: every move must be proven clear (G1 with the tolerance, holder too).
use forge_cam::ops::{self, Heights, ToolUse};
use forge_cam_3d::drop::Analytic;
use forge_cam_3d::link;
use forge_cam_3d::path::{self, Emit};
use forge_cam_3d::scallop::{Contact, step_for, waterline_step};
use forge_cam_3d::waterline::{self, Params};
use forge_cam_geom::check::{Face, Surf};
use forge_cam_geom::ival::Ival;
use forge_tooling::Library;

fn ball6() -> ToolUse {
    let l = Library::builtin();
    let t = l.tool("ball-6").unwrap().clone();
    let shape = t.shape(60.0, Some(l.holder("bt40-er32").unwrap())).unwrap();
    ToolUse {
        t: 1,
        tool: t,
        holder: Some("bt40-er32".into()),
        stickout: 60.0,
        shape,
    }
}

fn dome_faces() -> Vec<Face> {
    let o = [0.0, 0.0, -10.0];
    // elevation from just below the floor up to the pole: q = tan(φ/2) ∈ [0.15, 1]
    let q = Ival::new(0.15, 1.0);
    let halves = [
        ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([-1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),
    ];
    let mut v = Vec::new();
    for (k, (x, y)) in halves.into_iter().enumerate() {
        v.push(Face::new(
            Surf::Sphere {
                o,
                x,
                y,
                ax: [0.0, 0.0, 1.0],
                r: 30.0,
            },
            Ival::new(-1.0, 1.0),
            q,
            None,
            1e-9,
            k as u32,
        ));
    }
    v.push(Face::new(
        Surf::Plane {
            o: [-50.0, -50.0, 0.0],
            u: [1.0, 0.0, 0.0],
            v: [0.0, 1.0, 0.0],
        },
        Ival::new(0.0, 100.0),
        Ival::new(0.0, 100.0),
        None,
        1e-9,
        9,
    ));
    v
}

#[test]
fn dome_waterline_is_certified() {
    let dc = Analytic {
        r: 3.0,
        planes: vec![(0.0, [-50.0, -50.0], [50.0, 50.0])],
        spheres: vec![([0.0, 0.0, -10.0], 30.0)],
    };
    // scallop 0.01 mm on slopes of 45° and steeper
    let d = step_for(Contact::Ball { r: 3.0 }, 0.01, 6.0).unwrap();
    let h = std::f64::consts::FRAC_1_SQRT_2;
    let s45 = Ival::new(h.next_down(), h.next_up());
    let dz = waterline_step(d, s45);
    assert!(dz > 0.3 && dz < 0.5, "dz={dz}");
    let mut loops = Vec::new();
    // two levels of the steep band keep the test quick; FORGE_CAM3D_FULL=1 runs the band 1..10
    let full = std::env::var("FORGE_CAM3D_FULL").is_ok();
    let (top, bottom, step) = if full { (10.0, 1.0, dz) } else { (8.0, 5.0, 1.5) };
    for z in waterline::levels(top, bottom, step) {
        loops.extend(
            waterline::level(
                &dc,
                z,
                Params {
                    grid: 2.0,
                    tol_xy: 0.01,
                },
            )
            .unwrap(),
        );
    }
    assert!(!loops.is_empty());
    let e = Emit {
        tool: 0,
        h: Heights {
            clear: 40.0,
            retract: 25.0,
        },
        feed: (1500.0, 500.0),
        rpm: 12000.0,
        coolant: forge_post::Coolant::Flood,
    };
    // closed passes linked by the 3D rule (stay down / lift / retract), all certified below
    let passes: Vec<Vec<[f64; 3]>> = loops
        .iter()
        .map(|l| {
            let mut v: Vec<[f64; 3]> = l.pts.iter().map(|q| [q[0], q[1], l.z]).collect();
            v.push(v[0]);
            v
        })
        .collect();
    let passes = link::order(vec![passes], [0.0, 0.0, 40.0], false);
    let (p, count) = link::path(
        &dc,
        &passes,
        link::Rules {
            max_stay: 2.0 * dz,
            lift: 0.5,
        },
        e,
    );
    eprintln!("links {count:?}");
    assert_eq!(count.stay + count.lift + count.retract + 1, passes.len());
    let t0 = std::time::Instant::now();
    let sum = ops::certify(&p, &ball6(), &dome_faces(), &[], 0.01, 1.0);
    eprintln!(
        "moves {} pairs {} cells {} least {} secs {:.1}",
        p.moves.len(),
        sum.pairs,
        sum.cells,
        sum.least,
        t0.elapsed().as_secs_f64()
    );
    let bad: Vec<_> = sum
        .checks
        .iter()
        .filter(|c| c.verdict != "clear")
        .take(5)
        .collect();
    assert!(sum.clear, "{bad:?}");
}

#[test]
fn dome_draped_passes_are_certified() {
    use forge_cam_3d::drape::{Drape, drape};
    let dc = Analytic {
        r: 3.0,
        planes: vec![(0.0, [-50.0, -50.0], [50.0, 50.0])],
        spheres: vec![([0.0, 0.0, -10.0], 30.0)],
    };
    let d = Drape {
        step: 2.0,
        slack: 0.005,
        leave: 0.01,
        zmin: 0.0,
    };
    // one line across the dome's flank (more cost minutes: see the waterline test's note)
    let ps = drape(&dc, &[[-36.0, 17.0], [36.0, 17.0]], d).unwrap();
    let e = Emit {
        tool: 0,
        h: Heights {
            clear: 40.0,
            retract: 25.0,
        },
        feed: (1500.0, 500.0),
        rpm: 12000.0,
        coolant: forge_post::Coolant::Flood,
    };
    let p = path::passes(&ps, e);
    let t0 = std::time::Instant::now();
    let sum = ops::certify(&p, &ball6(), &dome_faces(), &[], 0.01, 1.0);
    eprintln!(
        "drape: moves {} cells {} secs {:.1}",
        p.moves.len(),
        sum.cells,
        t0.elapsed().as_secs_f64()
    );
    let bad: Vec<_> = sum
        .checks
        .iter()
        .filter(|c| c.verdict != "clear")
        .take(5)
        .collect();
    assert!(sum.clear, "{bad:?}");
}

/// Small samples of the other strategies, through the same certifier: pencil in the concave
/// corner at the dome's foot, a few spiral turns on the cap, one ramp turn, scallop rings on the
/// cap. Each sample is cut down to a few dozen moves to keep the certifier's time reasonable.
#[test]
fn other_strategies_certify_on_the_dome() {
    use forge_cam_3d::contain::Containment;
    use forge_cam_3d::drape::Drape;
    use forge_cam_3d::{geodesic, pencil, ramp, strategy};
    let dc = Analytic {
        r: 3.0,
        planes: vec![(0.0, [-50.0, -50.0], [50.0, 50.0])],
        spheres: vec![([0.0, 0.0, -10.0], 30.0)],
    };
    let d = Drape {
        step: 2.0,
        slack: 0.002,
        leave: 0.01,
        zmin: 0.0,
    };
    // a quarter of the work area, x > 0, y > 0, keeps every sample small
    let quarter = Containment {
        boundary: Some(vec![[0.0, 0.0], [12.0, 0.0], [12.0, 12.0], [0.0, 12.0]]),
        ..Default::default()
    };
    let mut ps: Vec<Vec<[f64; 3]>> = Vec::new();
    // pencil: the foot crease, one arc of it
    let foot = Containment {
        boundary: Some(vec![[20.0, 0.0], [34.0, 0.0], [34.0, 8.0], [20.0, 8.0]]),
        ..Default::default()
    };
    let pp = pencil::pencil(
        &dc,
        pencil::Params {
            grid: 0.5,
            kink: 1.0,
            min_len: 2.0,
        },
        d,
        &foot,
    )
    .unwrap();
    assert!(!pp.is_empty(), "pencil found nothing");
    ps.extend(pp);
    // spiral: two turns on the cap, clipped to the quarter
    ps.extend(strategy::spiral(&dc, [0.0, 0.0], 0.0, 6.0, 3.0, false, d, &quarter).unwrap());
    // ramp: from the level at 19 to the level at 18.5 (a small ring on the cap)
    let lv: Vec<_> = [19.0, 18.5]
        .iter()
        .map(|&z| {
            waterline::level(
                &dc,
                z,
                Params {
                    grid: 1.0,
                    tol_xy: 0.01,
                },
            )
            .unwrap()
        })
        .collect();
    let rp = ramp::ramps(&dc, &lv, d).unwrap();
    ps.extend(quarter.clip(rp, None));
    // scallop rings on the cap inside a small square
    let cap = Containment {
        boundary: Some(vec![[0.0, 0.0], [6.0, 0.0], [6.0, 6.0], [0.0, 6.0]]),
        ..Default::default()
    };
    ps.extend(
        geodesic::scallop(
            &dc,
            geodesic::Params {
                grid: 0.5,
                stepover: 2.0,
                tan_wall: f64::INFINITY,
            },
            d,
            &cap,
        )
        .unwrap(),
    );
    let e = Emit {
        tool: 0,
        h: Heights {
            clear: 40.0,
            retract: 25.0,
        },
        feed: (1500.0, 500.0),
        rpm: 12000.0,
        coolant: forge_post::Coolant::Flood,
    };
    let ps = link::order(vec![ps], [0.0, 0.0, 40.0], true);
    let (p, count) = link::path(
        &dc,
        &ps,
        link::Rules {
            max_stay: 1.0,
            lift: 0.5,
        },
        e,
    );
    let t0 = std::time::Instant::now();
    let sum = ops::certify(&p, &ball6(), &dome_faces(), &[], 0.01, 1.0);
    eprintln!(
        "others: passes {} moves {} links {count:?} cells {} secs {:.1}",
        ps.len(),
        p.moves.len(),
        sum.cells,
        t0.elapsed().as_secs_f64()
    );
    let bad: Vec<_> = sum
        .checks
        .iter()
        .filter(|c| c.verdict != "clear")
        .take(5)
        .collect();
    assert!(sum.clear, "{bad:?}");
}

/// The generic certified drop-cutter on the exact faces agrees with the analytic one, and a
/// pass draped through it certifies.
#[test]
fn certdrop_on_the_faces_agrees_and_certifies() {
    use forge_cam_3d::certdrop::CertDrop;
    use forge_cam_3d::drop::DropCutter;
    let tool = ball6();
    let faces = dome_faces();
    let cd = CertDrop {
        cutter: &tool.shape.bodies[0],
        faces: &faces,
        leave: 0.0,
        tol: 1e-4,
        budget: 20_000,
        res: 1e-4,
    };
    let an = Analytic {
        r: 3.0,
        planes: vec![(0.0, [-50.0, -50.0], [50.0, 50.0])],
        spheres: vec![([0.0, 0.0, -10.0], 30.0)],
    };
    for &(x, y) in &[
        (0.0, 0.0),
        (10.0, 5.0),
        (22.0, 3.0),
        (29.0, -4.0),
        (31.0, 0.0),
        (40.0, 40.0),
    ] {
        let c = cd.drop(x, y).unwrap();
        let a = an.drop(x, y).unwrap();
        assert!(
            c.lo <= a.hi + 1e-6 && a.lo <= c.hi + 1e-6 && c.width() <= 1e-3,
            "({x},{y}) cert {c:?} analytic {a:?}"
        );
    }
    // a short draped pass through it (the generic drop needs the branch and bound for every
    // witness: correct, slow; the exact drop-cutter is the primary), then the certifier
    use forge_cam_3d::drape::{Drape, drape};
    let t0 = std::time::Instant::now();
    let d = Drape {
        step: 1.0,
        slack: 0.002,
        leave: 0.02,
        zmin: 0.0,
    };
    let ps = drape(&cd, &[[18.0, 2.0], [22.0, 2.0]], d).unwrap();
    assert_eq!(ps.len(), 1);
    let e = Emit {
        tool: 0,
        h: Heights {
            clear: 40.0,
            retract: 25.0,
        },
        feed: (1500.0, 500.0),
        rpm: 12000.0,
        coolant: forge_post::Coolant::Flood,
    };
    let p = path::passes(&ps, e);
    let sum = ops::certify(&p, &tool, &faces, &[], 0.01, 1.0);
    eprintln!(
        "certdrop drape: {} moves, generated + certified in {:.1} s",
        p.moves.len(),
        t0.elapsed().as_secs_f64()
    );
    assert!(sum.clear);
}
