//! Native schematic and symbol documents. Source owns drawing geometry and functional
//! intent; each elaborated sheet occurrence owns a separate graph of electrical terminals.
mod author;
mod connection;
mod pull;
mod wire_readability;
pub use connection::{ConnectionCheck, ConnectionWant, PartType, PartValue};
mod block;
mod drawing;
pub use block::{BlockPlan, block_plan, block_plan_to};
mod project;
pub use project::{
    SearchHit, board, electrical_diff, pair_suggestions, project_annotation, reannotate, search,
};
mod graph;
mod move_cache;
mod reader;
mod report;
use crate::syntax::Span;
mod inspection;
pub use drawing::{
    graphic_page, pin_inspection_page, sheet_page, sheet_pdf, sheet_selection_page, symbol_page,
};
pub use graph::Circuit;
pub use inspection::{Direction, Evidence, PinInspection, inspect_pin, wire_directions};
mod bom;
pub mod datasheet;
pub use bom::{BomLine, bom, bom_csv, bom_for};
pub use reader::{parameter_dimension, read};
pub use report::response;
use std::collections::{BTreeMap as Map, BTreeSet as Set};
use std::path::Path;

pub const NEW_SCHEMATIC: &str = "schematic\nframes = library(file: \"forge:frames\")\nmain = sheet(title: \"Main sheet\", template: frames.A4)\n";
pub const POWER_LIBRARY: &str = include_str!("power.fg");
pub fn power_symbols() -> &'static std::collections::BTreeMap<String, String> {
    static NAMES: std::sync::OnceLock<std::collections::BTreeMap<String, String>> =
        std::sync::OnceLock::new();
    NAMES.get_or_init(|| {
        read(POWER_LIBRARY, None, None)
            .expect("bundled power library")
            .symbols
            .into_iter()
            .filter_map(|(id, s)| Some((s.power_net?, id)))
            .collect()
    })
}
pub const PART_KINDS: &[&str] = &[
    "resistor",
    "capacitor",
    "inductor",
    "diode",
    "fet",
    "mosfet",
    "jfet",
    "bjt",
    "relay",
    "switch",
    "fuse",
    "connector",
    "transformer",
    "crystal",
    "ic",
];
pub const ERC_RULES: &[&str] = &[
    "DriverConflict",
    "PullMismatch",
    "DiffPairShort",
    "DiffPairConflict",
    "WireNearPin",
    "WireOverlap",
    "PinConnectionMismatch",
    "VoltageOutOfRange",
    "ConnectedNoConnect",
    "ReturnMismatch",
    "DisconnectedFulfillment",
    "NetLabelConflict",
    "UnknownSupplyVoltage",
    "UnconnectedPin",
    "UnfulfilledWant",
    "UnplacedGate",
    "TextGeometryOverlap",
    "TextBoundsOverlap",
    "PeripheralConflict",
    "BusMemberShort",
];

pub const HEADERS: &[&str] = &["schematic", "symbol_library"];
pub const ARGS: &[(&str, &[&str])] = &[
    (
        "role",
        &[
            "kind",
            "key",
            "capacitance_nf",
            "voltage_mv",
            "voltage_range_v",
            "connect_to",
            "through",
            "part_path",
            "value",
            "value_range",
            "prefer",
        ],
    ),
    (
        "symbol",
        &[
            "pins",
            "graphics",
            "datasheet",
            "fields",
            "power_net",
            "ref_prefix",
            "peripherals",
            "part_kind",
            "paths",
        ],
    ),
    (
        "pin",
        &[
            "number",
            "kind",
            "at",
            "angle",
            "wants",
            "gives",
            "unit",
            "description",
            "aliases",
            "functions",
            "pads",
        ],
    ),
    ("line", &["from", "to", "unit", "sheet", "width"]),
    ("rect", &["from", "to", "unit", "sheet", "width"]),
    ("arc", &["from", "mid", "to", "unit", "sheet", "width"]),
    ("polyline", &["points", "unit"]),
    ("circle", &["at", "radius", "unit", "sheet", "width"]),
    ("text", &["at", "value", "unit"]),
    ("library", &["file"]),
    ("sheet", &["title", "template"]),
    ("erc_rule", &["rule", "severity"]),
    ("waiver", &["rule", "objects", "reason"]),
    ("annotation", &["occurrence", "reference", "locked"]),
    ("variant", &["omit", "fit"]),
    ("function", &["kind", "wants", "gives"]),
    (
        "place",
        &[
            "sheet",
            "symbol",
            "unit",
            "value",
            "footprint",
            "fields",
            "datasheet",
            "populated",
            "in_bom",
            "ref_offset",
            "value_offset",
            "reference",
            "bindings",
            "functions",
            "wants",
            "gives",
            "at",
            "angle",
            "mirror",
        ],
    ),
    ("gate", &["sheet", "of", "unit", "at", "angle", "mirror"]),
    ("subsheet", &["sheet", "of", "at", "ports"]),
    ("junction", &["sheet", "at"]),
    ("port", &["sheet", "at"]),
    ("wire", &["sheet", "from", "to", "via"]),
    ("label", &["sheet", "on", "value", "global", "at"]),
    ("no_connect", &["sheet", "on"]),
    ("note", &["sheet", "at", "value", "size", "bold"]),
    ("flow", &["sheet", "from", "to"]),
    ("diff_pair", &["sheet", "positive", "negative"]),
    ("bus", &["sheet", "members"]),
    (
        "fulfill",
        &[
            "sheet",
            "want",
            "give",
            "role",
            "return_from",
            "return_to",
            "prefer",
        ],
    ),
];

pub type Point = [f64; 2];

macro_rules! vocabulary {
    ($ty:ident { $($variant:ident => $word:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub enum $ty { $($variant),+ }
        impl $ty {
            pub fn parse(word: &str) -> Option<Self> {
                match word { $($word => Some(Self::$variant),)+ _ => None }
            }
            pub fn word(self) -> &'static str {
                match self { $(Self::$variant => $word),+ }
            }
        }
    };
}
vocabulary!(RoleKind { Power=>"power", Return=>"return", Bypass=>"bypass", Signal=>"signal",
    Clock=>"clock", Bias=>"bias", Termination=>"termination", Feedback=>"feedback",
    Protection=>"protection", Reference=>"reference", Interface=>"interface", PullUp=>"pull_up", PullDown=>"pull_down" });
vocabulary!(Electrical { Passive=>"passive", Input=>"input", Output=>"output", Bidirectional=>"bidirectional",
    Supply=>"power_in", Source=>"power_out", OpenDrain=>"open_drain", Unused=>"no_connect" });

impl RoleKind {
    pub fn is_pull(self) -> bool {
        matches!(self, Self::PullUp | Self::PullDown)
    }
}
/// Nominal identities distinguish two domains with the same electrical function.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoleId(pub String);
#[derive(Clone, Debug, PartialEq)]
pub enum Rating {
    CapacitanceNf(f64),
    VoltageMv(f64),
    VoltageRange(VoltageRange),
}
/// Inclusive pin operating or provided voltage, expressed in volts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoltageRange {
    pub min: f64,
    pub max: f64,
}
impl VoltageRange {
    pub fn accepts(self, offered: Self) -> bool {
        self.min <= offered.min && offered.max <= self.max
    }
}
impl std::fmt::Display for VoltageRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.min == self.max {
            write!(f, "{} V", self.min)
        } else {
            write!(f, "{}–{} V", self.min, self.max)
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Role {
    pub resistance: Option<[PartValue; 2]>,
    pub connection: Option<ConnectionWant>,
    pub kind: RoleKind,
    pub rating: Option<Rating>,
    pub prefer: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct PinFunction {
    pub electrical: Electrical,
    pub wants: Set<RoleId>,
    pub gives: Set<RoleId>,
}
#[derive(Clone, Debug)]
pub struct Pin {
    pub functions: Map<String, PinFunction>,
    pub pads: Vec<String>,
    pub aliases: Set<String>,
    pub description: String,
    pub number: String,
    pub unit: String,
    pub electrical: Electrical,
    pub at: Point,
    pub angle: f64,
    pub wants: Set<RoleId>,
    pub gives: Set<RoleId>,
}
#[derive(Clone, Debug)]
pub struct Graphic {
    pub unit: Option<String>,
    pub shape: Shape,
}
#[derive(Clone, Debug)]
pub enum Shape {
    Line(Point, Point),
    Rect(Point, Point),
    Circle(Point, f64),
    Text(Point, String),
    Polyline(Vec<Point>),
    Arc(Point, Point, Point),
}
#[derive(Clone, Debug)]
pub struct Symbol {
    pub part_kind: Option<String>,
    pub paths: Map<String, [String; 2]>,
    pub ref_prefix: String,
    pub peripherals: Map<String, Map<String, Set<String>>>,
    pub power_net: Option<String>,
    pub fields: Map<String, String>,
    pub datasheet: Option<String>,
    pub pins: Map<String, Pin>,
    pub graphics: Vec<Graphic>,
}
#[derive(Clone, Debug)]
pub enum Item {
    Place {
        reference: Option<String>,
        bindings: Map<String, String>,
        functions: Map<String, String>,
        symbol: String,
        wants: Map<String, Set<RoleId>>,
        gives: Map<String, Set<RoleId>>,
        unit: String,
        value: String,
        footprint: Option<String>,
        fields: Map<String, String>,
        datasheet: Option<String>,
        populated: bool,
        in_bom: bool,
        ref_offset: Option<Point>,
        value_offset: Option<Point>,
        at: Point,
        angle: f64,
        mirror: bool,
    },
    Gate {
        of: String,
        unit: String,
        at: Point,
        angle: f64,
        mirror: bool,
    },
    SheetInstance {
        of: String,
        at: Point,
        anchors: Map<String, Point>,
    },
    Flow {
        from: String,
        to: String,
    },
    Bus(Map<String, String>),
    DiffPair {
        positive: String,
        negative: String,
    },
    Junction(Point),
    Port(Point),
    Wire {
        from: String,
        to: String,
        via: Vec<Point>,
    },
    Label {
        target: String,
        text: String,
        global: bool,
        at: Point,
    },
    NoConnect(String),
    Note {
        at: Point,
        text: String,
        size: f64,
        bold: bool,
    },
    Drawing {
        shape: Shape,
        width: f64,
    },
    Fulfill {
        want: String,
        give: String,
        role: RoleId,
        returns: Option<(String, String)>,
        prefer: Option<f64>,
    },
}
#[derive(Clone, Debug)]
pub struct Row {
    pub sheet: String,
    pub id: String,
    pub item: Item,
    pose_span: Option<Span>,
    pub span: Span,
}
#[derive(Debug)]
pub struct Annotation {
    pub id: String,
    pub reference: String,
    pub locked: bool,
}
#[derive(Default, Debug)]
pub struct Document {
    pub(super) imports: Vec<(std::path::PathBuf, std::path::PathBuf)>,
    /// Qualified import namespace and owning file. Bundled libraries have no disk path.
    pub libraries: Map<String, Option<std::path::PathBuf>>,
    pub annotations: Map<String, Annotation>,
    pub templates: Map<String, String>,
    pub severities: Map<String, String>,
    pub waivers: Map<(String, Vec<String>), String>,
    pub variants: Map<String, Map<String, bool>>,
    pub library: bool,
    pub roles: Map<RoleId, Role>,
    pub role_names: Map<String, RoleId>,
    pub symbols: Map<String, Symbol>,
    pub sheets: Map<String, String>,
    pub rows: Vec<Row>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Fault {
    pub kind: &'static str,
    pub message: String,
    pub span: Span,
    pub file: Option<String>,
}
impl Fault {
    pub(super) fn new(kind: &'static str, span: Span, msg: impl Into<String>) -> Self {
        Self {
            kind,
            message: msg.into(),
            span,
            file: None,
        }
    }
}
pub type Result<T> = std::result::Result<T, Fault>;
pub fn is_document(src: &str) -> bool {
    reader::header(src).is_some_and(|word| HEADERS.contains(&word))
}
pub fn compile(src: &str, dir: Option<&Path>, root: Option<&Path>) -> Result<(Document, Circuit)> {
    let doc = read(src, dir, root)?;
    let circuit = Circuit::build(&doc)?;
    Ok((doc, circuit))
}
/// Keep one revision in an interactive session. A library symlink retarget and a file
/// replaced under its original name must both invalidate the compiled graph.
#[derive(Default)]
pub struct Cache {
    last: Option<Compiled>,
}
struct Compiled {
    file: std::path::PathBuf,
    root: Option<std::path::PathBuf>,
    src: String,
    doc: Document,
    circuit: Circuit,
    deps: crate::deps::Deps,
}
impl Cache {
    pub fn query(
        &mut self,
        file: &Path,
        root: Option<&Path>,
        src: &str,
        words: &[String],
    ) -> Result<forge_agent::json::Json> {
        let current = self.last.as_mut().is_some_and(|last| {
            last.file == file
                && last.root.as_deref() == root
                && last
                    .doc
                    .imports
                    .iter()
                    .all(|(path, was)| forge_dirs::fs::canonicalize(path).ok().as_ref() == Some(was))
                && last.deps.current()
                && (last.src == src || move_cache::update(last, src))
        });
        if !current {
            let watch = crate::deps::watch();
            let (doc, circuit) = compile(src, file.parent(), root)?;
            self.last = Some(Compiled {
                file: file.into(),
                root: root.map(Path::to_owned),
                src: src.into(),
                doc,
                circuit,
                deps: watch.done(),
            });
        }
        let last = self.last.as_ref().expect("compiled revision");
        use forge_agent::json::Json;
        let selection: Vec<_> = words.iter().map(String::as_str).collect();
        let plan = match selection.as_slice() {
            ["plan-block", mode @ ("copy" | "move"), scope, x, y, objects @ ..] => {
                let amount = |v: &str| {
                    v.parse::<f64>().map_err(|_| {
                        Fault::new(
                            "ExpectedLength",
                            Span::default(),
                            "enter the offset in millimetres",
                        )
                    })
                };
                Some(block_plan(
                    src,
                    &last.doc,
                    &last.circuit,
                    scope,
                    &objects.iter().map(|s| s.to_string()).collect(),
                    [amount(x)?, amount(y)?],
                    *mode == "copy",
                )?)
            }
            ["plan-block", "copy-to", scope, destination, x, y, objects @ ..] => {
                let offset = |v: &str| {
                    v.parse::<f64>()
                        .map_err(|_| Fault::new("ExpectedLength", Span::default(), "enter millimetres"))
                };
                Some(block_plan_to(
                    src,
                    &last.doc,
                    &last.circuit,
                    scope,
                    destination,
                    &objects.iter().map(|s| s.to_string()).collect(),
                    [offset(x)?, offset(y)?],
                    true,
                )?)
            }
            ["plan-reannotate"] => Some(BlockPlan {
                ops: project_annotation(src, &last.doc, &last.circuit)?,
                boundary: Set::new(),
            }),
            _ => None,
        };
        let reply = if let Some(plan) = plan {
            Json::obj(vec![
                ("schema", "forgecad.schematic/1".into()),
                ("ok", true.into()),
                (
                    "result",
                    Json::obj(vec![
                        ("ops", Json::Arr(plan.ops.iter().map(|o| o.json()).collect())),
                        (
                            "boundary",
                            Json::Arr(plan.boundary.into_iter().map(Into::into).collect()),
                        ),
                    ]),
                ),
            ])
        } else {
            response(&last.doc, &last.circuit, words)?
        };
        if matches!(
            words.first().map(String::as_str),
            Some("plan-bypass" | "plan-fulfillment" | "plan-reannotate" | "plan-block")
        ) {
            let forge_agent::json::Json::Arr(ops) = reply
                .get("result")
                .and_then(|r| r.get("ops"))
                .expect("planned ops")
            else {
                unreachable!()
            };
            let mut text = src.to_owned();
            for encoded in ops {
                let op = crate::error::Op::from_json(encoded).expect("encoded AddFeature");
                text = crate::edit::apply(&text, &op)
                    .map_err(|e| Fault::new(e.kind(), e.span().unwrap_or_default(), e.text()))?;
            }
            compile(&text, file.parent(), root)?;
        }
        Ok(reply)
    }
}

impl Document {
    pub fn reference<'a>(&'a self, path: &str, default: &'a str) -> &'a str {
        self.annotations
            .get(path)
            .map_or(default, |a| a.reference.as_str())
    }
    pub fn library_paths(&self) -> impl Iterator<Item = &Path> {
        self.imports.iter().map(|(_, path)| path.as_path())
    }
}
