//! Thermoforming in Forge, on forge-thermo. `thermoform(MOULD, t:, material:, ..)` forms a sheet
//! of thickness t over a modelled mould: the mould's surface is the part's inner skin (`side:
//! "inner"`, a male tool) or its outer one (`"outer"`, a female tool). It makes two bodies: the
//! formed part (a new body, named by the feature) and the tool, which takes the mould body's place.
//! The shop's checks run on every build, typed, with fixes where a param or argument cures them.
//! Materials and machines are data: `thermo_material(..)` and `thermo_shop(..)`.
use crate::build::{Builder, history_id};
use crate::builtins::{Args, LEN, dim, dir_of, not_in_sketch, num_of, positive, profile_of};
use crate::error::{Error, Literal, Op, Warning};
use crate::eval::{Eval, Feat, R, Status, Value};
use crate::syntax::{Arg, Span};
use crate::values::{Dim, Num, Src, Ty};
use forge_agent::json::Json;
use forge_math::{Point2, Point3, UnitVec3};
use forge_model::{Feature, Solid};
use forge_thermo::{self as ft, Side, Violation};
use forge_topo::Body;
use std::f64::consts::FRAC_PI_4;
use std::rc::Rc;

pub const BUILTINS: &[&str] = &["thermoform", "thermo_material", "thermo_shop"];

const RATIO: &[Dim] = &[Dim::Ratio];
const ANG: &[Dim] = &[Dim::Angle];

#[derive(Clone, Debug)]
pub enum ThermoVal {
    Material(Rc<ft::Material>),
    Shop(Rc<ft::FormShop>),
}

impl ThermoVal {
    pub fn ty(&self) -> Ty {
        match self {
            ThermoVal::Material(_) => Ty::ThermoMaterial,
            ThermoVal::Shop(_) => Ty::ThermoShop,
        }
    }
}

/// A thermoform as built, for the report, the app's view and the thickness model.
#[derive(Clone, Debug)]
pub struct ThermoOut {
    pub id: String,
    pub material: String,
    pub side: Side,
    pub t: f64,
    pub formed: Rc<ft::Formed>,
}

impl ThermoOut {
    pub fn json(&self) -> Json {
        let r = &self.formed.report;
        let (lo, hi) = r.trim_box;
        Json::obj(vec![
            ("id", self.id.as_str().into()),
            ("material", self.material.as_str().into()),
            (
                "side",
                match self.side {
                    Side::Inner => "inner",
                    Side::Outer => "outer",
                }
                .into(),
            ),
            ("t", self.t.into()),
            ("depth", r.depth.into()),
            ("formed_area", r.formed_area.into()),
            ("opening_area", r.opening_area.into()),
            ("areal_draw_ratio", r.adr.into()),
            ("linear_draw_ratio", r.ldr.into()),
            ("mean_wall", r.mean_wall.into()),
            (
                "trim_size",
                Json::Arr(vec![(hi.x - lo.x).into(), (hi.y - lo.y).into()]),
            ),
            ("tool_scale", r.scale.into()),
            (
                "vents",
                Json::Arr(
                    r.vents
                        .iter()
                        .map(|p| Json::Arr(vec![p.x.into(), p.y.into(), p.z.into()]))
                        .collect(),
                ),
            ),
        ])
    }
}

/// The language's refusals and the kernel's.
#[derive(Clone, Debug, PartialEq)]
pub enum ThermoErr {
    Kernel {
        span: Span,
        id: String,
        error: ft::FormError,
    },
    UnknownMaterial {
        span: Span,
        id: String,
        name: String,
        known: Vec<String>,
    },
    /// The mould names no body of the part.
    NotABody { span: Span, id: String },
}

fn mm(x: f64) -> String {
    format!("{} mm", crate::values::fmt((x * 100.0).round() / 100.0))
}

fn deg(a: f64) -> String {
    format!(
        "{} deg",
        crate::values::fmt((a.to_degrees() * 10.0).round() / 10.0)
    )
}

fn p3(p: Point3) -> Json {
    Json::Arr(vec![p.x.into(), p.y.into(), p.z.into()])
}

impl ThermoErr {
    pub fn span(&self) -> Option<Span> {
        match self {
            ThermoErr::Kernel { span, .. }
            | ThermoErr::UnknownMaterial { span, .. }
            | ThermoErr::NotABody { span, .. } => Some(*span),
        }
    }

    pub fn kind(&self) -> &'static str {
        use ft::FormError as E;
        match self {
            ThermoErr::UnknownMaterial { .. } => "UnknownMaterial",
            ThermoErr::NotABody { .. } => "NotABody",
            ThermoErr::Kernel { error, .. } => match error {
                E::Size { .. } => "ThermoSize",
                E::NoOpening { .. } => "NoOpening",
                E::TrimNotSquare => "TrimNotSquare",
                E::TrimCutsMould { .. } => "TrimCutsMould",
                E::Extrude(_) => "ThermoExtrude",
                E::Bool(_) => "ThermoBoolean",
                E::Offset(_) => "SheetOffset",
                E::Mesh => "ThermoMesh",
            },
        }
    }

    pub fn fixes(&self) -> Vec<Op> {
        match self {
            ThermoErr::Kernel {
                id,
                error: ft::FormError::NoOpening { side },
                ..
            } => vec![Op::SetArg {
                id: id.clone(),
                arg: "side".into(),
                text: if *side == Side::Inner {
                    "\"outer\""
                } else {
                    "\"inner\""
                }
                .into(),
            }],
            ThermoErr::UnknownMaterial { id, known, .. } => known
                .iter()
                .take(6)
                .map(|k| Op::SetArg {
                    id: id.clone(),
                    arg: "material".into(),
                    text: format!("\"{k}\""),
                })
                .collect(),
            _ => vec![],
        }
    }

    pub fn text(&self) -> String {
        use ft::FormError as E;
        match self {
            ThermoErr::UnknownMaterial { name, known, .. } => {
                format!("no material {name} in the shop; it has {}", known.join(", "))
            }
            ThermoErr::NotABody { id, .. } => format!("{id} made no body to form over"),
            ThermoErr::Kernel { id, error, .. } => match error {
                E::Size { what, value } => format!("{id}: {what} must be positive, not {value}"),
                E::NoOpening { side } => format!(
                    "{id}: the mould has no flat face square to the pull at its {} to stand the sheet's \
                     trim plane on (side: \"{}\")",
                    if *side == Side::Inner { "foot" } else { "top" },
                    if *side == Side::Inner { "inner" } else { "outer" }
                ),
                E::TrimNotSquare => {
                    format!("{id}: the trim must be one closed loop on a plane square to the pull")
                }
                E::TrimCutsMould { at, clear } => format!(
                    "{id}: the trim outline runs {} the mould near ({}, {}, {}); it must clear it by the sheet's thickness",
                    if *clear < 0.0 { "inside" } else { "too close to" },
                    crate::values::fmt((at.x * 100.0).round() / 100.0),
                    crate::values::fmt((at.y * 100.0).round() / 100.0),
                    crate::values::fmt((at.z * 100.0).round() / 100.0)
                ),
                E::Offset(e) => format!(
                    "{id}: the sheet cannot be offset off the tool by its thickness ({e:?}): a round on the tool \
                     is tighter than the sheet is thick, or its walls meet"
                ),
                E::Extrude(e) => format!("{id}: the trim or tool outline: {e:?}"),
                E::Bool(e) => format!("{id}: the kernel's boolean refused: {e:?}"),
                E::Mesh => format!("{id}: the mould could not be meshed"),
            },
        }
    }

    pub fn data(&self) -> Vec<(&'static str, Json)> {
        match self {
            ThermoErr::UnknownMaterial { name, known, .. } => vec![
                ("name", name.as_str().into()),
                (
                    "known",
                    Json::Arr(known.iter().map(|k| k.as_str().into()).collect()),
                ),
            ],
            ThermoErr::NotABody { id, .. } => vec![("id", id.as_str().into())],
            ThermoErr::Kernel { id, error, .. } => {
                let mut d = vec![("id", id.as_str().into())];
                if let ft::FormError::TrimCutsMould { at, clear } = error {
                    d.push(("at", p3(*at)));
                    d.push(("clear", (*clear).into()));
                }
                d
            }
        }
    }
}

/// A check the part failed: the build stands.
#[derive(Clone, Debug, PartialEq)]
pub struct ThermoNote {
    pub span: Span,
    pub id: String,
    pub v: Violation,
    /// The features that made the faces or edges it names.
    pub made_by: Vec<String>,
    pub fixes: Vec<Op>,
    /// How many places it was found (the first's are given).
    pub count: usize,
}

fn note_kind(v: &Violation) -> &'static str {
    use Violation as V;
    match v {
        V::DraftTooLow { .. } => "DraftTooLow",
        V::Undercut { .. } => "Undercut",
        V::RadiusTooTight { .. } => "RadiusTooTight",
        V::SharpEdge { .. } => "SharpEdge",
        V::DrawTooDeep { .. } => "DrawTooDeep",
        V::AreaDraw { .. } => "AreaDrawTooHigh",
        V::Webbing { .. } => "Webbing",
        V::ShrinkNotApplied { .. } => "ShrinkNotApplied",
        V::VentTooLarge { .. } => "VentTooLarge",
        V::VentMissing { .. } => "VentMissing",
        V::GaugeNotStocked { .. } => "GaugeNotStocked",
        V::SheetTooSmall { .. } => "SheetTooSmall",
        V::SheetTooLarge { .. } => "SheetTooLarge",
        V::TooDeep { .. } => "TooDeep",
        V::StandIn { .. } => "StandIn",
    }
}

impl ThermoNote {
    pub fn kind(&self) -> &'static str {
        note_kind(&self.v)
    }

    pub fn severity(&self) -> &'static str {
        match self.v {
            Violation::StandIn { .. } => "info",
            _ => "violation",
        }
    }

    /// Where it is, if it is somewhere.
    pub fn at(&self) -> Option<Point3> {
        use Violation as V;
        match &self.v {
            V::DraftTooLow { at, .. }
            | V::Undercut { at, .. }
            | V::RadiusTooTight { at, .. }
            | V::SharpEdge { at, .. }
            | V::DrawTooDeep { at, .. }
            | V::Webbing { at, .. }
            | V::VentMissing { at, .. } => Some(*at),
            _ => None,
        }
    }

    pub fn text(&self) -> String {
        let t = self.text_one();
        if self.count > 1 {
            format!("{t} ({} places like it)", self.count)
        } else {
            t
        }
    }

    fn text_one(&self) -> String {
        use Violation as V;
        let by = if self.made_by.is_empty() {
            String::new()
        } else {
            format!(" (from {})", self.made_by.join(", "))
        };
        match &self.v {
            V::DraftTooLow { draft, min, .. } => format!(
                "a face of the tool{by} is drafted {}; the material needs {} on this side",
                deg(*draft),
                deg(*min)
            ),
            V::Undercut { draft, .. } => format!(
                "a face of the tool{by} leans {} under the pull: the formed part locks on it",
                deg(-draft)
            ),
            V::RadiusTooTight {
                radius, min, convex, ..
            } => format!(
                "a {} round on the tool{by} is {}; the sheet needs at least {}",
                if *convex { "convex" } else { "concave" },
                mm(*radius),
                mm(*min)
            ),
            V::SharpEdge { convex, min, .. } => format!(
                "a sharp {} edge on the tool{by}: round it to at least {}",
                if *convex { "convex" } else { "concave" },
                mm(*min)
            ),
            V::DrawTooDeep {
                depth, width, max, ..
            } => format!(
                "a draw {} deep over {} across (H:D {:.2}); the material takes {:.2}",
                mm(*depth),
                mm(*width),
                depth / width,
                max
            ),
            V::AreaDraw { adr, max } => format!(
                "the sheet's area grows {adr:.2} times over the opening (areal draw ratio); the material takes {max:.2}"
            ),
            V::Webbing {
                depth, gap, min_gap, ..
            } => format!(
                "raised parts {} apart and {} deep between them: the sheet webs; leave at least {}",
                mm(*gap),
                mm(*depth),
                mm(*min_gap)
            ),
            V::ShrinkNotApplied { shrink, .. } => format!(
                "the tool is not grown by the shrink ({:.2}%): a surface of the mould has no exact scaled form, \
                 so the part comes out that much small",
                shrink * 100.0
            ),
            V::VentTooLarge { d, max } => {
                format!("vents {} across mark the part; at most {}", mm(*d), mm(*max))
            }
            V::VentMissing { at, .. } => format!(
                "no vent where the sheet meets the tool last, near ({:.1}, {:.1}, {:.1})",
                at.x, at.y, at.z
            ),
            V::GaugeNotStocked { t, nearest } => {
                format!("the shop stocks no {} sheet; nearest {}", mm(*t), mm(*nearest))
            }
            V::SheetTooSmall { need, have } => format!(
                "the sheet is {} x {}; the trim and the clamp need {} x {}",
                mm(have.0),
                mm(have.1),
                mm(need.0),
                mm(need.1)
            ),
            V::SheetTooLarge { need, max } => format!(
                "the part needs a {} x {} sheet; the machine clamps at most {} x {}",
                mm(need.0),
                mm(need.1),
                mm(max.0),
                mm(max.1)
            ),
            V::TooDeep { depth, max } => {
                format!(
                    "a draw {} deep; the machine draws at most {}",
                    mm(*depth),
                    mm(*max)
                )
            }
            V::StandIn { material, fields } => format!(
                "{material}'s {} are stand-ins, not sourced numbers: give the shop's own",
                fields.join(", ")
            ),
        }
    }

    pub fn json(&self, src: &str) -> Json {
        let (l, c, ..) = self.span.lines(src);
        let mut f: Vec<(&str, Json)> = vec![
            ("kind", self.kind().into()),
            ("severity", self.severity().into()),
            ("id", self.id.as_str().into()),
        ];
        if let Some(at) = self.at() {
            f.push(("at", p3(at)));
        }
        if self.count > 1 {
            f.push(("count", self.count.into()));
        }
        if !self.made_by.is_empty() {
            f.push((
                "made_by",
                Json::Arr(self.made_by.iter().map(|m| m.as_str().into()).collect()),
            ));
        }
        f.push(("line", (l as f64).into()));
        f.push(("col", (c as f64).into()));
        f.push(("text", self.text().into()));
        f.push(("fixes", Json::Arr(self.fixes.iter().map(Op::json).collect())));
        Json::obj(f)
    }
}

fn thermo_err(e: ThermoErr) -> Error {
    Error::Thermo(Box::new(e))
}

fn wrong(span: Span, what: &'static str, want: Ty, v: &Value) -> Error {
    Error::WrongType {
        span,
        what,
        want,
        found: v.ty(),
    }
}

/// SetParam on the param that drives `n`, else SetArg `arg` of `id` to the literal.
fn set(n: Option<&Num>, id: &str, arg: &str, value: f64, unit: &str) -> Op {
    if let Some(n) = n
        && let Src::Driving(p) = &n.src
    {
        let value = if n.dim == Dim::Angle {
            value.to_degrees()
        } else {
            value
        };
        return Op::SetParam {
            param: p.clone(),
            value: Literal { value, dim: n.dim },
        };
    }
    let shown = if unit == "deg" { value.to_degrees() } else { value };
    Op::SetArg {
        id: id.into(),
        arg: arg.into(),
        text: format!("{}{unit}", crate::values::fmt(shown)),
    }
}

/// Up to the next tenth.
fn up(x: f64) -> f64 {
    (x * 10.0 - 1e-9).ceil() / 10.0
}

pub fn call(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, name, span)?;
    match name {
        "thermo_material" => material(ev, args, span),
        "thermo_shop" => shop(ev, args, span),
        _ => thermoform(ev, args, span),
    }
}

fn material(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let keys = [
        "name",
        "shrink",
        "draft_male",
        "draft_female",
        "min_radius",
        "min_inside_radius",
        "max_hd",
        "max_adr",
        "min_gap",
        "max_vent",
        "min_wall",
        "forming",
    ];
    let a = Args::bind(ev, "thermo_material", &keys, args, span)?;
    let name = match a.req("name")? {
        Value::Str(s) => s.to_string(),
        o => return Err(wrong(a.span("name"), "the material's name", Ty::Str, &o)),
    };
    // a material the placeholder knows starts from its row; what is given here is sourced
    let mut m = ft::placeholder().material(&name).cloned().unwrap_or_else(|| {
        let mut m = ft::placeholder().materials[0].clone();
        m.name = name.clone();
        m.stand_in = vec![
            "shrink",
            "draft_male",
            "draft_female",
            "min_radius",
            "min_inside_radius",
            "max_hd",
            "max_adr",
            "min_gap",
            "max_vent",
            "min_wall",
        ];
        m
    });
    let given = |k: &'static str, m: &mut ft::Material| m.stand_in.retain(|s| *s != k);
    if let Some(n) = a.num("shrink", RATIO)? {
        m.shrink = n.v;
        given("shrink", &mut m);
    }
    for (k, slot) in [("draft_male", 0), ("draft_female", 1)] {
        if let Some(n) = a.num(k, ANG)? {
            if slot == 0 {
                m.draft_male = n.v;
            } else {
                m.draft_female = n.v;
            }
            given(k, &mut m);
        }
    }
    for k in ["min_radius", "min_inside_radius", "max_vent"] {
        let Some(v) = a.opt(k) else { continue };
        let p = per_t(v, a.span(k))?;
        match k {
            "min_radius" => m.min_radius = p,
            "min_inside_radius" => m.min_inside_radius = p,
            _ => m.max_vent = p,
        }
        given(k, &mut m);
    }
    for k in ["max_hd", "max_adr", "min_gap", "min_wall"] {
        if let Some(n) = a.num(k, RATIO)? {
            match k {
                "max_hd" => m.max_hd = n.v,
                "max_adr" => m.max_adr = n.v,
                "min_gap" => m.min_gap_ratio = n.v,
                _ => m.min_wall = n.v,
            }
            given(k, &mut m);
        }
    }
    if let Some(v) = a.opt("forming") {
        let (lo, hi) = pair(v, a.span("forming"), RATIO)?;
        m.forming = (lo, hi);
    }
    Ok(Value::Thermo(Rc::new(ThermoVal::Material(Rc::new(m)))))
}

/// A length, or per_t(F, plus: L): F t + L.
fn per_t(v: Value, span: Span) -> R<ft::PerT> {
    match v {
        Value::Sheet(s) => match &*s {
            crate::sheet::SheetVal::Scaled(x) => Ok(ft::PerT {
                per_t: x.per_t.v,
                plus: x.plus.v,
            }),
            _ => Err(wrong(
                span,
                "a length or per_t(..)",
                Ty::Scaled,
                &Value::Sheet(s.clone()),
            )),
        },
        v => Ok(ft::PerT {
            per_t: 0.0,
            plus: num_of(v, LEN, span, "a length")?.v,
        }),
    }
}

fn pair(v: Value, span: Span, dims: &[Dim]) -> R<(f64, f64)> {
    match &v {
        Value::Tuple(t) if t.len() == 2 => Ok((
            num_of(t[0].clone(), dims, span, "the first")?.v,
            num_of(t[1].clone(), dims, span, "the second")?.v,
        )),
        o => Err(wrong(span, "a pair (a, b)", Ty::Vec2, o)),
    }
}

fn shop(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let a = Args::bind(
        ev,
        "thermo_shop",
        &["materials", "max_sheet", "max_depth", "clamp", "gauges"],
        args,
        span,
    )?;
    let mut shop = ft::placeholder();
    if let Some(v) = a.opt("materials") {
        let Value::List(xs) = &v else {
            return Err(wrong(a.span("materials"), "materials: a list", Ty::List, &v));
        };
        for x in xs.iter() {
            let Value::Thermo(t) = x else {
                return Err(wrong(
                    a.span("materials"),
                    "a thermo_material(..)",
                    Ty::ThermoMaterial,
                    x,
                ));
            };
            let ThermoVal::Material(m) = &**t else {
                return Err(wrong(
                    a.span("materials"),
                    "a thermo_material(..)",
                    Ty::ThermoMaterial,
                    x,
                ));
            };
            shop.materials.retain(|o| !o.name.eq_ignore_ascii_case(&m.name));
            shop.materials.push((**m).clone());
        }
    }
    if let Some(v) = a.opt("max_sheet") {
        shop.machine.max_sheet = Some(pair(v, a.span("max_sheet"), LEN)?);
    }
    if let Some(n) = a.num("max_depth", LEN)? {
        shop.machine.max_depth = Some(n.v);
    }
    if let Some(n) = a.num("clamp", LEN)? {
        shop.machine.clamp = n.v;
    }
    if let Some(v) = a.opt("gauges") {
        let Value::List(xs) = &v else {
            return Err(wrong(
                a.span("gauges"),
                "gauges: a list of thicknesses",
                Ty::List,
                &v,
            ));
        };
        shop.machine.gauges = xs
            .iter()
            .map(|x| num_of(x.clone(), LEN, a.span("gauges"), "a gauge").map(|n| n.v))
            .collect::<R<_>>()?;
    }
    Ok(Value::Thermo(Rc::new(ThermoVal::Shop(Rc::new(shop)))))
}

const ARGS: &[&str] = &[
    "mould",
    "t",
    "material",
    "side",
    "pull",
    "trim",
    "flange",
    "sheet",
    "shop",
    "shrink",
    "base",
    "margin",
    "vent",
    "vents",
    "vent_pitch",
];

/// The arguments' numbers as written, for fixes.
struct Nums {
    t: Num,
    vent: Option<Num>,
}

/// A body's names and vertices, hashed: the memo's key for what was formed over it.
fn body_key(b: &Body) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let mut mix = |x: u64| h = (h ^ x).wrapping_mul(0x0100_0000_01b3);
    for v in &b.vertices {
        mix(v.point.x.to_bits());
        mix(v.point.y.to_bits());
        mix(v.point.z.to_bits());
    }
    for f in &b.faces {
        mix(f.name.0);
    }
    h
}

/// The mould's solid among `solids`: the one its feature made, else the only one.
fn mould_of(solids: &[Solid], idx: usize) -> Option<&Solid> {
    solids
        .iter()
        .find(|s| s.id == idx)
        .or(if solids.len() == 1 { solids.first() } else { None })
}

fn thermoform(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let id = ev.take_id("thermoform", span)?;
    let a = Args::bind(ev, "thermoform", ARGS, args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let mould = match a.req("mould")? {
        Value::Feature(f) => f,
        o => {
            return Err(wrong(
                a.span("mould"),
                "the mould (the feature that made its body)",
                Ty::Feature,
                &o,
            ));
        }
    };
    let t = a.req_num("t", LEN)?;
    positive(&t, a.span("t"), "the sheet's thickness")?;
    let shop = match a.opt("shop") {
        None => ft::placeholder(),
        Some(Value::Thermo(v)) if matches!(&*v, ThermoVal::Shop(_)) => match &*v {
            ThermoVal::Shop(s) => (**s).clone(),
            ThermoVal::Material(_) => unreachable!("matched"),
        },
        Some(o) => return Err(wrong(a.span("shop"), "a thermo_shop(..)", Ty::ThermoShop, &o)),
    };
    let m = match a.req("material")? {
        Value::Str(s) => match shop.material(&s) {
            Some(m) => m.clone(),
            None => {
                return Err(thermo_err(ThermoErr::UnknownMaterial {
                    span: a.span("material"),
                    id: id.clone(),
                    name: s.to_string(),
                    known: shop.materials.iter().map(|m| m.name.clone()).collect(),
                }));
            }
        },
        Value::Thermo(v) => match &*v {
            ThermoVal::Material(m) => (**m).clone(),
            ThermoVal::Shop(_) => {
                return Err(wrong(
                    a.span("material"),
                    "a material",
                    Ty::ThermoMaterial,
                    &Value::Thermo(v.clone()),
                ));
            }
        },
        o => return Err(wrong(a.span("material"), "the material's name", Ty::Str, &o)),
    };
    let side = match crate::wired::word(&a, "side", &["inner", "outer"])? {
        Some("outer") => Side::Outer,
        _ => Side::Inner,
    };
    let pull = match a.opt("pull") {
        None => UnitVec3::Z,
        Some(v) => crate::eval::unit_or_z(dir_of(v, a.span("pull"))?),
    };
    let trim = match a.opt("trim") {
        None => None,
        Some(v) => Some(profile_of(ev, v, a.span("trim"))?.1),
    };
    let len = |k: &'static str, d: f64| -> R<f64> { Ok(a.num(k, LEN)?.map_or(d, |n| n.v)) };
    let flange = len("flange", 10.0)?;
    let base = len("base", 10.0)?;
    let margin = len("margin", 5.0)?;
    let vent_pitch = len("vent_pitch", 25.0)?;
    let shrink = a.num("shrink", RATIO)?.map_or(m.shrink, |n| n.v);
    let (vent, vent_num) = match a.opt("vent") {
        Some(Value::Bool(false)) => (None, None),
        Some(v) => {
            let n = num_of(v, LEN, a.span("vent"), "the vent's diameter")?;
            (Some(n.v), Some(n))
        }
        None => (Some(m.max_vent.at(t.v).min(1.0)), None),
    };
    let vents = match a.opt("vents") {
        None => None,
        Some(Value::List(xs)) => Some(
            xs.iter()
                .map(|x| pair(x.clone(), a.span("vents"), LEN).map(|(x, y)| Point2::new(x, y)))
                .collect::<R<Vec<_>>>()?,
        ),
        Some(o) => return Err(wrong(a.span("vents"), "vents: a list of (x, y)", Ty::List, &o)),
    };
    let sheet = match a.opt("sheet") {
        None => None,
        Some(v) => Some(pair(v, a.span("sheet"), LEN)?),
    };
    let spec = ft::Spec {
        side,
        pull,
        t: t.v,
        trim,
        flange,
        shrink,
        base,
        margin,
        vent,
        vents,
        vent_pitch,
        sheet,
    };
    let nums = Nums {
        t: t.clone(),
        vent: vent_num,
    };
    let hid = history_id(&id);
    let text = format!("{spec:?} {:?} {}", m, mould.idx);
    let marker = Feature::Param(Box::new(forge_model::Param {
        name: format!("thermoform:{id}"),
        expr: text.clone(),
        unit: forge_model::Unit::None,
        value: 0.0,
    }));
    let solid = ev.b.features.len();
    let before_n = solid;
    let mut made: Option<(Rc<ft::Formed>, Body)> = None;
    let mut failed: Option<Body> = None;
    let (sp, idc) = (span, id.clone());
    let r = ev.push_made(
        &id,
        "thermoform",
        span,
        vec![dim("thickness", &t)],
        hid,
        |b: &mut Builder| {
            b.push_with(marker, hid, |before: &[Solid]| {
                let s = mould_of(before, mould.idx).ok_or_else(|| {
                    thermo_err(ThermoErr::NotABody {
                        span: sp,
                        id: mould.id.clone(),
                    })
                })?;
                let f = ft::form(&s.body, &spec, hid).map_err(|error| {
                    failed = Some(s.body.clone());
                    thermo_err(ThermoErr::Kernel {
                        span: sp,
                        id: idc.clone(),
                        error,
                    })
                })?;
                let mut out: Vec<Solid> = before.iter().filter(|x| x.id != s.id).cloned().collect();
                out.push(Solid {
                    id: s.id,
                    body: f.tool.clone(),
                });
                out.push(Solid {
                    id: solid,
                    body: f.part.clone(),
                });
                made = Some((Rc::new(f), s.body.clone()));
                Ok(out)
            })
        },
    );
    // the solids failed: what the checks say about the mould still stands
    if r.is_err()
        && let Some(mb) = failed
        && let Ok(f) = ft::analyse(&mb, &spec)
    {
        notes(ev, &id, span, before_n, &mb, &f, &spec, &m, &shop, &nums);
    }
    let (idx, _) = r?;
    let ok = ev.b.building && ev.feats.last().is_some_and(|f| f.status == Status::Ok);
    // the mould as it stood before, for the checks and the memo's key
    let mould_body = match &made {
        Some((_, b)) => Some(b.clone()),
        None if ok => {
            ev.b.stages
                .get(before_n.wrapping_sub(1))
                .and_then(|st| mould_of(&st.solids, mould.idx).map(|s| s.body.clone()))
        }
        None => None,
    };
    if let Some(mb) = mould_body {
        let key = format!("thermoform {text} {}", body_key(&mb));
        let formed = match made {
            Some((f, _)) => ev.b.memo(key, || Some(f)),
            None => ev.b.memo(key, || ft::form(&mb, &spec, hid).ok().map(Rc::new)),
        };
        if let Some(f) = formed {
            notes(ev, &id, span, before_n, &mb, &f, &spec, &m, &shop, &nums);
            ev.thermos.push(ThermoOut {
                id: id.clone(),
                material: m.name.clone(),
                side,
                t: t.v,
                formed: f,
            });
        }
    }
    Ok(Value::Feature(Rc::new(Feat {
        id,
        kind: "thermoform",
        idx,
        hid,
        shape: None,
        axis: None,
        prof: None,
    })))
}

/// The checks, as warnings with fixes.
#[allow(clippy::too_many_arguments)]
fn notes(
    ev: &mut Eval,
    id: &str,
    span: Span,
    before: usize,
    mould: &Body,
    f: &ft::Formed,
    spec: &ft::Spec,
    m: &ft::Material,
    shop: &ft::FormShop,
    nums: &Nums,
) {
    let found = ft::check(mould, f, spec, m, shop);
    if found.is_empty() {
        return;
    }
    let origin = ev.b.origin(before);
    // the features that made a face, by its name (or the faces either side of an edge)
    let feats_of = |names: &[forge_topo::Name]| -> Vec<(String, &'static str, usize)> {
        let mut out: Vec<(String, &'static str, usize)> = vec![];
        for n in names {
            for &i in origin.get(n).map(|v| v.as_slice()).unwrap_or(&[]) {
                if let Some(fi) = ev.feats.iter().find(|x| x.idx == i)
                    && !out.iter().any(|o| o.2 == i)
                {
                    out.push((fi.id.clone(), fi.kind, i));
                }
            }
        }
        out
    };
    let edge_faces = |e: forge_topo::Name| -> Vec<forge_topo::Name> {
        mould
            .edge_ids()
            .find(|&x| mould.edge(x).name == e)
            .map(|x| {
                mould
                    .edge(x)
                    .coedges
                    .iter()
                    .map(|&c| mould.face(mould.coedge_face(c)).name)
                    .collect()
            })
            .unwrap_or_default()
    };
    let num_of_feat = |i: usize, what: &str| -> Option<Num> {
        ev.feats
            .iter()
            .find(|x| x.idx == i)
            .and_then(|fi| fi.dims.iter().find(|d| d.what == what).map(|d| d.num.clone()))
    };
    let mut out = vec![];
    for v in found {
        let (made, fixes): (Vec<(String, &'static str, usize)>, Vec<Op>) = match &v {
            Violation::DraftTooLow { face, draft, .. } | Violation::Undercut { face, draft, .. } => {
                let want = if spec.side == Side::Inner {
                    m.draft_male
                } else {
                    m.draft_female
                };
                let made = feats_of(&[*face]);
                // an extrude's taper cures its own sides, a little under the pull at most
                let side = *draft > -FRAC_PI_4;
                let fixes = made
                    .iter()
                    .filter(|x| {
                        side && x.1 == "extrude"
                            && num_of_feat(x.2, "taper").is_none_or(|n| n.v < want - 1e-9)
                    })
                    .take(1)
                    .map(|x| {
                        set(
                            num_of_feat(x.2, "taper").as_ref(),
                            &x.0,
                            "taper",
                            up(want.to_degrees()).to_radians(),
                            "deg",
                        )
                    })
                    .collect();
                (made, fixes)
            }
            Violation::RadiusTooTight { face, min, .. } => {
                let made = feats_of(&[*face]);
                let fixes = made
                    .iter()
                    .filter(|x| x.1 == "fillet")
                    .take(1)
                    .map(|x| set(num_of_feat(x.2, "radius").as_ref(), &x.0, "r", up(*min), ""))
                    .collect();
                (made, fixes)
            }
            Violation::SharpEdge { edge, min, .. } => {
                let sides = edge_faces(*edge);
                let made = feats_of(&sides);
                // an extrude's cap: all its edges; else the edges where two features meet
                let cap = sides.iter().find_map(|n| {
                    feats_of(&[*n])
                        .into_iter()
                        .filter(|x| x.1 == "extrude")
                        .find_map(|x| {
                            ["end", "start"].into_iter().find_map(|c| {
                                let w = forge_topo::Name::feature(history_id(&x.0)).with_str(c);
                                (*n == w || n.is_piece_of(w)).then(|| format!("edges(of: {}.{c})", x.0))
                            })
                        })
                });
                let between = match (
                    feats_of(&sides[..1.min(sides.len())]).first(),
                    feats_of(&sides[1.min(sides.len())..]).first(),
                ) {
                    (Some(x), Some(y)) if x.2 != y.2 => Some(format!("edges(between: ({}, {}))", x.0, y.0)),
                    _ => None,
                };
                let fixes = between
                    .or(cap)
                    .map(|sel| Op::AddFeature {
                        before: Some(id.to_string()),
                        text: format!(
                            "{}_round = fillet({sel}, {})\n",
                            made.first()
                                .map_or(id, |x| x.0.as_str())
                                .replace(['[', ']', '.'], "_"),
                            crate::values::fmt(up(*min))
                        ),
                    })
                    .into_iter()
                    .collect();
                (made, fixes)
            }
            Violation::VentTooLarge { max, .. } => (
                vec![],
                vec![set(
                    nums.vent.as_ref(),
                    id,
                    "vent",
                    (max * 100.0).floor() / 100.0,
                    "",
                )],
            ),
            Violation::VentMissing { at, .. } => {
                let p = f.frame.point_to_local(*at);
                let mut list: Vec<String> = spec
                    .vents
                    .iter()
                    .flatten()
                    .map(|q| format!("({}, {})", crate::values::fmt(q.x), crate::values::fmt(q.y)))
                    .collect();
                list.push(format!(
                    "({}, {})",
                    crate::values::fmt((p.x * 10.0).round() / 10.0),
                    crate::values::fmt((p.y * 10.0).round() / 10.0)
                ));
                (
                    vec![],
                    vec![Op::SetArg {
                        id: id.into(),
                        arg: "vents".into(),
                        text: format!("[{}]", list.join(", ")),
                    }],
                )
            }
            Violation::GaugeNotStocked { nearest, .. } => {
                (vec![], vec![set(Some(&nums.t), id, "t", *nearest, "")])
            }
            Violation::SheetTooSmall { need, .. } => (
                vec![],
                vec![Op::SetArg {
                    id: id.into(),
                    arg: "sheet".into(),
                    text: format!(
                        "({}, {})",
                        crate::values::fmt(need.0.ceil()),
                        crate::values::fmt(need.1.ceil())
                    ),
                }],
            ),
            _ => (vec![], vec![]),
        };
        let made_by: Vec<String> = made.into_iter().map(|x| x.0).collect();
        // one note for each kind of fault the same features make, with how many places
        let same = out.iter_mut().find_map(|w| match w {
            Warning::Thermo(n)
                if n.kind() == note_kind(&v)
                    && n.made_by == made_by
                    && n.fixes == fixes
                    && n.at().is_some() =>
            {
                Some(n)
            }
            _ => None,
        });
        match same {
            Some(n) => n.count += 1,
            None => out.push(Warning::Thermo(Box::new(ThermoNote {
                span,
                id: id.into(),
                v,
                made_by,
                fixes,
                count: 1,
            }))),
        }
    }
    ev.warnings.extend(out);
}
