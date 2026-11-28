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
    pub fn severity(&self, rule: Rule) -> &str {
        self.severities.get(rule.name()).map_or_else(
            || self.packages.drc.get(rule.name()).map_or("error", |s| s.word()),
            String::as_str,
        )
    }
    fn finishes(&self) -> Json {
        Json::Arr(
            self.layout
                .finishes
                .iter()
                .map(|((side, film), f)| {
                    Json::obj(vec![
                        ("side", side.name().into()),
                        ("kind", film.name().into()),
                        ("material", f.material.clone().into()),
                        ("color", f.color.as_deref().map_or(Json::Null, Into::into)),
                        ("thickness_mm", mm(f.thickness_nm).into()),
                    ])
                })
                .collect(),
        )
    }
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
                        (
                            "usage",
                            l.laminate.usage.as_deref().map_or(Json::Null, Into::into),
                        ),
                        ("dk", l.laminate.dk.map_or(Json::Null, Into::into)),
                        ("df", l.laminate.df.map_or(Json::Null, Into::into)),
                        (
                            "frequency_mhz",
                            l.laminate.frequency_mhz.map_or(Json::Null, Into::into),
                        ),
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
    pub(super) fn json(&self, board: &Board) -> Json {
        let findings = self
            .findings
            .iter()
            .map(|f| {
                let mut fields = vec![
                    ("rule", f.rule.name().into()),
                    ("severity", board.severity(f.rule).into()),
                    ("object", f.object.into()),
                    ("span", crate::error::span_json(f.span)),
                ];
                match f.witness {
                    Witness::Library {
                        ref message,
                        ref objects,
                    } => fields.extend([
                        ("message", message.clone().into()),
                        (
                            "objects",
                            Json::Arr(objects.iter().cloned().map(Into::into).collect()),
                        ),
                    ]),
                    Witness::Pair { other, layer } => {
                        fields.extend([("other", other.into()), ("layer", layer.into())])
                    }
                    Witness::Disconnected { other, net } => {
                        fields.extend([("other", other.into()), ("net", net.into())])
                    }
                    Witness::Spacing {
                        other,
                        layer,
                        segment,
                        other_segment,
                        minimum_nm,
                        measured_floor_nm,
                    } => fields.extend([
                        ("other", other.into()),
                        ("layer", layer.into()),
                        ("segment", segment.map_or(Json::Null, Into::into)),
                        ("other_segment", other_segment.map_or(Json::Null, Into::into)),
                        ("measured_floor_mm", mm(measured_floor_nm).into()),
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

fn shape_json(s: &Shape) -> Json {
    match s {
        Shape::Stroke { axis, width } => Json::obj(vec![
            ("kind", "stroke".into()),
            ("axis_mm", path(axis)),
            ("width_mm", mm(*width).into()),
        ]),
        Shape::Region(ps) => Json::obj(vec![("kind", "region".into()), ("points_mm", path(ps))]),
    }
}
fn marks_json(ms: &[artwork::Mark<'_>]) -> Json {
    Json::Arr(
        ms.iter()
            .map(|m| {
                Json::obj(vec![
                    ("source", m.id.into()),
                    ("shape", shape_json(&m.shape)),
                    ("net", m.net.map_or(Json::Null, Into::into)),
                ])
            })
            .collect(),
    )
}
/// A minimum-distance set of remaining pad-centre airwires, using physical islands
/// as vertices. Prim's cut is updated only for the island most recently joined.
pub(super) fn airwires(board: &Board) -> Vec<(&str, &str, &str, i64)> {
    let mut report = Check {
        rules: vec![],
        findings: vec![],
    };
    let mut wires = Vec::new();
    for (net, groups) in spacing::inspect(board, &mut report) {
        let pads: Vec<Vec<_>> = groups
            .into_iter()
            .map(|g| {
                g.into_iter()
                    .filter(|id| board.layout.pads.contains_key(*id))
                    .collect::<Vec<_>>()
            })
            .filter(|g| !g.is_empty())
            .collect();
        wires.extend(
            pad_tree(board, &pads, |a, b| (a[0] - b[0]).pow(2) + (a[1] - b[1]).pow(2))
                .into_iter()
                .map(|(a, b, d)| (net, a, b, d)),
        );
    }
    wires
}
pub(super) fn pad_tree<'a>(
    board: &Board,
    pads: &[Vec<&'a str>],
    metric: impl Fn(Point, Point) -> i64,
) -> Vec<(&'a str, &'a str, i64)> {
    let mut edges = Vec::new();

    let mut visited = vec![false; pads.len()];
    let mut best = vec![None; pads.len()];
    let mut current = 0;
    for _ in 1..pads.len() {
        visited[current] = true;
        for (j, group) in pads.iter().enumerate().filter(|(j, _)| !visited[*j]) {
            for &a in &pads[current] {
                for &b in group {
                    let pa = board.layout.pads[a].at;
                    let pb = board.layout.pads[b].at;
                    let dist = metric(pa, pb);
                    let candidate = (dist, a, b);
                    if best[j].is_none_or(|old| candidate < old) {
                        best[j] = Some(candidate);
                    }
                }
            }
        }
        let (j, (dist, a, b)) = best
            .iter()
            .enumerate()
            .filter(|(j, _)| !visited[*j])
            .filter_map(|(j, v)| Some((j, (*v)?)))
            .min_by_key(|(_, v)| *v)
            .unwrap();
        edges.push((a, b, dist));
        current = j;
    }
    edges
}
fn ratsnest(board: &Board) -> Json {
    Json::Arr(
        airwires(board)
            .into_iter()
            .map(|(net, a, b, dist)| {
                Json::obj(vec![
                    ("net", net.into()),
                    ("from", a.into()),
                    ("to", b.into()),
                    ("from_mm", xy(board.layout.pads[a].at)),
                    ("to_mm", xy(board.layout.pads[b].at)),
                    ("length_mm", ((dist as f64).sqrt() / 1e6).into()),
                ])
            })
            .collect(),
    )
}
pub fn response(board: &Board, words: &[String]) -> Result<Json> {
    let mut ok = true;
    let result = match words.first().map(String::as_str).unwrap_or("summary") {
        "libraries" => board.packages.json(),
        "summary" | "stat" if words.len() <= 1 => Json::obj(vec![
            (
                "classes",
                Json::Arr(
                    board
                        .classes
                        .iter()
                        .map(|(id, c)| {
                            Json::obj(vec![
                                ("id", id.clone().into()),
                                (
                                    "nets",
                                    Json::Arr(c.nets.iter().cloned().map(Into::into).collect()),
                                ),
                                ("min_width_mm", mm(c.min_width_nm).into()),
                                ("min_spacing_mm", mm(c.min_spacing_nm).into()),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("zone_count", board.layout.zones.len().into()),
            ("filled_island_count", board.layout.fills.len().into()),
            ("pad_count", board.layout.pads.len().into()),
            ("placement_count", board.layout.placements.len().into()),
            (
                "unplaced",
                Json::Arr(board.unplaced().into_iter().map(Into::into).collect()),
            ),
            (
                "cutouts",
                Json::Arr(
                    board
                        .layout
                        .cutouts
                        .iter()
                        .map(|(id, ps)| Json::obj(vec![("id", id.clone().into()), ("points_mm", path(ps))]))
                        .collect(),
                ),
            ),
            ("edge_mm", path(&board.outline)),
            ("surface_finishes", board.finishes()),
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
                        (
                            "vendor",
                            p.selected
                                .as_ref()
                                .map_or(Json::Null, |v| v.vendor.clone().into()),
                        ),
                        (
                            "service",
                            p.selected
                                .as_ref()
                                .map_or(Json::Null, |v| v.service.id.clone().into()),
                        ),
                        (
                            "source",
                            p.selected
                                .as_ref()
                                .map_or(Json::Null, |v| v.service.source.clone().into()),
                        ),
                        ("min_slot_mm", p.min_slot_nm.map_or(Json::Null, |n| mm(n).into())),
                        (
                            "inner_drill_clearance_mm",
                            p.inner_drill_clearance_nm.map_or(Json::Null, |n| mm(n).into()),
                        ),
                        ("min_edge_mm", p.min_edge_nm.map_or(Json::Null, |n| mm(n).into())),
                        (
                            "hole_clearance_mm",
                            p.hole_clearance_nm.map_or(Json::Null, |n| mm(n).into()),
                        ),
                        (
                            "min_mask_web_mm",
                            p.min_mask_web_nm.map_or(Json::Null, |n| mm(n).into()),
                        ),
                        (
                            "silk_clearance_mm",
                            p.silk_clearance_nm.map_or(Json::Null, |n| mm(n).into()),
                        ),
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
        "layers" if words.len() == 1 => Json::Arr(
            board
                .plot_layers()
                .iter()
                .map(|l| {
                    Json::obj(vec![
                        ("id", l.id().into()),
                        ("name", l.title(board).into()),
                        ("group", l.group().into()),
                    ])
                })
                .collect(),
        ),
        "finishes" if words.len() == 1 => board.finishes(),
        "artwork" if words.len() == 2 => {
            let l = board
                .plot_layers()
                .into_iter()
                .find(|l| l.id() == words[1])
                .ok_or_else(|| {
                    Fault::new(
                        "UnknownArtworkLayer",
                        Span::default(),
                        "query layers for available IDs",
                    )
                })?;
            marks_json(&board.plot(&l))
        }
        "artwork" if words.len() == 3 => {
            if words[1] == "copper"
                && board
                    .layers
                    .iter()
                    .any(|l| l.id == words[2] && matches!(l.material, LayerMaterial::Copper { .. }))
            {
                marks_json(&board.copper(&words[2]))
            } else {
                let side = Side::parse(&words[1]).ok_or_else(|| {
                    Fault::new(
                        "UnknownArtworkLayer",
                        Span::default(),
                        "use top|bottom mask|paste|silk or copper LAYER",
                    )
                })?;
                let film = Film::ALL
                    .into_iter()
                    .find(|f| f.name() == words[2])
                    .ok_or_else(|| {
                        Fault::new(
                            "UnknownArtworkLayer",
                            Span::default(),
                            "choose mask, paste or silk",
                        )
                    })?;
                marks_json(&board.film(side, film))
            }
        }
        "pads" if words.len() == 1 => Json::Arr(
            board
                .layout
                .pads
                .iter()
                .map(|(id, p)| {
                    Json::obj(vec![
                        ("id", id.clone().into()),
                        ("placement", p.placement.clone().into()),
                        ("number", p.number.clone().into()),
                        ("net", p.net.clone().into()),
                        ("at_mm", xy(p.at)),
                        ("side", p.side.name().into()),
                        ("copper", shape_json(&p.copper)),
                        ("drill_mm", p.drill.map_or(Json::Null, |n| mm(n).into())),
                    ])
                })
                .collect(),
        ),
        "ratsnest" if words.len() == 1 => ratsnest(board),
        "placements" if words.len() == 1 => Json::Arr(
            board
                .layout
                .placements
                .iter()
                .map(|(id, p)| {
                    Json::obj(vec![
                        ("id", id.clone().into()),
                        ("footprint", p.footprint.clone().into()),
                        ("occurrence", p.occurrence.clone().map_or(Json::Null, Into::into)),
                        ("side", p.pose.side.name().into()),
                        ("reference", p.reference.clone().into()),
                        ("at_mm", xy(p.pose.at)),
                        ("angle_deg", p.pose.angle.into()),
                    ])
                })
                .collect(),
        ),
        "connectivity" if words.len() == 1 => {
            let mut report = Check {
                rules: Vec::new(),
                findings: Vec::new(),
            };
            Json::Arr(
                spacing::inspect(board, &mut report)
                    .into_iter()
                    .map(|(net, groups)| {
                        Json::obj(vec![
                            ("net", net.into()),
                            (
                                "islands",
                                Json::Arr(
                                    groups
                                        .into_iter()
                                        .map(|g| Json::Arr(g.into_iter().map(Into::into).collect()))
                                        .collect(),
                                ),
                            ),
                        ])
                    })
                    .collect(),
            )
        }
        "stackup" if words.len() == 1 => board.stackup(),
        "routing" if words.len() == 1 => board.routing(),
        "check" | "drc" if words.len() == 1 => {
            let report = board.check();
            ok = !report.findings.iter().any(|f| board.severity(f.rule) == "error");
            report.json(board)
        }
        _ => {
            return Err(Fault::new(
                "UnknownBoardQuery",
                Span::default(),
                "choose summary, stackup, finishes, layers, routing, placements, pads, ratsnest, artwork, connectivity, handoff [variant], floorplan or drc",
            ));
        }
    };
    Ok(Json::obj(vec![
        ("ok", ok.into()),
        ("schema", SCHEMA.into()),
        ("result", result),
    ]))
}
