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
use forge_math::dmath;

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

/// The xy length of a segment's tip path (an arc's from 256 chords: at most its true length).
fn xy_len(s: &Setup, seg: &Segment, t: u32) -> f64 {
    let p = s.tip_affine(seg, t).unwrap();
    let mut prev = p(0.0);
    let mut l = 0.0;
    for i in 1..=256 {
        let x = p(i as f64 / 256.0);
        l += ((x[0] - prev[0]).powi(2) + (x[1] - prev[1]).powi(2)).sqrt();
        prev = x;
    }
    // chords under-measure an arc by at most (1 - sin(x)/x) of it, x half a chord's angle
    l * 1.001
}

/// The docs' bound on a height between the closed form and the reference (the chained tip and
/// the search): the tips' bound δ, the nearest point's distance within
/// δd = 2·1e-13·L + 2δ + 16·u·Mxy (the search's last bracket of 1e-13 in s over the path's xy
/// length L, both tips, the distance's rounding), and the corner's profile, ½-Hölder with
/// constant √(2·corner): δ + δd + √(2·corner·δd).
fn height_bound(s: &Setup, seg: &Segment, t: u32, corner: f64) -> f64 {
    let d = bound(s, seg, t);
    let p = s.tip_affine(seg, t).unwrap();
    let mxy = (0..=16)
        .map(|i| {
            let x = p(i as f64 / 16.0);
            x[0].abs().max(x[1].abs())
        })
        .fold(0.0, f64::max)
        + 50.0;
    let dd = 2.0 * 1e-13 * xy_len(s, seg, t) + 2.0 * d + 16.0 * U * mxy;
    d + dd + (2.0 * corner * dd).sqrt()
}

/// The forms over a program on the flange's stock and grid: every tip of the affine form within
/// its bound of the chained one; every height of the closed form (what `mill` uses) and of the
/// affine form with the search within their bounds of the reference (chained tip, search).
fn compare(s: &Setup, prog: &str, min_cutting: usize) {
    let tl = sim::simulate(prog, s).unwrap();
    let mut worst_tip = 0.0f64;
    let mut max_bound = 0.0f64;
    let mut max_h = 0.0f64;
    let mut cutting = 0;
    let corner = match s.tools[0].cut {
        Cut::Mill { corner, .. } => corner,
        _ => unreachable!(),
    };
    for seg in &tl.segments {
        let Some(t) = seg.tool else { continue };
        let (Some(a), Some(c)) = (s.tip_affine(seg, t), s.tip_along(seg, t)) else {
            continue;
        };
        cutting += 1;
        let b = bound(s, seg, t);
        max_bound = max_bound.max(b);
        max_h = max_h.max(height_bound(s, seg, t, corner));
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
    assert!(cutting >= min_cutting, "{cutting}");
    let (lo, hi) = ([-43.0, -43.0, 0.0], [43.0, 43.0, 23.0]);
    let rr = removal::mill_with(s, &tl, lo, hi, 0.5, TipForm::Chained).unwrap();
    let mut gaps = Vec::new();
    for form in [TipForm::Affine, TipForm::Closed] {
        let r = removal::mill_with(s, &tl, lo, hi, 0.5, form).unwrap();
        assert_eq!(r.map.z.len(), rr.map.z.len());
        let worst = r
            .map
            .z
            .iter()
            .zip(&rr.map.z)
            .map(|(x, y)| (x - y).abs())
            .fold(0.0, f64::max);
        // a height moves with the tip and the nearest point, except at a node the cutter's rim
        // only grazes, where it is not continuous in either (docs); on these grids none is
        assert!(
            worst <= max_h,
            "{form:?}: heights differ by {worst:e} > {max_h:e}"
        );
        assert_eq!(r.rapid_cuts.len(), rr.rapid_cuts.len(), "{form:?}");
        assert_eq!(r.holder_hits.len(), rr.holder_hits.len(), "{form:?}");
        gaps.push(worst);
    }
    // the program cuts: the map is not the stock
    assert!(rr.map.z.iter().any(|z| *z < 23.0));
    println!(
        "tips: largest gap {worst_tip:e}, bound {max_bound:e}; heights against the reference: affine {:e}, closed {:e}, bound {max_h:e}",
        gaps[0], gaps[1]
    );
}

/// Arcs that cut fresh stock: part and full turns, both senses, off the origin, by I/J and R, at
/// several depths, with lines between.
fn rings() -> String {
    let mut p = String::from("G43 H1\nG0 X0 Y0 Z30.\n");
    // from (x, y), about the centre (x + i, y + j), through `turn` radians (+ ccw): the word
    // block and the end
    let arc = |x: f64, y: f64, i: f64, j: f64, turn: f64| {
        let (cx, cy) = (x + i, y + j);
        let a = dmath::atan2(y - cy, x - cx) + turn;
        let r = (i * i + j * j).sqrt();
        let (sn, cs) = dmath::sin_cos(a);
        let (ex, ey) = (cx + r * cs, cy + r * sn);
        let g = if turn > 0.0 { 3 } else { 2 };
        let (ex, ey) = ((ex * 1000.0).round() / 1000.0, (ey * 1000.0).round() / 1000.0);
        (format!("G{g} X{ex:.3} Y{ey:.3} I{i:.3} J{j:.3} F900.\n"), ex, ey)
    };
    for (k, z) in [21.0, 17.5, 13.25].iter().enumerate() {
        let o = 7.0 * k as f64 - 7.0;
        let (mut x, mut y) = (12.0 + o, o);
        p.push_str(&format!("G0 X{x:.3} Y{y:.3}\nG1 Z{z:.3} F500.\n"));
        for (i, j, turn) in [
            (-12.0, 0.0, 2.1),
            (3.5, -8.25, -1.3),
            (-6.0, 2.5, 2.0 * dmath::PI),
            (8.25, -3.5, -2.7),
            (1.2, -14.7, 0.9),
        ] {
            let (b, ex, ey) = arc(x, y, i, j, turn);
            p.push_str(&b);
            (x, y) = (ex, ey);
            p.push_str(&format!("G1 X{:.3} Y{:.3}\n", x - 2.5, y + 1.5));
            (x, y) = (x - 2.5, y + 1.5);
        }
        p.push_str("G0 Z30.\n");
    }
    p
}

#[test]
fn the_affine_tip_and_closed_form_are_within_their_bounds_on_the_three_axis_mill() {
    let m = builtin_named("generic-vmc3").unwrap();
    let mut s = Setup::new(&m, Control::for_dialect(&m, "fanuc", 1).unwrap()).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -500.0])];
    s.h = vec![(1, 100.0)];
    s.tools = vec![tool()];
    s.start_tool = Some(1);
    compare(&s, &pocket(), 200);
    compare(&s, &rings(), 15);
}

#[test]
fn the_affine_tip_and_closed_form_are_within_their_bounds_on_the_trunnion_turned_on_c() {
    let m = builtin_named("generic-trunnion-bc").unwrap();
    let mut s = Setup::new(&m, Control::for_dialect(&m, "fanuc", 1).unwrap()).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -400.0, 0.0, 0.0])];
    s.h = vec![(1, 100.0)];
    s.tools = vec![tool()];
    s.start_tool = Some(1);
    // C at an angle whose sine and cosine are not exact, from the start: the columns are rotated
    // axes
    s.start[m.joint_index("C").unwrap()] = -121.25;
    compare(&s, &pocket(), 200);
    compare(&s, &rings(), 15);
}
