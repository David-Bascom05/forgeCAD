use super::*;
use crate::syntax::{self, Arg, BinOp, Expr, ExprKind as E, StmtKind as S, Unit};
use crate::values::{self, Dim, Num};
use std::path::PathBuf;

struct Call<'a> {
    name: &'a str,
    args: Map<&'a str, &'a Expr>,
    span: Span,
}
impl<'a> Call<'a> {
    fn new(expr: &'a Expr) -> Result<Self> {
        if let E::Call(head, args) = &expr.kind
            && let E::Name(name) = &head.kind
        {
            let signature = ARGS
                .iter()
                .find(|(word, _)| *word == name)
                .ok_or_else(|| fault(head, "UnknownBuiltin", name))?
                .1;
            let mut fields = Map::new();
            for Arg { name: key, value } in args {
                let Some((key, span)) = key else {
                    return Err(fault(value, "ExpectedNamedArgument", "use a named argument"));
                };
                if !signature.contains(&key.as_str()) {
                    return Err(Fault::new(
                        "UnknownArgument",
                        *span,
                        format!("{name} has no argument {key}"),
                    ));
                }
                if fields.insert(key.as_str(), value).is_some() {
                    return Err(Fault::new("DuplicateArgument", *span, key));
                }
            }
            Ok(Self {
                name,
                args: fields,
                span: expr.span,
            })
        } else {
            Err(fault(expr, "ExpectedCall", "expected a schematic declaration"))
        }
    }
    fn take(&mut self, key: &str) -> Option<&'a Expr> {
        self.args.remove(key)
    }
    fn req(&mut self, key: &str) -> Result<&'a Expr> {
        self.take(key).ok_or_else(|| {
            Fault::new(
                "MissingArgument",
                self.span,
                format!("{} requires {key}", self.name),
            )
        })
    }
    fn end(self) -> Result<()> {
        if let Some((k, e)) = self.args.first_key_value() {
            Err(fault(
                e,
                "UnknownArgument",
                format!("{} has no argument {k}", self.name),
            ))
        } else {
            Ok(())
        }
    }
}
fn fault(e: &Expr, k: &'static str, msg: impl Into<String>) -> Fault {
    Fault::new(k, e.span, msg)
}
fn string(e: &Expr) -> Result<String> {
    match &e.kind {
        E::Str(s) => Ok(s.clone()),
        _ => Err(fault(e, "ExpectedString", "expected quoted text")),
    }
}
fn boolean(e: &Expr) -> Result<bool> {
    match e.kind {
        E::Bool(x) => Ok(x),
        _ => Err(fault(e, "ExpectedBool", "expected true or false")),
    }
}
fn reference(e: &Expr) -> Result<String> {
    match &e.kind {
        E::Name(s) => Ok(s.clone()),
        E::Attr(base, k) => Ok(format!("{}.{k}", reference(base)?)),
        _ => Err(fault(
            e,
            "ExpectedReference",
            "expected a declared name or pin reference",
        )),
    }
}
fn list(e: &Expr) -> Result<&[Expr]> {
    if let E::List(xs) = &e.kind {
        Ok(xs)
    } else {
        Err(fault(e, "ExpectedList", "expected a list"))
    }
}
type Params = Map<String, Num>;
fn number(e: &Expr, params: &Params) -> Result<Num> {
    let n = match &e.kind {
        E::Num(x, u) => match u {
            Unit::In => Num::konst(x * 25.4, Dim::Length),
            Unit::Mm => Num::konst(*x, Dim::Length),
            Unit::Deg => Num::konst(*x, Dim::Angle),
            Unit::None => Num::konst(*x, Dim::Any),
        },
        E::Neg(v) => values::neg(&number(v, params)?),
        E::Name(k) => params
            .get(k)
            .cloned()
            .ok_or_else(|| fault(e, "UnknownParameter", k))?,
        E::Bin(op, l, r) => {
            let (l, r) = (number(l, params)?, number(r, params)?);
            match op {
                BinOp::Add | BinOp::Sub => values::add(&l, &r, *op == BinOp::Sub, e.span),
                BinOp::Mul => values::mul(&l, &r, e.span),
                BinOp::Div => values::div(&l, &r, e.span),
                _ => return Err(fault(e, "ExpectedQuantity", "expected arithmetic")),
            }
            .map_err(|err| fault(e, err.kind(), err.text()))?
        }
        _ => return Err(fault(e, "ExpectedQuantity", "expected a numeric expression")),
    };
    if !n.v.is_finite() || n.v.abs() > 1e12 {
        return Err(fault(
            e,
            "QuantityOutOfRange",
            "quantity exceeds schematic bounds",
        ));
    }
    Ok(n)
}
fn measure(e: &Expr, p: &Params, d: Dim) -> Result<f64> {
    let n = number(e, p)?;
    if n.dim != Dim::Any && n.dim != d {
        return Err(fault(
            e,
            "WrongDim",
            format!("expected {}, got {}", d.name(), n.dim.name()),
        ));
    }
    Ok(n.v)
}
fn positive(e: &Expr, p: &Params) -> Result<f64> {
    let v = measure(e, p, Dim::Length)?;
    if v <= 0.0 {
        return Err(fault(e, "ExpectedPositiveLength", "expected a positive length"));
    }
    Ok(v)
}
fn point(e: &Expr, p: &Params) -> Result<Point> {
    if let E::Tuple(xs) = &e.kind
        && let [x, y] = xs.as_slice()
    {
        return Ok([measure(x, p, Dim::Length)?, measure(y, p, Dim::Length)?]);
    }
    Err(fault(e, "ExpectedPoint", "expected (x, y) in sheet units"))
}
fn rotation(e: Option<&Expr>, p: &Params) -> Result<f64> {
    let angle = e.map(|v| measure(v, p, Dim::Angle)).transpose()?.unwrap_or(0.0);
    if angle % 90.0 != 0.0 {
        return Err(Fault::new(
            "BadRotation",
            e.map(|v| v.span).unwrap_or_default(),
            "symbol rotation must be a multiple of 90 degrees",
        ));
    }
    Ok(angle.rem_euclid(360.0))
}
fn resolve_role(e: &Expr, doc: &Document) -> Result<RoleId> {
    let name = reference(e)?;
    doc.role_names
        .get(&name)
        .cloned()
        .ok_or_else(|| fault(e, "UnknownRole", name))
}
fn register_role(doc: &mut Document, name: String, id: RoleId, role: Role, e: &Expr) -> Result<()> {
    if doc.roles.get(&id).is_some_and(|old| *old != role) {
        return Err(fault(e, "ConflictingRoleDefinition", id.0));
    }
    doc.role_names.insert(name, id.clone());
    doc.roles.insert(id, role);
    Ok(())
}
fn roles(e: Option<&Expr>, doc: &Document) -> Result<Set<RoleId>> {
    let mut result = Set::new();
    if let Some(e) = e {
        for x in list(e)? {
            let id = resolve_role(x, doc)?;
            if !result.insert(id) {
                return Err(fault(x, "DuplicateRole", "role listed twice"));
            }
        }
    }
    Ok(result)
}
fn symbol(call: &mut Call<'_>, p: &Params, doc: &Document) -> Result<Symbol> {
    let e = call.req("pins")?;
    let E::Map(entries) = &e.kind else {
        return Err(fault(e, "ExpectedPinMap", "expected {pin_name: pin(...)}"));
    };
    let mut pins = Map::new();
    let mut numbers = Set::new();
    for (name, expr) in entries {
        let mut c = Call::new(expr)?;
        if c.name != "pin" {
            return Err(fault(expr, "ExpectedPin", "expected pin(...)"));
        }
        let n = string(c.req("number")?)?;
        if n.trim().is_empty() || !numbers.insert(n.clone()) {
            return Err(fault(expr, "InvalidPinNumber", n));
        }
        let k = c.req("kind")?;
        let word = string(k)?;
        let electrical = Electrical::parse(&word).ok_or_else(|| fault(k, "UnknownElectricalKind", word))?;
        let pin = Pin {
            number: n,
            unit: c
                .take("unit")
                .map(string)
                .transpose()?
                .unwrap_or_else(|| "A".into()),
            electrical,
            at: point(c.req("at")?, p)?,
            angle: rotation(c.take("angle"), p)?,
            wants: roles(c.take("wants"), doc)?,
            gives: roles(c.take("gives"), doc)?,
        };
        c.end()?;
        if pins.insert(name.clone(), pin).is_some() {
            return Err(fault(expr, "DuplicatePin", name));
        }
    }
    let mut graphics = Vec::new();
    if let Some(g) = call.take("graphics") {
        for expr in list(g)? {
            let mut c = Call::new(expr)?;
            let unit = c.take("unit").map(string).transpose()?;
            let shape = match c.name {
                "line" => Shape::Line(point(c.req("from")?, p)?, point(c.req("to")?, p)?),
                "rect" => Shape::Rect(point(c.req("from")?, p)?, point(c.req("to")?, p)?),
                "circle" => Shape::Circle(point(c.req("at")?, p)?, positive(c.req("radius")?, p)?),
                "text" => Shape::Text(point(c.req("at")?, p)?, string(c.req("value")?)?),
                "arc" => Shape::Arc(
                    point(c.req("from")?, p)?,
                    point(c.req("mid")?, p)?,
                    point(c.req("to")?, p)?,
                ),
                "polyline" => {
                    let e = c.req("points")?;
                    let points = list(e)?.iter().map(|e| point(e, p)).collect::<Result<Vec<_>>>()?;
                    if points.len() < 2 {
                        return Err(fault(e, "TooFewPoints", "a polyline needs at least two points"));
                    }
                    Shape::Polyline(points)
                }
                _ => return Err(fault(expr, "UnknownGraphic", c.name)),
            };
            graphics.push(Graphic { unit, shape });
            c.end()?;
        }
    }
    let power_net = if let Some(e) = call.take("power_net") {
        let name = string(e)?;
        if name.trim().is_empty()
            || pins.len() != 1
            || pins.values().any(|p| p.electrical != Electrical::Passive)
        {
            return Err(fault(
                e,
                "InvalidPowerSymbol",
                "A power symbol needs one passive pin and a nonempty net name",
            ));
        }
        Some(name)
    } else {
        None
    };
    Ok(Symbol {
        power_net,
        pins,
        graphics,
        fields: fields(call)?,
        datasheet: call
            .take("datasheet")
            .map(string)
            .transpose()?
            .filter(|s| !s.is_empty()),
    })
}
fn map<T>(e: &Expr, mut f: impl FnMut(&Expr) -> Result<T>) -> Result<Map<String, T>> {
    let E::Map(entries) = &e.kind else {
        return Err(fault(e, "ExpectedMap", "expected named members"));
    };
    let mut out = Map::new();
    for (k, v) in entries {
        if out.insert(k.clone(), f(v)?).is_some() {
            return Err(fault(v, "DuplicateMember", k));
        }
    }
    Ok(out)
}
fn fields(c: &mut Call<'_>) -> Result<Map<String, String>> {
    Ok(c.take("fields")
        .map(|v| map(v, string))
        .transpose()?
        .unwrap_or_default())
}
fn field_offset(expr: Option<&Expr>, p: &Params) -> Result<Option<Point>> {
    expr.filter(|e| !matches!(&e.kind,E::Name(n) if n=="auto"))
        .map(|e| point(e, p))
        .transpose()
}
fn item(c: &mut Call<'_>, p: &Params, doc: &Document) -> Result<Item> {
    Ok(match c.name {
        "place" => {
            let e = c.req("symbol")?;
            let sym = reference(e)?;
            if !doc.symbols.contains_key(&sym) {
                return Err(fault(e, "UnknownSymbol", sym));
            }
            Item::Place {
                symbol: sym,
                unit: c
                    .take("unit")
                    .map(string)
                    .transpose()?
                    .unwrap_or_else(|| "A".into()),
                value: c.take("value").map(string).transpose()?.unwrap_or_default(),
                footprint: c
                    .take("footprint")
                    .map(string)
                    .transpose()?
                    .filter(|s| !s.is_empty()),
                ref_offset: field_offset(c.take("ref_offset"), p)?,
                value_offset: field_offset(c.take("value_offset"), p)?,
                fields: fields(c)?,
                datasheet: c
                    .take("datasheet")
                    .map(string)
                    .transpose()?
                    .filter(|s| !s.is_empty()),
                populated: c.take("populated").map(boolean).transpose()?.unwrap_or(true),
                in_bom: c.take("in_bom").map(boolean).transpose()?.unwrap_or(true),
                at: point(c.req("at")?, p)?,
                angle: rotation(c.take("angle"), p)?,
                mirror: c.take("mirror").map(boolean).transpose()?.unwrap_or(false),
            }
        }
        "gate" => Item::Gate {
            of: reference(c.req("of")?)?,
            unit: string(c.req("unit")?)?,
            at: point(c.req("at")?, p)?,
            angle: rotation(c.take("angle"), p)?,
            mirror: c.take("mirror").map(boolean).transpose()?.unwrap_or(false),
        },
        "flow" => Item::Flow {
            from: reference(c.req("from")?)?,
            to: reference(c.req("to")?)?,
        },
        "bus" => Item::Bus(map(c.req("members")?, reference)?),
        "subsheet" => Item::SheetInstance {
            of: reference(c.req("of")?)?,
            at: point(c.req("at")?, p)?,
            anchors: c
                .take("ports")
                .map(|e| map(e, |e| point(e, p)))
                .transpose()?
                .unwrap_or_default(),
        },
        "junction" => Item::Junction(point(c.req("at")?, p)?),
        "port" => Item::Port(point(c.req("at")?, p)?),
        "wire" => Item::Wire {
            from: reference(c.req("from")?)?,
            to: reference(c.req("to")?)?,
            via: c
                .take("via")
                .map(|x| list(x)?.iter().map(|v| point(v, p)).collect::<Result<_>>())
                .transpose()?
                .unwrap_or_default(),
        },
        "label" => {
            let e = c.req("value")?;
            let text = string(e)?;
            if text.is_empty() {
                return Err(fault(e, "EmptyNetLabel", "a net label needs a name"));
            }
            Item::Label {
                target: reference(c.req("on")?)?,
                text,
                global: c.take("global").map(boolean).transpose()?.unwrap_or(false),
                at: point(c.req("at")?, p)?,
            }
        }
        "no_connect" => Item::NoConnect(reference(c.req("on")?)?),
        "note" => Item::Note(point(c.req("at")?, p)?, string(c.req("value")?)?),
        "fulfill" => {
            let e = c.req("role")?;
            let role = resolve_role(e, doc)?;
            let returns = match (c.take("return_from"), c.take("return_to")) {
                (Some(a), Some(b)) => Some((reference(a)?, reference(b)?)),
                (None, None) => None,
                _ => {
                    return Err(Fault::new(
                        "IncompleteReturn",
                        c.span,
                        "provide both return endpoints",
                    ));
                }
            };
            Item::Fulfill {
                want: reference(c.req("want")?)?,
                give: reference(c.req("give")?)?,
                prefer: c
                    .take("prefer")
                    .map(|e| positive(e, p))
                    .transpose()?
                    .or(doc.roles[&role].prefer),
                role,
                returns,
            }
        }
        _ => return Err(Fault::new("UnknownBuiltin", c.span, c.name)),
    })
}

pub(super) fn header(src: &str) -> Option<&str> {
    src.lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .find(|l| !l.is_empty())
}
pub fn read(src: &str, dir: Option<&Path>, root: Option<&Path>) -> Result<Document> {
    let prog =
        syntax::parse(src).map_err(|e| Fault::new(e.kind(), e.span().unwrap_or_default(), e.text()))?;
    let Some((head, body)) = prog.split_first() else {
        return Err(Fault::new(
            "ExpectedHeader",
            Span::default(),
            "expected schematic or symbol_library",
        ));
    };
    let S::Expr(Expr { kind: E::Name(h), .. }) = &head.kind else {
        return Err(Fault::new(
            "ExpectedHeader",
            head.span,
            "expected schematic or symbol_library",
        ));
    };
    let library = match h.as_str() {
        "symbol_library" => true,
        "schematic" => false,
        _ => {
            return Err(Fault::new(
                "ExpectedHeader",
                head.span,
                "expected schematic or symbol_library",
            ));
        }
    };
    let mut doc = Document {
        library,
        ..Default::default()
    };
    let mut names = Set::new();
    let mut params = Map::new();
    for stmt in body {
        let (name, e) = match &stmt.kind {
            S::Assign { name, value } | S::Shared { name, value } => (name, value),
            _ => {
                return Err(Fault::new(
                    "ExpectedDeclaration",
                    stmt.span,
                    "expected a named declaration or shared parameter",
                ));
            }
        };
        if !names.insert(name) {
            return Err(Fault::new("DuplicateName", stmt.span, name));
        }
        if matches!(&stmt.kind, S::Shared { .. }) {
            let mut n = number(e, &params)?;
            if n.dim == Dim::Any {
                n.dim = Dim::Length;
            }
            params.insert(name.clone(), n);
            continue;
        }
        let mut c = Call::new(e)?;
        match c.name {
            "role" => {
                let k = c.req("kind")?;
                let word = string(k)?;
                let kind = RoleKind::parse(&word).ok_or_else(|| fault(k, "UnknownRoleKind", word))?;
                let mut rating = None;
                for field in ["capacitance_nf", "voltage_mv"] {
                    if let Some(arg) = c.take(field) {
                        let v = measure(arg, &params, Dim::Ratio)?;
                        rating = Some(match (field, kind) {
                            ("capacitance_nf", RoleKind::Bypass) if v > 0.0 => Rating::CapacitanceNf(v),
                            ("voltage_mv", RoleKind::Power | RoleKind::Reference) => Rating::VoltageMv(v),
                            _ => {
                                return Err(fault(
                                    arg,
                                    "InvalidRoleRating",
                                    format!("{field} is incompatible with this role or value"),
                                ));
                            }
                        });
                    }
                }
                let id = if let Some(key) = c.take("key") {
                    let text = string(key)?;
                    if text.is_empty()
                        || !text
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                    {
                        return Err(fault(key, "InvalidRoleKey", text));
                    }
                    RoleId(format!("@{text}"))
                } else {
                    RoleId(name.clone())
                };
                register_role(
                    &mut doc,
                    name.clone(),
                    id,
                    Role {
                        kind,
                        rating,
                        prefer: c.take("prefer").map(|e| positive(e, &params)).transpose()?,
                    },
                    e,
                )?;
            }
            "symbol" => {
                let s = symbol(&mut c, &params, &doc)?;
                doc.symbols.insert(name.clone(), s);
            }
            "library" if !library => {
                let e = c.req("file")?;
                let file = string(e)?;
                let library = if file == "forge:power" {
                    read(POWER_LIBRARY, None, None)?
                } else {
                    load_library(&file, dir, root, e, &mut doc.imports)?
                };
                let qualify = |id: RoleId| {
                    if id.0.starts_with('@') {
                        id
                    } else {
                        RoleId(format!("{name}.{}", id.0))
                    }
                };
                for (alias, id) in library.role_names {
                    let ty = library.roles[&id].clone();
                    register_role(&mut doc, format!("{name}.{alias}"), qualify(id), ty, e)?;
                }
                for (id, mut sym) in library.symbols {
                    if let Some(link) = &mut sym.datasheet
                        && !link.contains("://")
                        && !Path::new(link).is_absolute()
                        && let Some(parent) = Path::new(&file).parent()
                    {
                        *link = parent.join(&*link).to_string_lossy().replace('\\', "/");
                    }

                    for pin in sym.pins.values_mut() {
                        for roles in [&mut pin.wants, &mut pin.gives] {
                            *roles = std::mem::take(roles).into_iter().map(&qualify).collect();
                        }
                    }
                    doc.symbols.insert(format!("{name}.{id}"), sym);
                }
            }
            "sheet" if !library => {
                let title = c
                    .take("title")
                    .map(string)
                    .transpose()?
                    .unwrap_or_else(|| name.clone());
                doc.sheets.insert(name.clone(), title);
            }
            _ if library => {
                return Err(fault(
                    e,
                    "InvalidLibraryItem",
                    "a symbol library declares roles, shared dimensions and symbols",
                ));
            }
            _ => {
                let sheet = reference(c.req("sheet")?)?;
                let pose_span = if c.name == "place" {
                    c.args.get("at").map(|e| e.span)
                } else {
                    None
                };
                let obj = item(&mut c, &params, &doc)?;
                doc.rows.push(Row {
                    sheet,
                    id: name.clone(),
                    item: obj,
                    pose_span,
                    span: stmt.span,
                });
            }
        }
        c.end()?;
    }
    Ok(doc)
}
fn load_library(
    file: &str,
    dir: Option<&Path>,
    root: Option<&Path>,
    e: &Expr,
    imports: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<Document> {
    let bad = || {
        fault(
            e,
            "LibraryUnavailable",
            format!("cannot read symbol library {file}"),
        )
    };
    let base = dir.unwrap_or_else(|| Path::new("."));
    let candidate = forge_dirs::fs::canonicalize(base.join(file)).map_err(|_| bad())?;
    let fence: PathBuf = forge_dirs::fs::canonicalize(root.unwrap_or(base)).map_err(|_| bad())?;
    if !candidate.starts_with(&fence) || candidate.extension().and_then(|x| x.to_str()) != Some("fg") {
        return Err(bad());
    }
    let bytes = crate::deps::read_plain(&candidate, 8 * 1024 * 1024).map_err(|_| bad())?;
    crate::deps::read(&candidate, Some(&bytes));
    imports.push((base.join(file), candidate.clone()));
    let src = std::str::from_utf8(&bytes).map_err(|_| bad())?;
    if header(src) != Some("symbol_library") {
        return Err(fault(e, "ExpectedSymbolLibrary", file));
    }
    read(src, candidate.parent(), Some(&fence)).map_err(|mut f| {
        f.file = Some(file.into());
        f
    })
}

pub fn parameter_dimension(src: &str, key: &crate::error::ParamKey) -> Option<Dim> {
    let mut params = Map::new();
    for stmt in syntax::parse(src).ok()? {
        if let S::Shared { name, value } = stmt.kind {
            let mut n = number(&value, &params).ok()?;
            if n.dim == Dim::Any {
                n.dim = Dim::Length;
            }
            if crate::error::ParamKey::shared(&name) == *key {
                return Some(n.dim);
            }
            params.insert(name, n);
        }
    }
    None
}

/// Read one complete, self-contained coordinate tuple with the normal grammar and unit checks.
pub(super) fn isolated_point(text: &str) -> Option<Point> {
    let inside = text.strip_prefix('(')?.strip_suffix(')')?;
    if text.len() > 128 || inside.contains(['(', ')', '#', '\n', '\r']) {
        return None;
    }
    let tree = syntax::parse(text).ok()?;
    let [stmt] = tree.as_slice() else {
        return None;
    };
    let S::Expr(e) = &stmt.kind else {
        return None;
    };
    if e.span != Span::new(0, text.len()) {
        return None;
    }
    let E::Tuple(xs) = &e.kind else { return None };
    if !xs
        .iter()
        .all(|v| matches!(v.kind, E::Num(..)) || matches!(&v.kind, E::Neg(x) if matches!(x.kind,E::Num(..))))
    {
        return None;
    }
    point(e, &Params::new()).ok()
}
