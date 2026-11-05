//! A document's iProperties, as Inventor has them: `properties(..)`, one bare statement whose
//! keyword arguments are the Summary, Project and Status fields, custom ones, and a part's
//! material (by body too). Materials are forge-material's library, found by name or alias; a
//! name it lacks is `UnknownMaterial` with the nearest names as fixes. The part's physical
//! properties (mass, centre of gravity, inertia) come from each solid's exact mass properties
//! and its material's density.
//!
//! What other tools read (a drawing's title block, a BOM): `Run::props` (`Props::get` by key,
//! `Props::title_block`), `Run::physical`, and an assembly's `AsmOut::part_docs` and
//! `asm_physical`.

use crate::builtins::Args;
use crate::error::{Error, Op};
use crate::eval::{Eval, R, Value};
use crate::syntax::{self, Arg, Expr, ExprKind, Key, Span, Stmt, StmtKind};
use forge_agent::json::Json;
use forge_material::Material;
use forge_math::{Frame, Vec3};
use forge_topo::mass::MassProps;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Summary,
    Project,
    Status,
}

/// Each text field: its key in the source, Inventor's label, its tab.
pub const FIELDS: &[(&str, &str, Tab)] = &[
    ("title", "Title", Tab::Summary),
    ("subject", "Subject", Tab::Summary),
    ("author", "Author", Tab::Summary),
    ("manager", "Manager", Tab::Summary),
    ("company", "Company", Tab::Summary),
    ("category", "Category", Tab::Summary),
    ("keywords", "Keywords", Tab::Summary),
    ("comments", "Comments", Tab::Summary),
    ("part_number", "Part Number", Tab::Project),
    ("stock_number", "Stock Number", Tab::Project),
    ("description", "Description", Tab::Project),
    ("revision", "Revision Number", Tab::Project),
    ("project", "Project", Tab::Project),
    ("designer", "Designer", Tab::Project),
    ("engineer", "Engineer", Tab::Project),
    ("authority", "Authority", Tab::Project),
    ("cost_center", "Cost Center", Tab::Project),
    ("estimated_cost", "Estimated Cost", Tab::Project),
    ("creation_date", "Creation Date", Tab::Project),
    ("vendor", "Vendor", Tab::Project),
    ("web_link", "WEB Link", Tab::Project),
    ("design_state", "Design State", Tab::Status),
    ("checked_by", "Checked By", Tab::Status),
    ("checked_date", "Checked Date", Tab::Status),
    ("eng_approved_by", "Eng. Approved By", Tab::Status),
    ("eng_approved_date", "Eng. Approved Date", Tab::Status),
    ("mfg_approved_by", "Mfg. Approved By", Tab::Status),
    ("mfg_approved_date", "Mfg. Approved Date", Tab::Status),
];

/// The design states Inventor's Status tab offers.
pub const STATES: [&str; 3] = ["WorkInProgress", "Pending", "Released"];

/// Every argument `properties(..)` takes.
pub fn arg_names() -> Vec<&'static str> {
    FIELDS
        .iter()
        .map(|f| f.0)
        .chain(["material", "bodies", "custom"])
        .collect()
}

fn is_date(key: &str) -> bool {
    key.ends_with("_date")
}

/// A document's properties as written; empty where it has no `properties(..)`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Props {
    /// Text fields given, in `FIELDS` order.
    pub fields: Vec<(&'static str, String)>,
    /// Custom properties, name and value, in the order written.
    pub custom: Vec<(String, String)>,
    /// The part's material, the library's name for it.
    pub material: Option<String>,
    /// Bodies with a material of their own: the feature that made each, the library's name.
    pub bodies: Vec<(String, String)>,
    /// The statement, if there is one.
    pub span: Option<Span>,
}

impl Props {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.iter().find(|f| f.0 == key).map(|f| f.1.as_str())
    }

    /// What a drawing's title block and parts list read: Inventor's labels, with the material.
    pub fn title_block(&self) -> Vec<(&'static str, String)> {
        let mut out: Vec<(&'static str, String)> = FIELDS
            .iter()
            .filter_map(|f| self.get(f.0).map(|v| (f.1, v.to_string())))
            .collect();
        if let Some(m) = &self.material {
            out.push(("Material", m.clone()));
        }
        out
    }

    pub fn json(&self) -> Json {
        let s = |x: &str| Json::Str(x.into());
        Json::obj(vec![
            (
                "fields",
                Json::Obj(self.fields.iter().map(|f| (f.0.to_string(), s(&f.1))).collect()),
            ),
            (
                "custom",
                Json::Obj(self.custom.iter().map(|f| (f.0.clone(), s(&f.1))).collect()),
            ),
            ("material", self.material.as_deref().map_or(Json::Null, s)),
            (
                "bodies",
                Json::Obj(self.bodies.iter().map(|f| (f.0.clone(), s(&f.1))).collect()),
            ),
        ])
    }
}

/// Where a material name was written: what a fix edits.
#[derive(Clone, Debug, PartialEq)]
pub enum MaterialAt {
    /// `properties(material: ..)`.
    Part,
    /// `properties(bodies: {ID: ..})`: the whole map as written, so the fix can rewrite it.
    Body { id: String, map: Vec<(String, String)> },
    /// `sheet(.., material: ..)` of sheet `id`.
    Sheet { id: String },
}

#[derive(Clone, Debug, PartialEq)]
pub enum PropErr {
    UnknownMaterial {
        span: Span,
        name: String,
        near: Vec<String>,
        at: MaterialAt,
    },
    /// A second `properties(..)`: a document has one.
    Twice { span: Span, first: Span },
    /// A date not written YYYY-MM-DD.
    BadDate { span: Span, key: String, text: String },
    /// A design state not one of `STATES`.
    BadState { span: Span, text: String },
    /// A material in an assembly: its parts carry theirs.
    MaterialInAssembly { span: Span },
    /// `bodies:` names something that made no body of the part.
    NoSuchBody { span: Span, id: String },
    /// `properties(..)` bound to a name or inside a block: it is a top-level statement of its own.
    NotTopLevel { span: Span },
}

impl PropErr {
    pub fn span(&self) -> Option<Span> {
        use PropErr::*;
        match self {
            UnknownMaterial { span, .. }
            | Twice { span, .. }
            | BadDate { span, .. }
            | BadState { span, .. }
            | MaterialInAssembly { span }
            | NoSuchBody { span, .. }
            | NotTopLevel { span } => Some(*span),
        }
    }

    pub fn kind(&self) -> &'static str {
        use PropErr::*;
        match self {
            UnknownMaterial { .. } => "UnknownMaterial",
            Twice { .. } => "PropertiesTwice",
            BadDate { .. } => "BadDate",
            BadState { .. } => "BadDesignState",
            MaterialInAssembly { .. } => "MaterialInAssembly",
            NoSuchBody { .. } => "NoSuchBody",
            NotTopLevel { .. } => "PropertiesNotTopLevel",
        }
    }

    pub fn text(&self) -> String {
        use PropErr::*;
        match self {
            UnknownMaterial { name, near, .. } if near.is_empty() => {
                format!("no material {name:?} in the library")
            }
            UnknownMaterial { name, near, .. } => {
                format!(
                    "no material {name:?} in the library; nearest: {}",
                    near.join(", ")
                )
            }
            Twice { .. } => "a document has one properties(..): put these in the first".into(),
            BadDate { key, text, .. } => format!("{key}: {text:?} is not a date written YYYY-MM-DD"),
            BadState { text, .. } => format!("design_state: {text:?} is not one of {}", STATES.join(", ")),
            MaterialInAssembly { .. } => {
                "an assembly has no material of its own: its parts' materials give its mass".into()
            }
            NoSuchBody { id, .. } => format!("bodies: {id} made no body of this part"),
            NotTopLevel { .. } => "properties(..) is a statement of its own, at the top level".into(),
        }
    }

    pub fn data(&self) -> Vec<(&'static str, Json)> {
        use PropErr::*;
        let s = |x: &str| Json::Str(x.into());
        match self {
            UnknownMaterial { name, near, .. } => vec![
                ("name", s(name)),
                ("near", Json::Arr(near.iter().map(|n| s(n)).collect())),
            ],
            BadDate { key, text, .. } => vec![("key", s(key)), ("text", s(text))],
            BadState { text, .. } => vec![("text", s(text))],
            NoSuchBody { id, .. } => vec![("id", s(id))],
            Twice { .. } | MaterialInAssembly { .. } | NotTopLevel { .. } => vec![],
        }
    }

    pub fn fixes(&self) -> Vec<Op> {
        let PropErr::UnknownMaterial { near, at, .. } = self else {
            return vec![];
        };
        near.iter()
            .map(|n| match at {
                MaterialAt::Part => Op::SetProperty {
                    key: "material".into(),
                    text: Some(quote(n)),
                },
                MaterialAt::Body { id, map } => Op::SetProperty {
                    key: "bodies".into(),
                    text: Some(map_text(
                        &map.iter()
                            .map(|(k, v)| (k.clone(), if k == id { n.clone() } else { v.clone() }))
                            .collect::<Vec<_>>(),
                    )),
                },
                MaterialAt::Sheet { id } => Op::SetArg {
                    id: id.clone(),
                    arg: "material".into(),
                    text: quote(n),
                },
            })
            .collect()
    }
}

/// A string literal's source text.
pub fn quote(s: &str) -> String {
    format!("{s:?}")
}

/// A `bodies:` map's source text.
pub fn map_text(m: &[(String, String)]) -> String {
    let rows: Vec<String> = m.iter().map(|(k, v)| format!("{k}: {}", quote(v))).collect();
    format!("{{{}}}", rows.join(", "))
}

fn err(e: PropErr) -> Error {
    Error::Props(Box::new(e))
}

/// A material by name, or the typed error with the nearest names.
pub fn material(ev: &Eval, name: &str, span: Span, at: MaterialAt) -> R<Material> {
    let libs = libraries(ev);
    libs.find(name).map(|m| m.1.clone()).ok_or_else(|| {
        err(PropErr::UnknownMaterial {
            span,
            name: name.into(),
            near: libs.near(name),
            at,
        })
    })
}

/// The material libraries a document sees: its project's, the user's, ForgeCAD's.
pub fn libraries(ev: &Eval) -> std::sync::Arc<forge_material::Libraries> {
    crate::deps::materials(ev.dir.as_deref());
    forge_material::Libraries::cached(ev.dir.as_deref())
}

fn text_of(v: &Value) -> Option<String> {
    match v {
        Value::Str(s) => Some(s.to_string()),
        Value::Num(n) => Some(crate::values::fmt(n.v)),
        _ => None,
    }
}

fn date_ok(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        && (1..=12).contains(&s[5..7].parse::<u32>().unwrap_or(0))
        && (1..=31).contains(&s[8..10].parse::<u32>().unwrap_or(0))
}

/// `properties(..)`: checked and kept on the evaluator; the statement's value is nothing.
pub fn call(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    if let Some(first) = ev.props.span {
        return Err(err(PropErr::Twice { span, first }));
    }
    let names = arg_names();
    let names: Vec<&'static str> = names.to_vec();
    let a = Args::bind(ev, "properties", &names, args, span)?;
    let mut p = Props {
        span: Some(span),
        ..Props::default()
    };
    let wrong = |k: &'static str, want: crate::values::Ty, v: &Value| Error::WrongType {
        span: a.span(k),
        what: k,
        want,
        found: v.ty(),
    };
    for (k, _, _) in FIELDS {
        let Some(v) = a.opt(k) else { continue };
        let text = match (&v, *k) {
            (Value::Num(_), "estimated_cost") => text_of(&v),
            (Value::Str(s), _) => Some(s.to_string()),
            _ => None,
        }
        .ok_or_else(|| wrong(k, crate::values::Ty::Str, &v))?;
        if is_date(k) && !text.is_empty() && !date_ok(&text) {
            return Err(err(PropErr::BadDate {
                span: a.span(k),
                key: k.to_string(),
                text,
            }));
        }
        if *k == "design_state" && !STATES.contains(&text.as_str()) {
            return Err(err(PropErr::BadState {
                span: a.span(k),
                text,
            }));
        }
        p.fields.push((k, text));
    }
    if let Some(v) = a.opt("custom") {
        let Value::List(rows) = &v else {
            return Err(wrong("custom", crate::values::Ty::List, &v));
        };
        for r in rows.iter() {
            let pair = match r {
                Value::Tuple(t) if t.len() == 2 => match (&t[0], text_of(&t[1])) {
                    (Value::Str(n), Some(x)) => Some((n.to_string(), x)),
                    _ => None,
                },
                _ => None,
            };
            p.custom
                .push(pair.ok_or_else(|| wrong("custom", crate::values::Ty::Tuple, r))?);
        }
    }
    let in_asm = ev.asm.is_some();
    if let Some(v) = a.opt("material") {
        if in_asm {
            return Err(err(PropErr::MaterialInAssembly {
                span: a.span("material"),
            }));
        }
        let Value::Str(s) = &v else {
            return Err(wrong("material", crate::values::Ty::Str, &v));
        };
        p.material = Some(material(ev, s, a.span("material"), MaterialAt::Part)?.name);
    }
    if let Some(v) = a.opt("bodies") {
        if in_asm {
            return Err(err(PropErr::MaterialInAssembly {
                span: a.span("bodies"),
            }));
        }
        let Value::Map(m) = &v else {
            return Err(wrong("bodies", crate::values::Ty::Map, &v));
        };
        let mut written = vec![];
        for (k, x) in m.iter() {
            let (Key::Label(id), Value::Str(s)) = (k, x) else {
                return Err(wrong("bodies", crate::values::Ty::Map, &v));
            };
            written.push((id.clone(), s.to_string()));
        }
        for (id, s) in &written {
            let at = MaterialAt::Body {
                id: id.clone(),
                map: written.clone(),
            };
            let m = material(ev, s, a.span("bodies"), at)?;
            p.bodies.push((id.clone(), m.name));
        }
    }
    ev.props = p;
    Ok(Value::Unresolved)
}

/// `properties(..)` anywhere but as a top-level statement: refused (its edit op finds it there).
pub fn check_placement(prog: &[Stmt], errors: &mut Vec<Error>) {
    let is_props = |e: &Expr| matches!(&e.kind, ExprKind::Call(f, _) if matches!(&f.kind, ExprKind::Name(n) if n == "properties"));
    for (k, s) in prog.iter().enumerate() {
        let top = matches!(&s.kind, StmtKind::Expr(e) if is_props(e));
        let mut found = vec![];
        syntax::walk(std::slice::from_ref(s), &mut |st| match &st.kind {
            StmtKind::Assign { value, .. } if is_props(value) => found.push(st.span),
            StmtKind::Expr(e) if is_props(e) && !(top && st.span == prog[k].span) => found.push(st.span),
            _ => {}
        });
        errors.extend(found.into_iter().map(|span| err(PropErr::NotTopLevel { span })));
    }
}

// ---- physical properties -------------------------------------------------------------------

/// One solid's share of a part's physical properties.
#[derive(Clone, Debug, PartialEq)]
pub struct SolidPhys {
    /// The feature that made it.
    pub id: String,
    pub material: Option<String>,
    /// kg/m^3.
    pub density: Option<f64>,
    /// mm^3, mm^2, kg, mm.
    pub volume: f64,
    pub area: f64,
    pub mass: Option<f64>,
    pub centroid: [f64; 3],
}

/// Mass, centre of gravity and inertia of a part or an assembly. Lengths mm, mass kg, inertia
/// kg mm^2. Solids without a material count in volume and area only (`missing`).
#[derive(Clone, Debug, PartialEq)]
pub struct Physical {
    pub volume: f64,
    pub area: f64,
    /// Of the solids with a material; None when none has one.
    pub mass: Option<f64>,
    /// The centre of gravity (of the volume, when nothing has a material).
    pub centroid: [f64; 3],
    /// About the centre of gravity, and about the origin.
    pub inertia: Option<[[f64; 3]; 3]>,
    pub origin_inertia: Option<[[f64; 3]; 3]>,
    /// Principal moments about the centre of gravity, smallest first, and their axes.
    pub principal: Option<([f64; 3], [[f64; 3]; 3])>,
    pub solids: Vec<SolidPhys>,
    /// Solids (or, in an assembly, occurrences) with no material.
    pub missing: Vec<String>,
    /// The largest relative error bound of the volumes integrated.
    pub rel_err: f64,
}

/// A solid's exact mass properties (unit density), where it is, and its material.
pub struct Piece<'a> {
    pub id: String,
    pub material: Option<&'a Material>,
    pub mass: &'a MassProps,
    /// Where the solid's space lies in the result's (an occurrence's pose).
    pub at: Frame,
}

fn rot(f: &Frame) -> [[f64; 3]; 3] {
    let (x, y, z) = (f.x().get(), f.y().get(), f.z().get());
    [[x.x, y.x, z.x], [x.y, y.y, z.y], [x.z, y.z, z.z]]
}

fn turn(r: &[[f64; 3]; 3], i: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    // R I R^T
    std::array::from_fn(|a| {
        std::array::from_fn(|b| {
            let mut s = 0.0;
            for p in 0..3 {
                for q in 0..3 {
                    s += r[a][p] * i[p][q] * r[b][q];
                }
            }
            s
        })
    })
}

/// The parallel axis term: a point mass m at d from the axes' origin.
fn shift(m: f64, d: [f64; 3]) -> [[f64; 3]; 3] {
    let dd = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
    std::array::from_fn(|a| std::array::from_fn(|b| m * (if a == b { dd } else { 0.0 } - d[a] * d[b])))
}

fn add(a: &mut [[f64; 3]; 3], b: &[[f64; 3]; 3]) {
    for i in 0..3 {
        for j in 0..3 {
            a[i][j] += b[i][j];
        }
    }
}

/// The physical properties of solids together.
pub fn combine(pieces: &[Piece]) -> Physical {
    let mut solids = vec![];
    let (mut vol, mut area, mut mass, mut rel) = (0.0, 0.0, 0.0, 0.0f64);
    let (mut vc, mut mc) = ([0.0; 3], [0.0; 3]);
    let mut missing = vec![];
    let world: Vec<[f64; 3]> = pieces
        .iter()
        .map(|p| {
            let c = p.at.point_to_world(p.mass.centroid);
            [c.x, c.y, c.z]
        })
        .collect();
    for (p, c) in pieces.iter().zip(&world) {
        let m = p.material.map(|x| x.kg_per_mm3() * p.mass.volume);
        vol += p.mass.volume;
        area += p.mass.area;
        rel = rel.max(p.mass.err[0] / p.mass.volume.abs().max(1e-300));
        for k in 0..3 {
            vc[k] += c[k] * p.mass.volume;
        }
        match m {
            Some(m) => {
                mass += m;
                for k in 0..3 {
                    mc[k] += c[k] * m;
                }
            }
            None => missing.push(p.id.clone()),
        }
        solids.push(SolidPhys {
            id: p.id.clone(),
            material: p.material.map(|x| x.name.clone()),
            density: p.material.map(|x| x.density),
            volume: p.mass.volume,
            area: p.mass.area,
            mass: m,
            centroid: *c,
        });
    }
    let any = pieces.iter().any(|p| p.material.is_some());
    let centroid = if any {
        mc.map(|x| x / mass)
    } else {
        vc.map(|x| x / vol)
    };
    let (inertia, origin_inertia, principal) = if any {
        let mut i = [[0.0; 3]; 3];
        for (p, c) in pieces.iter().zip(&world) {
            let Some(mat) = p.material else { continue };
            let k = mat.kg_per_mm3();
            let own = p.mass.inertia.map(|r| r.map(|x| x * k));
            add(&mut i, &turn(&rot(&p.at), &own));
            let d = [c[0] - centroid[0], c[1] - centroid[1], c[2] - centroid[2]];
            add(&mut i, &shift(k * p.mass.volume, d));
        }
        let mut o = i;
        add(&mut o, &shift(mass, centroid));
        let (m, axes) = forge_topo::mass::eigen(i);
        let axes: [[f64; 3]; 3] = axes.map(|a: Vec3| [a.x, a.y, a.z]);
        (Some(i), Some(o), Some((m, axes)))
    } else {
        (None, None, None)
    };
    Physical {
        volume: vol,
        area,
        mass: any.then_some(mass),
        centroid,
        inertia,
        origin_inertia,
        principal,
        solids,
        missing,
        rel_err: rel,
    }
}

impl Physical {
    pub fn json(&self) -> Json {
        let v3 = |v: [f64; 3]| Json::Arr(v.iter().map(|x| (*x).into()).collect());
        let m3 = |m: Option<[[f64; 3]; 3]>| {
            m.map_or(Json::Null, |m| Json::Arr(m.iter().map(|r| v3(*r)).collect()))
        };
        let opt = |x: Option<f64>| x.map_or(Json::Null, Json::from);
        Json::obj(vec![
            ("volume", self.volume.into()),
            ("area", self.area.into()),
            ("mass", opt(self.mass)),
            ("centroid", v3(self.centroid)),
            ("inertia", m3(self.inertia)),
            ("origin_inertia", m3(self.origin_inertia)),
            ("principal", self.principal.map_or(Json::Null, |p| v3(p.0))),
            (
                "solids",
                Json::Arr(
                    self.solids
                        .iter()
                        .map(|s| {
                            Json::obj(vec![
                                ("id", s.id.as_str().into()),
                                ("material", s.material.as_deref().map_or(Json::Null, Json::from)),
                                ("volume", s.volume.into()),
                                ("mass", opt(s.mass)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "missing",
                Json::Arr(self.missing.iter().map(|m| m.as_str().into()).collect()),
            ),
            ("rel_err", self.rel_err.into()),
        ])
    }
}

// ---- the edit op ---------------------------------------------------------------------------

/// The top-level `properties(..)` statement: its index and arguments.
fn statement(prog: &[Stmt]) -> Option<(usize, &Expr, &[Arg])> {
    prog.iter().enumerate().find_map(|(k, s)| match &s.kind {
        StmtKind::Expr(e) => match &e.kind {
            ExprKind::Call(f, args) if matches!(&f.kind, ExprKind::Name(n) if n == "properties") => {
                Some((k, e, args.as_slice()))
            }
            _ => None,
        },
        _ => None,
    })
}

/// `SetProperty`: the argument `key` of the document's `properties(..)` becomes `text` (put in
/// if missing; the statement made at the top if the document has none), or is taken out.
pub fn apply_set(src: &str, prog: &[Stmt], key: &str, text: Option<&str>) -> R<String> {
    if !arg_names().contains(&key) {
        return Err(Error::UnknownArg {
            span: Span::default(),
            func: "properties",
            arg: key.into(),
        });
    }
    if let Some(t) = text {
        // a value alone, reading no name
        let e = syntax::parse(&format!("x_ = {t}"))?;
        let probe = &e[0];
        if let Some(n) = crate::edit::mentions(probe).into_iter().find(|n| n != "x_") {
            return Err(Error::UnknownName {
                span: Span::default(),
                name: n,
            });
        }
    }
    let Some((_, call, args)) = statement(prog) else {
        let Some(t) = text else {
            return Ok(src.to_string());
        };
        // after the `assembly` line, or above the first statement (below the comments heading it)
        let at = match prog.first() {
            Some(s) if crate::asm::declared(prog) => {
                let end = s.span.hi as usize;
                src[end..].find('\n').map_or(src.len(), |i| end + i + 1)
            }
            Some(s) => src[..s.span.lo as usize].rfind('\n').map_or(0, |i| i + 1),
            None => src.len(),
        };
        let sep = if at > 0 && !src[..at].ends_with('\n') {
            "\n"
        } else {
            ""
        };
        return Ok(format!(
            "{}{sep}properties({key}: {t})\n{}",
            &src[..at],
            &src[at..]
        ));
    };
    let k = args
        .iter()
        .position(|a| a.name.as_ref().is_some_and(|n| n.0 == key));
    let (lo, hi, new) = match (k, text) {
        (Some(k), Some(t)) => (args[k].value.span.lo, args[k].value.span.hi, t.to_string()),
        (Some(k), None) => {
            let start = |a: &Arg| a.name.as_ref().map_or(a.value.span.lo, |n| n.1.lo);
            if k > 0 {
                (args[k - 1].value.span.hi, args[k].value.span.hi, String::new())
            } else if let Some(next) = args.get(1) {
                (start(&args[0]), start(next), String::new())
            } else {
                (start(&args[0]), args[0].value.span.hi, String::new())
            }
        }
        (None, Some(t)) => {
            let close = call.span.hi - 1;
            let sep = if args.is_empty() { "" } else { ", " };
            (close, close, format!("{sep}{key}: {t}"))
        }
        (None, None) => return Ok(src.to_string()),
    };
    let (lo, hi) = (lo as usize, hi as usize);
    Ok(format!("{}{}{}", &src[..lo], new, &src[hi..]))
}

/// Each solid of a finished part, as the assembly places it (an unfold's flat left out): the
/// feature that made it, its material (its own in `bodies:`, its sheet's, the part's), its body.
pub fn solids(ev: &Eval) -> Vec<(String, Option<Material>, forge_topo::Body)> {
    let Some(st) = ev.b.stages.last() else {
        return vec![];
    };
    let flats: Vec<usize> = ev
        .sheets
        .list
        .iter()
        .flat_map(|s| &s.unfolds)
        .filter_map(|u| ev.feats.iter().find(|f| f.id == u.id).map(|f| f.idx))
        .collect();
    let libs = libraries(ev);
    let find = |n: &str| libs.find(n).map(|m| m.1.clone());
    let part = ev.props.material.as_deref().and_then(find);
    st.solids
        .iter()
        .filter(|s| !flats.contains(&s.id))
        .map(|s| {
            let id = ev
                .feats
                .iter()
                .find(|f| f.idx == s.id)
                .map_or_else(String::new, |f| f.id.clone());
            let own = ev.props.bodies.iter().find(|b| b.0 == id).map(|b| b.1.as_str());
            let sheet = ev
                .sheets
                .list
                .iter()
                .find(|x| x.id == id)
                .and_then(|x| x.material.as_deref());
            // a formed part is of its sheet's material; its mould is tooling, of none
            let formed = ev
                .thermos
                .iter()
                .find(|x| x.part_solid == s.id)
                .map(|x| x.material.as_str());
            let m = own.or(sheet).or(formed).and_then(find).or(part.clone());
            (id, m, forge_model::finish(s.body.clone()))
        })
        .collect()
}

/// A built part's physical properties; `whole` is the finished body's own mass properties, used
/// when the part is one solid. `bodies:` naming no solid is an error here.
pub fn part_physical(ev: &mut Eval, whole: Option<&MassProps>) -> Option<Physical> {
    let solids = solids(ev);
    for (id, _) in ev.props.bodies.clone() {
        if !solids.iter().any(|s| s.0 == id) {
            ev.errors.push(err(PropErr::NoSuchBody {
                span: ev.props.span.unwrap_or_default(),
                id,
            }));
        }
    }
    if solids.is_empty() {
        return None;
    }
    let one = ev.b.stages.last().is_some_and(|st| st.solids.len() == 1);
    let masses: Vec<MassProps> = match whole {
        Some(w) if one => vec![w.clone()],
        _ => solids
            .iter()
            .map(|s| forge_topo::mass::mass(&s.2).ok())
            .collect::<Option<Vec<_>>>()?,
    };
    let pieces: Vec<Piece> = solids
        .iter()
        .zip(&masses)
        .map(|(s, m)| Piece {
            id: s.0.clone(),
            material: s.1.as_ref(),
            mass: m,
            at: Frame::WORLD,
        })
        .collect();
    Some(combine(&pieces))
}

/// A part file as an assembly holds it: its iProperties and solids, their masses worked out on
/// first asking (kept with the part's load, so across runs while its text is unchanged).
#[derive(Debug, Default)]
pub struct PartSummary {
    pub props: Props,
    /// Each solid: the feature that made it, its material, its body.
    pub solids: Vec<(String, Option<Material>, std::sync::Arc<forge_topo::Body>)>,
    masses: std::sync::OnceLock<Result<Vec<MassProps>, String>>,
}

impl PartSummary {
    pub fn of(ev: &Eval) -> PartSummary {
        PartSummary {
            props: ev.props.clone(),
            solids: solids(ev)
                .into_iter()
                .map(|(id, m, b)| (id, m, std::sync::Arc::new(b)))
                .collect(),
            masses: Default::default(),
        }
    }

    /// Each solid's exact mass properties, unit density.
    pub fn masses(&self) -> Result<&[MassProps], &str> {
        self.masses
            .get_or_init(|| {
                self.solids
                    .iter()
                    .map(|s| forge_topo::mass::mass(&s.2).map_err(|e| format!("{}: {e}", s.0)))
                    .collect()
            })
            .as_deref()
            .map_err(String::as_str)
    }

    /// The part's own physical properties, in its space.
    pub fn physical(&self) -> Result<Physical, String> {
        let masses = self.masses()?;
        let pieces: Vec<Piece> = self
            .solids
            .iter()
            .zip(masses)
            .map(|(s, m)| Piece {
                id: s.0.clone(),
                material: s.1.as_ref(),
                mass: m,
                at: Frame::WORLD,
            })
            .collect();
        Ok(combine(&pieces))
    }
}

/// An assembly's physical properties in its own frame: every occurrence that is not a
/// reference, under `under` (an occurrence path, or all), each part's solids at its pose.
/// Occurrences with no material are listed in `missing` by path.
pub fn asm_physical(out: &crate::asm::AsmOut, under: Option<&str>) -> Result<Physical, String> {
    let mut items = vec![];
    for (k, (path, doc, pose)) in out.occurrences.iter().enumerate() {
        let (body, reference) = out.occ_body.get(k).copied().unwrap_or((0, false));
        if reference
            || under.is_some_and(|u| {
                !(path == u || path.starts_with(&format!("{u}.")) || path.starts_with(&format!("{u}[")))
            })
        {
            continue;
        }
        let Some(sum) = out.part_docs.get(doc) else {
            continue;
        };
        let masses = sum.masses()?;
        for (s, m) in sum.solids.iter().zip(masses) {
            if body != 0 && crate::asm::hid(&s.0) != body {
                continue;
            }
            items.push((path.clone(), s.1.as_ref(), m, *pose));
        }
    }
    let pieces: Vec<Piece> = items
        .iter()
        .map(|(path, mat, m, pose)| Piece {
            id: path.clone(),
            material: *mat,
            mass: m,
            at: *pose,
        })
        .collect();
    let mut p = combine(&pieces);
    p.missing.dedup();
    Ok(p)
}

/// A BOM row as a parts list reads it: the part file, its count, its iProperties, its material
/// and one part's mass (kg, when it has a material).
pub type BomRow = (String, usize, Props, Option<String>, Option<f64>);

pub fn bom_rows(out: &crate::asm::AsmOut) -> Vec<BomRow> {
    out.bom
        .iter()
        .map(|(file, n)| {
            let sum = out.part_docs.get(file);
            let props = sum.map(|s| s.props.clone()).unwrap_or_default();
            let material =
                sum.and_then(|s| s.solids.iter().find_map(|x| x.1.as_ref().map(|m| m.name.clone())));
            let mass = sum.and_then(|s| s.physical().ok()).and_then(|p| p.mass);
            (file.clone(), *n, props, material, mass)
        })
        .collect()
}

pub fn bom_json(out: &crate::asm::AsmOut) -> Json {
    Json::Arr(
        bom_rows(out)
            .into_iter()
            .map(|(file, n, p, m, mass)| {
                let s = |x: Option<&str>| x.map_or(Json::Null, Json::from);
                Json::obj(vec![
                    ("file", file.as_str().into()),
                    ("count", n.into()),
                    ("part_number", s(p.get("part_number"))),
                    ("description", s(p.get("description"))),
                    ("material", s(m.as_deref())),
                    ("mass", mass.map_or(Json::Null, Json::from)),
                ])
            })
            .collect(),
    )
}
