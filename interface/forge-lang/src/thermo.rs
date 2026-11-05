//! Thermoforming in Forge, on forge-thermo. The user models the part's shape; `thermoform(SHAPE,
//! t:, material:, ..)` generates the rest: the shape's surface is the part's inner skin (`side:
//! "inner"`, a male mould) or its outer one (`"outer"`, a female mould). It makes two bodies: the
//! formed part (a new body, named by the feature) and the mould, which takes the shape body's place.
//! The shop's checks and the wall's thickness run on every build, typed, with fixes where a param
//! or argument cures them. Materials and machines are data: `thermo_material(..)`, `thermo_shop(..)`.
use crate::build::{Builder, history_id};
use crate::builtins::{Args, LEN, dim, dir_of, not_in_sketch, num_of, positive, profile_of};
use crate::error::{Error, Literal, Op, Warning};
use crate::eval::{Eval, Feat, R, Status, Value};
use crate::syntax::{Arg, Span};
use crate::values::{Dim, Num, Src, Ty};
use forge_agent::json::Json;
use forge_math::{Point2, Point3, UnitVec3};
use forge_model::{Feature, Solid};
use forge_thermo::{self as ft, Tool, Violation};
use forge_topo::Body;
use std::f64::consts::FRAC_PI_4;
use std::rc::Rc;

pub const BUILTINS: &[&str] = &["thermoform", "thermo_material", "thermo_shop"];

const RATIO: &[Dim] = &[Dim::Ratio];
const ANG: &[Dim] = &[Dim::Angle];
/// A temperature, deg C: a bare number.
const ANY: &[Dim] = &[Dim::Ratio, Dim::Length];

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
    pub tool: Tool,
    pub from: ft::Given,
    pub t: f64,
    pub formed: Rc<ft::Formed>,
    /// The thickness model's result (None: `walls: false`).
    pub walls: Option<Rc<ft::Stretch>>,
    /// The sheet's temperature and the law the model read at it.
    pub temperature: f64,
    pub law: ft::LawAt,
    /// The thinnest wall the material allows, over the sheet's.
    pub min_wall: f64,
    /// The material's density, kg/m3 (None: its record has none).
    pub density: Option<f64>,
    /// The blank asked for (None: the trim's box and the clamp's).
    pub sheet: Option<(f64, f64)>,
    /// The part as the thickness model leaves it, and the modelled part's exact mass, unit density.
    pub sim: Option<Rc<ft::Simulated>>,
    pub modelled: Option<forge_topo::mass::MassProps>,
    /// The part's solids, by the id the stages give them: the mould's and the formed part's.
    pub mould_solid: usize,
    pub part_solid: usize,
}

/// The clamp's margin round the trim, each side, for the blank.
const CLAMP: f64 = 25.0;

impl ThermoOut {
    /// The thinnest wall allowed, mm.
    pub fn limit(&self) -> f64 {
        self.min_wall * self.t
    }

    /// Passes: the thinnest wall at the limit or over, and the sheet reaching the tool everywhere.
    pub fn passes(&self) -> Option<bool> {
        let w = self.walls.as_ref()?;
        Some(w.min.0 * self.t >= self.limit() && w.unformed.is_empty())
    }

    /// The blank, (W, L) on the trim plane's axes.
    pub fn blank_size(&self) -> (f64, f64) {
        let (lo, hi) = self.formed.report.trim_box;
        self.sheet
            .unwrap_or((hi.x - lo.x + 2.0 * CLAMP, hi.y - lo.y + 2.0 * CLAMP))
    }

    /// The blank where the sheet starts, before it is formed.
    pub fn blank(&self) -> forge_mesh::TriMesh {
        let f = &self.formed;
        let (lo, hi) = f.report.trim_box;
        let z = self
            .walls
            .as_ref()
            .and_then(|w| w.steps.first())
            .and_then(|s| s.nodes.first())
            .map_or(0.0, |p| {
                f.frame
                    .point_to_local(Point3::new(p[0] as f64, p[1] as f64, p[2] as f64))
                    .z
            });
        let mid = Point2::new(0.5 * (lo.x + hi.x), 0.5 * (lo.y + hi.y));
        ft::blank(&f.frame, mid, self.blank_size(), z, self.t)
    }

    /// The wall at each point of the sheet on the part (inside the trim, on the tool): where
    /// (world, the tool's side) and how thick, mm.
    pub fn field(&self) -> Vec<(Point3, f64)> {
        let Some(w) = &self.walls else { return vec![] };
        let f = &self.formed;
        let poly = ft::build::polygon(&f.trim);
        (0..w.nodes.len())
            .filter(|&k| {
                let l = f.frame.point_to_local(w.nodes[k]);
                w.touched[k] && ft::build::inside(&poly, Point2::new(l.x, l.y)).0
            })
            .map(|k| (w.nodes[k], w.node_ratio[k] * self.t))
            .collect()
    }

    /// A mass, centre and inertia as JSON: the volume's mm3, kg, mm, kg mm2.
    fn mass_json(&self, volume: f64, centroid: Point3, inertia: [[f64; 3]; 3]) -> Json {
        let kg = self.density.map(|d| d * 1e-9);
        let m = |k: f64| kg.map_or(Json::Null, |kg| (k * kg).into());
        let rows = |f: &dyn Fn(f64) -> Json| {
            Json::Arr(
                inertia
                    .iter()
                    .map(|r| Json::Arr(r.iter().map(|x| f(*x)).collect()))
                    .collect(),
            )
        };
        Json::obj(vec![
            ("volume_mm3", volume.into()),
            ("mass_kg", m(volume)),
            ("cog_mm", p3(centroid)),
            ("inertia_kg_mm2", rows(&m)),
        ])
    }
}

impl ThermoOut {
    pub fn json(&self) -> Json {
        let r = &self.formed.report;
        let (lo, hi) = r.trim_box;
        let walls = self.walls.as_ref().map(|w| {
            (
                "walls",
                Json::obj(vec![
                    ("min", (w.min.0 * self.t).into()),
                    ("min_at", p3(w.min.1)),
                    ("max", (w.max.0 * self.t).into()),
                    ("max_at", p3(w.max.1)),
                    ("mean", (w.mean * self.t).into()),
                    ("min_ratio", w.min.0.into()),
                    ("unformed", w.unformed.len().into()),
                    (
                        "model",
                        "membrane, sticking contact, isothermal: an estimate".into(),
                    ),
                    ("temperature_c", self.temperature.into()),
                    (
                        "law",
                        Json::Arr(
                            self.law
                                .law
                                .terms
                                .iter()
                                .map(|(m, a)| Json::Arr(vec![(*m).into(), (*a).into()]))
                                .collect(),
                        ),
                    ),
                    ("law_source", self.law.source.as_str().into()),
                    ("law_stand_in", self.law.stand_in.into()),
                ]),
            )
        });
        let mut f = vec![
            ("id", self.id.as_str().into()),
            ("material", self.material.as_str().into()),
            (
                "tool",
                match self.tool {
                    Tool::Male => "male",
                    Tool::Female => "female",
                }
                .into(),
            ),
            (
                "from",
                match self.from {
                    ft::Given::Outer => "outer",
                    ft::Given::Wall => "wall",
                    ft::Given::Inner => "inner",
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
            ("mould_scale", r.scale.into()),
            (
                "vents",
                Json::Arr(
                    r.vents
                        .iter()
                        .map(|p| Json::Arr(vec![p.x.into(), p.y.into(), p.z.into()]))
                        .collect(),
                ),
            ),
        ];
        f.extend(walls);
        f.push((
            "limit",
            Json::obj(vec![
                ("min_wall", self.limit().into()),
                ("min_wall_of_t", self.min_wall.into()),
            ]),
        ));
        f.push((
            "verdict",
            match self.passes() {
                Some(true) => "pass".into(),
                Some(false) => "warn".into(),
                None => "no model (walls: false)".into(),
            },
        ));
        let mut mass = vec![("density_kg_m3", self.density.map_or(Json::Null, Json::from))];
        if let Some(m) = &self.modelled {
            mass.push(("modelled", self.mass_json(m.volume, m.centroid, m.inertia)));
        }
        if let Some(s) = &self.sim {
            mass.push((
                "simulated",
                self.mass_json(s.mass.volume, s.mass.centroid, s.mass.inertia),
            ));
            mass.push(("wall_integral_mm3", s.sheet_volume.into()));
        }
        if let (Some(m), Some(s)) = (&self.modelled, &self.sim) {
            mass.push(("simulated_over_modelled", (s.mass.volume / m.volume).into()));
        }
        f.push(("mass", Json::obj(mass)));
        let (bw, bl) = self.blank_size();
        f.push((
            "blank",
            Json::obj(vec![
                ("size", Json::Arr(vec![bw.into(), bl.into()])),
                ("t", self.t.into()),
                (
                    "mass_kg",
                    self.density
                        .map_or(Json::Null, |d| (bw * bl * self.t * d * 1e-9).into()),
                ),
            ]),
        ));
        if self.walls.is_some() {
            let pts = self.field();
            f.push((
                "field",
                Json::obj(vec![
                    (
                        "columns",
                        Json::Arr(["x", "y", "z", "wall"].map(Json::from).to_vec()),
                    ),
                    ("unit", "mm".into()),
                    ("at", "the sheet's side on the tool, inside the trim".into()),
                    ("count", pts.len().into()),
                    (
                        "get",
                        "fg run FILE --json --field OUT.csv | --field inline".into(),
                    ),
                ]),
            ));
        }
        Json::obj(f)
    }

    /// The thickness field's points as JSON: [x, y, z, wall], mm.
    pub fn field_json(&self) -> Json {
        Json::Arr(
            self.field()
                .iter()
                .map(|(p, h)| Json::Arr(vec![p.x.into(), p.y.into(), p.z.into(), (*h).into()]))
                .collect(),
        )
    }

    /// The thickness field as CSV: x, y, z, wall, mm.
    pub fn field_csv(&self) -> String {
        let mut s = String::from("x_mm,y_mm,z_mm,wall_mm\n");
        for (p, h) in self.field() {
            s += &format!("{:.3},{:.3},{:.3},{:.4}\n", p.x, p.y, p.z, h);
        }
        s
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
    /// A thickness given for a wall that is not the wall's.
    WallNotT {
        span: Span,
        id: String,
        given: f64,
        measured: f64,
    },
    /// The shape names no body of the part.
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
            | ThermoErr::WallNotT { span, .. }
            | ThermoErr::NotABody { span, .. } => Some(*span),
        }
    }

    pub fn kind(&self) -> &'static str {
        use ft::FormError as E;
        match self {
            ThermoErr::UnknownMaterial { .. } => "UnknownMaterial",
            ThermoErr::NotABody { .. } => "NotABody",
            ThermoErr::WallNotT { .. } => "WallNotT",
            ThermoErr::Kernel { error, .. } => match error {
                E::Size { .. } => "ThermoSize",
                E::NoOpening { .. } => "NoOpening",
                E::TrimNotSquare => "TrimNotSquare",
                E::TrimCutsShape { .. } => "TrimCutsShape",
                E::Extrude(_) => "ThermoExtrude",
                E::Bool(_) => "ThermoBoolean",
                E::Offset(_) => "SheetOffset",
                E::Mesh => "ThermoMesh",
                E::NotAWall => "NotAWall",
                E::WallNotUniform { .. } => "WallNotUniform",
            },
        }
    }

    pub fn fixes(&self) -> Vec<Op> {
        match self {
            ThermoErr::Kernel {
                id,
                error: ft::FormError::NoOpening { tool },
                ..
            } => vec![Op::SetArg {
                id: id.clone(),
                arg: "tool".into(),
                text: if *tool == Tool::Male {
                    "\"female\""
                } else {
                    "\"male\""
                }
                .into(),
            }],
            ThermoErr::WallNotT { id, measured, .. } => vec![Op::SetArg {
                id: id.clone(),
                arg: "t".into(),
                text: crate::values::fmt((measured * 1000.0).round() / 1000.0),
            }],
            ThermoErr::Kernel {
                id,
                error: ft::FormError::WallNotUniform { .. } | ft::FormError::NotAWall,
                ..
            } => vec![Op::SetArg {
                id: id.clone(),
                arg: "from".into(),
                text: "\"outer\"".into(),
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
            ThermoErr::WallNotT {
                id, given, measured, ..
            } => format!(
                "{id}: t is {} but the wall is {} thick (a wall says its own thickness)",
                mm(*given),
                mm(*measured)
            ),
            ThermoErr::Kernel { id, error, .. } => match error {
                E::Size { what, value } => format!("{id}: {what} must be positive, not {value}"),
                E::NoOpening { tool } => format!(
                    "{id}: the shape has no flat face square to the pull at its {} for a {} mould's trim \
                     plane (the part leaves a male mould along the pull, a female one too, its opening first)",
                    if *tool == Tool::Male { "foot" } else { "top" },
                    if *tool == Tool::Male { "male" } else { "female" }
                ),
                E::NotAWall => format!(
                    "{id}: the shape is no wall: nothing hollow under it opens at its foot (from: \"wall\" \
                     takes a shelled part)"
                ),
                E::WallNotUniform { t, min, max } => format!(
                    "{id}: the wall is not one thickness: mostly {}, from {} near ({:.1}, {:.1}, {:.1}) to {} \
                     near ({:.1}, {:.1}, {:.1})",
                    mm(*t),
                    mm(min.0),
                    min.1.x,
                    min.1.y,
                    min.1.z,
                    mm(max.0),
                    max.1.x,
                    max.1.y,
                    max.1.z
                ),
                E::TrimNotSquare => {
                    format!("{id}: the trim must be one closed loop on a plane square to the pull")
                }
                E::TrimCutsShape { at, clear } => format!(
                    "{id}: the trim outline runs {} the shape near ({}, {}, {}); it must clear it by the sheet's thickness",
                    if *clear < 0.0 { "inside" } else { "too close to" },
                    crate::values::fmt((at.x * 100.0).round() / 100.0),
                    crate::values::fmt((at.y * 100.0).round() / 100.0),
                    crate::values::fmt((at.z * 100.0).round() / 100.0)
                ),
                E::Offset(e) => format!(
                    "{id}: the sheet cannot be offset off the shape by its thickness ({e:?}): a round on the shape \
                     is tighter than the sheet is thick, or its walls meet"
                ),
                E::Extrude(e) => format!("{id}: the trim or mould outline: {e:?}"),
                E::Bool(e) => format!("{id}: the kernel's boolean refused: {e:?}"),
                E::Mesh => format!("{id}: the shape could not be meshed"),
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
            ThermoErr::WallNotT {
                id, given, measured, ..
            } => vec![
                ("id", id.as_str().into()),
                ("given", (*given).into()),
                ("measured", (*measured).into()),
            ],
            ThermoErr::Kernel { id, error, .. } => {
                let mut d = vec![("id", id.as_str().into())];
                if let ft::FormError::TrimCutsShape { at, clear } = error {
                    d.push(("at", p3(*at)));
                    d.push(("clear", (*clear).into()));
                }
                if let ft::FormError::WallNotUniform { t, min, max } = error {
                    d.push(("t", (*t).into()));
                    d.push(("min", (min.0).into()));
                    d.push(("min_at", p3(min.1)));
                    d.push(("max", (max.0).into()));
                    d.push(("max_at", p3(max.1)));
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
        V::ThinWall { .. } => "ThinWall",
        V::Bridged { .. } => "Bridged",
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
            Violation::ThinWall { .. } | Violation::Bridged { .. } => "warning",
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
            | V::VentMissing { at, .. }
            | V::ThinWall { at, .. }
            | V::Bridged { at, .. } => Some(*at),
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
                "a face of the shape{by} is drafted {}; the material needs {} on this side",
                deg(*draft),
                deg(*min)
            ),
            V::Undercut { draft, .. } => format!(
                "a face of the shape{by} leans {} under the pull: the formed part locks on it",
                deg(-draft)
            ),
            V::RadiusTooTight {
                radius, min, convex, ..
            } => format!(
                "a {} round on the shape{by} is {}; the sheet needs at least {}",
                if *convex { "convex" } else { "concave" },
                mm(*radius),
                mm(*min)
            ),
            V::SharpEdge { convex, min, .. } => format!(
                "a sharp {} edge on the shape{by}: round it to at least {}",
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
                "the mould is not grown by the shrink ({:.2}%): a surface of the shape has no exact scaled form, \
                 so the part comes out that much small",
                shrink * 100.0
            ),
            V::VentTooLarge { d, max } => {
                format!("vents {} across mark the part; at most {}", mm(*d), mm(*max))
            }
            V::VentMissing { at, .. } => format!(
                "no vent where the sheet meets the mould last, near ({:.1}, {:.1}, {:.1})",
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
            V::ThinWall { ratio, min, t, .. } => format!(
                "the wall is predicted to thin to {} ({:.0}% of the {} sheet){by}; the material's least is {:.0}%: \
                 a thicker sheet, more draft or a larger round there, or a plug assist",
                mm(ratio * t),
                ratio * 100.0,
                mm(*t),
                min * 100.0
            ),
            V::Bridged { count, .. } => format!(
                "the modelled sheet never reaches the mould at {count} points inside the trim: it bridges there"
            ),
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
        "law",
        "law_temperature",
    ];
    let a = Args::bind(ev, "thermo_material", &keys, args, span)?;
    let name = match a.req("name")? {
        Value::Str(s) => s.to_string(),
        o => return Err(wrong(a.span("name"), "the material's name", Ty::Str, &o)),
    };
    // a material the placeholder knows starts from its row; what is given here is sourced
    let mut m = library(ev).material(&name).cloned().unwrap_or_else(|| {
        let mut m = library(ev).materials[0].clone();
        m.name = name.clone();
        m.stand_in = vec![
            "forming_c",
            "shrink",
            "draft_male_deg",
            "draft_female_deg",
            "min_radius",
            "min_inside_radius",
            "max_hd",
            "max_adr",
            "min_gap_ratio",
            "max_vent",
            "min_wall",
            "law",
        ];
        m
    });
    let given = |k: &'static str, m: &mut ft::Material| m.stand_in.retain(|s| *s != k);
    if let Some(n) = a.num("shrink", RATIO)? {
        m.shrink = n.v;
        given("shrink", &mut m);
    }
    for (k, field) in [
        ("draft_male", "draft_male_deg"),
        ("draft_female", "draft_female_deg"),
    ] {
        if let Some(n) = a.num(k, ANG)? {
            if k == "draft_male" {
                m.draft_male = n.v;
            } else {
                m.draft_female = n.v;
            }
            given(field, &mut m);
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
            given(if k == "min_gap" { "min_gap_ratio" } else { k }, &mut m);
        }
    }
    if let Some(v) = a.opt("forming") {
        let (lo, hi) = pair(v, a.span("forming"), RATIO)?;
        m.forming = (lo, hi);
        given("forming_c", &mut m);
    }
    // the shop's own law, Ogden terms (mu in MPa, alpha), at its temperature
    if let Some(v) = a.opt("law") {
        let Value::List(xs) = &v else {
            return Err(wrong(a.span("law"), "law: [(mu, alpha), ..]", Ty::List, &v));
        };
        let terms = xs
            .iter()
            .map(|x| pair(x.clone(), a.span("law"), RATIO))
            .collect::<R<Vec<_>>>()?;
        let temperature = a
            .num("law_temperature", ANY)?
            .map_or(m.forming_temperature(), |n| n.v);
        m.fits = vec![ft::Fit {
            law: ft::Law { terms },
            temperature,
            test: "the shop's own".into(),
            source: format!("thermo_material(\"{name}\", law: ..)"),
            stand_in: false,
        }];
        m.curves.clear();
        given("law", &mut m);
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
    let mut shop = library(ev);
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
    "shape",
    "t",
    "material",
    "from",
    "tool",
    "open",
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
    "walls",
    "pressure",
    "temperature",
];

/// The materials a document finds: its project's library (materials/ beside it), the user's, then
/// ForgeCAD's; a session kept to its root reads only ForgeCAD's.
fn library(ev: &Eval) -> ft::FormShop {
    if ev.root.is_some() {
        return ft::placeholder();
    }
    crate::deps::materials(ev.dir.as_deref());
    let libs = forge_material::Libraries::cached(ev.dir.as_deref());
    ft::shop_of(libs.all().map(|x| x.1))
}

/// What a thermoform built: the mould's surface solid it read, and what it made of it.
#[derive(Clone)]
struct Made {
    formed: Rc<ft::Formed>,
    prep: Rc<ft::Prepared>,
}

/// The spec for a sheet `t` thick: a vent left to the material, its largest for that sheet.
fn sized(spec: &ft::Spec, m: &ft::Material, t: f64) -> ft::Spec {
    let mut s = spec.clone();
    s.t = t;
    if s.vent.is_some_and(f64::is_nan) {
        s.vent = Some(m.max_vent.at(t).min(1.0));
    }
    s
}

/// A thickness given for a wall must be the wall's.
fn check_t(given: Option<f64>, measured: f64, span: Span, id: &str) -> R<()> {
    match given {
        Some(g) if (g - measured).abs() > 0.05 * measured.max(0.4) => Err(thermo_err(ThermoErr::WallNotT {
            span,
            id: id.into(),
            given: g,
            measured,
        })),
        _ => Ok(()),
    }
}

/// The arguments' numbers as written, for fixes.
struct Nums {
    t: Option<Num>,
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

/// The shape's solid among `solids`: the one its feature made, else the only one.
fn shape_of(solids: &[Solid], idx: usize) -> Option<&Solid> {
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
    let shape = match a.req("shape")? {
        Value::Feature(f) => f,
        o => {
            return Err(wrong(
                a.span("shape"),
                "the shape (the feature that made its body)",
                Ty::Feature,
                &o,
            ));
        }
    };
    let from = match crate::wired::word(&a, "from", &["outer", "wall", "inner"])? {
        Some("wall") => ft::Given::Wall,
        Some("inner") => ft::Given::Inner,
        _ => ft::Given::Outer,
    };
    let tool = match crate::wired::word(&a, "tool", &["male", "female"])? {
        Some("female") => Tool::Female,
        _ => Tool::Male,
    };
    // a wall says its own thickness; the others are given it
    let t = match (a.num("t", LEN)?, from) {
        (Some(t), _) => {
            positive(&t, a.span("t"), "the sheet's thickness")?;
            Some(t)
        }
        (None, ft::Given::Wall) => None,
        (None, _) => {
            return Err(Error::MissingArg {
                span,
                func: "thermoform",
                arg: "t",
            });
        }
    };
    let shop = match a.opt("shop") {
        None => library(ev),
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
    // the open face: where the sheet meets the clamp; the part leaves the mould away from it
    // (male) or through it (female)
    let pull = match (a.opt("open"), a.opt("pull")) {
        (Some(v), _) => {
            let n = crate::builtins::plane_of(ev, v, a.span("open"), "the open face")?.z();
            if tool == Tool::Male { -n } else { n }
        }
        (None, None) => UnitVec3::Z,
        (None, Some(v)) => crate::eval::unit_or_z(dir_of(v, a.span("pull"))?),
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
        // the material's largest for the sheet, set once its thickness is known
        None => (Some(f64::NAN), None),
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
    let temperature = a
        .num("temperature", ANY)?
        .map_or(m.forming_temperature(), |n| n.v);
    let walls = match a.opt("walls") {
        Some(Value::Bool(false)) => None,
        None | Some(Value::Bool(true)) => Some(ft::Process {
            law: m.law_at(temperature).law,
            pressure: a.num("pressure", RATIO)?.map_or(0.09, |n| n.v),
            ..ft::Process::default()
        }),
        Some(o) => return Err(wrong(a.span("walls"), "walls: true or false", Ty::Bool, &o)),
    };
    let sheet = match a.opt("sheet") {
        None => None,
        Some(v) => Some(pair(v, a.span("sheet"), LEN)?),
    };
    let spec = ft::Spec {
        tool,
        pull,
        t: t.as_ref().map_or(0.0, |t| t.v),
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
    let t_given = t.as_ref().map(|t| t.v);
    let nums = Nums {
        t: t.clone(),
        vent: vent_num,
    };
    let hid = history_id(&id);
    let text = format!("{spec:?} {from:?} {:?} {}", m, shape.idx);
    let marker = Feature::Param(Box::new(forge_model::Param {
        name: format!("thermoform:{id}"),
        expr: text.clone(),
        unit: forge_model::Unit::None,
        value: 0.0,
    }));
    let solid = ev.b.features.len();
    let before_n = solid;
    let mut made: Option<(Made, Body)> = None;
    let mut failed: Option<(Body, ft::Prepared, ft::Spec)> = None;
    let (sp, idc) = (span, id.clone());
    let r = ev.push_made(
        &id,
        "thermoform",
        span,
        t.iter().map(|t| dim("thickness", t)).collect(),
        hid,
        |b: &mut Builder| {
            b.push_with(marker, hid, |before: &[Solid]| {
                let s = shape_of(before, shape.idx).ok_or_else(|| {
                    thermo_err(ThermoErr::NotABody {
                        span: sp,
                        id: shape.id.clone(),
                    })
                })?;
                let err = |error| {
                    thermo_err(ThermoErr::Kernel {
                        span: sp,
                        id: idc.clone(),
                        error,
                    })
                };
                let prep = ft::prepare(&s.body, from, &spec, hid).map_err(err)?;
                let spec = sized(&spec, &m, prep.t);
                check_t(t_given, prep.t, sp, &idc)?;
                let f = ft::form(&prep.shape, &spec, hid).map_err(|e| {
                    failed = Some((s.body.clone(), prep.clone(), spec.clone()));
                    err(e)
                })?;
                let mut out: Vec<Solid> = before.iter().filter(|x| x.id != s.id).cloned().collect();
                out.push(Solid {
                    id: s.id,
                    body: f.mould.clone(),
                });
                out.push(Solid {
                    id: solid,
                    body: f.part.clone(),
                });
                made = Some((
                    Made {
                        formed: Rc::new(f),
                        prep: Rc::new(prep),
                    },
                    s.body.clone(),
                ));
                Ok(out)
            })
        },
    );
    // the solids failed: what the checks say about the shape still stands
    if r.is_err()
        && let Some((_, prep, spec)) = failed
        && let Ok(f) = ft::analyse(&prep.shape, &spec)
    {
        notes(ev, &id, span, before_n, &prep, &f, None, &spec, &m, &shop, &nums);
    }
    let (idx, _) = r?;
    let ok = ev.b.building && ev.feats.last().is_some_and(|f| f.status == Status::Ok);
    // the shape as it stood before, for the checks and the memo's key
    let shape_body = match &made {
        Some((_, b)) => Some(b.clone()),
        None if ok => {
            ev.b.stages
                .get(before_n.wrapping_sub(1))
                .and_then(|st| shape_of(&st.solids, shape.idx).map(|s| s.body.clone()))
        }
        None => None,
    };
    if let Some(mb) = shape_body {
        let key = format!("thermoform {text} {}", body_key(&mb));
        let built = match made {
            Some((m, _)) => ev.b.memo(key.clone(), || Some(m)),
            None => ev.b.memo(key.clone(), || {
                let prep = ft::prepare(&mb, from, &spec, hid).ok()?;
                let spec = sized(&spec, &m, prep.t);
                let f = ft::form(&prep.shape, &spec, hid).ok()?;
                Some(Made {
                    formed: Rc::new(f),
                    prep: Rc::new(prep),
                })
            }),
        };
        if let Some(Made { formed: f, prep }) = built {
            let spec = sized(&spec, &m, prep.t);
            let st = walls.as_ref().filter(|_| ev.checks).map(|pr| {
                let key = format!("thermoform walls {key} {pr:?}");
                let t0 = std::time::Instant::now();
                let st = ev.b.memo(key, || Rc::new(ft::stretch(&f, &spec, pr)));
                ev.b.kernel_ms += t0.elapsed().as_secs_f64() * 1e3;
                st
            });
            notes(
                ev,
                &id,
                span,
                before_n,
                &prep,
                &f,
                st.as_deref(),
                &spec,
                &m,
                &shop,
                &nums,
            );
            // the part as the model leaves it, and the modelled one's mass, both kept with it
            let sim = st.as_ref().map(|w| {
                let key = format!("thermoform simulated {key} {walls:?}");
                ev.b.memo(key, || {
                    let trim = ft::build::polygon(&f.trim);
                    Rc::new(ft::simulated(&f.frame, &trim, w, prep.t))
                })
            });
            let modelled = ev.b.memo(format!("thermoform modelled {key}"), || {
                forge_topo::mass::mass(&f.part).ok()
            });
            let mould_solid =
                ev.b.stages
                    .get(before_n.wrapping_sub(1))
                    .and_then(|st| shape_of(&st.solids, shape.idx).map(|s| s.id))
                    .unwrap_or(shape.idx);
            let density = forge_material::Libraries::cached(ev.dir.as_deref())
                .find(&m.name)
                .map(|x| x.1.density)
                .filter(|d| *d > 0.0);
            ev.thermos.push(ThermoOut {
                id: id.clone(),
                material: m.name.clone(),
                tool,
                from,
                t: prep.t,
                formed: f,
                walls: st,
                temperature,
                law: m.law_at(temperature),
                min_wall: m.min_wall,
                density,
                sheet,
                sim,
                modelled,
                mould_solid,
                part_solid: solid,
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
    prep: &ft::Prepared,
    f: &ft::Formed,
    walls: Option<&ft::Stretch>,
    spec: &ft::Spec,
    m: &ft::Material,
    shop: &ft::FormShop,
    nums: &Nums,
) {
    let shape = &prep.shape;
    let found = ft::check(shape, f, spec, m, shop, walls);
    if found.is_empty() {
        return;
    }
    let origin = ev.b.origin(before);
    // the features that made a face, by its name (or the faces either side of an edge)
    let feats_of = |names: &[forge_topo::Name]| -> Vec<(String, &'static str, usize)> {
        let mut out: Vec<(String, &'static str, usize)> = vec![];
        for n in names {
            // a face of a generated surface goes by the user's face it was offset from
            let n = prep.origin.iter().find(|o| o.0 == *n).map_or(n, |o| &o.1);
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
        shape
            .edge_ids()
            .find(|&x| shape.edge(x).name == e)
            .map(|x| {
                shape
                    .edge(x)
                    .coedges
                    .iter()
                    .map(|&c| shape.face(shape.coedge_face(c)).name)
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
                let want = if spec.tool == Tool::Male {
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
            Violation::ThinWall { at, ratio, min, .. } => {
                // the sheet that comes to the least there, else the shape eased where it is thinnest
                let need = up(spec.t * min / ratio);
                let gauge = shop
                    .machine
                    .gauges
                    .iter()
                    .copied()
                    .filter(|g| *g >= need - 1e-9)
                    .fold(None, |b: Option<f64>, g| Some(b.map_or(g, |b| b.min(g))))
                    .unwrap_or(need);
                let mut fixes = vec![set(nums.t.as_ref(), id, "t", gauge, "")];
                let made = nearest_face(f, shape, *at)
                    .map(|n| feats_of(&[n]))
                    .unwrap_or_default();
                for x in &made {
                    match x.1 {
                        "extrude" => {
                            let now = num_of_feat(x.2, "taper").map_or(0.0, |n| n.v);
                            let more = (now.to_degrees() + 2.0).round().to_radians();
                            fixes.push(set(
                                num_of_feat(x.2, "taper").as_ref(),
                                &x.0,
                                "taper",
                                more,
                                "deg",
                            ));
                        }
                        "fillet" => {
                            if let Some(r) = num_of_feat(x.2, "radius") {
                                fixes.push(set(Some(&r), &x.0, "r", up(1.5 * r.v), ""));
                            }
                        }
                        _ => {}
                    }
                }
                (made, fixes)
            }
            Violation::GaugeNotStocked { nearest, .. } => {
                (vec![], vec![set(nums.t.as_ref(), id, "t", *nearest, "")])
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

/// The face of the shape nearest a point (the mesh's nearest vertex's face).
fn nearest_face(f: &ft::Formed, shape: &Body, at: Point3) -> Option<forge_topo::Name> {
    let mut best: Option<(f64, usize)> = None;
    for &t in &f.contact {
        for &v in &f.mesh.triangles[t] {
            let d = (f.mesh.positions[v as usize] - at).length();
            if best.is_none_or(|b| d < b.0) {
                best = Some((d, t));
            }
        }
    }
    let (_, t) = best?;
    Some(shape.face(forge_topo::FaceId(f.mesh.face_of[t])).name)
}
