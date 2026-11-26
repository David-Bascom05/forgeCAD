use super::reader::{coordinate, label, length, nm, points};
use super::*;
use crate::electronics::{Call, Fault, Params, Result, fault, list, measure, reference, string};
use crate::syntax::{Expr, ExprKind};
use crate::values::Dim;
use std::collections::{BTreeMap, BTreeSet};

const PAD_ARGS: &[&str] = &["number", "at", "size", "shape", "mask", "paste"];
pub(super) fn margin(e: Option<&Expr>, default: Option<i64>, p: &Params) -> Result<Option<i64>> {
    match e {
        None => Ok(default),
        Some(e) => match e.kind {
            ExprKind::Bool(false) => Ok(None),
            ExprKind::Bool(true) => Ok(Some(0)),
            _ => Ok(Some(nm(measure(e, p, Dim::Length)?, e)?)),
        },
    }
}
fn side(e: &Expr) -> Result<Side> {
    Side::parse(&string(e)?).ok_or_else(|| fault(e, "UnknownBoardSide", "use top or bottom"))
}
fn pose(c: &mut Call<'_>, p: &Params) -> Result<Pose> {
    Ok(Pose {
        at: coordinate(c.req("at")?, p)?,
        side: side(c.req("side")?)?,
        angle: c
            .take("angle")
            .map(|e| measure(e, p, Dim::Angle))
            .transpose()?
            .unwrap_or(0.)
            .rem_euclid(360.),
    })
}
pub(super) fn polygon(e: &Expr, p: &Params) -> Result<Vec<Point>> {
    let mut path = points(e, p)?;
    if path.len() > 1 && path.first() == path.last() {
        path.pop();
    }
    if !geometry::simple(&path) {
        return Err(fault(e, "InvalidPolygon", "use a simple closed polygon"));
    }
    Ok(path)
}
pub(super) const FOOTPRINT_ARGS: &[&str] = &["pads", "courtyard", "legend", "paste_windows"];
pub(super) fn package(call: &mut Call<'_>, p: &Params) -> Result<Footprint> {
    let courtyard = call.take("courtyard").map(|e| polygon(e, p)).transpose()?;
    let mut legend = Vec::new();
    if let Some(items) = call.take("legend") {
        for e in list(items)? {
            let mut c = Call::with_signature(
                e,
                &[
                    ("silk_text", &["at", "value", "height", "width"]),
                    ("silk_line", &["points", "width"]),
                ],
            )?;
            let width = length(c.req("width")?, p)?;
            legend.push(if c.name == "silk_text" {
                Legend::Text {
                    at: coordinate(c.req("at")?, p)?,
                    value: label(c.req("value")?)?,
                    height: length(c.req("height")?, p)?,
                    width,
                }
            } else {
                let ps = points(c.req("points")?, p)?;
                if ps.len() < 2 {
                    return Err(fault(e, "InvalidLegendLine", "use at least two points"));
                }
                Legend::Line { points: ps, width }
            });
        }
    }
    let mut paste_windows = Vec::new();
    if let Some(e) = call.take("paste_windows") {
        for ps in list(e)? {
            paste_windows.push(polygon(ps, p)?);
        }
    }
    let e = call.req("pads")?;
    let mut pads = Vec::new();
    let mut numbers = BTreeSet::new();
    for entry in list(e)? {
        let mut c = Call::with_signature(
            entry,
            &[
                ("smd", PAD_ARGS),
                (
                    "pth",
                    &[
                        "number",
                        "at",
                        "size",
                        "shape",
                        "mask",
                        "paste",
                        "drill",
                        "slot_length",
                    ],
                ),
            ],
        )?;
        let number = label(c.req("number")?)?;
        if !numbers.insert(number.clone()) {
            return Err(fault(entry, "DuplicatePadNumber", number));
        }
        let at = coordinate(c.req("at")?, p)?;
        let size = coordinate(c.req("size")?, p)?;
        if size.iter().any(|v| *v <= 0) {
            return Err(fault(entry, "InvalidPadSize", "pad size must be positive"));
        }
        let drill = if c.name == "pth" {
            Some(length(c.req("drill")?, p)?)
        } else {
            None
        };
        if drill.is_some_and(|d| d >= size[0].min(size[1])) {
            return Err(fault(
                entry,
                "InvalidPadDrill",
                "drill must fit inside both land dimensions",
            ));
        }
        let shape = match c
            .take("shape")
            .map(string)
            .transpose()?
            .as_deref()
            .unwrap_or(if drill.is_some() { "round" } else { "rect" })
        {
            "rect" => PadShape::Rect,
            "round" if size[0] == size[1] => PadShape::Round,
            "oval" => PadShape::Oval,
            _ => {
                return Err(fault(
                    entry,
                    "InvalidPadShape",
                    "use rect, oval, or round with equal dimensions",
                ));
            }
        };
        let slot_length = c.take("slot_length").map(|e| length(e, p)).transpose()?;
        if slot_length
            .is_some_and(|v| v < drill.unwrap_or(0) || v >= size[0].max(size[1]) || shape == PadShape::Round)
        {
            return Err(fault(
                entry,
                "InvalidSlot",
                "a plated slot follows the long pad axis and must fit inside its land",
            ));
        }
        let def = PadDef {
            number,
            at,
            size,
            shape,
            drill,
            slot_length,
            mask: margin(c.take("mask"), Some(0), p)?,
            paste: margin(c.take("paste"), drill.is_none().then_some(0), p)?,
        };
        for m in [def.mask, def.paste].into_iter().flatten() {
            if size.iter().any(|v| v + 2 * m <= 0) {
                return Err(fault(entry, "InvalidAperture", "margin consumes the entire pad"));
            }
        }
        c.end()?;
        pads.push(def);
    }
    if pads.is_empty() {
        return Err(fault(e, "MissingPads", "declare at least one pad"));
    }
    Ok(Footprint {
        pads,
        courtyard,
        legend,
        paste_windows,
    })
}

fn pin_map(source: &SchematicSource, id: &str, e: &Expr) -> Result<(String, BTreeMap<String, String>)> {
    let missing = || fault(e, "UnknownBoardOccurrence", id);
    let (scope, key) = id.rsplit_once('/').ok_or_else(missing)?;
    let sheet = source.circuit.occurrences.get(scope).ok_or_else(missing)?;
    let row = source
        .document
        .rows
        .iter()
        .find(|r| r.sheet == *sheet && r.id == key)
        .ok_or_else(missing)?;
    let crate::schematic::Item::Place {
        symbol, reference, ..
    } = &row.item
    else {
        return Err(missing());
    };
    let sym = &source.document.symbols[symbol];
    if sym.power_net.is_some() {
        return Err(missing());
    }
    let mut pins = BTreeMap::new();
    for (name, pin) in &sym.pins {
        let terminal = format!("{id}.{name}");
        for num in &pin.pads {
            if let Some(old) = pins.insert(num.clone(), terminal.clone())
                && source.circuit.net_of[&old] != source.circuit.net_of[&terminal]
            {
                return Err(fault(e, "ConflictingPadNets", num));
            }
        }
    }
    let reference = source.document.reference(id, reference.as_deref().unwrap_or(key));
    Ok((reference.to_owned(), pins))
}
pub(super) fn place(
    id: &str,
    c: &mut Call<'_>,
    p: &Params,
    layout: &mut Layout,
    nets: &mut BTreeMap<String, Net>,
    source: Option<&SchematicSource>,
) -> Result<()> {
    let e = c.req("footprint")?;
    let footprint = reference(e)?;
    let def = layout
        .footprints
        .get(&footprint)
        .ok_or_else(|| fault(e, "UnknownFootprint", &footprint))?;
    let pose = pose(c, p)?;
    let occurrence = c.take("occurrence").map(label).transpose()?;
    let (designator, map) = if let Some(occ) = &occurrence {
        if layout
            .placements
            .values()
            .any(|v| v.occurrence.as_ref() == Some(occ))
        {
            return Err(fault(e, "DuplicatePlacement", occ));
        }
        let source = source.ok_or_else(|| fault(e, "MissingSchematic", "occurrence requires a schematic"))?;
        let (r, pins) = pin_map(source, occ, e)?;
        let mut map = BTreeMap::new();
        for (k, terminal) in pins {
            let name = &source.circuit.net_of[&terminal];
            let key = nets
                .iter()
                .find(|(_, n)| n.name == *name)
                .map(|(id, _)| id.clone())
                .unwrap_or_else(|| format!("@{name}"));
            nets.entry(key.clone()).or_insert_with(|| Net {
                name: name.clone(),
                terminal: Some(terminal),
            });
            map.insert(k, key);
        }
        (r, map)
    } else {
        let mut map = BTreeMap::new();
        for entry in list(c.req("bindings")?)? {
            let mut b = Call::with_signature(entry, &[("bind", &["pad", "net"])])?;
            let k = label(b.req("pad")?)?;
            let n = reference(b.req("net")?)?;
            if !nets.contains_key(&n) {
                return Err(fault(entry, "UnknownBoardNet", n));
            }
            if map.insert(k.clone(), n).is_some() {
                return Err(fault(entry, "DuplicatePadBinding", k));
            }
        }
        (
            c.take("reference")
                .map(label)
                .transpose()?
                .unwrap_or_else(|| id.into()),
            map,
        )
    };
    if map.len() != def.pads.len() || def.pads.iter().any(|v| !map.contains_key(&v.number)) {
        return Err(fault(
            e,
            "PadMapMismatch",
            format!(
                "expected footprint pad numbers {:?}, got {:?}",
                def.pads.iter().map(|p| &p.number).collect::<Vec<_>>(),
                map.keys().collect::<Vec<_>>()
            ),
        ));
    }
    for d in &def.pads {
        let make = |m| d.artwork(pose, m, c.span);
        layout.pads.insert(
            format!("{id}.{}", d.number),
            Pad {
                placement: id.into(),
                number: d.number.clone(),
                net: map[&d.number].clone(),
                side: pose.side,
                at: parts::transform(pose, d.at, c.span)?,
                copper: make(0)?,
                mask: d.mask.map(make).transpose()?,
                paste: d.paste.map(make).transpose()?,
                drill: d.drill,
                drill_axis: d.drill_axis(pose, c.span)?,
                min_diameter: d.size[0]
                    .min(d.size[1])
                    .min(d.size[0].max(d.size[1]) - d.slot_length.unwrap_or(0) + d.drill.unwrap_or(0)),
                span: c.span,
            },
        );
    }
    if !def.paste_windows.is_empty() {
        let shapes = def
            .paste_windows
            .iter()
            .map(|ps| {
                ps.iter()
                    .map(|&v| parts::transform(pose, v, c.span))
                    .collect::<Result<Vec<_>>>()
                    .map(Shape::Region)
            })
            .collect::<Result<Vec<_>>>()?;
        layout.graphics.insert(
            format!("{id}:stencil"),
            Graphic {
                film: Film::Paste,
                side: pose.side,
                shapes,
                span: c.span,
            },
        );
    }
    for (i, l) in def.legend.iter().enumerate() {
        let shapes = match l {
            Legend::Text {
                at,
                value,
                height,
                width,
            } => parts::legend(
                &value.replace("{ref}", &designator),
                *height,
                *width,
                Pose {
                    at: parts::transform(pose, *at, c.span)?,
                    ..pose
                },
                c.span,
            )?,
            Legend::Line { points, width } => {
                let p = points
                    .iter()
                    .map(|&v| parts::transform(pose, v, c.span))
                    .collect::<Result<Vec<_>>>()?;
                p.windows(2)
                    .map(|s| Shape::Stroke {
                        axis: [s[0], s[1]],
                        width: *width,
                    })
                    .collect()
            }
        };
        layout.graphics.insert(
            format!("{id}:legend{i}"),
            Graphic {
                film: Film::Silk,
                side: pose.side,
                shapes,
                span: c.span,
            },
        );
    }
    let courtyard = def
        .courtyard
        .as_ref()
        .map(|ps| {
            ps.iter()
                .map(|&v| parts::transform(pose, v, c.span))
                .collect::<Result<Vec<_>>>()
                .map(Shape::Region)
        })
        .transpose()?;
    layout.placements.insert(
        id.into(),
        Placement {
            footprint,
            occurrence,
            courtyard,
            reference: designator,
            pose,
            span: c.span,
        },
    );
    Ok(())
}
pub(super) fn graphic(c: &mut Call<'_>, p: &Params) -> Result<Graphic> {
    let span = c.span;
    if c.name == "silk_text" {
        let pose = pose(c, p)?;
        let value = label(c.req("value")?)?;
        let height = length(c.req("height")?, p)?;
        let width = length(c.req("width")?, p)?;
        return Ok(Graphic {
            film: Film::Silk,
            side: pose.side,
            shapes: parts::legend(&value, height, width, pose, span)?,
            span,
        });
    }
    let side = side(c.req("side")?)?;
    let (film, shapes) = match c.name {
        "silk_line" => {
            let ps = points(c.req("points")?, p)?;
            let width = length(c.req("width")?, p)?;
            if ps.len() < 2 {
                return Err(Fault::new("InvalidLegendLine", span, "use at least two points"));
            }
            (
                Film::Silk,
                ps.windows(2)
                    .map(|v| Shape::Stroke {
                        axis: [v[0], v[1]],
                        width,
                    })
                    .collect(),
            )
        }
        name => (
            if name == "paste_opening" {
                Film::Paste
            } else {
                Film::Mask
            },
            vec![Shape::Region(polygon(c.req("points")?, p)?)],
        ),
    };
    Ok(Graphic {
        film,
        side,
        shapes,
        span,
    })
}
