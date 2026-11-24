//! A lookahead time for a timeline: what a control that blends moves within the acceleration
//! limits takes, between `t_min` (no acceleration limit at all) and `t_stop` (an exact stop at
//! every block). The planner is grbl's, restated from its source (gnea/grbl v1.1h.20190825,
//! grbl/planner.c, motion_control.c, nuts_bolts.c; GPL-3, read and restated, not copied):
//!
//! - Each block's acceleration and its nominal speed are limited by every axis along its
//!   direction: a = min_j accel_j / |u_j| (`limit_value_by_axis_maximum`), u the block's unit
//!   vector in axis space (mm and degrees together, as grbl treats ABC axes).
//! - A junction's speed (`plan_buffer_line`): with cos θ = −u_prev·u (θ the angle between the
//!   reversed previous direction and the new one) and a_j the acceleration limit along u − u_prev,
//!   v² = a_j δ sin(θ/2) / (1 − sin(θ/2)), δ the junction deviation; a straight continuation is
//!   unlimited, a full reversal 0. A block's entry is also at most its own and the previous block's
//!   nominal speed (`plan_compute_profile_parameters`).
//! - The plan (`planner_recalculate`): a reverse pass from the newest block in the buffer, planned
//!   to stop there, then a forward pass; each block a trapezoid between its entry and exit speeds.
//!   Here: as a program streams, the block being run has the next `blocks − 1` in the buffer
//!   behind it, so its exit is the reverse pass's over that window (ending at rest) and the
//!   forward limit from its entry; its entry is the previous block's exit.
//! - Arcs (`mc_arc`): chords, floor(|½ sweep r| / √(tol (2r − tol))) of them (at least one), each a
//!   block.
//!
//! The nominal speed of each block is this simulation's own feed bound (the segment's `t_min`),
//! so every feed, rapid and clamp rule `t_min` applies holds here too. The plan stops (exit 0)
//! wherever the timeline does something other than move continuously: a tool change, a dwell, a
//! spindle or coolant change (each a synchronisation in grbl), a jump in position or time, and the
//! program's ends.
//!
//! It is an estimate of one class of controls (grbl's, with its buffer and junction deviation),
//! not a bound: a control with a longer lookahead or jerk limits differs. Its parameters are
//! inputs so a comparison can be run over a range of them; the defaults are grbl's.
use crate::def::{JointType, MachineDef};
use crate::sim::{Path, Timeline};

/// The planner's inputs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LookAhead {
    /// Every joint's acceleration times this (a sensitivity knob; 1: the record's).
    pub accel_scale: f64,
    /// Junction deviation δ (mm; grbl's default 0.01, `DEFAULT_JUNCTION_DEVIATION`).
    pub junction_deviation: f64,
    /// Blocks planned at once, the one being run included (grbl: a 16-slot ring buffer, one kept
    /// empty: 15).
    pub blocks: usize,
    /// The chordal tolerance arcs are cut into blocks by (mm; grbl's `DEFAULT_ARC_TOLERANCE`
    /// 0.002).
    pub arc_tolerance: f64,
    /// The acceleration taken for a joint whose record has none: linear (mm/s²) and rotary
    /// (deg/s²). Stated, the same for every program; reported by `Timeline::accel_missing`.
    pub default_accel: [f64; 2],
}

impl Default for LookAhead {
    fn default() -> Self {
        LookAhead {
            accel_scale: 1.0,
            junction_deviation: 0.01,
            blocks: 15,
            arc_tolerance: 0.002,
            // conservative for a machining centre: about 0.1 g, and 10 rev/s² on a rotary axis
            // (a lower acceleration makes every stop and corner cost more)
            default_accel: [1000.0, 3600.0],
        }
    }
}

/// One planner block: its length (axis units), unit vector, nominal speed, acceleration, and
/// the segment it belongs to.
struct Blk {
    len: f64,
    u: Vec<f64>,
    v_nom: f64,
    acc: f64,
    seg: usize,
}

/// The acceleration limit along a direction: min_j accel_j / |u_j|.
fn limit(acc: &[f64], u: &[f64]) -> f64 {
    let mut a = f64::INFINITY;
    for (aj, uj) in acc.iter().zip(u) {
        if *uj != 0.0 {
            a = a.min(aj / uj.abs());
        }
    }
    a
}

/// Time over a block of length l, from speed v0 to v1, at most vn, acceleration a.
fn block_time(l: f64, v0: f64, v1: f64, vn: f64, a: f64) -> f64 {
    if l <= 0.0 {
        return 0.0;
    }
    if !a.is_finite() {
        return if vn.is_finite() { l / vn } else { 0.0 };
    }
    let d_acc = (vn * vn - v0 * v0).max(0.0) / (2.0 * a);
    let d_dec = (vn * vn - v1 * v1).max(0.0) / (2.0 * a);
    if vn.is_finite() && d_acc + d_dec <= l {
        (vn - v0).max(0.0) / a + (vn - v1).max(0.0) / a + (l - d_acc - d_dec) / vn
    } else {
        let vp = ((2.0 * a * l + v0 * v0 + v1 * v1) / 2.0).sqrt();
        (vp - v0).max(0.0) / a + (vp - v1).max(0.0) / a
    }
}

/// The lookahead time of every segment of `tl` (s), and their sum.
pub fn plan(def: &MachineDef, tl: &Timeline, p: &LookAhead) -> (Vec<f64>, f64) {
    let n = def.joints.len();
    let acc: Vec<f64> = def
        .joints
        .iter()
        .map(|j| {
            let a = j.accel.unwrap_or(match j.ty {
                JointType::Linear => p.default_accel[0],
                JointType::Rotary => p.default_accel[1],
            });
            a * p.accel_scale
        })
        .collect();
    let mut per = vec![0.0; tl.segments.len()];
    // runs of blocks between stops
    let mut run: Vec<Blk> = Vec::new();
    let flush = |run: &mut Vec<Blk>, per: &mut Vec<f64>| {
        let times = plan_run(run, p, &acc);
        for (b, t) in run.iter().zip(times) {
            per[b.seg] += t;
        }
        run.clear();
    };
    for (si, sg) in tl.segments.iter().enumerate() {
        if si > 0 {
            let pv = &tl.segments[si - 1];
            let gap_t = (sg.t0 - (pv.t0 + pv.t_min)).abs() > 1e-9 * sg.t0.abs().max(1.0);
            let gap_q = pv.q1.iter().zip(&sg.q0).any(|(a, b)| (a - b).abs() > 1e-9);
            if gap_t || gap_q || pv.tool != sg.tool || pv.spindle != sg.spindle {
                flush(&mut run, &mut per);
            }
        }
        // the segment's chords: the arc cut as grbl cuts it, a line whole
        let k = match &sg.path {
            Path::Line => 1,
            Path::Arc { r, r1, sweep, .. } => {
                let r = r.max(*r1);
                let tol = p.arc_tolerance;
                if r > tol {
                    ((0.5 * sweep * r).abs() / (tol * (2.0 * r - tol)).sqrt())
                        .floor()
                        .max(1.0) as usize
                } else {
                    1
                }
            }
        };
        let mut pts = Vec::with_capacity(k + 1);
        for i in 0..=k {
            pts.push(if i == 0 {
                sg.q0.clone()
            } else if i == k {
                sg.q1.clone()
            } else {
                sg.at(i as f64 / k as f64)
            });
        }
        let lens: Vec<f64> = pts
            .windows(2)
            .map(|w| (0..n).map(|j| (w[1][j] - w[0][j]).powi(2)).sum::<f64>().sqrt())
            .collect();
        let total: f64 = lens.iter().sum();
        if total == 0.0 {
            continue;
        }
        // the nominal speed: this simulation's feed bound over the segment
        let v_nom = if sg.t_min > 0.0 {
            total / sg.t_min
        } else {
            f64::INFINITY
        };
        for (w, l) in pts.windows(2).zip(lens) {
            if l == 0.0 {
                continue;
            }
            let u: Vec<f64> = (0..n).map(|j| (w[1][j] - w[0][j]) / l).collect();
            let a = limit(&acc, &u);
            run.push(Blk {
                len: l,
                u,
                v_nom,
                acc: a,
                seg: si,
            });
        }
    }
    flush(&mut run, &mut per);
    let total = per.iter().sum();
    (per, total)
}

/// One run's block times: from rest to rest, each block's exit planned over the window of the
/// next `p.blocks − 1`.
fn plan_run(run: &[Blk], p: &LookAhead, jacc: &[f64]) -> Vec<f64> {
    let m = run.len();
    let acc: Vec<f64> = run.iter().map(|b| b.acc).collect();
    // each block's largest entry (squared): its junction with the one before, and both nominal
    // speeds; the first starts from rest
    let mut max_entry2 = vec![0.0f64; m];
    for i in 1..m {
        let (a, b) = (&run[i - 1].u, &run[i].u);
        let cos_t: f64 = -a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
        let vj2 = if cos_t > 0.999999 {
            0.0 // a reversal (grbl: MINIMUM_JUNCTION_SPEED, 0)
        } else if cos_t < -0.999999 {
            f64::INFINITY // straight on
        } else {
            let d: Vec<f64> = b.iter().zip(a).map(|(x, y)| x - y).collect();
            let dl = d.iter().map(|x| x * x).sum::<f64>().sqrt();
            let du: Vec<f64> = d.iter().map(|x| x / dl).collect();
            let aj = limit(jacc, &du);
            let s = (0.5 * (1.0 - cos_t)).sqrt();
            aj * p.junction_deviation * s / (1.0 - s)
        };
        let vn = run[i].v_nom.min(run[i - 1].v_nom);
        max_entry2[i] = vj2.min(vn * vn);
    }
    let w = p.blocks.max(1);
    let mut out = Vec::with_capacity(m);
    let mut entry2 = 0.0f64;
    for i in 0..m {
        let end = (i + w).min(m);
        // reverse pass over the window: the entry each later block can still stop from
        let mut next2 = 0.0f64; // the window ends at rest
        for k in (i + 1..end).rev() {
            next2 = max_entry2[k].min(next2 + 2.0 * acc[k] * run[k].len);
        }
        // block i's exit: the next block's planned entry, and what it can reach from its entry
        let exit2 = if i + 1 < end { next2 } else { 0.0 };
        let exit2 = exit2.min(entry2 + 2.0 * acc[i] * run[i].len);
        let b = &run[i];
        out.push(block_time(b.len, entry2.sqrt(), exit2.sqrt(), b.v_nom, b.acc));
        entry2 = exit2;
    }
    out
}

// ------------------------------------------------------------------------------------------
// A jerk-limited planner (S-curve), for controls that limit jerk: SINUMERIK SOFT ("acceleration
// with jerk limitation", PGAsl 03/2013 10.3.1 p. 457; under it "both the maximum acceleration and
// the maximum jerk of each axis is maintained", PGsl 03/2013 11.2 p. 315), the default of CYCLE832
// with G642 smoothing within the programmed tolerance (PGZ 11/2006 3.17 pp. 264-271; G642: PGsl
// 11.2 p. 309). What it takes from each, and what it assumes:
//
// - Each block's acceleration and jerk are limited by every axis along it (as grbl limits the
//   acceleration, `limit_value_by_axis_maximum`).
// - A speed change from v0 to v1 is a jerk-limited S-curve: time Δv/a + a/j where Δv ≥ a²/j, else
//   2 √(Δv/j); its distance (v0 + v1)/2 × that time (the profile is point-symmetric).
// - A corner is smoothed within the tolerance δ (G642): the blend circle tangent to both blocks
//   whose deviation from the corner is δ (grbl's junction-deviation geometry, planner.c), radius
//   R = δ sin(θ/2) / (1 - sin(θ/2)); its speed √(a R) and (j R²)^(1/3) (the jerk of uniform
//   circular motion, v³/R²).
// - An arc's speed √(a r) and (j r²)^(1/3) (centripetal acceleration and jerk on radius r).
// - The look-ahead spans the run (reverse and forward passes over every block between stops):
//   SINUMERIK's LookAhead plans over many blocks (PGsl 11.2), its depth machine data.
// - Not modelled: the compressor (COMPCAD...), feed-forward, the interpolator's block cycle (a
//   rate limit on very short blocks), axis-specific smoothing filters: the fitted jerk absorbs them.

/// The jerk-limited planner's inputs. The acceleration and jerk apply to every linear (rotary)
/// joint alike; none of them is published for the machines this was built against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SCurve {
    /// mm/s² (linear), deg/s² (rotary).
    pub accel: [f64; 2],
    /// mm/s³, deg/s³.
    pub jerk: [f64; 2],
    /// The smoothing tolerance δ (mm): G642's (CYCLE832's _TOL).
    pub tolerance: f64,
}

/// Time of a jerk-limited speed change by dv at acceleration a, jerk j.
fn sc_time(dv: f64, a: f64, j: f64) -> f64 {
    let dv = dv.abs();
    if dv == 0.0 {
        return 0.0;
    }
    if dv >= a * a / j {
        dv / a + a / j
    } else {
        2.0 * (dv / j).sqrt()
    }
}

/// Distance of that change from v0 to v1.
fn sc_dist(v0: f64, v1: f64, a: f64, j: f64) -> f64 {
    0.5 * (v0 + v1) * sc_time(v1 - v0, a, j)
}

/// The largest speed reachable from (or that can stop to) v within distance l.
fn sc_reach(v: f64, l: f64, a: f64, j: f64) -> f64 {
    let mut hi = (v.max(1.0)) * 2.0;
    while sc_dist(v, hi, a, j) < l && hi < 1e12 {
        hi *= 2.0;
    }
    let mut lo = v;
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if sc_dist(v, mid, a, j) <= l {
            lo = mid
        } else {
            hi = mid
        }
    }
    lo
}

/// Time over a block of length l from v0 to v1, cruise at most vn.
fn sc_block_time(l: f64, v0: f64, v1: f64, vn: f64, a: f64, j: f64) -> f64 {
    if l <= 0.0 {
        return 0.0;
    }
    let base = v0.max(v1);
    let fits = |vp: f64| sc_dist(v0, vp, a, j) + sc_dist(vp, v1, a, j) <= l;
    let vp = if vn.is_finite() && fits(vn) {
        vn
    } else {
        let mut hi = if vn.is_finite() { vn } else { base.max(1.0) * 2.0 };
        if !vn.is_finite() {
            while fits(hi) && hi < 1e12 {
                hi *= 2.0;
            }
        }
        let mut lo = base;
        for _ in 0..80 {
            let mid = 0.5 * (lo + hi);
            if fits(mid) { lo = mid } else { hi = mid }
        }
        lo
    };
    let d = sc_dist(v0, vp, a, j) + sc_dist(vp, v1, a, j);
    let cruise = if vp > 0.0 { (l - d).max(0.0) / vp } else { 0.0 };
    sc_time(vp - v0, a, j) + sc_time(vp - v1, a, j) + cruise
}

/// One S-curve planner block.
struct SBlk {
    len: f64,
    /// unit tangents at the start and the end (axis space)
    u0: Vec<f64>,
    u1: Vec<f64>,
    v_nom: f64,
    acc: f64,
    jerk: f64,
    seg: usize,
}

/// The jerk-limited time of every segment of `tl` (s), and their sum.
pub fn plan_scurve(def: &MachineDef, tl: &Timeline, p: &SCurve) -> (Vec<f64>, f64) {
    let n = def.joints.len();
    let lin = |j: usize| matches!(def.joints[j].ty, JointType::Linear);
    let acc: Vec<f64> = (0..n)
        .map(|j| if lin(j) { p.accel[0] } else { p.accel[1] })
        .collect();
    let jrk: Vec<f64> = (0..n)
        .map(|j| if lin(j) { p.jerk[0] } else { p.jerk[1] })
        .collect();
    let mut per = vec![0.0; tl.segments.len()];
    let mut run: Vec<SBlk> = Vec::new();
    let flush = |run: &mut Vec<SBlk>, per: &mut Vec<f64>| {
        for (b, t) in run.iter().zip(scurve_run(run, p, &acc, &jrk)) {
            per[b.seg] += t;
        }
        run.clear();
    };
    let unit = |d: Vec<f64>| -> Option<Vec<f64>> {
        let l = d.iter().map(|x| x * x).sum::<f64>().sqrt();
        (l > 0.0).then(|| d.iter().map(|x| x / l).collect())
    };
    for (si, sg) in tl.segments.iter().enumerate() {
        if si > 0 {
            let pv = &tl.segments[si - 1];
            let gap_t = (sg.t0 - (pv.t0 + pv.t_min)).abs() > 1e-9 * sg.t0.abs().max(1.0);
            let gap_q = pv.q1.iter().zip(&sg.q0).any(|(a, b)| (a - b).abs() > 1e-9);
            if gap_t || gap_q || pv.tool != sg.tool || pv.spindle != sg.spindle {
                flush(&mut run, &mut per);
            }
        }
        let (len, u0, u1, curv) = match &sg.path {
            Path::Line => {
                let d: Vec<f64> = (0..n).map(|j| sg.q1[j] - sg.q0[j]).collect();
                let l = d.iter().map(|x| x * x).sum::<f64>().sqrt();
                let Some(u) = unit(d) else { continue };
                (l, u.clone(), u, None)
            }
            Path::Arc { a, b, r, r1, .. } => {
                let k = 64;
                let pts: Vec<Vec<f64>> = (0..=k).map(|i| sg.at(i as f64 / k as f64)).collect();
                let l: f64 = pts
                    .windows(2)
                    .map(|w| (0..n).map(|j| (w[1][j] - w[0][j]).powi(2)).sum::<f64>().sqrt())
                    .sum();
                let (Some(u0), Some(u1)) = (
                    unit((0..n).map(|j| pts[1][j] - pts[0][j]).collect()),
                    unit((0..n).map(|j| pts[k][j] - pts[k - 1][j]).collect()),
                ) else {
                    continue;
                };
                (l, u0, u1, Some((r.min(*r1), *a, *b)))
            }
        };
        if len == 0.0 {
            continue;
        }
        // the block's limits along it: the larger of the two tangents' (a straight block: one)
        let lim = |lim: &[f64], u: &[f64]| limit(lim, u);
        let mut a = lim(&acc, &u0).min(lim(&acc, &u1));
        let mut jk = lim(&jrk, &u0).min(lim(&jrk, &u1));
        let mut v_nom = if sg.t_min > 0.0 {
            len / sg.t_min
        } else {
            f64::INFINITY
        };
        if let Some((r, ja, jb)) = curv {
            // centripetal acceleration and jerk on the arc: the plane axes' limits
            let (ac, jc) = (acc[ja].min(acc[jb]), jrk[ja].min(jrk[jb]));
            if r > 0.0 {
                v_nom = v_nom
                    .min((ac * r).sqrt())
                    .min(forge_math::dmath::cbrt(jc * r * r));
            }
            a = a.min(ac);
            jk = jk.min(jc);
        }
        run.push(SBlk {
            len,
            u0,
            u1,
            v_nom,
            acc: a,
            jerk: jk,
            seg: si,
        });
    }
    flush(&mut run, &mut per);
    let total = per.iter().sum();
    (per, total)
}

/// One run from rest to rest: reverse and forward passes over all of it, S-curve blocks.
fn scurve_run(run: &[SBlk], p: &SCurve, acc: &[f64], jrk: &[f64]) -> Vec<f64> {
    let m = run.len();
    if m == 0 {
        return Vec::new();
    }
    // each block's largest entry: the G642 blend at the corner with the block before, and both
    // nominal speeds
    let mut max_entry = vec![0.0f64; m];
    for i in 1..m {
        let (a, b) = (&run[i - 1].u1, &run[i].u0);
        let cos_t: f64 = -a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
        let vj = if cos_t > 0.999999 {
            0.0
        } else if cos_t < -0.999999 {
            f64::INFINITY
        } else {
            let d: Vec<f64> = b.iter().zip(a).map(|(x, y)| x - y).collect();
            let dl = d.iter().map(|x| x * x).sum::<f64>().sqrt();
            let du: Vec<f64> = d.iter().map(|x| x / dl).collect();
            let s = (0.5 * (1.0 - cos_t)).sqrt();
            let rad = p.tolerance * s / (1.0 - s);
            (limit(acc, &du) * rad)
                .sqrt()
                .min(forge_math::dmath::cbrt(limit(jrk, &du) * rad * rad))
        };
        max_entry[i] = vj.min(run[i].v_nom).min(run[i - 1].v_nom);
    }
    // reverse: what each block can still stop from; forward: what it can reach
    let mut e = vec![0.0f64; m + 1];
    for k in (0..m).rev() {
        let reach = sc_reach(e[k + 1], run[k].len, run[k].acc, run[k].jerk);
        e[k] = if k == 0 { 0.0 } else { max_entry[k].min(reach) };
    }
    let mut x = vec![0.0f64; m + 1];
    for k in 0..m {
        let reach = sc_reach(x[k], run[k].len, run[k].acc, run[k].jerk);
        x[k + 1] = if k + 1 == m { 0.0 } else { e[k + 1].min(reach) };
    }
    (0..m)
        .map(|k| {
            let b = &run[k];
            sc_block_time(b.len, x[k], x[k + 1], b.v_nom, b.acc, b.jerk)
        })
        .collect()
}
