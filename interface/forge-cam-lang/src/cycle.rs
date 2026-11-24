//! Cycle times: each posted program run on its machine's timeline (forge-machine's `simulate`),
//! the guaranteed lower bound `t_min` beside the jerk-limited estimate `t_scurve`, per program and
//! per op (an op's share is its blocks', from its "(NAME KIND)" comment to the next op's). One
//! sum for every reader: the setup sheet here, forge-cam-quality, and so the app's Manufacture
//! view. Tool changes, indexes, dwells and pallet changes count in both numbers, as `t_min`
//! counts them (forge-machine's own `t_scurve` is the segments' alone).

use crate::Cam;

/// What the two numbers are, for a reader that shows them (the app's hover text says the same).
pub const CYCLE_NOTE: &str = "\u{2265}: the least time the posted program can take on this machine, from its feeds, rapid rates and tool changes (a guaranteed lower bound). est.: an estimate with stated assumptions, not a prediction: a jerk-limited motion planner with 1000 mm/s\u{b2} acceleration (the machine record's where it gives one), 20,000 mm/s\u{b3} jerk and corners blended within 0.02 mm. A real machine can differ by tens of per cent.";

/// An op's share of its program's timeline (s), tool change included.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OpTime {
    pub t_min: f64,
    pub t_stop: f64,
    pub t_scurve: f64,
}

/// A program's timeline (s).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProgTime {
    pub name: String,
    pub machine: String,
    pub t_min: f64,
    pub t_stop: f64,
    pub t_scurve: f64,
    pub tool_changes: usize,
    pub accel_missing: bool,
    pub violations: usize,
    /// Why it was not timed (no text, no such machine or dialect, the simulation refused).
    pub error: Option<String>,
}

/// Every program's time, and each op's share (indexed as `Cam::ops`; zero for an op no timed
/// program posts).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CycleTimes {
    pub ops: Vec<OpTime>,
    pub progs: Vec<ProgTime>,
}

/// m:ss, whole seconds rounded (59.6 s is 1:00, not 0:60).
pub fn mmss(secs: f64) -> String {
    let s = secs.max(0.0).round();
    format!("{}:{:02}", (s / 60.0).floor(), (s % 60.0) as u32)
}

/// A cycle time in words: the lower bound and the estimate together, never the estimate alone.
pub fn cycle_words(t_min: f64, t_scurve: f64) -> String {
    format!(
        "\u{2265} {} \u{b7} est. {}",
        mmss(t_min),
        mmss(t_scurve.max(t_min))
    )
}

impl Cam {
    /// Each program on its machine (`machine` overrides the setups' own, as forge-cam-quality's
    /// `--machine`): its timeline, and the ops' shares by their blocks.
    pub fn cycle_times(&self, mlib: &forge_machine::library::Library, machine: Option<&str>) -> CycleTimes {
        use forge_machine::sim::{self, Cut, Happening, Setup, SimTool};
        let mut out = CycleTimes {
            ops: vec![OpTime::default(); self.ops.len()],
            progs: vec![],
        };
        for p in &self.programs {
            let mut pq = ProgTime {
                name: p.name.clone(),
                ..ProgTime::default()
            };
            let Some(text) = p.text.as_deref() else {
                pq.error = Some(p.error.clone().unwrap_or_else(|| "no program text".into()));
                out.progs.push(pq);
                continue;
            };
            let mname = machine.map(str::to_string).unwrap_or_else(|| {
                p.setups
                    .first()
                    .and_then(|&s| self.setup_view(s))
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
                for tu in self.tool_uses() {
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
                    let names: Vec<(String, usize)> = self
                        .ops
                        .iter()
                        .enumerate()
                        .filter(|(_, o)| p.setups.contains(&o.setup))
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
                    pq.t_scurve = tl.seg_scurve.iter().sum();
                    for (i, sg) in tl.segments.iter().enumerate() {
                        if let Some(Some(k)) = op_of.get(sg.line) {
                            let o = &mut out.ops[*k];
                            o.t_min += sg.t_min;
                            o.t_stop += sg.t_stop;
                            o.t_scurve += tl.seg_scurve.get(i).copied().unwrap_or(sg.t_min);
                        }
                    }
                    for mk in &tl.marks {
                        let secs = match mk.what {
                            Happening::ToolChange { seconds, .. } => {
                                pq.tool_changes += 1;
                                seconds
                            }
                            Happening::Index { seconds, .. }
                            | Happening::Dwell { seconds }
                            | Happening::PalletChange { seconds } => seconds,
                            _ => 0.0,
                        };
                        pq.t_scurve += secs;
                        if let Some(Some(k)) = op_of.get(mk.line) {
                            let o = &mut out.ops[*k];
                            o.t_min += secs;
                            o.t_stop += secs;
                            o.t_scurve += secs;
                        }
                    }
                }
                Err(e) => pq.error = Some(e),
            }
            out.progs.push(pq);
        }
        out
    }
}
