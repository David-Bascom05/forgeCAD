//! A program another CAM system posted (verify/cam-parts), measured as forge-cam's own paths are:
//! the text is read by forge-machine's interpreter (gcode.rs, the subset the simulation runs),
//! its motion turned into paths per op, swept through a height map of the stock, and its cycle
//! time taken from the same machine's timeline. The same numbers, the same model, both programs:
//! that is the comparison.
//!
//! What the text does not say is given, never guessed:
//! - the stock box (`Foreign::stock`): from the dataset's description and the program's own
//!   facing pass (verify/cam-parts/*/NOTICE says where each number comes from);
//! - flutes: a tool comment gives D and CR, not the flute count; without one (`flutes` None) the
//!   chip numbers are per revolution (hex columns read mm/rev, fzTgt 0);
//! - the machine: the one the comparison runs on (default generic-vmc3, as the benchmark parts').
//!
//! Ops are the program's own: a comment line after the first block opens one (Autodesk names its
//! operations so: `(2D ADAPTIVE2)`); a tool comment `(T1 D=6. CR=0. ...)` declares tool 1.
use crate::{JobQ, OpQ, PathIn, ProgQ, measure_op, side_mill};
use forge_cam::ops::{PMove, Phase};
use forge_machine::gcode::{self, Centre, Ev, FeedMode};
use forge_tooling::{Kind, Tool};

#[derive(Clone, Debug)]
pub struct Foreign {
    /// The stock box, program coordinates (mm).
    pub stock: ([f64; 3], [f64; 3]),
    pub flutes: Option<u32>,
    /// The dialect whose control reads the text (forge-post's name).
    pub dialect: String,
    pub machine: String,
    pub cell: f64,
}

/// A tool comment: `(T1 D=6. CR=0. - ZMIN=-7. - SCHAFTFRSER)` → (1, D, CR, the words after).
fn tool_comment(line: &str) -> Option<(u32, f64, f64, String)> {
    let s = line.trim().strip_prefix('(')?.strip_suffix(')')?;
    let mut w = s.split_whitespace();
    let t: u32 = w.next()?.strip_prefix('T')?.parse().ok()?;
    let mut d = None;
    let mut cr = 0.0;
    for x in s.split_whitespace() {
        if let Some(v) = x.strip_prefix("D=") {
            d = v.parse().ok();
        } else if let Some(v) = x.strip_prefix("CR=") {
            cr = v.parse().ok()?;
        }
    }
    Some((t, d?, cr, s.to_string()))
}

fn tool_of(t: u32, d: f64, cr: f64, words: &str, flutes: Option<u32>) -> Tool {
    let r = 0.5 * d;
    let up = words.to_uppercase();
    let kind = if up.contains("BOHRER") || up.contains("DRILL") {
        Kind::Drill
    } else if cr >= r - 1e-9 && cr > 0.0 {
        Kind::Ball
    } else if cr > 0.0 {
        Kind::Bull
    } else {
        Kind::Flat
    };
    Tool {
        id: format!("T{t}"),
        kind,
        dc: d,
        re: cr,
        angle: if kind == Kind::Drill { 118.0 } else { 0.0 },
        tip_d: 0.0,
        tp: 0.0,
        nof: flutes.unwrap_or(0),
        lcf: 3.0 * d,
        dcon: d,
        oal: 50.0,
        hand: "right".into(),
        material: String::new(),
        vendor: String::new(),
        product_id: String::new(),
        source: words.to_string(),
        taken: String::new(),
        note: "from the program's tool comment".into(),
        presets: Vec::new(),
        // read off a program's comment, not a maker's cited product
        real: false,
        cites: Vec::new(),
    }
}

/// A comment line that opens an op: its text.
fn op_comment(line: &str) -> Option<String> {
    let s = line.trim();
    let c = s.strip_prefix('(')?.strip_suffix(')')?;
    if c.contains('(') || tool_comment(line).is_some() {
        return None;
    }
    Some(c.trim().to_string())
}

struct OpBuild {
    name: String,
    /// (tool number, rpm, start, moves)
    paths: Vec<(u32, f64, [f64; 3], Vec<PMove>)>,
    moves: usize,
}

/// An arc in plane 18 or 19 (or 17 by R) as points no further apart than `l` (mm), the end
/// included.
fn arc_points(
    from: [f64; 3],
    to: [f64; 3],
    axes: (usize, usize),
    c: [f64; 2],
    cw: bool,
    full: bool,
    l: f64,
) -> Vec<[f64; 3]> {
    let (a, b) = axes;
    let r = ((from[a] - c[0]).powi(2) + (from[b] - c[1]).powi(2)).sqrt();
    let a0 = forge_math::dmath::atan2(from[b] - c[1], from[a] - c[0]);
    let a1 = forge_math::dmath::atan2(to[b] - c[1], to[a] - c[0]);
    let tau = std::f64::consts::TAU;
    let mut sw = a1 - a0;
    if full {
        sw = if cw { -tau } else { tau };
    } else if cw {
        while sw >= 0.0 {
            sw -= tau;
        }
    } else {
        while sw <= 0.0 {
            sw += tau;
        }
    }
    let n = ((sw.abs() * r) / l).ceil().max(1.0) as usize;
    let other = 3 - a - b;
    (1..=n)
        .map(|k| {
            if k == n {
                return to;
            }
            let s = k as f64 / n as f64;
            let (sn, cs) = forge_math::dmath::sin_cos(a0 + s * sw);
            let mut p = [0.0; 3];
            p[a] = c[0] + r * cs;
            p[b] = c[1] + r * sn;
            p[other] = from[other] + s * (to[other] - from[other]);
            p
        })
        .collect()
}

/// The centre of an R arc in its plane: the one on the side the sign and direction give.
fn r_centre(p: [f64; 2], q: [f64; 2], r: f64, cw: bool) -> [f64; 2] {
    let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
    let d = (dx * dx + dy * dy).sqrt().max(1e-12);
    let h = (r * r - 0.25 * d * d).max(0.0).sqrt();
    let m = [0.5 * (p[0] + q[0]), 0.5 * (p[1] + q[1])];
    // left normal of the chord
    let n = [-dy / d, dx / d];
    // a cw arc of less than half a turn has its centre right of the chord; R < 0 flips it
    let side = if cw { -1.0 } else { 1.0 } * r.signum();
    [m[0] + side * h * n[0], m[1] + side * h * n[1]]
}

/// Measures a posted program `text` (the file's name in `file`).
pub fn measure_text(text: &str, file: String, f: &Foreign) -> JobQ {
    let mut job = JobQ {
        file,
        ok: true,
        cell: f.cell,
        ..JobQ::default()
    };
    let mlib = forge_machine::library::Library::with_builtin();
    let def = match mlib.get(&f.machine) {
        Ok(d) => d,
        Err(e) => {
            job.ok = false;
            job.errors.push(e.to_string());
            return job;
        }
    };
    let Some(dsrc) = gcode::Control::dialect_src(&f.dialect) else {
        job.ok = false;
        job.errors.push(format!("no dialect `{}`", f.dialect));
        return job;
    };
    let mut control = match gcode::Control::for_machine(def, dsrc, 1) {
        Ok(c) => c,
        Err(e) => {
            job.ok = false;
            job.errors.push(e);
            return job;
        }
    };
    // the dialect's digits are what forge-post writes; another post writes more (Autodesk:
    // F333.33): read up to 4 decimals of a coordinate and 3 of a feed, exactly
    control.decimals = control.decimals.max(4);
    control.feed_decimals = control.feed_decimals.max(3);
    // G91.1 (arc centres incremental) is LinuxCNC's default: no motion effect
    if !control.lex.g_ignore.contains(&911) {
        control.lex.g_ignore.push(911);
    }
    let blocks = match gcode::run(text, &control) {
        Ok((b, _)) => b,
        Err(e) => {
            job.ok = false;
            job.errors.push(format!("line {}: {}", e.line, e.what));
            return job;
        }
    };
    // tools and ops from the comments
    let mut tools: Vec<(u32, Tool)> = Vec::new();
    let mut op_at: Vec<(usize, String)> = Vec::new(); // (line, name)
    let mut seen_block = false;
    for (i, line) in text.lines().enumerate() {
        let s = line.trim();
        if let Some((t, d, cr, words)) = tool_comment(s) {
            if !tools.iter().any(|(n, _)| *n == t) {
                tools.push((t, tool_of(t, d, cr, &words, f.flutes)));
            }
            continue;
        }
        if s.is_empty() || s == "%" {
            continue;
        }
        if s.starts_with('(') {
            if seen_block && let Some(n) = op_comment(s) {
                op_at.push((i + 1, n));
            }
            continue;
        }
        seen_block = true;
    }
    let op_of_line = |line: usize| op_at.iter().rposition(|(l, _)| *l <= line);
    let kq = 10f64.powi(control.decimals as i32);
    let kf = 10f64.powi(control.feed_decimals as i32);
    let ix = |c: char| control.letters.iter().position(|l| *l == c);
    let (ixx, iyy, izz) = (ix('X').unwrap(), ix('Y').unwrap(), ix('Z').unwrap());
    let top = f.stock.1[2];
    // G53 Z / G28: the tool goes to the machine's top; in the part's frame that is high above
    // the stock: taken as 100 mm over its top (it bears on rapid length only, not on the
    // timeline, which runs the text itself)
    let z_home = top + 100.0;
    let mut pos = [0.0, 0.0, z_home];
    let mut known = [false; 3];
    let mut tool: Option<u32> = None;
    let mut rpm = 0.0;
    let mut ops: Vec<OpBuild> = op_at
        .iter()
        .map(|(_, n)| OpBuild {
            name: n.clone(),
            paths: Vec::new(),
            moves: 0,
        })
        .collect();
    let mut pre = OpBuild {
        name: "(before the first op)".into(),
        paths: Vec::new(),
        moves: 0,
    };
    let target = |to: &[Option<i64>], pos: [f64; 3]| -> [f64; 3] {
        let g = |i: usize, d: f64| to[i].map(|v| v as f64 / kq).unwrap_or(d);
        [g(ixx, pos[0]), g(iyy, pos[1]), g(izz, pos[2])]
    };
    for b in &blocks {
        let o = match op_of_line(b.line) {
            Some(k) => &mut ops[k],
            None => &mut pre,
        };
        let push = |o: &mut OpBuild, from: [f64; 3], m: PMove, tool: Option<u32>, rpm: f64| {
            let t = tool.unwrap_or(0);
            let new = match o.paths.last() {
                Some(p) => p.0 != t || p.1 != rpm,
                None => true,
            };
            if new {
                o.paths.push((t, rpm, from, Vec::new()));
            }
            o.paths.last_mut().unwrap().3.push(m);
            o.moves += 1;
        };
        match &b.ev {
            Ev::Rapid(to) => {
                let p = target(to, pos);
                if known.iter().all(|k| *k) {
                    push(o, pos, PMove::Rapid { to: p }, tool, rpm);
                }
                for (k, i) in [ixx, iyy, izz].iter().enumerate() {
                    known[k] |= to[*i].is_some();
                }
                pos = p;
            }
            Ev::Machine(axes) => {
                // only a Z move to the machine's top is read (G53 G0 Z0.)
                if axes.iter().any(|(i, _)| *i == izz) {
                    let p = [pos[0], pos[1], z_home];
                    if known.iter().all(|k| *k) {
                        push(o, pos, PMove::Rapid { to: p }, tool, rpm);
                    }
                    pos = p;
                    known[2] = true;
                }
                if axes.iter().any(|(i, _)| *i != izz) {
                    job.errors
                        .push(format!("line {}: G53 on X or Y is not read", b.line));
                    job.ok = false;
                }
            }
            Ev::Home { .. } => {
                let p = [pos[0], pos[1], z_home];
                push(o, pos, PMove::Rapid { to: p }, tool, rpm);
                pos = p;
            }
            Ev::Feed { to, f: fq, mode } | Ev::Arc { to, f: fq, mode, .. } => {
                if *mode != FeedMode::PerMin {
                    job.errors
                        .push(format!("line {}: only G94 feeds are read", b.line));
                    job.ok = false;
                    continue;
                }
                let fv = *fq as f64 / kf;
                let p = target(to, pos);
                match &b.ev {
                    Ev::Feed { .. } => push(
                        o,
                        pos,
                        PMove::Line {
                            to: p,
                            f: fv,
                            phase: Phase::Cut,
                        },
                        tool,
                        rpm,
                    ),
                    Ev::Arc {
                        plane,
                        centre,
                        cw,
                        full,
                        ..
                    } => {
                        // the plane's axes in x y z order (the control's letters may be in
                        // another: generic-vmc3 has Y X Z)
                        let axes = match plane {
                            17 => (0, 1),
                            18 => (2, 0),
                            _ => (1, 2),
                        };
                        let ap = [pos[axes.0], pos[axes.1]];
                        let bp = [p[axes.0], p[axes.1]];
                        let c = match centre {
                            Centre::Abs(c) => [c[0] as f64 / kq, c[1] as f64 / kq],
                            Centre::R(r) => r_centre(ap, bp, *r as f64 / kq, *cw),
                        };
                        if *plane == 17 && !*full {
                            push(
                                o,
                                pos,
                                PMove::Arc {
                                    to: p,
                                    c,
                                    cw: *cw,
                                    f: fv,
                                },
                                tool,
                                rpm,
                            );
                        } else {
                            let mut a = pos;
                            for q in arc_points(pos, p, axes, c, *cw, *full, 0.05) {
                                push(
                                    o,
                                    a,
                                    PMove::Line {
                                        to: q,
                                        f: fv,
                                        phase: Phase::Cut,
                                    },
                                    tool,
                                    rpm,
                                );
                                a = q;
                            }
                        }
                    }
                    _ => unreachable!(),
                }
                pos = p;
                known = [true; 3];
            }
            Ev::Tool { t, .. } => tool = Some(*t),
            Ev::Spindle { s, css, .. } => {
                if *css {
                    job.errors.push(format!("line {}: G96 is not read", b.line));
                    job.ok = false;
                }
                rpm = *s as f64;
            }
            Ev::SpindleStop => rpm = 0.0,
            _ => {}
        }
    }
    // measure each op on one stock
    let (smin, smax) = f.stock;
    let mut m = crate::map::HeightMap::new(smin, smax, f.cell);
    let curve: Vec<[f64; 2]> = def.spindles.first().map(|s| s.power.clone()).unwrap_or_default();
    let tool_ref = |t: u32| tools.iter().find(|(n, _)| *n == t).map(|(_, x)| x);
    let mut op_index: Vec<Option<usize>> = Vec::new();
    for o in std::iter::once(&pre).chain(ops.iter()) {
        let cutting = o.paths.iter().any(|p| {
            p.3.iter()
                .any(|m| matches!(m, PMove::Line { .. } | PMove::Arc { .. }))
        });
        if !cutting {
            op_index.push(None);
            continue;
        }
        let Some(t0) = o.paths.iter().find_map(|p| tool_ref(p.0)) else {
            job.errors
                .push(format!("op {}: no tool comment for its tool", o.name));
            job.ok = false;
            op_index.push(None);
            continue;
        };
        let mut q = OpQ {
            name: o.name.clone(),
            kind: "foreign".into(),
            status: "posted".into(),
            moves: o.moves,
            hex_min: f64::INFINITY,
            tool: t0.id.clone(),
            d: t0.dc,
            flutes: t0.nof,
            side: side_mill(t0.kind),
            ..OpQ::default()
        };
        q.setup = 0;
        let paths: Vec<PathIn> = o
            .paths
            .iter()
            .filter_map(|p| {
                Some(PathIn {
                    tool: tool_ref(p.0)?,
                    rpm: p.1,
                    start: p.2,
                    moves: &p.3,
                })
            })
            .collect();
        op_index.push(Some(job.ops.len()));
        job.ops.push(measure_op(q, &paths, &mut m, None, &curve, f.cell));
    }
    job.ends = vec![Some(m.clone())];
    // the timeline: the text itself on the machine
    let mut pq = ProgQ {
        name: job.file.clone(),
        setups: vec![0],
        machine: f.machine.clone(),
        ..ProgQ::default()
    };
    let res = (|| -> Result<forge_machine::sim::Timeline, String> {
        use forge_machine::sim::{Cut, Setup, SimTool};
        let mut s = Setup::new(def, control.clone()).map_err(|e| e.to_string())?;
        let letters = s.control.letters.clone();
        for n in 54..=59 {
            let v = letters
                .iter()
                .map(|&l| if l == 'Z' { -500.0 } else { 0.0 })
                .collect();
            s.wcs.push((n, v));
        }
        for (t, tl) in &tools {
            let r = 0.5 * tl.dc;
            s.tools.push(SimTool {
                t: *t,
                name: tl.id.clone(),
                tip: [0.0, 0.0, -100.0],
                body: vec![(r, 0.0, tl.lcf.max(1.0))],
                cut: Cut::Mill {
                    r,
                    corner: tl.re.min(r),
                },
            });
            s.h.push((*t, 100.0));
        }
        forge_machine::sim::simulate_blocks(&blocks, &s).map_err(|e| e.to_string())
    })();
    match res {
        Ok(tl) => {
            pq.t_min = tl.t_min;
            pq.t_stop = tl.t_stop;
            pq.accel_missing = tl.accel_missing;
            pq.violations = tl.violations.len();
            if std::env::var_os("FORGE_QUALITY_TRACE").is_some() {
                for v in &tl.violations {
                    eprintln!("violation {v:?}");
                }
            }
            let to_op = |line: usize| -> Option<usize> {
                let k = op_of_line(line).map(|k| k + 1).unwrap_or(0);
                op_index.get(k).copied().flatten()
            };
            for sg in &tl.segments {
                if let Some(k) = to_op(sg.line) {
                    job.ops[k].t_min += sg.t_min;
                    job.ops[k].t_stop += sg.t_stop;
                }
            }
            for mk in &tl.marks {
                use forge_machine::sim::Happening;
                let secs = match mk.what {
                    Happening::ToolChange { seconds, .. } => {
                        pq.tool_changes += 1;
                        seconds
                    }
                    Happening::Index { seconds, .. } | Happening::Dwell { seconds } => seconds,
                    _ => 0.0,
                };
                if let Some(k) = to_op(mk.line) {
                    job.ops[k].t_min += secs;
                    job.ops[k].t_stop += secs;
                }
            }
        }
        Err(e) => pq.error = Some(e),
    }
    job.progs.push(pq);
    job
}
