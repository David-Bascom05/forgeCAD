//! The thermoforming scoreboard: every model in examples/thermoform/validation formed and compared
//! with its measured (or exact) file, as cases.toml says, under each contact option asked for. The
//! data stay outside the repo (THERMO_VALIDATION_DATA, default ~/projects/forgecad-gate/thermo-fea/
//! validation); without them the test says so and passes. Long: run with
//! `cargo test --release -p forge-lang --test thermo_validation -- --ignored`.
//! THERMO_CONFIGS (default "height,height_fine"; OPTION[_fine][_nh][:mu]), THERMO_CASES (id
//! prefixes, default all), THERMO_SCOREBOARD_LABEL (default "run") and
//! THERMO_SCOREBOARD_OUT (default the data folder's ../scoreboard) steer it. Writes LABEL.csv,
//! LABEL.json, LABEL.md and LABEL/profiles/CASE_CONFIG.csv there.
use forge_lang::thermo::ThermoOut;
use forge_lang::{Options, Run};
use forge_thermo::{Process, Stretch};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
enum Val {
    S(String),
    N(f64),
    B(bool),
    L(Vec<f64>),
}

type Case = BTreeMap<String, Val>;

/// The cases file: `[[case]]` tables of flat `key = value` (string, number, bool, number list).
fn cases(text: &str) -> Vec<Case> {
    let mut out: Vec<Case> = vec![];
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        if l == "[[case]]" {
            out.push(Case::new());
            continue;
        }
        let Some((k, v)) = l.split_once('=') else { continue };
        let v = v.trim();
        let val = if let Some(s) = v.strip_prefix('"') {
            Val::S(s.trim_end_matches('"').to_string())
        } else if v == "true" || v == "false" {
            Val::B(v == "true")
        } else if let Some(s) = v.strip_prefix('[') {
            Val::L(
                s.trim_end_matches(']')
                    .split(',')
                    .filter_map(|x| x.trim().parse().ok())
                    .collect(),
            )
        } else {
            Val::N(v.parse().expect("a number"))
        };
        out.last_mut()
            .expect("[[case]] first")
            .insert(k.trim().to_string(), val);
    }
    out
}

fn s<'a>(c: &'a Case, k: &str) -> &'a str {
    match c.get(k) {
        Some(Val::S(v)) => v,
        _ => "",
    }
}
fn n(c: &Case, k: &str, d: f64) -> f64 {
    match c.get(k) {
        Some(Val::N(v)) => *v,
        _ => d,
    }
}
fn b(c: &Case, k: &str) -> bool {
    matches!(c.get(k), Some(Val::B(true)))
}
fn l(c: &Case, k: &str) -> Vec<f64> {
    match c.get(k) {
        Some(Val::L(v)) => v.clone(),
        _ => vec![],
    }
}

/// A CSV's rows (after `#` comments) as column name to text.
fn table(path: &Path) -> Vec<BTreeMap<String, String>> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut lines = text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty());
    let head: Vec<String> = lines
        .next()
        .expect("header")
        .split(',')
        .map(|h| h.trim().to_string())
        .collect();
    lines
        .map(|l| {
            head.iter()
                .cloned()
                .zip(l.split(',').map(|x| x.trim().to_string()))
                .collect()
        })
        .collect()
}

/// What the model is run with, past the language's own run: the solver's options.
/// `height` is the language's default (the height field, 56 nodes along the window); `_fine`
/// doubles the nodes; `_nh` swaps the law for a neo-Hookean one of the same small-strain shear
/// (how much the law's shape matters). Options this solver lacks return false (the row: "n/a").
fn configure(p: &mut Process, cfg: &str, _mu: f64) -> bool {
    let mut base = cfg;
    if let Some(b) = base.strip_suffix("_nh") {
        p.law = forge_thermo::Law::neo_hookean(p.law.shear());
        base = b;
    }
    if let Some(b) = base.strip_suffix("_fine") {
        p.resolution *= 2;
        base = b;
    }
    base == "height"
}

/// The sheet's nodes at the start (the grid), from the first step.
fn start(st: &Stretch) -> Vec<[f64; 3]> {
    st.steps[0]
        .nodes
        .iter()
        .map(|p| [p[0] as f64, p[1] as f64, p[2] as f64])
        .collect()
}

/// The grid node nearest the axis (x = y = 0) at the start.
fn centre(st: &Stretch) -> (usize, usize) {
    let (nx, _) = st.grid;
    let k = start(st)
        .iter()
        .enumerate()
        .min_by(|a, b| {
            let d = |p: &[f64; 3]| p[0] * p[0] + p[1] * p[1];
            d(a.1).total_cmp(&d(b.1))
        })
        .expect("nodes")
        .0;
    (k % (nx + 1), k / (nx + 1))
}

/// The grid nodes along the cut from the centre outward (0: +x row, 45: diagonal, 90: +y column).
fn cut_nodes(st: &Stretch, deg: f64) -> Vec<usize> {
    let (nx, ny) = st.grid;
    let (ci, cj) = centre(st);
    let (di, dj) = match deg.round() as i64 {
        45 => (1, 1),
        90 => (0, 1),
        _ => (1, 0),
    };
    let mut out = vec![];
    let (mut i, mut j) = (ci, cj);
    while i <= nx && j <= ny {
        out.push(j * (nx + 1) + i);
        i += di;
        j += dj;
    }
    out
}

/// Along the cut: (arc length from the centre, thickness, z, horizontal distance from the axis).
fn profile(st: &Stretch, t0: f64, deg: f64) -> Vec<[f64; 4]> {
    let mut s = 0.0;
    let mut prev: Option<forge_math::Point3> = None;
    cut_nodes(st, deg)
        .into_iter()
        .map(|k| {
            let p = st.nodes[k];
            if let Some(q) = prev {
                s += (p - q).length();
            }
            prev = Some(p);
            [s, st.node_ratio[k] * t0, p.z, (p.x * p.x + p.y * p.y).sqrt()]
        })
        .collect()
}

fn interp(xs: &[f64], ys: &[f64], x: f64) -> Option<f64> {
    if xs.is_empty() || x < xs[0] - 1e-9 || x > xs[xs.len() - 1] + 1e-9 {
        return None;
    }
    for w in 0..xs.len().saturating_sub(1) {
        if x <= xs[w + 1] {
            let f = (x - xs[w]) / (xs[w + 1] - xs[w]).max(1e-12);
            return Some(ys[w] + f.clamp(0.0, 1.0) * (ys[w + 1] - ys[w]));
        }
    }
    ys.last().copied()
}

#[derive(Default, Clone)]
struct Score {
    n: usize,
    mean: f64,
    max: f64,
    bias: f64,
    thin: Option<f64>,
    thin_loc: Option<f64>,
    drawin: Option<f64>,
    note: String,
    /// (x, measured, model) for the profile file.
    rows: Vec<(f64, f64, f64)>,
}

fn status(sc: &Score) -> &'static str {
    if sc.n == 0 {
        return "no data";
    }
    let worst = sc.mean.max(sc.thin.map_or(0.0, f64::abs));
    if worst <= 5.0 {
        "pass"
    } else if worst <= 10.0 {
        "marginal"
    } else {
        "fail"
    }
}

/// Measured (x, t) against the model's (x, t) curve: errors as % of t0, the thinnest point, and the
/// material either holds over the compared span when the part is turned about its axis.
fn compare(meas: &[(f64, f64)], mx: &[f64], mt: &[f64], mr: Option<&[f64]>, t0: f64) -> Score {
    let mut sc = Score::default();
    let mut errs = vec![];
    for &(x, t) in meas {
        if let Some(p) = interp(mx, mt, x) {
            errs.push(p - t);
            sc.rows.push((x, t, p));
        }
    }
    sc.n = errs.len();
    if sc.n == 0 {
        return sc;
    }
    sc.mean = errs.iter().map(|e| e.abs()).sum::<f64>() / sc.n as f64 / t0 * 100.0;
    sc.max = errs.iter().fold(0.0f64, |a, e| a.max(e.abs())) / t0 * 100.0;
    sc.bias = errs.iter().sum::<f64>() / sc.n as f64 / t0 * 100.0;
    let (x0, x1) = (sc.rows[0].0, sc.rows[sc.rows.len() - 1].0);
    let m = sc.rows.iter().min_by(|a, b| a.1.total_cmp(&b.1)).expect("rows");
    sc.thin = Some((m.2 - m.1) / t0 * 100.0);
    // the model's own thinnest on the compared span
    let pm = mx
        .iter()
        .zip(mt)
        .filter(|(x, _)| **x >= x0 - 1e-9 && **x <= x1 + 1e-9)
        .min_by(|a, b| a.1.total_cmp(b.1));
    sc.thin_loc = pm.map(|(x, _)| x - m.0);
    if let Some(r) = mr {
        let (mut vm, mut vp) = (0.0, 0.0);
        let mut sorted = sc.rows.clone();
        sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
        for w in sorted.windows(2) {
            let (ra, rb) = (
                interp(mx, r, w[0].0).unwrap_or(0.0),
                interp(mx, r, w[1].0).unwrap_or(0.0),
            );
            let dx = w[1].0 - w[0].0;
            vm += 0.5 * (w[0].1 * ra + w[1].1 * rb) * dx;
            vp += 0.5 * (w[0].2 * ra + w[1].2 * rb) * dx;
        }
        if vp > 0.0 {
            sc.drawin = Some((vm - vp) / vp * 100.0);
        }
    }
    sc
}

fn rows_of(c: &Case, data: &Path) -> Vec<BTreeMap<String, String>> {
    let f = s(c, "filter");
    table(data)
        .into_iter()
        .filter(|r| {
            f.is_empty() || {
                let (k, v) = f.split_once('=').expect("col=value");
                r.get(k).is_some_and(|x| x == v)
            }
        })
        .collect()
}

fn num(r: &BTreeMap<String, String>, k: &str) -> Option<f64> {
    r.get(k).and_then(|v| v.parse().ok())
}

/// A thickness profile case (arc, arc_edge, radial, index).
fn score_profile(c: &Case, data: &Path, st: &Stretch, tool_female: bool) -> Score {
    let t0 = n(c, "t0", 1.0);
    let mode = s(c, "mode");
    let pr = profile(st, t0, n(c, "cut_deg", 0.0));
    let mut meas: Vec<(f64, f64)> = rows_of(c, data)
        .iter()
        .filter_map(|r| Some((num(r, s(c, "s_col"))?, num(r, s(c, "t_col"))?)))
        .collect();
    if b(c, "fold") {
        let total = meas.iter().fold(0.0f64, |a, m| a.max(m.0));
        meas.iter_mut().for_each(|m| m.0 = m.0.min(total - m.0));
    }
    if let Some(Val::N(about)) = c.get("fold_about") {
        meas.iter_mut().for_each(|m| m.0 = (m.0 - about).abs());
    }
    if mode == "index" {
        let map = l(c, "index_map");
        let (ix, sx): (Vec<f64>, Vec<f64>) = map.chunks(2).map(|p| (p[0], p[1])).unzip();
        meas = meas
            .into_iter()
            .filter_map(|(i, t)| Some((interp(&ix, &sx, i)?, t)))
            .collect();
    }
    let mut xs: Vec<f64> = pr.iter().map(|p| p[0]).collect();
    let ts: Vec<f64> = pr.iter().map(|p| p[1]).collect();
    let rs: Vec<f64> = pr.iter().map(|p| p[3]).collect();
    if mode == "arc_edge" {
        // the clamp's own point is t0 by construction, not a prediction
        meas.retain(|m| m.0 > 1e-9);
        let total = xs.last().copied().unwrap_or(0.0);
        let mut v: Vec<(f64, f64, f64)> = xs
            .iter()
            .zip(&ts)
            .zip(&rs)
            .map(|((x, t), r)| (total - x, *t, *r))
            .collect();
        v.reverse();
        xs = v.iter().map(|p| p.0).collect();
        let ts: Vec<f64> = v.iter().map(|p| p.1).collect();
        return compare(&meas, &xs, &ts, None, t0);
    }
    if mode == "radial" {
        // the horizontal distance grows along the cut; keep it monotone
        let mut r = 0.0f64;
        let xr: Vec<f64> = rs
            .iter()
            .map(|x| {
                r = r.max(*x);
                r
            })
            .collect();
        return compare(&meas, &xr, &ts, None, t0);
    }
    // a female part ends at its rim (the sheet on the tool's top past it): compare up to there
    let mut upto = xs.len();
    if tool_female && let Some(k) = pr.iter().position(|p| p[2] > -0.02 * t0) {
        upto = k + 1;
    }
    let before = meas.len();
    meas.retain(|m| m.0 <= xs[upto - 1] + 1e-9);
    let mut sc = compare(
        &meas,
        &xs[..upto],
        &ts[..upto],
        b(c, "axisym").then_some(&rs[..upto]),
        t0,
    );
    if before > meas.len() {
        sc.note = format!(
            "{} rim/flange points past the model's rim not compared",
            before - meas.len()
        );
    }
    sc
}

/// The part's mean wall over its sheet below the rim plane, over t0.
fn part_mean(st: &Stretch) -> f64 {
    let x0 = start(st);
    let (mut a0, mut a) = (0.0, 0.0);
    for (v, r) in st.tris.iter().zip(&st.ratio) {
        let z = v.iter().map(|&k| st.nodes[k as usize].z).sum::<f64>() / 3.0;
        if z > -0.05 {
            continue;
        }
        let p = |k: u32| x0[k as usize];
        let (p0, p1, p2) = (p(v[0]), p(v[1]), p(v[2]));
        let e1 = [p1[0] - p0[0], p1[1] - p0[1]];
        let e2 = [p2[0] - p0[0], p2[1] - p0[1]];
        let ar = 0.5 * (e1[0] * e2[1] - e1[1] * e2[0]).abs();
        a0 += ar;
        a += ar / r;
    }
    if a > 0.0 { a0 / a } else { f64::NAN }
}

fn score_mean(c: &Case, data: &Path, st: &Stretch) -> Score {
    let m = part_mean(st);
    let mut sc = Score::default();
    let mut errs = vec![];
    for r in rows_of(c, data) {
        let (Some(t0), Some(t)) = (num(&r, s(c, "s_col")), num(&r, s(c, "t_col"))) else {
            continue;
        };
        let e = (m - t / t0) * 100.0;
        errs.push(e);
        sc.rows.push((t0, t / t0, m));
    }
    sc.n = errs.len();
    if sc.n > 0 {
        sc.mean = errs.iter().map(|e| e.abs()).sum::<f64>() / sc.n as f64;
        sc.max = errs.iter().fold(0.0f64, |a, e| a.max(e.abs()));
        sc.bias = errs.iter().sum::<f64>() / sc.n as f64;
        sc.note = format!("model mean {:.3} t0 over the part below the rim", m);
    }
    sc
}

/// Each vacuum load step: (pressure, the centre node's depth below the rim plane, its thickness ratio).
fn pole_steps(st: &Stretch, p_end: f64) -> Vec<(f64, f64, f64)> {
    let (nx, _) = st.grid;
    let (ci, cj) = centre(st);
    let k = cj * (nx + 1) + ci;
    st.steps
        .iter()
        .filter(|s| s.stage == "vacuum")
        .enumerate()
        .map(|(i, s)| {
            (
                p_end * (i + 1) as f64 / 10.0,
                -(s.nodes[k][2] as f64),
                s.ratio[k] as f64,
            )
        })
        .collect()
}

fn score_bubble(c: &Case, data: &Path, st: &Stretch, p_end: f64) -> Score {
    let a = n(c, "a", 1.0);
    let hs = n(c, "h_scale", 1.0);
    let mut pts: Vec<(f64, f64)> = rows_of(c, data)
        .iter()
        .filter_map(|r| Some((num(r, s(c, "s_col"))?, num(r, s(c, "t_col"))? * hs)))
        .collect();
    // the rising branch only
    if let Some(k) = pts.windows(2).position(|w| w[1].0 < w[0].0) {
        pts.truncate(k + 1);
    }
    let (px, hx): (Vec<f64>, Vec<f64>) = pts.into_iter().unzip();
    let mut sc = Score::default();
    let mut errs = vec![];
    for (p, h, _) in pole_steps(st, p_end) {
        if let Some(hm) = interp(&px, &hx, p) {
            errs.push((h - hm) / a * 100.0);
            sc.rows.push((p, hm, h));
        }
    }
    sc.n = errs.len();
    if sc.n > 0 {
        sc.mean = errs.iter().map(|e| e.abs()).sum::<f64>() / sc.n as f64;
        sc.max = errs.iter().fold(0.0f64, |a, e| a.max(e.abs()));
        sc.bias = errs.iter().sum::<f64>() / sc.n as f64;
        sc.note = "errors are pole height, % of the clamp radius".into();
    }
    sc
}

fn score_ref_inflation(c: &Case, data: &Path, st: &Stretch, p_end: f64) -> Score {
    let (a, t0, ss) = (n(c, "a", 1.0), n(c, "t0", 1.0), n(c, "S", 1.0));
    let rows = table(data);
    let mut ref_: Vec<(f64, f64, f64)> = rows
        .iter()
        .filter_map(|r| Some((num(r, "P")?, num(r, "H")?, num(r, "t_pole_over_t0")?)))
        .collect();
    if let Some(k) = ref_.windows(2).position(|w| w[1].0 < w[0].0) {
        ref_.truncate(k + 1);
    }
    let pp: Vec<f64> = ref_.iter().map(|r| r.0).collect();
    let hh: Vec<f64> = ref_.iter().map(|r| r.1).collect();
    let tt: Vec<f64> = ref_.iter().map(|r| r.2).collect();
    let mut sc = Score::default();
    let (mut eh, mut et) = (vec![], vec![]);
    for (p, h, r) in pole_steps(st, p_end) {
        let pn = p * a / (ss * t0);
        if let (Some(hr), Some(tr)) = (interp(&pp, &hh, pn), interp(&pp, &tt, pn)) {
            eh.push((h / a - hr) * 100.0);
            et.push((r - tr) * 100.0);
            sc.rows.push((pn, hr, h / a));
        }
    }
    sc.n = eh.len();
    if sc.n > 0 {
        sc.mean = eh.iter().map(|e| e.abs()).sum::<f64>() / sc.n as f64;
        sc.max = eh.iter().fold(0.0f64, |a, e| a.max(e.abs()));
        sc.bias = eh.iter().sum::<f64>() / sc.n as f64;
        // the pole's thickness error at the last step, % of t0
        sc.thin = et.last().copied();
        sc.note = format!(
            "mean/max: pole height H = h/a, % of a; thin: pole t/t0 at the last step, % of t0 (max |t err| {:.2})",
            et.iter().fold(0.0f64, |a, e| a.max(e.abs()))
        );
    }
    sc
}

fn score_corner(c: &Case, data: &Path, st: &Stretch, p_end: f64) -> Score {
    let (d, t0, g) = (n(c, "d", 20.0), n(c, "t0", 1.0), n(c, "G", 0.5));
    let w = d / 2f64.sqrt();
    let rows = table(data);
    let pi: Vec<f64> = rows.iter().filter_map(|r| num(r, "Pi")).collect();
    let rr: Vec<f64> = rows.iter().filter_map(|r| num(r, "R_over_d")).collect();
    let (nx, _) = st.grid;
    let (_, cj) = centre(st);
    let row: Vec<usize> = (0..=nx).map(|i| cj * (nx + 1) + i).collect();
    // the nearest node on the wall to the vertex, as a distance along the wall over d
    let reach = |s: &forge_thermo::Step| -> Option<f64> {
        row.iter()
            .map(|&k| s.nodes[k])
            .filter(|p| {
                let (x, z) = (p[0] as f64, p[2] as f64);
                x.abs() < w && z < -1e-3 && z - (x.abs() - w) <= 0.6 * t0
            })
            .map(|p| ((p[0] as f64).abs().powi(2) + (p[2] as f64 + w).powi(2)).sqrt() / d)
            .min_by(|a, b| a.total_cmp(b))
    };
    let mut sc = Score::default();
    let mut errs = vec![];
    let vac: Vec<&forge_thermo::Step> = st.steps.iter().filter(|s| s.stage == "vacuum").collect();
    for (i, s) in vac.iter().enumerate() {
        let p = p_end * (i + 1) as f64 / 10.0;
        let pin = p * d / (g * t0);
        let r_ref = if pin < 0.380952 {
            Some(1.0)
        } else {
            interp(&pi, &rr, pin)
        };
        let r_mod = reach(s).unwrap_or(1.0);
        if let Some(r) = r_ref {
            errs.push((r_mod - r) * 100.0);
            sc.rows.push((pin, r, r_mod));
        }
    }
    sc.n = errs.len();
    if sc.n > 0 {
        sc.mean = errs.iter().map(|e| e.abs()).sum::<f64>() / sc.n as f64;
        sc.max = errs.iter().fold(0.0f64, |a, e| a.max(e.abs()));
        sc.bias = errs.iter().sum::<f64>() / sc.n as f64;
    }
    // the wall laid at the last load step against (x/d)^k / lambda_c, x from 2R to d
    if let (Some(last), Some(&(pin, r_end, _))) = (vac.last(), sc.rows.last()) {
        let (k, lc) = (4.0 / std::f64::consts::PI - 1.0, 1.110_720_7);
        let mut we = vec![];
        for &kk in &row {
            let p = last.nodes[kk];
            let (x, z) = (p[0] as f64, p[2] as f64);
            if !(x > 0.0 && x < w && z - (x - w) <= 0.6 * t0) {
                continue;
            }
            let along = (x * x + (z + w).powi(2)).sqrt();
            if along >= 2.0 * r_end * d && along <= d {
                we.push((last.ratio[kk] as f64 - (along / d).powf(k) / lc) * 100.0);
            }
        }
        if !we.is_empty() {
            sc.thin = Some(we.iter().sum::<f64>() / we.len() as f64);
        }
        sc.note = format!(
            "mean/max: achieved fillet R/d, % of d, at Pi up to {pin:.2}; thin: mean wall t/t0 error for x >= 2R, % of t0"
        );
    }
    sc
}

struct Model {
    run: Run,
    female: bool,
}

#[test]
#[ignore = "long: the validation scoreboard (see the file's header)"]
fn scoreboard() {
    let home = std::env::var("HOME").unwrap_or_default();
    let data = std::env::var("THERMO_VALIDATION_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(&home).join("projects/forgecad-gate/thermo-fea/validation"));
    if !data.is_dir() {
        eprintln!("thermo validation data not found at {}: skipped", data.display());
        return;
    }
    let out = std::env::var("THERMO_SCOREBOARD_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| data.join("../scoreboard"));
    let label = std::env::var("THERMO_SCOREBOARD_LABEL").unwrap_or_else(|_| "run".into());
    let configs: Vec<String> = std::env::var("THERMO_CONFIGS")
        .unwrap_or_else(|_| "height,height_fine".into())
        .split(',')
        .map(|x| x.trim().to_string())
        .collect();
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/thermoform/validation");
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let only: Vec<String> = std::env::var("THERMO_CASES")
        .map(|v| v.split(',').map(|x| x.trim().to_string()).collect())
        .unwrap_or_default();
    let list: Vec<Case> = cases(&std::fs::read_to_string(dir.join("cases.toml")).expect("cases.toml"))
        .into_iter()
        .filter(|c| only.is_empty() || only.iter().any(|o| s(c, "id").starts_with(o.as_str())))
        .collect();
    std::fs::create_dir_all(out.join(&label).join("profiles")).expect("scoreboard folder");

    let mut models: BTreeMap<String, Model> = BTreeMap::new();
    let mut csv = String::from(
        "label,config,case,mode,n,mean_err_pct,max_err_pct,bias_pct,thin_err_pct,thin_loc_err_mm,drawin_excess_pct,status,cause,note\n",
    );
    let mut json = vec![];
    let mut md = format!(
        "# Thermoforming scoreboard: {label}\n\nErrors in % of t0 unless the note says otherwise (bubbles: % of the clamp radius; corner: % of d). \
         pass: mean and |thin| <= 5 %, marginal <= 10 %, else fail. drawin: material the measured part holds over the model's, same span (axisymmetric cases).\n\n"
    );
    for cfg in &configs {
        md += &format!(
            "## {cfg}\n\n| case | n | mean | max | bias | thin | thin loc mm | drawin | status | cause |\n|---|---|---|---|---|---|---|---|---|---|\n"
        );
        for c in &list {
            let id = s(c, "id").to_string();
            let file = s(c, "model").to_string();
            let m = models.entry(file.clone()).or_insert_with(|| {
                let src = std::fs::read_to_string(dir.join(&file)).expect("model");
                let opts = Options {
                    dir: Some(dir.clone()),
                    ..Options::default()
                };
                let run = forge_lang::run(&src, opts, Default::default()).0;
                let female = run
                    .thermos
                    .first()
                    .is_some_and(|t| t.tool == forge_thermo::Tool::Female);
                Model { run, female }
            });
            let d = s(c, "data");
            let data_file = match d.strip_prefix("repo:") {
                Some(p) => repo.join(p),
                None => data.join(d),
            };
            let th: Option<&ThermoOut> = m.run.thermos.first().filter(|_| m.run.errors.is_empty());
            // `slide:0.05` overrides the case's friction coefficient
            let (opt, mu) = match cfg.split_once(':') {
                Some((o, m)) => (o, m.parse().expect("cfg:mu")),
                None => (cfg.as_str(), n(c, "mu", 0.3)),
            };
            let (sc, st_label) = match th.and_then(|t| Some((t, t.process.clone()?))) {
                None => (None, "build error"),
                Some((t, mut p)) => {
                    if !configure(&mut p, opt, mu) {
                        (None, "n/a")
                    } else {
                        let fresh;
                        let st: &Stretch = if Some(&p) == t.process.as_ref() {
                            t.walls.as_deref().expect("walls")
                        } else {
                            fresh = forge_thermo::stretch(&t.formed, &t.spec, &p);
                            &fresh
                        };
                        let sc = match s(c, "mode") {
                            "mean" => score_mean(c, &data_file, st),
                            "bubble" => score_bubble(c, &data_file, st, p.pressure),
                            "ref_inflation" => score_ref_inflation(c, &data_file, st, p.pressure),
                            "corner" => score_corner(c, &data_file, st, p.pressure),
                            _ => score_profile(c, &data_file, st, m.female),
                        };
                        // the sheet through the tool: the solver lost contact (a robustness fault)
                        let floor = if m.female { t.formed.report.depth } else { 0.0 } + 2.0 * t.t;
                        let blown = st.nodes.iter().filter(|p| p.z < -floor).count();
                        let mut sc = sc;
                        let status = if blown > 0 {
                            sc.note = format!("BLOWN: {blown} nodes through the tool; {}", sc.note);
                            "blown"
                        } else {
                            status(&sc)
                        };
                        (Some(sc), status)
                    }
                }
            };
            let f = |x: Option<f64>| x.map_or(String::new(), |v| format!("{v:.2}"));
            let cause = s(c, "cause");
            let sc = sc.unwrap_or_default();
            csv += &format!(
                "{label},{cfg},{id},{},{},{:.2},{:.2},{:.2},{},{},{},{st_label},\"{cause}\",\"{}\"\n",
                s(c, "mode"),
                sc.n,
                sc.mean,
                sc.max,
                sc.bias,
                f(sc.thin),
                f(sc.thin_loc),
                f(sc.drawin),
                sc.note
            );
            let jn = |x: Option<f64>| x.map_or("null".to_string(), |v| format!("{v:.4}"));
            json.push(format!(
                "{{\"label\":\"{label}\",\"config\":\"{cfg}\",\"case\":\"{id}\",\"mode\":\"{}\",\"n\":{},\"mean_err_pct\":{:.4},\"max_err_pct\":{:.4},\"bias_pct\":{:.4},\"thin_err_pct\":{},\"thin_loc_err_mm\":{},\"drawin_excess_pct\":{},\"status\":\"{st_label}\",\"cause\":\"{cause}\",\"note\":\"{}\"}}",
                s(c, "mode"),
                sc.n,
                sc.mean,
                sc.max,
                sc.bias,
                jn(sc.thin),
                jn(sc.thin_loc),
                jn(sc.drawin),
                sc.note.replace('"', "'")
            ));
            md += &format!(
                "| {id} | {} | {:.1} | {:.1} | {:+.1} | {} | {} | {} | {st_label} | {cause} |\n",
                sc.n,
                sc.mean,
                sc.max,
                sc.bias,
                f(sc.thin),
                f(sc.thin_loc),
                f(sc.drawin)
            );
            if !sc.rows.is_empty() {
                let mut p = String::from("x,measured,model\n");
                for (x, a, b) in &sc.rows {
                    p += &format!("{x:.4},{a:.5},{b:.5}\n");
                }
                std::fs::write(
                    out.join(&label).join("profiles").join(format!("{id}_{cfg}.csv")),
                    p,
                )
                .expect("profile");
            }
        }
        md += "\n";
    }
    std::fs::write(out.join(format!("{label}.csv")), &csv).expect("csv");
    std::fs::write(
        out.join(format!("{label}.json")),
        format!("[\n{}\n]\n", json.join(",\n")),
    )
    .expect("json");
    std::fs::write(out.join(format!("{label}.md")), &md).expect("md");
    eprintln!(
        "scoreboard {label}: {} cases x {} configs -> {}",
        list.len(),
        configs.len(),
        out.display()
    );
}
