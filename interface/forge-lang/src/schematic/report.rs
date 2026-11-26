use super::graph::{Link, Terminal};
use super::*;
use forge_agent::json::Json;
fn point(p: Point) -> Json {
    Json::Arr(p.into_iter().map(Json::from).collect())
}
fn strings<'a>(s: impl IntoIterator<Item = &'a String>) -> Json {
    Json::Arr(s.into_iter().cloned().map(Json::from).collect())
}
fn role(id: &RoleId, d: &Document) -> Json {
    let r = &d.roles[id];
    let rating = match r.rating {
        None => Json::Null,
        Some(Rating::CapacitanceNf(v)) => Json::obj(vec![
            ("kind", "capacitance".into()),
            ("unit", "nF".into()),
            ("value", v.into()),
        ]),
        Some(Rating::VoltageRange(v)) => Json::obj(vec![
            ("kind", "voltage_range".into()),
            ("unit", "V".into()),
            ("min", v.min.into()),
            ("max", v.max.into()),
        ]),
        Some(Rating::VoltageMv(v)) => Json::obj(vec![
            ("kind", "voltage".into()),
            ("unit", "mV".into()),
            ("value", v.into()),
        ]),
    };
    Json::obj(vec![
        ("id", id.0.clone().into()),
        ("kind", r.kind.word().into()),
        ("rating", rating),
        ("prefer_mm", r.prefer.map_or(Json::Null, Into::into)),
    ])
}
fn roles(ids: &Set<RoleId>, d: &Document) -> Json {
    Json::Arr(ids.iter().map(|r| role(r, d)).collect())
}
fn graphic(g: &Graphic) -> Json {
    let (kind, fields) = match &g.shape {
        Shape::Rect(a, b) => ("rect", vec![("from", point(*a)), ("to", point(*b))]),
        Shape::Line(a, b) => ("line", vec![("from", point(*a)), ("to", point(*b))]),
        Shape::Arc(a, m, b) => (
            "arc",
            vec![("from", point(*a)), ("mid", point(*m)), ("to", point(*b))],
        ),
        Shape::Polyline(ps) => (
            "polyline",
            vec![("points", Json::Arr(ps.iter().copied().map(point).collect()))],
        ),
        Shape::Circle(a, r) => ("circle", vec![("at", point(*a)), ("radius", (*r).into())]),
        Shape::Text(a, t) => ("text", vec![("at", point(*a)), ("value", t.clone().into())]),
    };
    let mut fields = fields;
    fields.push(("kind", kind.into()));
    fields.push(("unit", g.unit.clone().map_or(Json::Null, Into::into)));
    Json::obj(fields)
}
fn symbol(id: &str, s: &Symbol, d: &Document) -> Json {
    Json::obj(vec![
        (
            "peripherals",
            Json::Obj(
                s.peripherals
                    .iter()
                    .map(|(id, signals)| {
                        (
                            id.clone(),
                            Json::Obj(
                                signals
                                    .iter()
                                    .map(|(signal, pins)| (signal.clone(), strings(pins)))
                                    .collect(),
                            ),
                        )
                    })
                    .collect(),
            ),
        ),
        ("ref_prefix", s.ref_prefix.clone().into()),
        ("id", id.into()),
        (
            "pins",
            Json::Arr(
                s.pins
                    .iter()
                    .map(|(id, p)| {
                        Json::obj(vec![
                            ("id", id.clone().into()),
                            ("description", p.description.clone().into()),
                            ("aliases", strings(&p.aliases)),
                            ("number", p.number.clone().into()),
                            ("unit", p.unit.clone().into()),
                            ("electrical", p.electrical.word().into()),
                            ("at", point(p.at)),
                            ("angle", p.angle.into()),
                            ("wants", roles(&p.wants, d)),
                            ("gives", roles(&p.gives, d)),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("power_net", s.power_net.clone().map_or(Json::Null, Into::into)),
        ("datasheet", s.datasheet.clone().map_or(Json::Null, Into::into)),
        ("fields", super::bom::field_json(&s.fields)),
        ("graphics", Json::Arr(s.graphics.iter().map(graphic).collect())),
    ])
}
fn link(l: &Link, d: &Document) -> Json {
    Json::obj(vec![
        ("id", l.id.clone().into()),
        ("want", l.want.clone().into()),
        ("give", l.give.clone().into()),
        ("role", role(&l.role, d)),
        ("prefer_mm", l.prefer.map_or(Json::Null, Into::into)),
        (
            "return",
            l.returns.as_ref().map_or(Json::Null, |(a, b)| {
                Json::Arr(vec![a.clone().into(), b.clone().into()])
            }),
        ),
    ])
}
fn findings(c: &Circuit) -> Json {
    Json::Arr(
        c.issues
            .iter()
            .map(|i| {
                Json::obj(vec![
                    ("kind", i.kind.into()),
                    ("severity", i.severity().into()),
                    ("objects", strings(&i.objects)),
                    (
                        "locations",
                        Json::Arr(
                            i.objects
                                .iter()
                                .filter_map(|id| {
                                    c.origins
                                        .get(id)
                                        .or_else(|| {
                                            id.split_once(':').and_then(|(owner, _)| c.origins.get(owner))
                                        })
                                        .map(|s| {
                                            Json::obj(vec![
                                                ("id", id.clone().into()),
                                                ("span", crate::error::span_json(*s)),
                                            ])
                                        })
                                })
                                .collect(),
                        ),
                    ),
                    ("message", i.message.clone().into()),
                ])
            })
            .collect(),
    )
}

fn placement_roles(roles: &Map<String, Set<RoleId>>) -> Json {
    Json::Obj(
        roles
            .iter()
            .map(|(pin, roles)| (pin.clone(), strings(roles.iter().map(|r| &r.0))))
            .collect(),
    )
}
fn scene(row: &Row) -> Json {
    let (kind, mut fields) = match &row.item {
        Item::Place {
            symbol,
            wants,
            gives,
            unit,
            value,
            footprint,
            at,
            angle,
            mirror,
            fields,
            datasheet,
            populated,
            in_bom,
            ref_offset,
            value_offset,
        } => (
            "place",
            vec![
                ("wants", placement_roles(wants)),
                ("gives", placement_roles(gives)),
                ("ref_offset", ref_offset.map_or(Json::Null, point)),
                ("value_offset", value_offset.map_or(Json::Null, point)),
                ("fields", super::bom::field_json(fields)),
                ("datasheet", datasheet.clone().map_or(Json::Null, Into::into)),
                ("populated", (*populated).into()),
                ("in_bom", (*in_bom).into()),
                ("unit", unit.clone().into()),
                ("value", value.clone().into()),
                ("footprint", footprint.clone().map_or(Json::Null, Into::into)),
                ("symbol", symbol.clone().into()),
                ("at", point(*at)),
                ("angle", (*angle).into()),
                ("mirror", (*mirror).into()),
            ],
        ),
        Item::Gate {
            of,
            unit,
            at,
            angle,
            mirror,
        } => (
            "gate",
            vec![
                ("of", of.clone().into()),
                ("unit", unit.clone().into()),
                ("at", point(*at)),
                ("angle", (*angle).into()),
                ("mirror", (*mirror).into()),
            ],
        ),
        Item::Flow { from, to } => (
            "flow",
            vec![("from", from.clone().into()), ("to", to.clone().into())],
        ),
        Item::Bus(members) => (
            "bus",
            vec![(
                "members",
                Json::Obj(
                    members
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone().into()))
                        .collect(),
                ),
            )],
        ),
        Item::SheetInstance { of, at, anchors } => (
            "subsheet",
            vec![
                (
                    "ports",
                    Json::Obj(anchors.iter().map(|(k, v)| (k.clone(), point(*v))).collect()),
                ),
                ("of", of.clone().into()),
                ("at", point(*at)),
            ],
        ),
        Item::Junction(at) => ("junction", vec![("at", point(*at))]),
        Item::Port(at) => ("port", vec![("at", point(*at))]),
        Item::Wire { from, to, via } => (
            "wire",
            vec![
                ("from", from.clone().into()),
                ("to", to.clone().into()),
                ("via", Json::Arr(via.iter().copied().map(point).collect())),
            ],
        ),
        Item::Label {
            target,
            text,
            at,
            global,
        } => (
            "label",
            vec![
                ("on", target.clone().into()),
                ("value", text.clone().into()),
                ("at", point(*at)),
                ("global", (*global).into()),
            ],
        ),
        Item::NoConnect(on) => ("no_connect", vec![("on", on.clone().into())]),
        Item::Note { at, text, size, bold } => (
            "note",
            vec![
                ("at", point(*at)),
                ("value", text.clone().into()),
                ("size", (*size).into()),
                ("bold", (*bold).into()),
            ],
        ),
        Item::Drawing { shape, width } => (
            "drawing",
            vec![
                (
                    "shape",
                    graphic(&Graphic {
                        unit: None,
                        shape: shape.clone(),
                    }),
                ),
                ("width", (*width).into()),
            ],
        ),
        Item::Fulfill { .. } => ("fulfill", vec![]),
    };
    fields.extend([
        ("id", row.id.clone().into()),
        ("kind", kind.into()),
        ("sheet", row.sheet.clone().into()),
        ("span", crate::error::span_json(row.span)),
    ]);
    Json::obj(fields)
}
fn pin(id: &str, p: &Terminal, d: &Document, c: &Circuit) -> Json {
    Json::obj(vec![
        ("id", id.into()),
        ("pin", p.pin.clone().into()),
        (
            "description",
            d.symbols[&p.symbol].pins[&p.pin].description.clone().into(),
        ),
        ("number", p.number.clone().into()),
        ("symbol", p.symbol.clone().into()),
        ("at", point(p.at)),
        ("electrical", p.electrical.word().into()),
        ("wants", roles(&p.wants, d)),
        ("gives", roles(&p.gives, d)),
        ("net", c.net_of[id].clone().into()),
    ])
}
fn net(id: &str, m: &[String], c: &Circuit) -> Json {
    Json::obj(vec![
        ("id", id.into()),
        ("nodes", strings(m)),
        ("pins", strings(m.iter().filter(|x| c.terminals.contains_key(*x)))),
    ])
}
fn query(d: &Document, c: &Circuit, w: &[String]) -> Result<Json> {
    let words: Vec<_> = w.iter().map(String::as_str).collect();
    match words.as_slice() {
        [] | ["summary"] => Ok(Json::obj(vec![
            (
                "documents",
                if d.library { "symbol_library" } else { "schematic" }.into(),
            ),
            ("roles", Json::Arr(d.roles.keys().map(|k| role(k, d)).collect())),
            (
                "role_bindings",
                Json::Obj(
                    d.role_names
                        .iter()
                        .map(|(name, id)| (name.clone(), id.0.clone().into()))
                        .collect(),
                ),
            ),
            ("symbols", strings(d.symbols.keys())),
            ("sheets", strings(c.occurrences.keys())),
            ("pin_count", c.terminals.len().into()),
            ("net_count", c.nets.len().into()),
            ("findings", findings(c)),
        ])),
        ["plan-fulfillment", args @ ..] => super::author::fulfillment(d, c, args),
        ["plan-bypass", args @ ..] => super::author::bypass(d, c, args),
        ["svg", scope] => Ok(super::drawing::sheet_svg(d, c, scope)?.into()),
        ["symbol-svg", id] => Ok(super::drawing::symbol_svg(d, id, "A")?.into()),
        ["symbol-svg", id, unit] => Ok(super::drawing::symbol_svg(d, id, unit)?.into()),
        ["erc"] => Ok(findings(c)),
        ["netlist"] => Ok(Json::Arr(
            c.nets
                .iter()
                .map(|(key, nodes)| {
                    Json::obj(vec![
                        ("id", key.clone().into()),
                        (
                            "pins",
                            Json::Arr(
                                nodes
                                    .iter()
                                    .filter_map(|k| c.terminals.get(k).map(|p| pin(k, p, d, c)))
                                    .collect(),
                            ),
                        ),
                    ])
                })
                .collect(),
        )),
        ["bom-csv"] => Ok(super::bom_csv(&super::bom(d, c)).into()),
        ["bom"] => Ok(Json::Arr(
            super::bom(d, c).iter().map(|line| line.json()).collect(),
        )),
        ["components"] => {
            let mut components = Vec::new();
            for (scope, sheet) in &c.occurrences {
                for row in d.rows.iter().filter(|r| r.sheet == *sheet) {
                    if let Item::Place {
                        symbol,
                        value,
                        footprint,
                        ..
                    } = &row.item
                    {
                        if d.symbols[symbol].power_net.is_some() {
                            continue;
                        }
                        components.push(Json::obj(vec![
                            ("occurrence", format!("{scope}/{}", row.id).into()),
                            ("symbol", symbol.clone().into()),
                            ("value", value.clone().into()),
                            ("footprint", footprint.clone().map_or(Json::Null, Into::into)),
                            ("span", crate::error::span_json(row.span)),
                        ]));
                    }
                }
            }
            Ok(Json::Arr(components))
        }
        ["nets"] => Ok(Json::Arr(c.nets.iter().map(|(k, m)| net(k, m, c)).collect())),
        ["bus", id] => c
            .buses
            .get(*id)
            .map(|m| {
                Json::Obj(
                    m.iter()
                        .map(|(k, v)| {
                            (
                                k.clone(),
                                Json::obj(vec![
                                    ("endpoint", v.clone().into()),
                                    ("net", c.net_of[v].clone().into()),
                                ]),
                            )
                        })
                        .collect(),
                )
            })
            .ok_or_else(|| Fault::new("UnknownBus", Span::default(), *id)),
        ["links"] => Ok(Json::Arr(c.links.iter().map(|l| link(l, d)).collect())),
        ["symbols"] => Ok(Json::Arr(
            d.symbols.iter().map(|(k, s)| symbol(k, s, d)).collect(),
        )),
        ["symbol", k] => d
            .symbols
            .get(*k)
            .map(|s| symbol(k, s, d))
            .ok_or_else(|| Fault::new("UnknownSymbol", Span::default(), *k)),
        ["sheet", id] => {
            let sheet = c
                .occurrences
                .get(*id)
                .ok_or_else(|| Fault::new("UnknownSheet", Span::default(), *id))?;
            Ok(Json::obj(vec![
                ("occurrence", (*id).into()),
                ("definition", sheet.clone().into()),
                ("title", d.sheets[sheet].clone().into()),
                (
                    "items",
                    Json::Arr(d.rows.iter().filter(|r| r.sheet == *sheet).map(scene).collect()),
                ),
            ]))
        }
        ["pin", k] => {
            let p = c
                .terminals
                .get(*k)
                .ok_or_else(|| Fault::new("UnknownPin", Span::default(), *k))?;
            let links = c
                .relations
                .get(*k)
                .into_iter()
                .flatten()
                .map(|i| link(&c.links[*i], d))
                .collect();
            let branch = c.drawn_branch(k);
            let mut candidates = Vec::new();
            for role_id in &p.wants {
                let mut providers = c.nets[&c.net_of[*k]]
                    .iter()
                    .filter(|other| {
                        other.as_str() != *k
                            && c.terminals.get(*other).is_some_and(|t| t.gives.contains(role_id))
                    })
                    .collect::<Vec<_>>();
                providers.sort_by_key(|other| branch.get(other.as_str()).copied().unwrap_or(usize::MAX));
                for other in providers {
                    let segments = branch.get(other.as_str());
                    let evidence = match segments {
                        Some(1) => "direct_wire",
                        Some(_) => "drawn_branch",
                        None => "same_net",
                    };
                    candidates.push(Json::obj(vec![
                        ("give", other.clone().into()),
                        ("role", role(role_id, d)),
                        ("evidence", evidence.into()),
                        ("drawn_segments", segments.map_or(Json::Null, |n| (*n).into())),
                    ]));
                }
            }
            Ok(Json::obj(vec![
                ("terminal", pin(k, p, d, c)),
                (
                    "voltage_checks",
                    Json::Arr(
                        c.voltages
                            .get(*k)
                            .into_iter()
                            .flatten()
                            .map(|(id, check)| {
                                Json::obj(vec![
                                    ("want", role(id, d)),
                                    ("status", check.status().into()),
                                    (
                                        "providers",
                                        Json::Arr(
                                            check
                                                .providers
                                                .iter()
                                                .map(|source| {
                                                    Json::obj(vec![
                                                        ("pin", source.pin.clone().into()),
                                                        ("give", role(&source.role, d)),
                                                        ("min_v", source.range.min.into()),
                                                        ("max_v", source.range.max.into()),
                                                        (
                                                            "accepted",
                                                            check.required.accepts(source.range).into(),
                                                        ),
                                                    ])
                                                })
                                                .collect(),
                                        ),
                                    ),
                                ])
                            })
                            .collect(),
                    ),
                ),
                ("fulfillments", Json::Arr(links)),
                ("candidates", Json::Arr(candidates)),
            ]))
        }
        ["tree", id] => Ok(Json::Arr(
            c.rooted(id)?
                .into_iter()
                .map(|step| {
                    Json::obj(vec![
                        ("kind", step.kind.into()),
                        ("id", step.id.map_or(Json::Null, Into::into)),
                        ("from", step.from.into()),
                        ("to", step.to.into()),
                        ("revisit", step.revisit.into()),
                    ])
                })
                .collect(),
        )),
        _ => Err(Fault::new(
            "UnknownSchematicQuery",
            Span::default(),
            "use summary, erc, symbols, symbol ID, sheet PATH, nets, links, pin PATH or tree PATH",
        )),
    }
}
pub fn response(d: &Document, c: &Circuit, words: &[String]) -> Result<Json> {
    Ok(Json::obj(vec![
        ("schema", "forgecad.schematic/1".into()),
        ("ok", true.into()),
        (
            "erc_ok",
            (!c.issues.iter().any(|i| i.severity() == "error")).into(),
        ),
        ("result", query(d, c, words)?),
    ]))
}
impl Fault {
    pub fn json(&self) -> Json {
        Json::obj(vec![
            ("schema", "forgecad.schematic/1".into()),
            ("ok", false.into()),
            (
                "error",
                Json::obj(vec![
                    ("kind", self.kind.into()),
                    ("message", self.message.clone().into()),
                    ("span", crate::error::span_json(self.span)),
                    ("file", self.file.clone().map_or(Json::Null, Into::into)),
                ]),
            ),
        ])
    }
}
