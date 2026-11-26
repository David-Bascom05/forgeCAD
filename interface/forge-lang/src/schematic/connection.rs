//! Verify pin-to-pin wiring across a single declared terminal pair of a fitted component.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartValue {
    pub magnitude: f64,
    pub unit: &'static str,
}

impl PartValue {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        // IEC resistor markings such as 4R7 and 2K2 also occur in BOM value fields.
        if let Some((i, mark)) = text
            .char_indices()
            .find(|(_, c)| matches!(c, 'R' | 'k' | 'K' | 'M'))
        {
            let (whole, fraction) = (&text[..i], &text[i + 1..]);
            if !whole.is_empty()
                && whole.bytes().all(|c| c.is_ascii_digit() || c == b'.')
                && fraction.bytes().all(|c| c.is_ascii_digit())
            {
                let number = if fraction.is_empty() {
                    whole.to_owned()
                } else {
                    format!("{whole}.{fraction}")
                };
                let magnitude = number.parse::<f64>().ok()?
                    * match mark {
                        'R' => 1.,
                        'M' => 1e6,
                        _ => 1e3,
                    };
                return magnitude.is_finite().then_some(Self {
                    magnitude, unit: "Ω"
                });
            }
        }
        let end = text
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(text.len());
        let value: f64 = text[..end].parse().ok()?;
        let suffix = text[end..].trim();
        let (prefix, unit) = [("ohms", "Ω"), ("ohm", "Ω"), ("Ω", "Ω"), ("F", "F"), ("H", "H")]
            .into_iter()
            .find_map(|(suffix_unit, unit)| Some((suffix.strip_suffix(suffix_unit)?, unit)))?;
        let factor = match prefix {
            "p" => 1e-12,
            "n" => 1e-9,
            "u" | "µ" | "μ" => 1e-6,
            "m" => 1e-3,
            "" => 1.,
            "k" => 1e3,
            "M" => 1e6,
            "G" => 1e9,
            _ => return None,
        };
        let magnitude = value * factor;
        magnitude.is_finite().then_some(Self { magnitude, unit })
    }
    pub(super) fn at_most(self, other: Self) -> bool {
        self.unit == other.unit
            && (self.magnitude <= other.magnitude
                || self.magnitude - other.magnitude <= other.magnitude * (4. * f64::EPSILON))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PartType {
    Kind(String),
    Symbol(String),
}
impl PartType {
    pub fn name(&self) -> &str {
        match self {
            Self::Kind(k) => k,
            Self::Symbol(s) => s,
        }
    }
    fn matches(&self, symbol: &str, doc: &Document) -> bool {
        match self {
            Self::Kind(k) => doc.symbols[symbol].part_kind.as_ref().is_some_and(|actual| {
                actual == k || k == "fet" && matches!(actual.as_str(), "mosfet" | "jfet")
            }),
            Self::Symbol(s) => s == symbol,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionWant {
    pub to: String,
    pub through: Option<PartType>,
    pub values: Option<[PartValue; 2]>,
    pub path: Option<String>,
}

#[derive(Debug)]
pub struct ConnectionCheck {
    /// The active pin role witnessing an internal pull, when the check uses one.
    pub source_role: Option<RoleId>,
    pub target: String,
    pub satisfied: bool,
    pub unknown: bool,
    pub detail: String,
    /// Ordered from the requesting net toward the destination net.
    pub bridge_pins: Vec<String>,
}

struct Bridge {
    path: Option<String>,
    default_path: bool,
    symbol: String,
    owner: String,
    pins: [String; 2],
    value: String,
}

impl Circuit {
    pub(super) fn check_connections(&mut self, doc: &Document, by_sheet: &Map<&str, Vec<&Row>>) {
        let wants: Vec<_> = self
            .terminals
            .iter()
            .filter(|(_, p)| p.populated)
            .flat_map(|(pin, term)| {
                term.wants
                    .iter()
                    .filter_map(move |role| Some((pin, role, doc.roles[role].connection.as_ref()?)))
            })
            .collect();
        if wants.is_empty() {
            return;
        }

        // Index actual component occurrences by symbol and unordered pair of distinct nets.
        let mut bridges: Map<(&str, &str), Vec<Bridge>> = Map::new();
        let kinds: Set<_> = wants.iter().filter_map(|(_, _, w)| w.through.as_ref()).collect();
        let accepted: Set<_> = doc
            .symbols
            .keys()
            .filter(|s| kinds.iter().any(|kind| kind.matches(s, doc)))
            .collect();
        for (scope, sheet) in &self.occurrences {
            for row in by_sheet.get(sheet.as_str()).into_iter().flatten() {
                let Item::Place { symbol, value, .. } = &row.item else {
                    continue;
                };
                if !accepted.contains(symbol) {
                    continue;
                }
                let owner = format!("{scope}/{}", row.id);
                let def = &doc.symbols[symbol];
                let mut insert = |a: &str, b: &str, path: Option<&str>| {
                    let pins = [format!("{owner}.{a}"), format!("{owner}.{b}")];
                    let (Some(x), Some(y)) = (self.net_of.get(&pins[0]), self.net_of.get(&pins[1])) else {
                        return;
                    };
                    if x != y && self.terminals[&pins[0]].populated {
                        bridges.entry((x.min(y), x.max(y))).or_default().push(Bridge {
                            symbol: symbol.clone(),
                            owner: owner.clone(),
                            pins,
                            value: value.clone(),
                            path: path.map(str::to_owned),
                            default_path: def.paths.len() <= 1,
                        });
                    }
                };
                if def.paths.is_empty() && def.pins.len() == 2 {
                    let mut pins = def.pins.keys();
                    insert(pins.next().unwrap(), pins.next().unwrap(), None);
                }
                for (path, [a, b]) in &def.paths {
                    insert(a, b, Some(path));
                }
            }
        }
        let bindings: Map<_, _> = doc
            .rows
            .iter()
            .filter_map(|r| {
                if let Item::Place { bindings, .. } = &r.item {
                    Some((r.id.as_str(), bindings))
                } else {
                    None
                }
            })
            .collect();
        for (pin, role, want) in wants {
            let (owner, _) = pin.rsplit_once('.').unwrap();
            let (scope, local) = owner.rsplit_once('/').unwrap();
            let (target_owner, target_pin) = match want.to.split_once('.') {
                Some((part, p)) => (
                    format!(
                        "{scope}/{}",
                        bindings[local].get(part).map(String::as_str).unwrap_or(part)
                    ),
                    p,
                ),
                None => (owner.to_owned(), want.to.as_str()),
            };
            let mut target = format!("{target_owner}.{target_pin}");
            if !self.terminals.contains_key(&target) {
                // Only renames need this fallback. Resolve against this occurrence's symbol.
                if let Some((_, name)) = self
                    .terminals
                    .range(format!("{target_owner}.")..)
                    .take_while(|(k, _)| k.rsplit_once('.').is_some_and(|(p, _)| p == target_owner))
                    .find(|(_, t)| doc.symbols[&t.symbol].pins[&t.pin].aliases.contains(target_pin))
                {
                    target = format!("{target_owner}.{}", name.pin);
                }
            }
            let start_net = &self.net_of[pin];
            let end = self.terminals.get(&target);
            let target_net = end.map(|_| &self.net_of[&target]);
            let mut check = ConnectionCheck {
                source_role: None,
                target: target.clone(),
                satisfied: false,
                unknown: end.is_none(),
                detail: String::new(),
                bridge_pins: Vec::new(),
            };
            check.detail = match (end, target_net, &want.through) {
                (None, _, _) => format!("Target pin {target} is missing or unplaced"),
                (Some(t), _, _) if !t.populated => format!("Target {target} is not fitted"),
                _ if pin == &target => "A connection needs two different pins".into(),
                (_, Some(b), None) => {
                    check.satisfied = start_net == b;
                    if check.satisfied {
                        format!("Connected to {target}")
                    } else {
                        format!("{pin} and {target} are on different nets")
                    }
                }
                (_, Some(b), Some(kind)) if start_net == b => {
                    format!("Direct connection bypasses the required {}", kind.name())
                }
                (_, Some(b), Some(kind)) => {
                    let candidates = bridges.get(&(start_net.min(b), start_net.max(b)));
                    let path = want
                        .path
                        .as_deref()
                        .map(|p| format!(" path {p}"))
                        .unwrap_or_default();
                    let mut detail = format!("Missing {}{path} between {pin} and {target}", kind.name());
                    for part in candidates.into_iter().flatten().filter(|p| {
                        p.owner != owner
                            && p.owner != target_owner
                            && kind.matches(&p.symbol, doc)
                            && want
                                .path
                                .as_ref()
                                .map_or(p.default_path, |name| p.path.as_ref() == Some(name))
                            && (p.path.is_none() || self.net_of[&p.pins[0]] == *start_net)
                    }) {
                        check.bridge_pins = part.pins.to_vec();
                        if self.net_of[&check.bridge_pins[0]] != *start_net {
                            check.bridge_pins.reverse();
                        }
                        if let Some([min, max]) = want.values
                            && !PartValue::parse(&part.value)
                                .is_some_and(|v| min.at_most(v) && v.at_most(max))
                        {
                            if PartValue::parse(&part.value).is_none() {
                                check.unknown = true;
                                detail = format!("{} has no known nominal value", part.owner);
                                continue;
                            }
                            if check.unknown {
                                continue;
                            }
                            detail = format!(
                                "{} value {:?} does not meet {}–{} {}",
                                part.owner, part.value, min.magnitude, max.magnitude, min.unit
                            );
                            continue;
                        }
                        check.satisfied = true;
                        check.unknown = false;
                        detail = format!("Via {} ({})", part.owner, part.symbol);
                        if let Some(name) = &part.path {
                            detail.push_str(&format!(" · {name}"));
                        }
                        if !part.value.is_empty() {
                            detail.push_str(&format!(" · {}", part.value));
                        }
                        break;
                    }
                    detail
                }
                _ => unreachable!("each terminal has a net"),
            };
            if !check.satisfied {
                let mut objects = vec![pin.clone(), target];
                objects.extend(check.bridge_pins.iter().cloned());
                self.issues.push(super::graph::Issue {
                    unknown: check.unknown,
                    kind: "PinConnectionMismatch",
                    objects,
                    message: check.detail.clone(),
                });
            }
            self.connections
                .entry(pin.clone())
                .or_default()
                .insert(role.clone(), check);
        }
    }
}
