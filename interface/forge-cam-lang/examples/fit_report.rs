//! The fitter's effect on 3D finishing (forge_cam::fit through forge-cam-3d's linking), per
//! strategy on the mould core, ball-6, cusp 0.01:
//! cargo run --release -p forge-cam-lang --example fit_report [-- strategy ...]
//!
//! Two runs of each op: cusp 0.01 with `fit: 0`, and with the fit's default. Reported: the op's blocks, the mean cutting-move length, the posted program's
//! size from the finishing tool's change on, and on generic-vmc3 the jerk-limited planner's time
//! (forge_machine::lookahead::plan_scurve, the default assumption set) with the share of cutting
//! time spent in blocks whose average speed is at least 90% of the programmed feed (a block's
//! t_min over its planned time), again with an assumed 2 ms block cycle (no published value; a
//! sensitivity). The deviation is the op's own measurement (`fit::deviation`, in its plan): the
//! fitted pieces against the very passes they replace, both ways. Every op must certify
//! (`cam.ok`). The part is given an alloy (6061-T6) for realistic feeds.
use forge_cam::ops::PMove;
use forge_machine::sim::SegKind;

const HEAD: &str = "machining
shared tol = 0.01
f     = part(\"mold_core.fg\", material: \"6061-T6\")
m     = machine(\"generic-vmc3\")
t1    = tool(\"flat-10\", t: 1, stickout: 40, holder: \"bt40-er32\")
t2    = tool(\"ball-6\", t: 2, stickout: 40, holder: \"bt40-er32\")
op1   = setup(f, machine: m, stock: box(margin: (0, 0, 2)), origin: \"top-center\", tol: tol, at: (-200, -150, -250))
rough = zlevel(op1, tool: t1, leave: 0.5, step: 3, stepover: 4, area: (-45, -35, 45, 35), grid: 1)
";

/// (name, the op's arguments with CUSP where the spacing goes, whether it has a step-down)
const OPS: &[(&str, &str, &str)] = &[
    (
        "parallel",
        "parallel(op1, tool: t2, CUSP, stepover: 0.5, area: (-30, -24, 30, 24)",
        "none",
    ),
    (
        "waterline",
        "waterline(op1, tool: t2, CUSP, from: 24, to: 12, step: 0.5, area: (-30, -24, 30, 24), grid: 0.5",
        "none",
    ),
    (
        "steep_shallow",
        "steep_shallow(op1, tool: t2, CUSP, threshold: 45, area: (-30, -24, 30, 24), from: 26, to: 10.5, grid: 1",
        "both",
    ),
    (
        "spiral",
        "spiral(op1, tool: t2, CUSP, area: (-12, -8, 12, 8), r1: 15",
        "over",
    ),
    (
        "radial",
        "radial(op1, tool: t2, CUSP, area: (-12, -8, 12, 8), r0: 1, r1: 15",
        "over",
    ),
    (
        "scallop",
        "scallop(op1, tool: t2, CUSP, area: (-12, -8, 12, 8), grid: 0.5",
        "over",
    ),
    (
        "ramping",
        "ramping(op1, tool: t2, CUSP, area: (-30, -24, 30, 24), from: 24, to: 12, grid: 1",
        "down",
    ),
    (
        "flat",
        "flat(op1, tool: t2, CUSP, area: (-30, -24, 30, 24), to: 9.5, from: 10.5",
        "over",
    ),
    (
        "rest",
        "rest(op1, tool: t2, after: t1, CUSP, area: (-30, -24, 30, 24), grid: 0.5",
        "over",
    ),
    (
        "pencil",
        "pencil(op1, tool: t2, CUSP, area: (-30, -24, 30, 24), grid: 0.5",
        "none",
    ),
];

struct Run {
    ok: bool,
    errs: Vec<String>,
    plan: Vec<String>,
    stepover: f64,
    stepdown: f64,
    blocks: usize,
    mean: f64,
    nc: usize,
    t: f64,
    fast: f64,
    /// t_scurve and the ≥90% share again with an assumed block cycle
    t_bc: f64,
    fast_bc: f64,
    secs: f64,
}

/// The block cycle assumed for the sensitivity columns (s): no published value; a control
/// that takes this long per block at least.
const BLOCK_CYCLE: f64 = 0.002;

fn arc_len(at: [f64; 3], to: [f64; 3], c: [f64; 2], cw: bool) -> f64 {
    let r = ((at[0] - c[0]).powi(2) + (at[1] - c[1]).powi(2)).sqrt();
    let a0 = forge_math::dmath::atan2(at[1] - c[1], at[0] - c[0]);
    let mut a1 = forge_math::dmath::atan2(to[1] - c[1], to[0] - c[0]);
    let tau = std::f64::consts::TAU;
    if cw {
        while a1 >= a0 {
            a1 -= tau;
        }
    } else {
        while a1 <= a0 {
            a1 += tau;
        }
    }
    let s = r * (a1 - a0).abs();
    (s * s + (to[2] - at[2]).powi(2)).sqrt()
}

fn run(op: &str, dir: &std::path::Path) -> Run {
    let doc = format!("{HEAD}fin   = {op})\nprog  = program(op1, post: \"fanuc\", number: 1004)\n");
    let t0 = std::time::Instant::now();
    let cam = forge_cam_lang::run(&doc, dir);
    let secs = t0.elapsed().as_secs_f64();
    let o = cam.ops.iter().find(|o| o.name == "fin").expect("the op");
    if !cam.ok {
        let bad: Vec<_> = o.checks.iter().filter(|c| c.verdict != "clear").take(4).collect();
        eprintln!(
            "  NOT OK: status {} refusal {:?} warnings {:?}\n  bad checks {bad:?}",
            o.status,
            o.refusal,
            &cam.warnings[..cam.warnings.len().min(3)]
        );
    }
    let path = o.paths[0].clone();
    if std::env::var("DIAG").is_ok() {
        // the path's time at feed by phase (length / feed), and its links as the op counts them
        let mut by = std::collections::BTreeMap::new();
        let mut at = path.start;
        for m in &path.moves {
            let (to, f, ph) = match *m {
                forge_cam::ops::PMove::Line { to, f, phase } => (to, f, format!("{phase:?}")),
                forge_cam::ops::PMove::Arc { to, f, .. } => (to, f, "Arc".into()),
                forge_cam::ops::PMove::Rapid { to } => (to, 0.0, "Rapid".into()),
                _ => continue,
            };
            let l = ((to[0] - at[0]).powi(2) + (to[1] - at[1]).powi(2) + (to[2] - at[2]).powi(2)).sqrt();
            let e = by.entry(ph).or_insert((0usize, 0.0f64, 0.0f64));
            e.0 += 1;
            e.1 += l;
            e.2 += if f > 0.0 { l / f * 60.0 } else { 0.0 };
            at = to;
        }
        let links = o
            .plan
            .iter()
            .find(|l| l.starts_with("links:"))
            .cloned()
            .unwrap_or_default();
        eprintln!("  PHASES (moves, mm, s at feed): {by:?}  {links}");
    }
    let fin = o.finish.clone();
    let mut len = 0.0;
    let mut n = 0usize;
    let mut at = path.start;
    for m in &path.moves {
        match *m {
            PMove::Rapid { to } => at = to,
            PMove::Line { to, .. } => {
                len += ((to[0] - at[0]).powi(2) + (to[1] - at[1]).powi(2) + (to[2] - at[2]).powi(2)).sqrt();
                n += 1;
                at = to;
            }
            PMove::Arc { to, c, cw, .. } => {
                len += arc_len(at, to, c, cw);
                n += 1;
                at = to;
            }
            _ => {}
        }
    }
    let (mut nc, mut t, mut fast, mut t_bc, mut fast_bc) = (0, 0.0, 0.0, 0.0, 0.0);
    match cam.machine_run(0) {
        Ok(mr) => {
            let from = mr.text.find("T2").unwrap_or(0);
            nc = mr.text.len() - from;
            let tally = |per: &[f64]| {
                let (mut t, mut fast) = (0.0, 0.0);
                for (g, ts) in mr.tl.segments.iter().zip(per) {
                    if g.tool == Some(2) && !matches!(g.kind, SegKind::Rapid) {
                        t += ts;
                        if g.t_min >= 0.9 * ts {
                            fast += ts;
                        }
                    }
                }
                (t, fast)
            };
            (t, fast) = tally(&mr.tl.seg_scurve);
            if std::env::var("DIAG").is_ok() {
                let mut by = std::collections::BTreeMap::new();
                // the highest the tool goes (its links' height)
                let ztop = mr
                    .tl
                    .segments
                    .iter()
                    .filter(|g| g.tool == Some(2) && g.q0.len() >= 3)
                    .map(|g| g.q0[2].max(g.q1[2]))
                    .fold(f64::NEG_INFINITY, f64::max);
                for (g, ts) in mr.tl.segments.iter().zip(&mr.tl.seg_scurve) {
                    if g.tool == Some(2) {
                        // a feed straight down or up (a plunge, or a lift): a link's, not a cut's
                        let vert = matches!(g.kind, SegKind::Feed)
                            && g.q0.len() >= 3
                            && (g.q1[0] - g.q0[0]).abs() < 1e-6
                            && (g.q1[1] - g.q0[1]).abs() < 1e-6
                            && (g.q1[2] - g.q0[2]).abs() > 1e-9;
                        let across = matches!(g.kind, SegKind::Feed)
                            && g.q0.len() >= 3
                            && g.q0[2] >= ztop - 1e-6
                            && g.q1[2] >= ztop - 1e-6;
                        let k = if across {
                            "Across".to_string()
                        } else if vert {
                            if g.q1[2] < g.q0[2] {
                                "Plunge".to_string()
                            } else {
                                "Lift".to_string()
                            }
                        } else {
                            format!("{:?}", g.kind)
                        };
                        let e = by.entry(k).or_insert((0usize, 0.0, 0.0));
                        e.0 += 1;
                        e.1 += g.t_min;
                        e.2 += ts;
                    }
                }
                eprintln!("  DIAG (count, t_min, t_scurve) by kind: {by:?}");
            }
            let mut sc = forge_machine::lookahead::SCurve::for_machine(&mr.def);
            sc.block_cycle = Some(BLOCK_CYCLE);
            let (per, _) = forge_machine::lookahead::plan_scurve(&mr.def, &mr.tl, &sc);
            (t_bc, fast_bc) = tally(&per);
        }
        Err(e) => eprintln!("  machine_run: {e}"),
    }
    Run {
        ok: cam.ok,
        errs: cam.errors.clone(),
        plan: o.plan.clone(),
        stepover: fin.as_ref().map_or(0.0, |f| f.stepover),
        stepdown: fin.as_ref().map_or(0.0, |f| f.stepdown),
        blocks: path.moves.len(),
        mean: len / n.max(1) as f64,
        nc,
        t,
        fast: if t > 0.0 { fast / t } else { 0.0 },
        t_bc,
        fast_bc: if t_bc > 0.0 { fast_bc / t_bc } else { 0.0 },
        secs,
    }
}

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let only: Vec<String> = std::env::args().skip(1).collect();
    println!(
        "| strategy | fit | ok | blocks | mean move mm | NC bytes | t_scurve s | ≥90% F | t, 2 ms block s | ≥90% F, 2 ms | stepover / step mm | fitted as | max dev mm | gen s |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for &(name, op, _) in OPS {
        if !only.is_empty() && !only.iter().any(|o| o == name) {
            continue;
        }
        let cusp = if matches!(name, "pencil" | "parallel" | "waterline") {
            ""
        } else {
            "cusp: 0.01, "
        };
        // (parallel and waterline take no `fit` here: both runs are the same op)
        let unfit = if matches!(name, "parallel" | "waterline") {
            ""
        } else {
            "fit: 0, "
        };
        let a = run(&op.replace("CUSP, ", &format!("{cusp}{unfit}")), &dir);
        // FIT=<mm> overrides the default fit for the second run
        let fit = std::env::var("FIT")
            .map(|f| format!("fit: {f}, "))
            .unwrap_or_default();
        let b = run(&op.replace("CUSP, ", &format!("{cusp}{fit}")), &dir);
        for (tag, r) in [("0", &a), ("default", &b)] {
            let fitline = r
                .plan
                .iter()
                .find(|l| l.starts_with("fit within"))
                .cloned()
                .unwrap_or_default();
            // "fit within T mm (...): N pass points as L lines and A arcs, measured D mm ..."
            let (as_, dev) = match fitline.split_once(": ") {
                Some((_, rest)) => {
                    let (pieces, m) = rest.split_once(", measured ").unwrap_or((rest, ""));
                    let tol = fitline
                        .trim_start_matches("fit within ")
                        .split(' ')
                        .next()
                        .unwrap_or("");
                    (
                        format!("{tol} mm: {pieces}"),
                        m.split(' ').next().unwrap_or("").to_string(),
                    )
                }
                None => (String::new(), String::new()),
            };
            println!(
                "| {name} | {tag} | {} | {} | {:.3} | {} | {:.1} | {:.0}% | {:.1} | {:.0}% | {:.4} / {:.4} | {as_} | {dev} | {:.1} |",
                r.ok,
                r.blocks,
                r.mean,
                r.nc,
                r.t,
                100.0 * r.fast,
                r.t_bc,
                100.0 * r.fast_bc,
                r.stepover,
                r.stepdown,
                r.secs
            );
            if !r.ok {
                eprintln!("  {name} fit {tag}: {:?}", &r.errs[..r.errs.len().min(3)]);
            }
        }
    }
}
