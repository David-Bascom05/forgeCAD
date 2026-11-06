//! Wire harnesses in an assembly: `harness NAME(..) { .. }`, or a file whose first line is
//! `harness(a, b, ..)` placed with `subassembly(FILE, a: inst, ..)`. Statements make wire types,
//! connectors (pins are a part's anchors), nodes, segments and wires; every place is a reference
//! on the assembly's parts, worked out after the solve, so the routes follow the assembly. Then
//! forge-harness routes it: paths, bundles, line-and-arc centrelines, lengths, tubes and typed
//! notes (BendTooTight, KinkAtNode, ..).

use crate::asm::{AsmVal, Issue, Level, RefDef};
use crate::builtins::{Args, LEN};
use crate::error::{Error, Op};
use crate::eval::{Eval, R, Value};
use crate::syntax::{Arg, Expr, ExprKind, Key, Span, Stmt, StmtKind};
use crate::values::{Dim, Ty, fmt};
use forge_agent::json::Json;
use forge_harness as fh;
use forge_math::{Frame, Point3, Vec3};
use std::rc::Rc;
use std::sync::Arc;

/// The built-ins of a harness block, known only inside one.
pub const BUILTINS: &[&str] = &["wire_type", "connector", "node", "segment", "wire"];

/// Wire-list CSV columns, in order.
pub const CSV_COLUMNS: &[&str] = &["id", "from", "to", "type", "colour", "path"];

// ---- values -------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum Pins {
    /// The anchors under one name (`ctrl.pin`: `pin[1]`, `pin[2]`, ..).
    Prefix(RefDef),
    /// Pins by key.
    Map(Vec<(String, RefDef)>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum HVal {
    Harness(usize),
    WireType(usize, usize),
    Connector {
        h: usize,
        k: usize,
        pins: Arc<Pins>,
    },
    Node(usize, usize),
    Segment(usize, usize),
    Wire(usize, usize),
    /// A connector's pin, by key; `r` None where the connector has no pin of that key.
    Pin {
        h: usize,
        conn: usize,
        key: String,
        r: Option<RefDef>,
    },
}

impl HVal {
    pub fn what(&self) -> &'static str {
        match self {
            HVal::Harness(_) => "harness",
            HVal::WireType(..) => "wire type",
            HVal::Connector { .. } => "connector",
            HVal::Node(..) => "node",
            HVal::Segment(..) => "segment",
            HVal::Wire(..) => "wire",
            HVal::Pin { .. } => "pin",
        }
    }
}

fn hv(v: HVal) -> Value {
    Value::Asm(Rc::new(AsmVal::Harness(v)))
}

/// `J1[3]` or `J1.a`: the connector's pin of that key.
pub fn pin_of(h: usize, conn: usize, pins: &Pins, key: Key) -> Value {
    let text = key.to_string();
    let r = match pins {
        Pins::Prefix(r) => {
            let mut r = r.clone();
            match key {
                Key::Ix(_) => {
                    if let Some(last) = r.segs.last_mut() {
                        last.index.push(key);
                    }
                }
                Key::Label(l) => r.segs.push(crate::syntax::Seg {
                    name: l,
                    index: vec![],
                }),
            }
            Some(r)
        }
        Pins::Map(m) => m.iter().find(|(k, _)| *k == text).map(|x| x.1.clone()),
    };
    hv(HVal::Pin {
        h,
        conn,
        key: text,
        r,
    })
}

// ---- what a harness is, as written -------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub bend_k: f64,
    pub k: f64,
    pub slack: f64,
    pub round_up: f64,
    pub lead: Option<f64>,
    pub insert: f64,
    pub empty_d: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeDef {
    pub id: String,
    pub od: f64,
    pub conductor: Option<f64>,
    pub min_bend: f64,
    pub colour: String,
    pub source: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnDef {
    pub id: String,
    pub face: RefDef,
    pub pins: Arc<Pins>,
    pub insert: Option<f64>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeAt {
    Ref(RefDef),
    Connector(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeDef {
    pub id: String,
    pub at: NodeAt,
    /// Along the reference's z (a connector's: out of its face).
    pub off: f64,
    /// The reference's axis routes pass along (a connector's: its z).
    pub along: Option<usize>,
    pub lead: Option<f64>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SegDef {
    pub id: String,
    pub a: usize,
    pub b: usize,
    pub via: Vec<RefDef>,
    pub k: Option<f64>,
    pub bend_k: Option<f64>,
    pub wall: f64,
    pub covering: Option<String>,
    pub service_loop: f64,
    pub d: Option<f64>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PinDef {
    pub conn: usize,
    pub key: String,
    pub r: RefDef,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WireDef {
    pub id: String,
    pub ty: usize,
    pub from: PinDef,
    pub to: PinDef,
    pub path: Option<Vec<usize>>,
    pub colour: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HarnessDef {
    pub id: String,
    pub span: Span,
    /// A harness file placed here: every item's span is the placement's.
    pub file: Option<String>,
    pub settings: Settings,
    pub types: Vec<TypeDef>,
    pub connectors: Vec<ConnDef>,
    pub nodes: Vec<NodeDef>,
    pub segments: Vec<SegDef>,
    pub wires: Vec<WireDef>,
}

impl HarnessDef {
    /// An item's name inside the harness (`W1` of `loom.W1`).
    fn local<'a>(&self, id: &'a str) -> &'a str {
        id.strip_prefix(&self.id)
            .and_then(|r| r.strip_prefix('.'))
            .unwrap_or(id)
    }
}

// ---- typed problems -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum HarnessProblem {
    /// A harness block outside an assembly, or inside a loop, function, component or harness.
    NotTopLevel,
    /// A corner tighter than its segment's minimum radius: built with the tightest arc that fits.
    BendTooTight {
        harness: String,
        segment: String,
        corner: usize,
        at: Point3,
        needed: f64,
        got: f64,
    },
    /// A wire's breakout from its bundle to its pin bends tighter than the wire may: the exit
    /// node is too near the connector for how far the pin is off the bundle.
    BreakoutTooTight {
        harness: String,
        wire: String,
        connector: String,
        needed: f64,
        got: f64,
    },
    /// A wire turns through a node with no `along:` (radians), toward a direction.
    KinkAtNode {
        harness: String,
        node: String,
        wire: String,
        angle: f64,
        toward: Vec3,
    },
    NoPath {
        harness: String,
        wire: String,
        from: String,
        to: String,
    },
    /// The given path's `at`-th segment does not join the one before it (or the last does not
    /// reach the wire's other end).
    PathBroken {
        harness: String,
        wire: String,
        at: usize,
    },
    /// `known`: the connector's pins in a map, or of its part's anchors, the ones this harness
    /// reaches.
    PinUnknown {
        harness: String,
        connector: String,
        pin: String,
        known: Vec<String>,
    },
    NoExitNode {
        harness: String,
        connector: String,
    },
    /// A reference that resolved to nothing on the solved assembly.
    BrokenRef {
        harness: String,
        item: String,
        reference: String,
    },
    UnknownWireType {
        name: String,
        known: Vec<String>,
    },
    /// A wire library row with no source: refused.
    WireRowUnsourced {
        file: String,
        line: usize,
    },
    WireRowBad {
        file: String,
        line: usize,
        why: String,
    },
    /// A wire type with a gauge and no insulation OD: its conductor's diameter is used.
    NoInsulationOd {
        wire_type: String,
        conductor: f64,
    },
    SegmentEmpty {
        harness: String,
        segment: String,
    },
    /// A harness file's first line names occurrences the placement did not give, or the
    /// placement gives ones the file does not name.
    Occurrences {
        file: String,
        missing: Vec<String>,
        unknown: Vec<String>,
    },
    /// A harness file run on its own: an assembly places it, giving these occurrences.
    PlacedOnly {
        occurrences: Vec<String>,
    },
    /// An error inside a placed harness file, where it is in that file.
    InFile {
        file: String,
        line: usize,
        col: usize,
        kind: &'static str,
        text: String,
    },
}

impl HarnessProblem {
    pub fn level(&self) -> Level {
        match self {
            HarnessProblem::BendTooTight { .. }
            | HarnessProblem::BreakoutTooTight { .. }
            | HarnessProblem::KinkAtNode { .. }
            | HarnessProblem::NoInsulationOd { .. }
            | HarnessProblem::NoPath { .. }
            | HarnessProblem::PathBroken { .. } => Level::Warning,
            HarnessProblem::SegmentEmpty { .. } | HarnessProblem::PlacedOnly { .. } => Level::Info,
            _ => Level::Error,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            HarnessProblem::NotTopLevel => "HarnessNotTopLevel",
            HarnessProblem::BendTooTight { .. } => "BendTooTight",
            HarnessProblem::KinkAtNode { .. } => "KinkAtNode",
            HarnessProblem::BreakoutTooTight { .. } => "BreakoutTooTight",
            HarnessProblem::NoPath { .. } => "NoPath",
            HarnessProblem::PathBroken { .. } => "PathBroken",
            HarnessProblem::PinUnknown { .. } => "PinUnknown",
            HarnessProblem::NoExitNode { .. } => "NoExitNode",
            HarnessProblem::BrokenRef { .. } => "HarnessBrokenRef",
            HarnessProblem::UnknownWireType { .. } => "UnknownWireType",
            HarnessProblem::WireRowUnsourced { .. } => "WireRowUnsourced",
            HarnessProblem::WireRowBad { .. } => "WireRowBad",
            HarnessProblem::NoInsulationOd { .. } => "NoInsulationOd",
            HarnessProblem::SegmentEmpty { .. } => "SegmentEmpty",
            HarnessProblem::Occurrences { .. } => "HarnessOccurrences",
            HarnessProblem::PlacedOnly { .. } => "HarnessPlacedOnly",
            HarnessProblem::InFile { .. } => "HarnessFile",
        }
    }

    pub fn text(&self) -> String {
        let list = |xs: &[String]| {
            if xs.is_empty() {
                "none".to_string()
            } else {
                xs.join(", ")
            }
        };
        match self {
            HarnessProblem::NotTopLevel => {
                "a harness is written at an assembly's top level (not in a loop, function, component or harness)".into()
            }
            HarnessProblem::BendTooTight {
                harness,
                segment,
                corner,
                needed,
                got,
                ..
            } => format!(
                "{harness}.{segment}: corner {} needs a {} mm radius and has room for {} mm (built at {} mm)",
                corner + 1,
                fmt(*needed),
                fmt(*got),
                fmt(*got)
            ),
            HarnessProblem::BreakoutTooTight {
                harness,
                wire,
                connector,
                needed,
                got,
            } => format!(
                "{harness}.{wire} bends {} mm from the bundle to its pin on {connector}, under its {} mm (move the exit node out)",
                fmt((got * 100.0).round() / 100.0),
                fmt(*needed)
            ),
            HarnessProblem::KinkAtNode {
                harness,
                node,
                wire,
                angle,
                toward,
            } => format!(
                "{harness}.{wire} turns {}deg at {harness}.{node} toward {}: the node has no direction (add along: there)",
                fmt((angle.to_degrees() * 10.0).round() / 10.0),
                toward_words(*toward)
            ),
            HarnessProblem::NoPath { harness, wire, from, to } => {
                format!("{harness}.{wire}: no chain of segments joins {from} and {to}")
            }
            HarnessProblem::PathBroken { harness, wire, at } => format!(
                "{harness}.{wire}: path segment {} does not join the one before it, or the path does not reach the wire's other end",
                at + 1
            ),
            HarnessProblem::PinUnknown {
                harness,
                connector,
                pin,
                known,
            } => format!(
                "{harness}.{connector} has no pin {pin}; the pins of it this harness reaches: {}",
                list(known)
            ),
            HarnessProblem::NoExitNode { harness, connector } => format!(
                "{harness}.{connector} has no exit node: add node({connector}, out: ..)"
            ),
            HarnessProblem::BrokenRef {
                harness,
                item,
                reference,
            } => format!("{harness}.{item}: {reference} is nothing on the solved assembly"),
            HarnessProblem::UnknownWireType { name, known } => {
                format!("no wire type {name} in the wire library; it has: {}", list(known))
            }
            HarnessProblem::WireRowUnsourced { file, line } => {
                format!("{file} line {line}: a wire library row needs its source")
            }
            HarnessProblem::WireRowBad { file, line, why } => format!("{file} line {line}: {why}"),
            HarnessProblem::NoInsulationOd { wire_type, conductor } => format!(
                "{wire_type} has no insulation OD: its conductor's {} mm is used (give od:)",
                fmt(*conductor)
            ),
            HarnessProblem::SegmentEmpty { harness, segment } => {
                format!("{harness}.{segment}: no wire runs through it")
            }
            HarnessProblem::Occurrences {
                file,
                missing,
                unknown,
            } => format!(
                "{file} wires {}: missing {}, not named by the file {}",
                list(missing),
                list(missing),
                list(unknown)
            ),
            HarnessProblem::PlacedOnly { occurrences } => format!(
                "a harness file: an assembly places it with subassembly(FILE, {})",
                occurrences.iter().map(|o| format!("{o}: ..")).collect::<Vec<_>>().join(", ")
            ),
            HarnessProblem::InFile {
                file, line, col, text, ..
            } => format!("{file}:{line}:{col}: {text}"),
        }
    }

    pub fn data(&self) -> Vec<(&'static str, Json)> {
        let s = |x: &str| Json::from(x);
        let strs = |xs: &[String]| Json::Arr(xs.iter().map(|x| s(x)).collect());
        let p3 = |p: Point3| Json::Arr(vec![p.x.into(), p.y.into(), p.z.into()]);
        match self {
            HarnessProblem::NotTopLevel => vec![],
            HarnessProblem::BendTooTight {
                harness,
                segment,
                corner,
                at,
                needed,
                got,
            } => vec![
                ("harness", s(harness)),
                ("segment", s(segment)),
                ("corner", (*corner).into()),
                ("at", p3(*at)),
                ("needed", (*needed).into()),
                ("got", (*got).into()),
            ],
            HarnessProblem::BreakoutTooTight {
                harness,
                wire,
                connector,
                needed,
                got,
            } => vec![
                ("harness", s(harness)),
                ("wire", s(wire)),
                ("connector", s(connector)),
                ("needed", (*needed).into()),
                ("got", (*got).into()),
            ],
            HarnessProblem::KinkAtNode {
                harness,
                node,
                wire,
                angle,
                toward,
            } => vec![
                ("harness", s(harness)),
                ("node", s(node)),
                ("wire", s(wire)),
                ("angle", angle.to_degrees().into()),
                (
                    "toward",
                    Json::Arr(vec![toward.x.into(), toward.y.into(), toward.z.into()]),
                ),
            ],
            HarnessProblem::NoPath {
                harness,
                wire,
                from,
                to,
            } => vec![
                ("harness", s(harness)),
                ("wire", s(wire)),
                ("from", s(from)),
                ("to", s(to)),
            ],
            HarnessProblem::PathBroken { harness, wire, at } => {
                vec![("harness", s(harness)), ("wire", s(wire)), ("at", (*at).into())]
            }
            HarnessProblem::PinUnknown {
                harness,
                connector,
                pin,
                known,
            } => vec![
                ("harness", s(harness)),
                ("connector", s(connector)),
                ("pin", s(pin)),
                ("known", strs(known)),
            ],
            HarnessProblem::NoExitNode { harness, connector } => {
                vec![("harness", s(harness)), ("connector", s(connector))]
            }
            HarnessProblem::BrokenRef {
                harness,
                item,
                reference,
            } => vec![
                ("harness", s(harness)),
                ("item", s(item)),
                ("reference", s(reference)),
            ],
            HarnessProblem::UnknownWireType { name, known } => {
                vec![("name", s(name)), ("known", strs(known))]
            }
            HarnessProblem::WireRowUnsourced { file, line } => {
                vec![("file", s(file)), ("row", (*line).into())]
            }
            HarnessProblem::WireRowBad { file, line, why } => {
                vec![("file", s(file)), ("row", (*line).into()), ("why", s(why))]
            }
            HarnessProblem::NoInsulationOd { wire_type, conductor } => {
                vec![("wire_type", s(wire_type)), ("conductor", (*conductor).into())]
            }
            HarnessProblem::SegmentEmpty { harness, segment } => {
                vec![("harness", s(harness)), ("segment", s(segment))]
            }
            HarnessProblem::Occurrences {
                file,
                missing,
                unknown,
            } => vec![
                ("file", s(file)),
                ("missing", strs(missing)),
                ("unknown", strs(unknown)),
            ],
            HarnessProblem::PlacedOnly { occurrences } => vec![("occurrences", strs(occurrences))],
            HarnessProblem::InFile {
                file,
                line,
                col,
                kind,
                ..
            } => vec![
                ("file", s(file)),
                ("file_line", (*line).into()),
                ("file_col", (*col).into()),
                ("inner", s(kind)),
            ],
        }
    }

    /// Typed edits that remove it.
    pub fn fixes(&self) -> Vec<Op> {
        vec![]
    }
}

/// A direction in world words: its axes' signs, the larger parts first (`-y, +z`).
pub fn toward_words(v: Vec3) -> String {
    let mut parts: Vec<(f64, String)> = [(v.x, "x"), (v.y, "y"), (v.z, "z")]
        .into_iter()
        .filter(|(c, _)| c.abs() >= 0.25)
        .map(|(c, n)| (c.abs(), format!("{}{n}", if c < 0.0 { "-" } else { "+" })))
        .collect();
    parts.sort_by(|a, b| b.0.total_cmp(&a.0));
    parts.into_iter().map(|p| p.1).collect::<Vec<_>>().join(", ")
}

fn problem(span: Span, p: HarnessProblem) -> Error {
    Error::Asm(Box::new(crate::asm::AsmNote {
        span,
        issue: Issue::Harness(p),
    }))
}

fn push(ev: &mut Eval, span: Span, p: HarnessProblem) {
    let n = crate::asm::AsmNote {
        span,
        issue: Issue::Harness(p),
    };
    match n.issue.level() {
        Level::Error => ev.errors.push(Error::Asm(Box::new(n))),
        _ => ev.warnings.push(crate::error::Warning::Asm(Box::new(n))),
    }
}

// ---- evaluation ---------------------------------------------------------------------------------

fn harnesses<'a>(ev: &'a mut Eval<'_>) -> &'a mut Vec<HarnessDef> {
    &mut ev
        .asm
        .as_mut()
        .expect("a harness is evaluated in an assembly")
        .harnesses
}

fn current<'a>(ev: &'a mut Eval<'_>) -> (usize, &'a mut HarnessDef) {
    let h = ev.harness.expect("inside a harness block");
    (h, &mut harnesses(ev)[h])
}

/// A reference every harness place is: worked out after the solve, as expects' probes are.
fn probe(ev: &mut Eval, r: &RefDef) {
    let d = ev.asm.as_mut().expect("an assembly");
    if !d
        .probes
        .iter()
        .any(|p| p.inst == r.inst && p.segs == r.segs && p.local == r.local)
    {
        d.probes.push(r.clone());
    }
}

fn reference(ev: &mut Eval, v: &Value, span: Span, what: &'static str) -> R<RefDef> {
    match v {
        Value::Asm(a) => match &**a {
            AsmVal::Ref(r) => {
                probe(ev, r);
                Ok(r.clone())
            }
            AsmVal::Inst(id) => {
                let r = RefDef {
                    inst: id.clone(),
                    segs: vec![],
                    span,
                    local: None,
                };
                probe(ev, &r);
                Ok(r)
            }
            _ => Err(Error::WrongType {
                span,
                what,
                want: Ty::Reference,
                found: v.ty(),
            }),
        },
        _ => Err(Error::WrongType {
            span,
            what,
            want: Ty::Reference,
            found: v.ty(),
        }),
    }
}

fn hval(v: &Value) -> Option<&HVal> {
    match v {
        Value::Asm(a) => match &**a {
            AsmVal::Harness(h) => Some(h),
            _ => None,
        },
        _ => None,
    }
}

fn want_h(v: &Value, span: Span, what: &'static str, ok: impl Fn(&HVal) -> Option<usize>) -> R<usize> {
    hval(v).and_then(ok).ok_or(Error::WrongType {
        span,
        what,
        want: Ty::Harness,
        found: v.ty(),
    })
}

fn len(a: &Args, k: &'static str) -> R<Option<f64>> {
    Ok(a.num(k, LEN)?.map(|n| n.v))
}

fn plain(a: &Args, k: &'static str) -> R<Option<f64>> {
    Ok(a.num(k, &[Dim::Ratio, Dim::Count, Dim::Length])?.map(|n| n.v))
}

fn text(a: &Args, k: &'static str) -> R<Option<String>> {
    match a.opt(k) {
        None => Ok(None),
        Some(Value::Str(s)) => Ok(Some(s.to_string())),
        Some(o) => Err(Error::WrongType {
            span: a.span(k),
            what: k,
            want: Ty::Str,
            found: o.ty(),
        }),
    }
}

fn settings(ev: &mut Eval, args: &[Arg], span: Span) -> R<Settings> {
    let a = Args::bind(
        ev,
        "harness",
        &["bend_k", "k", "slack", "round_up", "lead", "insert", "empty_d"],
        args,
        span,
    )?;
    Ok(Settings {
        bend_k: plain(&a, "bend_k")?.unwrap_or(3.0),
        k: plain(&a, "k")?.unwrap_or(1.5),
        slack: a.num("slack", &[Dim::Ratio])?.map_or(0.0, |n| n.v),
        round_up: len(&a, "round_up")?.unwrap_or(0.0),
        lead: len(&a, "lead")?,
        insert: len(&a, "insert")?.unwrap_or(0.0),
        empty_d: len(&a, "empty_d")?.unwrap_or(2.0),
    })
}

/// Evaluates a harness's statements in this document, under `name`, with `bound` given (a
/// harness file's occurrences).
fn evaluate(
    ev: &mut Eval,
    name: &str,
    span: Span,
    sets: Settings,
    file: Option<String>,
    bound: Vec<(String, Value)>,
    body: &[Stmt],
) -> R<()> {
    if ev.harness.is_some() || !ev.at_top() || ev.asm.is_none() {
        return Err(problem(span, HarnessProblem::NotTopLevel));
    }
    let d_file = file.is_some();
    ev.ctx_pending(name, span);
    let id = ev.take_id("harness", span)?;
    harnesses(ev).push(HarnessDef {
        id,
        span,
        file,
        settings: sets,
        types: vec![],
        connectors: vec![],
        nodes: vec![],
        segments: vec![],
        wires: vec![],
    });
    let h = harnesses(ev).len() - 1;
    ev.harness = Some(h);
    ev.in_block(name, bound, |ev| {
        for st in body {
            if let Err(e) = ev.stmt(st) {
                ev.errors.push(e);
                if let StmtKind::Assign { name, .. } = &st.kind {
                    let _ = ev.bind_name(name, Value::Unresolved, st.span);
                }
            }
        }
    });
    ev.harness = None;
    let def = &harnesses(ev)[h];
    let missing: Vec<(Span, String)> = def
        .connectors
        .iter()
        .enumerate()
        .filter(|(k, _)| !def.nodes.iter().any(|n| n.at == NodeAt::Connector(*k)))
        .map(|(_, c)| (c.span, def.local(&c.id).to_string()))
        .collect();
    let hid = def.id.clone();
    for (sp, c) in missing {
        push(
            ev,
            sp,
            HarnessProblem::NoExitNode {
                harness: hid.clone(),
                connector: c,
            },
        );
    }
    // a placed file's harness is bound by the assignment placing it
    if d_file {
        return Ok(());
    }
    ev.bind_name(name, hv(HVal::Harness(h)), span)
}

/// `harness NAME(..) { .. }`.
pub fn block(ev: &mut Eval, s: &Stmt, name: &str, args: &[Arg], body: &[Stmt]) -> R<()> {
    let sets = settings(ev, args, s.span)?;
    evaluate(ev, name, s.span, sets, None, vec![], body)
}

/// A harness file's first line (`harness(a, b)`): the names of the occurrences it wires.
pub fn header(prog: &[Stmt]) -> Option<(Vec<String>, &[Arg])> {
    let first = prog.first()?;
    let StmtKind::Expr(Expr {
        kind: ExprKind::Call(f, args),
        ..
    }) = &first.kind
    else {
        return None;
    };
    if !matches!(&f.kind, ExprKind::Name(n) if n == "harness") {
        return None;
    }
    let mut names = vec![];
    let mut settings = None;
    for (k, a) in args.iter().enumerate() {
        match (&a.name, &a.value.kind) {
            (None, ExprKind::Name(n)) if settings.is_none() => names.push(n.clone()),
            (Some(_), _) => {
                settings.get_or_insert(k);
            }
            _ => return None,
        }
    }
    Some((names, &args[settings.unwrap_or(args.len())..]))
}

/// A harness file run on its own: nothing to wire until an assembly places it.
pub fn placed_only(ev: &mut Eval, span: Span, names: &[String]) {
    push(
        ev,
        span,
        HarnessProblem::PlacedOnly {
            occurrences: names.to_vec(),
        },
    );
}

/// `subassembly(FILE, a: inst, ..)` of a harness file: its statements evaluated here, as a
/// harness named by the assignment, the file's occurrences bound to the ones given. None: not a
/// harness file.
pub fn place(ev: &mut Eval, args: &[Arg], span: Span) -> Option<R<Value>> {
    let first = args.first()?;
    let ExprKind::Str(file) = &first.value.kind else {
        return None;
    };
    if first.name.is_some() {
        return None;
    }
    let path = crate::asm::locate(&ev.dir, &ev.root, file)?;
    let src = crate::asm::read_capped(&path)?;
    let prog = crate::syntax::parse(&src).ok()?;
    let (names, sets_args) = header(&prog)?;
    let name = ev.binding(span).unwrap_or_else(|| "harness".into());
    Some(place_file(
        ev,
        file,
        &src,
        &prog,
        names,
        sets_args,
        &args[1..],
        &name,
        span,
    ))
}

#[allow(clippy::too_many_arguments)]
fn place_file(
    ev: &mut Eval,
    file: &str,
    src: &str,
    prog: &[Stmt],
    names: Vec<String>,
    sets_args: &[Arg],
    given: &[Arg],
    name: &str,
    span: Span,
) -> R<Value> {
    let mut bound = vec![];
    let mut unknown = vec![];
    for a in given {
        let Some((n, _)) = &a.name else {
            return Err(Error::TooManyArgs {
                span: a.value.span,
                func: "subassembly".into(),
                max: 1,
            });
        };
        let v = ev.expr(&a.value)?;
        if names.contains(n) {
            bound.push((n.clone(), v));
        } else {
            unknown.push(n.clone());
        }
    }
    let missing: Vec<String> = names
        .iter()
        .filter(|n| !bound.iter().any(|b| b.0 == **n))
        .cloned()
        .collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(problem(
            span,
            HarnessProblem::Occurrences {
                file: file.into(),
                missing,
                unknown,
            },
        ));
    }
    // the file's text and spans are its own: what goes wrong there is said at the placement
    let saved = ev.src;
    let (e0, w0) = (ev.errors.len(), ev.warnings.len());
    ev.src = crate::asm::kept_text(src.to_string());
    let sets = settings(ev, sets_args, span);
    let r = sets.and_then(|s| evaluate(ev, name, span, s, Some(file.into()), bound, &prog[1..]));
    ev.src = saved;
    let inner: Vec<Error> = ev.errors.split_off(e0);
    ev.warnings.truncate(w0);
    if let Some(h) = ev
        .harness
        .or(ev.asm.as_ref().map(|d| d.harnesses.len().saturating_sub(1)))
    {
        respan(&mut harnesses(ev)[h], span);
    }
    for e in inner.into_iter().chain(r.err()) {
        let (line, col) = e.span().map_or((0, 0), |s| {
            let (l, c, ..) = s.lines(src);
            (l, c)
        });
        let p = match e {
            Error::Asm(n) if matches!(n.issue, Issue::Harness(_)) => {
                let Issue::Harness(p) = n.issue else {
                    unreachable!()
                };
                p
            }
            e => HarnessProblem::InFile {
                file: file.into(),
                line,
                col,
                kind: e.kind(),
                text: e.text(),
            },
        };
        push(ev, span, p);
    }
    Ok(hv(HVal::Harness(
        ev.asm.as_ref().map_or(0, |d| d.harnesses.len().saturating_sub(1)),
    )))
}

fn respan(d: &mut HarnessDef, span: Span) {
    for t in &mut d.types {
        t.span = span;
    }
    for c in &mut d.connectors {
        c.span = span;
    }
    for n in &mut d.nodes {
        n.span = span;
    }
    for s in &mut d.segments {
        s.span = span;
    }
    for w in &mut d.wires {
        w.span = span;
    }
}

/// A harness built-in's name, as a value, inside a harness block.
pub fn constant(ev: &Eval, name: &str) -> Option<Value> {
    ev.harness?;
    BUILTINS.iter().find(|b| **b == name).map(|b| Value::Builtin(b))
}

pub fn call(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    if ev.harness.is_none() {
        return Err(Error::UnknownName {
            span,
            name: name.into(),
        });
    }
    match name {
        "wire_type" => wire_type(ev, args, span),
        "connector" => connector(ev, args, span),
        "node" => node(ev, args, span),
        "segment" => segment(ev, args, span),
        _ => wire(ev, args, span),
    }
}

fn wire_type(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let a = Args::bind(
        ev,
        "wire_type",
        &["name", "awg", "od", "min_bend", "colour", "source", "color"],
        args,
        span,
    )?;
    let id = ev.take_id("wire_type", span)?;
    let row = match text(&a, "name")? {
        Some(n) => Some(library_row(ev, &n, a.span("name"))?),
        None => None,
    };
    let awg = plain(&a, "awg")?.or(row.as_ref().and_then(|r| r.awg));
    let conductor = awg.map(fh::awg_d);
    let od = match len(&a, "od")?.or(row.as_ref().and_then(|r| r.od)) {
        Some(od) => od,
        None => match conductor {
            Some(c) => {
                push(
                    ev,
                    span,
                    HarnessProblem::NoInsulationOd {
                        wire_type: id.clone(),
                        conductor: c,
                    },
                );
                c
            }
            None => {
                return Err(Error::MissingArg {
                    span,
                    func: "wire_type",
                    arg: "od (or awg)",
                });
            }
        },
    };
    let source = text(&a, "source")?
        .or(row.as_ref().map(|r| format!("{} ({})", r.source, r.file)))
        .unwrap_or_default();
    let t = TypeDef {
        id,
        od,
        conductor,
        min_bend: len(&a, "min_bend")?
            .or(row.as_ref().and_then(|r| r.min_bend))
            .unwrap_or(0.0),
        colour: text(&a, "colour")?
            .or(text(&a, "color")?)
            .or(row.as_ref().map(|r| r.colour.clone()))
            .unwrap_or_default(),
        source,
        span,
    };
    let (h, d) = current(ev);
    d.types.push(t);
    Ok(hv(HVal::WireType(h, d.types.len() - 1)))
}

fn connector(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let a = Args::bind(ev, "connector", &["face", "pins", "insert"], args, span)?;
    let face = reference(
        ev,
        &a.req("face")?,
        a.span("face"),
        "a connector's face: an anchor",
    )?;
    let pins = match a.req("pins")? {
        Value::Map(m) => {
            let mut out = vec![];
            for (k, v) in m.iter() {
                out.push((
                    k.to_string(),
                    reference(ev, v, a.span("pins"), "a pin: an anchor")?,
                ));
            }
            Pins::Map(out)
        }
        v => {
            let r = reference(ev, &v, a.span("pins"), "pins: anchors under one name, or a map")?;
            // the name alone is no place: only its pins are worked out
            let d = ev.asm.as_mut().expect("an assembly");
            d.probes.retain(|p| p != &r);
            Pins::Prefix(r)
        }
    };
    let id = ev.take_id("connector", span)?;
    let insert = len(&a, "insert")?;
    let (h, d) = current(ev);
    let pins = Arc::new(pins);
    d.connectors.push(ConnDef {
        id,
        face,
        pins: pins.clone(),
        insert,
        span,
    });
    Ok(hv(HVal::Connector {
        h,
        k: d.connectors.len() - 1,
        pins,
    }))
}

fn axis(a: &Args, k: &'static str) -> R<Option<usize>> {
    match text(a, k)?.as_deref() {
        None => Ok(None),
        Some("x") => Ok(Some(0)),
        Some("y") => Ok(Some(1)),
        Some("z") => Ok(Some(2)),
        Some(_) => Err(Error::WrongType {
            span: a.span(k),
            what: "along: \"x\", \"y\" or \"z\" (the reference's axis)",
            want: Ty::Str,
            found: Ty::Str,
        }),
    }
}

fn node(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let a = Args::bind(ev, "node", &["at", "off", "out", "along", "lead"], args, span)?;
    let at_v = a.req("at")?;
    let (at, off, along) = match hval(&at_v) {
        Some(HVal::Connector { k, .. }) => (NodeAt::Connector(*k), len(&a, "out")?.unwrap_or(0.0), Some(2)),
        _ => {
            let r = reference(ev, &at_v, a.span("at"), "a node's place: an anchor, face or edge")?;
            (NodeAt::Ref(r), len(&a, "off")?.unwrap_or(0.0), axis(&a, "along")?)
        }
    };
    let id = ev.take_id("node", span)?;
    let lead = len(&a, "lead")?;
    let (h, d) = current(ev);
    d.nodes.push(NodeDef {
        id,
        at,
        off,
        along,
        lead,
        span,
    });
    Ok(hv(HVal::Node(h, d.nodes.len() - 1)))
}

fn segment(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let a = Args::bind(
        ev,
        "segment",
        &["a", "b", "via", "k", "bend_k", "wall", "covering", "loop", "d"],
        args,
        span,
    )?;
    let node_of = |v: &Value, sp: Span| {
        want_h(v, sp, "a segment's end: a node", |h| match h {
            HVal::Node(_, k) => Some(*k),
            _ => None,
        })
    };
    let na = node_of(&a.req("a")?, a.span("a"))?;
    let nb = node_of(&a.req("b")?, a.span("b"))?;
    let mut via = vec![];
    match a.opt("via") {
        None => {}
        Some(Value::List(xs)) => {
            for x in xs.iter() {
                via.push(reference(ev, x, a.span("via"), "a via: an anchor, face or edge")?);
            }
        }
        Some(o) => {
            return Err(Error::WrongType {
                span: a.span("via"),
                what: "via: a list of anchors",
                want: Ty::List,
                found: o.ty(),
            });
        }
    }
    let id = ev.take_id("segment", span)?;
    let s = SegDef {
        id,
        a: na,
        b: nb,
        via,
        k: plain(&a, "k")?,
        bend_k: plain(&a, "bend_k")?,
        wall: len(&a, "wall")?.unwrap_or(0.0),
        covering: text(&a, "covering")?,
        service_loop: len(&a, "loop")?.unwrap_or(0.0),
        d: len(&a, "d")?,
        span,
    };
    let (h, d) = current(ev);
    d.segments.push(s);
    Ok(hv(HVal::Segment(h, d.segments.len() - 1)))
}

fn pin_def(ev: &mut Eval, v: &Value, span: Span) -> R<PinDef> {
    let Some(HVal::Pin { conn, key, r, .. }) = hval(v).cloned() else {
        return Err(Error::WrongType {
            span,
            what: "a wire's end: a connector's pin (J1[3], J1.a)",
            want: Ty::Harness,
            found: v.ty(),
        });
    };
    let Some(r) = r else {
        let (_, d) = current(ev);
        let c = &d.connectors[conn];
        let known = match &*c.pins {
            Pins::Map(m) => m.iter().map(|x| x.0.clone()).collect(),
            Pins::Prefix(_) => vec![],
        };
        return Err(problem(
            span,
            HarnessProblem::PinUnknown {
                harness: d.id.clone(),
                connector: d.local(&c.id).to_string(),
                pin: key,
                known,
            },
        ));
    };
    probe(ev, &r);
    Ok(PinDef { conn, key, r })
}

fn wire(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let a = Args::bind(
        ev,
        "wire",
        &["from", "to", "type", "through", "colour", "color"],
        args,
        span,
    )?;
    let from = pin_def(ev, &a.req("from")?, a.span("from"))?;
    let to = pin_def(ev, &a.req("to")?, a.span("to"))?;
    let ty = want_h(
        &a.req("type")?,
        a.span("type"),
        "a wire's type: a wire_type",
        |h| match h {
            HVal::WireType(_, k) => Some(*k),
            _ => None,
        },
    )?;
    let path = match a.opt("through") {
        None => None,
        Some(Value::List(xs)) => Some(
            xs.iter()
                .map(|x| {
                    want_h(x, a.span("through"), "through: segments", |h| match h {
                        HVal::Segment(_, k) => Some(*k),
                        _ => None,
                    })
                })
                .collect::<R<Vec<usize>>>()?,
        ),
        Some(o) => {
            return Err(Error::WrongType {
                span: a.span("through"),
                what: "through: a list of segments",
                want: Ty::List,
                found: o.ty(),
            });
        }
    };
    let id = ev.take_id("wire", span)?;
    let colour = text(&a, "colour")?.or(text(&a, "color")?);
    let (h, d) = current(ev);
    d.wires.push(WireDef {
        id,
        ty,
        from,
        to,
        path,
        colour,
        span,
    });
    Ok(hv(HVal::Wire(h, d.wires.len() - 1)))
}

// ---- the wire library ---------------------------------------------------------------------------

/// One row of a wire library file: `name,awg,mm2,od,min_bend,colour,source`.
#[derive(Clone, Debug, PartialEq)]
pub struct LibRow {
    pub name: String,
    pub awg: Option<f64>,
    pub mm2: Option<f64>,
    pub od: Option<f64>,
    pub min_bend: Option<f64>,
    pub colour: String,
    pub source: String,
    pub file: String,
    pub line: usize,
}

/// The library's files: the project's `wires/*.csv`, then the user's
/// (`$XDG_CONFIG_HOME/forgecad/wires`, else `~/.config/forgecad/wires`), each sorted by name.
pub fn library_files(dir: Option<&std::path::Path>) -> Vec<std::path::PathBuf> {
    let user = forge_material::user_dir().and_then(|m| m.parent().map(|p| p.join("wires")));
    let mut out = vec![];
    for d in dir.map(|d| d.join("wires")).into_iter().chain(user) {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        let mut fs: Vec<_> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "csv"))
            .collect();
        fs.sort();
        out.extend(fs);
    }
    out
}

/// A library file's rows; Err: the first bad row.
pub fn parse_library(file: &str, text: &str) -> Result<Vec<LibRow>, HarnessProblem> {
    let mut rows = vec![];
    for (k, line) in text.lines().enumerate() {
        let n = k + 1;
        let cols = split_csv(line);
        if cols.iter().all(|c| c.is_empty()) || cols[0].starts_with('#') || (k == 0 && cols[0] == "name") {
            continue;
        }
        let bad = |why: String| HarnessProblem::WireRowBad {
            file: file.into(),
            line: n,
            why,
        };
        if cols.len() != 7 {
            return Err(bad(format!(
                "{} columns, not 7 (name,awg,mm2,od,min_bend,colour,source)",
                cols.len()
            )));
        }
        let num = |i: usize, what: &str| -> Result<Option<f64>, HarnessProblem> {
            let c = cols[i].trim();
            if c.is_empty() {
                return Ok(None);
            }
            c.parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .map(Some)
                .ok_or_else(|| bad(format!("{what} {c:?} is not a number")))
        };
        let row = LibRow {
            name: cols[0].trim().into(),
            awg: num(1, "awg")?,
            mm2: num(2, "mm2")?,
            od: num(3, "od")?,
            min_bend: num(4, "min_bend")?,
            colour: cols[5].trim().into(),
            source: cols[6].trim().into(),
            file: file.into(),
            line: n,
        };
        if row.source.is_empty() {
            return Err(HarnessProblem::WireRowUnsourced {
                file: file.into(),
                line: n,
            });
        }
        rows.push(row);
    }
    Ok(rows)
}

fn library_row(ev: &mut Eval, name: &str, span: Span) -> R<LibRow> {
    let mut known = vec![];
    for f in library_files(ev.dir.as_deref()) {
        let shown = f.to_string_lossy().to_string();
        let Some(text) = read_text(&f) else { continue };
        let rows = parse_library(&shown, &text).map_err(|p| problem(span, p))?;
        if let Some(r) = rows.iter().find(|r| r.name == name) {
            return Ok(r.clone());
        }
        known.extend(rows.into_iter().map(|r| r.name));
    }
    Err(problem(
        span,
        HarnessProblem::UnknownWireType {
            name: name.into(),
            known,
        },
    ))
}

/// A library file's text, its read recorded so an edit to it re-runs the document.
fn read_text(p: &std::path::Path) -> Option<String> {
    let bytes = std::fs::canonicalize(p)
        .ok()
        .and_then(|real| crate::deps::read_plain(&real, crate::asm::LIMIT).ok());
    crate::deps::read(p, bytes.as_deref());
    String::from_utf8(bytes?).ok()
}

/// A CSV line's fields: commas split them, double quotes hold commas and `""` is a quote.
pub fn split_csv(line: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut quoted = false;
    let mut chars = line.trim_end_matches('\r').chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                out.last_mut().expect("a field").push('"');
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(String::new()),
            c => out.last_mut().expect("a field").push(c),
        }
    }
    out
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

// ---- after the solve ----------------------------------------------------------------------------

/// How far a drawn wire goes into its pin, at most (mm).
const MAX_IN: f64 = 3.0;

/// A wire's colour as drawn: its code (IEC 60757 letters, `GN/YE` by its first) or English word,
/// else one of a palette by its name, so wires without one still tell apart.
pub fn rgb(colour: &str, name: &str) -> [f32; 3] {
    let c = colour
        .split(['/', '-', ' '])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let known = match c.as_str() {
        "bk" | "black" => Some([0.08, 0.08, 0.09]),
        "bn" | "brown" => Some([0.45, 0.26, 0.12]),
        "rd" | "red" => Some([0.80, 0.10, 0.10]),
        "og" | "orange" => Some([0.95, 0.50, 0.08]),
        "ye" | "yellow" => Some([0.95, 0.85, 0.15]),
        "gn" | "green" => Some([0.10, 0.60, 0.20]),
        "bu" | "blue" => Some([0.12, 0.32, 0.85]),
        "vt" | "violet" | "purple" => Some([0.52, 0.22, 0.75]),
        "gy" | "grey" | "gray" => Some([0.55, 0.55, 0.57]),
        "wh" | "white" => Some([0.93, 0.93, 0.92]),
        "pk" | "pink" => Some([0.96, 0.55, 0.70]),
        "gd" | "gold" => Some([0.83, 0.68, 0.22]),
        "tq" | "turquoise" => Some([0.10, 0.75, 0.75]),
        "sr" | "silver" => Some([0.75, 0.76, 0.78]),
        _ => None,
    };
    known.unwrap_or_else(|| {
        const PALETTE: [[f32; 3]; 10] = [
            [0.85, 0.37, 0.01],
            [0.46, 0.44, 0.70],
            [0.11, 0.62, 0.47],
            [0.91, 0.16, 0.54],
            [0.40, 0.65, 0.12],
            [0.90, 0.67, 0.01],
            [0.65, 0.46, 0.11],
            [0.12, 0.47, 0.71],
            [0.89, 0.10, 0.11],
            [0.42, 0.24, 0.60],
        ];
        let h = name
            .bytes()
            .fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b)));
        PALETTE[h as usize % PALETTE.len()]
    })
}

/// A routed harness, for the report, the browser and the CSV export.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HarnessOut {
    pub id: String,
    pub file: Option<String>,
    pub connectors: Vec<ConnOut>,
    pub nodes: Vec<NodeOut>,
    pub segments: Vec<SegOut>,
    pub wires: Vec<WireOut>,
    /// Wires by type and coverings by name: total length (mm); connectors: count.
    pub bom: Vec<BomRow>,
    /// Each covered segment's sleeve, in world space (none for a bare segment), by segment.
    pub tubes: Vec<Vec<Arc<forge_topo::Body>>>,
    /// Each wire's own centreline, pin to pin, in world space, and its radius (by wire; none
    /// where it has no path).
    pub paths: Vec<Option<(Vec<Point3>, f64)>>,
    /// Where each statement is, by item id (`loom.W1`).
    pub spans: Vec<(String, Span)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnOut {
    pub name: String,
    pub face: String,
    pub pins: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeOut {
    pub name: String,
    pub at: Point3,
    pub directed: bool,
    pub degree: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SegOut {
    pub name: String,
    pub a: String,
    pub b: String,
    pub length: f64,
    pub d: f64,
    pub r_min: f64,
    /// The tightest corner's radius (None: no corner).
    pub r_least: Option<f64>,
    pub wires: Vec<String>,
    pub covering: Option<String>,
    /// The diameter the wires' packing takes (the rule's `d` may be less, or more).
    pub d_pack: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WireOut {
    pub name: String,
    pub from: String,
    pub to: String,
    pub ty: String,
    pub colour: String,
    pub od: f64,
    /// The path as written (None: chosen), and the one taken.
    pub given: bool,
    pub path: Vec<String>,
    pub routed: Option<fh::WireOut>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BomRow {
    pub item: String,
    pub qty: f64,
    pub unit: &'static str,
}

/// Every harness of the document, routed: frames from the solve (`world`, None: nothing there;
/// `refs_of`, the names a reference's part does have), problems pushed onto `ev`.
pub fn finish(
    ev: &mut Eval,
    defs: &[HarnessDef],
    world: &dyn Fn(&RefDef) -> Option<Frame>,
    refs_of: &dyn Fn(&RefDef) -> Vec<String>,
    tubes: bool,
) -> Vec<HarnessOut> {
    defs.iter()
        .map(|d| route_one(ev, d, world, refs_of, tubes))
        .collect()
}

fn ref_text(r: &RefDef) -> String {
    let mut t = r.inst.clone();
    if !r.segs.is_empty() {
        t.push('.');
        t.push_str(&crate::error::path_text(&r.segs));
    }
    t
}

fn route_one(
    ev: &mut Eval,
    d: &HarnessDef,
    world: &dyn Fn(&RefDef) -> Option<Frame>,
    refs_of: &dyn Fn(&RefDef) -> Vec<String>,
    tubes: bool,
) -> HarnessOut {
    let st = &d.settings;
    let mut out = HarnessOut {
        id: d.id.clone(),
        file: d.file.clone(),
        ..HarnessOut::default()
    };
    let mut broken = false;
    let mut at = |ev: &mut Eval, item: &str, span: Span, r: &RefDef| -> Option<Frame> {
        let f = world(r);
        if f.is_none() {
            broken = true;
            push(
                ev,
                span,
                HarnessProblem::BrokenRef {
                    harness: d.id.clone(),
                    item: d.local(item).into(),
                    reference: ref_text(r),
                },
            );
        }
        f
    };
    let faces: Vec<Option<Frame>> = d
        .connectors
        .iter()
        .map(|c| at(ev, &c.id, c.span, &c.face))
        .collect();
    let mut nodes = vec![];
    for n in &d.nodes {
        let f = match &n.at {
            NodeAt::Connector(k) => faces[*k],
            NodeAt::Ref(r) => at(ev, &n.id, n.span, r),
        };
        let f = f.unwrap_or(Frame::WORLD);
        let dir = n.along.map(|a| match a {
            0 => f.x(),
            1 => f.y(),
            _ => f.z(),
        });
        nodes.push(fh::Node {
            id: d.local(&n.id).into(),
            at: f.origin + f.z().get() * n.off,
            dir,
            lead: n.lead.or(st.lead),
        });
    }
    let mut segments = vec![];
    for s in &d.segments {
        let via = s
            .via
            .iter()
            .map(|r| at(ev, &s.id, s.span, r).map_or(Point3::ORIGIN, |f| f.origin))
            .collect();
        segments.push(fh::Segment {
            id: d.local(&s.id).into(),
            a: s.a,
            b: s.b,
            via,
            k: s.k.unwrap_or(st.k),
            bend_k: s.bend_k.unwrap_or(st.bend_k),
            wall: s.wall,
            service_loop: s.service_loop,
            d: s.d,
        });
    }
    let types = d
        .types
        .iter()
        .map(|t| fh::WireType {
            id: d.local(&t.id).into(),
            od: t.od,
            min_bend: t.min_bend,
        })
        .collect();
    // each connector's pins: the ones its wires use, in the order first used
    let mut pins: Vec<Vec<(String, Point3)>> = vec![vec![]; d.connectors.len()];
    let mut ends = vec![];
    for w in &d.wires {
        let mut end = |ev: &mut Eval, p: &PinDef| -> fh::End {
            let list = &mut pins[p.conn];
            let k = match list.iter().position(|x| x.0 == p.key) {
                Some(k) => k,
                None => {
                    let f = world(&p.r);
                    if f.is_none() {
                        broken = true;
                        let c = &d.connectors[p.conn];
                        let prefix = match &*c.pins {
                            Pins::Prefix(r) => crate::error::path_text(&r.segs),
                            Pins::Map(_) => String::new(),
                        };
                        let known = match &*c.pins {
                            Pins::Map(m) => m.iter().map(|x| x.0.clone()).collect(),
                            Pins::Prefix(_) => refs_of(&p.r)
                                .into_iter()
                                .filter_map(|n| {
                                    n.strip_prefix(&prefix).map(|x| {
                                        x.trim_start_matches(['[', '.']).trim_end_matches(']').to_string()
                                    })
                                })
                                .filter(|x| !x.is_empty())
                                .collect(),
                        };
                        push(
                            ev,
                            w.span,
                            HarnessProblem::PinUnknown {
                                harness: d.id.clone(),
                                connector: d.local(&c.id).into(),
                                pin: p.key.clone(),
                                known,
                            },
                        );
                    }
                    list.push((p.key.clone(), f.map_or(Point3::ORIGIN, |f| f.origin)));
                    list.len() - 1
                }
            };
            fh::End {
                connector: p.conn,
                pin: k,
            }
        };
        let from = end(ev, &w.from);
        let to = end(ev, &w.to);
        ends.push((from, to));
    }
    let exits: Vec<Option<usize>> = (0..d.connectors.len())
        .map(|k| d.nodes.iter().position(|n| n.at == NodeAt::Connector(k)))
        .collect();
    if broken || exits.iter().any(Option::is_none) {
        return out;
    }
    let connectors = d
        .connectors
        .iter()
        .zip(&pins)
        .zip(&exits)
        .map(|((c, p), e)| fh::Connector {
            id: d.local(&c.id).into(),
            exit: e.expect("checked"),
            pins: p.clone(),
            insert: c.insert.unwrap_or(st.insert),
        })
        .collect();
    let wires = d
        .wires
        .iter()
        .zip(&ends)
        .map(|(w, (from, to))| fh::Wire {
            id: d.local(&w.id).into(),
            ty: w.ty,
            from: *from,
            to: *to,
            path: w.path.clone(),
        })
        .collect();
    let net = fh::Network {
        nodes,
        segments,
        types,
        connectors,
        wires,
        settings: fh::Settings {
            slack: st.slack,
            round_up: st.round_up,
            empty_d: st.empty_d,
        },
    };
    let r = fh::route(&net);
    let local = |id: &str| d.local(id).to_string();
    for n in &r.notes {
        let (span, p) = match *n {
            fh::Note::BendTooTight {
                segment,
                corner,
                at,
                needed,
                got,
            } => (
                d.segments[segment].span,
                HarnessProblem::BendTooTight {
                    harness: d.id.clone(),
                    segment: local(&d.segments[segment].id),
                    corner,
                    at,
                    needed,
                    got,
                },
            ),
            fh::Note::KinkAtNode {
                node,
                wire,
                angle,
                toward,
            } => (
                d.nodes[node].span,
                HarnessProblem::KinkAtNode {
                    harness: d.id.clone(),
                    node: local(&d.nodes[node].id),
                    wire: local(&d.wires[wire].id),
                    angle,
                    toward: toward.get(),
                },
            ),
            fh::Note::NoPath { wire } => {
                let w = &d.wires[wire];
                (
                    w.span,
                    HarnessProblem::NoPath {
                        harness: d.id.clone(),
                        wire: local(&w.id),
                        from: format!("{}.{}", local(&d.connectors[w.from.conn].id), w.from.key),
                        to: format!("{}.{}", local(&d.connectors[w.to.conn].id), w.to.key),
                    },
                )
            }
            fh::Note::PathBroken { wire, at } => (
                d.wires[wire].span,
                HarnessProblem::PathBroken {
                    harness: d.id.clone(),
                    wire: local(&d.wires[wire].id),
                    at,
                },
            ),
            fh::Note::SegmentEmpty { segment } => (
                d.segments[segment].span,
                HarnessProblem::SegmentEmpty {
                    harness: d.id.clone(),
                    segment: local(&d.segments[segment].id),
                },
            ),
        };
        push(ev, span, p);
    }
    for (k, end, got) in report(&mut out, d, &net, &r, tubes) {
        let w = &d.wires[k];
        let pin = if end == 0 { &w.from } else { &w.to };
        push(
            ev,
            w.span,
            HarnessProblem::BreakoutTooTight {
                harness: d.id.clone(),
                wire: local(&w.id),
                connector: local(&d.connectors[pin.conn].id),
                needed: d.types[w.ty].min_bend,
                got,
            },
        );
    }
    out
}

/// The report; and each breakout tighter than its wire's own radius: (wire, end 0 or 1, radius).
fn report(
    out: &mut HarnessOut,
    d: &HarnessDef,
    net: &fh::Network,
    r: &fh::Routed,
    tubes: bool,
) -> Vec<(usize, usize, f64)> {
    let local = |id: &str| d.local(id).to_string();
    out.connectors = d
        .connectors
        .iter()
        .zip(&net.connectors)
        .map(|(c, n)| ConnOut {
            name: local(&c.id),
            face: ref_text(&c.face),
            pins: n.pins.iter().map(|p| p.0.clone()).collect(),
        })
        .collect();
    out.nodes = net
        .nodes
        .iter()
        .enumerate()
        .map(|(k, n)| NodeOut {
            name: n.id.clone(),
            at: n.at,
            directed: n.dir.is_some(),
            degree: net.segments.iter().filter(|s| s.a == k).count()
                + net.segments.iter().filter(|s| s.b == k).count(),
        })
        .collect();
    out.segments = d
        .segments
        .iter()
        .zip(&r.segments)
        .map(|(s, o)| SegOut {
            name: local(&s.id),
            a: net.nodes[s.a].id.clone(),
            b: net.nodes[s.b].id.clone(),
            length: o.length,
            d: o.d,
            r_min: o.r_min,
            r_least: o.corners.iter().map(|c| c.got).reduce(f64::min),
            wires: o.wires.iter().map(|&w| net.wires[w].id.clone()).collect(),
            covering: s.covering.clone(),
            d_pack: 0.0,
        })
        .collect();
    out.wires = d
        .wires
        .iter()
        .zip(&r.wires)
        .map(|(w, o)| {
            let t = &d.types[w.ty];
            WireOut {
                name: local(&w.id),
                from: format!("{}.{}", local(&d.connectors[w.from.conn].id), w.from.key),
                to: format!("{}.{}", local(&d.connectors[w.to.conn].id), w.to.key),
                ty: local(&t.id),
                colour: w.colour.clone().unwrap_or_else(|| t.colour.clone()),
                od: t.od,
                given: w.path.is_some(),
                path: o
                    .as_ref()
                    .map(|o| o.path.iter().map(|&(s, _)| net.segments[s].id.clone()).collect())
                    .unwrap_or_default(),
                routed: o.clone(),
            }
        })
        .collect();
    // the BOM: wire by type (cut lengths summed), coverings by length, connectors
    let mut bom: Vec<BomRow> = vec![];
    let mut add = |item: String, qty: f64, unit: &'static str| match bom.iter_mut().find(|b| b.item == item) {
        Some(b) => b.qty += qty,
        None => bom.push(BomRow { item, qty, unit }),
    };
    for w in &out.wires {
        if let Some(o) = &w.routed {
            add(format!("wire {}", w.ty), o.length, "mm");
        }
    }
    for s in &out.segments {
        if let Some(c) = &s.covering {
            add(format!("covering {c}"), s.length, "mm");
        }
    }
    for c in &out.connectors {
        add(
            format!(
                "connector {} ({})",
                c.name,
                c.face.split('.').next().unwrap_or("")
            ),
            1.0,
            "each",
        );
    }
    out.bom = bom;
    out.spans = d
        .connectors
        .iter()
        .map(|c| (c.id.clone(), c.span))
        .chain(d.nodes.iter().map(|n| (n.id.clone(), n.span)))
        .chain(d.segments.iter().map(|s| (s.id.clone(), s.span)))
        .chain(d.wires.iter().map(|w| (w.id.clone(), w.span)))
        .chain(d.types.iter().map(|t| (t.id.clone(), t.span)))
        .collect();
    let slots = fh::wires::slots(net, r);
    for (s, sl) in out.segments.iter_mut().zip(&slots) {
        s.d_pack = sl.d;
    }
    let paths = fh::wires::paths(net, r, &slots, MAX_IN);
    let mut tight = vec![];
    for (k, p) in paths.iter().enumerate() {
        let need = net.types[net.wires[k].ty].min_bend;
        for (end, got) in p.iter().flat_map(|p| p.breakout.iter().enumerate()) {
            if let Some(got) = got
                && *got < need * (1.0 - 1e-9)
            {
                tight.push((k, end, *got));
            }
        }
    }
    if tubes {
        // each wire its own, a covered segment's sleeve round its wires
        out.paths = paths.into_iter().map(|p| p.map(|w| (w.points, w.r))).collect();
        out.tubes = r
            .segments
            .iter()
            .enumerate()
            .map(|(k, s)| {
                if d.segments[k].covering.is_none() {
                    return vec![];
                }
                let od = s.d.max(slots[k].d + 2.0 * d.segments[k].wall);
                fh::tube::tube(&s.pieces, od, crate::asm::hid(&d.segments[k].id))
                    .into_iter()
                    .map(Arc::new)
                    .collect()
            })
            .collect();
    }
    tight
}

// ---- JSON and CSV -------------------------------------------------------------------------------

pub fn json(h: &HarnessOut) -> Json {
    let s = |x: &str| Json::from(x);
    let strs = |xs: &[String]| Json::Arr(xs.iter().map(|x| s(x)).collect());
    let opt = |x: Option<f64>| x.map_or(Json::Null, Json::from);
    Json::obj(vec![
        ("id", s(&h.id)),
        ("file", h.file.as_deref().map_or(Json::Null, s)),
        (
            "connectors",
            Json::Arr(
                h.connectors
                    .iter()
                    .map(|c| {
                        Json::obj(vec![
                            ("name", s(&c.name)),
                            ("face", s(&c.face)),
                            ("pins", strs(&c.pins)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "nodes",
            Json::Arr(
                h.nodes
                    .iter()
                    .map(|n| {
                        Json::obj(vec![
                            ("name", s(&n.name)),
                            ("at", Json::Arr(vec![n.at.x.into(), n.at.y.into(), n.at.z.into()])),
                            ("directed", n.directed.into()),
                            ("degree", n.degree.into()),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "segments",
            Json::Arr(
                h.segments
                    .iter()
                    .map(|g| {
                        Json::obj(vec![
                            ("name", s(&g.name)),
                            ("a", s(&g.a)),
                            ("b", s(&g.b)),
                            ("length", g.length.into()),
                            ("d", g.d.into()),
                            ("r_min", g.r_min.into()),
                            ("r_least", opt(g.r_least)),
                            ("d_pack", g.d_pack.into()),
                            ("wires", strs(&g.wires)),
                            ("covering", g.covering.as_deref().map_or(Json::Null, s)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "wires",
            Json::Arr(
                h.wires
                    .iter()
                    .map(|w| {
                        let mut f = vec![
                            ("name", s(&w.name)),
                            ("from", s(&w.from)),
                            ("to", s(&w.to)),
                            ("type", s(&w.ty)),
                            ("colour", s(&w.colour)),
                            ("od", w.od.into()),
                            ("path_given", w.given.into()),
                            ("path", strs(&w.path)),
                            ("routed", w.routed.is_some().into()),
                        ];
                        if let Some(o) = &w.routed {
                            f.extend([
                                ("stubs", (o.stubs[0] + o.stubs[1]).into()),
                                ("route", o.route.into()),
                                ("loops", o.loops.into()),
                                ("slack", o.slack.into()),
                                ("insertion", o.insertion.into()),
                                ("exact", o.exact.into()),
                                ("length", o.length.into()),
                            ]);
                        }
                        Json::obj(f)
                    })
                    .collect(),
            ),
        ),
        (
            "bom",
            Json::Arr(
                h.bom
                    .iter()
                    .map(|b| {
                        Json::obj(vec![
                            ("item", s(&b.item)),
                            ("qty", b.qty.into()),
                            ("unit", s(b.unit)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

/// The wire list as CSV: the import's columns, then every length term.
pub fn export_csv(h: &HarnessOut) -> String {
    let mut out = String::from("id,from,to,type,colour,path,route,loops,slack,insertion,exact,length\n");
    for w in &h.wires {
        let path = if w.given { w.path.join(";") } else { String::new() };
        let mut row: Vec<String> = [&w.name, &w.from, &w.to, &w.ty, &w.colour, &path]
            .iter()
            .map(|x| csv_field(x))
            .collect();
        match &w.routed {
            Some(o) => row.extend(
                [o.route, o.loops, o.slack, o.insertion, o.exact, o.length]
                    .iter()
                    .map(|x| fmt((x * 1000.0).round() / 1000.0)),
            ),
            None => row.extend(std::iter::repeat_n(String::new(), 6)),
        }
        out.push_str(&row.join(","));
        out.push('\n');
    }
    out
}

/// One wire as the import reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct CsvWire {
    pub id: String,
    pub from: String,
    pub to: String,
    pub ty: String,
    pub colour: String,
    pub path: Vec<String>,
}

/// A wire list's rows (the columns by their header, any order; extra columns ignored).
pub fn parse_wire_list(text: &str) -> Result<Vec<CsvWire>, String> {
    let mut lines = text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty());
    let Some((_, head)) = lines.next() else {
        return Ok(vec![]);
    };
    let head: Vec<String> = split_csv(head)
        .into_iter()
        .map(|c| c.trim().to_lowercase())
        .collect();
    let col = |n: &str| head.iter().position(|h| h == n);
    let need = |n: &str| col(n).ok_or_else(|| format!("the header has no {n} column"));
    let (id, from, to, ty) = (need("id")?, need("from")?, need("to")?, need("type")?);
    let (colour, path) = (col("colour").or(col("color")), col("path"));
    let mut out: Vec<CsvWire> = vec![];
    for (k, l) in lines {
        let c = split_csv(l);
        let get = |i: usize| c.get(i).map_or(String::new(), |x| x.trim().to_string());
        let w = CsvWire {
            id: get(id),
            from: get(from),
            to: get(to),
            ty: get(ty),
            colour: colour.map(get).unwrap_or_default(),
            path: path
                .map(get)
                .unwrap_or_default()
                .split(';')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect(),
        };
        for (what, v) in [("id", &w.id), ("from", &w.from), ("to", &w.to), ("type", &w.ty)] {
            if v.is_empty() {
                return Err(format!("line {}: no {what}", k + 1));
            }
        }
        if crate::error::parse_path(&w.id).is_none_or(|p| p.len() != 1 || !p[0].index.is_empty()) {
            return Err(format!("line {}: {:?} is not a name", k + 1, w.id));
        }
        if out.iter().any(|o| o.id == w.id) {
            return Err(format!("line {}: wire {} twice", k + 1, w.id));
        }
        out.push(w);
    }
    Ok(out)
}

/// `J1.3` as .fg: `J1[3]`; `J2.a`: `J2.a`.
fn pin_text(p: &str) -> Result<String, String> {
    let (c, k) = p
        .split_once('.')
        .ok_or_else(|| format!("{p:?} is not CONNECTOR.PIN"))?;
    if k.chars().all(|x| x.is_ascii_digit()) {
        Ok(format!("{c}[{k}]"))
    } else {
        Ok(format!("{c}.{k}"))
    }
}

fn wire_line(w: &CsvWire) -> Result<String, String> {
    let mut t = format!(
        "{} = wire({}, {}, type: {}",
        w.id,
        pin_text(&w.from)?,
        pin_text(&w.to)?,
        w.ty
    );
    if !w.path.is_empty() {
        t.push_str(&format!(", through: [{}]", w.path.join(", ")));
    }
    if !w.colour.is_empty() {
        t.push_str(&format!(", colour: \"{}\"", w.colour.replace('"', "")));
    }
    t.push(')');
    Ok(t)
}

/// A changed field: its name, what it was, what it is now.
pub type FieldChange = (&'static str, String, String);

/// What a re-import changes, by wire id.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Diff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    /// Each changed wire and its fields: (field, was, now).
    pub changed: Vec<(String, Vec<FieldChange>)>,
}

impl Diff {
    pub fn text(&self) -> String {
        let mut out = String::new();
        for a in &self.added {
            out.push_str(&format!("+ {a}\n"));
        }
        for r in &self.removed {
            out.push_str(&format!("- {r}\n"));
        }
        for (w, fs) in &self.changed {
            for (f, was, now) in fs {
                out.push_str(&format!("~ {w} {f}: {was} -> {now}\n"));
            }
        }
        out
    }
}

/// The wires a harness block holds, as the CSV would say them (from its text).
fn written_wires(src: &str, body: &[Stmt]) -> Vec<(CsvWire, Span)> {
    let mut out = vec![];
    for s in body {
        let StmtKind::Assign { name, value } = &s.kind else {
            continue;
        };
        let ExprKind::Call(f, args) = &value.kind else {
            continue;
        };
        if !matches!(&f.kind, ExprKind::Name(n) if n == "wire") {
            continue;
        }
        let txt = |e: &Expr| {
            src.get(e.span.lo as usize..e.span.hi as usize)
                .unwrap_or("")
                .to_string()
        };
        let pin = |e: &Expr| txt(e).replace(' ', "").replace('[', ".").replace(']', "");
        let mut w = CsvWire {
            id: name.clone(),
            from: String::new(),
            to: String::new(),
            ty: String::new(),
            colour: String::new(),
            path: vec![],
        };
        for (k, a) in args.iter().enumerate() {
            match (a.name.as_ref().map(|x| x.0.as_str()), k) {
                (None, 0) | (Some("from"), _) => w.from = pin(&a.value),
                (None, 1) | (Some("to"), _) => w.to = pin(&a.value),
                (None, 2) | (Some("type"), _) => w.ty = txt(&a.value),
                (Some("colour"), _) => w.colour = txt(&a.value).trim_matches('"').to_string(),
                (Some("through"), _) => {
                    if let ExprKind::List(xs) = &a.value.kind {
                        w.path = xs.iter().map(txt).collect();
                    }
                }
                _ => {}
            }
        }
        out.push((w, s.span));
    }
    out
}

/// A harness's statements in a document, and the prefix its ids take (none in a harness file).
fn block_of<'a>(prog: &'a [Stmt], harness: &str) -> Result<(String, &'a [Stmt]), String> {
    match prog.iter().find_map(|s| match &s.kind {
        StmtKind::Block {
            kind: crate::syntax::BlockKind::Harness,
            name,
            body,
            ..
        } if name == harness => Some(body.as_slice()),
        _ => None,
    }) {
        Some(b) => Ok((format!("{harness}."), b)),
        // a harness file: its statements after the first line
        None if header(prog).is_some() => Ok((String::new(), &prog[1..])),
        None => Err(format!("no harness {harness} in the document")),
    }
}

/// Route Wires: with `pin`, each wire written with no path gets the one the run chose
/// (`through: [..]`); without, every written path is taken out, so the shortest is chosen again.
pub fn route_ops(src: &str, routed: &HarnessOut, pin: bool) -> Result<Vec<Op>, String> {
    let prog = crate::syntax::parse(src).map_err(|e| e.text())?;
    let (prefix, body) = block_of(&prog, &routed.id)?;
    let mut ops = vec![];
    for (mut w, _) in written_wires(src, body) {
        let now = match routed.wires.iter().find(|x| x.name == w.id) {
            Some(o) if pin && w.path.is_empty() && o.routed.is_some() => o.path.clone(),
            _ if !pin && !w.path.is_empty() => vec![],
            _ => continue,
        };
        w.path = now;
        ops.push(Op::ReplaceFeature {
            id: format!("{prefix}{}", w.id),
            text: wire_line(&w)?,
        });
    }
    Ok(ops)
}

/// A wire list imported into harness `harness` of the document: the edit ops (all or nothing:
/// AddFeature for new wires, ReplaceFeature for changed ones, DeleteFeature for ones the list
/// no longer has) and the diff they make. `routed`, the harness as last run: a wire whose colour
/// is its type's has that colour.
pub fn import(
    src: &str,
    harness: &str,
    csv: &str,
    routed: Option<&HarnessOut>,
) -> Result<(Vec<Op>, Diff), String> {
    let prog = crate::syntax::parse(src).map_err(|e| e.text())?;
    let (prefix, body) = block_of(&prog, harness)?;
    let rows = parse_wire_list(csv)?;
    let mut have = written_wires(src, body);
    for (w, _) in &mut have {
        if w.colour.is_empty()
            && let Some(o) = routed.and_then(|h| h.wires.iter().find(|x| x.name == w.id))
        {
            w.colour = o.colour.clone();
        }
    }
    let mut ops = vec![];
    let mut diff = Diff::default();
    for r in &rows {
        let line = wire_line(r)?;
        match have.iter().find(|(w, _)| w.id == r.id) {
            None => {
                diff.added.push(r.id.clone());
                ops.push(Op::AddFeature {
                    before: (!prefix.is_empty()).then(|| prefix.clone()),
                    text: line,
                });
            }
            Some((w, _)) => {
                let mut fs = vec![];
                for (f, was, now) in [
                    ("from", &w.from, &r.from),
                    ("to", &w.to, &r.to),
                    ("type", &w.ty, &r.ty),
                    ("colour", &w.colour, &r.colour),
                ] {
                    if was != now {
                        fs.push((f, was.clone(), now.clone()));
                    }
                }
                if w.path != r.path {
                    fs.push(("path", w.path.join(";"), r.path.join(";")));
                }
                if !fs.is_empty() {
                    diff.changed.push((r.id.clone(), fs));
                    ops.push(Op::ReplaceFeature {
                        id: format!("{prefix}{}", r.id),
                        text: line,
                    });
                }
            }
        }
    }
    for (w, _) in &have {
        if !rows.iter().any(|r| r.id == w.id) {
            diff.removed.push(w.id.clone());
            ops.push(Op::DeleteFeature {
                id: format!("{prefix}{}", w.id),
            });
        }
    }
    Ok((ops, diff))
}
