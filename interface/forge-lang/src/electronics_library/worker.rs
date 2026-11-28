//! Version 1 native worker protocol. Package authors implement ordinary Rust functions
//! over batches of fitted pin, component or track facts. The host owns object identity.
use super::*;
use crate::props::quote;
use std::io::{BufRead, Write};
pub const PROTOCOL: &str = "forgecad.electronics-worker/1";
pub const LIMIT: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Addresses,
    Components,
    Supplies,
    PinSpecifications,
    Tracks,
}
impl Input {
    pub(crate) fn word(self) -> &'static str {
        match self {
            Self::Addresses => "i2c_addresses",
            Self::Components => "components",
            Self::Supplies => "supplies",
            Self::PinSpecifications => "pin_specifications",
            Self::Tracks => "tracks",
        }
    }
    pub(crate) fn parse(s: &str) -> Option<Self> {
        [
            Self::Addresses,
            Self::Components,
            Self::Supplies,
            Self::PinSpecifications,
            Self::Tracks,
        ]
        .into_iter()
        .find(|x| x.word() == s)
    }
}
#[derive(Clone, Debug)]
pub enum Fact {
    Address {
        owner: String,
        attribute: String,
        sda: String,
        scl: String,
        address: u8,
    },
    Component {
        id: String,
        symbol: String,
        fields: BTreeMap<String, String>,
        typed: Json,
    },
    PinSpecifications {
        pin: String,
        net: String,
        symbol: String,
        function: Option<String>,
        device: Json,
        base: Json,
        active: Json,
    },
    Supply {
        pin: String,
        provider: String,
        minimum_v: f64,
        maximum_v: f64,
    },
    Track {
        id: String,
        net: String,
        layer: String,
        width_nm: i64,
    },
}
impl Fact {
    pub(crate) fn json(&self) -> Json {
        match self {
            Self::Address {
                owner,
                attribute,
                sda,
                scl,
                address,
            } => Json::obj(vec![
                ("owner", owner.clone().into()),
                ("attribute", attribute.clone().into()),
                ("sda", sda.clone().into()),
                ("scl", scl.clone().into()),
                ("address", (*address as usize).into()),
            ]),
            Self::Component {
                id,
                symbol,
                fields,
                typed,
            } => Json::obj(vec![
                ("id", id.clone().into()),
                ("symbol", symbol.clone().into()),
                (
                    "fields",
                    Json::Obj(
                        fields
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone().into()))
                            .collect(),
                    ),
                ),
                ("typed", typed.clone()),
            ]),
            Self::PinSpecifications {
                pin,
                net,
                symbol,
                function,
                device,
                base,
                active,
            } => Json::obj(vec![
                ("pin", pin.clone().into()),
                ("net", net.clone().into()),
                ("symbol", symbol.clone().into()),
                ("function", function.clone().map_or(Json::Null, Into::into)),
                ("device", device.clone()),
                ("base", base.clone()),
                ("active", active.clone()),
            ]),
            Self::Supply {
                pin,
                provider,
                minimum_v,
                maximum_v,
            } => Json::obj(vec![
                ("pin", pin.clone().into()),
                ("provider", provider.clone().into()),
                ("min_v", (*minimum_v).into()),
                ("max_v", (*maximum_v).into()),
            ]),
            Self::Track {
                id,
                net,
                layer,
                width_nm,
            } => Json::obj(vec![
                ("id", id.clone().into()),
                ("net", net.clone().into()),
                ("layer", layer.clone().into()),
                ("width_nm", (*width_nm as f64).into()),
            ]),
        }
    }
    fn decode(i: Input, j: &Json) -> Option<Self> {
        let s = |k| j.get(k)?.str().map(str::to_owned);
        Some(match i {
            Input::Components => {
                let Json::Obj(fields) = j.get("fields")? else {
                    return None;
                };
                Self::Component {
                    id: s("id")?,
                    symbol: s("symbol")?,
                    fields: fields
                        .iter()
                        .map(|(k, v)| Some((k.clone(), v.str()?.into())))
                        .collect::<Option<_>>()?,
                    typed: j.get("typed")?.clone(),
                }
            }
            Input::PinSpecifications => Self::PinSpecifications {
                pin: s("pin")?,
                net: s("net")?,
                symbol: s("symbol")?,
                function: s("function"),
                device: j.get("device")?.clone(),
                base: j.get("base")?.clone(),
                active: j.get("active")?.clone(),
            },
            Input::Supplies => Self::Supply {
                pin: s("pin")?,
                provider: s("provider")?,
                minimum_v: j.get("min_v")?.num()?,
                maximum_v: j.get("max_v")?.num()?,
            },
            Input::Addresses => {
                let n = j.get("address")?.num()?;
                if n.fract() != 0. || !(0.0..=127.0).contains(&n) {
                    return None;
                }
                Self::Address {
                    owner: s("owner")?,
                    attribute: s("attribute")?,
                    sda: s("sda")?,
                    scl: s("scl")?,
                    address: n as u8,
                }
            }
            Input::Tracks => Self::Track {
                id: s("id")?,
                net: s("net")?,
                layer: s("layer")?,
                width_nm: j.get("width_nm")?.num()? as i64,
            },
        })
    }
}
#[derive(Debug)]
pub struct Diagnostic {
    pub objects: Vec<String>,
    pub message: String,
}
pub type Evaluate = fn(&[Fact]) -> Vec<Diagnostic>;
#[derive(Clone, Copy, Debug)]
pub struct Check {
    pub rule: RuleSpec,
    pub input: Input,
    pub evaluate: Evaluate,
}
/// A mandatory input for one comparison, never a symbol completeness requirement.
/// With Components this names a component field. With PinSpecifications it names
/// a quantity in the base pin or selected function. Other symbols remain eligible.
#[derive(Clone, Copy, Debug)]
pub struct RequiredField {
    pub rule: &'static str,
    pub field: &'static str,
    pub symbol: Option<&'static str>,
}
impl RequiredField {
    pub(crate) fn absent<'a>(&self, fact: &'a Fact) -> Option<&'a str> {
        let (id, symbol, present) = match fact {
            Fact::Component {
                id, symbol, fields, ..
            } => (
                id,
                symbol,
                fields.get(self.field).is_some_and(|v| !v.trim().is_empty()),
            ),
            Fact::PinSpecifications {
                pin,
                symbol,
                base,
                active,
                ..
            } => {
                let found=[base,active].iter().any(|j|matches!(j,Json::Arr(rows) if rows.iter().any(|row|row.get("kind").and_then(Json::str)==Some(self.field))));
                (pin, symbol, found)
            }
            _ => return None,
        };
        (!present && self.symbol.is_none_or(|s| s == symbol)).then_some(id)
    }
}
impl Check {
    fn description(&self, lib: &Library) -> Json {
        Json::obj(vec![
            ("id", self.rule.id.into()),
            ("severity", self.rule.severity.word().into()),
            ("input", self.input.word().into()),
            (
                "requires",
                Json::Arr(
                    lib.requirements
                        .iter()
                        .filter(|r| r.rule == self.rule.id)
                        .map(|r| {
                            Json::obj(vec![
                                ("field", r.field.into()),
                                ("symbol", r.symbol.map_or(Json::Null, Into::into)),
                            ])
                        })
                        .collect(),
                ),
            ),
        ])
    }
}
fn type_source(id: &str, t: &sch::AttributeType) -> String {
    let units = t
        .units
        .iter()
        .map(|u| {
            let Json::Obj(pairs) = u.json() else {
                unreachable!()
            };
            let fields = pairs
                .into_iter()
                .filter(|(k, v)| v != &Json::Null && !(t.kind == sch::StrapKind::Lookup && k == "factor"))
                .map(|(k, v)| format!("{k}:{v}"))
                .collect::<Vec<_>>();
            format!("{{{}}}", fields.join(","))
        })
        .collect::<Vec<_>>();
    let mut args = vec![
        format!("label:{}", quote(&t.label)),
        format!("kind:{}", quote(t.kind.word())),
        format!("units:[{}]", units.join(",")),
    ];
    if !t.modes.is_empty() {
        args.push(format!(
            "modes:[{}]",
            t.modes.iter().map(|m| quote(m)).collect::<Vec<_>>().join(",")
        ));
    }
    if let Some([lo, hi]) = &t.range {
        args.push(format!("range:[{},{}]", quote(lo), quote(hi)));
    }
    let tables = t
        .conversions
        .tables()
        .iter()
        .map(|t| {
            let rows = t
                .rows
                .iter()
                .map(|[a, b]| format!("[{},{}]", quote(a), quote(b)))
                .collect::<Vec<_>>();
            format!(
                "{{name:{},from:{},to:{},rows:[{}]}}",
                quote(&t.name),
                quote(&t.from),
                quote(&t.to),
                rows.join(",")
            )
        })
        .collect::<Vec<_>>();
    if !tables.is_empty() {
        args.push(format!("conversions:[{}]", tables.join(",")));
    }
    format!("{id}=attribute_type({})\n", args.join(","))
}
fn describe(lib: &Library) -> Json {
    let props = lib
        .properties
        .map(|f| f())
        .unwrap_or_default()
        .iter()
        .map(|(id, t)| type_source(id, t))
        .collect::<String>();
    Json::obj(vec![
        ("name", lib.name.into()),
        ("properties", props.into()),
        (
            "checks",
            Json::Arr(lib.worker_checks.iter().map(|c| c.description(lib)).collect()),
        ),
        (
            "children",
            Json::Arr(lib.children.iter().map(|c| describe(c)).collect()),
        ),
    ])
}
fn evaluate(lib: &Library, msg: &Json) -> Result<Json, String> {
    let input = msg
        .get("input")
        .and_then(Json::str)
        .and_then(Input::parse)
        .ok_or("Unknown input")?;
    let id = msg.get("rule").and_then(Json::str).ok_or("Missing rule")?;
    fn find<'a>(lib: &'a Library, id: &str) -> Option<&'a Check> {
        lib.worker_checks
            .iter()
            .find(|r| r.rule.id == id)
            .or_else(|| lib.children.iter().find_map(|c| find(c, id)))
    }
    let rule = find(lib, id)
        .filter(|r| r.input == input)
        .ok_or("Unknown check")?;
    let Some(Json::Arr(rows)) = msg.get("facts") else {
        return Err("Missing facts".into());
    };
    let facts = rows
        .iter()
        .map(|j| Fact::decode(input, j))
        .collect::<Option<Vec<_>>>()
        .ok_or("Invalid fact")?;
    let (findings, not_run) = evidence::capture(|| (rule.evaluate)(&facts));
    let findings = Json::Arr(
        findings
            .into_iter()
            .map(|d| {
                Json::obj(vec![
                    ("message", d.message.into()),
                    (
                        "objects",
                        Json::Arr(d.objects.into_iter().map(Into::into).collect()),
                    ),
                ])
            })
            .collect(),
    );
    if not_run.is_empty() {
        return Ok(findings);
    }
    Ok(Json::obj(vec![
        ("findings", findings),
        ("not_run", Json::Arr(not_run.iter().map(|r| r.json()).collect())),
    ]))
}
/// Entrypoint for the library author's native executable.
pub fn serve(lib: &'static Library, version: &str) -> Result<(), String> {
    let mut checked = Loaded::default();
    checked.add(lib)?;
    match std::env::args().nth(1).as_deref() {
        Some("--manifest") => {
            if checked
                .libs
                .iter()
                .any(|l| l.resolve.is_some() || l.erc.is_some() || l.drc.is_some())
            {
                return Err("Native workers export callbacks with Library::with_checks".into());
            }
            println!(
                "{}",
                Json::obj(vec![
                    ("protocol", PROTOCOL.into()),
                    ("version", version.into()),
                    ("library", describe(lib))
                ])
            );
            Ok(())
        }
        Some("--worker") => {
            let stdin = std::io::stdin();
            let mut reader = stdin.lock();
            let mut out = std::io::stdout().lock();
            while let Some(msg) = read_frame(&mut reader)? {
                let result = evaluate(lib, &msg)?;
                writeln!(out, "{result}")
                    .and_then(|_| out.flush())
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        _ => Err("Use --manifest or --worker".into()),
    }
}
pub(crate) fn read_frame(reader: &mut impl BufRead) -> Result<Option<Json>, String> {
    use std::io::Read;
    let mut line = Vec::new();
    let n = reader
        .take((LIMIT + 1) as u64)
        .read_until(b'\n', &mut line)
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Ok(None);
    }
    if n > LIMIT || line.last() != Some(&b'\n') {
        return Err("Incomplete or oversized electronics worker frame".into());
    }
    Json::parse(std::str::from_utf8(&line).map_err(|e| e.to_string())?).map(Some)
}
