//! Tool life over the job: each milling op's posted paths measured per fixed-length sample
//! against the stock it starts from (forge_cam::life), integrated per tool with the cited
//! Taylor row for its substrate, coating and the part's material (forge_tooling::life).
//! A warning when a program would use more life than the loaded tool has left; never an error.
use crate::Cam;
use forge_cam::engage::{HeightMap, Profile};
use forge_cam::life::{ToolGeom, measure};
use forge_tooling::Kind;
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
    pub warning: Option<Warning>,
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

    /// Life use per loaded tool over `ops` (indices; all ops when empty), with `rows` the
    /// tool-life data (life::builtin() plus any user rows) and `limits` each tool number's
    /// wear limit and life already used (default: a fresh tool, limit at the row's criterion).
    pub fn tool_life(&self, rows: &[Row], ops: &[usize], limits: &BTreeMap<u32, Limit>) -> Vec<ToolLifeOut> {
        let all: Vec<usize> = if ops.is_empty() { (0..self.ops.len()).collect() } else { ops.to_vec() };
        let mut cuts: BTreeMap<usize, (Vec<Cut>, f64, usize, Vec<String>, Option<usize>)> = BTreeMap::new();
        for &oi in &all {
            let Some(o) = self.ops.get(oi) else { continue };
            let Some(ti) = o.tool else { continue };
            let e = cuts.entry(ti).or_insert_with(|| (vec![], 0.0, 0, vec![], None));
            if e.4.is_none() {
                e.4 = Some(self.setups[o.setup].part);
            }
            if o.paths.is_empty() {
                continue;
            }
            let tu = &self.tools[ti];
            let (Some(prof), Some(ip)) = (Profile::of(&tu.tool), o.stock_before.as_ref()) else {
                e.3.push(o.name.clone());
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
            for p in paths {
                let rpm = o.feeds.map(|f| f.rpm).unwrap_or(0.0);
                let m = measure(oi, p, &mut map, &prof, geom, tu.tool.dc, cell, rpm);
                e.0.extend(m.cuts);
                e.1 += m.unsampled_minutes;
                e.2 += m.cycles;
            }
        }
        let mut out = Vec::new();
        for (ti, (c, unsampled, cycles, unmeasured, part)) in cuts {
            let tu = &self.tools[ti];
            let coating = self.coating_of(&tu.tool.id);
            let mat = part.and_then(|p| self.parts[p].material.clone());
            let mut names: Vec<String> = Vec::new();
            if let Some(m) = &mat {
                names.push(m.clone());
                if let Some(fm) = forge_material::find(m) {
                    names.push(fm.name.clone());
                    names.extend(fm.aliases.iter().cloned());
                }
            }
            let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            let row = life::find(rows, "milling", &tu.tool.material, &coating, &refs);
            let why = match &mat {
                None => "the part names no material (part(.., material: \"..\"))".to_string(),
                Some(m) => format!(
                    "no cited tool-life data for {}{} milling {m}",
                    tu.tool.material,
                    if coating.is_empty() { String::new() } else { format!(" {coating}") }
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
                warning,
            });
        }
        out
    }
}
