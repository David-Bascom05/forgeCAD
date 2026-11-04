//! Sheet metal in Forge, on forge-sheet's wall-and-joint graph. `sheet(..)` makes a part's base
//! wall from a profile, `flange(..)` turns a new wall up off a straight edge, `cutout(..)` takes a
//! profile out of the sheet (across bends too), `unfold(..)` adds the flat body. Each is a feature
//! with its own id; the graph is rebuilt whole by the kernel after each (tens of microseconds), and
//! the part's sheet body replaced. The shop (stock, bend rules, tooled radii, minimum lengths) is
//! data: `stock(..)` and `shop(..)` values a param may hold, checked against at the end of a run.
//! Ordinary features on a sheet part are refused, both ways round; reliefs wait on a decision and
//! are refused when asked for. Every kernel error, warning and violation comes through typed, with
//! typed fixes where a param would cure it.
use crate::build::{Builder, history_id, role_faces};
use crate::builtins::{Args, LEN, num_of, profile_of, word};
use crate::error::{Error, Literal, Op, ParamKey, Role, Warning};
use crate::eval::{Dimension, Eval, Faces, R, Status, Value};
use crate::sketch::Sk;
use crate::syntax::{Arg, Span};
use crate::values::{Dim, Num, Src, Ty};
use forge_agent::json::Json;
use forge_geom::spline::PlaneSpline;
use forge_math::{Frame, Point2, Point3, Vec3, dmath};
use forge_model::{Feature, Solid};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{self as fs, CutId, JointId, Map, SheetError, Skin, Strip, TrimRef, WallId};
use forge_topo::{Body, Name};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;
use std::time::Instant;

pub const BUILTINS: &[&str] = &[
    "sheet",
    "flange",
    "cutout",
    "unfold",
    "contour",
    "hem",
    "kfactor",
    "ktable",
    "bend_table",
    "per_t",
    "stock",
    "shop",
    "obround",
    "circular_relief",
    "square_relief",
    "open_corner",
    "overlap_corner",
    "corner_relief",
    "constant_width_corner",
    "seam",
    "punch",
    "form",
    "sketched_bend",
    "jog",
    "hardware",
    "pem",
    "relief_rule",
];

/// Sheet constants: the relief kinds and corner treatments that take no size.
pub fn constant(name: &str) -> Option<Value> {
    Some(match name {
        "tear" => sv(SheetVal::Relief(Rc::new(ReliefV {
            kind: Some(fs::ReliefKind::Tear),
            width: None,
            depth: None,
        }))),
        "no_relief" => sv(SheetVal::Relief(Rc::new(ReliefV {
            kind: None,
            width: None,
            depth: None,
        }))),
        "closed_corner" => sv(SheetVal::Corner(Rc::new(CornerV {
            kind: fs::CornerKind::Closed,
        }))),
        "tear_corner" => sv(SheetVal::Corner(Rc::new(CornerV {
            kind: fs::CornerKind::Tear,
        }))),
        "trim_to_bend" => sv(SheetVal::Corner(Rc::new(CornerV {
            kind: fs::CornerKind::TrimToBend,
        }))),
        _ => return None,
    })
}

/// Methods sheet values answer: `S.edge(entity)`, `U.of(op)`.
pub const METHODS: &[&str] = &["edge", "of"];

#[path = "sheet_feat.rs"]
mod feat;
pub use feat::FeatErr;

const ANG: &[Dim] = &[Dim::Angle];
const RATIO: &[Dim] = &[Dim::Ratio];

// ---- values ------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum SheetVal {
    /// A sheet feature (base, flange or cutout), on the folded body or an unfold's.
    Op(Rc<OpRef>),
    /// A straight outline edge of a wall: what a flange turns off, and its side faces.
    Edge(Rc<EdgeRef>),
    Unfold(Rc<UnfoldRef>),
    Rule(Rc<RuleV>),
    Scaled(Rc<ScaledV>),
    Stock(Rc<StockV>),
    Shop(Rc<ShopV>),
    /// A bend relief: `rect(w, d)`, `obround(w, d)`, `tear`, `no_relief` (sizes left out: the
    /// sheet's default).
    Relief(Rc<ReliefV>),
    /// A hardware catalogue row.
    Hardware(Rc<fs::Hardware>),
    /// A shop's relief rule (bend reliefs, corners).
    Rule2(Rc<fs::ReliefRule>),
    /// A corner treatment: `circular_relief(r)`, `square_relief(h)`, `open_corner(gap)`,
    /// `closed_corner`, `overlap_corner(gap)`.
    Corner(Rc<CornerV>),
}

#[derive(Clone, Debug)]
pub struct ReliefV {
    /// None: no relief.
    pub kind: Option<fs::ReliefKind>,
    pub width: Option<Num>,
    pub depth: Option<Num>,
}

#[derive(Clone, Debug)]
pub struct CornerV {
    pub kind: fs::CornerKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpKind {
    Base {
        wall: WallId,
    },
    Flange {
        wall: WallId,
        joint: JointId,
    },
    Cut {
        cut: CutId,
    },
    Corner {
        a: JointId,
        b: JointId,
    },
    /// A contour's walls, made in a row (ids `first` to `last`), and the bends before each.
    Contour {
        first: WallId,
        last: WallId,
    },
    /// A forming tool's stroke: `Sheet::forms[form]`.
    Form {
        form: u32,
    },
}

/// Which body an anchor reads: the folded one, or unfold `k`'s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum On {
    Folded,
    Flat(usize),
}

#[derive(Clone, Debug)]
pub struct OpRef {
    pub sheet: usize,
    pub id: String,
    pub kind: OpKind,
    pub on: On,
}

#[derive(Clone, Debug)]
pub struct EdgeRef {
    pub op: Rc<OpRef>,
    pub wall: WallId,
    pub segment: u64,
}

#[derive(Clone, Debug)]
pub struct UnfoldRef {
    pub sheet: usize,
    pub index: usize,
    pub id: String,
}

#[derive(Clone, Debug)]
pub struct RuleV {
    pub rule: fs::BendRule,
    /// K, for a K rule: its param is what a fix sets.
    pub k: Option<Num>,
}

#[derive(Clone, Debug)]
pub struct ScaledV {
    pub per_t: Num,
    pub per_r: Num,
    pub plus: Num,
}

#[derive(Clone, Debug)]
pub struct StockV {
    pub stock: fs::Stock,
    pub t: Num,
    pub radii: Vec<Num>,
    pub rule: Rc<RuleV>,
}

#[derive(Clone, Debug)]
pub struct ShopV {
    pub shop: fs::ShopProfile,
    pub stocks: Vec<Rc<StockV>>,
    /// Its hems: closed hems' allowances and gap (`shop(closed_hems:, hem_gap:)`).
    pub hems: fs::HemTable,
}

impl SheetVal {
    pub fn ty(&self) -> Ty {
        match self {
            SheetVal::Op(_) => Ty::Sheet,
            SheetVal::Edge(_) => Ty::SheetEdge,
            SheetVal::Unfold(_) => Ty::Unfold,
            SheetVal::Rule(_) => Ty::BendRule,
            SheetVal::Scaled(_) => Ty::Scaled,
            SheetVal::Stock(_) => Ty::Stock,
            SheetVal::Shop(_) => Ty::Shop,
            SheetVal::Relief(_) => Ty::Relief,
            SheetVal::Corner(_) => Ty::CornerKind,
            SheetVal::Hardware(_) => Ty::Hardware,
            SheetVal::Rule2(_) => Ty::ReliefRule,
        }
    }

    /// The value as read through param `key`: its numbers driven by that param.
    pub fn driven(&self, key: &ParamKey) -> SheetVal {
        let d = |n: &Num| Num {
            v: n.v,
            dim: n.dim,
            src: Src::Driven([key.clone()].into()),
        };
        let rule = |r: &RuleV| RuleV {
            rule: r.rule.clone(),
            k: r.k.as_ref().map(d),
        };
        let stock = |s: &StockV| StockV {
            stock: s.stock.clone(),
            t: d(&s.t),
            radii: s.radii.iter().map(d).collect(),
            rule: Rc::new(rule(&s.rule)),
        };
        match self {
            SheetVal::Rule(r) => SheetVal::Rule(Rc::new(rule(r))),
            SheetVal::Scaled(s) => SheetVal::Scaled(Rc::new(ScaledV {
                per_t: d(&s.per_t),
                per_r: d(&s.per_r),
                plus: d(&s.plus),
            })),
            SheetVal::Stock(s) => SheetVal::Stock(Rc::new(stock(s))),
            SheetVal::Shop(s) => SheetVal::Shop(Rc::new(ShopV {
                shop: s.shop.clone(),
                stocks: s.stocks.iter().map(|x| Rc::new(stock(x))).collect(),
                hems: s.hems.clone(),
            })),
            o => o.clone(),
        }
    }

    pub fn params(&self, out: &mut BTreeSet<ParamKey>) {
        let stock = |s: &StockV, out: &mut BTreeSet<ParamKey>| {
            out.extend(s.t.src.params());
            s.radii.iter().for_each(|r| out.extend(r.src.params()));
            if let Some(k) = &s.rule.k {
                out.extend(k.src.params());
            }
        };
        match self {
            SheetVal::Rule(r) => {
                if let Some(k) = &r.k {
                    out.extend(k.src.params());
                }
            }
            SheetVal::Scaled(s) => {
                out.extend(s.per_t.src.params());
                out.extend(s.per_r.src.params());
                out.extend(s.plus.src.params());
            }
            SheetVal::Stock(s) => stock(s, out),
            SheetVal::Shop(s) => s.stocks.iter().for_each(|x| stock(x, out)),
            _ => {}
        }
    }
}

// ---- the sheets of a part ----------------------------------------------------------------------

/// A flange's numbers as written, for fixes.
#[derive(Clone, Debug)]
struct FlangeDims {
    /// The feature that made it.
    id: String,
    joint: JointId,
    wall: WallId,
    /// Mold line to tangent line.
    setback: f64,
    /// The bend's inside radius.
    r: f64,
    height: Num,
    angle: Num,
    /// Its own radius, if it has one.
    radius: Option<Num>,
}

#[derive(Clone, Debug)]
pub struct UnfoldState {
    pub id: String,
    pub hid: u64,
    pub frame: Frame,
    pub built: Option<fs::Built>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct SheetState {
    pub id: String,
    pub hid: u64,
    /// The base feature's place in the history: the folded body's solid id.
    pub idx: usize,
    pub sheet: fs::Sheet,
    pub sketch: Rc<RefCell<Sk>>,
    pub shop: Option<Rc<ShopV>>,
    /// Kernel ids to the features that made them, with their statements' spans.
    pub walls: Vec<(WallId, String, Span)>,
    pub joints: Vec<(JointId, String, Span)>,
    pub cuts: Vec<(CutId, String, Span)>,
    /// Where each cut was sketched: for fixes that move or resize it.
    pub cut_srcs: Vec<CutSrc>,
    /// Corner treatments by the features that asked.
    pub corners: Vec<(JointId, JointId, String, Span)>,
    /// The features that made each of `sheet.forms`, in order.
    pub forms: Vec<(String, Span)>,
    fix: FixCtx,
    pub folded: Option<fs::Built>,
    pub unfolds: Vec<UnfoldState>,
    /// The stock as the part names it (`sheet(.., material: "..")`), for the drawing's note.
    pub material: Option<String>,
}

#[derive(Default, Debug)]
pub struct Sheets {
    pub list: Vec<SheetState>,
}

/// A finished sheet, for the run's report and the DXF.
#[derive(Clone, Debug)]
pub struct SheetOut {
    pub id: String,
    pub sheet: fs::Sheet,
    pub walls: Vec<(WallId, String)>,
    pub joints: Vec<(JointId, String)>,
    pub layout: Option<fs::Layout>,
    /// The folded body as the part holds it, and each unfold's (its names salted by the unfold).
    pub folded: Option<Body>,
    pub unfolds: Vec<(String, Option<Body>)>,
    /// The reliefs and corner reliefs the builder cut, as typed data.
    pub reliefs: Vec<fs::ReliefMade>,
    pub corners: Vec<fs::build::CornerMade>,
    pub material: Option<String>,
}

impl SheetState {
    fn names(&self) -> Names {
        Names {
            walls: self.walls.iter().map(|x| (x.0, x.1.clone())).collect(),
            joints: self.joints.iter().map(|x| (x.0, x.1.clone())).collect(),
            cuts: self.cuts.iter().map(|x| (x.0, x.1.clone())).collect(),
            forms: self.forms.iter().map(|x| x.0.clone()).collect(),
        }
    }
}

/// A cut's sketch, and the frames that carried it onto its wall; for hardware, the entity it is
/// centred on.
#[derive(Clone, Debug)]
pub struct CutSrc {
    pub cut: CutId,
    pub sketch: Rc<RefCell<Sk>>,
    pub from: Frame,
    pub to: Frame,
    pub at: Option<forge_sketch::EntityId>,
}

/// What fixes are made from: the numbers as written, with where they came from.
#[derive(Clone, Debug)]
struct FixCtx {
    flanges: Vec<FlangeDims>,
    t: Num,
    r: Num,
    k: Option<Num>,
    /// What an automatic relief would be.
    relief: fs::ReliefRule,
}

impl FixCtx {
    /// The relief texts a flange needing one could ask for, sized from the default and past
    /// its tangent line.
    fn relief_texts(&self, f: &FlangeDims) -> Vec<String> {
        let t = self.t.v;
        let fm = crate::values::fmt;
        let w = self.relief.bend.width.at(t, f.r);
        // the rule's depth (from the mold line), at least to the tangent line
        let d = |w: f64, round: bool| {
            let need = f.setback + if round { 0.5 * w } else { 0.0 };
            self.relief.bend.depth.at(t, f.r).max(need)
        };
        vec![
            format!("rect({}, {})", fm(w), fm(d(w, false))),
            format!("obround({}, {})", fm(w), fm(d(w, true))),
            "tear".into(),
        ]
    }
}

/// Kernel ids by Forge feature id, for rendering kernel errors.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Names {
    walls: Vec<(WallId, String)>,
    joints: Vec<(JointId, String)>,
    cuts: Vec<(CutId, String)>,
    forms: Vec<String>,
}

impl Names {
    pub(super) fn form(&self, k: usize) -> String {
        self.forms.get(k).cloned().unwrap_or_else(|| format!("form {k}"))
    }
    fn wall(&self, w: WallId) -> String {
        self.walls
            .iter()
            .find(|x| x.0 == w)
            .map_or_else(|| format!("wall {}", w.0), |x| x.1.clone())
    }
    fn joint(&self, j: JointId) -> String {
        self.joints
            .iter()
            .find(|x| x.0 == j)
            .map_or_else(|| format!("joint {}", j.0), |x| x.1.clone())
    }
    fn cut(&self, c: CutId) -> String {
        self.cuts
            .iter()
            .find(|x| x.0 == c)
            .map_or_else(|| format!("cut {}", c.0), |x| x.1.clone())
    }
    fn strip_name(&self, s: Strip) -> String {
        match s {
            Strip::Wall(w) => self.wall(w),
            Strip::Bend(j) => format!("the bend of {}", self.joint(j)),
        }
    }
    fn strip(&self, s: Strip) -> Json {
        match s {
            Strip::Wall(w) => Json::obj(vec![("wall", self.wall(w).into())]),
            Strip::Bend(j) => Json::obj(vec![("bend", self.joint(j).into())]),
        }
    }
    fn trim(&self, t: TrimRef) -> Json {
        let side = |s: fs::Side| if s == fs::Side::A { "a" } else { "b" };
        match t {
            TrimRef::Outline { wall, segment } => Json::obj(vec![
                ("outline", self.wall(wall).into()),
                ("segment", (segment as f64).into()),
            ]),
            TrimRef::Cut { cut, segment } => Json::obj(vec![
                ("cut", self.cut(cut).into()),
                ("segment", (segment as f64).into()),
            ]),
            TrimRef::TangentLine { joint, side: s } => Json::obj(vec![
                ("tangent_line", self.joint(joint).into()),
                ("side", side(s).into()),
            ]),
            TrimRef::BendEnd { joint, end } => Json::obj(vec![
                ("bend_end", self.joint(joint).into()),
                ("end", if end == fs::End::Start { "start" } else { "end" }.into()),
            ]),
            TrimRef::MoldLine { joint, side: s } => Json::obj(vec![
                ("mold_line", self.joint(joint).into()),
                ("side", side(s).into()),
            ]),
            TrimRef::Relief { joint, end, segment } => Json::obj(vec![
                ("relief", self.joint(joint).into()),
                ("end", end_name(end).into()),
                ("segment", (segment as f64).into()),
            ]),
            TrimRef::Corner { a, b, segment } => Json::obj(vec![
                (
                    "corner",
                    Json::Arr(vec![self.joint(a).into(), self.joint(b).into()]),
                ),
                ("segment", (segment as f64).into()),
            ]),
            TrimRef::Form { form, segment } => Json::obj(vec![
                ("form", self.form(form as usize).into()),
                ("segment", (segment as f64).into()),
            ]),
        }
    }
}

// ---- errors and notes --------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum SheetErr {
    /// The kernel refused the sheet as this feature left it.
    Kernel {
        span: Span,
        id: String,
        error: SheetError,
        names: Names,
        fixes: Vec<Op>,
    },
    /// A feature that is not sheet metal, in a part with a sheet (refused at first).
    OrdinaryOnSheet { span: Span, id: String, sheet: String },
    /// A sheet started in a part that already has an ordinary solid.
    SheetOnSolid { span: Span, id: String, solid: String },
    /// A sheet feature after its sheet was unfolded: the unfold would be stale.
    AfterUnfold { span: Span, id: String, unfold: String },
    /// `S.edge(entity)`: the entity is no straight segment of the wall's outline.
    NotAnEdge { span: Span, wall: String },
    /// A cutout whose sketch does not lie on the wall it names (`distance` off its skins).
    CutOffWall { span: Span, wall: String, distance: f64 },
    /// `U.of(op)` with an op of another sheet.
    OtherSheet { span: Span, op: String, unfold: String },
    /// A member only some sheet features have (`bend` of a base wall, ..).
    NoSuchRole { span: Span, op: String, role: String },
    /// No catalogue row of this kind and size.
    NoSuchHardware {
        span: Span,
        kind: String,
        size: String,
        sizes: Vec<String>,
    },
    /// `corner_relief(a, b, ..)` with flanges that do not meet at a corner of one wall.
    NotACorner { span: Span, id: String },
    /// A table row not after the one before it in its group (`after`), as the kernel reads them.
    TableOrder { span: Span, row: usize, after: usize },
    /// `pattern(of: ..)` of a sheet feature that is not a cutout.
    PatternOfSheet { span: Span, op: String },
    /// A pattern copy whose plane leaves its cutout's wall (`distance` off it, or tilted).
    PatternOffWall { span: Span, distance: f64 },
    /// A rule table row of the wrong shape.
    TableRow { span: Span, row: usize, want: usize },
    /// The newer features' own refusals (`sheet_feat.rs`).
    Feat(FeatErr),
}

impl SheetErr {
    pub fn span(&self) -> Option<Span> {
        use SheetErr::*;
        match self {
            Kernel { span, .. }
            | OrdinaryOnSheet { span, .. }
            | SheetOnSolid { span, .. }
            | AfterUnfold { span, .. }
            | NotAnEdge { span, .. }
            | CutOffWall { span, .. }
            | OtherSheet { span, .. }
            | NoSuchRole { span, .. }
            | TableRow { span, .. }
            | TableOrder { span, .. }
            | PatternOfSheet { span, .. }
            | PatternOffWall { span, .. }
            | NotACorner { span, .. }
            | NoSuchHardware { span, .. } => Some(*span),
            Feat(e) => Some(e.span()),
        }
    }

    pub fn kind(&self) -> &'static str {
        use SheetErr::*;
        match self {
            Kernel { error, .. } => kernel_kind(error),
            OrdinaryOnSheet { .. } => "OrdinaryOnSheet",
            SheetOnSolid { .. } => "SheetOnSolid",
            AfterUnfold { .. } => "AfterUnfold",
            NotAnEdge { .. } => "NotAnEdge",
            CutOffWall { .. } => "CutOffWall",
            OtherSheet { .. } => "OtherSheet",
            NoSuchRole { .. } => "NoSuchRole",
            TableRow { .. } => "TableRow",
            TableOrder { .. } => "TableOrder",
            PatternOfSheet { .. } => "PatternOfSheet",
            PatternOffWall { .. } => "PatternOffWall",
            Feat(e) => e.kind(),
            NotACorner { .. } => "NotACorner",
            NoSuchHardware { .. } => "NoSuchHardware",
        }
    }

    pub fn fixes(&self) -> Vec<Op> {
        use SheetErr::*;
        match self {
            Kernel { fixes, .. } => fixes.clone(),
            OrdinaryOnSheet { id, .. } | SheetOnSolid { id, .. } => {
                vec![Op::DeleteFeature { id: id.clone() }]
            }
            AfterUnfold { unfold, .. } => vec![Op::MoveFeature {
                id: unfold.clone(),
                before: None,
            }],
            _ => vec![],
        }
    }

    pub fn text(&self) -> String {
        use SheetErr::*;
        match self {
            Kernel { id, error, names, .. } => format!(
                "{id}: the sheet is refused: {}",
                feat::form_error_text(error, names).unwrap_or_else(|| kernel_text(error))
            ),
            OrdinaryOnSheet { id, sheet, .. } => format!(
                "{id} is not sheet metal, and the part has the sheet {sheet}: ordinary features on a sheet are refused"
            ),
            SheetOnSolid { id, solid, .. } => {
                format!(
                    "sheet {id} in a part that already has the solid {solid}: sheet parts hold sheets only"
                )
            }
            AfterUnfold { id, unfold, .. } => format!("{id} changes a sheet already unfolded by {unfold}"),
            NotAnEdge { wall, .. } => format!("not a straight edge of {wall}'s outline"),
            CutOffWall { wall, distance, .. } => format!(
                "the cutout's sketch lies {} off {wall}'s skins: sketch it on {wall}.top or {wall}.bottom",
                crate::values::fmt(*distance)
            ),
            OtherSheet { op, unfold, .. } => format!("{op} is not of the sheet {unfold} unfolds"),
            NoSuchRole { op, role, .. } => format!("{op} has no {role}"),
            TableRow { row, want, .. } => format!("table row {row} needs {want} values"),
            PatternOfSheet { op, .. } => {
                format!("{op} is a sheet feature but not a cutout: only cutouts are patterned on a sheet")
            }
            PatternOffWall { distance, .. } => format!(
                "a pattern copy leaves its cutout's wall ({} off, or turned out of its plane)",
                crate::values::fmt(*distance)
            ),
            TableOrder { row, after, .. } => {
                format!("table row {row} does not come after row {after}: rows run in increasing order")
            }
            Feat(e) => e.text(),
            NotACorner { id, .. } => format!("{id}: the two flanges do not meet at a corner of one wall"),
            NoSuchHardware {
                kind, size, sizes, ..
            } => {
                format!("no {kind} {size} in the catalogue; it has {}", sizes.join(", "))
            }
        }
    }

    pub fn data(&self) -> Vec<(&'static str, Json)> {
        use SheetErr::*;
        let s = |x: &str| Json::Str(x.into());
        match self {
            Kernel { id, error, names, .. } => {
                let mut v = vec![("id", s(id))];
                v.extend(kernel_data(error, names));
                v
            }
            OrdinaryOnSheet { id, sheet, .. } => vec![("id", s(id)), ("sheet", s(sheet))],
            SheetOnSolid { id, solid, .. } => vec![("id", s(id)), ("solid", s(solid))],
            AfterUnfold { id, unfold, .. } => vec![("id", s(id)), ("unfold", s(unfold))],
            NotAnEdge { wall, .. } => vec![("wall", s(wall))],
            CutOffWall { wall, distance, .. } => vec![("wall", s(wall)), ("distance", (*distance).into())],
            OtherSheet { op, unfold, .. } => vec![("op", s(op)), ("unfold", s(unfold))],
            NoSuchRole { op, role, .. } => vec![("op", s(op)), ("role", s(role))],
            TableRow { row, want, .. } => {
                vec![("row", (*row as f64).into()), ("want", (*want as f64).into())]
            }
            TableOrder { row, after, .. } => {
                vec![("row", (*row as f64).into()), ("after", (*after as f64).into())]
            }
            PatternOfSheet { op, .. } => vec![("op", s(op))],
            PatternOffWall { distance, .. } => vec![("distance", (*distance).into())],
            NotACorner { id, .. } => vec![("id", s(id))],
            Feat(e) => e.data(),
            NoSuchHardware {
                kind, size, sizes, ..
            } => vec![
                ("hardware_kind", s(kind)),
                ("size", s(size)),
                ("sizes", Json::Arr(sizes.iter().map(|x| s(x)).collect())),
            ],
        }
    }
}

/// A kernel warning or a shop violation: the part built.
#[derive(Clone, Debug, PartialEq)]
pub enum Note {
    Warning(fs::SheetWarning),
    Violation(fs::Violation),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SheetNote {
    pub span: Span,
    pub id: String,
    pub note: Note,
    pub names: Names,
    pub fixes: Vec<Op>,
}

impl SheetNote {
    pub fn kind(&self) -> &'static str {
        use fs::SheetWarning as W;
        use fs::Violation as V;
        match &self.note {
            Note::Warning(w) => match w {
                W::FlatOverlap { .. } => "FlatOverlap",
                W::ReliefRemnant { .. } => "ReliefRemnant",
                W::UnverifiedHardware { .. } => "UnverifiedHardware",
                W::FlatOverlapUndecided { .. } => "FlatOverlapUndecided",
                W::FoldedClash { .. } => "FoldedClash",
                W::FlatTouch { .. } => "FlatTouch",
                W::FoldedTouch { .. } => "FoldedTouch",
                W::FormClash { .. } => "FormClash",
            },
            Note::Violation(v) => match v {
                V::HoleNearBend { .. } => "HoleNearBend",
                V::HoleNearEdge { .. } => "HoleNearEdge",
                V::FlangeTooShort { .. } => "FlangeTooShort",
                V::RadiusNotTooled { .. } => "RadiusNotTooled",
                V::ThicknessNotStocked { .. } => "ThicknessNotStocked",
                V::SlotTooNarrow { .. } => "SlotTooNarrow",
                V::HoleTooSmall { .. } => "HoleTooSmall",
                V::HardwareThickness { .. } => "HardwareThickness",
                V::HardwareNearEdge { .. } => "HardwareNearEdge",
                V::HardwareOverlapsHole { .. } => "HardwareOverlapsHole",
                V::FlatTooLarge { .. } => "FlatTooLarge",
                V::PartTooSmall { .. } => "PartTooSmall",
                V::BendTooLong { .. } => "BendTooLong",
                V::BoxFlangeTooTall { .. } => "BoxFlangeTooTall",
                V::BendAngleOutOfRange { .. } => "BendAngleOutOfRange",
                V::HardwareNearHole { .. } => "HardwareNearHole",
                V::SheetTooHard { .. } => "SheetTooHard",
                V::HardnessUnknown { .. } => "HardnessUnknown",
                V::ReliefUndersized { .. } => "ReliefUndersized",
                V::CutCrossesBend { .. } => "CutCrossesBend",
                V::LipTooShort { .. } => "LipTooShort",
                V::Punch { why, .. } => feat::punch_kind(why),
                V::Form { why, .. } => feat::form_kind(why),
            },
        }
    }

    pub fn severity(&self) -> &'static str {
        match self.note {
            Note::Warning(_) => "warning",
            // a cut marked as meant to cross its bend is only information
            Note::Violation(fs::Violation::CutCrossesBend { intended: true, .. }) => "info",
            Note::Violation(_) => "violation",
        }
    }

    pub fn data(&self) -> Vec<(&'static str, Json)> {
        use fs::SheetWarning as W;
        use fs::Violation as V;
        let n = &self.names;
        let f = |x: f64| Json::Num(x);
        let v2 = |v: Vec3Ish| Json::Arr(vec![v.0.into(), v.1.into()]);
        let mut v: Vec<(&'static str, Json)> = vec![("id", self.id.as_str().into())];
        match &self.note {
            Note::Warning(w) => match *w {
                W::FlatOverlap {
                    a,
                    b,
                    at,
                    area,
                    total,
                    limit,
                    ..
                } => v.extend([
                    ("a", n.strip(a)),
                    ("b", n.strip(b)),
                    ("at", p2(at)),
                    ("area", f(area)),
                    ("total", f(total)),
                    ("limit", f(limit)),
                ]),
                W::FlatOverlapUndecided {
                    a, b, at, gap, limit, ..
                } => v.extend([
                    ("a", n.strip(a)),
                    ("b", n.strip(b)),
                    ("at", p2(at)),
                    ("gap", f(gap)),
                    ("limit", f(limit)),
                ]),
                W::FoldedClash {
                    a,
                    b,
                    at,
                    depth,
                    limit,
                    ..
                } => v.extend([
                    ("a", n.strip(a)),
                    ("b", n.strip(b)),
                    ("at", p3(at)),
                    ("depth", f(depth)),
                    ("limit", f(limit)),
                ]),
                W::FlatTouch { a, b, at, .. } => {
                    v.extend([("a", n.strip(a)), ("b", n.strip(b)), ("at", p2(at))])
                }
                W::FoldedTouch { a, b, at, .. } => {
                    v.extend([("a", n.strip(a)), ("b", n.strip(b)), ("at", p3(at))])
                }
                W::ReliefRemnant {
                    joint,
                    remnant,
                    limit,
                    ..
                } => v.extend([
                    ("joint", n.joint(joint).into()),
                    ("remnant", f(remnant)),
                    ("limit", f(limit)),
                ]),
                W::UnverifiedHardware { cut, .. } => v.push(("cut", n.cut(cut).into())),
                W::FormClash {
                    form,
                    with,
                    at,
                    depth,
                    limit,
                    ..
                } => v.extend([
                    ("form", n.form(form).into()),
                    (
                        "with",
                        match with {
                            fs::FormAgainst::Strip(s) => n.strip(s),
                            fs::FormAgainst::Form(k) => Json::obj(vec![("form", n.form(k).into())]),
                        },
                    ),
                    ("at", p3(at)),
                    ("depth", f(depth)),
                    ("limit", f(limit)),
                ]),
            },
            Note::Violation(V::Form { form, why, .. }) => {
                v.push(("form", n.form(*form).into()));
                v.extend(feat::form_data(why, n));
            }
            Note::Violation(V::Punch { punch, why, .. }) => {
                v.push(("punch", (*punch as f64).into()));
                v.extend(feat::punch_data(why, n));
            }
            Note::Violation(x) => match *x {
                V::Punch { .. } | V::Form { .. } => unreachable!("above"),
                V::HoleNearBend {
                    trim,
                    joint,
                    distance,
                    limit,
                    thickness,
                    away,
                    ..
                } => v.extend([
                    ("trim", n.trim(trim)),
                    ("joint", n.joint(joint).into()),
                    ("distance", f(distance)),
                    ("limit", f(limit)),
                    ("thickness", f(thickness)),
                    ("away", v2(Vec3Ish(away.x, away.y))),
                ]),
                V::HoleNearEdge {
                    trim,
                    distance,
                    limit,
                    away,
                    ..
                } => v.extend([
                    ("trim", n.trim(trim)),
                    ("distance", f(distance)),
                    ("limit", f(limit)),
                    ("away", v2(Vec3Ish(away.x, away.y))),
                ]),
                V::FlangeTooShort {
                    wall,
                    joint,
                    length,
                    limit,
                    ..
                } => v.extend([
                    ("wall", n.wall(wall).into()),
                    ("joint", n.joint(joint).into()),
                    ("length", f(length)),
                    ("limit", f(limit)),
                ]),
                V::RadiusNotTooled {
                    joint,
                    radius,
                    nearest,
                    ..
                } => v.extend([
                    ("joint", n.joint(joint).into()),
                    ("radius", f(radius)),
                    ("nearest", f(nearest)),
                ]),
                V::ThicknessNotStocked {
                    thickness, nearest, ..
                } => v.extend([("thickness", f(thickness)), ("nearest", f(nearest))]),
                V::SlotTooNarrow {
                    trim,
                    width,
                    limit,
                    thickness,
                    ..
                } => v.extend([
                    ("trim", n.trim(trim)),
                    ("width", f(width)),
                    ("limit", f(limit)),
                    ("thickness", f(thickness)),
                ]),
                V::HoleTooSmall {
                    trim,
                    diameter,
                    limit,
                    thickness,
                    ..
                } => v.extend([
                    ("trim", n.trim(trim)),
                    ("diameter", f(diameter)),
                    ("limit", f(limit)),
                    ("thickness", f(thickness)),
                ]),
                V::HardwareThickness {
                    cut,
                    thickness,
                    min,
                    max,
                    ..
                } => v.extend([
                    ("cut", n.cut(cut).into()),
                    ("thickness", f(thickness)),
                    ("min", f(min)),
                    ("max", f(max)),
                ]),
                V::FlatTooLarge {
                    size, limit, coated, ..
                } => v.extend([
                    ("size", Json::Arr(vec![size.0.into(), size.1.into()])),
                    ("limit", Json::Arr(vec![limit.0.into(), limit.1.into()])),
                    ("coated", Json::Bool(coated)),
                ]),
                V::PartTooSmall { size, limit, .. } => v.extend([
                    ("size", Json::Arr(vec![size.0.into(), size.1.into()])),
                    ("limit", Json::Arr(vec![limit.0.into(), limit.1.into()])),
                ]),
                V::BendTooLong {
                    joint, length, limit, ..
                } => v.extend([
                    ("joint", n.joint(joint).into()),
                    ("length", f(length)),
                    ("limit", f(limit)),
                ]),
                V::BoxFlangeTooTall {
                    wall,
                    joint,
                    height,
                    limit,
                    ..
                } => v.extend([
                    ("wall", n.strip(Strip::Wall(wall))),
                    ("joint", n.joint(joint).into()),
                    ("height", f(height)),
                    ("limit", f(limit)),
                ]),
                V::BendAngleOutOfRange {
                    joint,
                    angle,
                    min,
                    max,
                    ..
                } => v.extend([
                    ("joint", n.joint(joint).into()),
                    ("angle", f(angle.to_degrees())),
                    ("min", f(min.to_degrees())),
                    ("max", f(max.to_degrees())),
                ]),
                V::HardwareOverlapsHole {
                    cut,
                    other,
                    distance,
                    hole,
                    limit,
                    ..
                } => v.extend([
                    ("cut", n.cut(cut).into()),
                    ("other", n.trim(other)),
                    ("distance", f(distance)),
                    ("hole", f(hole)),
                    ("limit", f(limit)),
                ]),
                V::HardwareNearHole {
                    cut,
                    other,
                    distance,
                    limit,
                    away,
                    ..
                } => v.extend([
                    ("cut", n.cut(cut).into()),
                    ("other", n.trim(other)),
                    ("distance", f(distance)),
                    ("limit", f(limit)),
                    ("away", v2(Vec3Ish(away.x, away.y))),
                ]),
                V::HardwareNearEdge {
                    cut,
                    distance,
                    limit,
                    away,
                    ..
                } => v.extend([
                    ("cut", n.cut(cut).into()),
                    ("distance", f(distance)),
                    ("limit", f(limit)),
                    ("away", v2(Vec3Ish(away.x, away.y))),
                ]),
                V::SheetTooHard {
                    cut, hardness, limit, ..
                } => v.extend([
                    ("cut", n.cut(cut).into()),
                    ("hardness", f(hardness)),
                    ("limit", f(limit)),
                ]),
                V::HardnessUnknown { cut, limit, .. } => {
                    v.extend([("cut", n.cut(cut).into()), ("limit", f(limit))])
                }
                V::ReliefUndersized {
                    joint,
                    end,
                    width,
                    depth,
                    min_width,
                    min_depth,
                    ..
                } => v.extend([
                    ("joint", n.joint(joint).into()),
                    ("end", end_name(end).into()),
                    ("width", f(width)),
                    ("depth", f(depth)),
                    ("min_width", f(min_width)),
                    ("min_depth", f(min_depth)),
                ]),
                V::CutCrossesBend {
                    trim,
                    joint,
                    intended,
                    away,
                    ..
                } => v.extend([
                    ("trim", n.trim(trim)),
                    ("joint", n.joint(joint).into()),
                    ("intended", intended.into()),
                    ("away", v2(Vec3Ish(away.x, away.y))),
                ]),
                V::LipTooShort {
                    cut,
                    wall,
                    width,
                    need,
                    ..
                } => v.extend([
                    ("cut", n.cut(cut).into()),
                    ("wall", n.wall(wall).into()),
                    ("width", f(width)),
                    ("need", f(need)),
                ]),
            },
        }
        v
    }

    pub fn text(&self) -> String {
        use fs::SheetWarning as W;
        use fs::Violation as V;
        let fm = crate::values::fmt;
        let n = &self.names;
        match &self.note {
            Note::Warning(W::FlatOverlap { a, b, total, .. }) => format!(
                "{} and {} overlap in the flat pattern ({} mm2 in all): the folded part builds, no blank can be cut",
                n.strip_name(*a),
                n.strip_name(*b),
                fm(*total)
            ),
            Note::Warning(W::FlatOverlapUndecided { a, b, gap, .. }) => format!(
                "{} and {} come within {} of each other in the flat pattern: overlap or touch cannot be told",
                n.strip_name(*a),
                n.strip_name(*b),
                fm(*gap)
            ),
            Note::Warning(W::FoldedClash { a, b, depth, .. }) => format!(
                "{} and {} pass through each other folded ({} deep); the flat builds",
                n.strip_name(*a),
                n.strip_name(*b),
                fm(*depth)
            ),
            Note::Warning(W::FlatTouch { a, b, .. }) => format!(
                "{} and {} touch in the flat pattern, no gap between: the folded part builds, the blank cannot be cut there",
                n.strip_name(*a),
                n.strip_name(*b)
            ),
            Note::Warning(W::FoldedTouch { a, b, .. }) => format!(
                "{} and {} touch folded, an edge on a face with no gap: the flat builds, the folded solid is not valid",
                n.strip_name(*a),
                n.strip_name(*b)
            ),
            Note::Warning(W::ReliefRemnant { joint, remnant, .. }) => {
                format!(
                    "a sliver {} wide beside the bend of {}",
                    fm(*remnant),
                    n.joint(*joint)
                )
            }
            Note::Warning(W::UnverifiedHardware { cut, .. }) => format!(
                "{}: the hardware's numbers are placeholders, not a maker's table",
                n.cut(*cut)
            ),
            Note::Violation(V::FlangeTooShort {
                wall, length, limit, ..
            }) => format!(
                "{} reaches {} from its bend; the die needs {}",
                n.wall(*wall),
                fm(*length),
                fm(*limit)
            ),
            Note::Violation(V::RadiusNotTooled {
                joint,
                radius,
                nearest,
                ..
            }) => format!(
                "the bend of {} has radius {}; the brake is tooled for {} nearest",
                n.joint(*joint),
                fm(*radius),
                fm(*nearest)
            ),
            Note::Violation(V::ThicknessNotStocked {
                thickness, nearest, ..
            }) => {
                format!(
                    "thickness {} is not stocked; {} is nearest",
                    fm(*thickness),
                    fm(*nearest)
                )
            }
            Note::Violation(V::HoleNearBend {
                joint,
                distance,
                limit,
                ..
            }) => format!(
                "a hole {} from the bend of {}; the shop needs {}",
                fm(*distance),
                n.joint(*joint),
                fm(*limit)
            ),
            Note::Violation(V::HoleNearEdge { distance, limit, .. }) => {
                format!(
                    "a hole {} from the edge; the shop needs {}",
                    fm(*distance),
                    fm(*limit)
                )
            }
            Note::Violation(V::SlotTooNarrow { width, limit, .. }) => {
                format!("a cut {} wide; the shop needs {}", fm(*width), fm(*limit))
            }
            Note::Violation(V::HoleTooSmall { diameter, limit, .. }) => {
                format!(
                    "a hole {} across; the shop pierces {} at least",
                    fm(*diameter),
                    fm(*limit)
                )
            }
            Note::Violation(V::HardwareThickness {
                thickness, min, max, ..
            }) => format!(
                "the hardware takes sheet {} to {}; this is {}",
                fm(*min),
                fm(*max),
                fm(*thickness)
            ),
            Note::Violation(V::HardwareOverlapsHole {
                cut, distance, hole, ..
            }) if *distance == 0.0 => format!(
                "{}: the hardware's centre is inside another hole (its own hole is {})",
                n.cut(*cut),
                fm(*hole)
            ),
            Note::Violation(V::HardwareOverlapsHole {
                cut, distance, hole, ..
            }) => format!(
                "{}: the hardware's {} hole runs into another hole, whose edge is {} from its centre",
                n.cut(*cut),
                fm(*hole),
                fm(*distance)
            ),
            Note::Violation(V::FlatTooLarge {
                size, limit, coated, ..
            }) => format!(
                "the flat blank is {} x {}; the shop takes {} x {}{}",
                fm(size.0),
                fm(size.1),
                fm(limit.0),
                fm(limit.1),
                if *coated { " powder coated" } else { "" }
            ),
            Note::Violation(V::PartTooSmall { size, limit, .. }) => format!(
                "the part is {} x {}; the shop handles {} x {} at least",
                fm(size.0),
                fm(size.1),
                fm(limit.0),
                fm(limit.1)
            ),
            Note::Violation(V::BendTooLong {
                joint, length, limit, ..
            }) => format!(
                "the bend of {} is {} long; the brake bends {}",
                n.joint(*joint),
                fm(*length),
                fm(*limit)
            ),
            Note::Violation(V::BoxFlangeTooTall {
                joint, height, limit, ..
            }) => format!(
                "{} is {} tall round a closed box; the tooling allows {}",
                n.joint(*joint),
                fm(*height),
                fm(*limit)
            ),
            Note::Violation(V::BendAngleOutOfRange {
                joint,
                angle,
                min,
                max,
                ..
            }) => format!(
                "the bend of {} turns {} deg; the brake makes {} to {} deg",
                n.joint(*joint),
                fm(angle.to_degrees()),
                fm(min.to_degrees()),
                fm(max.to_degrees())
            ),
            Note::Violation(V::HardwareNearHole { distance, limit, .. }) => format!(
                "the hardware's hole centre is {} from another hole's edge; it needs {}",
                fm(*distance),
                fm(*limit)
            ),
            Note::Violation(V::HardwareNearEdge { distance, limit, .. }) => format!(
                "the hardware's hole centre is {} from an edge or bend; it needs {}",
                fm(*distance),
                fm(*limit)
            ),
            Note::Violation(V::SheetTooHard { hardness, limit, .. }) => format!(
                "the sheet is HRB {}; the hardware clinches into HRB {} at most",
                fm(*hardness),
                fm(*limit)
            ),
            Note::Violation(V::HardnessUnknown { limit, .. }) => format!(
                "the hardware needs sheet no harder than HRB {}; the stock states no hardness",
                fm(*limit)
            ),
            Note::Violation(V::ReliefUndersized {
                joint,
                width,
                depth,
                min_width,
                min_depth,
                ..
            }) => format!(
                "the relief at {} is {} wide and {} deep; the shop's rule makes them {} by {}",
                n.joint(*joint),
                fm(*width),
                fm(*depth),
                fm(*min_width),
                fm(*min_depth)
            ),
            Note::Violation(V::CutCrossesBend { joint, intended, .. }) => {
                if *intended {
                    format!("a cut runs across the bend of {}, as meant", n.joint(*joint))
                } else {
                    format!(
                        "a cut runs across the bend of {}: if meant, say crosses_bend: true",
                        n.joint(*joint)
                    )
                }
            }
            Note::Violation(V::LipTooShort {
                wall, width, need, ..
            }) => format!(
                "{} is {} across where the hardware sits; it needs {} (its edge distance both ways)",
                n.wall(*wall),
                fm(*width),
                fm(*need)
            ),
            Note::Violation(V::Punch { why, .. }) => feat::punch_text(why, n),
            Note::Violation(V::Form { why, .. }) => feat::form_text(why, n),
            Note::Warning(W::FormClash {
                form, with, depth, ..
            }) => format!(
                "{} passes {} deep through {} folded; the flat builds",
                n.form(*form),
                fm(*depth),
                match with {
                    fs::FormAgainst::Strip(s) => n.strip_name(*s),
                    fs::FormAgainst::Form(k) => n.form(*k),
                }
            ),
        }
    }

    pub fn json(&self, src: &str) -> Json {
        let (l, c, ..) = self.span.lines(src);
        let mut f: Vec<(&str, Json)> =
            vec![("kind", self.kind().into()), ("severity", self.severity().into())];
        f.extend(self.data());
        f.push(("line", (l as f64).into()));
        f.push(("col", (c as f64).into()));
        f.push(("text", self.text().into()));
        f.push(("fixes", Json::Arr(self.fixes.iter().map(Op::json).collect())));
        Json::obj(f)
    }
}

/// A 2D vector, for JSON.
struct Vec3Ish(f64, f64);

fn end_name(e: fs::End) -> &'static str {
    if e == fs::End::Start { "start" } else { "end" }
}

fn p2(p: Point2) -> Json {
    Json::Arr(vec![p.x.into(), p.y.into()])
}

fn p3(p: Point3) -> Json {
    Json::Arr(vec![p.x.into(), p.y.into(), p.z.into()])
}

fn kernel_kind(e: &SheetError) -> &'static str {
    use SheetError::*;
    if let Some(k) = feat::kernel_kind(e).or_else(|| feat::form_error_kind(e)) {
        return k;
    }
    match e {
        Thickness { .. } => "Thickness",
        Radius { .. } => "Radius",
        Profile { .. } => "SheetProfile",
        NoSuchWall { .. } => "NoSuchWall",
        NoSuchSegment { .. } => "NoSuchSegment",
        NotStraight { .. } => "NotStraight",
        EdgeTaken { .. } => "EdgeTaken",
        BendAngle { .. } => "BendAngle",
        WallTooShort { .. } => "WallTooShort",
        JointLengths { .. } => "JointLengths",
        JointWalls { .. } => "JointWalls",
        WallUnplaced { .. } => "WallUnplaced",
        JointKindUnbuilt { .. } => "JointKindUnbuilt",
        BendRule { .. } => "BendRule",
        Pinched { .. } => "Pinched",
        NearCoincident { .. } => "NearCoincident",
        Unchained { .. } => "Unchained",
        HoleOutside { .. } => "HoleOutside",
        TrimAlongBendLine { .. } => "TrimAlongBendLine",
        TrimTouchesBendLine { .. } => "TrimTouchesBendLine",
        NeedsRelief { .. } => "NeedsRelief",
        TangentMismatch { .. } => "TangentMismatch",
        ArcSweep { .. } => "ArcSweep",
        FoldedClash { .. } => "FoldedClash",
        TrimCurveUnsupported { .. } => "TrimCurveUnsupported",
        WallFolds { .. } => "WallFolds",
        EdgeTooShort { .. } => "EdgeTooShort",
        ReliefTooShallow { .. } => "ReliefTooShallow",
        ReliefWidth { .. } => "ReliefWidth",
        EdgeMoveBlocked { .. } => "EdgeMoveBlocked",
        EdgeMoveCurve { .. } => "EdgeMoveCurve",
        CornerUnbuilt { .. } => "CornerUnbuilt",
        FlatOverlap { .. } => "FlatOverlap",
        FlatOverlapUndecided { .. } => "FlatOverlapUndecided",
        FlatTouch { .. } => "FlatTouch",
        FoldedTouch { .. } => "FoldedTouch",
        HardwareOffWall { .. } => "HardwareOffWall",
        ReliefExtendUnbuilt { .. } => "ReliefExtendUnbuilt",
        SheetSplit { .. } => "SheetSplit",
        _ => unreachable!("the features' own, above"),
    }
}

fn kernel_text(e: &SheetError) -> String {
    use SheetError::*;
    let fm = crate::values::fmt;
    if let Some(t) = feat::kernel_text(e) {
        return t;
    }
    match e {
        Thickness { thickness, .. } => format!("thickness {} is not positive", fm(*thickness)),
        Radius { radius, .. } => format!("bend radius {} is not positive", fm(*radius)),
        Profile { why, .. } => format!("the base profile: {why:?}"),
        NoSuchWall { .. } => "no such wall".into(),
        NoSuchSegment { .. } => "no such outline segment".into(),
        NotStraight { curve, .. } => format!("a flange needs a straight edge, not a {curve:?}"),
        EdgeTaken { .. } => "the edge already carries a bend".into(),
        BendAngle { angle, limit, .. } => format!(
            "bend angle {}deg outside (0, 180) (limit {}deg)",
            fm(angle.to_degrees()),
            fm(limit.to_degrees())
        ),
        WallTooShort { length, .. } => format!(
            "the wall has {} left past the bend's setback; it needs more than nothing",
            fm(*length)
        ),
        JointLengths { a, b, .. } => format!("a joint's two edges differ: {} and {}", fm(*a), fm(*b)),
        JointWalls { .. } => "a joint's walls are missing or reached twice".into(),
        WallUnplaced { .. } => "a wall no bend reaches".into(),
        JointKindUnbuilt { kind, .. } => format!("{kind:?} joints are not built yet"),
        BendRule { why, .. } => format!("the bend rule: {why:?}"),
        Pinched { pinches, .. } => format!("a trim leaves zero width at {} places", pinches.len()),
        NearCoincident { gap, .. } => format!("two trim edges {} apart, too close to tell apart", fm(*gap)),
        Unchained { .. } => "trim pieces do not chain into loops".into(),
        HoleOutside { .. } => "a hole outside every wall".into(),
        TrimAlongBendLine { .. } => "a trim edge runs along a tangent line (a relief to tangent: reliefs wait on a decision)".into(),
        TrimTouchesBendLine { gap, .. } => format!(
            "a trim touches a tangent line without crossing it ({} off)",
            fm(*gap)
        ),
        NeedsRelief { .. } => "the wall keeps material beside the bend zone: it needs a relief there (cut one past the tangent line)".into(),
        TangentMismatch { gap, .. } => format!("a wall's edge meets the tangent line at a slant ({} off)", fm(*gap)),
        ArcSweep { sweep, .. } => format!("an arc sweeping {}", fm(*sweep)),
        FoldedClash { depth, .. } => format!("folded, two strips pass through each other ({} deep)", fm(*depth)),
        TrimCurveUnsupported { curve, .. } => format!("a {curve:?} trim this strip cannot carry"),
        WallFolds { .. } => "a side wall in a bend folds over".into(),
        EdgeTooShort { length, .. } => format!("a trim edge {} long", fm(*length)),
        ReliefTooShallow { depth, limit, .. } => format!(
            "a relief {} deep from the mold line; past the tangent line it needs more than {}",
            fm(*depth),
            fm(*limit)
        ),
        ReliefWidth { width, limit, .. } => format!("a relief {} wide; it needs {}", fm(*width), fm(*limit)),
        EdgeMoveBlocked { .. } => {
            "the flange's position moves its edge, and the edge beside it carries a bend".into()
        }
        EdgeMoveCurve { curve, .. } => {
            format!("the flange's position moves its edge, and the edge beside it is a {curve:?}")
        }
        CornerUnbuilt { size, .. } if size.is_nan() => {
            "a closed or overlapping corner is a seam: cut it with seam(a, b, kind, gap) or corner_relief(a, b, closed_corner), or give the corner a relief".into()
        }
        CornerUnbuilt { size, .. } => format!("a corner relief of size {}", fm(*size)),
        FlatOverlap { total, .. } => format!(
            "the flat pattern overlaps itself ({} mm2): a single blank cannot",
            fm(*total)
        ),
        FlatOverlapUndecided { gap, .. } => format!(
            "two strips of the flat come within {}: overlap or touch cannot be told",
            fm(*gap)
        ),
        HardwareOffWall { .. } => "the hardware's centre is off its wall (outside it, or in a hole)".into(),
        FlatTouch { .. } => "two strips of the flat pattern touch with no gap: the blank has no width there".into(),
        FoldedTouch { .. } => "folded, two walls touch with no gap (an edge on a face): no valid solid".into(),
        ReliefExtendUnbuilt { .. } => "the relief rule asks reliefs to extend for flat collisions: not built yet".into(),
        SheetSplit { .. } => "the cuts part the sheet in two".into(),
        _ => unreachable!("the features' own, above"),
    }
}

fn kernel_data(e: &SheetError, n: &Names) -> Vec<(&'static str, Json)> {
    use SheetError::*;
    let f = |x: f64| Json::Num(x);
    if let Some(d) = feat::kernel_data(e).or_else(|| feat::form_error_data(e, n)) {
        return d;
    }
    match e {
        Thickness { thickness, limit, .. } => vec![("thickness", f(*thickness)), ("limit", f(*limit))],
        Radius { radius, limit, .. } => vec![("radius", f(*radius)), ("limit", f(*limit))],
        Profile { wall, why, .. } => vec![("wall", n.wall(*wall).into()), ("why", format!("{why:?}").into())],
        NoSuchWall { wall, .. } => vec![("wall", n.wall(*wall).into())],
        NoSuchSegment { wall, segment, .. } => {
            vec![
                ("wall", n.wall(*wall).into()),
                ("segment", (*segment as f64).into()),
            ]
        }
        NotStraight { trim, curve, .. } => {
            vec![("trim", n.trim(*trim)), ("curve", format!("{curve:?}").into())]
        }
        EdgeTaken { trim, joint, .. } => vec![("trim", n.trim(*trim)), ("joint", n.joint(*joint).into())],
        BendAngle {
            joint, angle, limit, ..
        } => vec![
            ("joint", n.joint(*joint).into()),
            ("angle", f(angle.to_degrees())),
            ("limit", f(limit.to_degrees())),
        ],
        WallTooShort {
            wall,
            joint,
            length,
            limit,
            ..
        } => vec![
            ("wall", n.wall(*wall).into()),
            ("joint", n.joint(*joint).into()),
            ("length", f(*length)),
            ("limit", f(*limit)),
        ],
        JointLengths {
            joint, a, b, limit, ..
        } => vec![
            ("joint", n.joint(*joint).into()),
            ("a", f(*a)),
            ("b", f(*b)),
            ("limit", f(*limit)),
        ],
        JointWalls { joint, .. } => vec![("joint", n.joint(*joint).into())],
        WallUnplaced { wall, .. } => vec![("wall", n.wall(*wall).into())],
        JointKindUnbuilt { joint, kind, .. } => vec![
            ("joint", n.joint(*joint).into()),
            ("joint_kind", format!("{kind:?}").into()),
        ],
        BendRule { joint, why, .. } => {
            let mut v = vec![("joint", n.joint(*joint).into())];
            v.extend(match *why {
                fs::RuleError::KRange { k, lo, hi } => {
                    vec![
                        ("rule", "KRange".into()),
                        ("k", f(k)),
                        ("lo", f(lo)),
                        ("hi", f(hi)),
                    ]
                }
                fs::RuleError::OffTable { r_over_t, lo, hi } => vec![
                    ("rule", "OffTable".into()),
                    ("r_over_t", f(r_over_t)),
                    ("lo", f(lo)),
                    ("hi", f(hi)),
                ],
                fs::RuleError::NoRow {
                    radius,
                    angle,
                    nearest,
                } => vec![
                    ("rule", "NoRow".into()),
                    ("radius", f(radius)),
                    ("angle", f(angle.to_degrees())),
                    ("nearest", f(nearest)),
                ],
                fs::RuleError::TableOrder { row } => {
                    vec![("rule", "TableOrder".into()), ("row", f(row as f64))]
                }
                fs::RuleError::Allowance { ba } => vec![("rule", "Allowance".into()), ("ba", f(ba))],
            });
            v
        }
        Pinched { strip, pinches, .. } => vec![
            ("strip", n.strip(*strip)),
            (
                "pinches",
                Json::Arr(
                    pinches
                        .iter()
                        .map(|p| {
                            Json::obj(vec![
                                ("at", p2(p.at)),
                                ("a", n.trim(p.a)),
                                ("b", n.trim(p.b)),
                                ("width", f(p.width)),
                                ("limit", f(p.limit)),
                            ])
                        })
                        .collect(),
                ),
            ),
        ],
        NearCoincident {
            strip,
            a,
            b,
            at,
            gap,
            limit,
            ..
        } => vec![
            ("strip", n.strip(*strip)),
            ("a", n.trim(*a)),
            ("b", n.trim(*b)),
            ("at", p2(*at)),
            ("gap", f(*gap)),
            ("limit", f(*limit)),
        ],
        Unchained {
            strip,
            at,
            trim,
            leaving,
            arriving,
            ..
        } => vec![
            ("strip", n.strip(*strip)),
            ("at", p2(*at)),
            ("trim", n.trim(*trim)),
            ("leaving", f(*leaving as f64)),
            ("arriving", f(*arriving as f64)),
        ],
        HoleOutside { strip, trim, at, .. } => {
            vec![
                ("strip", n.strip(*strip)),
                ("trim", n.trim(*trim)),
                ("at", p2(*at)),
            ]
        }
        TrimAlongBendLine {
            joint,
            trim,
            from,
            to,
            ..
        } => vec![
            ("joint", n.joint(*joint).into()),
            ("trim", n.trim(*trim)),
            ("from", p2(*from)),
            ("to", p2(*to)),
        ],
        TrimTouchesBendLine {
            joint,
            trim,
            at,
            gap,
            limit,
            ..
        } => vec![
            ("joint", n.joint(*joint).into()),
            ("trim", n.trim(*trim)),
            ("at", p2(*at)),
            ("gap", f(*gap)),
            ("limit", f(*limit)),
        ],
        NeedsRelief {
            joint, wall, end, at, ..
        } => vec![
            ("joint", n.joint(*joint).into()),
            ("wall", n.wall(*wall).into()),
            ("end", if *end == fs::End::Start { "start" } else { "end" }.into()),
            ("at", p2(*at)),
        ],
        TangentMismatch {
            joint,
            side,
            at,
            gap,
            limit,
            ..
        } => vec![
            ("joint", n.joint(*joint).into()),
            ("side", if *side == fs::Side::A { "a" } else { "b" }.into()),
            ("at", p2(*at)),
            ("gap", f(*gap)),
            ("limit", f(*limit)),
        ],
        ArcSweep {
            trim, sweep, limit, ..
        } => vec![
            ("trim", n.trim(*trim)),
            ("sweep", f(*sweep)),
            ("limit", f(*limit)),
        ],
        FoldedClash {
            a,
            b,
            at,
            depth,
            limit,
            ..
        } => vec![
            ("a", n.strip(*a)),
            ("b", n.strip(*b)),
            ("at", p3(*at)),
            ("depth", f(*depth)),
            ("limit", f(*limit)),
        ],
        TrimCurveUnsupported {
            strip, trim, curve, ..
        } => vec![
            ("strip", n.strip(*strip)),
            ("trim", n.trim(*trim)),
            ("curve", format!("{curve:?}").into()),
        ],
        WallFolds { joint, trim, .. } => vec![("joint", n.joint(*joint).into()), ("trim", n.trim(*trim))],
        EdgeTooShort {
            strip,
            trim,
            length,
            limit,
            ..
        } => vec![
            ("strip", n.strip(*strip)),
            ("trim", n.trim(*trim)),
            ("length", f(*length)),
            ("limit", f(*limit)),
        ],
        ReliefTooShallow {
            joint,
            end,
            depth,
            limit,
            ..
        } => vec![
            ("joint", n.joint(*joint).into()),
            ("end", end_name(*end).into()),
            ("depth", f(*depth)),
            ("limit", f(*limit)),
        ],
        ReliefWidth {
            joint,
            end,
            width,
            limit,
            ..
        } => vec![
            ("joint", n.joint(*joint).into()),
            ("end", end_name(*end).into()),
            ("width", f(*width)),
            ("limit", f(*limit)),
        ],
        EdgeMoveBlocked { trim, joint, .. } => {
            vec![("trim", n.trim(*trim)), ("joint", n.joint(*joint).into())]
        }
        EdgeMoveCurve { trim, curve, .. } => {
            vec![("trim", n.trim(*trim)), ("curve", format!("{curve:?}").into())]
        }
        CornerUnbuilt { joints, size, .. } => vec![
            (
                "joints",
                Json::Arr(vec![n.joint(joints[0]).into(), n.joint(joints[1]).into()]),
            ),
            ("size", f(*size)),
        ],
        FlatOverlap {
            a,
            b,
            at,
            area,
            total,
            limit,
            ..
        } => vec![
            ("a", n.strip(*a)),
            ("b", n.strip(*b)),
            ("at", p2(*at)),
            ("area", f(*area)),
            ("total", f(*total)),
            ("limit", f(*limit)),
        ],
        FlatOverlapUndecided {
            a, b, at, gap, limit, ..
        } => vec![
            ("a", n.strip(*a)),
            ("b", n.strip(*b)),
            ("at", p2(*at)),
            ("gap", f(*gap)),
            ("limit", f(*limit)),
        ],
        ReliefExtendUnbuilt { joint, end, .. } => {
            vec![("joint", n.joint(*joint).into()), ("end", end_name(*end).into())]
        }
        FlatTouch { a, b, at, .. } => vec![("a", n.strip(*a)), ("b", n.strip(*b)), ("at", p2(*at))],
        HardwareOffWall { wall, at, .. } => vec![("wall", n.strip(Strip::Wall(*wall))), ("at", p2(*at))],
        FoldedTouch { a, b, at, .. } => vec![("a", n.strip(*a)), ("b", n.strip(*b)), ("at", p3(*at))],
        SheetSplit { a, b, .. } => vec![("a", n.strip(*a)), ("b", n.strip(*b))],
        _ => unreachable!("the features' own, above"),
    }
}

/// SetParam on the param that drives `n`, if one does.
fn set(n: &Num, value: f64) -> Option<Op> {
    let Src::Driving(p) = &n.src else { return None };
    let value = if n.dim == Dim::Angle {
        value.to_degrees()
    } else {
        value
    };
    Some(Op::SetParam {
        param: p.clone(),
        value: Literal { value, dim: n.dim },
    })
}

fn kernel_fixes(st: &FixCtx, e: &SheetError) -> Vec<Op> {
    use SheetError::*;
    let flange = |j: JointId| st.flanges.iter().find(|f| f.joint == j);
    let flange_of = |w: WallId| st.flanges.iter().find(|f| f.wall == w);
    let set_arg = |f: &FlangeDims, arg: &str, text: String| Op::SetArg {
        id: f.id.clone(),
        arg: arg.into(),
        text,
    };
    // the ones with more than one fix
    match e {
        NeedsRelief { joint, .. } | ReliefTooShallow { joint, .. } | ReliefWidth { joint, .. } => {
            return flange(*joint)
                .map(|f| {
                    st.relief_texts(f)
                        .into_iter()
                        .map(|t| set_arg(f, "relief", t))
                        .collect()
                })
                .unwrap_or_default();
        }
        EdgeMoveBlocked { .. } | EdgeMoveCurve { .. } => {
            // the flange being made is the last
            return st
                .flanges
                .last()
                .map(|f| vec![set_arg(f, "position", "\"outside\"".into())])
                .unwrap_or_default();
        }
        _ => {}
    }
    let out = match e {
        // a flange left a thickness past its setback
        WallTooShort { wall, length, .. } => {
            flange_of(*wall).and_then(|f| set(&f.height, f.height.v - length + st.t.v))
        }
        BendAngle { joint, .. } => flange(*joint).and_then(|f| set(&f.angle, dmath::PI / 2.0)),
        BendRule { why, .. } => match *why {
            fs::RuleError::KRange { k, lo, hi } => st.k.as_ref().and_then(|n| set(n, k.clamp(lo, hi))),
            fs::RuleError::NoRow { nearest, .. } if nearest.is_finite() => {
                set(radius_of(st, joint_of(e)), nearest)
            }
            _ => None,
        },
        _ => None,
    };
    out.into_iter().collect()
}

/// The number a joint's radius came from: its flange's own, or the sheet's.
fn radius_of(st: &FixCtx, joint: Option<JointId>) -> &Num {
    joint
        .and_then(|j| st.flanges.iter().find(|f| f.joint == j))
        .and_then(|f| f.radius.as_ref())
        .unwrap_or(&st.r)
}

fn joint_of(e: &SheetError) -> Option<JointId> {
    match e {
        SheetError::BendRule { joint, .. } => Some(*joint),
        _ => None,
    }
}

fn violation_fixes(st: &FixCtx, v: &fs::Violation) -> Vec<Op> {
    // a relief smaller than the rule's: the rule's own size, of each kind
    if let fs::Violation::ReliefUndersized { joint, .. } = *v {
        return st
            .flanges
            .iter()
            .find(|f| f.joint == joint)
            .map(|f| {
                st.relief_texts(f)
                    .into_iter()
                    .map(|text| Op::SetArg {
                        id: f.id.clone(),
                        arg: "relief".into(),
                        text,
                    })
                    .collect()
            })
            .unwrap_or_default();
    }
    let out = match *v {
        fs::Violation::ThicknessNotStocked { nearest, .. } if nearest.is_finite() => set(&st.t, nearest),
        fs::Violation::RadiusNotTooled { joint, nearest, .. } if nearest.is_finite() => {
            set(radius_of(st, Some(joint)), nearest)
        }
        fs::Violation::FlangeTooShort {
            wall, length, limit, ..
        } => st
            .flanges
            .iter()
            .find(|f| f.wall == wall)
            .and_then(|f| set(&f.height, f.height.v + (limit - length))),
        // the flange lowered to the limit, to a thousandth below
        fs::Violation::BoxFlangeTooTall {
            wall, height, limit, ..
        } => st.flanges.iter().find(|f| f.wall == wall).and_then(|f| {
            set(
                &f.height,
                ((f.height.v - (height - limit)) * 1000.0 + 1e-6).floor() / 1000.0,
            )
        }),
        // the angle into the brake's range
        fs::Violation::BendAngleOutOfRange {
            joint,
            angle,
            min,
            max,
            ..
        } => st.flanges.iter().find(|f| f.joint == joint).and_then(|f| {
            let to = angle.clamp(min, max);
            set(&f.angle, f.angle.v.signum() * to)
        }),
        _ => None,
    };
    out.into_iter().collect()
}

fn sheet_err(e: SheetErr) -> Error {
    Error::Sheet(Box::new(e))
}

fn kernel_error(fix: &FixCtx, names: Names, span: Span, id: &str, error: SheetError) -> Error {
    let fixes = kernel_fixes(fix, &error);
    sheet_err(SheetErr::Kernel {
        span,
        id: id.into(),
        error,
        names,
        fixes,
    })
}

fn kernel_err(st: &SheetState, span: Span, id: &str, error: SheetError) -> Error {
    let extra = match error {
        SheetError::FlatOverlap { a, b, .. } => overlap_fixes(st, a, b),
        SheetError::FoldedClash { a, b, .. } => clash_fixes(st, a, b),
        SheetError::FlatTouch { a, b, seam, .. } => {
            // before the step that is refused
            let mut out = seam_fixes(st, seam, Some(id.to_string()));
            out.extend(touch_fixes(st, a, b, Map::Flat));
            out
        }
        SheetError::FoldedTouch { a, b, .. } => touch_fixes(st, a, b, Map::Folded),
        _ => vec![],
    };
    match kernel_error(&st.fix, st.names(), span, id, error) {
        Error::Sheet(mut e) => {
            if let SheetErr::Kernel { fixes, .. } = &mut *e {
                fixes.extend(extra);
            }
            Error::Sheet(e)
        }
        o => o,
    }
}

/// Fixes for two strips passing through each other folded: the later flange stood outside its
/// edge, or the sheet's corners opened (the later wall trimmed back a thickness and the rule's
/// gap). Closed, overlapping and underlapping seams are another lane's.
fn clash_fixes(st: &SheetState, a: Strip, b: Strip) -> Vec<Op> {
    let owner = |s: Strip| {
        st.fix.flanges.iter().position(|f| match s {
            Strip::Wall(w) => f.wall == w,
            Strip::Bend(j) => f.joint == j,
        })
    };
    // each tried on the kernel first: the flanges shorter, the corner opened
    let mut out = height_fixes(st, a, b, Map::Folded, &|r| r.is_ok());
    let gap = st.fix.relief.corner.gap.at(st.fix.t.v, st.fix.r.v);
    let mut opened = st.sheet.clone();
    opened.corner_default = Some(fs::CornerKind::Open { gap });
    if owner(a).is_some() && owner(b).is_some() && fs::build_with(&opened, Map::Folded).is_ok() {
        out.push(Op::SetArg {
            id: st.id.clone(),
            arg: "corner".into(),
            text: format!("open_corner({})", crate::values::fmt(gap)),
        });
    }
    out
}

/// Fixes that part two strips touching in `map`, each tried on the kernel first: the corner
/// opened (the later wall trimmed back a thickness and the shop's gap), and the flange heights
/// that clear a flat.
fn touch_fixes(st: &SheetState, a: Strip, b: Strip, map: Map) -> Vec<Op> {
    let mut out = vec![];
    let gap = st.fix.relief.corner.gap.at(st.fix.t.v, st.fix.r.v);
    let mut s = st.sheet.clone();
    s.corner_default = Some(fs::CornerKind::Open { gap });
    if fs::build_with(&s, map).is_ok() {
        out.push(Op::SetArg {
            id: st.id.clone(),
            arg: "corner".into(),
            text: format!("open_corner({})", crate::values::fmt(gap)),
        });
    }
    if map == Map::Flat {
        out.extend(height_fixes(st, a, b, Map::Flat, &|r| r.is_ok()));
    }
    out
}

/// A slot along a flat touch's run, cut into the base (a relief along the edge the bend's
/// strip lies on), as wide as the rule's bend relief and a width past each end: offered when
/// the kernel builds the flat with it.
fn seam_fixes(st: &SheetState, seam: Option<fs::FlatSeam>, before: Option<String>) -> Vec<Op> {
    let Some(seam) = seam else { return vec![] };
    // the base's coordinates are its sketch's; other walls' are not a sketch's as written
    if seam.wall != WallId(0) {
        return vec![];
    }
    let g = st
        .fix
        .relief
        .bend
        .width
        .at(st.fix.t.v, st.fix.r.v)
        .max(st.fix.t.v);
    let len = seam.from.distance(seam.to);
    if len <= 0.0 {
        return vec![];
    }
    let d = (seam.to - seam.from) * (1.0 / len);
    let (p, q) = (seam.from - d * g, seam.to + d * g);
    let corners = [p, q, q + seam.into * g, p + seam.into * g];
    let lp = ProfileLoop::new(
        (0..4)
            .map(|i| {
                (
                    Segment::Line {
                        a: corners[i],
                        b: corners[(i + 1) % 4],
                    },
                    i as u64,
                )
            })
            .collect(),
    );
    let mut s = st.sheet.clone();
    // counter-clockwise for the kernel
    let area: f64 = (0..4)
        .map(|i| corners[i].to_vec().cross(corners[(i + 1) % 4].to_vec()))
        .sum();
    let lp = if area < 0.0 { lp.reversed() } else { lp };
    // it parts this touch: the flat builds, or what stops it is elsewhere
    let parted = s.cut(WallId(0), lp).is_ok()
        && match fs::build_with(&s, Map::Flat) {
            Ok(_) => true,
            Err(SheetError::FlatTouch { seam: Some(o), .. }) => {
                o.wall != seam.wall || o.from.distance(seam.from) > g
            }
            Err(SheetError::FlatTouch { .. }) => true,
            Err(_) => false,
        };
    if !parted {
        return vec![];
    }
    let used = |n: &str| st.cuts.iter().any(|c| c.1 == n) || st.joints.iter().any(|j| j.1 == n) || st.id == n;
    let name = (1..)
        .map(|k| {
            if k == 1 {
                "touch_slot".to_string()
            } else {
                format!("touch_slot{k}")
            }
        })
        .find(|n| !used(n) && !used(&format!("{n}_cut")) && !used(&format!("{n}_loop")))
        .expect("a free name");
    let fm = crate::values::fmt;
    let pts: Vec<String> = corners
        .iter()
        .map(|c| format!("({}, {})", fm(c.x), fm(c.y)))
        .collect();
    vec![Op::AddFeature {
        before: before.or_else(|| st.unfolds.first().map(|u| u.id.clone())),
        text: format!(
            "sketch {name}(on: {base}.top) {{\n  {name}_loop = poly([{}])\n}}\n{name}_cut = cutout({name}, wall: {base})",
            pts.join(", "),
            base = st.id
        ),
    }]
}

/// Shorter flanges that clear a flat overlap: an overlap cleared to a touch is cleared (the
/// touch is its own note, with its own fixes).
fn overlap_fixes(st: &SheetState, a: Strip, b: Strip) -> Vec<Op> {
    height_fixes(st, a, b, Map::Flat, &|r| {
        matches!(r, Ok(_) | Err(SheetError::FlatTouch { .. }))
    })
}

/// SetParam fixes: for each flange owning strip a or b, the tallest height (to a thousandth,
/// rounded down) at which the kernel's build in `map` passes `cleared`, found by halving.
fn height_fixes(
    st: &SheetState,
    a: Strip,
    b: Strip,
    map: Map,
    cleared: &dyn Fn(&Result<fs::Built, SheetError>) -> bool,
) -> Vec<Op> {
    let owner = |s: Strip| {
        st.fix.flanges.iter().find(|f| match s {
            Strip::Wall(w) => f.wall == w,
            Strip::Bend(j) => f.joint == j,
        })
    };
    let mut out = vec![];
    let mut seen = vec![];
    for f in [owner(a), owner(b)].into_iter().flatten() {
        if seen.contains(&f.wall) {
            continue;
        }
        seen.push(f.wall);
        // the tip (segment 1) at x = the kernel's height
        let Some(tip) = st.sheet.wall(f.wall).and_then(|w| {
            w.outline.iter().flat_map(|l| &l.segments).find_map(|g| match g {
                (Segment::Line { a, .. }, 1) => Some(a.x),
                _ => None,
            })
        }) else {
            continue;
        };
        let clear = |delta: f64| {
            let mut s = st.sheet.clone();
            s.move_edge(f.wall, 1, -delta).is_ok() && cleared(&fs::build_with(&s, map))
        };
        let most = tip - f.setback - 1e-3;
        if most <= 0.0 || !clear(most) {
            continue;
        }
        let (mut lo, mut hi) = (0.0, most);
        while hi - lo > 1e-5 {
            let mid = 0.5 * (lo + hi);
            if clear(mid) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        let value = ((f.height.v - hi) * 1000.0).floor() / 1000.0;
        if clear(f.height.v - value)
            && let Some(op) = set(&f.height, value)
        {
            out.push(op);
        }
    }
    out
}

// ---- refusing ordinary features ----------------------------------------------------------------

/// Called before an ordinary feature is pushed: refused in a part with a sheet.
pub fn guard(ev: &Eval, f: &Feature, id: &str, span: Span) -> R<()> {
    let param_like = matches!(
        f,
        Feature::Sketch(_) | Feature::Param(_) | Feature::Plane(_) | Feature::Work(_)
    );
    match ev.sheets.list.first() {
        Some(s) if !param_like => Err(sheet_err(SheetErr::OrdinaryOnSheet {
            span,
            id: id.into(),
            sheet: s.id.clone(),
        })),
        _ => Ok(()),
    }
}

// ---- calls -------------------------------------------------------------------------------------

fn wrong(span: Span, what: &'static str, want: Ty, v: &Value) -> Error {
    Error::WrongType {
        span,
        what,
        want,
        found: v.ty(),
    }
}

fn sv(v: SheetVal) -> Value {
    Value::Sheet(Rc::new(v))
}

pub fn call(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    if ev.sketch_ref().is_some() {
        return Err(Error::WrongPlace {
            span,
            func: name,
            needs_sketch: false,
        });
    }
    match name {
        "kfactor" => {
            let a = Args::bind(ev, name, &["k"], args, span)?;
            let k = a.req_num("k", RATIO)?;
            Ok(sv(SheetVal::Rule(Rc::new(RuleV {
                rule: fs::BendRule::K(k.v),
                k: Some(k),
            }))))
        }
        "ktable" => {
            let a = Args::bind(ev, name, &["rows"], args, span)?;
            let rows = table(&a, "rows", &[RATIO, RATIO])?;
            in_order(&rows, a.span("rows"), |r| (0.0, r[0]))?;
            Ok(sv(SheetVal::Rule(Rc::new(RuleV {
                rule: fs::BendRule::KTable(
                    rows.iter()
                        .map(|r| fs::KRow {
                            r_over_t: r[0],
                            k: r[1],
                        })
                        .collect(),
                ),
                k: None,
            }))))
        }
        "bend_table" => {
            let a = Args::bind(ev, name, &["rows"], args, span)?;
            let rows = table(&a, "rows", &[LEN, ANG, LEN])?;
            in_order(&rows, a.span("rows"), |r| (r[0], r[1]))?;
            Ok(sv(SheetVal::Rule(Rc::new(RuleV {
                rule: fs::BendRule::Allowance(
                    rows.iter()
                        .map(|r| fs::AllowanceRow {
                            radius: r[0],
                            angle: r[1],
                            ba: r[2],
                        })
                        .collect(),
                ),
                k: None,
            }))))
        }
        "per_t" => {
            let a = Args::bind(ev, name, &["f", "per_r", "plus"], args, span)?;
            let per_t = a.req_num("f", RATIO)?;
            let per_r = a.num("per_r", RATIO)?.unwrap_or(Num::konst(0.0, Dim::Ratio));
            let plus = a.num("plus", LEN)?.unwrap_or(Num::konst(0.0, Dim::Length));
            Ok(sv(SheetVal::Scaled(Rc::new(ScaledV { per_t, per_r, plus }))))
        }
        "stock" => stock(ev, args, span),
        "shop" => {
            let a = Args::bind(
                ev,
                name,
                &[
                    "stock",
                    "kerf",
                    "hardware",
                    "closed_hems",
                    "hem_gap",
                    "max_flat",
                    "max_flat_coated",
                    "max_bend_length",
                    "max_box_flange",
                    "box_flange_when",
                    "bend_angles",
                    "min_part",
                    "tools",
                    "punch_web",
                    "forms",
                    "form_bend",
                    "form_edge",
                    "form_web",
                    "form_depth",
                ],
                args,
                span,
            )?;
            let hems = feat::hem_table(&a)?;
            let kerf = a.num("kerf", LEN)?;
            // a (shorter, longer) or (least, greatest) pair, either way round
            let pair = |k: &'static str, dims: &'static [Dim], what: &'static str| -> R<Option<(f64, f64)>> {
                let Some(v) = a.opt(k) else { return Ok(None) };
                let Value::Tuple(xs) = &v else {
                    return Err(wrong(a.span(k), what, Ty::Vec2, &v));
                };
                let [x, y] = &xs[..] else {
                    return Err(wrong(a.span(k), what, Ty::Vec2, &v));
                };
                let x = num_of(x.clone(), dims, a.span(k), what)?.v;
                let y = num_of(y.clone(), dims, a.span(k), what)?.v;
                Ok(Some((x.min(y), x.max(y))))
            };
            let process = fs::Process {
                flat_max: pair("max_flat", LEN, "max_flat (width, length)")?,
                coated_max: pair("max_flat_coated", LEN, "max_flat_coated (width, length)")?,
                bend_length_max: a.num("max_bend_length", LEN)?.map(|n| n.v),
                box_flange_max: a.num("max_box_flange", LEN)?.map(|n| n.v),
                box_flange_with_hardware: word(&a, "box_flange_when", &["always", "hardware"])?
                    == Some("hardware"),
                angles: pair("bend_angles", ANG, "bend_angles (least, greatest)")?,
                part_min: pair("min_part", LEN, "min_part (width, length)")?,
            };
            let Value::List(xs) = a.req("stock")? else {
                return Err(wrong(a.span("stock"), "shop's stock", Ty::List, &a.req("stock")?));
            };
            let mut stocks = vec![];
            for x in xs.iter() {
                match x {
                    Value::Sheet(s) if matches!(**s, SheetVal::Stock(_)) => {
                        let SheetVal::Stock(st) = &**s else {
                            unreachable!("matched")
                        };
                        stocks.push(st.clone());
                    }
                    o => return Err(wrong(a.span("stock"), "a stock(..)", Ty::Stock, o)),
                }
            }
            Ok(sv(SheetVal::Shop(Rc::new(ShopV {
                shop: fs::ShopProfile {
                    stock: stocks.iter().map(|s| s.stock.clone()).collect(),
                    hardware: match a.opt("hardware") {
                        None => fs::placeholder_catalogue(),
                        Some(Value::List(rows)) => rows
                            .iter()
                            .map(|r| match r {
                                Value::Sheet(v) if matches!(**v, SheetVal::Hardware(_)) => {
                                    let SheetVal::Hardware(h) = &**v else {
                                        unreachable!("matched")
                                    };
                                    Ok((**h).clone())
                                }
                                o => Err(wrong(a.span("hardware"), "a hardware(..) row", Ty::Hardware, o)),
                            })
                            .collect::<R<_>>()?,
                        Some(o) => return Err(wrong(a.span("hardware"), "hardware", Ty::List, &o)),
                    },
                    kerf: kerf.map_or(0.2, |k| k.v),
                    process,
                    tools: feat::tools_of(&a)?,
                    punch_rules: feat::punch_rules_of(&a)?,
                    forms: feat::form_tools_of(&a)?,
                    form_rules: feat::form_rules_of(&a)?,
                },
                stocks,
                hems,
            }))))
        }
        "sheet" => base(ev, args, span),
        "flange" => flange(ev, args, span),
        "cutout" => cutout(ev, args, span),
        "unfold" => unfold(ev, args, span),
        "contour" => feat::contour(ev, args, span),
        "hem" => feat::hem(ev, args, span),
        "rect" | "obround" => {
            let a = Args::bind(ev, name, &["width", "depth"], args, span)?;
            let width = a.num("width", LEN)?;
            let depth = a.num("depth", LEN)?;
            let kind = if name == "rect" {
                fs::ReliefKind::Rect
            } else {
                fs::ReliefKind::Obround
            };
            Ok(sv(SheetVal::Relief(Rc::new(ReliefV {
                kind: Some(kind),
                width,
                depth,
            }))))
        }
        "circular_relief" | "square_relief" => {
            let a = Args::bind(ev, name, &["size", "placement"], args, span)?;
            let size = a.num("size", LEN)?.map(|n| n.v);
            let placement =
                match word(&a, "placement", &["intersection", "tangent"])?.unwrap_or("intersection") {
                    "tangent" => fs::Placement::Tangent,
                    _ => fs::Placement::Intersection,
                };
            let kind = if name == "circular_relief" {
                fs::CornerKind::Circular { size, placement }
            } else {
                fs::CornerKind::Square { size, placement }
            };
            Ok(sv(SheetVal::Corner(Rc::new(CornerV { kind }))))
        }
        "open_corner" | "overlap_corner" => {
            let a = Args::bind(ev, name, &["gap"], args, span)?;
            let gap = a.req_num("gap", LEN)?.v;
            let kind = if name == "open_corner" {
                fs::CornerKind::Open { gap }
            } else {
                fs::CornerKind::Overlap { gap }
            };
            Ok(sv(SheetVal::Corner(Rc::new(CornerV { kind }))))
        }
        "corner_relief" => corner(ev, args, span),
        "constant_width_corner" => {
            let a = Args::bind(ev, name, &["width"], args, span)?;
            let width = a.num("width", LEN)?.map(|n| n.v);
            Ok(sv(SheetVal::Corner(Rc::new(CornerV {
                kind: fs::CornerKind::ConstantWidth { width },
            }))))
        }
        "seam" => feat::seam(ev, args, span),
        "punch" => feat::punch(ev, args, span),
        "form" => feat::form(ev, args, span),
        "sketched_bend" => feat::sketched_bend(ev, args, span, false),
        "jog" => feat::sketched_bend(ev, args, span, true),
        "hardware" => {
            let a = Args::bind(
                ev,
                name,
                &[
                    "kind",
                    "size",
                    "sheet",
                    "hole",
                    "edge",
                    "max_hardness",
                    "code",
                    "source",
                ],
                args,
                span,
            )?;
            let kind = word(&a, "kind", &["nut", "stud", "standoff"])?.ok_or(Error::MissingArg {
                span,
                func: "hardware",
                arg: "kind",
            })?;
            let text = |k: &'static str| -> R<Option<String>> {
                match a.opt(k) {
                    None => Ok(None),
                    Some(Value::Str(x)) => Ok(Some(x.to_string())),
                    Some(o) => Err(wrong(a.span(k), k, Ty::Str, &o)),
                }
            };
            let size = text("size")?.ok_or(Error::MissingArg {
                span,
                func: "hardware",
                arg: "size",
            })?;
            let range = a.req("sheet")?;
            let Value::Tuple(mm) = &range else {
                return Err(wrong(a.span("sheet"), "sheet (min, max)", Ty::Vec2, &range));
            };
            let [lo, hi] = &mm[..] else {
                return Err(wrong(a.span("sheet"), "sheet (min, max)", Ty::Vec2, &range));
            };
            let lo = num_of(lo.clone(), LEN, a.span("sheet"), "the thinnest sheet")?;
            let hi = num_of(hi.clone(), LEN, a.span("sheet"), "the thickest sheet")?;
            Ok(sv(SheetVal::Hardware(Rc::new(fs::Hardware {
                kind: fs::HardwareKind::from_name(kind).expect("a checked word"),
                size,
                code: text("code")?.unwrap_or_default(),
                sheet_min: lo.v,
                sheet_max: hi.v,
                hole: a.req_num("hole", LEN)?.v,
                edge: a.req_num("edge", LEN)?.v,
                max_hardness: a.req_num("max_hardness", RATIO)?.v,
                source: text("source")?.map_or(fs::Source::Placeholder, fs::Source::Cited),
            }))))
        }
        "pem" => pem(ev, args, span),
        "relief_rule" => relief_rule(ev, args, span),
        _ => unreachable!("only sheet built-ins"),
    }
}

/// A table's rows in increasing order of their key within each group (a radius's angles; the
/// r/t of a K table), as the kernel interpolates them: refused at the table, not at the first
/// bend that reads it.
fn in_order(rows: &[Vec<f64>], span: Span, key: impl Fn(&[f64]) -> (f64, f64)) -> R<()> {
    for (i, r) in rows.iter().enumerate() {
        let (g, x) = key(r);
        if let Some((j, _)) = rows[..i]
            .iter()
            .enumerate()
            .rev()
            .find(|(_, q)| (key(q).0 - g).abs() <= 1e-9)
            && key(&rows[j]).1 >= x
        {
            return Err(sheet_err(SheetErr::TableOrder {
                span,
                row: i,
                after: j,
            }));
        }
    }
    Ok(())
}

/// A list of tuples, each `dims.len()` numbers of those dimensions.
fn table(a: &Args, k: &'static str, dims: &[&[Dim]]) -> R<Vec<Vec<f64>>> {
    let span = a.span(k);
    let v = a.req(k)?;
    let Value::List(rows) = &v else {
        return Err(wrong(span, "a table", Ty::List, &v));
    };
    let mut out = vec![];
    for (i, r) in rows.iter().enumerate() {
        let Value::Tuple(xs) = r else {
            return Err(wrong(span, "a table row", Ty::Tuple, r));
        };
        if xs.len() != dims.len() {
            return Err(sheet_err(SheetErr::TableRow {
                span,
                row: i,
                want: dims.len(),
            }));
        }
        let mut row = vec![];
        for (x, d) in xs.iter().zip(dims) {
            row.push(num_of(x.clone(), d, span, "a table value")?.v);
        }
        out.push(row);
    }
    Ok(out)
}

fn scaled_of(a: &Args, k: &'static str) -> R<fs::Scaled> {
    match a.opt(k) {
        None => Ok(fs::Scaled::NONE),
        Some(Value::Sheet(s)) if matches!(*s, SheetVal::Scaled(_)) => {
            let SheetVal::Scaled(x) = &*s else {
                unreachable!("matched")
            };
            Ok(fs::Scaled {
                per_t: x.per_t.v,
                per_r: x.per_r.v,
                plus: x.plus.v,
            })
        }
        Some(o) => Err(wrong(a.span(k), k, Ty::Scaled, &o)),
    }
}

fn rule_of(a: &Args, k: &'static str) -> R<Option<Rc<RuleV>>> {
    match a.opt(k) {
        None => Ok(None),
        Some(Value::Sheet(s)) if matches!(*s, SheetVal::Rule(_)) => {
            let SheetVal::Rule(r) = &*s else {
                unreachable!("matched")
            };
            Ok(Some(r.clone()))
        }
        Some(o) => Err(wrong(a.span(k), k, Ty::BendRule, &o)),
    }
}

fn stock(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let a = Args::bind(
        ev,
        "stock",
        &[
            "t",
            "bend",
            "radii",
            "min_flange",
            "hole_to_bend",
            "hole_to_edge",
            "min_slot",
            "min_hole",
            "hardness",
            "relief",
        ],
        args,
        span,
    )?;
    let t = a.req_num("t", LEN)?;
    let rule = rule_of(&a, "bend")?.ok_or(Error::MissingArg {
        span,
        func: "stock",
        arg: "bend",
    })?;
    let rv = a.req("radii")?;
    let Value::List(rs) = &rv else {
        return Err(wrong(a.span("radii"), "radii", Ty::List, &rv));
    };
    let radii: Vec<Num> = rs
        .iter()
        .map(|r| num_of(r.clone(), LEN, a.span("radii"), "a radius"))
        .collect::<R<_>>()?;
    if radii.is_empty() {
        return Err(Error::BadValue {
            span: a.span("radii"),
            what: "a stock's radii (none)",
            value: 0.0,
        });
    }
    let stock = fs::Stock {
        thickness: t.v,
        rule: rule.rule.clone(),
        radii: radii.iter().map(|r| r.v).collect(),
        min_flange: scaled_of(&a, "min_flange")?,
        hole_to_bend: scaled_of(&a, "hole_to_bend")?,
        hole_to_edge: scaled_of(&a, "hole_to_edge")?,
        min_slot: scaled_of(&a, "min_slot")?,
        min_hole: scaled_of(&a, "min_hole")?,
        relief: match a.opt("relief") {
            None => fs::ReliefRule::default(),
            Some(Value::Sheet(v)) if matches!(*v, SheetVal::Rule2(_)) => {
                let SheetVal::Rule2(r) = &*v else {
                    unreachable!("matched")
                };
                **r
            }
            Some(o) => {
                return Err(wrong(
                    a.span("relief"),
                    "relief: a relief_rule(..)",
                    Ty::ReliefRule,
                    &o,
                ));
            }
        },
        hardness: a.num("hardness", RATIO)?.map(|n| n.v),
    };
    Ok(sv(SheetVal::Stock(Rc::new(StockV {
        stock,
        t,
        radii,
        rule,
    }))))
}

/// The placeholder the history keeps for a step built here: equal exactly when the step is.
fn marker(id: &str, what: String) -> Feature {
    Feature::Param(Box::new(forge_model::Param {
        name: format!("sheet:{id}"),
        expr: what,
        unit: forge_model::Unit::None,
        value: 0.0,
    }))
}

/// The bodies after a sheet step: `body` in place of solid `id` (or added).
fn with_body(before: &[Solid], id: usize, body: Body) -> Vec<Solid> {
    let mut out: Vec<Solid> = before.to_vec();
    match out.iter_mut().find(|s| s.id == id) {
        Some(s) => s.body = body,
        None => out.push(Solid { id, body }),
    }
    out
}

/// Pushes sheet feature `id` and builds sheet `si` folded onto the part.
fn push_sheet_step(
    ev: &mut Eval,
    si: usize,
    id: &str,
    kind: &'static str,
    span: Span,
    dims: Vec<Dimension>,
) -> R<usize> {
    let st = &ev.sheets.list[si];
    let text = format!("{:?}", st.sheet);
    let marker = marker(id, text.clone());
    let sheet = st.sheet.clone();
    let solid = st.idx;
    let snapshot = st.clone();
    let hid = history_id(id);
    let mut made: Option<fs::Built> = None;
    let r = ev.push_made(id, kind, span, dims, hid, |b: &mut Builder| {
        b.push_with(marker, hid, |before: &[Solid]| {
            let built =
                fs::build_with(&sheet, Map::Folded).map_err(|e| kernel_err(&snapshot, span, id, e))?;
            let out = with_body(before, solid, built.body.clone());
            made = Some(built);
            Ok(out)
        })
    });
    let (idx, _) = r?;
    let built = ev.b.building && ev.feats.last().is_some_and(|f| f.status == Status::Ok);
    let key = format!("folded {text}");
    ev.sheets.list[si].folded = match (made, built) {
        (Some(b), _) => ev.b.memo(key, || Some(b)),
        // the stage came from the cache: the build that made it, or the roles from the graph again
        (None, true) => {
            let t = Instant::now();
            let b = ev.b.memo(key, || fs::build_with(&sheet, Map::Folded).ok());
            ev.b.kernel_ms += t.elapsed().as_secs_f64() * 1e3;
            b
        }
        (None, false) => None,
    };
    Ok(idx)
}

fn op_value(si: usize, id: &str, kind: OpKind) -> Value {
    sv(SheetVal::Op(Rc::new(OpRef {
        sheet: si,
        id: id.into(),
        kind,
        on: On::Folded,
    })))
}

fn base(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let id = ev.take_id("sheet", span)?;
    let a = Args::bind(
        ev,
        "sheet",
        &[
            "profile", "t", "r", "bend", "shop", "corner", "material", "finish",
        ],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    // what the drawing's part note names the stock as, and its density: a library material
    let material = match a.opt("material") {
        None => None,
        Some(Value::Str(m)) => {
            let at = crate::props::MaterialAt::Sheet { id: id.clone() };
            crate::props::material(ev, &m, a.span("material"), at)?;
            Some(m.to_string())
        }
        Some(o) => return Err(wrong(a.span("material"), "material", Ty::Str, &o)),
    };
    // a sheet part holds sheets only
    let makes = |f: &crate::eval::FeatInfo| {
        !matches!(
            ev.b.features.get(f.idx),
            None | Some(Feature::Sketch(_) | Feature::Param(_) | Feature::Plane(_) | Feature::Work(_))
        )
    };
    if let Some(f) = ev.feats.iter().find(|f| makes(f) && f.status != Status::Failed) {
        return Err(sheet_err(SheetErr::SheetOnSolid {
            span,
            id,
            solid: f.id.clone(),
        }));
    }
    let (sk, profile, _) = profile_of(ev, a.req("profile")?, a.span("profile"))?;
    let t = a.req_num("t", LEN)?;
    let shop = match a.opt("shop") {
        None => None,
        Some(Value::Sheet(s)) if matches!(*s, SheetVal::Shop(_)) => {
            let SheetVal::Shop(x) = &*s else {
                unreachable!("matched")
            };
            Some(x.clone())
        }
        Some(o) => return Err(wrong(a.span("shop"), "shop", Ty::Shop, &o)),
    };
    let stock = shop.as_ref().and_then(|s| {
        s.stocks
            .iter()
            .find(|x| (x.stock.thickness - t.v).abs() <= 1e-9 * t.v.abs().max(1.0))
            .cloned()
    });
    let r = match (a.num("r", LEN)?, &stock) {
        (Some(r), _) => r,
        (None, Some(s)) => s.radii[0].clone(),
        (None, None) => {
            return Err(Error::MissingArg {
                span,
                func: "sheet",
                arg: "r",
            });
        }
    };
    let rule = match (rule_of(&a, "bend")?, &stock) {
        (Some(r), _) => r,
        (None, Some(s)) => s.rule.clone(),
        (None, None) => {
            return Err(Error::MissingArg {
                span,
                func: "sheet",
                arg: "bend",
            });
        }
    };
    let mut dims = vec![dim("thickness", &t), dim("radius", &r)];
    if let Some(k) = &rule.k {
        dims.push(dim("k", k));
    }
    let hid = history_id(&id);
    let gauge = fs::Gauge {
        thickness: t.v,
        radius: r.v,
        rule: rule.rule.clone(),
    };
    let frame = sk.borrow().frame;
    let si = ev.sheets.list.len();
    // reliefs from the shop's stock of this thickness, else the stand-in default
    let relief = shop
        .as_ref()
        .and_then(|s| s.shop.relief_rule(t.v))
        .unwrap_or_default();
    let corner = match a.opt("corner") {
        None => None,
        Some(Value::Sheet(v)) if matches!(*v, SheetVal::Corner(_)) => {
            let SheetVal::Corner(c) = &*v else {
                unreachable!("matched")
            };
            Some(c.kind)
        }
        Some(o) => return Err(wrong(a.span("corner"), "corner", Ty::CornerKind, &o)),
    };
    let coated = word(&a, "finish", &["raw", "powder_coat"])? == Some("powder_coat");
    let fix = FixCtx {
        flanges: vec![],
        t: t.clone(),
        r: r.clone(),
        k: rule.k.clone(),
        relief,
    };
    let sheet = match fs::Sheet::face(fs::FeatureId(hid), gauge, frame, &profile.loops) {
        Ok((mut s, _)) => {
            s.rule = relief;
            s.corner_default = corner;
            s.coated = coated;
            s
        }
        Err(error) => {
            // reported on the feature; the sheet stays out of the part
            let names = Names {
                walls: vec![(WallId(0), id.clone())],
                ..Names::default()
            };
            return Err(kernel_error(&fix, names, span, &id, error));
        }
    };
    let st = SheetState {
        id: id.clone(),
        hid,
        idx: ev.b.features.len(),
        sheet,
        sketch: sk.clone(),
        shop,
        walls: vec![(WallId(0), id.clone(), span)],
        joints: vec![],
        cuts: vec![],
        corners: vec![],
        forms: vec![],
        cut_srcs: vec![],
        fix,
        folded: None,
        unfolds: vec![],
        material,
    };
    ev.sheets.list.push(st);
    push_sheet_step(ev, si, &id, "sheet", span, dims)?;
    Ok(op_value(si, &id, OpKind::Base { wall: WallId(0) }))
}

/// `corner_relief(a, b, kind)`: the treatment where flanges a and b, off one wall, meet.
fn corner(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let id = ev.take_id("corner_relief", span)?;
    let a = Args::bind(ev, "corner_relief", &["a", "b", "kind"], args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let joint = |k: &'static str| -> R<(usize, JointId)> {
        let v = a.req(k)?;
        let op = op_of(&v, a.span(k), "a flange")?;
        match op.kind {
            OpKind::Flange { joint, .. } if op.on == On::Folded => Ok((op.sheet, joint)),
            _ => Err(wrong(a.span(k), "a flange", Ty::Sheet, &v)),
        }
    };
    let ((si, ja), (sb, jb)) = (joint("a")?, joint("b")?);
    if si != sb {
        return Err(sheet_err(SheetErr::NotACorner { span, id }));
    }
    let kv = a.req("kind")?;
    let kind = match &kv {
        Value::Sheet(v) if matches!(**v, SheetVal::Corner(_)) => {
            let SheetVal::Corner(c) = &**v else {
                unreachable!("matched")
            };
            c.kind
        }
        o => return Err(wrong(a.span("kind"), "kind", Ty::CornerKind, o)),
    };
    not_unfolded(ev, si, &id, span)?;
    let st = &mut ev.sheets.list[si];
    // closed and overlapping corners are seams: the flanges' ends cut to meet
    // (`sheet_feat::seam`); a closed one keeps the shop's kerf apart
    let seam = match kind {
        fs::CornerKind::Closed => Some(fs::Seam::Closed {
            gap: st.sheet.rule.kerf.max(fs::GAP_FLOOR),
        }),
        fs::CornerKind::Overlap { gap } => Some(fs::Seam::Overlap { gap }),
        _ => None,
    };
    match seam {
        Some(seam) => {
            if let Err(e) = st.sheet.seam(ja, jb, seam) {
                return Err(kernel_err(st, span, &id, e));
            }
        }
        None => st.sheet.set_corner(ja, jb, kind),
    }
    st.corners.push((ja, jb, id.clone(), span));
    let mut dims = vec![];
    if let fs::CornerKind::Circular { size: Some(_), .. } | fs::CornerKind::Square { size: Some(_), .. } =
        kind
        && let Some(n) = a.num("kind", LEN).ok().flatten()
    {
        dims.push(dim("size", &n));
    }
    push_sheet_step(ev, si, &id, "corner_relief", span, dims)?;
    // the two must meet at a corner of their wall
    let st = &ev.sheets.list[si];
    if let Some(b) = &st.folded
        && !b
            .corners
            .iter()
            .any(|c| c.joints == [ja, jb] || c.joints == [jb, ja])
    {
        return Err(sheet_err(SheetErr::NotACorner { span, id }));
    }
    Ok(op_value(si, &id, OpKind::Corner { a: ja, b: jb }))
}

/// `pem(kind, size, at: POINT | CIRCLE, wall: W)`: a catalogue part pressed into wall W at a
/// point sketched on its top or bottom (a circle: its centre). The shop's catalogue, else the
/// placeholder one.
fn pem(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let id = ev.take_id("pem", span)?;
    let a = Args::bind(ev, "pem", &["kind", "size", "at", "wall"], args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let kind = word(&a, "kind", &["nut", "stud", "standoff"])?.ok_or(Error::MissingArg {
        span,
        func: "pem",
        arg: "kind",
    })?;
    let size = match a.req("size")? {
        Value::Str(x) => x.to_string(),
        o => return Err(wrong(a.span("size"), "size", Ty::Str, &o)),
    };
    let wv = a.req("wall")?;
    let op = op_of(&wv, a.span("wall"), "the wall the part goes in")?;
    let wall = match op.kind {
        OpKind::Base { wall } | OpKind::Flange { wall, .. } if op.on == On::Folded => wall,
        _ => {
            return Err(wrong(
                a.span("wall"),
                "a base or flange of the folded sheet",
                Ty::Sheet,
                &wv,
            ));
        }
    };
    let si = op.sheet;
    not_unfolded(ev, si, &id, span)?;
    let Value::Ent(e) = a.req("at")? else {
        return Err(wrong(
            a.span("at"),
            "at: a sketch point or circle",
            Ty::Entity,
            &a.req("at")?,
        ));
    };
    let st = &ev.sheets.list[si];
    let catalogue = st
        .shop
        .as_ref()
        .map_or_else(fs::placeholder_catalogue, |s| s.shop.hardware.clone());
    let want = fs::HardwareKind::from_name(kind).expect("a checked word");
    let Some(part) = catalogue
        .iter()
        .find(|h| h.kind == want && h.size == size)
        .cloned()
    else {
        return Err(sheet_err(SheetErr::NoSuchHardware {
            span,
            kind: kind.into(),
            size,
            sizes: catalogue
                .iter()
                .filter(|h| h.kind == want)
                .map(|h| h.size.clone())
                .collect(),
        }));
    };
    let Some(frame) = wall_frame(st, wall, None) else {
        push_sheet_step(ev, si, &id, "pem", span, vec![])?;
        return Ok(op_value(si, &id, OpKind::Cut { cut: CutId(u32::MAX) }));
    };
    let (from, centre) = {
        let sk = e.sk.borrow();
        let c = match sk.s.entity(e.id) {
            Some(forge_sketch::Entity::Circle { center, .. }) => sk.pos(center),
            _ => sk.pos(e.id),
        };
        (sk.frame, c)
    };
    let world = from.point_to_world(Point3::new(centre.x, centre.y, 0.0));
    let local = frame.point_to_local(world);
    let t = st.sheet.gauge.thickness;
    if local.z.abs().min((local.z + t).abs()) > 1e-7 {
        return Err(sheet_err(SheetErr::CutOffWall {
            span,
            wall: op.id.clone(),
            distance: local.z.abs().min((local.z + t).abs()),
        }));
    }
    let on_skin = frame.with_origin(frame.point_to_world(Point3::new(0.0, 0.0, local.z)));
    let st = &mut ev.sheets.list[si];
    let cut = match st.sheet.mount(wall, Point2::new(local.x, local.y), part) {
        Ok(c) => c,
        Err(err) => {
            let mut snap = st.clone();
            snap.cuts.push((CutId(snap.cuts.len() as u32), id.clone(), span));
            return Err(kernel_err(&snap, span, &id, err));
        }
    };
    st.cuts.push((cut, id.clone(), span));
    st.cut_srcs.push(CutSrc {
        cut,
        sketch: e.sk.clone(),
        from,
        to: on_skin,
        at: Some(e.id),
    });
    push_sheet_step(ev, si, &id, "pem", span, vec![])?;
    Ok(op_value(si, &id, OpKind::Cut { cut }))
}

/// A size: a Length (sized, mm) or per_t(..) (scaled).
fn size_of(a: &Args, k: &'static str) -> R<Option<fs::Scaled>> {
    match a.opt(k) {
        None => Ok(None),
        Some(Value::Num(n)) => {
            let n = num_of(Value::Num(n), LEN, a.span(k), k)?;
            Ok(Some(fs::Scaled {
                per_t: 0.0,
                per_r: 0.0,
                plus: n.v,
            }))
        }
        Some(_) => scaled_of(a, k).map(Some),
    }
}

/// `relief_rule(..)`: a shop's relief rule, every part as data, the rest the rules of thumb.
fn relief_rule(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let a = Args::bind(
        ev,
        "relief_rule",
        &[
            "shape",
            "width",
            "depth",
            "from",
            "min_remnant",
            "extend",
            "corner",
            "corner_placement",
            "corner_size",
            "corner_gap",
            "corner3",
            "corner3_radius",
        ],
        args,
        span,
    )?;
    let mut r = fs::ReliefRule::default();
    if let Some(w) = word(&a, "shape", &["rect", "obround", "tear"])? {
        r.bend.kind = match w {
            "obround" => fs::ReliefKind::Obround,
            "tear" => fs::ReliefKind::Tear,
            _ => fs::ReliefKind::Rect,
        };
    }
    if let Some(x) = size_of(&a, "width")? {
        r.bend.width = x;
    }
    if let Some(x) = size_of(&a, "depth")? {
        r.bend.depth = x;
    }
    if let Some(w) = word(&a, "from", &["bend_line", "up_to_bend", "tangent_to_bend"])? {
        r.bend.reference = match w {
            "up_to_bend" => fs::DepthRef::UpToBend,
            "tangent_to_bend" => fs::DepthRef::TangentToBend,
            _ => fs::DepthRef::BendLine,
        };
    }
    if let Some(x) = size_of(&a, "min_remnant")? {
        r.bend.min_remnant = x;
    }
    r.bend.extend = a.flag("extend")?;
    if let Some(w) = word(
        &a,
        "corner",
        &[
            "round",
            "square",
            "tear",
            "trim_to_bend",
            "closed",
            "constant_width",
        ],
    )? {
        r.corner.shape = match w {
            "square" => fs::CornerShape::Square,
            "tear" => fs::CornerShape::Tear,
            "trim_to_bend" => fs::CornerShape::TrimToBend,
            "closed" => fs::CornerShape::Closed,
            "constant_width" => fs::CornerShape::ConstantWidth,
            _ => fs::CornerShape::Round,
        };
    }
    if let Some(w) = word(&a, "corner_placement", &["intersection", "tangent"])? {
        r.corner.placement = if w == "tangent" {
            fs::Placement::Tangent
        } else {
            fs::Placement::Intersection
        };
    }
    r.corner.size = size_of(&a, "corner_size")?;
    if let Some(x) = size_of(&a, "corner_gap")? {
        r.corner.gap = x;
    }
    if let Some(w) = word(&a, "corner3", &["none", "intersection", "full_round", "round"])? {
        r.corner3.shape = match w {
            "intersection" => fs::Corner3Shape::Intersection,
            "full_round" => fs::Corner3Shape::FullRound,
            "round" => fs::Corner3Shape::Round,
            _ => fs::Corner3Shape::None,
        };
    }
    if let Some(x) = size_of(&a, "corner3_radius")? {
        r.corner3.radius = x;
    }
    Ok(sv(SheetVal::Rule2(Rc::new(r))))
}

/// `pattern(of: CUTOUT, layout: ..)` on a sheet: each copy a cut in the cutout's wall, the
/// layout's motions carried into the wall's own plane (a copy off that plane is
/// `PatternOffWall`). Path and point layouts are read the same way, from the part as it stands.
pub fn pattern(
    ev: &mut Eval,
    id: String,
    of: Value,
    of_span: Span,
    layout: &forge_model::Layout,
    dims: Vec<Dimension>,
    span: Span,
) -> R<Value> {
    let op = op_of(&of, of_span, "a sheet cutout")?;
    let OpKind::Cut { .. } = op.kind else {
        return Err(sheet_err(SheetErr::PatternOfSheet {
            span: of_span,
            op: op.id.clone(),
        }));
    };
    let si = op.sheet;
    not_unfolded(ev, si, &id, span)?;
    let places =
        forge_model::pattern::placements(ev.b.history(), ev.b.features.len(), layout).map_err(|error| {
            Error::Build {
                span,
                id: id.clone(),
                error,
            }
        })?;
    let st = &ev.sheets.list[si];
    // the source's cuts, each with its wall's frame
    let sources: Vec<(WallId, ProfileLoop)> = st
        .cuts
        .iter()
        .filter(|c| c.1 == op.id)
        .filter_map(|c| st.sheet.cuts.iter().find(|x| x.id == c.0))
        .map(|x| (x.wall, x.lp.clone()))
        .collect();
    let mut made = vec![];
    for (wall, lp) in sources {
        let Some(frame) = wall_frame(&ev.sheets.list[si], wall, None) else {
            continue;
        };
        for [cx, cy, cz, at] in &places {
            let m = |p: Point3| *at + cx.to_vec() * p.x + cy.to_vec() * p.y + cz.to_vec() * p.z;
            let mv = |v: Vec3| cx.to_vec() * v.x + cy.to_vec() * v.y + cz.to_vec() * v.z;
            // the copy's plane: the wall's, moved
            let o = m(frame.origin);
            let (x, y) = (mv(frame.x().get()), mv(frame.y().get()));
            let off = frame.point_to_local(o).z.abs();
            let tilt = frame.vec_to_local(x).z.abs().max(frame.vec_to_local(y).z.abs());
            if off > 1e-7 || tilt > 1e-9 {
                return Err(sheet_err(SheetErr::PatternOffWall { span, distance: off }));
            }
            let z = x.cross(y);
            let moved = Frame::from_axes(o, x, y, z).ok_or(Error::BadValue {
                span,
                what: "a pattern's motion",
                value: f64::NAN,
            })?;
            let lp2 = carried(&lp, &moved, &frame).ok_or(Error::BadValue {
                span,
                what: "a spline carried by the pattern",
                value: f64::NAN,
            })?;
            let st = &mut ev.sheets.list[si];
            match st.sheet.cut(wall, lp2) {
                Ok(c) => made.push(c),
                Err(e) => {
                    let mut snap = st.clone();
                    snap.cuts.push((CutId(snap.cuts.len() as u32), id.clone(), span));
                    return Err(kernel_err(&snap, span, &id, e));
                }
            }
        }
    }
    let st = &mut ev.sheets.list[si];
    for &c in &made {
        st.cuts.push((c, id.clone(), span));
    }
    push_sheet_step(ev, si, &id, "pattern", span, dims)?;
    let first = made.first().copied().unwrap_or(CutId(u32::MAX));
    Ok(op_value(si, &id, OpKind::Cut { cut: first }))
}

fn dim(what: &'static str, n: &Num) -> Dimension {
    Dimension { what, num: n.clone() }
}

fn op_of(v: &Value, span: Span, what: &'static str) -> R<Rc<OpRef>> {
    match v {
        Value::Sheet(s) => match &**s {
            SheetVal::Op(o) => Ok(o.clone()),
            _ => Err(wrong(span, what, Ty::Sheet, v)),
        },
        o => Err(wrong(span, what, Ty::Sheet, o)),
    }
}

/// Refuses a sheet feature on a sheet already unfolded.
fn not_unfolded(ev: &Eval, si: usize, id: &str, span: Span) -> R<()> {
    match ev.sheets.list[si].unfolds.first() {
        Some(u) => Err(sheet_err(SheetErr::AfterUnfold {
            span,
            id: id.into(),
            unfold: u.id.clone(),
        })),
        None => Ok(()),
    }
}

fn flange(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let id = ev.take_id("flange", span)?;
    let a = Args::bind(
        ev,
        "flange",
        &[
            "edge", "height", "angle", "side", "measure", "r", "relief", "position",
        ],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let relief = match a.opt("relief") {
        None => None,
        Some(Value::Sheet(v)) if matches!(*v, SheetVal::Relief(_)) => {
            let SheetVal::Relief(r) = &*v else {
                unreachable!("matched")
            };
            Some(r.clone())
        }
        Some(o) => return Err(wrong(a.span("relief"), "relief", Ty::Relief, &o)),
    };
    // inside by default: Inventor's documented default ("Inside of base face extents"), and
    // Fusion's dialog's by tutorials; SolidWorks' and Onshape's help state none
    let position = match word(&a, "position", &["outside", "inside", "bend_outside"])?.unwrap_or("inside") {
        "inside" => fs::Position::Inside,
        "bend_outside" => fs::Position::BendOutside,
        _ => fs::Position::Outside,
    };
    let ev_edge = a.req("edge")?;
    let edge = match &ev_edge {
        Value::Sheet(s) => match &**s {
            SheetVal::Edge(e) => e.clone(),
            _ => return Err(wrong(a.span("edge"), "a sheet edge", Ty::SheetEdge, &ev_edge)),
        },
        o => return Err(wrong(a.span("edge"), "a sheet edge", Ty::SheetEdge, o)),
    };
    let si = edge.op.sheet;
    not_unfolded(ev, si, &id, span)?;
    let height = a.req_num("height", LEN)?;
    let angle = a
        .num("angle", ANG)?
        .unwrap_or(Num::konst(dmath::PI / 2.0, Dim::Angle));
    let up = word(&a, "side", &["up", "down"])?.unwrap_or("up") == "up";
    let outer = word(&a, "measure", &["inner", "outer"])?.unwrap_or("outer") == "outer";
    let t = ev.sheets.list[si].sheet.gauge.thickness;
    // the kernel measures from the reference skin's mold line: the inside one going up, the
    // outside one going down; the other skin's virtual sharp is t tan(angle / 2) along from it
    let (sn, cs) = dmath::sin_cos(angle.v);
    let shift = t * sn / (1.0 + cs);
    let offset = match (up, outer) {
        (true, false) | (false, true) => 0.0,
        (true, true) => shift,
        (false, false) => -shift,
    };
    let signed = if up { angle.v } else { -angle.v };
    let radius = a.num("r", LEN)?;
    let mut dims = vec![dim("height", &height), dim("angle", &angle)];
    if let Some(r) = &radius {
        dims.push(dim("radius", r));
    }
    let st = &mut ev.sheets.list[si];
    let rv = radius.as_ref().map_or(st.sheet.gauge.radius, |r| r.v);
    let setback = fs::build::setback_of(&st.sheet.gauge, rv, signed);
    let dims_of = |joint, wall| FlangeDims {
        id: id.clone(),
        joint,
        wall,
        setback,
        r: rv,
        height: height.clone(),
        angle: angle.clone(),
        radius: radius.clone(),
    };
    let r = st
        .sheet
        .flange_at(edge.wall, edge.segment, signed, height.v - offset, rv, position);
    let (wall, joint) = match r {
        Ok(x) => x,
        Err(e) => {
            let mut snap = st.clone();
            // the refused flange's wall and joint are the next ids
            snap.fix.flanges.push(dims_of(
                JointId(snap.joints.len() as u32),
                WallId(snap.walls.len() as u32),
            ));
            snap.walls
                .push((WallId(snap.walls.len() as u32), id.clone(), span));
            snap.joints
                .push((JointId(snap.joints.len() as u32), id.clone(), span));
            return Err(kernel_err(&snap, span, &id, e));
        }
    };
    let fd = dims_of(joint, wall);
    st.walls.push((wall, id.clone(), span));
    st.joints.push((joint, id.clone(), span));
    if let Some(r) = &relief {
        let t = st.sheet.gauge.thickness;
        let d = st.sheet.rule.bend;
        let resolved = match r.kind {
            None => fs::Relief::None,
            Some(kind) => {
                let width = r.width.as_ref().map_or(d.width.at(t, rv), |n| n.v);
                let need = setback
                    + if kind == fs::ReliefKind::Obround {
                        0.5 * width
                    } else {
                        0.0
                    };
                let depth = r.depth.as_ref().map_or(d.depth.at(t, rv).max(need), |n| n.v);
                fs::Relief::Cut { kind, width, depth }
            }
        };
        for e in [fs::End::Start, fs::End::End] {
            st.sheet.set_relief(joint, e, resolved);
        }
        for n in [&r.width, &r.depth].into_iter().flatten() {
            dims.push(dim("relief", n));
        }
    }
    st.fix.flanges.push(fd);
    push_sheet_step(ev, si, &id, "flange", span, dims)?;
    Ok(op_value(si, &id, OpKind::Flange { wall, joint }))
}

/// A loop in one frame's plane carried into another's coordinates (the planes coincide; a flip
/// between them mirrors the loop).
fn carried(l: &ProfileLoop, from: &Frame, to: &Frame) -> Option<ProfileLoop> {
    let map = |p: Point2| {
        let q = to.point_to_local(from.point_to_world(Point3::new(p.x, p.y, 0.0)));
        Point2::new(q.x, q.y)
    };
    let dir = |a: f64| {
        let (s, c) = dmath::sin_cos(a);
        let v = to.vec_to_local(from.vec_to_world(Vec3::new(c, s, 0.0)));
        dmath::atan2(v.y, v.x)
    };
    // whether the map mirrors: the two planes' normals opposed
    let mirror = from.z().get().dot(to.z().get()) < 0.0;
    let mut out = Vec::with_capacity(l.segments.len());
    for (s, id) in &l.segments {
        let s = match s {
            Segment::Line { a, b } => Segment::Line {
                a: map(*a),
                b: map(*b),
            },
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => Segment::Arc {
                center: map(*center),
                radius: *radius,
                start: dir(*start),
                sweep: if mirror { -sweep } else { *sweep },
            },
            Segment::Circle { center, radius, ccw } => Segment::Circle {
                center: map(*center),
                radius: *radius,
                ccw: ccw ^ mirror,
            },
            Segment::Spline(ps) => {
                let (degree, knots, pts, weights) = ps.c.control_form();
                let pts2 = pts.iter().map(|p| map(Point2::new(p.x, p.y))).collect();
                Segment::Spline(std::sync::Arc::new(PlaneSpline::control(
                    degree,
                    pts2,
                    Some(knots),
                    weights,
                )?))
            }
        };
        out.push((s, *id));
    }
    Some(ProfileLoop::new(out))
}

fn cutout(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let id = ev.take_id("cutout", span)?;
    let a = Args::bind(ev, "cutout", &["profile", "wall", "crosses_bend"], args, span)?;
    let crosses = a.flag("crosses_bend")?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let wv = a.req("wall")?;
    let op = op_of(&wv, a.span("wall"), "the wall a cutout is sketched on")?;
    let wall = match op.kind {
        OpKind::Base { wall } | OpKind::Flange { wall, .. } if op.on == On::Folded => wall,
        _ => {
            return Err(wrong(
                a.span("wall"),
                "a base or flange of the folded sheet",
                Ty::Sheet,
                &wv,
            ));
        }
    };
    let si = op.sheet;
    not_unfolded(ev, si, &id, span)?;
    // a whole sketch cuts all its regions; a loop or entity, its own
    let (sk, loops) = match a.req("profile")? {
        Value::Sketch(sk) => {
            let ps = sk.borrow().profiles(a.span("profile"))?;
            (sk, ps.into_iter().flat_map(|p| p.loops).collect::<Vec<_>>())
        }
        v => {
            let (sk, p, _) = profile_of(ev, v, a.span("profile"))?;
            (sk, p.loops)
        }
    };
    let Some(frame) = wall_frame(&ev.sheets.list[si], wall, None) else {
        // nothing built to place it on (check, or an earlier failure): the feature is kept, the
        // graph as it was
        push_sheet_step(ev, si, &id, "cutout", span, vec![])?;
        return Ok(op_value(si, &id, OpKind::Cut { cut: CutId(u32::MAX) }));
    };
    let from = sk.borrow().frame;
    // the sketch must lie on the wall's reference skin or its back
    let t = ev.sheets.list[si].sheet.gauge.thickness;
    let z = frame.point_to_local(from.origin).z;
    let normal = from.z().get().dot(frame.z().get()).abs();
    let off = z.abs().min((z + t).abs());
    if off > 1e-7 || (normal - 1.0).abs() > 1e-9 {
        return Err(sheet_err(SheetErr::CutOffWall {
            span,
            wall: op.id.clone(),
            distance: if (normal - 1.0).abs() > 1e-9 {
                f64::NAN
            } else {
                off
            },
        }));
    }
    let on_skin = frame.with_origin(frame.point_to_world(Point3::new(0.0, 0.0, z)));
    let mut cut_ids = vec![];
    let st = &mut ev.sheets.list[si];
    for l in &loops {
        let lp = carried(l, &from, &on_skin).ok_or_else(|| Error::BadValue {
            span: a.span("profile"),
            what: "a spline carried onto the wall",
            value: f64::NAN,
        })?;
        match st.sheet.cut(wall, lp) {
            Ok(c) => cut_ids.push(c),
            Err(e) => {
                let mut snap = st.clone();
                snap.cuts.push((CutId(snap.cuts.len() as u32), id.clone(), span));
                return Err(kernel_err(&snap, span, &id, e));
            }
        }
    }
    for &c in &cut_ids {
        st.cuts.push((c, id.clone(), span));
        st.cut_srcs.push(CutSrc {
            cut: c,
            sketch: sk.clone(),
            from,
            to: on_skin,
            at: None,
        });
        if let Some(x) = st.sheet.cuts.iter_mut().find(|x| x.id == c) {
            x.crosses_bend = crosses;
        }
    }
    push_sheet_step(ev, si, &id, "cutout", span, vec![])?;
    let first = cut_ids.first().copied().unwrap_or(CutId(u32::MAX));
    Ok(op_value(si, &id, OpKind::Cut { cut: first }))
}

fn unfold(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let id = ev.take_id("unfold", span)?;
    let a = Args::bind(ev, "unfold", &["sheet", "on"], args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let sv0 = a.req("sheet")?;
    let op = op_of(&sv0, a.span("sheet"), "the sheet to unfold")?;
    let si = op.sheet;
    let st = &ev.sheets.list[si];
    let frame = match a.opt("on") {
        None => st.sheet.frame,
        Some(p) => crate::builtins::plane_of(ev, p, a.span("on"), "on")?,
    };
    let st = &ev.sheets.list[si];
    let mut flat = st.sheet.clone();
    flat.frame = frame;
    let hid = history_id(&id);
    let text = format!("{flat:?}");
    let marker = marker(&id, text.clone());
    let solid = ev.b.features.len();
    let snapshot = st.clone();
    let salt = Name::feature(hid);
    let mut made = None;
    let r = ev.push_made(&id, "unfold", span, vec![], hid, |b: &mut Builder| {
        b.push_with(marker, hid, |before: &[Solid]| {
            let built = fs::build_with(&flat, Map::Flat).map_err(|e| kernel_err(&snapshot, span, &id, e))?;
            let body = salted(&built.body, salt);
            made = Some(built);
            Ok(with_body(before, solid, body))
        })
    });
    r?;
    let ok = ev.feats.last().is_some_and(|f| f.status == Status::Ok);
    let key = format!("flat {text}");
    let built = match (made, ok) {
        (Some(b), _) => ev.b.memo(key, || Some(b)),
        (None, true) => ev.b.memo(key, || fs::build_with(&flat, Map::Flat).ok()),
        (None, false) => None,
    };
    let index = ev.sheets.list[si].unfolds.len();
    ev.sheets.list[si].unfolds.push(UnfoldState {
        id: id.clone(),
        hid,
        frame,
        built,
        span,
    });
    Ok(sv(SheetVal::Unfold(Rc::new(UnfoldRef { sheet: si, index, id }))))
}

fn salted(b: &Body, salt: Name) -> Body {
    let mut body = b.clone();
    body.vertices
        .iter_mut()
        .for_each(|v| v.name = v.name.salted(salt));
    body.edges.iter_mut().for_each(|e| e.name = e.name.salted(salt));
    body.faces.iter_mut().for_each(|f| f.name = f.name.salted(salt));
    body
}

// ---- anchors -----------------------------------------------------------------------------------

/// The built sheet an anchor reads, and the salt its body's names carry.
fn built_on(st: &SheetState, on: On) -> Option<(&fs::Built, Option<Name>)> {
    match on {
        On::Folded => st.folded.as_ref().map(|b| (b, None)),
        On::Flat(k) => {
            let u = st.unfolds.get(k)?;
            u.built.as_ref().map(|b| (b, Some(Name::feature(u.hid))))
        }
    }
}

fn wall_frame(st: &SheetState, wall: WallId, on: Option<usize>) -> Option<Frame> {
    let (b, _) = built_on(st, on.map_or(On::Folded, On::Flat))?;
    b.layout.walls.iter().find(|w| w.wall == wall).map(|w| w.frame)
}

fn names_in(ev: &Eval, names: Vec<Name>, salt: Option<Name>) -> Option<Vec<Name>> {
    let body = ev.b.body()?;
    let mut out = vec![];
    for n in names {
        let n = salt.map_or(n, |s| n.salted(s));
        for f in role_faces(body, n) {
            let m = body.face(f).name;
            if !out.contains(&m) {
                out.push(m);
            }
        }
    }
    Some(out)
}

fn edge_names_in(ev: &Eval, names: Vec<Name>, salt: Option<Name>) -> Option<Vec<Name>> {
    let body = ev.b.body()?;
    let have: BTreeSet<Name> = body.edge_ids().map(|e| body.edge(e).name).collect();
    Some(
        names
            .into_iter()
            .map(|n| salt.map_or(n, |s| n.salted(s)))
            .filter(|n| have.contains(n))
            .collect(),
    )
}

fn faces_value(names: Option<Vec<Name>>, owner: &str, role: Role, plane: Option<Frame>) -> Value {
    match names {
        None => Value::Unresolved,
        Some(names) => Value::Faces(Rc::new(Faces {
            names,
            owner: owner.into(),
            role,
            plane,
        })),
    }
}

/// The trims an op's faces and edges come from.
fn op_trims(st: &SheetState, op: &OpRef) -> Vec<TrimRef> {
    let outline = |w: WallId| -> Vec<TrimRef> {
        st.sheet
            .wall(w)
            .map(|wl| {
                wl.outline
                    .iter()
                    .flat_map(|l| &l.segments)
                    .map(|s| TrimRef::Outline {
                        wall: w,
                        segment: s.1,
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    match op.kind {
        OpKind::Base { wall } => outline(wall),
        OpKind::Flange { wall, joint } => {
            let mut v = outline(wall);
            for end in [fs::End::Start, fs::End::End] {
                v.push(TrimRef::BendEnd { joint, end });
                // its reliefs: five segments at most
                v.extend((0..5).map(|segment| TrimRef::Relief { joint, end, segment }));
            }
            v
        }
        OpKind::Corner { a, b } => (0..4)
            .flat_map(|segment| {
                [
                    TrimRef::Corner { a, b, segment },
                    TrimRef::Corner { a: b, b: a, segment },
                ]
            })
            .collect(),
        OpKind::Contour { .. } => {
            let (walls, joints) = feat::parts(st, op);
            let mut v: Vec<TrimRef> = walls.into_iter().flat_map(outline).collect();
            for joint in joints {
                v.extend([fs::End::Start, fs::End::End].map(|end| TrimRef::BendEnd { joint, end }));
            }
            v
        }
        OpKind::Cut { .. } => st
            .cuts
            .iter()
            .filter(|c| c.1 == op.id)
            .flat_map(|c| {
                st.sheet.cuts.iter().filter(move |x| x.id == c.0).flat_map(|x| {
                    x.lp.segments.iter().map(move |s| TrimRef::Cut {
                        cut: x.id,
                        segment: s.1,
                    })
                })
            })
            .collect(),
        // its rim (folded: its faces, by profile piece; flat: its pierce)
        OpKind::Form { form } => (0..2u64)
            .flat_map(|c| (0..16).map(move |i| c * 16 + i))
            .chain([100, 101])
            .map(|segment| TrimRef::Form { form, segment })
            .collect(),
    }
}

fn op_strips(st: &SheetState, op: &OpRef) -> Vec<Strip> {
    match op.kind {
        OpKind::Base { wall } => vec![Strip::Wall(wall)],
        OpKind::Flange { wall, joint } => vec![Strip::Wall(wall), Strip::Bend(joint)],
        OpKind::Contour { .. } => {
            let (walls, joints) = feat::parts(st, op);
            walls
                .into_iter()
                .map(Strip::Wall)
                .chain(joints.into_iter().map(Strip::Bend))
                .collect()
        }
        OpKind::Cut { .. } | OpKind::Corner { .. } | OpKind::Form { .. } => vec![],
    }
}

pub fn member(ev: &mut Eval, s: &SheetVal, name: &str, span: Span) -> R<Value> {
    let unknown = || Error::UnknownMember {
        span,
        ty: s.ty(),
        member: name.into(),
    };
    match s {
        SheetVal::Op(op) => {
            if op.on == On::Folded
                && let Some(v) = crate::builtins::feature_local(ev, &op.id, name, span)
            {
                return v;
            }
            op_member(ev, op, name, span).unwrap_or_else(|| Err(unknown()))
        }
        SheetVal::Edge(e) => {
            let st = &ev.sheets.list[e.op.sheet];
            let trim = TrimRef::Outline {
                wall: e.wall,
                segment: e.segment,
            };
            let on = e.op.on;
            let Some((b, salt)) = built_on(st, on) else {
                return Ok(Value::Unresolved);
            };
            match name {
                "face" => {
                    let plane = edge_plane(st, b, e.wall, e.segment);
                    let names = b.roles.side(trim);
                    Ok(faces_value(
                        names_in(ev, names, salt),
                        &e.op.id,
                        Role::Side,
                        plane,
                    ))
                }
                "edges" | "top" | "bottom" => {
                    let mut names = vec![];
                    if name != "bottom" {
                        names.extend(b.roles.edge(trim, Skin::Reference));
                    }
                    if name != "top" {
                        names.extend(b.roles.edge(trim, Skin::Back));
                    }
                    Ok(
                        edge_names_in(ev, names, salt)
                            .map_or(Value::Unresolved, |n| Value::Edges(Rc::new(n))),
                    )
                }
                _ => Err(unknown()),
            }
        }
        SheetVal::Unfold(u) => {
            let st = &ev.sheets.list[u.sheet];
            let Some(b) = st.unfolds[u.index].built.as_ref() else {
                return Ok(Value::Unresolved);
            };
            let salt = Some(Name::feature(st.unfolds[u.index].hid));
            match name {
                "faces" => {
                    let names = b
                        .roles
                        .skins
                        .iter()
                        .map(|x| x.2)
                        .chain(b.roles.sides.iter().map(|x| x.1))
                        .collect();
                    Ok(faces_value(names_in(ev, names, salt), &u.id, Role::Faces, None))
                }
                "edges" => {
                    let names = b.roles.edges.iter().map(|x| x.2).collect();
                    Ok(
                        edge_names_in(ev, names, salt)
                            .map_or(Value::Unresolved, |n| Value::Edges(Rc::new(n))),
                    )
                }
                _ => Err(unknown()),
            }
        }
        _ => Err(unknown()),
    }
}

/// The plane of a straight outline edge's side face: origin at its start on the reference skin,
/// x along it, z out of the wall.
fn edge_plane(st: &SheetState, b: &fs::Built, wall: WallId, segment: u64) -> Option<Frame> {
    let w = st.sheet.wall(wall)?;
    let (a, e) = w.outline.iter().flat_map(|l| &l.segments).find_map(|s| match s {
        (Segment::Line { a, b }, id) if *id == segment => Some((*a, *b)),
        _ => None,
    })?;
    let f = b.layout.walls.iter().find(|x| x.wall == wall)?.frame;
    let d = (e - a).normalize()?;
    let x = f.vec_to_world(Vec3::new(d.x, d.y, 0.0));
    let z = f.vec_to_world(Vec3::new(d.y, -d.x, 0.0));
    let o = f.point_to_world(Point3::new(a.x, a.y, 0.0));
    Frame::from_axes(o, x, z.cross(x), z)
}

fn op_member(ev: &mut Eval, op: &Rc<OpRef>, name: &str, span: Span) -> Option<R<Value>> {
    let st = &ev.sheets.list[op.sheet];
    let flat = match op.on {
        On::Folded => None,
        On::Flat(k) => Some(k),
    };
    let no_role = || {
        Some(Err(sheet_err(SheetErr::NoSuchRole {
            span,
            op: op.id.clone(),
            role: name.into(),
        })))
    };
    let edge = |segment: u64| -> Option<R<Value>> {
        let wall = match op.kind {
            OpKind::Flange { wall, .. } => wall,
            // a contour's last wall: its tip is the far edge, like a flange's
            OpKind::Contour { last, .. } if segment == 1 => last,
            _ => return no_role(),
        };
        Some(Ok(sv(SheetVal::Edge(Rc::new(EdgeRef {
            op: op.clone(),
            wall,
            segment,
        })))))
    };
    match name {
        // a flange's wall: x from the mold line (segment 3) to the tip (1), segment 0 at the
        // start of the edge it turned off
        "tip" => return edge(1),
        "start_side" => return edge(0),
        "end_side" => return edge(2),
        _ => {}
    }
    let built = built_on(st, op.on);
    let wall = match op.kind {
        OpKind::Base { wall } | OpKind::Flange { wall, .. } => Some(wall),
        // the contour's first wall's plane; its faces, every wall's
        OpKind::Contour { first, .. } => Some(first),
        OpKind::Cut { .. } | OpKind::Corner { .. } | OpKind::Form { .. } => None,
    };
    let walls = match op.kind {
        OpKind::Contour { .. } => feat::parts(st, op).0,
        _ => wall.into_iter().collect(),
    };
    match name {
        "top" | "bottom" => {
            let Some(wall) = wall else { return no_role() };
            let Some((b, salt)) = built else {
                return Some(Ok(Value::Unresolved));
            };
            let skin = if name == "top" {
                Skin::Reference
            } else {
                Skin::Back
            };
            let names: Vec<Name> = walls
                .iter()
                .flat_map(|w| b.roles.skin(Strip::Wall(*w), skin))
                .collect();
            let frame = wall_frame(st, wall, flat).map(|f| {
                if name == "top" {
                    f
                } else {
                    f.with_origin(f.point_to_world(Point3::new(0.0, 0.0, -st.sheet.gauge.thickness)))
                        .flipped()
                }
            });
            let found = names_in(ev, names, salt);
            Some(Ok(faces_value(found, &op.id, Role::Wall, frame)))
        }
        "bend" | "bend_top" | "bend_bottom" => {
            let joints = match op.kind {
                OpKind::Flange { joint, .. } => vec![joint],
                OpKind::Contour { .. } => feat::parts(st, op).1,
                _ => return no_role(),
            };
            let Some((b, salt)) = built else {
                return Some(Ok(Value::Unresolved));
            };
            let mut names = vec![];
            for joint in joints {
                if name != "bend_bottom" {
                    names.extend(b.roles.skin(Strip::Bend(joint), Skin::Reference));
                }
                if name != "bend_top" {
                    names.extend(b.roles.skin(Strip::Bend(joint), Skin::Back));
                }
            }
            Some(Ok(faces_value(
                names_in(ev, names, salt),
                &op.id,
                Role::Faces,
                None,
            )))
        }
        "faces" | "edges" => {
            let Some((b, salt)) = built else {
                return Some(Ok(Value::Unresolved));
            };
            let trims = op_trims(st, op);
            let strips = op_strips(st, op);
            if name == "faces" {
                let mut names: Vec<Name> = b
                    .roles
                    .skins
                    .iter()
                    .filter(|x| strips.contains(&x.0))
                    .map(|x| x.2)
                    .collect();
                names.extend(b.roles.sides.iter().filter(|x| trims.contains(&x.0)).map(|x| x.1));
                Some(Ok(faces_value(
                    names_in(ev, names, salt),
                    &op.id,
                    Role::Faces,
                    None,
                )))
            } else {
                let names = b
                    .roles
                    .edges
                    .iter()
                    .filter(|x| trims.contains(&x.0))
                    .map(|x| x.2)
                    .collect();
                Some(Ok(
                    edge_names_in(ev, names, salt).map_or(Value::Unresolved, |n| Value::Edges(Rc::new(n)))
                ))
            }
        }
        _ => None,
    }
}

/// `S.edge(entity)` and `U.of(op)`.
pub fn method(ev: &mut Eval, s: &SheetVal, m: &str, args: &[Arg], span: Span) -> R<Value> {
    let [arg] = args else {
        return Err(Error::TooManyArgs {
            span,
            func: m.into(),
            max: 1,
        });
    };
    let v = ev.expr(&arg.value)?;
    match (s, m) {
        (SheetVal::Op(op), "edge") => {
            let OpKind::Base { wall } = op.kind else {
                return Err(sheet_err(SheetErr::NoSuchRole {
                    span,
                    op: op.id.clone(),
                    role: "edge(entity)".into(),
                }));
            };
            let Value::Ent(e) = &v else {
                return Err(wrong(arg.value.span, "edge(entity)", Ty::Entity, &v));
            };
            let st = &ev.sheets.list[op.sheet];
            let not_edge = || {
                sheet_err(SheetErr::NotAnEdge {
                    span: arg.value.span,
                    wall: op.id.clone(),
                })
            };
            if !Rc::ptr_eq(&e.sk, &st.sketch) {
                return Err(not_edge());
            }
            const PIECE: u64 = 0xffff << 48;
            let sid = e.sk.borrow().seg_id(e.id);
            let segs: Vec<(bool, u64)> = st
                .sheet
                .wall(wall)
                .map(|w| {
                    w.outline
                        .iter()
                        .flat_map(|l| &l.segments)
                        .filter(|s| s.1 & !PIECE == sid)
                        .map(|s| (matches!(s.0, Segment::Line { .. }), s.1))
                        .collect()
                })
                .unwrap_or_default();
            let [(true, segment)] = segs[..] else {
                return Err(not_edge());
            };
            Ok(sv(SheetVal::Edge(Rc::new(EdgeRef {
                op: op.clone(),
                wall,
                segment,
            }))))
        }
        (SheetVal::Unfold(u), "of") => {
            let op = op_of(&v, arg.value.span, "a feature of the unfolded sheet")?;
            if op.sheet != u.sheet {
                return Err(sheet_err(SheetErr::OtherSheet {
                    span: arg.value.span,
                    op: op.id.clone(),
                    unfold: u.id.clone(),
                }));
            }
            Ok(sv(SheetVal::Op(Rc::new(OpRef {
                on: On::Flat(u.index),
                ..(*op).clone()
            }))))
        }
        _ => Err(Error::UnknownMember {
            span,
            ty: s.ty(),
            member: m.into(),
        }),
    }
}

// ---- the end of a run --------------------------------------------------------------------------

/// The shop checks of every sheet, as typed notes on the features they name.
pub fn finish(ev: &mut Eval) {
    let mut notes = vec![];
    // each build's warning about the other map: a flat overlap on the folded part, a folded clash
    // on an unfold
    for st in &ev.sheets.list {
        let owner = |s: Strip| -> (String, Span) {
            let found = match s {
                Strip::Wall(w) => st.walls.iter().find(|x| x.0 == w).map(|x| (x.1.clone(), x.2)),
                Strip::Bend(j) => st.joints.iter().find(|x| x.0 == j).map(|x| (x.1.clone(), x.2)),
            };
            found.unwrap_or((st.id.clone(), st.walls[0].2))
        };
        let folded = st.folded.iter().map(|b| (&b.warnings, None));
        let flat = st.unfolds.iter().filter_map(|u| {
            u.built
                .as_ref()
                .map(|b| (&b.warnings, Some((u.id.clone(), u.span))))
        });
        for (warnings, on) in folded.chain(flat) {
            for w in warnings {
                if let fs::SheetWarning::FormClash { form, .. } = *w {
                    let (id, span) = on
                        .clone()
                        .or_else(|| st.forms.get(form).cloned())
                        .unwrap_or((st.id.clone(), st.walls[0].2));
                    notes.push(Warning::Sheet(Box::new(SheetNote {
                        span,
                        id,
                        note: Note::Warning(w.clone()),
                        names: st.names(),
                        fixes: vec![],
                    })));
                    continue;
                }
                let (a, b) = match *w {
                    fs::SheetWarning::FlatOverlap { a, b, .. }
                    | fs::SheetWarning::FlatOverlapUndecided { a, b, .. }
                    | fs::SheetWarning::FoldedClash { a, b, .. }
                    | fs::SheetWarning::FlatTouch { a, b, .. }
                    | fs::SheetWarning::FoldedTouch { a, b, .. } => (a, b),
                    _ => continue,
                };
                let (id, span) = on.clone().unwrap_or_else(|| owner(a));
                let fixes = match w {
                    fs::SheetWarning::FlatOverlap { .. } => overlap_fixes(st, a, b),
                    fs::SheetWarning::FlatTouch { seam, .. } => {
                        let mut out = seam_fixes(st, *seam, None);
                        out.extend(touch_fixes(st, a, b, Map::Flat));
                        out
                    }
                    fs::SheetWarning::FoldedTouch { .. } => touch_fixes(st, a, b, Map::Folded),
                    _ => vec![],
                };
                notes.push(Warning::Sheet(Box::new(SheetNote {
                    span,
                    id,
                    note: Note::Warning(w.clone()),
                    names: st.names(),
                    fixes,
                })));
            }
        }
    }
    for st in &ev.sheets.list {
        // the shop's rules where there is a shop; hardware's own rows always
        let (violations, warnings) = {
            let key = format!("checks {:?} {:?}", st.sheet, st.shop.as_ref().map(|s| &s.shop));
            ev.b.memo(key, || {
                let violations = match &st.shop {
                    Some(shop) => fs::check(&st.sheet, &shop.shop),
                    None => fs::checks::mounts(&st.sheet, None),
                };
                (violations, fs::checks::warnings(&st.sheet))
            })
        };
        let cut_owner = |c: CutId| st.cuts.iter().find(|x| x.0 == c).map(|x| (x.1.clone(), x.2));
        for w in warnings {
            let fs::SheetWarning::UnverifiedHardware { cut, .. } = w else {
                continue;
            };
            let (id, span) = cut_owner(cut).unwrap_or((st.id.clone(), st.walls[0].2));
            notes.push(Warning::Sheet(Box::new(SheetNote {
                span,
                id,
                note: Note::Warning(w),
                names: st.names(),
                fixes: vec![],
            })));
        }
        for v in violations {
            let (id, span) = match v {
                fs::Violation::FlangeTooShort { wall, .. } => {
                    st.walls.iter().find(|w| w.0 == wall).map(|w| (w.1.clone(), w.2))
                }
                fs::Violation::RadiusNotTooled { joint, .. } => st
                    .joints
                    .iter()
                    .find(|j| j.0 == joint)
                    .map(|j| (j.1.clone(), j.2)),
                fs::Violation::HoleNearBend { trim, .. }
                | fs::Violation::HoleNearEdge { trim, .. }
                | fs::Violation::HoleTooSmall { trim, .. }
                | fs::Violation::SlotTooNarrow { trim, .. } => match trim {
                    TrimRef::Cut { cut, .. } => cut_owner(cut),
                    // a hole drawn in a wall's own outline: on that wall
                    TrimRef::Outline { wall, .. } => {
                        st.walls.iter().find(|w| w.0 == wall).map(|w| (w.1.clone(), w.2))
                    }
                    _ => None,
                },
                fs::Violation::CutCrossesBend { trim, .. } => match trim {
                    TrimRef::Cut { cut, .. } => cut_owner(cut),
                    _ => None,
                },
                fs::Violation::HardwareThickness { cut, .. }
                | fs::Violation::LipTooShort { cut, .. }
                | fs::Violation::HardwareNearEdge { cut, .. }
                | fs::Violation::HardwareOverlapsHole { cut, .. }
                | fs::Violation::HardwareNearHole { cut, .. }
                | fs::Violation::SheetTooHard { cut, .. }
                | fs::Violation::HardnessUnknown { cut, .. } => cut_owner(cut),
                fs::Violation::ThicknessNotStocked { .. }
                | fs::Violation::FlatTooLarge { .. }
                | fs::Violation::PartTooSmall { .. } => None,
                fs::Violation::BendTooLong { joint, .. }
                | fs::Violation::BoxFlangeTooTall { joint, .. }
                | fs::Violation::BendAngleOutOfRange { joint, .. } => st
                    .joints
                    .iter()
                    .find(|j| j.0 == joint)
                    .map(|j| (j.1.clone(), j.2)),
                fs::Violation::Form { form, .. } => st.forms.get(form).map(|f| (f.0.clone(), f.1)),
                fs::Violation::Punch { punch, .. } => st
                    .sheet
                    .punches
                    .get(punch)
                    .and_then(|p| p.cuts.first())
                    .and_then(|c| cut_owner(*c)),
                fs::Violation::ReliefUndersized { joint, .. } => st
                    .joints
                    .iter()
                    .find(|j| j.0 == joint)
                    .map(|j| (j.1.clone(), j.2)),
            }
            .unwrap_or((st.id.clone(), st.walls[0].2));
            let mut fixes = violation_fixes(&st.fix, &v);
            fixes.extend(cut_fixes(st, &v));
            notes.push(Warning::Sheet(Box::new(SheetNote {
                span,
                id,
                note: Note::Violation(v),
                names: st.names(),
                fixes,
            })));
        }
    }
    ev.warnings.extend(notes);
}

/// The numbers that placed and sized a cut's sketch entity: its centre's x and y, its radius.
fn cut_nums(src: &CutSrc, segment: Option<u64>) -> (Option<Num>, Option<Num>, Option<Num>) {
    const PIECE: u64 = 0xffff << 48;
    let sk = src.sketch.borrow();
    let ent = src.at.or_else(|| {
        let seg = segment? & !PIECE;
        sk.ents.iter().find(|e| sk.seg_id(e.0) == seg).map(|e| e.0)
    });
    let Some(e) = ent else {
        return (None, None, None);
    };
    let num = |id: forge_sketch::EntityId, k: &str| {
        sk.nums
            .iter()
            .find(|n| n.0 == id && n.1 == k)
            .map(|n| n.2.clone())
    };
    let r = num(e, "r");
    // the centre: written on the circle, or its own point
    let centre = match sk.s.entity(e) {
        Some(forge_sketch::Entity::Circle { center, .. }) if num(e, "x").is_none() => center,
        _ => e,
    };
    (num(centre, "x"), num(centre, "y"), r)
}

/// Fixes that move or resize a cut or a part: along `away` (the wall's coordinates, carried into
/// the sketch's) by what is missing, where the way is along a sketch axis; or a radius.
fn cut_fixes(st: &SheetState, v: &fs::Violation) -> Vec<Op> {
    let src = |c: CutId| st.cut_srcs.iter().find(|x| x.cut == c);
    // a hole drawn in the base sketch: that sketch, in the base wall's own frame
    let frame = st.sketch.borrow().frame;
    let base = CutSrc {
        cut: CutId(u32::MAX),
        sketch: st.sketch.clone(),
        from: frame,
        to: frame,
        at: None,
    };
    let hole = |t: TrimRef| match t {
        TrimRef::Cut { cut, segment } => src(cut).map(|s| (s, segment)),
        TrimRef::Outline {
            wall: WallId(0),
            segment,
        } => Some((&base, segment)),
        _ => None,
    };
    // to a thousandth, the way that clears (a measured distance's last bits aside)
    let round_up = |x: f64| ((x * 1000.0 - 1e-6).ceil() / 1000.0).max(0.001);
    let round_down = |x: f64| (x * 1000.0 + 1e-6).floor() / 1000.0;
    let moved = |s: &CutSrc, segment: Option<u64>, away: forge_math::Vec2, by: f64, resize: bool| {
        let (x, y, r) = cut_nums(s, segment);
        let v = s
            .from
            .vec_to_local(s.to.vec_to_world(Vec3::new(away.x, away.y, 0.0)));
        let by = round_up(by);
        let mut out = vec![];
        for (axis, n) in [(v.x, x), (v.y, y)] {
            if (axis.abs() - 1.0).abs() < 1e-9
                && let Some(n) = n
                && let Some(op) = set(&n, n.v + axis.signum() * by)
            {
                out.push(op);
            }
        }
        if resize
            && let Some(r) = r
            && r.v - by > 0.0
            && let Some(op) = set(&r, round_down(r.v - by))
        {
            out.push(op);
        }
        out
    };
    match *v {
        fs::Violation::HoleNearBend {
            trim,
            distance,
            limit,
            away,
            ..
        }
        | fs::Violation::HoleNearEdge {
            trim,
            distance,
            limit,
            away,
            ..
        } => hole(trim).map_or(vec![], |(s, segment)| {
            moved(s, Some(segment), away, limit - distance, true)
        }),
        fs::Violation::HardwareNearHole {
            cut,
            distance,
            limit,
            away,
            ..
        } => src(cut).map_or(vec![], |s| moved(s, None, away, limit - distance, false)),
        // clear of the other hole by the part's edge distance, which clears the overlap too
        fs::Violation::HardwareOverlapsHole {
            cut,
            distance,
            limit,
            away,
            ..
        } if distance > 0.0 => src(cut).map_or(vec![], |s| moved(s, None, away, limit - distance, false)),
        fs::Violation::HardwareNearEdge {
            cut,
            distance,
            limit,
            away,
            ..
        } => src(cut).map_or(vec![], |s| moved(s, None, away, limit - distance, false)),
        fs::Violation::HoleTooSmall { trim, limit, .. } => hole(trim)
            .and_then(|(s, segment)| cut_nums(s, Some(segment)).2)
            .and_then(|r| set(&r, round_up(0.5 * limit)))
            .into_iter()
            .collect(),
        fs::Violation::CutCrossesBend {
            trim: TrimRef::Cut { cut, .. },
            intended: false,
            ..
        } => st
            .cuts
            .iter()
            .find(|x| x.0 == cut)
            .map(|x| Op::SetArg {
                id: x.1.clone(),
                arg: "crosses_bend".into(),
                text: "true".into(),
            })
            .into_iter()
            .collect(),
        // the lip raised till the kernel finds room (the width across is not the height's
        // change one for one where the hardware sits near the bend), by halving
        fs::Violation::LipTooShort {
            cut,
            wall,
            width,
            need,
            ..
        } => st
            .fix
            .flanges
            .iter()
            .find(|f| f.wall == wall)
            .and_then(|f| {
                let stock = st
                    .shop
                    .as_ref()
                    .and_then(|s| s.shop.stock(st.sheet.gauge.thickness).cloned());
                let room = |up: f64| {
                    let mut s = st.sheet.clone();
                    s.move_edge(f.wall, 1, up).is_ok()
                        && !fs::checks::mounts(&s, stock.as_ref())
                            .iter()
                            .any(|v| matches!(v, fs::Violation::LipTooShort { cut: c, .. } if *c == cut))
                };
                let mut hi = (need - width).max(0.001);
                while !room(hi) && hi < 4.0 * need {
                    hi *= 2.0;
                }
                if !room(hi) {
                    return None;
                }
                let mut lo = 0.0;
                while hi - lo > 1e-5 {
                    let mid = 0.5 * (lo + hi);
                    if room(mid) { hi = mid } else { lo = mid }
                }
                set(&f.height, round_up(f.height.v + hi))
            })
            .into_iter()
            .collect(),
        fs::Violation::HardwareThickness { min, max, .. } => {
            let t = st.fix.t.v.clamp(min, max);
            set(&st.fix.t, t).into_iter().collect()
        }
        _ => vec![],
    }
}

pub fn outputs(ev: &Eval) -> Vec<SheetOut> {
    ev.sheets
        .list
        .iter()
        .map(|st| SheetOut {
            id: st.id.clone(),
            sheet: st.sheet.clone(),
            walls: st.walls.iter().map(|w| (w.0, w.1.clone())).collect(),
            joints: st.joints.iter().map(|j| (j.0, j.1.clone())).collect(),
            layout: st.folded.as_ref().map(|b| b.layout.clone()),
            folded: st.folded.as_ref().map(|b| b.body.clone()),
            reliefs: st.folded.as_ref().map_or(vec![], |b| b.reliefs.clone()),
            corners: st.folded.as_ref().map_or(vec![], |b| b.corners.clone()),
            material: st.material.clone(),
            unfolds: st
                .unfolds
                .iter()
                .map(|u| {
                    (
                        u.id.clone(),
                        u.built.as_ref().map(|b| salted(&b.body, Name::feature(u.hid))),
                    )
                })
                .collect(),
        })
        .collect()
}

impl SheetOut {
    /// The flat pattern: contours, bend lines, hits and forms on the flat.
    pub fn pattern(&self) -> Result<forge_sheet::Pattern, SheetError> {
        let b = fs::build_with(&self.sheet, Map::Flat)?;
        Ok(b.pattern.expect("a flat build's pattern"))
    }

    /// The flat pattern as DXF: contours on CUT, bend lines on BEND, and on BEND_NOTES each
    /// bend's direction, angle and inside radius, and the part's note.
    pub fn dxf(&self) -> Result<String, SheetError> {
        self.dxf_with(&fs::DxfLayers::default())
    }

    /// `dxf` with each kind of line on the layer asked for, or left out.
    pub fn dxf_with(&self, layers: &fs::DxfLayers) -> Result<String, SheetError> {
        Ok(fs::dxf_layered(&self.pattern()?, Some(&self.note()), layers))
    }

    /// The flat pattern as 2D drawing data, in the first wall's own coordinates (`sheet.frame`).
    pub fn pattern(&self) -> Result<fs::Pattern, SheetError> {
        let b = fs::build_with(&self.sheet, Map::Flat)?;
        Ok(b.pattern.expect("a flat build's pattern"))
    }

    /// The drawing's part note: the part, its material, its thickness and its bend rule.
    pub fn note(&self) -> String {
        let g = &self.sheet.gauge;
        let rule = match &g.rule {
            fs::BendRule::K(k) => format!("K {k:.3}"),
            fs::BendRule::KTable(_) => "K BY TABLE".into(),
            fs::BendRule::Allowance(_) => "BEND ALLOWANCE BY TABLE".into(),
        };
        format!(
            "{}  MATERIAL {}  T {:.2}  {rule}",
            self.id.to_uppercase(),
            self.material.as_deref().unwrap_or("UNSPECIFIED").to_uppercase(),
            g.thickness
        )
    }

    pub fn json(&self) -> Json {
        let g = &self.sheet.gauge;
        let rule = match &g.rule {
            fs::BendRule::K(k) => Json::obj(vec![("k", (*k).into())]),
            fs::BendRule::KTable(rows) => Json::obj(vec![("k_table_rows", rows.len().into())]),
            fs::BendRule::Allowance(rows) => Json::obj(vec![("allowance_rows", rows.len().into())]),
        };
        let name = |j: JointId| {
            self.joints
                .iter()
                .find(|x| x.0 == j)
                .map_or(String::new(), |x| x.1.clone())
        };
        let bends = self.layout.as_ref().map_or(vec![], |l| {
            l.bends
                .iter()
                .map(|b| {
                    Json::obj(vec![
                        ("flange", name(b.joint).into()),
                        ("angle", b.angle.to_degrees().into()),
                        ("radius", b.radius.into()),
                        ("allowance", b.allowance.into()),
                        ("setback", b.setback.into()),
                        ("length", b.length.into()),
                    ])
                })
                .collect()
        });
        Json::obj(vec![
            ("id", self.id.as_str().into()),
            ("thickness", g.thickness.into()),
            ("radius", g.radius.into()),
            ("rule", rule),
            ("walls", self.walls.len().into()),
            ("bends", Json::Arr(bends)),
            (
                "reliefs",
                Json::Arr(
                    self.reliefs
                        .iter()
                        .map(|r| {
                            Json::obj(vec![
                                ("flange", name(r.joint).into()),
                                ("end", end_name(r.end).into()),
                                ("kind", r.kind.name().into()),
                                ("width", r.width.into()),
                                ("depth", r.depth.into()),
                                ("automatic", r.automatic.into()),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "corners",
                Json::Arr(
                    self.corners
                        .iter()
                        .map(|c| {
                            let kind = match c.kind {
                                fs::CornerKind::Circular { .. } => "circular",
                                fs::CornerKind::Square { .. } => "square",
                                fs::CornerKind::Open { .. } => "open",
                                fs::CornerKind::Closed => "closed",
                                fs::CornerKind::Overlap { .. } => "overlap",
                                fs::CornerKind::Tear => "tear",
                                fs::CornerKind::TrimToBend => "trim_to_bend",
                                fs::CornerKind::ConstantWidth { .. } => "constant_width",
                            };
                            Json::obj(vec![
                                (
                                    "flanges",
                                    Json::Arr(vec![name(c.joints[0]).into(), name(c.joints[1]).into()]),
                                ),
                                ("kind", kind.into()),
                                ("size", c.size.into()),
                                ("automatic", c.automatic.into()),
                            ])
                        })
                        .collect(),
                ),
            ),
        ])
    }
}
