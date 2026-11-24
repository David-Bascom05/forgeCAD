//! The strategies of this crate on a real part, through CAM-3AXIS's drop-cutter: the mould core
//! (examples/machined/mold_core.fg: a block with every convex edge rounded on a plate, its foot
//! filleted; planes, horizontal and vertical cylinders, sphere corners, tori), lowered by
//! forge-cam's lower3d, with a 6 mm ball. Each strategy's passes, linked, are certified by
//! forge-cam's checker (the column route, G1 at tol 0.01, holder and shank too).
use forge_cam::ops::{self, Heights, ToolUse};
use forge_cam::surf::Surfacer;
use forge_cam_3d::contain::Containment;
use forge_cam_3d::drape::{Drape, Pass};
use forge_cam_3d::drop::DropCutter;
use forge_cam_3d::link;
use forge_cam_3d::path::Emit;
use forge_cam_3d::surfdrop::SurfDrop;
use forge_cam_3d::{geodesic, pencil, ramp, steep, strategy, waterline};
use forge_cam_geom::check::Face;
use forge_tooling::Library;
use std::path::Path;

fn mould() -> Vec<Face> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined/mold_core.fg");
    let src = std::fs::read_to_string(p).unwrap();
    let (run, _) = forge_lang::run(&src, forge_lang::Options::default(), Default::default());
    let body = run.body.expect("the mould core builds");
    forge_cam::lower3d::lower_surfacing(&body).unwrap().0
}

fn ball6() -> ToolUse {
    let l = Library::builtin();
    let t = l.tool("ball-6").unwrap().clone();
    let shape = t.shape(40.0, Some(l.holder("bt40-er32").unwrap())).unwrap();
    ToolUse {
        t: 2,
        tool: t,
        holder: Some("bt40-er32".into()),
        stickout: 40.0,
        shape,
    }
}

fn emit() -> Emit {
    Emit {
        tool: 0,
        h: Heights {
            clear: 50.0,
            retract: 40.0,
        },
        feed: (1500.0, 500.0),
        rpm: 12000.0,
        coolant: forge_post::Coolant::Flood,
    }
}

/// Links the passes and certifies the path; returns (moves, seconds).
fn certify(dc: &dyn DropCutter, what: &str, ps: Vec<Pass>, faces: &[Face], tool: &ToolUse) -> (usize, f64) {
    assert!(!ps.is_empty(), "{what}: no passes");
    let ps = link::order(vec![ps], [0.0, 0.0, 50.0], true);
    let (p, count) = link::path(
        dc,
        &ps,
        link::Rules {
            max_stay: 2.0,
            lift: 0.5,
        },
        emit(),
    );
    let t0 = std::time::Instant::now();
    let sum = ops::certify_with(&p, tool, faces, &[(0, Vec::new())], &[], 0.0, 0.01, 1.0, true);
    let secs = t0.elapsed().as_secs_f64();
    eprintln!(
        "{what}: passes {} moves {} links {count:?} certified in {secs:.1} s",
        ps.len(),
        p.moves.len()
    );
    let bad: Vec<_> = sum
        .checks
        .iter()
        .filter(|c| c.verdict != "clear")
        .take(5)
        .collect();
    assert!(sum.clear, "{what}: {bad:?}");
    (p.moves.len(), secs)
}

#[test]
fn every_strategy_on_the_mould_core_is_certified() {
    let faces = mould();
    let tool = ball6();
    let sf = Surfacer::new(&faces, &tool.shape.bodies[0], 0.0, 0.01, 10.0).unwrap();
    let dc = SurfDrop::new(&sf).unwrap();
    let d = Drape {
        step: 1.0,
        slack: 0.004,
        leave: 0.01,
        zmin: 10.0,
    };
    // the block's top is at 26 (plate 10 + block 16), its footprint ±20 × ±14 (rounded r 5)
    let top_box = Containment {
        boundary: Some(vec![[-12.0, -8.0], [12.0, -8.0], [12.0, 8.0], [-12.0, 8.0]]),
        ..Default::default()
    };
    let quarter = Containment {
        boundary: Some(vec![[0.0, 0.0], [30.0, 0.0], [30.0, 24.0], [0.0, 24.0]]),
        ..Default::default()
    };

    // steep and shallow over a quarter of the part (the waterlines on the walls, parallel on top
    // and on the plate), 45°
    let h = std::f64::consts::FRAC_1_SQRT_2;
    let p = steep::Params {
        sin0: h,
        cos0: h,
        overlap: 1.0,
        dz: 2.0,
        stepover: 2.0,
        angle_deg: 0.0,
        top: 26.0,
        bottom: 10.5,
        grid: 1.0,
        tol: 0.01,
    };
    let (st, sh) = steep::steep_and_shallow(&dc, p).unwrap();
    let mut ss = quarter.clip(st, None);
    ss.extend(quarter.clip(sh, None));
    certify(&dc, "steep_shallow", ss, &faces, &tool);

    // spiral and radial on the block's top
    certify(
        &dc,
        "spiral",
        strategy::spiral(&dc, [0.0, 0.0], 0.0, 12.0, 2.0, false, d, &top_box).unwrap(),
        &faces,
        &tool,
    );
    certify(
        &dc,
        "radial",
        strategy::radial(&dc, [0.0, 0.0], 2.0, 12.0, 3.0, d, &top_box).unwrap(),
        &faces,
        &tool,
    );

    // ramp between two waterline levels on the walls, a quarter of each ring
    let lv: Vec<_> = [22.0, 20.0]
        .iter()
        .map(|&z| {
            waterline::level(
                &dc,
                z,
                waterline::Params {
                    grid: 1.0,
                    tol_xy: 0.01,
                },
            )
            .unwrap()
        })
        .collect();
    certify(
        &dc,
        "ramp",
        quarter.clip(ramp::ramps(&dc, &lv, d).unwrap(), None),
        &faces,
        &tool,
    );

    // flat: the block's top and the plate, a quarter
    let sm = steep::slope_map(&dc, 1.0);
    let lv = strategy::flat_levels(&dc, &sm, 1e-3, 1e-3, 20);
    assert!(
        lv.iter().any(|z| (z - 26.0).abs() < 0.01) && lv.iter().any(|z| (z - 10.0).abs() < 0.01),
        "{lv:?}"
    );
    certify(
        &dc,
        "flat",
        strategy::flat(&dc, &lv, 0.0, 3.0, 0.5, 0.01, &quarter),
        &faces,
        &tool,
    );

    // scallop rings on the top, pencil along the foot fillet (a quarter)
    certify(
        &dc,
        "scallop",
        geodesic::scallop(
            &dc,
            geodesic::Params {
                grid: 0.5,
                stepover: 2.0,
                tan_wall: 1.0,
            },
            d,
            &top_box,
        )
        .unwrap(),
        &faces,
        &tool,
    );
    let pc = pencil::pencil(
        &dc,
        pencil::Params {
            grid: 0.5,
            kink: 0.5,
            min_len: 3.0,
        },
        d,
        &quarter,
    )
    .unwrap();
    certify(&dc, "pencil", pc, &faces, &tool);
}
