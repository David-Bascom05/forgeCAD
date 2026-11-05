//! Sheet metal, as Inventor has it: the Sheet Metal and Flat Pattern tabs, and a dialog for each
//! command that writes Forge text. A new feature is its params and one statement, added as one
//! op; an edit is the ops that change what the dialog changed (SetParam on the params it reads,
//! SetArg on its arguments, AddLocalParam where a number had none). The host previews the ops
//! live and keeps them as one step of undo on OK.

use super::fg::{FgPanel, ParamRow, parse_value};
use super::fgtool::{Slot, SlotKind};
use forge_lang::error::{Literal, Op, ParamKey};
use forge_lang::syntax::{ExprKind, Seg, StmtKind};
use forge_lang::values::Dim;

/// What the Sheet Metal and Flat Pattern tabs ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SheetCmd {
    /// File > New: a sheet metal part, its defaults written.
    NewPart,
    /// The part into the sheet metal environment, and back out.
    Convert,
    ConvertBack,
    Defaults,
    /// The Style and Standard Editor's sheet metal rules: the part's shops.
    Rules,
    Face,
    Contour,
    Flange,
    Hem,
    Fold,
    Jog,
    Cut,
    CornerSeam,
    CornerRelief,
    Punch,
    Hardware,
    Unfold,
    /// The Flat Pattern environment, and back to the folded part.
    FlatPattern,
    Folded,
    ExportDxf,
    BendTable,
    /// What the flat view draws over the blank.
    ShowExtents,
    ShowBends,
    ShowTangents,
    ShowTools,
}

/// The feature kinds the sheet metal dialogs make, by the call each writes.
pub fn cmd_of_kind(kind: &str) -> Option<SheetCmd> {
    Some(match kind {
        "sheet" => SheetCmd::Face,
        "contour" => SheetCmd::Contour,
        "flange" => SheetCmd::Flange,
        "hem" => SheetCmd::Hem,
        "sketched_bend" => SheetCmd::Fold,
        "jog" => SheetCmd::Jog,
        "cutout" => SheetCmd::Cut,
        "seam" => SheetCmd::CornerSeam,
        "corner_relief" => SheetCmd::CornerRelief,
        "punch" | "form" => SheetCmd::Punch,
        "pem" => SheetCmd::Hardware,
        "unfold" => SheetCmd::Unfold,
        _ => return None,
    })
}

pub fn is_sheet_kind(kind: &str) -> bool {
    cmd_of_kind(kind).is_some()
}

/// A dialog's title, as Inventor names the command; and the base of a new feature's name.
pub fn title(c: SheetCmd) -> (&'static str, &'static str) {
    match c {
        SheetCmd::Face => ("Face", "face"),
        SheetCmd::Contour => ("Contour Flange", "contour"),
        SheetCmd::Flange => ("Flange", "flange"),
        SheetCmd::Hem => ("Hem", "hem"),
        SheetCmd::Fold => ("Fold", "fold"),
        SheetCmd::Jog => ("Jog", "jog"),
        SheetCmd::Cut => ("Cut", "cut"),
        SheetCmd::CornerSeam => ("Corner Seam", "seam"),
        SheetCmd::CornerRelief => ("Corner Relief", "corner"),
        SheetCmd::Punch => ("PunchTool", "punch"),
        SheetCmd::Hardware => ("Insert Hardware", "pem"),
        SheetCmd::Unfold => ("Unfold", "flat"),
        _ => ("Sheet Metal", "sheet"),
    }
}

/// The icon a command and its features show.
pub fn icon(c: SheetCmd) -> &'static str {
    match c {
        SheetCmd::Face => "smface",
        SheetCmd::Contour => "contourflange",
        SheetCmd::Flange => "smflange",
        SheetCmd::Hem => "hem",
        SheetCmd::Fold => "fold",
        SheetCmd::Jog => "jog",
        SheetCmd::Cut => "smcut",
        SheetCmd::CornerSeam => "cornerseam",
        SheetCmd::CornerRelief => "cornerrelief",
        SheetCmd::Punch => "punch",
        SheetCmd::Hardware => "pem",
        SheetCmd::Unfold => "unfold",
        SheetCmd::FlatPattern => "flatpattern",
        SheetCmd::Folded => "folded",
        SheetCmd::ExportDxf => "dxf",
        SheetCmd::BendTable => "bendtable",
        SheetCmd::Defaults => "smdefaults",
        SheetCmd::Rules => "params",
        _ => "smface",
    }
}

/// One of a choice's options: the word written, the word shown, and whether it's built.
pub type Opt = (&'static str, &'static str, bool);

#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    /// A number: blank leaves it out (the default).
    Num {
        text: String,
        dim: Dim,
    },
    Choice {
        opts: &'static [Opt],
        at: usize,
    },
    Flag(bool),
    /// Free words, written as a string.
    Text(String),
}

/// A field of a dialog: the argument it writes, its label, the dialog tab it's on.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// The keyword argument; "" for a number only a composite reads (a relief's width).
    pub arg: &'static str,
    /// A number's param: `param NAME.PARAM = ..` and `ARG: PARAM`.
    pub param: &'static str,
    pub label: &'static str,
    pub tab: usize,
    pub val: Val,
    /// The word or number that's left out: as the call has it by default.
    pub default: &'static str,
    /// Editing: the argument's text as written (None: not written), or the value its param holds.
    pub first: Option<String>,
}

impl Field {
    pub fn num(&self) -> &str {
        match &self.val {
            Val::Num { text, .. } | Val::Text(text) => text,
            _ => "",
        }
    }

    pub fn word(&self) -> &'static str {
        match &self.val {
            Val::Choice { opts, at } => opts[*at].0,
            Val::Flag(true) => "true",
            Val::Flag(false) => "false",
            _ => "",
        }
    }

    fn written_choice(&self) -> Option<String> {
        match &self.val {
            Val::Choice { .. } => {
                let w = self.word();
                (w != self.default).then(|| format!("{w:?}"))
            }
            Val::Flag(on) => (*on && self.default != "true").then(|| "true".into()),
            Val::Text(t) => (!t.trim().is_empty()).then(|| format!("{:?}", t.trim())),
            Val::Num { .. } => None,
        }
    }
}

const SIDE: &[Opt] = &[("up", "Up", true), ("down", "Down (flip)", true)];
const MEASURE: &[Opt] = &[
    ("outer", "Outer face intersection", true),
    ("inner", "Inner face intersection", true),
    ("orthogonal", "Orthogonal (coming)", false),
];
const POSITION: &[Opt] = &[
    ("inside", "Inside of base face extents", true),
    ("outside", "Outside of base face extents", true),
    ("bend_outside", "Bend from adjacent face", true),
    ("tangent", "Bend tangent to side face (coming)", false),
];
const RELIEF: &[Opt] = &[
    ("default", "Default (the rule's)", true),
    ("rect", "Straight", true),
    ("obround", "Round", true),
    ("tear", "Tear", true),
    ("none", "None", true),
];
const HEM: &[Opt] = &[
    ("closed", "Single (closed)", true),
    ("open", "Single (open)", true),
    ("teardrop", "Teardrop", true),
    ("rolled", "Rolled", true),
    ("double", "Double (coming)", false),
];
const HEM_POS: &[Opt] = &[
    ("inside", "Inside the edge", true),
    ("outside", "Off the edge", true),
];
const HAND: &[Opt] = &[
    ("right", "Right of the path", true),
    ("left", "Left (flip)", true),
];
const FOLD_AT: &[Opt] = &[
    ("centre", "Centreline of bend", true),
    ("inside", "Start of bend", true),
    ("outside", "End of bend", true),
    ("bend_outside", "Bend from the line", true),
];
const SEAM: &[Opt] = &[
    ("overlap", "Overlap", true),
    ("underlap", "Reverse overlap", true),
    ("closed", "Symmetric gap", true),
];
const CORNER: &[Opt] = &[
    ("round", "Round", true),
    ("square", "Square", true),
    ("trim", "Trim to bend", true),
    ("tear", "Tear", true),
    ("open", "Open (gap)", true),
    ("constant", "Constant width", true),
    ("closed", "Closed", true),
];
const PLACE: &[Opt] = &[
    ("intersection", "Intersection of bend lines", true),
    ("tangent", "Intersection of tangent lines", true),
];
pub const HW_KINDS: &[Opt] = &[
    ("nut", "Nut", true),
    ("stud", "Stud", true),
    ("standoff", "Standoff", true),
];

/// A turret or forming tool of the punch library: its id, its kind, its sizes as written, and
/// what they are.
#[derive(Clone, Debug, PartialEq)]
pub struct LibTool {
    pub id: &'static str,
    pub kind: &'static str,
    pub sizes: &'static [f64],
    pub form: bool,
    pub about: &'static str,
    /// forge-sheet builds it (louvres and lances are typed, not yet built).
    pub built: bool,
}

/// The standard library: common turret stations and the forming tools forge-sheet builds. A
/// shop's own tools are read from its `tools:` and `forms:` as the part writes them.
pub const LIBRARY: &[LibTool] = &[
    LibTool {
        id: "RD3.5",
        kind: "round",
        sizes: &[3.5],
        form: false,
        about: "Round 3.5 (M3 clearance)",
        built: true,
    },
    LibTool {
        id: "RD4.5",
        kind: "round",
        sizes: &[4.5],
        form: false,
        about: "Round 4.5 (M4 clearance)",
        built: true,
    },
    LibTool {
        id: "RD5.5",
        kind: "round",
        sizes: &[5.5],
        form: false,
        about: "Round 5.5 (M5 clearance)",
        built: true,
    },
    LibTool {
        id: "RD6.6",
        kind: "round",
        sizes: &[6.6],
        form: false,
        about: "Round 6.6 (M6 clearance)",
        built: true,
    },
    LibTool {
        id: "RD10",
        kind: "round",
        sizes: &[10.0],
        form: false,
        about: "Round 10",
        built: true,
    },
    LibTool {
        id: "OB10X5",
        kind: "obround",
        sizes: &[10.0, 5.0],
        form: false,
        about: "Obround 10 x 5",
        built: true,
    },
    LibTool {
        id: "OB20X6",
        kind: "obround",
        sizes: &[20.0, 6.0],
        form: false,
        about: "Obround 20 x 6",
        built: true,
    },
    LibTool {
        id: "SQ10",
        kind: "square",
        sizes: &[10.0],
        form: false,
        about: "Square 10",
        built: true,
    },
    LibTool {
        id: "RE20X10",
        kind: "rect",
        sizes: &[20.0, 10.0],
        form: false,
        about: "Rectangle 20 x 10",
        built: true,
    },
    LibTool {
        id: "DD6.4",
        kind: "d",
        sizes: &[6.4, 5.4],
        form: false,
        about: "Double D (keyed) 6.4",
        built: true,
    },
    LibTool {
        id: "KH8",
        kind: "keyhole",
        sizes: &[8.0, 4.0, 10.0],
        form: false,
        about: "Keyhole 8 / 4 x 10",
        built: true,
    },
    LibTool {
        id: "HX10",
        kind: "hex",
        sizes: &[10.0],
        form: false,
        about: "Hex 10 across flats",
        built: true,
    },
    LibTool {
        id: "VENT4",
        kind: "cluster",
        sizes: &[4.0, 6.0, 6.0, 4.0, 4.0],
        form: false,
        about: "Vent cluster 4 x 4 of 4",
        built: true,
    },
    LibTool {
        id: "DM20",
        kind: "dimple",
        sizes: &[20.0, 4.0, 60.0, 1.0, 2.0],
        form: true,
        about: "Dimple 20, 4 deep",
        built: true,
    },
    LibTool {
        id: "EH8",
        kind: "extruded_hole",
        sizes: &[8.0, 5.0, 1.0, 5.0],
        form: true,
        about: "Extruded hole 8, 5 high",
        built: true,
    },
    LibTool {
        id: "LV30",
        kind: "louvre",
        sizes: &[30.0, 6.0, 3.0],
        form: true,
        about: "Louvre 30 x 6, 3 high",
        built: false,
    },
    LibTool {
        id: "LN20",
        kind: "lance",
        sizes: &[20.0, 6.0, 3.0],
        form: true,
        about: "Lance 20 x 6, 3 high",
        built: false,
    },
];

impl LibTool {
    /// The shared param a part holds it in: its numbers live in a definition, not the call.
    pub fn param(&self) -> String {
        format!("tool_{}", self.id.to_lowercase().replace('.', "_"))
    }

    /// The tool written out, as `punch(..)` and `form(..)` take it.
    pub fn text(&self) -> String {
        let mut s = format!("({:?}, {:?}", self.id, self.kind);
        for (k, v) in self.sizes.iter().enumerate() {
            // a dimple's third size is its wall angle
            if self.kind == "dimple" && k == 2 {
                s.push_str(&format!(", {}deg", forge_lang::values::fmt(*v)));
            } else {
                s.push_str(&format!(", {}", forge_lang::values::fmt(*v)));
            }
        }
        s.push(')');
        s
    }
}

/// A catalogue row for Insert Hardware: kind, size, mounting hole, least edge distance.
#[derive(Clone, Debug, PartialEq)]
pub struct HwRow {
    pub kind: String,
    pub size: String,
    pub hole: f64,
    pub edge: f64,
    pub code: String,
    /// The maker's table it cites, or none: a placeholder row.
    pub source: Option<String>,
}

/// The part's sheet metal defaults: the shared params a Face writes, and the rule it uses.
#[derive(Clone, Debug, PartialEq)]
pub struct Defaults {
    /// A shop param: r and the bend rule come from its stock of thickness t.
    pub rule: Option<String>,
    pub t: f64,
    pub r: f64,
    pub k: f64,
    pub material: String,
}

impl Default for Defaults {
    fn default() -> Defaults {
        // Inventor's Default_mm rule: 0.5 mm stock? Its template's thickness is the user's; a
        // K of 0.44 is its Default_KFactor.
        Defaults {
            rule: None,
            t: 2.0,
            r: 2.0,
            k: 0.44,
            material: String::new(),
        }
    }
}

/// The params named `t`, `r`, `k` at the top: what the defaults are and what a Face reads.
pub const DEFAULT_NAMES: [&str; 3] = ["t", "r", "k"];

fn shared<'a>(params: &'a [ParamRow], name: &str) -> Option<&'a ParamRow> {
    params
        .iter()
        .find(|p| p.key.owner.is_empty() && p.key.name == name)
}

impl Defaults {
    /// As the part's shared params hold them.
    pub fn read(params: &[ParamRow]) -> Defaults {
        let mut d = Defaults::default();
        let v = |n: &str| shared(params, n).and_then(|p| p.value).map(|v| v.0);
        if let Some(x) = v("t") {
            d.t = x;
        }
        if let Some(x) = v("r") {
            d.r = x;
        }
        if let Some(x) = v("k") {
            d.k = x;
        }
        d
    }

    /// The shared lines the part lacks: what a Face reads.
    pub fn header(&self, params: &[ParamRow]) -> String {
        let mut s = String::new();
        let lit = |v, dim| Literal { value: v, dim }.text();
        for (n, v, d) in [
            ("t", self.t, Dim::Length),
            ("r", self.r, Dim::Length),
            ("k", self.k, Dim::Ratio),
        ] {
            if shared(params, n).is_none() {
                s.push_str(&format!("shared {n} = {}\n", lit(v, d)));
            }
        }
        s
    }

    /// The ops that make the part's shared defaults these: SetParam where one is, else a line.
    pub fn ops(&self, params: &[ParamRow]) -> Vec<Op> {
        let mut ops = vec![];
        for (n, v, d) in [
            ("t", self.t, Dim::Length),
            ("r", self.r, Dim::Length),
            ("k", self.k, Dim::Ratio),
        ] {
            if let Some(p) = shared(params, n)
                && p.driving()
                && p.value.is_some_and(|x| x.0 != v)
            {
                ops.push(Op::SetParam {
                    param: p.key.clone(),
                    value: Literal { value: v, dim: d },
                });
            }
        }
        let head = self.header(params);
        if !head.is_empty() {
            ops.push(Op::AddFeature {
                before: None,
                text: head,
            });
        }
        ops
    }
}

/// The shops a part declares (`shared NAME = shop(..)`), by name.
pub fn rules(params: &[ParamRow]) -> Vec<String> {
    params
        .iter()
        .filter(|p| p.key.owner.is_empty() && p.text.trim_start().starts_with("shop("))
        .map(|p| p.key.name.clone())
        .collect()
}

/// A new sheet metal rule, as the Style and Standard Editor writes one: a stock and the shop
/// holding it.
#[derive(Clone, Debug, PartialEq)]
pub struct NewRule {
    pub name: String,
    pub t: String,
    pub k: String,
    pub radius: String,
    pub min_flange: String,
    pub hole_to_bend: String,
    pub relief: usize,
    pub relief_w: String,
    pub relief_d: String,
    pub corner: usize,
    pub kerf: String,
    pub material: String,
}

/// A rule's bend relief shapes and corner reliefs, as `relief_rule` takes them.
pub const RULE_RELIEF: &[Opt] = &[
    ("rect", "Straight", true),
    ("obround", "Round", true),
    ("tear", "Tear", true),
];
pub const RULE_CORNER: &[Opt] = &[
    ("round", "Round", true),
    ("square", "Square", true),
    ("tear", "Tear", true),
    ("trim_to_bend", "Trim to bend", true),
    ("closed", "Closed", true),
];

impl NewRule {
    pub fn new(taken: &[String]) -> NewRule {
        let name = (1..)
            .map(|k| format!("rule{k}"))
            .find(|n| !taken.contains(n))
            .expect("a free name");
        NewRule {
            name,
            t: "2".into(),
            k: "0.44".into(),
            radius: "2".into(),
            min_flange: "4".into(),
            hole_to_bend: "2".into(),
            relief: 0,
            relief_w: "2".into(),
            relief_d: "4".into(),
            corner: 0,
            kerf: "0.2".into(),
            material: String::new(),
        }
    }

    /// Its two shared lines: `NAME_stock = stock(..)` and `NAME = shop([NAME_stock], kerf: ..)`.
    pub fn text(&self) -> Result<String, String> {
        let n = &self.name;
        if !ident(n) {
            return Err(format!("{n:?} is not a name"));
        }
        let num = |label: &str, s: &str, dim| {
            parse_value(s, dim)
                .filter(|l| l.dim == dim)
                .map(|l| l.text())
                .ok_or(format!("{label}: {s:?} is not a number"))
        };
        let t = num("Thickness", &self.t, Dim::Length)?;
        let k = num("K-factor", &self.k, Dim::Ratio)?;
        let r = num("Bend radius", &self.radius, Dim::Length)?;
        let mf = num("Minimum flange", &self.min_flange, Dim::Length)?;
        let hb = num("Hole to bend", &self.hole_to_bend, Dim::Length)?;
        let rw = num("Relief width", &self.relief_w, Dim::Length)?;
        let rd = num("Relief depth", &self.relief_d, Dim::Length)?;
        let kerf = num("Kerf", &self.kerf, Dim::Length)?;
        let relief = RULE_RELIEF[self.relief].0;
        let corner = RULE_CORNER[self.corner].0;
        Ok(format!(
            "shared {n}_stock = stock(t: {t}, bend: kfactor({k}), radii: [{r}], min_flange: per_t(ratio(0), plus: {mf}), hole_to_bend: per_t(ratio(0), plus: {hb}), relief: relief_rule(shape: {relief:?}, width: {rw}, depth: {rd}, corner: {corner:?}))\nshared {n} = shop([{n}_stock], kerf: {kerf})\n"
        ))
    }
}

fn ident(n: &str) -> bool {
    !n.is_empty()
        && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !n.starts_with(|c: char| c.is_ascii_digit())
}

/// A dialog's tabs, as Inventor's sheet metal dialogs have them.
pub const TABS: [&str; 3] = ["Shape", "Unfold Options", "Bend"];

/// A sheet metal command's dialog.
#[derive(Clone, Debug, PartialEq)]
pub struct SheetDlg {
    pub cmd: SheetCmd,
    /// The feature being edited; none for a new one.
    pub edit: Option<String>,
    pub name: String,
    pub tab: usize,
    /// Picks, each with the argument it writes (`#0`: the first positional one).
    pub slots: Vec<Slot>,
    pub args: Vec<&'static str>,
    pub active: usize,
    pub fields: Vec<Field>,
    /// The punch library's tool chosen, or a shop's (by id); a catalogue row, for hardware.
    pub tool: usize,
    pub shop_tool: Option<String>,
    /// A point typed instead of picked: (x, y) on the wall.
    pub typed_at: bool,
    pub error: Option<String>,
    /// The part's params as they were when the dialog opened: what its ops are made against;
    /// and how many errors it had then (more while previewing: the dialog's).
    pub base: Option<Vec<ParamRow>>,
    pub base_errors: usize,
    /// The sheet edges as they were when the dialog opened: its own preview takes some away.
    pub edges: Option<Vec<String>>,
    /// Editing: each argument as the call wrote it, by keyword or `#K`.
    pub firsts: Vec<(String, Option<String>)>,
    /// The ops last sent to preview, so the host previews only what changed.
    pub sent: Option<Vec<Op>>,
    /// OK pressed: the host commits once the preview's taken.
    pub done: bool,
    /// Apply (+): commit this one and start another.
    pub again: bool,
}

fn slot(label: &'static str, kind: SlotKind) -> Slot {
    Slot {
        label,
        kind,
        picks: vec![],
        optional: false,
    }
}

fn num(arg: &'static str, param: &'static str, label: &'static str, tab: usize, v: &str, dim: Dim) -> Field {
    Field {
        arg,
        param,
        label,
        tab,
        val: Val::Num { text: v.into(), dim },
        default: "",
        first: None,
    }
}

fn choice(
    arg: &'static str,
    label: &'static str,
    tab: usize,
    opts: &'static [Opt],
    default: &'static str,
) -> Field {
    Field {
        arg,
        param: "",
        label,
        tab,
        val: Val::Choice {
            opts,
            at: opts.iter().position(|o| o.0 == default).unwrap_or(0),
        },
        default,
        first: None,
    }
}

fn flag(arg: &'static str, label: &'static str, tab: usize) -> Field {
    Field {
        arg,
        param: "",
        label,
        tab,
        val: Val::Flag(false),
        default: "false",
        first: None,
    }
}

/// The ids a dialog may not take for a new feature: the part's statements and their params'
/// owners.
pub fn taken(fg: &FgPanel) -> Vec<String> {
    let mut t = super::fg::statements(&fg.feats);
    t.extend(
        fg.params
            .iter()
            .filter_map(|x| x.key.owner.first().map(|s| s.name.clone())),
    );
    t
}

impl SheetDlg {
    /// A new feature's dialog, named its kind and the first free count.
    pub fn new(cmd: SheetCmd, taken: &[String]) -> Option<SheetDlg> {
        use SlotKind as K;
        let len = Dim::Length;
        let ang = Dim::Angle;
        let (slots, args, fields): (Vec<Slot>, Vec<&'static str>, Vec<Field>) = match cmd {
            SheetCmd::Face => (vec![slot("Profile", K::Profile)], vec!["#0"], vec![]),
            SheetCmd::Contour => (
                vec![slot("Profile", K::Path), slot("Edge", K::SheetEdge)],
                vec!["#0", "edge"],
                vec![
                    num("width", "w", "Distance", 0, "40", len),
                    choice("side", "Material side", 0, HAND, "right"),
                    num("r", "r", "Bend radius", 2, "", len),
                ],
            ),
            SheetCmd::Flange => (
                vec![slot("Edges", K::SheetEdge)],
                vec!["#0"],
                vec![
                    num("height", "h", "Distance", 0, "20", len),
                    Field {
                        default: "90",
                        ..num("angle", "ang", "Flange angle", 0, "90", ang)
                    },
                    choice("side", "Direction", 0, SIDE, "up"),
                    choice("measure", "Height datum", 0, MEASURE, "outer"),
                    choice("position", "Bend position", 0, POSITION, "inside"),
                    num("r", "r", "Bend radius", 2, "", len),
                    Field {
                        param: "relief",
                        ..choice("", "Relief shape", 2, RELIEF, "default")
                    },
                    num("", "rw", "Relief width", 2, "", len),
                    num("", "rd", "Relief depth", 2, "", len),
                ],
            ),
            SheetCmd::Hem => (
                vec![slot("Edge", K::SheetEdge)],
                vec!["#0"],
                vec![
                    choice("kind", "Type", 0, HEM, "closed"),
                    num("length", "leg", "Length", 0, "8", len),
                    num("gap", "gap", "Gap", 0, "", len),
                    num("r", "r", "Radius", 0, "", len),
                    num("angle", "ang", "Angle", 0, "", ang),
                    choice("side", "Direction", 0, SIDE, "up"),
                    choice("position", "Position", 0, HEM_POS, "inside"),
                ],
            ),
            SheetCmd::Fold | SheetCmd::Jog => {
                let mut f = vec![
                    num("", "x0", "From x", 0, "0", len),
                    num("", "y0", "From y", 0, "0", len),
                    num("", "x1", "To x", 0, "0", len),
                    num("", "y1", "To y", 0, "20", len),
                ];
                if cmd == SheetCmd::Jog {
                    f.push(num("offset", "off", "Offset", 0, "10", len));
                }
                f.extend([
                    Field {
                        default: "90",
                        ..num("angle", "ang", "Fold angle", 0, "90", ang)
                    },
                    choice("position", "Fold location", 0, FOLD_AT, "centre"),
                    num("r", "r", "Bend radius", 2, "", len),
                ]);
                (
                    vec![
                        slot("Face", K::Feature),
                        slot("From", K::Points),
                        slot("To", K::Points),
                    ],
                    vec!["wall", "from", "to"],
                    f,
                )
            }
            SheetCmd::Cut => (
                vec![slot("Profile", K::Profiles), slot("Face", K::Feature)],
                vec!["#0", "wall"],
                vec![flag("crosses_bend", "Cut across bend", 0)],
            ),
            SheetCmd::CornerSeam => (
                vec![slot("Flange 1", K::Feature), slot("Flange 2", K::Feature)],
                vec!["#0", "#1"],
                vec![
                    choice("kind", "Seam", 0, SEAM, "overlap"),
                    num("gap", "gap", "Gap", 0, "", len),
                ],
            ),
            SheetCmd::CornerRelief => (
                vec![slot("Flange 1", K::Feature), slot("Flange 2", K::Feature)],
                vec!["#0", "#1"],
                vec![
                    choice("", "Relief shape", 0, CORNER, "round"),
                    num("", "size", "Relief size", 0, "", len),
                    choice("", "Placement", 0, PLACE, "intersection"),
                ],
            ),
            SheetCmd::Punch => (
                vec![slot("Face", K::Feature), slot("Centre", K::Points)],
                vec!["wall", "at"],
                vec![
                    num("", "x", "x", 0, "20", len),
                    num("", "y", "y", 0, "20", len),
                    Field {
                        default: "0",
                        ..num("angle", "ang", "Angle", 0, "0", ang)
                    },
                    flag("up", "Form up", 0),
                ],
            ),
            SheetCmd::Hardware => (
                vec![slot("Face", K::Feature), slot("Centre", K::Points)],
                vec!["wall", "at"],
                vec![
                    choice("#0", "Type", 0, HW_KINDS, "nut"),
                    Field {
                        arg: "#1",
                        param: "",
                        label: "Size",
                        tab: 0,
                        val: Val::Text("M4".into()),
                        default: "",
                        first: None,
                    },
                    num("", "x", "x", 0, "20", len),
                    num("", "y", "y", 0, "20", len),
                ],
            ),
            SheetCmd::Unfold => (
                vec![slot("Sheet", K::Feature)],
                vec!["#0"],
                vec![num("", "gap", "Below the part by", 0, "", len)],
            ),
            _ => return None,
        };
        let base = title(cmd).1;
        let name = (1..)
            .map(|k| format!("{base}{k}"))
            .find(|n| !taken.contains(n) && !taken.contains(&format!("{n}_at")))
            .expect("a free name");
        let mut d = SheetDlg {
            cmd,
            edit: None,
            name,
            tab: 0,
            slots,
            args,
            active: 0,
            fields,
            tool: 0,
            shop_tool: None,
            typed_at: false,
            error: None,
            firsts: vec![],
            edges: None,
            base: None,
            base_errors: 0,
            sent: None,
            done: false,
            again: false,
        };
        // a hem's other numbers, by its type
        d.settle();
        Some(d)
    }

    pub fn field(&self, param_or_label: &str) -> Option<&Field> {
        self.fields.iter().find(|f| {
            f.param == param_or_label
                || f.label == param_or_label
                || (f.param.is_empty() && f.arg == param_or_label)
        })
    }

    pub fn field_mut(&mut self, param_or_label: &str) -> Option<&mut Field> {
        self.fields.iter_mut().find(|f| {
            f.param == param_or_label
                || f.label == param_or_label
                || (f.param.is_empty() && f.arg == param_or_label)
        })
    }

    pub fn set(&mut self, key: &str, text: &str) {
        if let Some(f) = self.field_mut(key)
            && let Val::Num { text: t, .. } | Val::Text(t) = &mut f.val
        {
            *t = text.into();
        }
    }

    pub fn choose(&mut self, key: &str, word: &str) {
        if let Some(f) = self.field_mut(key) {
            match &mut f.val {
                Val::Choice { opts, at } => {
                    if let Some(k) = opts.iter().position(|o| o.0 == word && o.2) {
                        *at = k;
                    }
                }
                Val::Flag(on) => *on = word == "true",
                _ => {}
            }
        }
        self.settle();
    }

    fn word_of(&self, key: &str) -> &'static str {
        self.field(key).map_or("", Field::word)
    }

    /// Whether a field applies, as the others stand (a hem's gap only on an open one).
    pub fn shown(&self, f: &Field) -> bool {
        let key = if f.param.is_empty() { f.arg } else { f.param };
        match (self.cmd, key) {
            (SheetCmd::Hem, "gap") => self.word_of("kind") == "open",
            (SheetCmd::Hem, "r" | "ang") => matches!(self.word_of("kind"), "teardrop" | "rolled"),
            (SheetCmd::Flange, "rw" | "rd") => matches!(self.word_of("relief"), "rect" | "obround"),
            (SheetCmd::Contour, "w") => self.slot("Edge").is_none_or(|s| s.picks.is_empty()),
            (SheetCmd::Punch, "up") => self.lib().is_some_and(|t| t.form),
            (SheetCmd::Punch | SheetCmd::Hardware, "x" | "y") => self.typed_at,
            (SheetCmd::Fold | SheetCmd::Jog, "x0" | "y0") => {
                self.slot("From").is_some_and(|s| s.picks.is_empty())
            }
            (SheetCmd::Fold | SheetCmd::Jog, "x1" | "y1") => {
                self.slot("To").is_some_and(|s| s.picks.is_empty())
            }
            _ => true,
        }
    }

    /// Defaults that follow a choice: a teardrop's radius and angle, a rolled hem's.
    fn settle(&mut self) {
        if self.cmd == SheetCmd::Hem {
            let k = self.word_of("kind");
            let (g, a) = match k {
                "teardrop" => ("", "270"),
                "rolled" => ("", "250"),
                _ => ("", ""),
            };
            for (p, v) in [("r", g), ("ang", a)] {
                if let Some(f) = self.field_mut(p)
                    && let Val::Num { text, .. } = &mut f.val
                    && (text.is_empty() || !matches!(k, "teardrop" | "rolled"))
                {
                    *text = v.into();
                }
            }
            if k == "open"
                && let Some(f) = self.field_mut("gap")
                && let Val::Num { text, .. } = &mut f.val
                && text.is_empty()
            {
                *text = "1".into();
            }
        }
    }

    /// Whether a feature of this name is one the dialog writes (a flange on several edges
    /// writes several).
    pub fn writes(&self, id: &str) -> bool {
        let base = self.name.trim_end_matches(|c: char| c.is_ascii_digit());
        let n: usize = self.name[base.len()..].parse().unwrap_or(1);
        let count = self.slots.first().map_or(1, |s| s.picks.len().max(1));
        id == self.name
            || id == format!("{}_at", self.name)
            || (self.cmd == SheetCmd::Flange
                && id
                    .strip_prefix(base)
                    .and_then(|k| k.parse::<usize>().ok())
                    .is_some_and(|k| k >= n && k < n + count))
    }

    /// The part holds a shared param of this name (as the dialog found it).
    fn has_shared(&self, name: &str) -> bool {
        self.base
            .as_ref()
            .is_some_and(|ps| ps.iter().any(|p| p.key.owner.is_empty() && p.key.name == name))
    }

    pub fn slot(&self, label: &str) -> Option<&Slot> {
        self.slots.iter().find(|s| s.label == label)
    }

    /// The library tool chosen (a shop's own is by id).
    pub fn lib(&self) -> Option<&'static LibTool> {
        (self.cmd == SheetCmd::Punch && self.shop_tool.is_none())
            .then(|| LIBRARY.get(self.tool))
            .flatten()
    }

    /// A pick for the active slot: taken (the same again takes it back out), and the next empty
    /// slot waits.
    pub fn take(&mut self, text: Result<String, String>) {
        let Some(s) = self.slots.get_mut(self.active) else {
            return;
        };
        match text {
            Ok(t) => {
                self.error = None;
                if let Some(k) = s.picks.iter().position(|p| *p == t) {
                    s.picks.remove(k);
                } else if s.kind.many() || (self.cmd == SheetCmd::Flange && self.edit.is_none()) {
                    s.picks.push(t);
                } else {
                    s.picks = vec![t];
                    if let Some(k) = self.slots.iter().position(|x| x.picks.is_empty()) {
                        self.active = k;
                    }
                }
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// What a pick in the view or the tree becomes for the active slot: a wall is the feature
    /// whose face was picked.
    pub fn pick_for_slot(&self, what: &str, text: &str) -> Result<String, String> {
        let Some(s) = self.slots.get(self.active) else {
            return Err("nothing to pick".into());
        };
        match (s.kind, what) {
            (SlotKind::SheetEdge, "sheet edge") => Ok(text.into()),
            (SlotKind::Feature, "face" | "feature") => {
                Ok(text.split(['.', '[', '(']).next().unwrap_or(text).to_string())
            }
            (SlotKind::Profile | SlotKind::Profiles | SlotKind::Path, "sketch") => Ok(text.into()),
            (SlotKind::Points, "point") => Ok(text.into()),
            _ => Err(format!("{} takes no {what}", s.label)),
        }
    }

    fn one(&self, label: &str) -> Result<String, String> {
        self.slot(label)
            .and_then(|s| s.picks.first().cloned())
            .ok_or(format!("{label}: nothing picked yet"))
    }

    /// A number field's literal, or none when blank.
    fn lit(&self, f: &Field) -> Result<Option<Literal>, String> {
        let Val::Num { text, dim } = &f.val else {
            return Ok(None);
        };
        if text.trim().is_empty() {
            return Ok(None);
        }
        let l = parse_value(text, *dim)
            .filter(|l| l.dim == *dim)
            .ok_or(format!("{}: {text:?} is not a number", f.label))?;
        // the call's own default, not written where it wasn't
        let default = parse_value(f.default, *dim).filter(|d| d.value == l.value);
        Ok(if default.is_some() && f.first.is_none() {
            None
        } else {
            Some(l)
        })
    }

    /// The relief's or corner's text, from its shape and sizes (params by name).
    fn composite(&self, have: &dyn Fn(&str) -> bool) -> Option<(&'static str, String)> {
        let size = |p: &str| have(p).then(|| p.to_string());
        match self.cmd {
            SheetCmd::Flange => {
                let sizes = match (size("rw"), size("rd")) {
                    (Some(w), Some(d)) => format!("{w}, {d}"),
                    _ => String::new(),
                };
                Some((
                    "relief",
                    match self.word_of("relief") {
                        "default" => return None,
                        "rect" => format!("rect({sizes})"),
                        "obround" => format!("obround({sizes})"),
                        "tear" => "tear".into(),
                        _ => "no_relief".into(),
                    },
                ))
            }
            SheetCmd::CornerRelief => {
                let s = size("size").unwrap_or_default();
                let place = self.word_of("Placement");
                let at = if place == "intersection" {
                    String::new()
                } else {
                    format!("{}placement: {place:?}", if s.is_empty() { "" } else { ", " })
                };
                Some((
                    "#2",
                    match self.word_of("Relief shape") {
                        "round" => format!("circular_relief({s}{at})"),
                        "square" => format!("square_relief({s}{at})"),
                        "trim" => "trim_to_bend".into(),
                        "tear" => "tear_corner".into(),
                        "open" => format!("open_corner({s})"),
                        "constant" if s.is_empty() => "constant_width_corner()".into(),
                        "constant" => format!("constant_width_corner(width: {s})"),
                        _ => "closed_corner".into(),
                    },
                ))
            }
            _ => None,
        }
    }

    /// The point a punch or a part goes at: picked, or typed (x, y) on the wall.
    fn at_text(&self, name: &str) -> Result<String, String> {
        if self.typed_at || self.slot("Centre").is_some_and(|s| s.picks.is_empty()) {
            if self.cmd == SheetCmd::Hardware {
                // hardware sits on a sketch point: one written on the wall's top for it
                return Ok(format!("{name}_at.p"));
            }
            return Ok("(x, y)".into());
        }
        self.one("Centre")
    }

    /// A new feature's text: its params, then its statement (several for a flange on several
    /// edges).
    pub fn text(&self, defaults: &Defaults, params: &[ParamRow]) -> Result<String, String> {
        if !ident(&self.name) {
            return Err(format!("{:?} is not a feature name", self.name));
        }
        let edges: Vec<String> = match self.cmd {
            SheetCmd::Flange => {
                let s = self.slot("Edges").expect("its slot");
                if s.picks.is_empty() {
                    return Err("Edges: pick an edge of the sheet".into());
                }
                s.picks.clone()
            }
            _ => vec![String::new()],
        };
        let mut out = String::new();
        if matches!(self.cmd, SheetCmd::Face | SheetCmd::Contour) && defaults.rule.is_none() {
            out.push_str(&defaults.header(params));
        }
        for (k, edge) in edges.iter().enumerate() {
            let name = if k == 0 {
                self.name.clone()
            } else {
                // the next free count of the same base
                let base = self.name.trim_end_matches(|c: char| c.is_ascii_digit());
                let n: usize = self.name[base.len()..].parse().unwrap_or(1);
                format!("{base}{}", n + k)
            };
            out.push_str(&self.one_text(&name, edge, defaults)?);
        }
        Ok(out)
    }

    fn one_text(&self, n: &str, edge: &str, d: &Defaults) -> Result<String, String> {
        let mut params = String::new();
        let mut written: Vec<&str> = vec![];
        for f in &self.fields {
            if !self.shown(f) {
                continue;
            }
            if let Some(l) = self.lit(f)? {
                // the punch's x and y are its own only when typed; a hardware point is its sketch's
                let owner = if self.cmd == SheetCmd::Hardware && matches!(f.param, "x" | "y") {
                    format!("{n}_at")
                } else {
                    n.to_string()
                };
                params.push_str(&format!("param {owner}.{} = {}\n", f.param, l.text()));
                written.push(f.param);
            }
        }
        let has = |p: &str| written.contains(&p);
        let mut pos: Vec<String> = vec![];
        let mut kw: Vec<String> = vec![];
        let picks: Vec<(&str, Option<String>)> = self
            .slots
            .iter()
            .zip(&self.args)
            .map(|(s, a)| (*a, s.picks.first().cloned()))
            .collect();
        let pick = |label: &str| self.one(label);
        let func = match self.cmd {
            SheetCmd::Face => {
                pos.push(pick("Profile")?);
                if let Some(rule) = &d.rule {
                    kw.push("t: t".into());
                    kw.push(format!("shop: {rule}"));
                } else {
                    kw.extend(["t: t".into(), "r: r".into(), "bend: kfactor(k)".into()]);
                }
                "sheet"
            }
            SheetCmd::Contour => {
                let s = self.slot("Profile").expect("its slot");
                pos.push(match s.picks.len() {
                    0 => return Err("Profile: pick the path's lines and arcs, in order".into()),
                    1 => s.picks[0].clone(),
                    _ => format!("[{}]", s.picks.join(", ")),
                });
                match &picks[1].1 {
                    Some(e) => kw.push(format!("edge: {e}")),
                    None => {
                        kw.push("width: w".into());
                        kw.push("t: t".into());
                        match &d.rule {
                            Some(rule) => kw.push(format!("shop: {rule}")),
                            None => kw.extend(["r: r".into(), "bend: kfactor(k)".into()]),
                        }
                    }
                }
                "contour"
            }
            SheetCmd::Flange => {
                pos.push(edge.to_string());
                "flange"
            }
            SheetCmd::Hem => {
                pos.push(pick("Edge")?);
                "hem"
            }
            SheetCmd::Fold | SheetCmd::Jog => {
                kw.push(format!("wall: {}", pick("Face")?));
                for (s, x, y) in [("From", "x0", "y0"), ("To", "x1", "y1")] {
                    kw.push(match self.slot(s).and_then(|s| s.picks.first()) {
                        Some(p) => format!("{}: {p}", s.to_lowercase()),
                        None => format!("{}: ({x}, {y})", s.to_lowercase()),
                    });
                }
                if self.cmd == SheetCmd::Jog {
                    "jog"
                } else {
                    "sketched_bend"
                }
            }
            SheetCmd::Cut => {
                let s = self.slot("Profile").expect("its slot");
                if s.picks.is_empty() {
                    return Err("Profile: pick a sketch or a closed loop".into());
                }
                pos.push(s.picks.join(" + "));
                kw.push(format!("wall: {}", pick("Face")?));
                "cutout"
            }
            SheetCmd::CornerSeam => {
                pos.push(pick("Flange 1")?);
                pos.push(pick("Flange 2")?);
                "seam"
            }
            SheetCmd::CornerRelief => {
                pos.push(pick("Flange 1")?);
                pos.push(pick("Flange 2")?);
                pos.push(self.composite(&has).expect("a corner's kind").1);
                "corner_relief"
            }
            SheetCmd::Punch => {
                let (tool, form) = match (&self.shop_tool, self.lib()) {
                    (Some(id), _) => (
                        format!("{id:?}"),
                        self.field("up").is_some_and(|f| f.word() == "true"),
                    ),
                    (None, Some(t)) => (t.param(), t.form),
                    _ => return Err("Pick a tool from the library".into()),
                };
                pos.push(tool);
                kw.push(format!("at: {}", self.at_text(n)?));
                kw.push(format!("wall: {}", pick("Face")?));
                if form { "form" } else { "punch" }
            }
            SheetCmd::Hardware => {
                pos.push(format!("{:?}", self.word_of("#0")));
                let size = self.field("Size").map_or("", Field::num).trim().to_string();
                if size.is_empty() {
                    return Err("Size: choose one from the catalogue".into());
                }
                pos.push(format!("{size:?}"));
                kw.push(format!("at: {}", self.at_text(n)?));
                kw.push(format!("wall: {}", pick("Face")?));
                "pem"
            }
            SheetCmd::Unfold => {
                let s = pick("Sheet")?;
                if has("gap") {
                    kw.push(format!("on: offset({s}.bottom, gap)"));
                }
                pos.push(s);
                "unfold"
            }
            _ => return Err("no dialog".into()),
        };
        for f in &self.fields {
            if !self.shown(f) || f.arg.is_empty() || f.arg.starts_with('#') {
                continue;
            }
            match &f.val {
                Val::Num { .. } if has(f.param) => kw.push(format!("{}: {}", f.arg, f.param)),
                Val::Num { .. } => {}
                _ => {
                    if let Some(w) = f.written_choice() {
                        kw.push(format!("{}: {w}", f.arg));
                    }
                }
            }
        }
        if let Some((arg, text)) = self.composite(&has)
            && !arg.starts_with('#')
        {
            kw.push(format!("{arg}: {text}"));
        }
        let mut pre = String::new();
        if let Some(t) = self.lib().filter(|_| self.cmd == SheetCmd::Punch)
            && !self.has_shared(&t.param())
        {
            pre.push_str(&format!("shared {} = {}\n", t.param(), t.text()));
        }
        if self.cmd == SheetCmd::Hardware && self.at_text(n)?.ends_with("_at.p") {
            pre = format!(
                "sketch {n}_at(on: {}.top) {{\n  p = pt(x, y)\n}}\n",
                pick("Face")?
            );
        }
        pos.extend(kw);
        Ok(format!("{params}{pre}{n} = {func}({})\n", pos.join(", ")))
    }

    /// The ops OK makes: a new feature's AddFeature, or an edit's changes.
    pub fn ops(&self, defaults: &Defaults, params: &[ParamRow]) -> Result<Vec<Op>, String> {
        let Some(id) = &self.edit else {
            return Ok(vec![Op::AddFeature {
                before: None,
                text: self.text(defaults, params)?,
            }]);
        };
        let mut ops = vec![];
        let seg = |s: &str| Seg {
            name: s.into(),
            index: vec![],
        };
        let at_sketch = format!("{id}_at");
        let owner_of = |f: &Field| -> &str {
            if self.cmd == SheetCmd::Hardware && matches!(f.param, "x" | "y") {
                &at_sketch
            } else {
                id
            }
        };
        let own = |owner: &str, name: &str| -> Option<&ParamRow> {
            params
                .iter()
                .find(|p| p.key.name == name && p.key.owner.len() == 1 && p.key.owner[0].name == owner)
                .or_else(|| {
                    params
                        .iter()
                        .find(|p| p.key.owner.is_empty() && p.key.name == name)
                })
        };
        let mut have: Vec<&str> = vec![];
        for f in &self.fields {
            let Some(l) = self.lit(f)? else {
                // a number cleared: left as written
                if f.first.is_some() && !f.param.is_empty() {
                    have.push(f.param);
                }
                continue;
            };
            if !self.shown(f) {
                continue;
            }
            have.push(f.param);
            // the param it reads: the argument's own name, or the feature's own param
            let owner = owner_of(f);
            let read = f
                .first
                .as_deref()
                .filter(|t| ident(t))
                .and_then(|t| own(owner, t))
                .or_else(|| own(owner, f.param).filter(|p| !p.key.owner.is_empty()));
            match read {
                Some(p) if p.driving() => {
                    if p.value.is_none_or(|v| (v.0 - l.value).abs() > 1e-12) {
                        ops.push(Op::SetParam {
                            param: p.key.clone(),
                            value: l,
                        });
                    }
                }
                Some(_) if !f.arg.is_empty() => ops.push(Op::SetArg {
                    id: id.clone(),
                    arg: f.arg.into(),
                    text: l.text(),
                }),
                Some(_) => {}
                None => {
                    ops.push(Op::AddLocalParam {
                        feature: vec![seg(owner)],
                        name: f.param.into(),
                        value: l,
                    });
                    if !f.arg.is_empty() {
                        ops.push(Op::SetArg {
                            id: id.clone(),
                            arg: f.arg.into(),
                            text: f.param.into(),
                        });
                    }
                }
            }
        }
        for f in &self.fields {
            if !matches!(f.val, Val::Num { .. }) && !f.arg.is_empty() && self.shown(f) {
                let now = if f.arg.starts_with('#') {
                    Some(match &f.val {
                        Val::Text(t) => format!("{:?}", t.trim()),
                        _ => format!("{:?}", f.word()),
                    })
                } else {
                    f.written_choice()
                };
                let was = f.first.clone();
                if now != was
                    && !(now.is_none() && was.as_deref().is_none_or(|w| w.trim_matches('"') == f.default))
                {
                    ops.push(Op::SetArg {
                        id: id.clone(),
                        arg: f.arg.into(),
                        text: now.unwrap_or_else(|| format!("{:?}", f.word())),
                    });
                }
            }
        }
        let has = |p: &str| have.contains(&p);
        if let Some((arg, text)) = self.composite(&has) {
            let was = self.firsts.iter().find(|x| x.0 == arg).and_then(|x| x.1.clone());
            if was.as_deref() != Some(text.as_str()) {
                ops.push(Op::SetArg {
                    id: id.clone(),
                    arg: arg.into(),
                    text,
                });
            }
        }
        for (s, arg) in self.slots.iter().zip(&self.args) {
            let was = self.firsts.iter().find(|x| x.0 == *arg).and_then(|x| x.1.clone());
            let now = if s.kind.many() && s.picks.len() > 1 {
                Some(s.picks.join(" + "))
            } else {
                s.picks.first().cloned()
            };
            if let Some(now) = now
                && was.as_deref() != Some(now.as_str())
            {
                if was.is_none() && arg.starts_with('#') {
                    return Err(format!("{}: can't be added to {id}", s.label));
                }
                ops.push(Op::SetArg {
                    id: id.clone(),
                    arg: (*arg).into(),
                    text: now,
                });
            }
        }
        Ok(ops)
    }
}

/// A call's arguments as written: positional ones as `#K`.
pub fn call_args(src: &str, id: &str) -> Option<(String, Vec<(String, String)>)> {
    let prog = forge_lang::syntax::parse(src).ok()?;
    // a feature of a component (`peg.shank`): the statement inside its block
    let (stmts, id) = match id.split_once('.').and_then(|(c, rest)| {
        prog.iter().find_map(|s| match &s.kind {
            StmtKind::Block {
                kind: forge_lang::syntax::BlockKind::Component,
                name,
                body,
                ..
            } if name == c => Some((body.as_slice(), rest)),
            _ => None,
        })
    }) {
        Some(x) => x,
        None => (prog.as_slice(), id),
    };
    stmts.iter().find_map(|s| match &s.kind {
        StmtKind::Assign { name, value } if name == id => match &value.kind {
            ExprKind::Call(f, args) => {
                let text = |sp: forge_lang::syntax::Span| {
                    src.get(sp.lo as usize..sp.hi as usize).unwrap_or("").to_string()
                };
                let mut k = 0;
                let args = args
                    .iter()
                    .map(|a| match &a.name {
                        Some((n, _)) => (n.clone(), text(a.value.span)),
                        None => {
                            k += 1;
                            (format!("#{}", k - 1), text(a.value.span))
                        }
                    })
                    .collect();
                Some((text(f.span), args))
            }
            _ => None,
        },
        _ => None,
    })
}

impl SheetDlg {
    /// The dialog for a feature as the part writes it: its picks, words and numbers (a number
    /// read from a param shows the param's value).
    pub fn edit(id: &str, kind: &str, src: &str, params: &[ParamRow]) -> Option<SheetDlg> {
        let cmd = cmd_of_kind(kind)?;
        let (func, args) = call_args(src, id)?;
        let mut d = SheetDlg::new(cmd, &[])?;
        d.edit = Some(id.into());
        d.name = id.into();
        d.firsts = args.iter().map(|(a, t)| (a.clone(), Some(t.clone()))).collect();
        let arg = |a: &str| args.iter().find(|x| x.0 == a).map(|x| x.1.clone());
        let value = |text: &str| -> Option<f64> {
            if let Some(l) = parse_value(text, Dim::Length).or_else(|| parse_value(text, Dim::Angle)) {
                return Some(l.value);
            }
            params
                .iter()
                .find(|p| p.key.name == text && (p.key.owner.is_empty() || p.key.owner[0].name == id))
                .and_then(|p| p.value)
                .map(|v| v.0)
        };
        let at_sketch = format!("{id}_at");
        let own = |p: &str| {
            let owner = if cmd == SheetCmd::Hardware && matches!(p, "x" | "y") {
                at_sketch.as_str()
            } else {
                id
            };
            params
                .iter()
                .find(|x| x.key.name == p && x.key.owner.len() == 1 && x.key.owner[0].name == owner)
                .and_then(|x| x.value)
                .map(|v| v.0)
        };
        for s in 0..d.slots.len() {
            if let Some(t) = arg(d.args[s]) {
                d.slots[s].picks = if d.slots[s].kind.many() {
                    t.split(" + ").map(str::to_string).collect()
                } else {
                    vec![t]
                };
            }
        }
        let fmt = forge_lang::values::fmt;
        for f in &mut d.fields {
            let written = if f.arg.is_empty() || f.arg.starts_with('#') && !matches!(f.val, Val::Num { .. }) {
                if f.arg.starts_with('#') { arg(f.arg) } else { None }
            } else {
                arg(f.arg)
            };
            f.first = written.clone();
            match &mut f.val {
                Val::Num { text, .. } => {
                    let v = written.as_deref().and_then(value).or_else(|| own(f.param));
                    // not written: the call's default, as the dialog would show a new one
                    *text = v.map_or_else(|| f.default.to_string(), fmt);
                    if f.first.is_none() && v.is_some() {
                        f.first = Some(f.param.into());
                    }
                }
                Val::Choice { opts, at } => {
                    if let Some(w) = &written
                        && let Some(k) = opts.iter().position(|o| o.0 == w.trim_matches('"'))
                    {
                        *at = k;
                    }
                }
                Val::Flag(on) => *on = written.as_deref() == Some("true"),
                Val::Text(t) => {
                    if let Some(w) = &written {
                        *t = w.trim_matches('"').into();
                    }
                }
            }
        }
        // what a composite reads back: a relief's or a corner's shape
        match cmd {
            SheetCmd::Flange => {
                let r = arg("relief").unwrap_or_default();
                let w = if r.starts_with("rect") {
                    "rect"
                } else if r.starts_with("obround") {
                    "obround"
                } else if r == "tear" {
                    "tear"
                } else if r == "no_relief" {
                    "none"
                } else {
                    "default"
                };
                d.choose("relief", w);
            }
            SheetCmd::CornerRelief => {
                let r = arg("#2").or_else(|| arg("kind")).unwrap_or_default();
                let w = [
                    ("circular", "round"),
                    ("square", "square"),
                    ("trim", "trim"),
                    ("tear", "tear"),
                    ("open", "open"),
                    ("constant", "constant"),
                ]
                .iter()
                .find(|x| r.starts_with(x.0))
                .map_or("closed", |x| x.1);
                d.choose("Relief shape", w);
                if r.contains("tangent") {
                    d.choose("Placement", "tangent");
                }
            }
            SheetCmd::Punch => {
                let t = arg("#0").unwrap_or_default();
                match LIBRARY.iter().position(|l| l.text() == t || l.param() == t) {
                    Some(k) => d.tool = k,
                    None => d.shop_tool = Some(t.trim_matches('"').to_string()),
                }
                if func == "form" && d.shop_tool.is_some() {
                    d.choose(
                        "up",
                        if arg("up").as_deref() == Some("true") {
                            "true"
                        } else {
                            "false"
                        },
                    );
                }
                d.typed_at = arg("at").is_some_and(|a| a.starts_with('('));
                if d.typed_at {
                    d.slots[1].picks.clear();
                }
            }
            SheetCmd::Hardware => {
                d.typed_at = arg("at").is_some_and(|a| a.ends_with("_at.p"));
            }
            _ => {}
        }
        d.settle();
        Some(d)
    }
}

/// What the flat view draws over the blank, from the sheet's flat pattern: in the first wall's
/// own coordinates (the frame the flat lies in).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlatView {
    pub sheet: String,
    pub frame: Option<forge_math::Frame>,
    /// The cut contours as polylines; the first is the blank's outline.
    pub outline: Vec<Vec<[f64; 2]>>,
    pub bends: Vec<FlatBend>,
    /// Turret hits and forms: centre, tool id.
    pub tools: Vec<([f64; 2], String)>,
    pub lo: [f64; 2],
    pub hi: [f64; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlatBend {
    pub a: [f64; 2],
    pub b: [f64; 2],
    /// Degrees, signed: up towards the viewer.
    pub angle: f64,
    pub radius: f64,
    pub allowance: f64,
    /// The feature that made it.
    pub feature: String,
}

/// The DXF export's choices, as Inventor's Flat Pattern DXF Export Options has them: each kind of
/// line on its layer, or left out.
#[derive(Clone, Debug, PartialEq)]
pub struct DxfChoice {
    pub sheet: String,
    /// (what, layer name, on).
    pub layers: Vec<(&'static str, String, bool)>,
    pub path: String,
}

impl DxfChoice {
    pub fn new(sheet: &str, path: String) -> DxfChoice {
        let l = |w, n: &str, on| (w, n.to_string(), on);
        DxfChoice {
            sheet: sheet.into(),
            layers: vec![
                l("Outer profile", "IV_OUTER_PROFILE", true),
                l("Inner profiles", "IV_INTERIOR_PROFILES", true),
                l("Bend centre lines, up", "IV_BEND", true),
                l("Bend centre lines, down", "IV_BEND_DOWN", true),
                l("Tangent lines", "IV_TANGENT", false),
                l("Tool centres (punches)", "IV_TOOL_CENTER", true),
                l("Forming tools", "IV_TOOL_CENTER_DOWN", true),
                l("Bend notes and part note", "IV_FEATURE_PROFILES", true),
            ],
            path,
        }
    }

    /// Its eight layers in forge-sheet's order: outer, inner, bend up, bend down, tangent,
    /// punch, form, notes.
    pub fn all(&self) -> [Option<String>; 8] {
        std::array::from_fn(|k| self.layer(k))
    }

    pub fn layer(&self, k: usize) -> Option<String> {
        self.layers
            .get(k)
            .filter(|l| l.2 && !l.1.trim().is_empty())
            .map(|l| l.1.trim().to_string())
    }
}

/// The sheet metal environment's state in the shell.
#[derive(Default)]
pub struct SheetUi {
    /// The Sheet Metal tab is shown (a sheet metal part, or one converted).
    pub env: bool,
    /// The Flat Pattern environment.
    pub flat_env: bool,
    /// Converted back to a part: not put back in by itself.
    pub left: bool,
    pub dlg: Option<SheetDlg>,
    pub defaults: Defaults,
    pub defaults_open: Option<Defaults>,
    pub rules_open: Option<NewRule>,
    pub dxf_open: Option<DxfChoice>,
    pub bend_table: bool,
    /// The flat view's overlays.
    pub extents: bool,
    pub bend_lines: bool,
    pub tangents: bool,
    pub tool_marks: bool,
    pub flats: Vec<FlatView>,
    /// The sheet edges the part names, for Edge picks.
    pub edges: Vec<String>,
    pub catalogue: Vec<HwRow>,
    /// The tree asked to edit this feature.
    pub edit_req: Option<String>,
    pub out: Vec<SheetAct>,
}

/// What the sheet metal chrome asks the host for.
#[derive(Clone, Debug, PartialEq)]
pub enum SheetAct {
    /// Preview the dialog's ops (none: back to before the dialog).
    Preview(Vec<Op>),
    /// Keep the preview as one step of undo.
    Commit,
    /// Drop it.
    Cancel,
    /// Ops applied at once, as one step (Defaults, a new rule).
    Apply(Vec<Op>),
    Flat(bool),
    WriteDxf(DxfChoice),
}

impl SheetUi {
    pub fn new() -> SheetUi {
        SheetUi {
            bend_lines: true,
            tool_marks: true,
            extents: true,
            ..SheetUi::default()
        }
    }

    /// A command from the ribbon.
    pub fn command(&mut self, c: SheetCmd, fg: &FgPanel) -> Result<(), String> {
        match c {
            SheetCmd::Convert => {
                self.env = true;
                self.left = false;
                self.defaults = Defaults::read(&fg.params);
            }
            SheetCmd::ConvertBack => {
                if !fg.sheets.is_empty() {
                    return Err("Convert to Standard Part: the part's sheet metal features stay sheet metal; delete them first".into());
                }
                self.env = false;
                self.flat_env = false;
                self.left = true;
            }
            SheetCmd::Defaults => {
                self.defaults_open = Some(self.defaults.clone());
            }
            SheetCmd::Rules => {
                let mut t = taken(fg);
                t.extend(fg.params.iter().map(|p| p.key.name.clone()));
                self.rules_open = Some(NewRule::new(&t));
            }
            SheetCmd::FlatPattern => {
                if fg.sheets.is_empty() {
                    return Err("Flat Pattern: make a Face first".into());
                }
                if let Some(why) = fg.sheets.iter().find_map(|s| s.refused.clone()) {
                    return Err(format!("no flat pattern: {why}"));
                }
                self.flat_env = true;
                self.dlg = None;
                self.out.push(SheetAct::Flat(true));
            }
            SheetCmd::Folded => {
                self.flat_env = false;
                self.out.push(SheetAct::Flat(false));
            }
            SheetCmd::ExportDxf => {
                let s = fg.sheets.first().ok_or("no sheet to export")?;
                let stem = fg.file.trim_end_matches(".fg");
                let path = if fg.sheets.len() > 1 {
                    format!("{stem}-{}.dxf", s.id)
                } else {
                    format!("{stem}.dxf")
                };
                self.dxf_open = Some(DxfChoice::new(&s.id, path));
            }
            SheetCmd::BendTable => self.bend_table = !self.bend_table,
            SheetCmd::ShowExtents => self.extents = !self.extents,
            SheetCmd::ShowBends => self.bend_lines = !self.bend_lines,
            SheetCmd::ShowTangents => self.tangents = !self.tangents,
            SheetCmd::ShowTools => self.tool_marks = !self.tool_marks,
            SheetCmd::NewPart => {}
            c => {
                if self.dlg.is_some() {
                    self.out.push(SheetAct::Cancel);
                }
                let needs_sheet = !matches!(c, SheetCmd::Face | SheetCmd::Contour);
                if needs_sheet && fg.sheets.is_empty() {
                    return Err(format!("{}: make a Face or a Contour Flange first", title(c).0));
                }
                self.env = true;
                self.dlg = SheetDlg::new(c, &taken(fg));
                if let Some(d) = &mut self.dlg
                    && c == SheetCmd::Hardware
                    && let Some(row) = self.catalogue.first()
                {
                    d.choose("#0", &row.kind.clone());
                    d.set("Size", &row.size.clone());
                }
            }
        }
        Ok(())
    }

    /// Opens a sheet feature's dialog to edit it.
    pub fn edit(&mut self, id: &str, fg: &FgPanel) -> Result<(), String> {
        let kind = fg
            .feats
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.kind.clone())
            .ok_or(format!("no feature {id}"))?;
        if self.dlg.is_some() {
            return Err("Finish or cancel the dialog that's open first".into());
        }
        self.dlg = Some(
            SheetDlg::edit(id, &kind, &fg.source, &fg.params)
                .ok_or(format!("{id} is not a sheet metal feature written as one call"))?,
        );
        self.env = true;
        Ok(())
    }

    /// The dialog's ops for the host to preview, if they changed since the last.
    /// `settled`: the part as built is the text before any preview, so its params are the ones
    /// the dialog's ops are made against (a preview's own would undo them).
    pub fn pump(&mut self, params: &[ParamRow], errors: usize, settled: bool) {
        let defaults = self.defaults.clone();
        let Some(d) = &mut self.dlg else { return };
        if d.base.is_none() {
            if !settled {
                return;
            }
            d.base = Some(params.to_vec());
            d.base_errors = errors;
        }
        let ops = d.ops(&defaults, d.base.as_deref().expect("set"));
        match ops {
            Ok(ops) => {
                if d.sent.as_ref() != Some(&ops) {
                    d.sent = Some(ops.clone());
                    self.out.push(SheetAct::Preview(ops));
                }
            }
            Err(e) => {
                if d.sent.as_ref().is_some_and(|s| !s.is_empty()) {
                    d.sent = Some(vec![]);
                    self.out.push(SheetAct::Preview(vec![]));
                }
                // a pick still to make is said where it's asked, not as an error
                if !e.contains("nothing picked") && !e.contains("pick ") {
                    d.error = Some(e);
                }
            }
        }
    }
}

/// The part's ParamKey for a feature's own param.
pub fn own_key(id: &str, name: &str) -> ParamKey {
    ParamKey {
        owner: vec![Seg {
            name: id.into(),
            index: vec![],
        }],
        name: name.into(),
    }
}

/// A sheet metal part's first line: it opens in the sheet metal environment.
pub const PART_MARK: &str = "# a sheet metal part";

/// A new sheet metal part's text: its defaults; Face makes its first wall.
pub fn new_part_text(d: &Defaults) -> String {
    format!(
        "{PART_MARK}: Sheet Metal Defaults below; Face makes its first wall\n{}",
        d.header(&[])
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const L: &str = "shared t = 2
shared r = 2
shared k = ratio(0.44)
param plan.L = 60
param plan.W = 40
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = sheet(plan, t: t, r: r, bend: kfactor(k))
param up.h = 20
up = flange(base.edge(plan.outline[1]), height: h)
";

    /// The params as the panel holds them, from a run.
    fn params(src: &str) -> Vec<ParamRow> {
        let run = forge_lang::run(src, forge_lang::Options::default(), Default::default()).0;
        run.shared
            .iter()
            .chain(&run.locals)
            .map(|p| ParamRow {
                key: p.key.clone(),
                value: match &p.value {
                    forge_lang::eval::Value::Num(n) => Some((
                        if n.dim == Dim::Angle {
                            n.v.to_degrees()
                        } else {
                            n.v
                        },
                        n.dim,
                    )),
                    _ => None,
                },
                text: src[p.rhs.lo as usize..p.rhs.hi as usize].to_string(),
                links: p.links.iter().cloned().collect(),
                used_by: vec![],
                structural: false,
                line: 0,
            })
            .collect()
    }

    /// The ops applied, checked, and the part built clean.
    fn apply(src: &str, ops: &[Op]) -> String {
        let mut text = src.to_string();
        for op in ops {
            text = forge_lang::edit::apply_checked(&text, op)
                .unwrap_or_else(|e| panic!("{op:?}: {}\n{text}", e.text()))
                .0;
        }
        let run = forge_lang::run(&text, forge_lang::Options::default(), Default::default()).0;
        assert!(run.ok(), "{:?}\n{text}", run.errors);
        text
    }

    fn new(cmd: SheetCmd, src: &str) -> SheetDlg {
        let run = forge_lang::run(src, forge_lang::Options::default(), Default::default()).0;
        let taken: Vec<String> = run.feats.iter().map(|f| f.id.clone()).collect();
        SheetDlg::new(cmd, &taken).unwrap()
    }

    fn ops(d: &SheetDlg, src: &str) -> Vec<Op> {
        d.ops(&Defaults::default(), &params(src)).unwrap()
    }

    #[test]
    fn face_writes_the_defaults_it_lacks_and_its_sheet() {
        let src = "param plan.L = 60\nparam plan.W = 40\nsketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (L, W))\n}\n";
        let mut d = new(SheetCmd::Face, src);
        assert!(d.ops(&Defaults::default(), &[]).is_err());
        d.take(Ok("plan".into()));
        let text = apply(src, &ops(&d, src));
        assert!(
            text.contains("shared t = 2\nshared r = 2\nshared k = ratio(0.44)\n"),
            "{text}"
        );
        assert!(
            text.contains("face1 = sheet(plan, t: t, r: r, bend: kfactor(k))"),
            "{text}"
        );
        // with a rule: thickness and the shop, radius and K from its stock
        let rule = Defaults {
            rule: Some("acme".into()),
            ..Defaults::default()
        };
        assert!(
            d.text(&rule, &[])
                .unwrap()
                .contains("face1 = sheet(plan, t: t, shop: acme)")
        );
    }

    #[test]
    fn flange_writes_its_params_and_only_what_differs_from_the_default() {
        let mut d = new(SheetCmd::Flange, L);
        assert_eq!(d.name, "flange1");
        d.take(Ok("base.edge(plan.outline[3])".into()));
        d.set("h", "15");
        d.set("ang", "60");
        d.choose("measure", "inner");
        d.choose("position", "outside");
        // the tangent position isn't built: not taken
        d.choose("position", "tangent");
        d.choose("relief", "obround");
        d.set("rw", "2");
        d.set("rd", "5");
        let text = apply(L, &ops(&d, L));
        assert!(text.contains("param flange1.h = 15\nparam flange1.ang = 60deg\nparam flange1.rw = 2\nparam flange1.rd = 5\n"), "{text}");
        assert!(
            text.contains("flange1 = flange(base.edge(plan.outline[3]), height: h, angle: ang, measure: \"inner\", position: \"outside\", relief: obround(rw, rd))"),
            "{text}"
        );
        // several edges: a flange each
        let mut d = new(SheetCmd::Flange, L);
        d.take(Ok("base.edge(plan.outline[0])".into()));
        d.take(Ok("base.edge(plan.outline[2])".into()));
        let text = apply(L, &ops(&d, L));
        assert!(
            text.contains("flange1 = flange(base.edge(plan.outline[0]), height: h)"),
            "{text}"
        );
        assert!(
            text.contains("flange2 = flange(base.edge(plan.outline[2]), height: h)"),
            "{text}"
        );
    }

    #[test]
    fn editing_a_flange_sets_its_param_and_adds_what_it_lacked() {
        let ps = params(L);
        let mut d = SheetDlg::edit("up", "flange", L, &ps).unwrap();
        assert_eq!(d.field("h").unwrap().num(), "20");
        assert_eq!(d.slots[0].picks, ["base.edge(plan.outline[1])"]);
        // nothing changed: nothing to do
        assert_eq!(d.ops(&Defaults::default(), &ps).unwrap(), []);
        d.set("h", "35");
        d.set("ang", "120");
        d.choose("side", "down");
        let o = d.ops(&Defaults::default(), &ps).unwrap();
        assert_eq!(
            o[0],
            Op::SetParam {
                param: own_key("up", "h"),
                value: Literal {
                    value: 35.0,
                    dim: Dim::Length
                }
            }
        );
        let text = apply(L, &o);
        assert!(text.contains("param up.h = 35"), "{text}");
        assert!(text.contains("param up.ang = 120deg"), "{text}");
        assert!(
            text.contains("up = flange(base.edge(plan.outline[1]), height: h, angle: ang, side: \"down\")"),
            "{text}"
        );
        // the edge picked again: its positional argument changes
        let mut d = SheetDlg::edit("up", "flange", L, &ps).unwrap();
        d.slots[0].picks = vec!["base.edge(plan.outline[2])".into()];
        let text = apply(L, &d.ops(&Defaults::default(), &ps).unwrap());
        assert!(
            text.contains("up = flange(base.edge(plan.outline[2]), height: h)"),
            "{text}"
        );
    }

    #[test]
    fn hem_fold_jog_cut_seam_corner_and_unfold_write_what_builds() {
        let box_ = format!(
            "{}param left.h = 20\nleft = flange(base.edge(plan.outline[0]), height: h, position: \"outside\")\nparam hs.x = 30\nparam hs.y = 20\nparam hs.d = 6\nsketch hs(on: base.top) {{\n  c = circle((x, y), d)\n  p = pt(x, y)\n}}\n",
            L.replace("height: h)", "height: h, position: \"outside\")")
        );
        let src = box_.as_str();
        let mut d = new(SheetCmd::Hem, L);
        d.take(Ok("up.tip".into()));
        d.choose("kind", "open");
        assert_eq!(d.field("gap").unwrap().num(), "1");
        let text = apply(L, &ops(&d, L));
        assert!(
            text.contains("hem1 = hem(up.tip, kind: \"open\", length: leg, gap: gap)"),
            "{text}"
        );

        let mut d = new(SheetCmd::Cut, src);
        d.take(Ok("hs.c".into()));
        d.active = 1;
        d.take(Ok("base".into()));
        let text = apply(src, &ops(&d, src));
        assert!(text.contains("cut1 = cutout(hs.c, wall: base)"), "{text}");

        let mut d = new(SheetCmd::CornerSeam, src);
        d.take(Ok("up".into()));
        d.take(Ok("left".into()));
        d.choose("kind", "closed");
        d.set("gap", "0.5");
        let text = apply(src, &ops(&d, src));
        assert!(
            text.contains("seam1 = seam(up, left, kind: \"closed\", gap: gap)"),
            "{text}"
        );

        let mut d = new(SheetCmd::CornerRelief, src);
        d.take(Ok("up".into()));
        d.take(Ok("left".into()));
        d.choose("Relief shape", "square");
        d.set("size", "3");
        let text = apply(src, &ops(&d, src));
        assert!(
            text.contains("corner1 = corner_relief(up, left, square_relief(size))"),
            "{text}"
        );

        let mut d = new(SheetCmd::Fold, L);
        d.take(Ok("base".into()));
        for (k, v) in [("x0", "30"), ("y0", "-5"), ("x1", "30"), ("y1", "45")] {
            d.set(k, v);
        }
        let text = apply(L, &ops(&d, L));
        assert!(
            text.contains("fold1 = sketched_bend(wall: base, from: (x0, y0), to: (x1, y1))"),
            "{text}"
        );

        let mut d = new(SheetCmd::Jog, L);
        d.take(Ok("base".into()));
        for (k, v) in [("x0", "30"), ("y0", "-5"), ("x1", "30"), ("y1", "45")] {
            d.set(k, v);
        }
        d.choose("position", "outside");
        let text = apply(L, &ops(&d, L));
        assert!(
            text.contains(
                "jog1 = jog(wall: base, from: (x0, y0), to: (x1, y1), offset: off, position: \"outside\")"
            ),
            "{text}"
        );

        let mut d = new(SheetCmd::Unfold, L);
        d.take(Ok("base".into()));
        d.set("gap", "50");
        let text = apply(L, &ops(&d, L));
        assert!(
            text.contains("flat1 = unfold(base, on: offset(base.bottom, gap))"),
            "{text}"
        );
    }

    #[test]
    fn punch_form_and_hardware_place_from_the_library_and_the_catalogue() {
        let src =
            format!("{L}param hs.x = 30\nparam hs.y = 20\nsketch hs(on: base.top) {{\n  p = pt(x, y)\n}}\n");
        let src = src.as_str();
        let mut d = new(SheetCmd::Punch, src);
        d.take(Ok("base".into()));
        d.take(Ok("hs.p".into()));
        d.tool = LIBRARY.iter().position(|t| t.id == "OB10X5").unwrap();
        d.set("ang", "45");
        let text = apply(src, &ops(&d, src));
        assert!(text.contains("shared tool_ob10x5 = (\"OB10X5\", \"obround\", 10, 5)\npunch1 = punch(tool_ob10x5, at: hs.p, wall: base, angle: ang)"), "{text}");
        // a form, at a point typed on the face
        let mut d = new(SheetCmd::Punch, src);
        d.take(Ok("base".into()));
        d.tool = LIBRARY.iter().position(|t| t.id == "DM20").unwrap();
        d.typed_at = true;
        d.set("ang", "0");
        let text = apply(src, &ops(&d, src));
        assert!(
            text.contains("param punch1.x = 20\nparam punch1.y = 20\n"),
            "{text}"
        );
        assert!(text.contains("shared tool_dm20 = (\"DM20\", \"dimple\", 20, 4, 60deg, 1, 2)\npunch1 = form(tool_dm20, at: (x, y), wall: base"), "{text}");
        // every library tool writes a tool forge-lang takes
        for (k, t) in LIBRARY.iter().enumerate() {
            let mut d = new(SheetCmd::Punch, src);
            d.take(Ok("base".into()));
            d.take(Ok("hs.p".into()));
            d.tool = k;
            let o = ops(&d, src);
            let text = forge_lang::edit::apply(src, &o[0]).unwrap();
            let run = forge_lang::run(&text, forge_lang::Options::default(), Default::default()).0;
            assert!(
                !run.errors
                    .iter()
                    .any(|e| matches!(e.kind(), "BadTool" | "WrongType" | "TypeMismatch")),
                "{}: {:?}",
                t.id,
                run.errors
            );
        }
        // hardware on a point typed: a sketch for it on the wall's top
        let mut d = new(SheetCmd::Hardware, src);
        d.take(Ok("base".into()));
        d.typed_at = true;
        d.choose("#0", "nut");
        d.set("Size", "M4");
        let text = apply(src, &ops(&d, src));
        assert!(text.contains("param pem1_at.x = 20\nparam pem1_at.y = 20\nsketch pem1_at(on: base.top) {\n  p = pt(x, y)\n}\npem1 = pem(\"nut\", \"M4\", at: pem1_at.p, wall: base)"), "{text}");
        // and edited: its point moves by its sketch's params
        let ps = params(&text);
        let mut e = SheetDlg::edit("pem1", "pem", &text, &ps).unwrap();
        assert!(e.typed_at);
        e.set("x", "25");
        let o = e.ops(&Defaults::default(), &ps).unwrap();
        assert_eq!(
            o,
            [Op::SetParam {
                param: own_key("pem1_at", "x"),
                value: Literal {
                    value: 25.0,
                    dim: Dim::Length
                }
            }]
        );
    }

    #[test]
    fn defaults_set_what_the_part_has_and_add_what_it_lacks() {
        let ps = params(L);
        let d = Defaults {
            t: 1.5,
            ..Defaults::read(&ps)
        };
        assert_eq!(Defaults::read(&ps).k, 0.44);
        let o = d.ops(&ps);
        assert_eq!(
            o,
            [Op::SetParam {
                param: ParamKey {
                    owner: vec![],
                    name: "t".into()
                },
                value: Literal {
                    value: 1.5,
                    dim: Dim::Length
                }
            }]
        );
        apply(L, &o);
        assert!(new_part_text(&Defaults::default()).starts_with(PART_MARK));
        let rule = NewRule::new(&[]);
        let text = format!(
            "{}{}",
            rule.text().unwrap(),
            L.replace("bend: kfactor(k))", "bend: kfactor(k), shop: rule1)")
        );
        apply(&text, &[]);
    }
}
