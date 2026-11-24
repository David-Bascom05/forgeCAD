//! Removal's tip in its two forms (docs/cam-machine.md, Removal: the tip's two forms): the affine
//! form `mill` uses against the pose chain at every s it replaced. Over the flange-sized pocket at
//! CAM-SIM's grid, on the 3-axis mill and on the trunnion turned on C (rotations whose bits are not
//! exact): every tip along every segment within the stated bound of the chained one, and every
//! height of the map within it too.
use forge_machine::MachineDef;
use forge_machine::def::JointType;
use forge_machine::gcode::Control;
use forge_machine::library::builtin_named;
use forge_machine::removal::{self, TipForm};
use forge_machine::sim::{self, Cut, Path, Segment, Setup, SimTool};

const U: f64 = f64::EPSILON / 2.0;

fn inf(v: [f64; 3]) -> f64 {
    v.iter().fold(0.0f64, |m, x| m.max(x.abs()))
}

/// The docs' bound on |affine − chained| (each coordinate) for a segment:
/// 2 (28k + 74) u M + ((n + 1)(4n + 1) + 52n) u M, k the joints on the tool's and the part's
/// chains, n the moving linear joints, M twice the sum of the chain's magnitudes.
fn bound(s: &Setup, seg: &Segment, t: u32) -> f64 {
    let def: &MachineDef = s.def;
    let tool = s.tool(t).unwrap();
    let m = s.mount_for(t).unwrap();
    let pm = def.mount(&s.part_mount).unwrap();
    let chain = |l: &str| match def.joint_index(l) {
        Some(_) => def.chain(l).unwrap(),
        None => Vec::new(),
    };
    let (ct, cp) = (chain(&m.link), chain(&pm.link));
    let k = (ct.len() + cp.len()) as f64;
    let n = s.tip_columns(seg, t).unwrap().1.len() as f64;
    let mut sum = inf([
        m.at[0] + tool.tip[0],
        m.at[1] + tool.tip[1],
        m.at[2] + tool.tip[2],
    ]) + inf(pm.at);
    for j in ct.iter().chain(&cp) {
        let jt = &def.joints[*j];
        sum += match jt.ty {
            JointType::Linear => {
                let mut q = seg.q0[*j].abs().max(seg.q1[*j].abs());
                if let Path::Arc { a, b, c, r, r1, .. } = &seg.path {
                    let rr = r.abs().max(r1.abs());
                    if *j == *a {
                        q = q.max(c[0].abs() + rr);
                    }
                    if *j == *b {
                        q = q.max(c[1].abs() + rr);
                    }
                }
                q
            }
            JointType::Rotary => 3.0 * inf(jt.pivot),
        };
    }
    let mm = 2.0 * sum;
    (2.0 * (28.0 * k + 74.0) + (n + 1.0) * (4.0 * n + 1.0) + 52.0 * n) * U * mm
}

fn pocket() -> String {
    let mut p = String::from("G43 H1\nG0 X0 Y0 Z30.\n");
    for lvl in 1..=8 {
        let z = 23.0 - 2.0 * lvl as f64;
        p.push_str(&format!("G0 X-40. Y-40.\nG1 Z{z:.1} F500.\n"));
        let mut y = -40.0;
        let mut dir = 1.0;
        while y <= 40.0 {
            p.push_str(&format!("G1 X{:.1} Y{y:.1} F1500.\n", 40.0 * dir));
            y += 6.0;
            p.push_str(&format!("G1 Y{y:.1}\n"));
            dir = -dir;
        }
        p.push_str("G0 X30. Y0\nG2 X30. Y0 I-30. J0 F1200.\nG0 Z30.\n");
    }
    p
}

fn tool() -> SimTool {
    SimTool {
        t: 1,
        name: "e10".into(),
        tip: [0.0, 0.0, -100.0],
        body: vec![(5.0, 0.0, 25.0), (12.0, 25.0, 80.0)],
        cut: Cut::Mill { r: 5.0, corner: 1.0 },
    }
}

/// The two forms over a program: (the largest tip gap, its bound's least ratio margin), then the
/// heights' largest gap and the largest segment bound.
fn compare(s: &Setup, prog: &str) {
    let tl = sim::simulate(prog, s).unwrap();
    let mut worst_tip = 0.0f64;
    let mut max_bound = 0.0f64;
    let mut cutting = 0;
    for seg in &tl.segments {
        let Some(t) = seg.tool else { continue };
        let (Some(a), Some(c)) = (s.tip_affine(seg, t), s.tip_along(seg, t)) else {
            continue;
        };
        cutting += 1;
        let b = bound(s, seg, t);
        max_bound = max_bound.max(b);
        for i in 0..=1024 {
            let u = i as f64 / 1024.0;
            let (x, y) = (a(u), c(u));
            let gap = (0..3).map(|k| (x[k] - y[k]).abs()).fold(0.0, f64::max);
            assert!(
                gap <= b,
                "line {} s {u}: {x:?} vs {y:?}, gap {gap:e} > {b:e}",
                seg.line
            );
            worst_tip = worst_tip.max(gap);
        }
    }
    assert!(cutting > 200, "{cutting}");
    let (lo, hi) = ([-43.0, -43.0, 0.0], [43.0, 43.0, 23.0]);
    let ra = removal::mill_with(s, &tl, lo, hi, 0.5, TipForm::Affine).unwrap();
    let rc = removal::mill_with(s, &tl, lo, hi, 0.5, TipForm::Chained).unwrap();
    assert_eq!(ra.map.z.len(), rc.map.z.len());
    let worst_h = ra
        .map
        .z
        .iter()
        .zip(&rc.map.z)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max);
    // a height moves with the tip (the cutter's flat or corner below it), except at a cell the
    // cutter's rim only grazes: there it is not continuous in the tip (docs); on this grid none is
    assert!(
        worst_h <= max_bound,
        "heights differ by {worst_h:e} > {max_bound:e}"
    );
    assert_eq!(ra.rapid_cuts.len(), rc.rapid_cuts.len());
    assert_eq!(ra.holder_hits.len(), rc.holder_hits.len());
    println!(
        "tips: largest gap {worst_tip:e}, largest bound {max_bound:e}; heights: largest gap {worst_h:e}"
    );
}

#[test]
fn the_affine_tip_is_within_its_bound_on_the_three_axis_mill() {
    let m = builtin_named("generic-vmc3").unwrap();
    let mut s = Setup::new(&m, Control::for_dialect(&m, "fanuc", 1).unwrap()).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -500.0])];
    s.h = vec![(1, 100.0)];
    s.tools = vec![tool()];
    s.start_tool = Some(1);
    compare(&s, &pocket());
}

#[test]
fn the_affine_tip_is_within_its_bound_on_the_trunnion_turned_on_c() {
    let m = builtin_named("generic-trunnion-bc").unwrap();
    let mut s = Setup::new(&m, Control::for_dialect(&m, "fanuc", 1).unwrap()).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -400.0, 0.0, 0.0])];
    s.h = vec![(1, 100.0)];
    s.tools = vec![tool()];
    s.start_tool = Some(1);
    // C at an angle whose sine and cosine are not exact, from the start: the columns are rotated
    // axes
    s.start[m.joint_index("C").unwrap()] = -121.25;
    compare(&s, &pocket());
}
