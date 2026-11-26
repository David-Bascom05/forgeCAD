use super::*;
use crate::electronics::{
    Call, Fault, Params, Result, fault, header, list, measure, number, point, reference, string,
};
use crate::syntax::{self, Expr, ExprKind, StmtKind};
use crate::values::Dim;
use std::{collections::BTreeSet, path::Path};

const SIGNATURES: &[(&str, &[&str])] = &[
    ("edge", &["points"]),
    ("copper", &["weight_oz_ft2", "thickness"]),
    ("dielectric", &["material", "thickness"]),
    ("stackup", &["layers"]),
    ("schematic", &["file"]),
    ("net", &["name", "terminal"]),
    ("track", &["net", "layer", "width", "points"]),
    ("through_via", &["net", "at", "diameter", "drill"]),
    (
        "process",
        &["min_width", "min_spacing", "min_drill", "min_annular_ring"],
    ),
];

fn nm(mm: f64, expr: &Expr) -> Result<i64> {
    let scaled = mm * 1_000_000.0;
    if scaled.abs() > 500_000_000.0 || (scaled - scaled.round()).abs() > 0.000001 {
        return Err(fault(
            expr,
            "BoardGrid",
            "use a length on the 1 nm grid within ±500 mm",
        ));
    }
    Ok(scaled.round() as i64)
}

fn length(e: &Expr, p: &Params) -> Result<i64> {
    let v = nm(measure(e, p, Dim::Length)?, e)?;
    if v <= 0 {
        return Err(fault(e, "ExpectedPositiveLength", "use a positive length"));
    }
    Ok(v)
}
fn coordinate(e: &Expr, p: &Params) -> Result<Point> {
    let [x, y] = point(e, p)?;
    Ok([nm(x, e)?, nm(y, e)?])
}
fn points(e: &Expr, p: &Params) -> Result<Vec<Point>> {
    list(e)?.iter().map(|v| coordinate(v, p)).collect()
}
fn label(e: &Expr) -> Result<String> {
    let value = string(e)?;
    if value.trim().is_empty() {
        return Err(fault(e, "EmptyName", "supply a nonempty name"));
    }
    Ok(value)
}
fn load_circuit(file: &str, e: &Expr, dir: Option<&Path>, root: Option<&Path>) -> Result<SchematicSource> {
    let bad = || fault(e, "SchematicUnavailable", format!("cannot read schematic {file}"));
    let base = dir.unwrap_or_else(|| Path::new("."));
    let fence = forge_dirs::fs::canonicalize(root.unwrap_or(base)).map_err(|_| bad())?;
    let path = forge_dirs::fs::canonicalize(base.join(file)).map_err(|_| bad())?;
    if !path.starts_with(&fence) || path.extension().and_then(|x| x.to_str()) != Some("fg") {
        return Err(bad());
    }
    let bytes = crate::deps::read_plain(&path, 8 * 1024 * 1024).map_err(|_| bad())?;
    crate::deps::read(&path, Some(&bytes));
    let src = std::str::from_utf8(&bytes).map_err(|_| bad())?;
    if header(src) != Some("schematic") {
        return Err(fault(e, "ExpectedSchematic", file));
    }
    crate::schematic::compile(src, path.parent(), Some(&fence))
        .map(|(document, circuit)| SchematicSource {
            file: file.to_owned(),
            document,
            circuit,
        })
        .map_err(|mut err| {
            err.file.get_or_insert_with(|| file.to_owned());
            err
        })
}

fn order_layers(e: &Expr, available: &mut BTreeMap<String, Layer>) -> Result<Vec<Layer>> {
    let mut layers = Vec::new();
    let mut copper_count = 0;
    let mut last_copper = false;
    for item in list(e)? {
        let id = reference(item)?;
        let layer = available.remove(&id).ok_or_else(|| {
            fault(
                item,
                "InvalidStackLayer",
                format!("layer {id} is missing or repeated"),
            )
        })?;
        let copper = matches!(layer.material, LayerMaterial::Copper { .. });
        if copper && last_copper {
            return Err(fault(
                item,
                "AdjacentCopperLayers",
                "place dielectric between copper layers",
            ));
        }
        copper_count += usize::from(copper);
        last_copper = copper;
        layers.push(layer);
    }
    if !available.is_empty() || copper_count == 0 || copper_count == layers.len() {
        return Err(fault(
            e,
            "InvalidStack",
            "list every declared layer once, including copper and dielectric",
        ));
    }
    Ok(layers)
}

pub fn compile(src: &str, dir: Option<&Path>, root: Option<&Path>) -> Result<Board> {
    let tree =
        syntax::parse(src).map_err(|e| Fault::new(e.kind(), e.span().unwrap_or_default(), e.text()))?;
    let Some((head, body)) = tree.split_first() else {
        return Err(Fault::new("ExpectedHeader", Span::default(), "expected pcb"));
    };
    if !matches!(&head.kind,StmtKind::Expr(Expr{kind:ExprKind::Name(n),..}) if n=="pcb") {
        return Err(Fault::new("ExpectedHeader", head.span, "expected pcb"));
    }
    let mut outline = None;
    let mut available = BTreeMap::new();
    let mut stack: Option<Vec<Layer>> = None;
    let mut params = Params::new();
    let mut names = BTreeSet::new();
    let mut resolved_nets = BTreeSet::new();
    let mut nets = BTreeMap::new();
    let mut tracks = BTreeMap::new();
    let mut vias = BTreeMap::new();
    let mut process = None;
    let mut schematic = None;
    for stmt in body {
        let (id, e) = match &stmt.kind {
            StmtKind::Shared { name, value } | StmtKind::Assign { name, value } => (name, value),
            _ => {
                return Err(Fault::new(
                    "ExpectedDeclaration",
                    stmt.span,
                    "use a named board declaration or shared parameter",
                ));
            }
        };
        if !names.insert(id) {
            return Err(Fault::new("DuplicateName", stmt.span, id));
        }
        if matches!(stmt.kind, StmtKind::Shared { .. }) {
            let mut n = number(e, &params)?;
            if n.dim == Dim::Any {
                n.dim = Dim::Length;
            }
            params.insert(id.clone(), n);
            continue;
        }
        let mut c = Call::with_signature(e, SIGNATURES)?;
        match c.name {
            "edge" => {
                if outline.is_some() {
                    return Err(fault(e, "DuplicateEdge", "one perimeter per rigid board"));
                }
                let expr = c.req("points")?;
                let mut path = points(expr, &params)?;
                if path.len() > 1 && path.first() == path.last() {
                    path.pop();
                }
                if !geometry::simple(&path) {
                    return Err(fault(
                        expr,
                        "InvalidBoardEdge",
                        "expected a simple, closed polygon with at least three corners",
                    ));
                }
                outline = Some(path);
            }
            "copper" | "dielectric" => {
                if stack.is_some() {
                    return Err(fault(e, "LayerAfterStack", "declare layers before the stackup"));
                }
                let thickness_nm = length(c.req("thickness")?, &params)?;
                let material = if c.name == "copper" {
                    let weight = c.req("weight_oz_ft2")?;
                    let weight_oz_ft2 = measure(weight, &params, Dim::Ratio)?;
                    if weight_oz_ft2 <= 0.0 {
                        return Err(fault(
                            weight,
                            "InvalidCopperWeight",
                            "foil weight in oz/ft² must be positive",
                        ));
                    }
                    LayerMaterial::Copper { weight_oz_ft2 }
                } else {
                    LayerMaterial::Dielectric {
                        material: label(c.req("material")?)?,
                    }
                };
                available.insert(
                    id.clone(),
                    Layer {
                        id: id.clone(),
                        thickness_nm,
                        material,
                    },
                );
            }
            "stackup" => {
                if stack.is_some() {
                    return Err(fault(e, "DuplicateStack", "one ordered stackup per board"));
                }
                stack = Some(order_layers(c.req("layers")?, &mut available)?);
            }
            "schematic" => {
                if schematic.is_some() {
                    return Err(fault(e, "DuplicateSchematic", "one schematic project per board"));
                }
                let expr = c.req("file")?;
                let file = label(expr)?;
                schematic = Some(load_circuit(&file, expr, dir, root)?);
            }
            "net" => {
                let (name, terminal) = match (c.take("name"), c.take("terminal")) {
                    (Some(v), None) => (label(v)?, None),
                    (None, Some(v)) => {
                        let pin = label(v)?;
                        let source = schematic.as_ref().ok_or_else(|| {
                            fault(
                                v,
                                "MissingSchematic",
                                "declare a schematic before a terminal-bound net",
                            )
                        })?;
                        let name = source
                            .circuit
                            .net_of
                            .get(&pin)
                            .filter(|_| source.circuit.terminals.contains_key(&pin))
                            .ok_or_else(|| fault(v, "UnknownTerminal", &pin))?;
                        (name.clone(), Some(pin))
                    }
                    _ => return Err(fault(e, "InvalidNet", "specify one of name or terminal")),
                };
                if !resolved_nets.insert(name.clone()) {
                    return Err(fault(e, "DuplicateNet", format!("net {name} already declared")));
                }
                nets.insert(id.clone(), Net { name, terminal });
            }
            "track" | "through_via" => {
                let expr = c.req("net")?;
                let net = reference(expr)?;
                if !nets.contains_key(&net) {
                    return Err(fault(expr, "UnknownBoardNet", net));
                }
                let layers = stack
                    .as_ref()
                    .ok_or_else(|| fault(e, "MissingStack", "declare the stackup before routing"))?;
                if c.name == "track" {
                    let expr = c.req("layer")?;
                    let layer = reference(expr)?;
                    if !layers
                        .iter()
                        .any(|l| l.id == layer && matches!(l.material, LayerMaterial::Copper { .. }))
                    {
                        return Err(fault(expr, "ExpectedCopperLayer", layer));
                    }
                    let width_nm = length(c.req("width")?, &params)?;
                    let expr = c.req("points")?;
                    let path = points(expr, &params)?;
                    if path.len() < 2 || path.windows(2).any(|p| p[0] == p[1]) {
                        return Err(fault(
                            expr,
                            "InvalidTrack",
                            "a track needs at least two points and nonzero segments",
                        ));
                    }
                    tracks.insert(
                        id.clone(),
                        Track {
                            net,
                            layer,
                            width_nm,
                            points: path,
                            span: stmt.span,
                        },
                    );
                } else {
                    if layers
                        .iter()
                        .filter(|l| matches!(l.material, LayerMaterial::Copper { .. }))
                        .count()
                        < 2
                    {
                        return Err(fault(
                            e,
                            "InvalidViaSpan",
                            "a through via needs at least two copper layers",
                        ));
                    }
                    let at = coordinate(c.req("at")?, &params)?;
                    let diameter_nm = length(c.req("diameter")?, &params)?;
                    let expr = c.req("drill")?;
                    let drill_nm = length(expr, &params)?;
                    if drill_nm >= diameter_nm {
                        return Err(fault(
                            expr,
                            "InvalidViaLand",
                            "drill must be smaller than land diameter",
                        ));
                    }
                    vias.insert(
                        id.clone(),
                        ThroughVia {
                            net,
                            at,
                            diameter_nm,
                            drill_nm,
                            span: stmt.span,
                        },
                    );
                }
            }
            "process" => {
                if process.is_some() {
                    return Err(fault(e, "DuplicateProcess", "one fabrication process per board"));
                }
                process = Some(Process {
                    id: id.clone(),
                    min_spacing_nm: length(c.req("min_spacing")?, &params)?,
                    min_width_nm: length(c.req("min_width")?, &params)?,
                    min_drill_nm: length(c.req("min_drill")?, &params)?,
                    min_annular_ring_nm: length(c.req("min_annular_ring")?, &params)?,
                });
            }
            _ => unreachable!("checked board signature"),
        }
        c.end()?;
    }
    Ok(Board {
        outline: outline.ok_or_else(|| Fault::new("MissingEdge", head.span, "declare the board edge"))?,
        layers: stack.ok_or_else(|| Fault::new("MissingStack", head.span, "declare an ordered stackup"))?,
        schematic,
        process,
        nets,
        tracks,
        vias,
    })
}
