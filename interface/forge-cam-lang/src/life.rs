//! Tool life over the job: each milling op's posted paths measured per fixed-length sample
//! against the stock it starts from (forge_cam::life), integrated per tool with the cited
//! Taylor row for its substrate, coating and the part's material (forge_tooling::life).
//! A warning when a program would use more life than the loaded tool has left; never an error.
use crate::Cam;
use forge_cam::engage::{HeightMap, Profile};
use forge_cam::life::{Entry, ToolGeom, measure};
use forge_tooling::Kind;
use forge_tooling::entry::{self, Check, Finding};
use forge_tooling::life::{self, Cut, Limit, Row, ToolLife, Warning};
use std::collections::BTreeMap;

/// One loaded tool's life over the job.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolLifeOut {
    /// The tool number and its library record.
    pub t: u32,
    pub tool: String,
    pub life: ToolLife,
    /// Plunge/ramp minutes and canned cycles the estimate leaves out.
    pub unsampled_minutes: f64,
    pub cycles: usize,
    /// Ops of this tool the sampler cannot measure (a tool shape it has no profile for).
    pub unmeasured_ops: Vec<String>,
    /// Entry load against cited makers' guidance (forge_tooling::entry): NOT a life estimate.
    pub entry: Vec<Finding>,
    /// Entry checks with no cited rule for the part's material group (not checked).
    pub entry_not_checked: Vec<String>,
    pub warning: Option<Warning>,
}

/// The name a material is matched to tool-life rows by: forge-material's own name when the
/// library has it (an alias such as "AISI 1045" resolves to the library's "1045 cold drawn",
/// and a row for another condition of the grade must not match it by the alias), else as given.
pub fn work_name(m: &str) -> String {
    forge_material::find(m)
        .map(|f| f.name.clone())
        .unwrap_or_else(|| m.to_string())
}

#[derive(Default)]
struct Meas {
    cuts: Vec<Cut>,
    unsampled: f64,
    cycles: usize,
    unmeasured: Vec<String>,
    part: Option<usize>,
    entry: Vec<Finding>,
    not_checked: Vec<String>,
}

impl Cam {
    /// The coating text of a library tool's record (`coating = ".."`), or "".
    fn coating_of(&self, id: &str) -> String {
        self.lib
            .sources
            .iter()
            .find(|(k, i, _)| k == "tool" && i == id)
            .and_then(|(_, _, t)| forge_tooling::toml::text(t, "coating"))
            .unwrap_or("")
            .to_string()
    }

    /// The library the job was made with.
    pub fn library(&self) -> &forge_tooling::Library {
        &self.lib
    }

    /// The library id of the tool loaded as T`t`.
    pub fn tool_number_id(&self, t: u32) -> Option<String> {
        self.tools.iter().find(|u| u.t == t).map(|u| u.tool.id.clone())
    }

    /// The first part's work material, when it names one.
    pub fn material(&self) -> Option<String> {
        self.parts.iter().find_map(|p| p.material.clone())
    }

    /// Every engaged sample of T`t` over the job (all ops, in order).
    pub fn tool_cuts(&self, t: u32) -> Vec<Cut> {
        let all: Vec<usize> = (0..self.ops.len()).collect();
        self.measured(&all)
            .into_iter()
            .filter(|(ti, _)| self.tools[*ti].t == t)
            .flat_map(|(_, m)| m.cuts)
            .collect()
    }

    /// Per tool index: its samples, unsampled minutes, cycles, unmeasured ops, its part.
    fn measured(&self, ops: &[usize]) -> BTreeMap<usize, Meas> {
        let mut cuts: BTreeMap<usize, Meas> = BTreeMap::new();
        for &oi in ops {
            let Some(o) = self.ops.get(oi) else { continue };
            let Some(ti) = o.tool else { continue };
            let e = cuts.entry(ti).or_default();
            if e.part.is_none() {
                e.part = Some(self.setups[o.setup].part);
            }
            if o.paths.is_empty() {
                continue;
            }
            let tu = &self.tools[ti];
            let (Some(prof), Some(ip)) = (Profile::of(&tu.tool), o.stock_before.as_ref()) else {
                e.unmeasured.push(o.name.clone());
                continue;
            };
            let cell = (tu.tool.dc / 30.0).clamp(0.05, 0.25);
            let mut map = HeightMap::of_stock(ip, cell);
            let geom = ToolGeom {
                d: tu.tool.dc,
                flutes: tu.tool.nof,
                ball: tu.tool.kind == Kind::Ball,
            };
            // the posted paths when there are (what runs), else the made ones
            let paths = if o.posted.is_empty() { &o.paths } else { &o.posted };
            let rpm = o.feeds.map(|f| f.rpm).unwrap_or(0.0);
            let group = self.parts[self.setups[o.setup].part].group.clone();
            let rules = entry::builtin();
            for (pi, p) in paths.iter().enumerate() {
                let m = measure(oi, p, &mut map, &prof, geom, tu.tool.dc, cell, rpm);
                let fmax = m.max_cut_feed;
                for en in &m.entries {
                    let mut j = |c: Check, v: f64, mv: usize| {
                        if entry::rule_for(&rules, c, &group).is_none() {
                            let n = format!("{} (group {group}: no cited rule)", c.name());
                            if !e.not_checked.contains(&n) {
                                e.not_checked.push(n);
                            }
                        } else if let Some(f) = entry::judge(&rules, c, &group, v, (oi, pi, mv)) {
                            e.entry.push(f);
                        }
                    };
                    match *en {
                        Entry::Contact { mv, feed } if fmax > 0.0 => j(Check::EntryFeed, feed / fmax, mv),
                        Entry::Plunge { mv, feed } if fmax > 0.0 => j(Check::PlungeFeed, feed / fmax, mv),
                        Entry::Ramp { mv, deg } => j(Check::RampAngle, deg, mv),
                        Entry::Helix { mv, deg, dia_ratio } => {
                            j(Check::RampAngle, deg, mv);
                            j(Check::HelixDiameter, dia_ratio, mv);
                        }
                        _ => {}
                    }
                }
                e.cuts.extend(m.cuts.into_iter().map(|c| Cut { path: pi, ..c }));
                e.unsampled += m.unsampled_minutes;
                e.cycles += m.cycles;
            }
        }
        cuts
    }

    /// Life use per loaded tool over `ops` (indices; all ops when empty), with `rows` the
    /// tool-life data (life::builtin() plus any user rows) and `limits` each tool number's
    /// wear limit and life already used (default: a fresh tool, limit at the row's criterion).
    pub fn tool_life(&self, rows: &[Row], ops: &[usize], limits: &BTreeMap<u32, Limit>) -> Vec<ToolLifeOut> {
        let all: Vec<usize> = if ops.is_empty() {
            (0..self.ops.len()).collect()
        } else {
            ops.to_vec()
        };
        let mut out = Vec::new();
        for (ti, m) in self.measured(&all) {
            let Meas {
                cuts: c,
                unsampled,
                cycles,
                unmeasured,
                part,
                entry,
                not_checked,
            } = m;
            let tu = &self.tools[ti];
            let coating = self.coating_of(&tu.tool.id);
            let mat = part.and_then(|p| self.parts[p].material.clone());
            let names: Vec<String> = mat.iter().map(|m| work_name(m)).collect();
            let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            let row = life::find(rows, "milling", &tu.tool.material, &coating, &refs);
            let why = match &mat {
                None => "the part names no material (part(.., material: \"..\"))".to_string(),
                Some(m) => format!(
                    "no cited tool-life data for {}{} milling {m}",
                    tu.tool.material,
                    if coating.is_empty() {
                        String::new()
                    } else {
                        format!(" {coating}")
                    }
                ),
            };
            let limit = limits.get(&tu.t).copied().unwrap_or_default();
            let l = life::integrate(row, &c, limit, &why);
            let sisters: Vec<String> = self
                .tools
                .iter()
                .filter(|o| o.t != tu.t && o.tool.id == tu.tool.id)
                .map(|o| format!("T{}", o.t))
                .collect();
            let sref: Vec<&str> = sisters.iter().map(|s| s.as_str()).collect();
            let warning = life::check(&format!("T{}", tu.t), &l, limit, &sref);
            out.push(ToolLifeOut {
                t: tu.t,
                tool: tu.tool.id.clone(),
                life: l,
                unsampled_minutes: unsampled,
                cycles,
                unmeasured_ops: unmeasured,
                entry,
                entry_not_checked: not_checked,
                warning,
            });
        }
        out
    }
}

/// Where a tool reaches its wear limit, by name: the op, its path and the move in it (posted
/// paths: the same move index the checks report).
#[derive(Clone, Debug, PartialEq)]
pub struct LimitAt {
    pub op: String,
    pub path: usize,
    pub mv: Option<usize>,
    /// Minutes into the tool's cut in this job.
    pub minutes: f64,
}

/// One tool, by name (CAM-UI's Tool Life column).
#[derive(Clone, Debug, PartialEq)]
pub struct ToolLifeView {
    pub tool: String,
    pub t: u32,
    pub minutes_in_cut: f64,
    /// Fraction of the cited life used (1 = the row's wear criterion); None: not estimated.
    pub life_used: Option<f64>,
    pub wear_limit_at: Option<LimitAt>,
    /// The cite (or why not estimated).
    pub source: String,
    /// Minutes outside the data's fitted range, and plunge/ramp minutes not sampled.
    pub outside_minutes: f64,
    pub unsampled_minutes: f64,
    /// A non-blocking warning text.
    pub warning: Option<String>,
}

/// One op's share (the per-op life bar).
#[derive(Clone, Debug, PartialEq)]
pub struct OpLifeView {
    pub op: String,
    pub tool: String,
    pub t: u32,
    pub minutes: f64,
    pub fraction: Option<f64>,
}

/// An entry-load finding by name (cited makers' guidance; NOT a tool-life estimate).
#[derive(Clone, Debug, PartialEq)]
pub struct EntryView {
    pub op: String,
    pub tool: String,
    pub t: u32,
    pub path: usize,
    pub mv: usize,
    /// entry_feed | plunge_feed | ramp_angle | helix_diameter.
    pub check: &'static str,
    pub value: f64,
    pub limit: f64,
    pub why: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LifeReport {
    pub tools: Vec<ToolLifeView>,
    pub ops: Vec<OpLifeView>,
    /// Entry load, reported apart from life.
    pub entry: Vec<EntryView>,
    /// Entry checks not made (no cited rule for the material group).
    pub entry_not_checked: Vec<String>,
}

/// The job's tool-life report keyed by names. Pure: no file system, clock or threads (runs in
/// the browser). `rows`: forge_tooling::life::builtin() (plus user rows); `limits`: per tool
/// number, the life already used on the loaded instance and the change fraction.
pub fn estimate(cam: &Cam, rows: &[Row], limits: &BTreeMap<u32, Limit>) -> LifeReport {
    let mut rep = LifeReport::default();
    for t in cam.tool_life(rows, &[], limits) {
        let name = |oi: usize| cam.ops.get(oi).map(|o| o.name.clone()).unwrap_or_default();
        for f in &t.entry {
            rep.entry.push(EntryView {
                op: name(f.op),
                tool: t.tool.clone(),
                t: t.t,
                path: f.path,
                mv: f.mv,
                check: f.check.name(),
                value: f.value,
                limit: f.limit,
                why: f.why.clone(),
            });
        }
        for n in &t.entry_not_checked {
            if !rep.entry_not_checked.contains(n) {
                rep.entry_not_checked.push(n.clone());
            }
        }
        for o in &t.life.ops {
            rep.ops.push(OpLifeView {
                op: name(o.op),
                tool: t.tool.clone(),
                t: t.t,
                minutes: o.minutes,
                fraction: o.used,
            });
        }
        rep.tools.push(ToolLifeView {
            tool: t.tool.clone(),
            t: t.t,
            minutes_in_cut: t.life.minutes_in_cut,
            life_used: t.life.used,
            wear_limit_at: t.life.limit_at.map(|a| LimitAt {
                op: name(a.op),
                path: a.path,
                mv: a.mv,
                minutes: a.minutes,
            }),
            source: t.life.note.clone(),
            outside_minutes: t.life.outside,
            unsampled_minutes: t.unsampled_minutes,
            warning: t.warning.as_ref().map(|w| w.why.clone()),
        });
    }
    rep
}
