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
fn reference_name(e: &Expr) -> Result<String> {
    if let E::Name(name) = &e.kind {
        Ok(name.clone())
    } else {
        Err(fault(e, "ExpectedReference", "choose a component on this sheet"))
    }
}
fn class_name(e: &Expr) -> Result<String> {
    let s = string(e)?;
    if s.is_empty()
        || !s
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
    {
        return Err(fault(
            e,
            "InvalidPartKind",
            "use a part type such as mosfet or relay",
        ));
    }
    Ok(PART_KINDS
        .iter()
        .find(|name| name.eq_ignore_ascii_case(&s))
        .map_or(s.clone(), |name| name.to_string()))
}
fn nominal_unit(kind: &str) -> Option<&'static str> {
    match kind {
        "resistor" => Some("Ω"),
        "capacitor" => Some("F"),
        "inductor" => Some("H"),
        _ => None,
    }
}
fn value_bounds(c: &mut Call<'_>) -> Result<Option<[PartValue; 2]>> {
    let quantity = |e: &Expr| {
        PartValue::parse(&string(e)?).ok_or_else(|| {
            fault(
                e,
                "InvalidPartValue",
                "expected an electrical value such as 4.7 kΩ, 100 nF, or 1 uH",
            )
        })
    };
    let values = match (c.take("value"), c.take("value_range")) {
        (None, None) => None,
        (Some(v), None) => Some([quantity(v)?; 2]),
        (None, Some(e)) => {
            let [lo, hi] = list(e)? else {
                return Err(fault(e, "InvalidPartRange", "expected two limits"));
            };
            let (lo, hi) = (quantity(lo)?, quantity(hi)?);
            if lo.unit != hi.unit || lo.magnitude > hi.magnitude {
                return Err(fault(
                    e,
                    "InvalidPartRange",
                    "use ordered limits of the same dimension",
                ));
            }
            Some([lo, hi])
        }
        (Some(e), Some(_)) => {
            return Err(fault(
                e,
                "InvalidPartRange",
                "choose an exact value or an interval",
            ));
        }
    };
    Ok(values)
}
fn connection_want(c: &mut Call<'_>) -> Result<Option<ConnectionWant>> {
    let Some(to) = c.take("connect_to") else {
        return Ok(None);
    };
    let target = reference(to)?;
    if target.matches('.').count() > 1 {
        return Err(fault(
            to,
            "ExpectedPin",
            "use a sibling pin or component.pin on this sheet",
        ));
    }
    let through = c
        .take("through")
        .map(|e| match &e.kind {
            E::Str(_) => class_name(e).map(PartType::Kind),
            _ => reference(e).map(PartType::Symbol),
        })
        .transpose()?;
    let values = value_bounds(c)?;
    let path = c.take("part_path").map(class_name).transpose()?;
    if (values.is_some() || path.is_some()) && through.is_none() {
        return Err(fault(
            to,
            "MissingThroughPart",
            "a path or component value requires a through part",
        ));
    }
    if let (Some(PartType::Kind(kind)), Some([lo, _])) = (&through, values)
        && let Some(unit) = nominal_unit(kind)
        && lo.unit != unit
    {
        return Err(fault(
            to,
            "WrongPartValueDimension",
            format!("a {kind} value requires {unit}"),
        ));
    }
    Ok(Some(ConnectionWant {
        path,
        to: target,
        through,
        values,
    }))
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
fn roles(e: Option<&Expr>, doc: &Document, gives: bool) -> Result<Set<RoleId>> {
    let mut result = Set::new();
    if let Some(e) = e {
        for x in list(e)? {
            let id = resolve_role(x, doc)?;
            if gives && doc.roles[&id].connection.is_some() {
                return Err(fault(
                    x,
                    "ExpectedConnectionWant",
                    "a target-pin rule belongs in the Wants list",
                ));
            }
            if !result.insert(id) {
                return Err(fault(x, "DuplicateRole", "role listed twice"));
            }
        }
    }
    Ok(result)
}
fn graphic_shape(c: &mut Call<'_>, p: &Params, span: Span) -> Result<Shape> {
    Ok(match c.name {
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
        _ => return Err(Fault::new("UnknownGraphic", span, c.name)),
    })
}
fn symbol(call: &mut Call<'_>, p: &Params, doc: &Document) -> Result<Symbol> {
    let e = call.req("pins")?;
    let E::Map(entries) = &e.kind else {
        return Err(fault(e, "ExpectedPinMap", "expected {pin_name: pin(...)}"));
    };
    let mut pins = Map::new();
    let mut numbers = Set::new();
    let mut pin_names = Set::new();
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
        if !pin_names.insert(name.clone()) {
            return Err(fault(expr, "DuplicatePin", name));
        }
        let mut aliases = Set::new();
        if let Some(previous) = c.take("aliases") {
            for alias in list(previous)? {
                let E::Name(id) = &alias.kind else {
                    return Err(fault(alias, "InvalidPinAlias", "use a pin identifier"));
                };
                if !pin_names.insert(id.clone()) {
                    return Err(fault(alias, "DuplicatePin", id));
                }
                aliases.insert(id.clone());
            }
        }
        let functions = c
            .take("functions")
            .map(|e| {
                map(e, |fun| {
                    let mut alt = Call::new(fun)?;
                    if alt.name != "function" {
                        return Err(fault(fun, "ExpectedFunction", "expected function(kind: ...)"));
                    }
                    let kind = alt.req("kind")?;
                    let electrical = Electrical::parse(&string(kind)?).ok_or_else(|| {
                        fault(kind, "UnknownElectricalKind", "unknown function electrical kind")
                    })?;
                    let mode = PinFunction {
                        electrical,
                        wants: roles(alt.take("wants"), doc, false)?,
                        gives: roles(alt.take("gives"), doc, true)?,
                    };
                    alt.end()?;
                    Ok(mode)
                })
            })
            .transpose()?
            .unwrap_or_default();
        let pads = if let Some(e) = c.take("pads") {
            let mut pads = Vec::new();
            for entry in list(e)? {
                let pad = string(entry)?;
                if pad.trim().is_empty() || pads.contains(&pad) {
                    return Err(fault(
                        entry,
                        "InvalidPad",
                        "pad numbers must be nonempty and unique",
                    ));
                }
                pads.push(pad);
            }
            if pads.is_empty() {
                return Err(fault(e, "InvalidPad", "a physical pin needs a pad"));
            }
            pads
        } else {
            vec![n.clone()]
        };
        let pin = Pin {
            functions,
            pads,
            aliases,
            description: c.take("description").map(string).transpose()?.unwrap_or_default(),
            number: n,
            unit: c
                .take("unit")
                .map(string)
                .transpose()?
                .unwrap_or_else(|| "A".into()),
            electrical,
            at: point(c.req("at")?, p)?,
            angle: c
                .take("angle")
                .map(|e| measure(e, p, Dim::Angle))
                .transpose()?
                .unwrap_or(0.)
                .rem_euclid(360.),
            wants: roles(c.take("wants"), doc, false)?,
            gives: roles(c.take("gives"), doc, true)?,
        };
        c.end()?;
        pins.insert(name.clone(), pin);
    }
    let mut mapped = Set::new();
    for pad in pins.values().flat_map(|pin| &pin.pads) {
        if !mapped.insert(pad) {
            return Err(fault(e, "DuplicatePad", pad));
        }
    }
    let mut graphics = Vec::new();
    if let Some(g) = call.take("graphics") {
        for expr in list(g)? {
            let mut c = Call::new(expr)?;
            let unit = c.take("unit").map(string).transpose()?;
            let shape = graphic_shape(&mut c, p, expr.span)?;
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
    let ref_prefix = if let Some(e) = call.take("ref_prefix") {
        let prefix = string(e)?;
        if prefix.is_empty() || !prefix.bytes().all(|b| b.is_ascii_alphabetic()) {
            return Err(fault(
                e,
                "InvalidReferencePrefix",
                "use one or more letters, such as R or TP",
            ));
        }
        prefix
    } else if power_net.is_some() {
        "PWR".into()
    } else {
        "U".into()
    };
    // Each peripheral maps a signal to the physical pins that can carry it.
    // Resolve historical names here too so a pin rename preserves membership.
    let peripherals = call
        .take("peripherals")
        .map(|groups| {
            map(groups, |signals| {
                map(signals, |members| {
                    let mut names = Set::new();
                    for e in list(members)? {
                        let id = reference(e)?;
                        let name = pins
                            .get_key_value(&id)
                            .or_else(|| pins.iter().find(|(_, p)| p.aliases.contains(&id)))
                            .ok_or_else(|| fault(e, "UnknownPeripheralPin", &id))?
                            .0;
                        if !names.insert(name.clone()) {
                            return Err(fault(e, "DuplicatePeripheralPin", &id));
                        }
                    }
                    if names.is_empty() {
                        return Err(fault(
                            members,
                            "EmptyPeripheralSignal",
                            "expected at least one pin",
                        ));
                    }
                    Ok(names)
                })
            })
        })
        .transpose()?
        .unwrap_or_default();
    let part_kind = call
        .take("part_kind")
        .map(class_name)
        .transpose()?
        .filter(|k| k != "generic");
    let paths = call
        .take("paths")
        .map(|e| {
            map(e, |pair| {
                let [a, b] = list(pair)? else {
                    return Err(fault(
                        pair,
                        "InvalidPartPath",
                        "a path takes an entry pin and an exit pin",
                    ));
                };
                let resolve = |e: &Expr| {
                    let name = reference(e)?;
                    pins.get_key_value(&name)
                        .or_else(|| pins.iter().find(|(_, p)| p.aliases.contains(&name)))
                        .map(|(name, _)| name.clone())
                        .ok_or_else(|| fault(e, "UnknownPathPin", name))
                };
                let pair_names = [resolve(a)?, resolve(b)?];
                if pair_names[0] == pair_names[1] {
                    return Err(fault(pair, "InvalidPartPath", "select two distinct pins"));
                }
                Ok(pair_names)
            })
        })
        .transpose()?
        .unwrap_or_default();
    Ok(Symbol {
        paths,
        part_kind,
        peripherals,
        ref_prefix,
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
fn placement_roles(
    input: Option<&Expr>,
    symbol: &Symbol,
    doc: &Document,
    gives: bool,
) -> Result<Map<String, Set<RoleId>>> {
    let Some(input) = input else {
        return Ok(Map::new());
    };
    let bindings = map(input, |e| roles(Some(e), doc, gives))?;
    let mut result = Map::new();
    for (key, set) in bindings {
        let pin = symbol
            .pins
            .get_key_value(&key)
            .or_else(|| symbol.pins.iter().find(|(_, p)| p.aliases.contains(&key)))
            .ok_or_else(|| fault(input, "UnknownPin", &key))?
            .0;
        if result.insert(pin.clone(), set).is_some() {
            return Err(fault(input, "DuplicatePin", &key));
        }
    }
    Ok(result)
}
fn item(c: &mut Call<'_>, p: &Params, doc: &Document) -> Result<Item> {
    Ok(match c.name {
        "place" => {
            let e = c.req("symbol")?;
            let sym = reference(e)?;
            let def = doc
                .symbols
                .get(&sym)
                .ok_or_else(|| fault(e, "UnknownSymbol", &sym))?;
            let mut functions = Map::new();
            if let Some(e) = c.take("functions") {
                for (name, function) in map(e, reference)? {
                    let Some((pin, def)) = def
                        .pins
                        .get_key_value(&name)
                        .or_else(|| def.pins.iter().find(|(_, p)| p.aliases.contains(&name)))
                    else {
                        return Err(fault(e, "UnknownPin", name));
                    };
                    if !def.functions.contains_key(&function) {
                        return Err(fault(e, "UnknownPinFunction", format!("{pin}.{function}")));
                    }
                    if functions.insert(pin.clone(), function).is_some() {
                        return Err(fault(e, "DuplicatePin", pin));
                    }
                }
            }
            let reference = c.take("reference").map(string).transpose()?;
            if reference
                .as_ref()
                .is_some_and(|r| r.is_empty() || !r.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'))
            {
                return Err(fault(e, "InvalidReference", "use a reference such as U2"));
            }
            Item::Place {
                bindings: c
                    .take("bindings")
                    .map(|e| map(e, reference_name))
                    .transpose()?
                    .unwrap_or_default(),
                functions,
                reference,
                wants: placement_roles(c.take("wants"), def, doc, false)?,
                gives: placement_roles(c.take("gives"), def, doc, true)?,
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
        "diff_pair" => Item::DiffPair {
            positive: reference(c.req("positive")?)?,
            negative: reference(c.req("negative")?)?,
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
        "note" => Item::Note {
            at: point(c.req("at")?, p)?,
            text: string(c.req("value")?)?,
            size: c
                .take("size")
                .map(|e| positive(e, p))
                .transpose()?
                .unwrap_or(1.55),
            bold: c.take("bold").map(boolean).transpose()?.unwrap_or(false),
        },
        "line" | "rect" | "circle" | "arc" => Item::Drawing {
            shape: graphic_shape(c, p, Span::default())?,
            width: c
                .take("width")
                .map(|e| positive(e, p))
                .transpose()?
                .unwrap_or(0.25),
        },
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
    read_nested(src, dir, root, &mut ImportState::default())
}

#[derive(Default)]
struct ImportState {
    active: Vec<PathBuf>,
    bytes: usize,
    count: usize,
}

fn read_nested(
    src: &str,
    dir: Option<&Path>,
    root: Option<&Path>,
    state: &mut ImportState,
) -> Result<Document> {
    state.bytes += src.len();
    state.count += 1;
    if state.active.len() > 64 || state.bytes > 64 * 1024 * 1024 || state.count > 8192 {
        return Err(Fault::new(
            "LibraryLimit",
            Span::default(),
            "library expansion exceeds the project limit",
        ));
    }
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
                            (
                                "voltage_mv",
                                RoleKind::Power | RoleKind::Reference | RoleKind::PullUp | RoleKind::PullDown,
                            ) => Rating::VoltageMv(v),
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
                if let Some(arg) = c.take("voltage_range_v") {
                    if !(kind == RoleKind::Power || kind.is_pull()) || rating.is_some() {
                        return Err(fault(
                            arg,
                            "InvalidRoleRating",
                            "use a single voltage rating on a power role",
                        ));
                    }
                    let [min, max] = list(arg)? else {
                        return Err(fault(
                            arg,
                            "InvalidVoltageRange",
                            "expected [minimum, maximum] in volts",
                        ));
                    };
                    let range = VoltageRange {
                        min: measure(min, &params, Dim::Ratio)?,
                        max: measure(max, &params, Dim::Ratio)?,
                    };
                    if range.min > range.max {
                        return Err(fault(
                            arg,
                            "InvalidVoltageRange",
                            "minimum voltage exceeds maximum",
                        ));
                    }
                    rating = Some(Rating::VoltageRange(range));
                }
                let resistance = if kind.is_pull() {
                    if rating.is_none() {
                        return Err(fault(
                            e,
                            "InvalidRoleRating",
                            "declare the pull rail voltage or voltage range",
                        ));
                    }
                    let bounds = value_bounds(&mut c)?;
                    if bounds.is_some_and(|[lo, _]| lo.unit != "Ω" || lo.magnitude <= 0.) {
                        return Err(fault(
                            e,
                            "WrongPartValueDimension",
                            "pull resistance must be positive ohms",
                        ));
                    }
                    bounds
                } else {
                    None
                };
                let connection = connection_want(&mut c)?;
                if connection.is_some() && rating.is_some() {
                    return Err(fault(
                        e,
                        "InvalidRoleRating",
                        "use separate wants for pin connectivity and electrical ratings",
                    ));
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
                        resistance,
                        connection,
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
            "library" => {
                let e = c.req("file")?;
                let file = string(e)?;
                let bundled = match file.as_str() {
                    "forge:power" => Some(POWER_LIBRARY),
                    "forge:frames" => Some(include_str!("frames.fg")),
                    _ => None,
                };
                let library = if let Some(text) = bundled {
                    read_nested(text, None, None, state)?
                } else {
                    load_library(&file, dir, root, e, &mut doc.imports, state)?
                };
                let owner = if bundled.is_some() {
                    None
                } else {
                    doc.imports.last().map(|(_, p)| p.clone())
                };
                let asset_dir = owner.as_ref().and_then(|path| {
                    let dir = forge_dirs::fs::canonicalize(dir.unwrap_or_else(|| Path::new("."))).ok()?;
                    let parent = path.parent()?;
                    for (up, ancestor) in dir.ancestors().enumerate() {
                        if let Ok(rest) = parent.strip_prefix(ancestor) {
                            let mut relative = PathBuf::new();
                            for _ in 0..up {
                                relative.push("..");
                            }
                            return Some(relative.join(rest));
                        }
                    }
                    None
                });
                doc.libraries.insert(name.clone(), owner);
                doc.libraries.extend(
                    library
                        .libraries
                        .into_iter()
                        .map(|(child, path)| (format!("{name}.{child}"), path)),
                );
                doc.imports.extend(library.imports);
                let qualify = |id: RoleId| {
                    if id.0.starts_with('@') {
                        id
                    } else {
                        RoleId(format!("{name}.{}", id.0))
                    }
                };
                for (alias, id) in library.role_names {
                    let mut ty = library.roles[&id].clone();
                    if let Some(PartType::Symbol(through)) =
                        ty.connection.as_mut().and_then(|w| w.through.as_mut())
                    {
                        *through = format!("{name}.{through}");
                    }
                    register_role(&mut doc, format!("{name}.{alias}"), qualify(id), ty, e)?;
                }
                for (id, mut sym) in library.symbols {
                    if let Some(link) = &mut sym.datasheet
                        && !link.contains("://")
                        && !Path::new(link).is_absolute()
                        && let Some(parent) = &asset_dir
                    {
                        *link = parent.join(&*link).to_string_lossy().replace('\\', "/");
                    }

                    for pin in sym.pins.values_mut() {
                        for roles in [&mut pin.wants, &mut pin.gives].into_iter().chain(
                            pin.functions
                                .values_mut()
                                .flat_map(|f| [&mut f.wants, &mut f.gives]),
                        ) {
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
                if let Some(e) = c.take("template") {
                    let id = reference(e)?;
                    if !doc.symbols.get(&id).is_some_and(|s| s.pins.is_empty()) {
                        return Err(fault(
                            e,
                            "InvalidSheetTemplate",
                            "choose a graphics-only library symbol",
                        ));
                    }
                    doc.templates.insert(name.clone(), id);
                }
                doc.sheets.insert(name.clone(), title);
            }
            "erc_rule" | "waiver" if !library => {
                let rule = string(c.req("rule")?)?;
                if !ERC_RULES.contains(&rule.as_str()) {
                    return Err(fault(e, "UnknownErcRule", rule));
                }
                if c.name == "waiver" {
                    let mut objects = list(c.req("objects")?)?
                        .iter()
                        .map(string)
                        .collect::<Result<Vec<_>>>()?;
                    objects.sort();
                    objects.dedup();
                    let reason = string(c.req("reason")?)?;
                    if reason.trim().is_empty() || objects.is_empty() {
                        return Err(fault(
                            e,
                            "InvalidWaiver",
                            "specify the affected objects and a reason",
                        ));
                    }
                    if doc.waivers.insert((rule, objects), reason).is_some() {
                        return Err(fault(e, "DuplicateWaiver", name));
                    }
                } else {
                    let severity = string(c.req("severity")?)?;
                    if !["error", "warning", "info"].contains(&severity.as_str()) {
                        return Err(fault(e, "InvalidSeverity", severity));
                    }
                    if doc.severities.insert(rule, severity).is_some() {
                        return Err(fault(e, "DuplicateErcRule", name));
                    }
                }
            }
            "annotation" if !library => {
                let path = string(c.req("occurrence")?)?;
                let reference = string(c.req("reference")?)?;
                if reference.trim().is_empty()
                    || !reference.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
                {
                    return Err(fault(e, "InvalidReference", "use a reference such as U2"));
                }
                let locked = c.take("locked").map(boolean).transpose()?.unwrap_or(false);
                if doc
                    .annotations
                    .insert(
                        path.clone(),
                        Annotation {
                            id: name.clone(),
                            reference,
                            locked,
                        },
                    )
                    .is_some()
                {
                    return Err(fault(e, "DuplicateAnnotation", path));
                }
            }
            "variant" if !library => {
                let mut overrides = Map::new();
                for (arg, fitted) in [("omit", false), ("fit", true)] {
                    if let Some(entries) = c.take(arg) {
                        for entry in list(entries)? {
                            let path = string(entry)?;
                            if overrides.insert(path.clone(), fitted).is_some() {
                                return Err(fault(entry, "DuplicateVariantPart", path));
                            }
                        }
                    }
                }
                doc.variants.insert(name.clone(), overrides);
            }
            _ if library => {
                return Err(fault(
                    e,
                    "InvalidLibraryItem",
                    "a symbol library declares libraries, roles, shared dimensions and symbols",
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
    for (name, id) in &doc.role_names {
        if let Some(PartType::Symbol(through)) =
            doc.roles[id].connection.as_ref().and_then(|w| w.through.as_ref())
        {
            let span = body
                .iter()
                .find(|s| matches!(&s.kind, S::Assign { name: n, .. } if n == name))
                .map(|s| s.span)
                .unwrap_or_default();
            let symbol = doc
                .symbols
                .get(through)
                .ok_or_else(|| Fault::new("UnknownSymbol", span, through))?;
            if let (Some(kind), Some([lo, _])) = (
                symbol.part_kind.as_deref(),
                doc.roles[id].connection.as_ref().and_then(|w| w.values),
            ) && let Some(unit) = nominal_unit(kind)
                && lo.unit != unit
            {
                return Err(Fault::new(
                    "WrongPartValueDimension",
                    span,
                    format!("{through} requires {unit}"),
                ));
            }
            let wanted = doc.roles[id].connection.as_ref().unwrap();
            if let Some(path) = &wanted.path {
                if !symbol.paths.contains_key(path) {
                    return Err(Fault::new(
                        "UnknownPartPath",
                        span,
                        format!("{through} has no path {path}"),
                    ));
                }
            } else if !(symbol.paths.len() == 1 || symbol.paths.is_empty() && symbol.pins.len() == 2) {
                return Err(Fault::new(
                    "ExpectedPartPath",
                    span,
                    format!("{through} needs an explicit terminal path"),
                ));
            }
        }
    }
    resolve_pin_aliases(&mut doc);
    Ok(doc)
}

// Resolve historical pin names before graph construction so every consumer sees the
// same canonical terminal. Project files can keep their original spelling on disk.
fn resolve_pin_aliases(doc: &mut Document) {
    let mut symbols = Map::<&str, Vec<(&str, &str)>>::new();
    for (id, symbol) in &doc.symbols {
        for (name, pin) in &symbol.pins {
            for alias in &pin.aliases {
                symbols.entry(id).or_default().push((alias, name));
            }
        }
    }
    if symbols.is_empty() {
        return;
    }
    let mut aliases = Map::<String, Map<String, String>>::new();
    for row in &doc.rows {
        if let Item::Place { symbol, .. } = &row.item
            && let Some(pins) = symbols.get(symbol.as_str())
        {
            for (alias, name) in pins {
                aliases
                    .entry(row.sheet.clone())
                    .or_default()
                    .insert(format!("{}.{alias}", row.id), format!("{}.{name}", row.id));
            }
        }
    }
    if aliases.is_empty() {
        return;
    }
    for row in &mut doc.rows {
        let Some(sheet) = aliases.get(&row.sheet) else {
            continue;
        };
        let resolve = |p: &mut String| {
            if let Some(name) = sheet.get(p) {
                p.clone_from(name);
            }
        };
        match &mut row.item {
            Item::Wire { from, to, .. } | Item::Flow { from, to } => {
                resolve(from);
                resolve(to);
            }
            Item::NoConnect(p) | Item::Label { target: p, .. } => resolve(p),
            Item::DiffPair { positive, negative } => {
                resolve(positive);
                resolve(negative);
            }
            Item::Bus(members) => members.values_mut().for_each(resolve),
            Item::Fulfill {
                want, give, returns, ..
            } => {
                resolve(want);
                resolve(give);
                if let Some((a, b)) = returns {
                    resolve(a);
                    resolve(b);
                }
            }
            _ => {}
        }
    }
}
fn load_library(
    file: &str,
    dir: Option<&Path>,
    root: Option<&Path>,
    e: &Expr,
    imports: &mut Vec<(PathBuf, PathBuf)>,
    state: &mut ImportState,
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
    if state.active.contains(&candidate) {
        return Err(fault(
            e,
            "LibraryCycle",
            format!("cyclic library import of {file}"),
        ));
    }
    let bytes = crate::deps::read_plain(&candidate, 8 * 1024 * 1024).map_err(|_| bad())?;
    crate::deps::read(&candidate, Some(&bytes));
    imports.push((base.join(file), candidate.clone()));
    let src = std::str::from_utf8(&bytes).map_err(|_| bad())?;
    if header(src) != Some("symbol_library") {
        return Err(fault(e, "ExpectedSymbolLibrary", file));
    }
    state.active.push(candidate.clone());
    let result = read_nested(src, candidate.parent(), Some(&fence), state);
    state.active.pop();
    result.map_err(|mut f| {
        f.file.get_or_insert_with(|| {
            candidate
                .strip_prefix(&fence)
                .unwrap_or(&candidate)
                .to_string_lossy()
                .into_owned()
        });
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
