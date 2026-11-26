//! Deterministic artwork and drill outputs in a single top-view datum. X2 mask
//! images describe openings. The FilePolarity attribute expresses that convention.
use super::*;
use crate::electronics::{Fault, Result};
use forge_agent::json::Json;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

fn mm(n: i64) -> String {
    let a = n.unsigned_abs();
    format!(
        "{}{}.{:06}",
        if n < 0 { "-" } else { "" },
        a / 1_000_000,
        a % 1_000_000
    )
}
fn coord(out: &mut String, p: Point, op: &str) {
    writeln!(out, "X{}Y{}{op}*", p[0], p[1]).unwrap();
}
fn gerber(function: &str, negative: bool, shapes: &[Shape]) -> String {
    let mut out = format!(
        "%TF.GenerationSoftware,ForgeCAD,ForgeCAD*%\n%TF.FileFunction,{function}*%\n%TF.FilePolarity,{}*%\n%FSLAX46Y46*%\n%MOMM*%\n%LPD*%\n",
        if negative { "Negative" } else { "Positive" }
    );
    let widths: BTreeSet<_> = shapes
        .iter()
        .map(|s| match s {
            Shape::Stroke { width, .. } => *width,
            Shape::Region(_) => 1,
        })
        .collect();
    let mut apertures = BTreeMap::new();
    for (i, width) in widths.into_iter().enumerate() {
        let d = i + 10;
        apertures.insert(width, d);
        writeln!(out, "%ADD{d}C,{}*%", mm(width)).unwrap();
    }
    out.push_str("G01*\n");
    for shape in shapes {
        match shape {
            Shape::Stroke { axis: [a, b], width } => {
                writeln!(out, "D{}*", apertures[width]).unwrap();
                if a == b {
                    coord(&mut out, *a, "D03");
                } else {
                    coord(&mut out, *a, "D02");
                    coord(&mut out, *b, "D01");
                }
            }
            Shape::Region(ps) => {
                writeln!(out, "D{}*\nG36*", apertures[&1]).unwrap();
                coord(&mut out, ps[0], "D02");
                for &p in ps.iter().skip(1).chain(ps.first()) {
                    coord(&mut out, p, "D01");
                }
                out.push_str("G37*\n");
            }
        }
    }
    out.push_str("M02*\n");
    out
}
fn drill(file_function: &str, holes: impl Iterator<Item = (i64, [Point; 2])>) -> String {
    let mut tools: BTreeMap<i64, Vec<[Point; 2]>> = BTreeMap::new();
    for (d, p) in holes {
        tools.entry(d).or_default().push(p);
    }
    let mut out = format!("M48\n; #@! TF.FileFunction,{file_function}\nMETRIC\nG90\n");
    for (i, &d) in tools.keys().enumerate() {
        writeln!(out, "T{:02}C{}", i + 1, mm(d)).unwrap();
    }
    out.push_str("%\nG05\n");
    for (i, ps) in tools.values().enumerate() {
        writeln!(out, "T{:02}", i + 1).unwrap();
        for &[a, b] in ps {
            if a == b {
                writeln!(out, "X{}Y{}", mm(a[0]), mm(a[1])).unwrap();
            } else {
                writeln!(
                    out,
                    "G00X{}Y{}\nM15\nG01X{}Y{}\nM16\nG05",
                    mm(a[0]),
                    mm(a[1]),
                    mm(b[0]),
                    mm(b[1])
                )
                .unwrap();
            }
        }
    }
    out.push_str("M30\n");
    out
}
fn csv(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}
impl Board {
    /// Members stay individually inspectable and testable before the CLI packages them.
    pub fn fabrication_files(&self, variant: Option<&str>) -> Result<BTreeMap<String, String>> {
        let mut files = BTreeMap::new();
        let layers: Vec<_> = self
            .layers
            .iter()
            .filter(|l| matches!(l.material, LayerMaterial::Copper { .. }))
            .collect();
        let n = layers.len();
        for (i, l) in layers.iter().enumerate() {
            let pos = if i == 0 && self.layers.first().is_some_and(|s| s.id == l.id) {
                "Top"
            } else if i + 1 == n && self.layers.last().is_some_and(|s| s.id == l.id) {
                "Bot"
            } else {
                "Inr"
            };
            files.insert(
                format!("copper-L{}-{}.gbr", i + 1, l.id),
                gerber(
                    &format!("Copper,L{},{pos}", i + 1),
                    false,
                    &self
                        .copper(&l.id)
                        .into_iter()
                        .map(|m| m.shape)
                        .collect::<Vec<_>>(),
                ),
            );
        }
        for side in Side::BOTH {
            for film in Film::ALL {
                let name = match film {
                    Film::Mask => "Soldermask",
                    Film::Paste => "Paste",
                    Film::Silk => "Legend",
                };
                let pos = if side == Side::Top { "Top" } else { "Bot" };
                files.insert(
                    format!("{}-{}.gbr", side.name(), film.name()),
                    gerber(
                        &format!("{name},{pos}"),
                        film == Film::Mask,
                        &self
                            .film(side, film)
                            .into_iter()
                            .map(|m| m.shape)
                            .collect::<Vec<_>>(),
                    ),
                );
            }
        }
        let profile: Vec<_> = std::iter::once(&self.outline)
            .chain(self.layout.cutouts.values())
            .flat_map(|ps| geometry::sides(ps).map(|axis| Shape::Stroke { axis, width: 10_000 }))
            .collect();
        files.insert("profile.gbr".into(), gerber("Profile,NP", false, &profile));
        let bores = self.drills();
        for (name, style, plated) in [
            ("plated.drl", format!("Plated,1,{n},PTH"), true),
            ("nonplated.drl", format!("NonPlated,1,{n},NPTH"), false),
        ] {
            let tools = bores
                .iter()
                .filter(|b| b.net.is_some() == plated)
                .map(|b| match b.shape {
                    Shape::Stroke { axis, width } => (width, axis),
                    _ => unreachable!("drill sweep"),
                });
            files.insert(name.into(), drill(&style, tools));
        }
        let positions_by_ref: BTreeMap<_, _> = self
            .layout
            .placements
            .values()
            .filter_map(|p| {
                Some((
                    format!("{}/{}", p.occurrence.as_ref()?.rsplit_once('/')?.0, p.reference),
                    p,
                ))
            })
            .collect();
        let mut ref_counts = BTreeMap::new();
        for p in self.layout.placements.values() {
            *ref_counts.entry(&p.reference).or_insert(0) += 1;
        }
        let ref_for = |id: &str, p: &Placement| {
            if ref_counts[&p.reference] == 1 {
                p.reference.clone()
            } else {
                p.occurrence.clone().unwrap_or_else(|| id.to_owned())
            }
        };
        let mut grouped = BTreeMap::new();
        let mut omitted = BTreeSet::new();
        if let Some(s) = &self.schematic {
            let overrides = variant
                .map(|v| {
                    s.document
                        .variants
                        .get(v)
                        .ok_or_else(|| Fault::new("UnknownVariant", Span::default(), v))
                })
                .transpose()?;
            let empty = BTreeMap::new();
            let lines = crate::schematic::bom_for(&s.document, &s.circuit, overrides.unwrap_or(&empty));
            for line in lines {
                for name in line.references {
                    let mut spec = line.part.clone();
                    let assembly_ref = if let Some(p) = positions_by_ref.get(&name) {
                        spec.footprint = Some(p.footprint.clone());
                        ref_for(&name, p)
                    } else {
                        name
                    };
                    grouped.entry(spec).or_insert_with(Vec::new).push(assembly_ref);
                }
            }
            for (scope, sheet) in &s.circuit.occurrences {
                for row in s.document.rows.iter().filter(|r| &r.sheet == sheet) {
                    if let crate::schematic::Item::Place { populated, .. } = row.item {
                        let id = format!("{scope}/{}", row.id);
                        if !overrides.and_then(|o| o.get(&id)).copied().unwrap_or(populated) {
                            omitted.insert(id);
                        }
                    }
                }
            }
        } else if variant.is_some() {
            return Err(Fault::new(
                "MissingSchematic",
                Span::default(),
                "assembly variants require a schematic",
            ));
        }
        for (id, p) in self
            .layout
            .placements
            .iter()
            .filter(|(_, p)| p.occurrence.is_none())
        {
            // A board-only placement has no symbol or purchasing fields to infer.
            let spec = crate::schematic::PartSpec {
                symbol: String::new(),
                value: String::new(),
                footprint: Some(p.footprint.clone()),
                datasheet: None,
                fields: BTreeMap::new(),
            };
            grouped.entry(spec).or_default().push(ref_for(id, p));
        }
        let lines = grouped
            .into_iter()
            .map(|(part, references)| crate::schematic::BomLine { part, references })
            .collect::<Vec<_>>();
        files.insert("bom.csv".into(), crate::schematic::bom_csv(&lines));
        let mut positions = String::from("Reference,Footprint,Side,X_mm,Y_mm,Rotation_deg,Occurrence\n");
        for (id, p) in &self.layout.placements {
            if p.occurrence.as_ref().is_some_and(|k| omitted.contains(k)) {
                continue;
            }
            writeln!(
                positions,
                "{},{},{},{},{},{},{}",
                csv(&ref_for(id, p)),
                csv(&p.footprint),
                p.pose.side.name(),
                mm(p.pose.at[0]),
                mm(p.pose.at[1]),
                p.pose.angle,
                csv(p.occurrence.as_deref().unwrap_or(""))
            )
            .unwrap();
        }
        files.insert("placements.csv".into(), positions);
        let unplaced = self.unplaced();
        files.insert(
            "review.json".into(),
            Json::obj(vec![
                ("schema", "forgecad.pcb-fabrication/1".into()),
                ("drc", self.check().json()),
                (
                    "unplaced",
                    Json::Arr(unplaced.into_iter().map(Into::into).collect()),
                ),
                ("board", response(self, &[])?),
            ])
            .to_string(),
        );
        Ok(files)
    }
    pub fn unplaced(&self) -> Vec<String> {
        let Some(s) = &self.schematic else {
            return Vec::new();
        };
        let placed: BTreeSet<_> = self
            .layout
            .placements
            .values()
            .filter_map(|p| p.occurrence.as_ref())
            .collect();
        let mut missing = Vec::new();
        for (scope, sheet) in &s.circuit.occurrences {
            for row in s.document.rows.iter().filter(|r| &r.sheet == sheet) {
                if let crate::schematic::Item::Place { symbol, .. } = &row.item
                    && s.document.symbols[symbol].power_net.is_none()
                {
                    let id = format!("{scope}/{}", row.id);
                    if !placed.contains(&id) {
                        missing.push(id);
                    }
                }
            }
        }
        missing
    }
}
