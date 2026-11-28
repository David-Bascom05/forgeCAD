//! Evaluate datasheet strap tables against fitted schematic connectivity.
#[path = "../standard_library/erc/attributes.rs"]
mod standard;
use super::reader::map;
use super::*;
use crate::electronics::{Call, fault, list, reference, string};
use crate::syntax::{Expr, ExprKind};
use forge_agent::json::Json;

impl StrapKind {
    pub fn typed(self, text: &str) -> Option<Json> {
        let text = text.trim();
        match self {
            Self::Quantity | Self::Integer | Self::Lookup => None,
            Self::Text => Some(text.to_owned().into()),
            Self::BootMode | Self::Choice => (!text.is_empty()).then(|| text.to_owned().into()),
            Self::Address7 | Self::Current => self.typed_with_units(text, self.standard_units(), None),
        }
    }
    pub fn formats(self) -> &'static [&'static str] {
        match self {
            Self::Current => &["A", "mA", "µA"],
            Self::Address7 => &["Hex", "Decimal", "Binary"],
            _ => &[],
        }
    }
    pub fn converted(self, value: &str, format: &str) -> Option<String> {
        if matches!(self, Self::Address7 | Self::Current) {
            self.format_unit(value, self.standard_units(), None, format)
        } else {
            None
        }
    }
}

/// The electrical condition described by a symbol author for one strap pin.
#[derive(Debug, Clone, PartialEq)]
pub enum StrapCondition {
    Reference(String),
    Floating,
    Voltage(VoltageRange),
}
#[derive(Debug, Clone, PartialEq)]
pub struct StrapTable {
    pub conversions: ConversionGraph,
    pub label: Option<String>,
    pub unit: Option<String>,
    pub units: Vec<AttributeUnit>,
    pub source_type: Option<String>,
    pub column_order: Vec<String>,
    pub row_order: Vec<Map<String, String>>,
    pub kind: StrapKind,
    pub bus: Option<[String; 2]>,
    pub range: Option<[String; 2]>,
    pub modes: Vec<String>,
    pub inputs: Map<String, Map<String, StrapCondition>>,
    pub rows: Map<Map<String, String>, String>,
    pub default: Option<String>,
}

impl StrapTable {
    pub(super) fn json(&self) -> Json {
        let inputs = self
            .inputs
            .iter()
            .map(|(pin, states)| {
                (
                    pin.clone(),
                    Json::Obj(
                        states
                            .iter()
                            .map(|(state, target)| {
                                (
                                    state.clone(),
                                    match target {
                                        StrapCondition::Reference(id) => id.clone().into(),
                                        StrapCondition::Floating => Json::Null,
                                        StrapCondition::Voltage(v) => {
                                            Json::obj(vec![("min_v", v.min.into()), ("max_v", v.max.into())])
                                        }
                                    },
                                )
                            })
                            .collect(),
                    ),
                )
            })
            .collect();
        let rows = self
            .row_order
            .iter()
            .map(|pattern| {
                let value = &self.rows[pattern];
                Json::obj(vec![
                    (
                        "states",
                        Json::Obj(
                            pattern
                                .iter()
                                .map(|(p, s)| (p.clone(), s.clone().into()))
                                .collect(),
                        ),
                    ),
                    (
                        "value",
                        self.conversions
                            .typed(self.kind, value, &self.units, self.unit.as_deref())
                            .unwrap(),
                    ),
                ])
            })
            .collect();
        Json::obj(vec![
            ("kind", self.kind.word().into()),
            ("label", self.label.clone().map_or(Json::Null, Into::into)),
            ("unit", self.unit.clone().map_or(Json::Null, Into::into)),
            ("type", self.source_type.clone().map_or(Json::Null, Into::into)),
            (
                "columns",
                Json::Arr(self.column_order.iter().cloned().map(Into::into).collect()),
            ),
            ("inputs", Json::Obj(inputs)),
            (
                "bus",
                self.bus.as_ref().map_or(Json::Null, |v| {
                    Json::Arr(v.iter().cloned().map(Into::into).collect())
                }),
            ),
            (
                "range",
                self.range.as_ref().map_or(Json::Null, |v| {
                    Json::Arr(v.iter().map(|s| self.kind.typed(s).unwrap()).collect())
                }),
            ),
            (
                "modes",
                Json::Arr(self.modes.iter().cloned().map(Into::into).collect()),
            ),
            ("rows", Json::Arr(rows)),
            (
                "default",
                self.default
                    .as_ref()
                    .and_then(|v| {
                        self.conversions
                            .typed(self.kind, v, &self.units, self.unit.as_deref())
                    })
                    .unwrap_or(Json::Null),
            ),
        ])
    }
}
fn read_constraints(c: &mut Call<'_>, kind: StrapKind) -> Result<(Option<[String; 2]>, Vec<String>)> {
    let range = c
        .take("range")
        .map(|v| {
            let [a, b] = list(v)? else {
                return Err(fault(
                    v,
                    "InvalidCurrentRange",
                    "expected minimum and maximum current",
                ));
            };
            let bounds = [string(a)?, string(b)?];
            if kind != StrapKind::Current
                || !amps(&bounds[0])
                    .zip(amps(&bounds[1]))
                    .is_some_and(|(a, b)| a <= b)
            {
                return Err(fault(
                    v,
                    "InvalidCurrentRange",
                    "use ordered current limits with units",
                ));
            }
            Ok(bounds)
        })
        .transpose()?;
    let modes = c
        .take("modes")
        .map(|v| {
            let allowed = list(v)?.iter().map(string).collect::<Result<Vec<_>>>()?;
            if !matches!(kind, StrapKind::BootMode | StrapKind::Choice)
                || allowed.is_empty()
                || allowed.iter().any(|s| s.trim().is_empty())
                || allowed.iter().collect::<Set<_>>().len() != allowed.len()
            {
                return Err(fault(
                    v,
                    if kind == StrapKind::BootMode {
                        "InvalidBootModes"
                    } else {
                        "InvalidAttributeOptions"
                    },
                    "use distinct, nonempty option names on a choice or boot mode",
                ));
            }
            Ok(allowed)
        })
        .transpose()?
        .unwrap_or_default();
    Ok((range, modes))
}
pub(super) fn read_type(c: &mut Call<'_>) -> Result<AttributeType> {
    let e = c.req("kind")?;
    let kind = StrapKind::parse(&string(e)?)
        .ok_or_else(|| fault(e, "UnknownStrapKind", "unknown native value kind"))?;
    let label = string(c.req("label")?)?;
    if label.trim().is_empty() {
        return Err(fault(e, "EmptyTypeLabel", "give the type a display name"));
    }
    let (range, modes) = read_constraints(c, kind)?;
    let mut units = if let Some(e) = c.take("units") {
        attribute_units::read_units(e, kind == StrapKind::Lookup)?
    } else if matches!(kind, StrapKind::Quantity | StrapKind::Integer | StrapKind::Lookup) {
        return Err(fault(
            e,
            "MissingAttributeUnits",
            "declare the units before using a quantity type",
        ));
    } else {
        kind.standard_units().to_vec()
    };
    if kind == StrapKind::Address7 && units.iter().all(|u| u.radix.is_none()) {
        for u in &mut units {
            let native = kind
                .standard_units()
                .iter()
                .find(|r| r.name == u.name && u.factor == 1.)
                .ok_or_else(|| {
                    fault(
                        e,
                        "InvalidAttributeUnit",
                        "declare a radix for this representation",
                    )
                })?;
            u.radix = native.radix.clone();
        }
    }
    let valid = match kind {
        StrapKind::Current => units[0].name == "A" && units[0].radix.is_none(),
        StrapKind::Address7 | StrapKind::Integer => units[0].radix.is_some(),
        StrapKind::Text | StrapKind::Choice | StrapKind::BootMode => {
            units.len() == 1
                && units[0].name == "Unitless"
                && units[0].min.is_none()
                && units[0].max.is_none()
                && units[0].radix.is_none()
        }
        StrapKind::Quantity => units[0].radix.is_none(),
        StrapKind::Lookup => units
            .iter()
            .all(|u| u.factor == 1. && u.radix.is_none() && u.min.is_none() && u.max.is_none()),
    };
    if !valid {
        return Err(fault(
            e,
            "InvalidAttributeUnit",
            "use the native representation for this type, or declare a custom quantity",
        ));
    }
    let tables = c
        .take("conversions")
        .map(conversions::read)
        .transpose()?
        .unwrap_or_else(|| ConversionGraph::defaults(&units));
    let conversions = ConversionGraph::build(kind, &units, tables)
        .map_err(|msg| fault(e, "InvalidConversionGraph", msg))?;
    Ok(AttributeType {
        origin: None,
        conversions,
        units,
        kind,
        label,
        range,
        modes,
    })
}
pub(super) fn read_tables(
    expr: Option<&Expr>,
    pins: &Map<String, Pin>,
    params: &crate::electronics::Params,
    doc: &Document,
) -> Result<Map<String, StrapTable>> {
    let Some(expr) = expr else { return Ok(Map::new()) };
    map(expr, |e| {
        let mut c = Call::with_signature(e, ARGS)?;
        if c.name != "strap_table" {
            return Err(fault(e, "ExpectedStrapTable", "use strap_table"));
        }
        let k = c.req("kind")?;
        let source_type = if matches!(k.kind, ExprKind::Str(_)) {
            None
        } else {
            Some(reference(k)?)
        };
        let definition = source_type
            .as_ref()
            .map(|key| {
                doc.attribute_types
                    .get(key)
                    .ok_or_else(|| fault(k, "UnknownAttributeType", key))
            })
            .transpose()?;
        let kind = if let Some(t) = definition {
            t.kind
        } else {
            StrapKind::parse(&string(k)?).ok_or_else(|| fault(k, "UnknownStrapKind", "unknown value kind"))?
        };
        let units = definition.map_or_else(|| kind.standard_units().to_vec(), |t| t.units.clone());
        if units.is_empty() {
            return Err(fault(
                k,
                "MissingAttributeUnits",
                "reference a type that declares its units",
            ));
        }
        let conversions = definition
            .or_else(|| standard_attribute_types().values().find(|t| t.kind == kind))
            .map(|t| t.conversions.clone())
            .unwrap_or_default();
        let label = c
            .take("label")
            .map(|e| {
                let s = string(e)?;
                if s.trim().is_empty() {
                    Err(fault(e, "EmptyAttributeLabel", "give the attribute a name"))
                } else {
                    Ok(s)
                }
            })
            .transpose()?;
        let unit = c
            .take("unit")
            .map(|e| {
                let s = string(e)?;
                if units.iter().any(|u| u.name == s) {
                    Ok(s)
                } else {
                    Err(fault(e, "UnknownAttributeUnit", s))
                }
            })
            .transpose()?;
        let pin_name = |name: &str, e: &Expr| {
            pins.get_key_value(name)
                .or_else(|| pins.iter().find(|(_, p)| p.aliases.contains(name)))
                .map(|(name, _)| name.clone())
                .ok_or_else(|| fault(e, "UnknownStrapPin", name))
        };
        let bus = c
            .take("bus")
            .map(|v| {
                let [a, b] = list(v)? else {
                    return Err(fault(v, "InvalidI2cBus", "select SDA and SCL"));
                };
                let pins = [pin_name(&reference(a)?, a)?, pin_name(&reference(b)?, b)?];
                if kind != StrapKind::Address7 || pins[0] == pins[1] {
                    return Err(fault(
                        v,
                        "InvalidI2cBus",
                        "an address needs distinct SDA and SCL pins",
                    ));
                }
                Ok(pins)
            })
            .transpose()?;
        let (local_range, local_modes) = read_constraints(&mut c, kind)?;
        let range = local_range.or_else(|| definition.and_then(|t| t.range.clone()));
        let modes = if local_modes.is_empty() {
            definition.map(|t| t.modes.clone()).unwrap_or_default()
        } else {
            local_modes
        };
        let mut inputs = Map::new();
        let mut column_order = Vec::new();
        if let Some(i) = c.take("inputs") {
            let ExprKind::Map(columns) = &i.kind else {
                return Err(fault(i, "ExpectedMap", "expected named pin columns"));
            };
            for (id, entry) in columns {
                let states = map(entry, |target| {
                    if matches!(&target.kind, ExprKind::Str(s) if s == "open" || s == "floating") {
                        Ok(StrapCondition::Floating)
                    } else if let ExprKind::List(bounds) = &target.kind {
                        let [lo, hi] = bounds.as_slice() else {
                            return Err(fault(
                                target,
                                "InvalidStrapVoltage",
                                "expected [minimum, maximum] in volts",
                            ));
                        };
                        let volts = |e| crate::electronics::measure(e, params, crate::values::Dim::Ratio);
                        let range = VoltageRange {
                            min: volts(lo)?,
                            max: volts(hi)?,
                        };
                        if range.min > range.max || !range.min.is_finite() || !range.max.is_finite() {
                            return Err(fault(
                                target,
                                "InvalidStrapVoltage",
                                "use finite, ordered voltage limits",
                            ));
                        }
                        Ok(StrapCondition::Voltage(range))
                    } else {
                        pin_name(&reference(target)?, target).map(StrapCondition::Reference)
                    }
                })?;
                let pin = pin_name(id, i)?;
                column_order.push(pin.clone());
                let mut seen = Vec::new();
                for condition in states.values() {
                    if seen.iter().any(|prior| match (prior, condition) {
                        (StrapCondition::Voltage(a), StrapCondition::Voltage(b)) => {
                            a.min <= b.max && b.min <= a.max
                        }
                        (a, b) => a == b,
                    }) {
                        return Err(fault(
                            entry,
                            "InvalidStrapInput",
                            "each state's electrical condition must be distinct; voltage intervals cannot overlap",
                        ));
                    }
                    seen.push(condition.clone());
                }
                if states.is_empty()
                    || states
                        .values()
                        .any(|r| matches!(r,StrapCondition::Reference(p) if p==&pin))
                    || inputs.insert(pin, states).is_some()
                {
                    return Err(fault(
                        i,
                        "InvalidStrapInput",
                        "each pin needs a distinct reference, floating condition or voltage interval",
                    ));
                }
            }
        }
        let value = |e: &Expr| {
            let v = string(e)?;
            conversions
                .typed(kind, &v, &units, unit.as_deref())
                .ok_or_else(|| {
                    fault(
                        e,
                        "InvalidStrapValue",
                        format!("invalid {} value {v}", kind.word()),
                    )
                })?;
            if let Some(to) = &unit {
                conversions
                    .convert(kind, &v, &units, Some(to), to)
                    .ok_or_else(|| fault(e, "InvalidStrapValue", format!("{v} is outside the {to} range")))
            } else {
                Ok(v)
            }
        };
        let mut rows = Map::new();
        let mut row_order = Vec::new();
        if let Some(r) = c.take("rows") {
            for row in list(r)? {
                let [pattern, out] = list(row)? else {
                    return Err(fault(row, "InvalidStrapRow", "use [pin states, result]"));
                };
                let mut states = Map::new();
                for (pin, state) in map(pattern, reference)? {
                    let pin = pin_name(&pin, pattern)?;
                    if !inputs
                        .get(&pin)
                        .is_some_and(|allowed| allowed.contains_key(&state))
                        || states.insert(pin, state).is_some()
                    {
                        return Err(fault(
                            pattern,
                            "InvalidStrapRow",
                            "choose a declared state for each input pin",
                        ));
                    }
                }
                if states.is_empty()
                    || states.len() != inputs.len()
                    || rows.insert(states.clone(), value(out)?).is_some()
                {
                    return Err(fault(
                        row,
                        "InvalidStrapRow",
                        "each row must cover all inputs and be unique",
                    ));
                }
                row_order.push(states);
            }
        }
        let default = c.take("default").map(value).transpose()?;
        if rows.is_empty() && default.is_none() {
            return Err(fault(e, "EmptyStrapTable", "supply a row or a default"));
        }
        c.end()?;
        Ok(StrapTable {
            conversions,
            label,
            units,
            unit,
            source_type,
            column_order,
            row_order,
            kind,
            bus,
            range,
            modes,
            inputs,
            rows,
            default,
        })
    })
}

#[derive(Debug, Default)]
pub struct StrapInputState {
    pub state: Option<String>,
    pub open: bool,
    pub evidence: Set<String>,
}
#[derive(Debug)]
pub struct StrapResult {
    pub label: Option<String>,
    typed: Option<Json>,
    pub kind: StrapKind,
    pub status: &'static str,
    pub value: Option<String>,
    pub inputs: Map<String, StrapInputState>,
}

impl StrapResult {
    fn json(&self) -> Json {
        Json::obj(vec![
            ("kind", self.kind.word().into()),
            ("status", self.status.into()),
            ("display_value", self.value.clone().map_or(Json::Null, Into::into)),
            ("value", self.typed.clone().unwrap_or(Json::Null)),
            (
                "inputs",
                Json::Obj(
                    self.inputs
                        .iter()
                        .map(|(pin, s)| {
                            (
                                pin.clone(),
                                Json::obj(vec![
                                    ("state", s.state.clone().map_or(Json::Null, Into::into)),
                                    ("open", s.open.into()),
                                    (
                                        "evidence",
                                        Json::Arr(s.evidence.iter().cloned().map(Into::into).collect()),
                                    ),
                                ]),
                            )
                        })
                        .collect(),
                ),
            ),
        ])
    }
}

#[derive(Default)]
struct AttributeChecks {
    addresses: Vec<crate::electronics_library::worker::Fact>,
    issues: Vec<super::graph::Issue>,
}

struct Bridge {
    near: String,
    far: String,
    known: bool,
}

impl Circuit {
    pub(super) fn resolve_straps(
        &mut self,
        doc: &Document,
        sheets: &Map<&str, Vec<&Row>>,
        fitted: &Map<String, bool>,
    ) {
        if !doc.symbols.values().any(|s| !s.strap_tables.is_empty()) {
            return;
        }
        let mut resistors: Map<&str, Vec<Bridge>> = Map::new();
        let mut owners = Vec::new();
        for (scope, sheet) in &self.occurrences {
            for row in sheets.get(sheet.as_str()).into_iter().flatten() {
                let Item::Place {
                    symbol,
                    populated,
                    value,
                    ..
                } = &row.item
                else {
                    continue;
                };
                let owner = format!("{scope}/{}", row.id);
                if !fitted.get(&owner).unwrap_or(populated) {
                    continue;
                }
                let def = &doc.symbols[symbol];
                if !def.strap_tables.is_empty() {
                    owners.push((owner.clone(), def));
                }
                if def.part_kind.as_deref() != Some("resistor") {
                    continue;
                }
                for [a, b] in def.resistor_paths() {
                    let a = format!("{owner}.{a}");
                    let b = format!("{owner}.{b}");
                    for (near, far) in [(&a, &b), (&b, &a)] {
                        if let (Some(net), Some(other)) = (self.net_of.get(near), self.net_of.get(far))
                            && net != other
                        {
                            resistors.entry(net).or_default().push(Bridge {
                                near: near.clone(),
                                far: far.clone(),
                                known: PartValue::parse(value).is_some_and(|v| v.unit == "Ω"),
                            });
                        }
                    }
                }
            }
        }
        let mut checks = AttributeChecks::default();
        for (owner, symbol) in owners {
            let mut attrs = Map::new();
            for (name, table) in &symbol.strap_tables {
                let mut inputs = Map::new();
                let mut pattern = Map::new();
                for (pin, states) in &table.inputs {
                    let terminal = format!("{owner}.{pin}");
                    let input = self.strap_input(doc, &owner, &terminal, states, &resistors);
                    if let Some(state) = &input.state {
                        pattern.insert(pin.clone(), state.clone());
                    }
                    inputs.insert(terminal, input);
                }
                let (status, value) = if inputs.values().any(|s| s.state.is_none() && !s.open) {
                    ("unresolved", None)
                } else if let Some(v) = table.rows.get(&pattern) {
                    ("matched", Some(v.clone()))
                } else if let Some(v) = &table.default {
                    ("default", Some(v.clone()))
                } else {
                    ("unmapped", None)
                };
                let result = StrapResult {
                    label: table.label.clone(),
                    typed: value.as_ref().and_then(|v| {
                        table
                            .conversions
                            .typed(table.kind, v, &table.units, table.unit.as_deref())
                    }),
                    kind: table.kind,
                    status,
                    value,
                    inputs,
                };
                self.check_attribute(&owner, symbol, name, table, &result, &mut checks);
                attrs.insert(name.clone(), result);
            }
            self.straps.insert(owner, attrs);
        }
        self.issues.extend(checks.issues);
        self.addresses = checks.addresses;
    }
    fn strap_input(
        &self,
        doc: &Document,
        owner: &str,
        terminal: &str,
        states: &Map<String, StrapCondition>,
        resistors: &Map<&str, Vec<Bridge>>,
    ) -> StrapInputState {
        let mut input = StrapInputState::default();
        let Some(net) = self.net_of.get(terminal) else {
            return input;
        };

        input.open = self.strap_is_open(net, terminal);
        let bridges = resistors.get(net.as_str()).map(Vec::as_slice).unwrap_or_default();
        let mut matches = Set::new();
        let mut reference_nets = Set::new();
        let observed = states
            .values()
            .any(|s| matches!(s, StrapCondition::Voltage(_)))
            .then(|| self.strap_voltage(doc, net, terminal, bridges))
            .flatten();
        for (state, target) in states {
            if let StrapCondition::Reference(target) = target {
                let id = format!("{owner}.{target}");
                if let Some(reference) = self.net_of.get(&id) {
                    reference_nets.insert(reference);
                    if net == reference {
                        matches.insert(state);
                        input.evidence.insert(id.clone());
                    }
                    for b in bridges
                        .iter()
                        .filter(|b| b.known && self.net_of[&b.far] == *reference)
                    {
                        matches.insert(state);
                        input.evidence.extend([b.near.clone(), b.far.clone(), id.clone()]);
                    }
                }
            } else if matches!(target, StrapCondition::Floating) && input.open {
                matches.insert(state);
            } else if let (StrapCondition::Voltage(want), Some((actual, evidence))) = (target, &observed)
                && want.accepts(*actual)
            {
                matches.insert(state);
                input.evidence.extend(evidence.iter().cloned());
            }
        }
        // A third bias branch or a dynamic driver can change a resistively set strap.
        let direct = reference_nets.contains(net) || observed.is_some();
        let uncertain = !direct
            && (bridges
                .iter()
                .any(|b| !b.known || !reference_nets.contains(&self.net_of[&b.far]))
                || self.nets[net].iter().any(|id| {
                    id != terminal
                        && self.terminals.get(id).is_some_and(|p| p.populated)
                        && !bridges.iter().any(|bridge| bridge.near == *id)
                }));
        if !uncertain && matches.len() == 1 {
            input.state = matches.first().map(|s| (*s).clone());
        }

        input
    }
    // A voltage window needs a rated source. Net names and input operating limits are insufficient.
    // Follow a single fitted, valued pull resistor only when the strap itself is unloaded.
    fn strap_voltage(
        &self,
        doc: &Document,
        net: &str,
        terminal: &str,
        bridges: &[Bridge],
    ) -> Option<(VoltageRange, Vec<String>)> {
        let mut evidence = Vec::new();
        let source_net = match bridges {
            [] => net,
            [b] if b.known
                && self.nets[net]
                    .iter()
                    .all(|id| id == terminal || id == &b.near || self.inactive_strap_member(id)) =>
            {
                evidence.extend([b.near.clone(), b.far.clone()]);
                &self.net_of[&b.far]
            }
            _ => return None,
        };
        let mut voltage = None;
        for id in &self.nets[source_net] {
            if id == terminal {
                continue;
            }
            let Some(pin) = self.terminals.get(id).filter(|p| p.populated) else {
                continue;
            };
            let mut rated = false;
            for role in &pin.gives {
                if let Some(v) = doc.roles[role].provided_voltage() {
                    if voltage.is_some_and(|prior| prior != v) {
                        return None;
                    }
                    voltage = Some(v);
                    rated = true;
                }
            }
            if rated {
                evidence.push(id.clone());
            } else if matches!(
                pin.electrical,
                Electrical::Source | Electrical::Output | Electrical::OpenDrain | Electrical::Bidirectional
            ) {
                return None;
            }
        }
        voltage.map(|v| (v, evidence))
    }
    pub(super) fn strap_json(&self, owner: &str) -> Json {
        Json::Obj(
            self.straps
                .get(owner)
                .into_iter()
                .flatten()
                .map(|(name, r)| (name.clone(), r.json()))
                .collect(),
        )
    }
}

impl Symbol {
    pub(super) fn resistor_paths(&self) -> Vec<[String; 2]> {
        if self.paths.is_empty() && self.pins.len() == 2 {
            let mut pins = self.pins.keys();
            vec![[pins.next().unwrap().clone(), pins.next().unwrap().clone()]]
        } else {
            self.paths.values().cloned().collect()
        }
    }
}

fn amps(v: &str) -> Option<f64> {
    StrapKind::Current.typed(v)?.get("value")?.num()
}

fn within_current(table: &StrapTable, v: &str, lo: &str, hi: &str) -> bool {
    let quantity = |s: &str| {
        table
            .kind
            .typed_with_units(s, &table.units, table.unit.as_deref())
            .and_then(|j| j.get("value")?.num())
            .map(|magnitude| PartValue { magnitude, unit: "A" })
    };
    match (quantity(lo), quantity(v), quantity(hi)) {
        (Some(low), Some(value), Some(high)) => low.at_most(value) && value.at_most(high),
        _ => false,
    }
}
