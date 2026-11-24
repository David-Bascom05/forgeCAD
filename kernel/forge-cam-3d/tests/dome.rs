//! End to end on a dome (sphere r = 30 about (0, 0, −10) on the floor z = 0) with a 6 mm ball:
//! waterline passes from the analytic drop-cutter, then forge-cam's certifier on the exact
//! sphere and plane faces: every move must be proven clear (G1 with the tolerance, holder too).
use forge_cam::ops::{self, Heights, ToolUse};
use forge_cam_3d::drop::Analytic;
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
    let (top, bottom, step) = if full { (10.0, 1.0, dz) } else { (8.0, 6.0, 2.0) };
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
    let p = path::loops(&loops, e);
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
