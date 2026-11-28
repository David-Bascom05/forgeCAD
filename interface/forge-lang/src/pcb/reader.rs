use super::*;
use crate::electronics::{
    Call, Fault, Params, Result, fault, header, list, measure, number, point, reference, string,
};
use crate::syntax::{self, Expr, ExprKind, StmtKind};
use crate::values::Dim;
use std::{collections::BTreeSet, path::Path};

const SIGNATURES: &[(&str, &[&str])] = &[
    ("library", &["file"]),
    ("drc_rule", &["rule", "severity"]),
    ("net_class", &["nets", "min_width", "min_spacing"]),
    (
        "zone",
        &[
            "net",
            "layer",
            "points",
            "clearance",
            "priority",
            "thermal_gap",
            "spoke_width",
        ],
    ),
    ("footprint", reader_layout::FOOTPRINT_ARGS),
    (
        "place",
        &[
            "footprint",
            "occurrence",
            "at",
            "angle",
            "side",
            "bindings",
            "reference",
        ],
    ),
    ("npth", &["at", "to", "drill"]),
    ("cutout", &["points"]),
    ("copper_region", &["net", "layer", "points"]),
    ("keepout", &["layer", "points", "restrict"]),
    ("silk_text", &["value", "at", "side", "height", "width", "angle"]),
    ("silk_line", &["points", "side", "width"]),
    ("mask_opening", &["side", "points"]),
    ("paste_opening", &["side", "points"]),
    ("user_layer", &["name", "plot"]),
    ("layer_dot", layers::DOT),
    ("layer_line", layers::LINE),
    ("layer_region", layers::REGION),
    ("layer_text", layers::TEXT),
    ("edge", &["points"]),
    ("copper", &["weight_oz_ft2", "thickness", "kind"]),
    (
        "surface_finish",
        &["side", "kind", "material", "thickness", "color"],
    ),
    (
        "dielectric",
        &["material", "thickness", "kind", "dk", "df", "frequency_mhz"],
    ),
    ("stackup", &["layers"]),
    ("schematic", &["file"]),
    ("net", &["name", "terminal"]),
    ("track", &["net", "layer", "width", "points"]),
    (
        "through_via",
        &["net", "at", "diameter", "drill", "mask_top", "mask_bottom"],
    ),
    (
        "process",
        &[
            "vendor",
            "service",
            "min_slot",
            "inner_drill_clearance",
            "min_width",
            "min_spacing",
            "min_drill",
            "min_annular_ring",
            "min_mask_web",
            "silk_clearance",
            "min_edge",
            "hole_clearance",
        ],
    ),
];

pub(super) fn nm(mm: f64, expr: &Expr) -> Result<i64> {
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

pub(super) fn length(e: &Expr, p: &Params) -> Result<i64> {
    let v = nm(measure(e, p, Dim::Length)?, e)?;
    if v <= 0 {
        return Err(fault(e, "ExpectedPositiveLength", "use a positive length"));
    }
    Ok(v)
}
pub(super) fn coordinate(e: &Expr, p: &Params) -> Result<Point> {
    let [x, y] = point(e, p)?;
    Ok([nm(x, e)?, nm(y, e)?])
}
pub(super) fn points(e: &Expr, p: &Params) -> Result<Vec<Point>> {
    let mut path = Vec::new();
    for v in list(e)? {
        if matches!(v.kind, ExprKind::Call(..)) {
            let mut c = Call::with_signature(v, &[("arc", &["via", "to"])])?;
            arc::append(
                &mut path,
                coordinate(c.req("via")?, p)?,
                coordinate(c.req("to")?, p)?,
                v.span,
            )?;
        } else {
            path.push(coordinate(v, p)?);
        }
    }
    Ok(path)
}
pub(super) fn label(e: &Expr) -> Result<String> {
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
    let mut packages = crate::electronics_library::Loaded::standard();
    let mut imports = library::Imports::default();
    let mut severities = BTreeMap::new();
    let mut defaults = crate::electronics::RuleDefaults::default();
    let mut classes = BTreeMap::new();
    let mut layout = Layout::default();
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
            "drc_rule" => library::policy(&mut c, &mut severities)?,
            "library" => {
                let file = label(c.req("file")?)?;
                let package = imports.load(&file, dir, root, e.span)?;
                packages
                    .merge(&package.packages)
                    .map_err(|m| fault(e, "InvalidLibrary", m))?;
                defaults.import(id, package.rules);
                for (key, def) in package.footprints {
                    layout.footprints.insert(format!("{id}.{key}"), def);
                }
            }
            "footprint" => {
                layout
                    .footprints
                    .insert(id.clone(), reader_layout::package(&mut c, &params)?);
            }
            "place" => reader_layout::place(id, &mut c, &params, &mut layout, &mut nets, schematic.as_ref())?,
            "npth" => {
                layout.holes.insert(
                    id.clone(),
                    Hole {
                        at: coordinate(c.req("at")?, &params)?,
                        to: c.take("to").map(|v| coordinate(v, &params)).transpose()?,
                        drill: length(c.req("drill")?, &params)?,
                        span: stmt.span,
                    },
                );
            }
            "cutout" => {
                layout
                    .cutouts
                    .insert(id.clone(), reader_layout::polygon(c.req("points")?, &params)?);
            }
            "zone" | "copper_region" | "keepout" => {
                let layer_expr = c.req("layer")?;
                let layer = if c.name == "keepout" && matches!(&layer_expr.kind,ExprKind::Str(s) if s=="all")
                {
                    "*".into()
                } else {
                    reference(layer_expr)?
                };
                if layer != "*"
                    && !stack.as_ref().is_some_and(|v| {
                        v.iter()
                            .any(|l| l.id == layer && matches!(l.material, LayerMaterial::Copper { .. }))
                    })
                {
                    return Err(fault(layer_expr, "ExpectedCopperLayer", layer));
                }
                let net = c.take("net").map(reference).transpose()?;
                if c.name != "keepout" && net.as_ref().is_none_or(|n| !nets.contains_key(n)) {
                    return Err(fault(
                        e,
                        "UnknownBoardNet",
                        "copper region requires a declared net",
                    ));
                }
                let polygon = reader_layout::polygon(c.req("points")?, &params)?;
                if c.name == "zone" {
                    let gap = length(c.req("clearance")?, &params)?;
                    let priority = c
                        .take("priority")
                        .map(|v| measure(v, &params, Dim::Count))
                        .transpose()?
                        .unwrap_or(0.);
                    if priority.fract() != 0. || priority.abs() > i32::MAX as f64 {
                        return Err(fault(e, "ZonePriority", "use a whole priority"));
                    }
                    let thermal = match (c.take("thermal_gap"), c.take("spoke_width")) {
                        (None, None) => None,
                        (Some(g), Some(w)) => Some((length(g, &params)?, length(w, &params)?)),
                        _ => {
                            return Err(fault(
                                e,
                                "IncompleteThermal",
                                "specify both thermal_gap and spoke_width for PTH relief",
                            ));
                        }
                    };
                    layout.zones.insert(
                        id.clone(),
                        Zone {
                            layer,
                            net: net.unwrap(),
                            boundary: polygon,
                            gap,
                            priority: priority as i32,
                            thermal,
                            span: stmt.span,
                        },
                    );
                } else {
                    let mut restrict = BTreeSet::new();
                    if let Some(arg) = c.take("restrict") {
                        for el in list(arg)? {
                            let kind = string(el)?;
                            restrict.insert(CopperObject::parse(&kind).ok_or_else(|| {
                                fault(
                                    el,
                                    "UnknownKeepoutObject",
                                    "choose track, via, smd, pth, region or pour",
                                )
                            })?);
                        }
                        if restrict.is_empty() {
                            return Err(fault(
                                arg,
                                "EmptyKeepout",
                                "list the restricted objects or omit restrict for all copper",
                            ));
                        }
                    }
                    layout.areas.insert(
                        id.clone(),
                        Area {
                            restrict,
                            layer,
                            net,
                            polygon,
                            span: stmt.span,
                        },
                    );
                }
            }
            "silk_text" | "silk_line" | "mask_opening" | "paste_opening" => {
                layout
                    .graphics
                    .insert(id.clone(), reader_layout::graphic(&mut c, &params)?);
            }
            "user_layer" => {
                let name = label(c.req("name")?)?;
                let plot = c
                    .take("plot")
                    .map(crate::electronics::boolean)
                    .transpose()?
                    .unwrap_or(false);
                layout.user_layers.insert(id.clone(), (name, plot));
            }
            "layer_dot" | "layer_text" | "layer_line" | "layer_region" => {
                let g = layers::board_graphic(&mut c, &params, &layout)?;
                layout.layer_graphics.insert(id.clone(), g);
            }
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
            "surface_finish" => {
                let (key, finish) = laminate::finish(&mut c, &params)?;
                if layout.finishes.insert(key, finish).is_some() {
                    return Err(fault(e, "DuplicateFinish", "one coating per face and role"));
                }
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
                        laminate: laminate::read(&mut c, &params)?,
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
                    let mask_top = reader_layout::margin(c.take("mask_top"), None, &params)?;
                    let mask_bottom = reader_layout::margin(c.take("mask_bottom"), None, &params)?;
                    if [mask_top, mask_bottom]
                        .into_iter()
                        .flatten()
                        .any(|m| diameter_nm + 2 * m <= 0)
                    {
                        return Err(fault(
                            e,
                            "InvalidAperture",
                            "via opening must have a positive diameter",
                        ));
                    }
                    vias.insert(
                        id.clone(),
                        ThroughVia {
                            mask_top,
                            mask_bottom,
                            net,
                            at,
                            diameter_nm,
                            drill_nm,
                            span: stmt.span,
                        },
                    );
                }
            }
            "net_class" => {
                let mut members = BTreeSet::new();
                for item in list(c.req("nets")?)? {
                    let net = reference(item)?;
                    if !nets.contains_key(&net) {
                        return Err(fault(item, "UnknownBoardNet", net));
                    }
                    members.insert(net);
                }
                if members.is_empty() {
                    return Err(fault(e, "EmptyNetClass", "select at least one declared net"));
                }
                classes.insert(
                    id.clone(),
                    NetClass {
                        nets: members,
                        min_width_nm: length(c.req("min_width")?, &params)?,
                        min_spacing_nm: length(c.req("min_spacing")?, &params)?,
                    },
                );
            }
            "process" => {
                if process.is_some() {
                    return Err(fault(e, "DuplicateProcess", "one fabrication process per board"));
                }
                process = Some(process::read(&mut c, id, &params, dir, root)?);
            }
            _ => unreachable!("checked board signature"),
        }
        c.end()?;
    }
    // A placement can precede the public alias for its schematic net.
    let aliases: BTreeMap<_, _> = nets
        .iter()
        .filter(|(k, _)| !k.starts_with('@'))
        .map(|(k, n)| (n.name.clone(), k.clone()))
        .collect();
    let renamed: BTreeMap<_, _> = nets
        .iter()
        .filter(|(k, _)| k.starts_with('@'))
        .filter_map(|(k, n)| Some((k.clone(), aliases.get(&n.name)?.clone())))
        .collect();
    for p in layout.pads.values_mut() {
        if let Some(k) = renamed.get(&p.net) {
            p.net = k.clone();
        }
    }
    for k in renamed.keys() {
        nets.remove(k);
    }
    let outline = outline.ok_or_else(|| Fault::new("MissingEdge", head.span, "declare the board edge"))?;
    for (name, cut) in &layout.cutouts {
        let shape = Shape::Region(cut.clone());
        if !shape.within(&outline)
            || layout
                .cutouts
                .range(..name.clone())
                .any(|(_, other)| shape.near(&Shape::Region(other.clone()), 0, true))
        {
            return Err(Fault::new(
                "InvalidCutout",
                head.span,
                format!("{name} must lie inside the perimeter and clear other cutouts"),
            ));
        }
    }
    defaults.resolve(&mut severities, head.span)?;
    library::validate_rules(&packages, &severities, head.span)?;
    let mut board = Board {
        packages,
        severities,
        classes,
        layout,
        outline,
        layers: stack.ok_or_else(|| Fault::new("MissingStack", head.span, "declare an ordered stackup"))?,
        schematic,
        process,
        nets,
        tracks,
        vias,
    };
    if let Some(s) = board.process.as_ref().and_then(|p| p.selected.as_ref()) {
        s.check_stack(&board.layers)?;
    }
    for p in board.layout.pads.values() {
        if p.drill.is_none() && board.surface_layer(p.side).is_none() {
            return Err(Fault::new(
                "MissingSurfaceCopper",
                p.span,
                "SMD pads require outer copper on their mounting face",
            ));
        }
    }
    board.refill()?;
    Ok(board)
}
