use super::*;
use crate::electronics::{Fault, Result};
use forge_agent::json::Json;

fn mm(n: i64) -> f64 {
    n as f64 / 1_000_000.0
}
fn xy([x, y]: Point) -> Json {
    Json::Arr(vec![mm(x).into(), mm(y).into()])
}
fn path(points: &[Point]) -> Json {
    Json::Arr(points.iter().copied().map(xy).collect())
}

impl Board {
    fn stackup(&self) -> Json {
        let mut depth = 0;
        Json::Arr(
            self.layers
                .iter()
                .map(|l| {
                    let top = depth;
                    depth += l.thickness_nm;
                    let mut fields = vec![
                        ("id", l.id.clone().into()),
                        ("thickness_mm", mm(l.thickness_nm).into()),
                        ("top_depth_mm", mm(top).into()),
                        ("bottom_depth_mm", mm(depth).into()),
                    ];
                    match &l.material {
                        LayerMaterial::Copper { weight_oz_ft2 } => fields.extend([
                            ("kind", "copper".into()),
                            ("weight_oz_ft2", (*weight_oz_ft2).into()),
                        ]),
                        LayerMaterial::Dielectric { material } => fields.extend([
                            ("kind", "dielectric".into()),
                            ("material", material.clone().into()),
                        ]),
                    }
                    Json::obj(fields)
                })
                .collect(),
        )
    }

    fn routing(&self) -> Json {
        let tracks = self
            .tracks
            .iter()
            .map(|(id, t)| {
                Json::obj(vec![
                    ("id", id.clone().into()),
                    ("net", t.net.clone().into()),
                    ("layer", t.layer.clone().into()),
                    ("width_mm", mm(t.width_nm).into()),
                    ("points_mm", path(&t.points)),
                ])
            })
            .collect();
        let vias = self
            .vias
            .iter()
            .map(|(id, v)| {
                Json::obj(vec![
                    ("id", id.clone().into()),
                    ("kind", "through".into()),
                    ("net", v.net.clone().into()),
                    ("at_mm", xy(v.at)),
                    ("diameter_mm", mm(v.diameter_nm).into()),
                    ("drill_mm", mm(v.drill_nm).into()),
                ])
            })
            .collect();
        Json::obj(vec![("tracks", Json::Arr(tracks)), ("vias", Json::Arr(vias))])
    }
}

impl Check<'_> {
    fn json(&self) -> Json {
        let findings = self
            .findings
            .iter()
            .map(|f| {
                let mut fields = vec![
                    ("rule", f.rule.name().into()),
                    ("object", f.object.into()),
                    ("span", crate::error::span_json(f.span)),
                ];
                match f.witness {
                    Witness::Spacing {
                        other,
                        layer,
                        segment,
                        other_segment,
                        minimum_nm,
                    } => fields.extend([
                        ("other", other.into()),
                        ("layer", layer.into()),
                        ("segment", segment.map_or(Json::Null, Into::into)),
                        ("other_segment", other_segment.map_or(Json::Null, Into::into)),
                        ("minimum_mm", mm(minimum_nm).into()),
                    ]),
                    Witness::Outside { segment } => {
                        fields.push(("segment", segment.map_or(Json::Null, Into::into)))
                    }
                    Witness::BelowMinimum {
                        twice_measured_nm,
                        minimum_nm,
                    } => fields.extend([
                        ("measured_mm", (mm(twice_measured_nm) / 2.0).into()),
                        ("minimum_mm", mm(minimum_nm).into()),
                    ]),
                }
                Json::obj(fields)
            })
            .collect();
        Json::obj(vec![
            (
                "checked_rules",
                Json::Arr(self.rules.iter().map(|r| r.name().into()).collect()),
            ),
            ("findings", Json::Arr(findings)),
        ])
    }
}

pub fn response(board: &Board, words: &[String]) -> Result<Json> {
    let mut ok = true;
    let result = match words.first().map(String::as_str).unwrap_or("summary") {
        "summary" | "stat" if words.len() <= 1 => Json::obj(vec![
            ("edge_mm", path(&board.outline)),
            ("stackup", board.stackup()),
            (
                "stack_thickness_mm",
                mm(board.layers.iter().map(|l| l.thickness_nm).sum()).into(),
            ),
            (
                "schematic",
                board
                    .schematic
                    .as_ref()
                    .map_or(Json::Null, |s| s.file.clone().into()),
            ),
            (
                "process",
                board.process.as_ref().map_or(Json::Null, |p| {
                    Json::obj(vec![
                        ("id", p.id.clone().into()),
                        ("min_width_mm", mm(p.min_width_nm).into()),
                        ("min_spacing_mm", mm(p.min_spacing_nm).into()),
                        ("min_drill_mm", mm(p.min_drill_nm).into()),
                        ("min_annular_ring_mm", mm(p.min_annular_ring_nm).into()),
                    ])
                }),
            ),
            (
                "nets",
                Json::Arr(
                    board
                        .nets
                        .iter()
                        .map(|(id, n)| {
                            Json::obj(vec![
                                ("id", id.clone().into()),
                                ("name", n.name.clone().into()),
                                ("terminal", n.terminal.clone().map_or(Json::Null, Into::into)),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("track_count", board.tracks.len().into()),
            ("via_count", board.vias.len().into()),
        ]),
        "handoff" if words.len() <= 2 => {
            let source = board.schematic.as_ref().ok_or_else(|| {
                Fault::new(
                    "MissingSchematic",
                    Span::default(),
                    "the board has no linked schematic",
                )
            })?;
            crate::schematic::board(
                &source.document,
                &source.circuit,
                words.get(1).map(String::as_str),
            )?
        }
        "stackup" if words.len() == 1 => board.stackup(),
        "routing" if words.len() == 1 => board.routing(),
        "check" | "drc" if words.len() == 1 => {
            let report = board.check();
            ok = report.findings.is_empty();
            report.json()
        }
        _ => {
            return Err(Fault::new(
                "UnknownBoardQuery",
                Span::default(),
                "choose summary, stackup, routing, handoff [variant] or drc",
            ));
        }
    };
    Ok(Json::obj(vec![
        ("ok", ok.into()),
        ("schema", SCHEMA.into()),
        ("result", result),
    ]))
}
