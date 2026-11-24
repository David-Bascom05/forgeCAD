//! A simulation setup as data (docs/cam-machine.md, "Setup files"): which machine and control,
//! the program, the work offsets, the tools in the changer or turret with their offset
//! registers, and the stock and fixture. Our TOML subset; nothing defaulted silently except the
//! fields documented as optional.
//!
//! ```toml
//! [setup]
//! machine = "generic-vmc3"
//! dialect = "fanuc"          # a forge-post dialect name
//! program = "op1.nc"         # beside the setup file
//! step = 0.5                 # removal grid (mm); optional, 1.0
//!
//! [[setup.wcs]]
//! n = 54
//! x = 0                      # per axis letter: the joint value at word 0
//! y = 0
//! z = -500
//!
//! [[setup.tool]]
//! t = 1
//! name = "flat 10"
//! tip = [0, 0, -100]         # from the gauge point, home pose
//! body = [5, 0, 30, 8, 30, 100]   # (radius, from, to) triples from the tip
//! mill = [5, 0]              # radius, corner radius   (or: nose = [rn, dr, dz])
//! h = 100                    # its length register (mills); lathes: [[setup.geo]]
//!
//! [[setup.stock]]
//! min = [-40, -40, 0]        # a box, part frame    (or: bar = [r0, z0, z1] on a lathe)
//! max = [40, 40, 30]
//! ```
use crate::collide::Role;
use crate::def::{MachineDef, Prim};
use crate::gcode::Control;
use crate::sim::{Cut, SimError, SimTool};
use forge_tooling::toml::{self, Table, Val, boolean, num, nums, tables, text};

#[derive(Clone, Debug, PartialEq)]
pub enum Stock {
    Box { min: [f64; 3], max: [f64; 3] },
    Bar { r0: f64, z0: f64, z1: f64 },
}

/// A named workholding body in the part frame: one or more boxes and cylinders, collision-checked
/// with the part and reported by name.
#[derive(Clone, Debug, PartialEq)]
pub struct Fixture {
    pub name: String,
    pub prims: Vec<Prim>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetupSpec {
    pub machine: String,
    pub dialect: String,
    pub program: String,
    pub channel: u8,
    pub tool_mount: Option<String>,
    pub part_mount: Option<String>,
    pub dogleg: bool,
    pub feed_combined: bool,
    pub step: f64,
    /// (n, letter → value)
    pub wcs: Vec<(u32, Vec<(char, f64)>)>,
    pub tools: Vec<SimTool>,
    pub h: Vec<(u32, f64)>,
    pub geo: Vec<(u32, Vec<(char, f64)>)>,
    pub stock: Option<Stock>,
    /// Workholding (vises, clamps, plates, chucks), part frame.
    pub fixture: Vec<Fixture>,
    pub start_tool: Option<u32>,
}

fn err(what: impl Into<String>) -> String {
    what.into()
}

fn need<'a>(t: &'a Table, rec: &str, k: &str) -> Result<&'a Val, String> {
    t.get(k).ok_or_else(|| err(format!("{rec}: missing `{k}`")))
}

fn need_num(t: &Table, rec: &str, k: &str) -> Result<f64, String> {
    need(t, rec, k)?;
    num(t, k)
        .filter(|v| v.is_finite())
        .ok_or_else(|| err(format!("{rec}: `{k}` must be a number")))
}

fn need_vec<const N: usize>(t: &Table, rec: &str, k: &str) -> Result<[f64; N], String> {
    need(t, rec, k)?;
    let v = nums(t, k)
        .filter(|v| v.len() == N)
        .ok_or_else(|| err(format!("{rec}: `{k}` must be {N} numbers")))?;
    let mut a = [0.0; N];
    a.copy_from_slice(&v);
    Ok(a)
}

fn fixture_box(t: &Table, rec: &str) -> Result<Prim, String> {
    let (min, max) = (need_vec::<3>(t, rec, "min")?, need_vec::<3>(t, rec, "max")?);
    if (0..3).any(|k| min[k] >= max[k]) {
        return Err(format!("{rec}: a box needs min < max on every axis"));
    }
    Ok(Prim::Box { min, max })
}

fn whole(v: f64, rec: &str, k: &str) -> Result<u32, String> {
    if v >= 0.0 && v.fract() == 0.0 && v <= u32::MAX as f64 {
        Ok(v as u32)
    } else {
        Err(format!("{rec}: `{k}` must be a whole number"))
    }
}

fn letters(t: &Table) -> Vec<(char, f64)> {
    let mut out = Vec::new();
    for c in ['x', 'y', 'z', 'u', 'v', 'w', 'a', 'b', 'c'] {
        if let Some(v) = num(t, &c.to_string()) {
            out.push((c.to_ascii_uppercase(), v));
        }
    }
    out
}

impl SetupSpec {
    pub fn parse(src: &str) -> Result<SetupSpec, String> {
        let root = toml::parse(src).map_err(|e| format!("line {}: {}", e.line, e.what))?;
        let Some(Val::Table(s)) = root.get("setup") else {
            return Err("no [setup] table".into());
        };
        let rec = "[setup]";
        let txt = |k: &str| -> Result<String, String> {
            text(s, k)
                .map(str::to_string)
                .ok_or_else(|| err(format!("{rec}: missing `{k}`")))
        };
        let step = num(s, "step").unwrap_or(1.0);
        if step.is_nan() || step <= 0.0 {
            return Err(format!("{rec}: `step` must be > 0"));
        }
        let mut wcs = Vec::new();
        for w in tables(s, "wcs") {
            let n = whole(need_num(w, "[[setup.wcs]]", "n")?, "[[setup.wcs]]", "n")?;
            wcs.push((n, letters(w)));
        }
        let mut geo = Vec::new();
        for w in tables(s, "geo") {
            let n = whole(need_num(w, "[[setup.geo]]", "n")?, "[[setup.geo]]", "n")?;
            geo.push((n, letters(w)));
        }
        let mut tools = Vec::new();
        let mut h = Vec::new();
        for t in tables(s, "tool") {
            let r = "[[setup.tool]]";
            let tn = whole(need_num(t, r, "t")?, r, "t")?;
            let rec = format!("[[setup.tool]] t = {tn}");
            let body = nums(t, "body")
                .ok_or_else(|| err(format!("{rec}: missing `body` (radius, from, to triples)")))?;
            if body.is_empty() || body.len() % 3 != 0 || body.chunks(3).any(|c| c[0] <= 0.0 || c[1] >= c[2]) {
                return Err(format!("{rec}: `body` must be (radius > 0, from < to) triples"));
            }
            let cut = match (nums(t, "mill"), nums(t, "nose")) {
                (Some(m), None) if m.len() == 2 && m[0] > 0.0 && (0.0..=m[0]).contains(&m[1]) => Cut::Mill {
                    r: m[0],
                    corner: m[1],
                },
                (None, Some(n)) if n.len() == 3 && n[0] > 0.0 => Cut::Nose {
                    rn: n[0],
                    centre: [n[1], n[2]],
                },
                _ => {
                    return Err(format!(
                        "{rec}: give one of `mill = [r, corner]` (0 ≤ corner ≤ r) or `nose = [rn, dr, dz]`"
                    ));
                }
            };
            if let Some(hv) = num(t, "h") {
                h.push((tn, hv));
            }
            tools.push(SimTool {
                t: tn,
                name: text(t, "name").unwrap_or("").to_string(),
                tip: need_vec::<3>(t, &rec, "tip")?,
                body: body.chunks(3).map(|c| (c[0], c[1], c[2])).collect(),
                cut,
            });
        }
        let stock = match tables(s, "stock") {
            [] => None,
            [st] => {
                let r = "[[setup.stock]]";
                if st.contains_key("bar") {
                    let b = need_vec::<3>(st, r, "bar")?;
                    Some(Stock::Bar {
                        r0: b[0],
                        z0: b[1],
                        z1: b[2],
                    })
                } else {
                    Some(Stock::Box {
                        min: need_vec::<3>(st, r, "min")?,
                        max: need_vec::<3>(st, r, "max")?,
                    })
                }
            }
            _ => return Err("one [[setup.stock]] at most".into()),
        };
        let mut fixture = Vec::new();
        for (i, f) in tables(s, "fixture").iter().enumerate() {
            let name = text(f, "name")
                .map(str::to_string)
                .unwrap_or_else(|| format!("fixture {}", i + 1));
            let r = format!("[[setup.fixture]] `{name}`");
            let mut prims = Vec::new();
            // the short form: one box given on the fixture itself
            if f.contains_key("min") || f.contains_key("max") {
                prims.push(fixture_box(f, &r)?);
            }
            for b in tables(f, "box") {
                prims.push(fixture_box(b, &r)?);
            }
            for c in tables(f, "cyl") {
                let base = need_vec::<3>(c, &r, "base")?;
                let axis = need_vec::<3>(c, &r, "axis")?;
                let n = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
                let (rad, h) = (need_num(c, &r, "r")?, need_num(c, &r, "h")?);
                if !(n > 0.0 && rad > 0.0 && h > 0.0) {
                    return Err(format!("{r}: a cyl needs a non-zero axis and r, h > 0"));
                }
                prims.push(Prim::Cyl {
                    base,
                    axis: [axis[0] / n, axis[1] / n, axis[2] / n],
                    r: rad,
                    h,
                });
            }
            if prims.is_empty() {
                return Err(format!(
                    "{r}: give `min`/`max`, or [[setup.fixture.box]] / [[setup.fixture.cyl]]"
                ));
            }
            if fixture.iter().any(|x: &Fixture| x.name == name) {
                return Err(format!("{r}: two fixtures with one name"));
            }
            fixture.push(Fixture { name, prims });
        }
        let channel = num(s, "channel").unwrap_or(1.0);
        Ok(SetupSpec {
            machine: txt("machine")?,
            dialect: txt("dialect")?,
            program: txt("program")?,
            channel: whole(channel, rec, "channel")? as u8,
            tool_mount: text(s, "tool_mount").map(str::to_string),
            part_mount: text(s, "part_mount").map(str::to_string),
            dogleg: boolean(s, "dogleg").unwrap_or(false),
            feed_combined: boolean(s, "feed_combined").unwrap_or(true),
            step,
            wcs,
            tools,
            h,
            geo,
            stock,
            fixture,
            start_tool: num(s, "start_tool")
                .map(|v| whole(v, rec, "start_tool"))
                .transpose()?,
        })
    }

    /// The simulation setup on a machine, with a dialect's text (forge-post's `[dialect]` TOML).
    pub fn build<'a>(
        &self,
        def: &'a MachineDef,
        dialect_src: &str,
    ) -> Result<crate::sim::Setup<'a>, SimError> {
        let control = Control::for_machine(def, dialect_src, self.channel).map_err(SimError::Setup)?;
        let mut s = crate::sim::Setup::new(def, control)?;
        let per_letter = |vals: &[(char, f64)], what: &str| -> Result<Vec<f64>, SimError> {
            for (l, _) in vals {
                if !s.control.letters.contains(l) {
                    return Err(SimError::Setup(format!(
                        "{what}: {l} is not one of the control's letters {:?}",
                        s.control.letters
                    )));
                }
            }
            Ok(s.control
                .letters
                .iter()
                .map(|l| vals.iter().find(|(c, _)| c == l).map(|(_, v)| *v).unwrap_or(0.0))
                .collect())
        };
        let mut wcs = Vec::new();
        for (n, v) in &self.wcs {
            wcs.push((*n, per_letter(v, &format!("G{n}"))?));
        }
        let mut geo = Vec::new();
        for (n, v) in &self.geo {
            geo.push((*n, per_letter(v, &format!("offset {n}"))?));
        }
        s.wcs = wcs;
        s.geo = geo;
        s.tools = self.tools.clone();
        s.h = self.h.clone();
        s.dogleg = self.dogleg;
        s.feed_combined = self.feed_combined;
        s.start_tool = self.start_tool;
        if let Some(m) = &self.tool_mount {
            def.mount(m)
                .ok_or_else(|| SimError::Setup(format!("no mount `{m}`")))?;
            s.tool_mount = m.clone();
        }
        if let Some(m) = &self.part_mount {
            def.mount(m)
                .ok_or_else(|| SimError::Setup(format!("no mount `{m}`")))?;
            s.part_mount = m.clone();
        }
        Ok(s)
    }

    /// The workpiece's collision bodies: the stock (a box; a bar as a cylinder on the part's z)
    /// and the fixture boxes.
    pub fn work_bodies(&self, s: &crate::sim::Setup) -> Vec<crate::collide::CBody> {
        let mut out = Vec::new();
        match &self.stock {
            Some(Stock::Box { min, max }) => out.push(crate::collide::part_body(
                s,
                "stock",
                &[Prim::Box { min: *min, max: *max }],
                Role::Stock,
            )),
            Some(Stock::Bar { r0, z0, z1 }) => out.push(crate::collide::part_body(
                s,
                "stock",
                &[Prim::Cyl {
                    base: [0.0, 0.0, *z0],
                    axis: [0.0, 0.0, 1.0],
                    r: *r0,
                    h: z1 - z0,
                }],
                Role::Stock,
            )),
            None => {}
        }
        for f in &self.fixture {
            out.push(crate::collide::part_body(s, &f.name, &f.prims, Role::Fixture));
        }
        out
    }
}
