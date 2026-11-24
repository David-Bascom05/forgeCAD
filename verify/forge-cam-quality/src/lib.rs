//! Toolpath quality: what forge-cam's certified paths are like to run. Per op and per program:
//!
//! - cycle time on a machine's timeline (forge-machine: `t_min`, a lower bound from the feeds and
//!   rapid rates, and `t_stop`, exact stops with trapezoidal acceleration), tool changes included;
//! - radial engagement (engagement angle φ and ae/D) through every cutting piece, its maximum and
//!   its time distribution; climb, conventional and slotting time;
//! - the chip: the maximum chip thickness the programmed feed gives at that engagement
//!   (hex = fz · max cos θ over the engaged edge, i.e. after radial chip thinning), against the
//!   cutting data's feed per tooth for the tool's diameter;
//! - material removal: volume, MRR, and the share of feed time spent in air;
//! - links: rapid distance, retracts, straight plunges into material, ramps, sharp corners at feed;
//! - load: Kienzle (kc = kc1.1 · hm^−mc) tangential force per tooth and spindle power, against
//!   the spindle's power curve.
//!
//! The stock is a height map (map.rs) over the setup's stock box: a measurement model within
//! about a cell of the exact sweep. Spindle rotation is taken as M03 (clockwise from above), so
//! material right of the direction of travel is climb. Everything here is deterministic.
pub mod map;

use forge_cam::ops::{PMove, Phase};
use forge_cam_lang::Cam;
use forge_tooling::Kind;
use map::{HeightMap, Profile};
use std::fmt::Write as _;
use std::path::Path;

/// What the harness is asked.
#[derive(Clone, Debug, Default)]
pub struct Config {
    /// Cell size of the height map (mm); None: from the smallest tool (D/60, in [0.02, 0.1]).
    pub cell: Option<f64>,
    /// The machine whose timeline gives cycle time; None: the setup's own.
    pub machine: Option<String>,
}

/// Engagement-angle bins (degrees), upper ends: air is φ = 0.
pub const PHI_BINS: [f64; 6] = [30.0, 60.0, 90.0, 120.0, 150.0, 180.0 + 1e-9];

#[derive(Clone, Debug, Default)]
pub struct OpQ {
    pub name: String,
    pub kind: String,
    pub status: String,
    pub tool: String,
    pub d: f64,
    pub flutes: u32,
    pub rpm: f64,
    pub moves: usize,
    /// Path lengths (mm): at feed meeting material, at feed in air, at rapid.
    pub len_cut: f64,
    pub len_air: f64,
    pub len_rapid: f64,
    /// Times from length / feed (s).
    pub t_cut: f64,
    pub t_air: f64,
    /// Timeline times of the op's blocks (s), tool change included.
    pub t_min: f64,
    pub t_stop: f64,
    pub retracts: usize,
    pub plunges: usize,
    pub ramps: usize,
    pub cycles: usize,
    /// Direction changes at feed of more than 30° and more than 90°, and the largest (degrees).
    pub corners30: usize,
    pub corners90: usize,
    pub turn_max: f64,
    /// Whether the side-engagement numbers apply (a side-cutting mill).
    pub side: bool,
    pub phi_max: f64,
    pub phi_mean: f64,
    /// Time share of cutting in each PHI_BINS bin.
    pub phi_hist: [f64; 6],
    /// Time shares of side cutting: climb, conventional, slotting (material at both extremes of the
    /// front half: the tool cuts both walls).
    pub climb: f64,
    pub conventional: f64,
    pub slotting: f64,
    /// Feed per tooth the cutting data asks for this diameter (mm).
    pub fz_target: f64,
    /// Max chip thickness after thinning: min, time-weighted mean, max (mm).
    pub hex_min: f64,
    pub hex_mean: f64,
    pub hex_max: f64,
    /// The chip that 99% of the side-cutting time is at or under (mm): hex_max without the
    /// slivers a few cells wide.
    pub hex_p99: f64,
    /// Time shares with hex < 0.5 fz_target (rubbing) and > 1.25 fz_target (overload).
    pub thin: f64,
    pub thick: f64,
    pub removed: f64,
    /// Mean MRR over the cutting time (mm³/min).
    pub mrr: f64,
    /// Kienzle tangential force per tooth, peak (N); spindle power peak (kW) and the spindle's
    /// power at the op's speed (kW, 0 = unknown).
    pub fc_max: f64,
    pub p_max: f64,
    pub p_avail: f64,
    /// Rapid pieces that met material (a certified path has none).
    pub rapid_cuts: usize,
    /// Hole cycles: the feed per spindle revolution (mm/rev), the least over the op's tools
    /// (spot, drill, tap).
    pub f_rev: f64,
}

#[derive(Clone, Debug, Default)]
pub struct ProgQ {
    pub name: String,
    pub machine: String,
    pub t_min: f64,
    pub t_stop: f64,
    pub tool_changes: usize,
    pub accel_missing: bool,
    pub violations: usize,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct JobQ {
    pub file: String,
    pub ok: bool,
    pub cell: f64,
    pub ops: Vec<OpQ>,
    pub progs: Vec<ProgQ>,
    pub errors: Vec<String>,
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// A path move cut into straight pieces no longer than `l` (arcs by chords).
fn pieces(from: [f64; 3], m: &PMove, l: f64) -> Vec<[f64; 3]> {
    match *m {
        PMove::Rapid { to } | PMove::Line { to, .. } => {
            let n = (dist(from, to) / l).ceil().max(1.0) as usize;
            (1..=n)
                .map(|k| {
                    let s = k as f64 / n as f64;
                    [
                        from[0] + s * (to[0] - from[0]),
                        from[1] + s * (to[1] - from[1]),
                        from[2] + s * (to[2] - from[2]),
                    ]
                })
                .collect()
        }
        PMove::Arc { to, c, cw, .. } => {
            let r = ((from[0] - c[0]).powi(2) + (from[1] - c[1]).powi(2)).sqrt();
            let a0 = forge_math::dmath::atan2(from[1] - c[1], from[0] - c[0]);
            let a1 = forge_math::dmath::atan2(to[1] - c[1], to[0] - c[0]);
            let tau = std::f64::consts::TAU;
            let mut sw = a1 - a0;
            if cw {
                while sw >= 0.0 {
                    sw -= tau;
                }
            } else {
                while sw <= 0.0 {
                    sw += tau;
                }
            }
            // same point: a full turn; otherwise sw in (−2π, 0) or (0, 2π)
            let n = ((sw.abs() * r).max(dist(from, to)) / l).ceil().max(1.0) as usize;
            (1..=n)
                .map(|k| {
                    let s = k as f64 / n as f64;
                    if k == n {
                        return to;
                    }
                    let (sn, cs) = forge_math::dmath::sin_cos(a0 + s * sw);
                    [c[0] + r * cs, c[1] + r * sn, from[2] + s * (to[2] - from[2])]
                })
                .collect()
        }
        PMove::Cycle { .. } => vec![],
    }
}

fn profile(t: &forge_tooling::Tool) -> Profile {
    let r = 0.5 * t.dc;
    match t.kind {
        Kind::Ball => Profile::Ball { r },
        Kind::Bull => Profile::Bull { r, re: t.re.min(r) },
        Kind::Chamfer | Kind::Spot | Kind::Countersink | Kind::Engrave | Kind::Drill => {
            let half = if t.angle > 0.0 { 0.5 * t.angle } else { 59.0 };
            Profile::Cone {
                r,
                tip: 0.5 * t.tip_d,
                half: half.to_radians(),
            }
        }
        _ => Profile::Bull { r, re: t.re.min(r) },
    }
}

fn side_mill(k: Kind) -> bool {
    matches!(k, Kind::Flat | Kind::Bull | Kind::Ball | Kind::Slot)
}

/// Power (kW) of a spindle curve at rpm, linear between points; 0 if the curve is empty.
fn power_at(curve: &[[f64; 2]], rpm: f64) -> f64 {
    if curve.is_empty() {
        return 0.0;
    }
    if rpm <= curve[0][0] {
        return curve[0][1];
    }
    for w in curve.windows(2) {
        if rpm <= w[1][0] {
            let f = (rpm - w[0][0]) / (w[1][0] - w[0][0]).max(1e-12);
            return w[0][1] + f * (w[1][1] - w[0][1]);
        }
    }
    curve[curve.len() - 1][1]
}

/// The material threshold: a piece that removes less is in air (mm³).
const AIR_VOL: f64 = 1e-4;
/// A point of the edge is in material if the stock there is this far above the tip (mm).
const EPS_Z: f64 = 1e-3;

/// Measures one machining document.
pub fn measure(path: &Path, cfg: &Config) -> JobQ {
    let file = path.display().to_string();
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            return JobQ {
                file,
                errors: vec![e.to_string()],
                ..JobQ::default()
            };
        }
    };
    let dir = path.parent().unwrap_or(Path::new("."));
    let cam = forge_cam_lang::run(&src, dir);
    measure_cam(&cam, file, cfg)
}

pub fn measure_cam(cam: &Cam, file: String, cfg: &Config) -> JobQ {
    let tools = cam.tool_uses();
    let dmin = cam
        .ops
        .iter()
        .filter_map(|o| o.tool.map(|t| tools[t].tool.dc))
        .fold(f64::INFINITY, f64::min);
    let cell = cfg.cell.unwrap_or_else(|| {
        if dmin.is_finite() {
            (dmin / 60.0).clamp(0.02, 0.1)
        } else {
            0.1
        }
    });
    let mut job = JobQ {
        file,
        ok: cam.ok,
        cell,
        errors: cam.errors.clone(),
        ..JobQ::default()
    };
    let mlib = forge_machine::library::Library::with_builtin();
    let mut maps: Vec<Option<HeightMap>> = Vec::new();
    for op in &cam.ops {
        let Some(sv) = cam.setup_view(op.setup) else {
            continue;
        };
        if maps.len() <= op.setup {
            maps.resize_with(op.setup + 1, || None);
        }
        let m = maps[op.setup].get_or_insert_with(|| HeightMap::new(sv.stock.min, sv.stock.max, cell));
        let group = cam.cutting_group(sv.group);
        let mdef = mlib.get(cfg.machine.as_deref().unwrap_or(sv.machine)).ok();
        let curve: Vec<[f64; 2]> = mdef
            .and_then(|d| d.spindles.first())
            .map(|s| s.power.clone())
            .unwrap_or_default();
        let mut q = OpQ {
            name: op.name.clone(),
            kind: op.kind.to_string(),
            status: op.status.to_string(),
            moves: op.moves,
            hex_min: f64::INFINITY,
            ..OpQ::default()
        };
        if let Some(ti) = op.tool {
            let t = &tools[ti].tool;
            q.tool = t.id.clone();
            q.d = t.dc;
            q.flutes = t.nof.max(1);
            q.side = side_mill(t.kind);
            q.fz_target = group.map(|g| g.fz_at(t.dc)).unwrap_or(0.0);
        }
        let mut phi_t = 0.0; // Σ φ dt
        let mut side_t = 0.0; // time of side-engaged cutting
        let mut hex_t = 0.0;
        let mut hex_tt = 0.0;
        let mut hexes: Vec<(f64, f64)> = Vec::new();
        for p in &op.paths {
            let tu = &tools[p.tool];
            let prof = profile(&tu.tool);
            let r = prof.r();
            let flutes = tu.tool.nof.max(1) as f64;
            // as the post does (forge-cam-lang): the path's speed, else the op's feeds', else 3000
            let rpm = if p.rpm > 0.0 {
                p.rpm
            } else {
                op.feeds.map(|f| f.rpm).unwrap_or(3000.0)
            };
            q.rpm = rpm;
            q.p_avail = power_at(&curve, rpm);
            let l = cell.max((0.25 * r).min(0.5));
            let mut at = p.start;
            // the last feed direction (xy), for corners
            let mut last_dir: Option<[f64; 2]> = None;
            let mut was_descending = false;
            for mv in &p.moves {
                let (f, phase) = match *mv {
                    PMove::Rapid { .. } => (0.0, Phase::Rapid),
                    PMove::Line { f, phase, .. } => (f, phase),
                    PMove::Arc { f, .. } => (f, Phase::Cut),
                    PMove::Cycle { holes, z, f, .. } => {
                        q.cycles += 1;
                        if rpm > 0.0 {
                            q.f_rev = if q.f_rev > 0.0 {
                                q.f_rev.min(f / rpm)
                            } else {
                                f / rpm
                            };
                        }
                        let top = [holes[0], holes[1], m.top(holes[0], holes[1]).max(z)];
                        q.removed += m.sweep(top, [holes[0], holes[1], z], &prof);
                        continue;
                    }
                };
                let to = match *mv {
                    PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } => to,
                    PMove::Cycle { .. } => unreachable!(),
                };
                let dxy = ((to[0] - at[0]).powi(2) + (to[1] - at[1]).powi(2)).sqrt();
                let dz = to[2] - at[2];
                if dxy < 1e-9 && dz > 0.5 {
                    q.retracts += 1;
                }
                if phase == Phase::Rapid {
                    q.len_rapid += dist(at, to);
                    let mut a = at;
                    for b in pieces(at, mv, l) {
                        if m.sweep(a, b, &prof) > AIR_VOL {
                            q.rapid_cuts += 1;
                        }
                        a = b;
                    }
                    at = to;
                    last_dir = None;
                    was_descending = false;
                    continue;
                }
                // a direction change at feed, at the move's start
                let start_dir = match *mv {
                    PMove::Arc { c, cw, .. } => {
                        let (rx, ry) = (at[0] - c[0], at[1] - c[1]);
                        let n = (rx * rx + ry * ry).sqrt().max(1e-12);
                        Some(if cw { [ry / n, -rx / n] } else { [-ry / n, rx / n] })
                    }
                    _ if dxy > 1e-9 => Some([(to[0] - at[0]) / dxy, (to[1] - at[1]) / dxy]),
                    _ => None,
                };
                if let (Some(u), Some(v)) = (last_dir, start_dir) {
                    let turn = forge_math::dmath::acos_clamped(u[0] * v[0] + u[1] * v[1]).to_degrees();
                    q.turn_max = q.turn_max.max(turn);
                    if turn > 30.0 {
                        q.corners30 += 1;
                    }
                    if turn > 90.0 {
                        q.corners90 += 1;
                    }
                }
                let mut moved_in = false;
                let mut a = at;
                for b in pieces(at, mv, l) {
                    let len = dist(a, b);
                    let dt = if f > 0.0 { 60.0 * len / f } else { 0.0 };
                    let pxy = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
                    let eng = if q.side && pxy > 1e-9 {
                        let u = [(b[0] - a[0]) / pxy, (b[1] - a[1]) / pxy];
                        Some((map::engage(m, b, u, r, EPS_Z), u))
                    } else {
                        None
                    };
                    let removed = m.sweep(a, b, &prof);
                    q.removed += removed;
                    if removed > AIR_VOL {
                        moved_in = true;
                        q.len_cut += len;
                        q.t_cut += dt;
                        let fz = if rpm > 0.0 { f / (rpm * flutes) } else { 0.0 };
                        if let Some((e, _)) = eng.filter(|(e, _)| e.phi > 0.0) {
                            phi_t += e.phi * dt;
                            q.phi_max = q.phi_max.max(e.phi);
                            let bin = PHI_BINS.iter().position(|&hi| e.phi <= hi).unwrap_or(5);
                            q.phi_hist[bin] += dt;
                            side_t += dt;
                            // M03: material right of the travel is climb (the tooth enters
                            // it at full chip and leaves at the wall at none)
                            if e.edge_r && e.edge_l {
                                q.slotting += dt;
                            } else if e.edge_r || (!e.edge_l && e.right >= e.left) {
                                q.climb += dt;
                            } else {
                                q.conventional += dt;
                            }
                            let hex = fz * e.cos_max;
                            if std::env::var_os("FORGE_QUALITY_PIECES").is_some() {
                                eprintln!(
                                    "piece {} f={f:.1} dt={dt:.4} phi={:.1} r={:.1} l={:.1} hex={hex:.4} ap={:.3} at={:?}",
                                    op.name, e.phi, e.right, e.left, e.ap, b
                                );
                            }
                            q.hex_min = q.hex_min.min(hex);
                            q.hex_max = q.hex_max.max(hex);
                            hex_t += hex * dt;
                            hex_tt += dt;
                            hexes.push((hex, dt));
                            if hex < 0.5 * q.fz_target {
                                q.thin += dt;
                            }
                            if hex > 1.25 * q.fz_target {
                                q.thick += dt;
                            }
                            if let Some(g) = group {
                                let ap = e.ap.min(tu.tool.lcf.max(1e-3));
                                let hm = (fz * e.cos_mean).max(1e-4);
                                let kc = g.kc1_1 * forge_math::dmath::powf(hm, -g.mc);
                                // the width of material across the travel: r (sin θhi − sin θlo)
                                // for one arc; ae of a side cut, D for a slot
                                let ae =
                                    r * (1.0 - forge_math::dmath::cos(e.phi.to_radians())).clamp(0.0, 2.0);
                                let mrr = ap * ae * f; // mm³/min
                                q.p_max = q.p_max.max(mrr * kc / 6.0e7);
                                if hex > 0.0 {
                                    let fc = g.kc1_1 * ap * forge_math::dmath::powf(hex, 1.0 - g.mc);
                                    q.fc_max = q.fc_max.max(fc);
                                }
                            }
                        }
                    } else {
                        q.len_air += len;
                        q.t_air += dt;
                    }
                    a = b;
                }
                let descending = dz < -1e-9 && dxy >= 1e-9;
                if moved_in && dz < -1e-9 {
                    if dxy < 1e-9 {
                        q.plunges += 1;
                        if std::env::var_os("FORGE_QUALITY_TRACE").is_some() {
                            eprintln!("{} plunge into material at {:?} -> {:?}", op.name, at, to);
                        }
                    } else if !was_descending {
                        // a ramp entry: a run of descending moves with xy (a helix's arcs) counts once
                        q.ramps += 1;
                    }
                }
                was_descending = descending;
                let end_dir = match *mv {
                    PMove::Arc { c, cw, .. } => {
                        let (rx, ry) = (to[0] - c[0], to[1] - c[1]);
                        let n = (rx * rx + ry * ry).sqrt().max(1e-12);
                        Some(if cw { [ry / n, -rx / n] } else { [-ry / n, rx / n] })
                    }
                    _ => start_dir,
                };
                if end_dir.is_some() {
                    last_dir = end_dir;
                } else if dxy < 1e-9 {
                    // a pure z move breaks the run of feed moves: no corner across it
                    last_dir = None;
                }
                at = to;
            }
        }
        if phi_t > 0.0 && side_t > 0.0 {
            q.phi_mean = phi_t / side_t;
            for h in &mut q.phi_hist {
                *h /= side_t;
            }
            q.climb /= side_t;
            q.conventional /= side_t;
            q.slotting /= side_t;
            q.thin /= side_t;
            q.thick /= side_t;
        }
        if hex_tt > 0.0 {
            q.hex_mean = hex_t / hex_tt;
            hexes.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut acc = 0.0;
            for (h, dt) in &hexes {
                acc += dt;
                q.hex_p99 = *h;
                if acc >= 0.99 * hex_tt {
                    break;
                }
            }
        }
        if !q.hex_min.is_finite() {
            q.hex_min = 0.0;
        }
        if q.t_cut > 0.0 {
            q.mrr = q.removed / (q.t_cut / 60.0);
        }
        job.ops.push(q);
    }
    timeline(cam, cfg, &mlib, &mut job);
    job
}

/// Each program on the machine: its timeline, the ops' shares by their blocks.
fn timeline(cam: &Cam, cfg: &Config, mlib: &forge_machine::library::Library, job: &mut JobQ) {
    use forge_machine::sim::{self, Cut, Happening, Setup, SimTool};
    for p in &cam.programs {
        let mut pq = ProgQ {
            name: p.name.clone(),
            ..ProgQ::default()
        };
        let Some(text) = p.text.as_deref() else {
            pq.error = Some(p.error.clone().unwrap_or_else(|| "no program text".into()));
            job.progs.push(pq);
            continue;
        };
        let mname = cfg.machine.clone().unwrap_or_else(|| {
            p.setups
                .first()
                .and_then(|&s| cam.setup_view(s))
                .map(|v| v.machine.to_string())
                .unwrap_or_default()
        });
        pq.machine = mname.clone();
        let res = (|| -> Result<sim::Timeline, String> {
            let def = mlib.get(&mname).map_err(|e| e.to_string())?;
            let dsrc = forge_machine::gcode::Control::dialect_src(&p.post)
                .ok_or_else(|| format!("no dialect `{}`", p.post))?;
            let control =
                forge_machine::gcode::Control::for_machine(def, dsrc, 1).map_err(|e| e.to_string())?;
            let mut s = Setup::new(def, control).map_err(|e| e.to_string())?;
            let letters = s.control.letters.clone();
            // G54..G59: word 0 at the machine's tip 500 − Z − H above its zero (as the generic
            // VMC's example setup); travel limits do not bear on the time
            for n in 54..=59 {
                let v = letters
                    .iter()
                    .map(|&l| if l == 'Z' { -500.0 } else { 0.0 })
                    .collect();
                s.wcs.push((n, v));
            }
            for tu in cam.tool_uses() {
                let r = 0.5 * tu.tool.dc;
                s.tools.push(SimTool {
                    t: tu.t,
                    name: tu.tool.id.clone(),
                    tip: [0.0, 0.0, -100.0],
                    body: vec![(r, 0.0, tu.tool.lcf.max(1.0))],
                    cut: Cut::Mill {
                        r,
                        corner: tu.tool.re.min(r),
                    },
                });
                s.h.push((tu.t, 100.0));
            }
            sim::simulate(text, &s).map_err(|e| e.to_string())
        })();
        match res {
            Ok(tl) => {
                pq.t_min = tl.t_min;
                pq.t_stop = tl.t_stop;
                pq.accel_missing = tl.accel_missing;
                pq.violations = tl.violations.len();
                // text line (1-based) → op: the op whose comment "(name kind)" came last
                let names: Vec<(String, usize)> = job
                    .ops
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| p.setups.contains(&cam.ops[*i].setup))
                    .map(|(i, o)| (format!("({} {})", o.name, o.kind), i))
                    .collect();
                let mut op_of = vec![None; text.lines().count() + 2];
                let mut cur = None;
                for (i, line) in text.lines().enumerate() {
                    if let Some((_, k)) = names.iter().find(|(n, _)| line.contains(n.as_str())) {
                        cur = Some(*k);
                    }
                    op_of[i + 1] = cur;
                }
                for sg in &tl.segments {
                    if let Some(Some(k)) = op_of.get(sg.line) {
                        job.ops[*k].t_min += sg.t_min;
                        job.ops[*k].t_stop += sg.t_stop;
                    }
                }
                for mk in &tl.marks {
                    let secs = match mk.what {
                        Happening::ToolChange { seconds, .. } => {
                            pq.tool_changes += 1;
                            seconds
                        }
                        Happening::Index { seconds, .. } | Happening::Dwell { seconds } => seconds,
                        _ => 0.0,
                    };
                    if let Some(Some(k)) = op_of.get(mk.line) {
                        job.ops[*k].t_min += secs;
                        job.ops[*k].t_stop += secs;
                    }
                }
            }
            Err(e) => pq.error = Some(e),
        }
        job.progs.push(pq);
    }
}

fn f(x: f64, d: usize) -> String {
    format!("{x:.d$}")
}

fn pct(x: f64) -> String {
    format!("{:.0}%", 100.0 * x)
}

/// The table, one row per op and a line per program.
pub fn table(jobs: &[JobQ]) -> String {
    let mut o = String::new();
    let _ = writeln!(
        o,
        "{:<22} {:<10} {:<8} {:>6} {:>7} {:>7} {:>5} {:>5} {:>4} {:>4} {:>4} {:>5} {:>5} {:>5} {:>5} {:>6} {:>6} {:>6} {:>6} {:>5} {:>5} {:>7} {:>6} {:>5} {:>6}",
        "job/op",
        "kind",
        "tool",
        "t_stop",
        "t_min",
        "cut_s",
        "air",
        "rapid",
        "retr",
        "plng",
        "c>90",
        "phiMx",
        "phiAv",
        "climb",
        "slot",
        "hexMx",
        "hex99",
        "hexAv",
        "fzTgt",
        "thin",
        "thick",
        "MRR",
        "Fc_N",
        "P_kW",
        "f/rev"
    );
    for j in jobs {
        let stem = Path::new(&j.file)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        for q in &j.ops {
            let side = |s: String| if q.side { s } else { "-".into() };
            let air = if q.t_cut + q.t_air > 0.0 {
                pct(q.t_air / (q.t_cut + q.t_air))
            } else {
                "-".into()
            };
            let _ = writeln!(
                o,
                "{:<22} {:<10} {:<8} {:>6} {:>7} {:>7} {:>5} {:>5} {:>4} {:>4} {:>4} {:>5} {:>5} {:>5} {:>5} {:>6} {:>6} {:>6} {:>6} {:>5} {:>5} {:>7} {:>6} {:>5} {:>6}",
                format!("{stem}/{}", q.name),
                q.kind,
                q.tool.chars().take(8).collect::<String>(),
                f(q.t_stop, 1),
                f(q.t_min, 1),
                f(q.t_cut, 1),
                air,
                f(q.len_rapid, 0),
                q.retracts,
                q.plunges,
                q.corners90,
                side(f(q.phi_max, 0)),
                side(f(q.phi_mean, 0)),
                side(pct(q.climb)),
                side(pct(q.slotting)),
                side(f(q.hex_max, 3)),
                side(f(q.hex_p99, 3)),
                side(f(q.hex_mean, 3)),
                f(q.fz_target, 3),
                side(pct(q.thin)),
                side(pct(q.thick)),
                f(q.mrr, 0),
                side(f(q.fc_max, 0)),
                side(f(q.p_max, 2)),
                if q.cycles > 0 { f(q.f_rev, 4) } else { "-".into() },
            );
        }
        for p in &j.progs {
            match &p.error {
                Some(e) => {
                    let _ = writeln!(o, "{stem}: program {} on {}: {e}", p.name, p.machine);
                }
                None => {
                    let _ = writeln!(
                        o,
                        "{stem}: program {} on {}: t_stop {} s, t_min {} s, {} tool changes, {} violations{}",
                        p.name,
                        p.machine,
                        f(p.t_stop, 1),
                        f(p.t_min, 1),
                        p.tool_changes,
                        p.violations,
                        if p.accel_missing {
                            " (accel unknown: t_stop = t_min on some joint)"
                        } else {
                            ""
                        }
                    );
                }
            }
        }
        for e in &j.errors {
            let _ = writeln!(o, "{stem}: error: {e}");
        }
    }
    o
}

fn jn(x: f64) -> String {
    if x.is_finite() {
        format!("{x:.6}")
    } else {
        "null".into()
    }
}

fn js(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// `forgecad.cam-quality/1`: every number of every op and program.
pub fn json(jobs: &[JobQ]) -> String {
    let mut o = String::from("{\"format\":\"forgecad.cam-quality/1\",\"jobs\":[");
    for (ji, j) in jobs.iter().enumerate() {
        if ji > 0 {
            o.push(',');
        }
        let _ = write!(
            o,
            "{{\"file\":{},\"ok\":{},\"cell\":{},\"errors\":[{}],\"ops\":[",
            js(&j.file),
            j.ok,
            jn(j.cell),
            j.errors.iter().map(|e| js(e)).collect::<Vec<_>>().join(",")
        );
        for (i, q) in j.ops.iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            let _ = write!(
                o,
                "{{\"name\":{},\"kind\":{},\"status\":{},\"tool\":{},\"d\":{},\"flutes\":{},\"rpm\":{},\"moves\":{},\
\"len_cut\":{},\"len_air\":{},\"len_rapid\":{},\"t_cut\":{},\"t_air\":{},\"t_min\":{},\"t_stop\":{},\
\"retracts\":{},\"plunges\":{},\"ramps\":{},\"cycles\":{},\"corners30\":{},\"corners90\":{},\"turn_max\":{},\
\"side\":{},\"phi_max\":{},\"phi_mean\":{},\"phi_hist\":[{}],\"climb\":{},\"conventional\":{},\"slotting\":{},\
\"fz_target\":{},\"hex_min\":{},\"hex_mean\":{},\"hex_max\":{},\"hex_p99\":{},\"thin\":{},\"thick\":{},\"removed\":{},\"mrr\":{},\
\"fc_max\":{},\"p_max\":{},\"p_avail\":{},\"rapid_cuts\":{},\"f_rev\":{}}}",
                js(&q.name),
                js(&q.kind),
                js(&q.status),
                js(&q.tool),
                jn(q.d),
                q.flutes,
                jn(q.rpm),
                q.moves,
                jn(q.len_cut),
                jn(q.len_air),
                jn(q.len_rapid),
                jn(q.t_cut),
                jn(q.t_air),
                jn(q.t_min),
                jn(q.t_stop),
                q.retracts,
                q.plunges,
                q.ramps,
                q.cycles,
                q.corners30,
                q.corners90,
                jn(q.turn_max),
                q.side,
                jn(q.phi_max),
                jn(q.phi_mean),
                q.phi_hist.iter().map(|x| jn(*x)).collect::<Vec<_>>().join(","),
                jn(q.climb),
                jn(q.conventional),
                jn(q.slotting),
                jn(q.fz_target),
                jn(q.hex_min),
                jn(q.hex_mean),
                jn(q.hex_max),
                jn(q.hex_p99),
                jn(q.thin),
                jn(q.thick),
                jn(q.removed),
                jn(q.mrr),
                jn(q.fc_max),
                jn(q.p_max),
                jn(q.p_avail),
                q.rapid_cuts,
                jn(q.f_rev)
            );
        }
        o.push_str("],\"programs\":[");
        for (i, p) in j.progs.iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            let _ = write!(
                o,
                "{{\"name\":{},\"machine\":{},\"t_min\":{},\"t_stop\":{},\"tool_changes\":{},\"accel_missing\":{},\"violations\":{},\"error\":{}}}",
                js(&p.name),
                js(&p.machine),
                jn(p.t_min),
                jn(p.t_stop),
                p.tool_changes,
                p.accel_missing,
                p.violations,
                p.error.as_deref().map(js).unwrap_or_else(|| "null".into())
            );
        }
        o.push_str("]}");
    }
    o.push_str("]}\n");
    o
}
