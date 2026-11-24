//! The sketcher's working form: a typed sketch, row after row (a line between two points, a
//! rectangle, a horizontal constraint, a distance), built straight into a `forge_sketch::Sketch`
//! and solved as the drawing changes, with the points, lines and regions the tools read back.
//! Entities are named by their kind and count, as Forge names them (`l3` is the third line, `l3.a`
//! its start, `c1` a circle, as a point its centre). The rows live only in the app while a sketch
//! is open: `fgsketch` writes them as a Forge block and reads a block back into them. A value is a
//! number, or Forge text (a param's name, an expression) read through the sketcher's `Num`.

use forge_math::{Frame, Point2, dmath, tol};
use forge_ops::Profile;
use forge_sketch::project::flat;
pub use forge_sketch::project::{Flat, Source};
use forge_sketch::{Constraint as C, ConstraintId, Entity, EntityId, ExtractError, SketchError, Status};
use forge_topo::Name;
use std::fmt;

type R<T> = Result<T, String>;

/// What a number slot of a row wants: a length (mm), an angle (degrees in the row, radians out)
/// or a count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Want {
    Len,
    Angle,
    Count,
}

/// Reads a value's Forge text (a param's name, an expression) as the slot wants it.
pub type Num<'a> = dyn Fn(&str, Want) -> R<f64> + 'a;

/// A value in a row: a number, or Forge text read when the sketch is solved.
#[derive(Clone, Debug, PartialEq)]
pub enum V {
    N(f64),
    X(String),
}

impl V {
    /// The value `text` writes: a number if it is one.
    pub fn of(text: &str) -> V {
        match text.trim().parse::<f64>() {
            Ok(v) => V::N(v),
            Err(_) => V::X(text.trim().to_string()),
        }
    }

    pub fn num(&self) -> Option<f64> {
        match self {
            V::N(v) => Some(*v),
            V::X(_) => None,
        }
    }

    /// Its number as the slot wants it (degrees in the row, radians out).
    fn read(&self, num: &Num, want: Want) -> R<f64> {
        match self {
            V::N(v) => Ok(match want {
                Want::Angle => v * core::f64::consts::PI / 180.0,
                _ => *v,
            }),
            V::X(t) => num(t, want),
        }
    }
}

impl From<f64> for V {
    fn from(v: f64) -> V {
        V::N(v)
    }
}

impl fmt::Display for V {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            V::N(v) => write!(f, "{v}"),
            V::X(t) => write!(f, "{t}"),
        }
    }
}

/// Which point of an entity a reference means, or the entity itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Part {
    Whole,
    /// A line's or arc's start (a spline's first point).
    A,
    /// Its end.
    B,
    /// A circle's, arc's or ellipse's centre.
    C,
    /// An ellipse's major axis end.
    M,
    /// A spline's k-th point.
    K(usize),
}

/// An entity of the sketch by its name (`l3`, `c1`, `j2`, `o` the origin), or one of its points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ref {
    pub base: String,
    pub part: Part,
}

impl Ref {
    pub fn to(base: &str) -> Ref {
        Ref {
            base: base.to_string(),
            part: Part::Whole,
        }
    }
    pub fn a(base: &str) -> Ref {
        Ref {
            base: base.to_string(),
            part: Part::A,
        }
    }
    pub fn b(base: &str) -> Ref {
        Ref {
            base: base.to_string(),
            part: Part::B,
        }
    }
    pub fn center(base: &str) -> Ref {
        Ref {
            base: base.to_string(),
            part: Part::C,
        }
    }

    /// The reference a name of the sketch's points or entities is (`l3`, `l3.a`, `s1.2`, `e1.m`).
    pub fn name(t: &str) -> Ref {
        let (base, part) = t.split_once('.').unwrap_or((t, ""));
        let part = match part {
            "" => Part::Whole,
            "a" => Part::A,
            "b" => Part::B,
            "c" => Part::C,
            "m" => Part::M,
            k => k.parse().map_or(Part::Whole, Part::K),
        };
        Ref {
            base: base.to_string(),
            part,
        }
    }

    /// A point of an entity rather than the entity.
    pub fn is_point(&self) -> bool {
        self.part != Part::Whole
    }
}

impl fmt::Display for Ref {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.part {
            Part::Whole => write!(f, "{}", self.base),
            Part::A => write!(f, "{}.a", self.base),
            Part::B => write!(f, "{}.b", self.base),
            Part::C => write!(f, "{}.c", self.base),
            Part::M => write!(f, "{}.m", self.base),
            Part::K(k) => write!(f, "{}.{k}", self.base),
        }
    }
}

/// A point a geometry row runs through: a free one there (where the solver starts), or one the
/// sketch has.
#[derive(Clone, Debug, PartialEq)]
pub enum Pt {
    At(V, V),
    R(Ref),
}

impl Pt {
    pub fn at(x: f64, y: f64) -> Pt {
        Pt::At(V::N(x), V::N(y))
    }
    pub fn of(name: &str) -> Pt {
        Pt::R(Ref::name(name))
    }
    /// The sketch's point it is, if it is one.
    pub fn named(&self) -> Option<&Ref> {
        match self {
            Pt::R(r) => Some(r),
            Pt::At(..) => None,
        }
    }
}

impl fmt::Display for Pt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Pt::At(x, y) => write!(f, "near({x}, {y})"),
            Pt::R(r) => write!(f, "{r}"),
        }
    }
}

/// What a dimension sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum D {
    /// A line's length, or between two points, or a point and a line.
    Dist,
    /// The second point's x less the first's.
    HDist,
    VDist,
    Rad,
    Dia,
    /// From the first line to the second, counter-clockwise.
    Ang,
}

impl D {
    fn word(self) -> &'static str {
        match self {
            D::Dist => "distance",
            D::HDist => "hdistance",
            D::VDist => "vdistance",
            D::Rad => "radius",
            D::Dia => "diameter",
            D::Ang => "angle",
        }
    }
}

/// One row of the sketch: geometry (named by its kind and count), a constraint or a dimension.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    Pt(Pt),
    Line(Pt, Pt),
    /// Lines through the points in turn, back to the first unless open.
    Poly(Vec<Pt>, bool),
    /// Corner, width and height: four lines, level and plumb.
    Rect(V, V, V, V),
    Circle(Pt, V),
    /// Centre, start, end; clockwise if so.
    Arc(Pt, Pt, Pt, bool),
    /// The cubic through the points; open if so.
    Spline(Vec<Pt>, bool),
    /// On control vertices of this degree; open if so.
    BSpline(V, Vec<Pt>, bool),
    /// Centre, an end of the major axis, the semi-minor axis.
    Ellipse(Pt, Pt, V),
    /// The part's geometry `of` names (Forge text) laid on the plane and held there: one curve
    /// rows may name (`j1`), or `many` a face's boundary no row names; fitted within `approx`.
    Project {
        of: String,
        many: bool,
        approx: Option<V>,
    },
    /// The geometry of a row as construction: in no region.
    Cons(Box<Row>),
    Fix(Vec<Ref>),
    Co(Ref, Ref),
    Hor(Vec<Ref>),
    Ver(Vec<Ref>),
    Par(Ref, Ref),
    Perp(Ref, Ref),
    Tan(Ref, Ref),
    Conc(Ref, Ref),
    Colin(Ref, Ref),
    Eq(Vec<Ref>),
    Mid(Ref, Ref),
    Sym(Ref, Ref, Ref),
    /// A dimension on these, to this value; with none, driven: it shows what it measures.
    Dim(D, Vec<Ref>, Option<V>),
}

impl Row {
    /// The row without `Cons`.
    pub fn plain(&self) -> &Row {
        match self {
            Row::Cons(r) => r.plain(),
            r => r,
        }
    }

    pub fn is_cons(&self) -> bool {
        matches!(self, Row::Cons(_))
    }

    /// The row as construction geometry.
    pub fn cons(self) -> Row {
        match self {
            r @ Row::Cons(_) => r,
            r => Row::Cons(Box::new(r)),
        }
    }

    /// The kinds of the names it makes, in order (a rect: four lines).
    pub fn made(&self) -> Vec<char> {
        match self {
            Row::Pt(_) => vec!['p'],
            Row::Line(..) => vec!['l'],
            Row::Circle(..) => vec!['c'],
            Row::Arc(..) => vec!['a'],
            Row::Spline(..) | Row::BSpline(..) => vec!['s'],
            Row::Ellipse(..) => vec!['e'],
            Row::Rect(..) => vec!['l'; 4],
            Row::Poly(pts, open) => vec!['l'; pts.len() - usize::from(*open)],
            Row::Project { many: false, .. } => vec!['j'],
            Row::Cons(r) => r.made(),
            _ => vec![],
        }
    }

    /// Its points, as written.
    pub fn points(&self) -> Vec<&Pt> {
        match self {
            Row::Pt(p) => vec![p],
            Row::Line(a, b) => vec![a, b],
            Row::Circle(c, _) => vec![c],
            Row::Arc(c, a, b, _) => vec![c, a, b],
            Row::Ellipse(c, m, _) => vec![c, m],
            Row::Poly(ps, _) | Row::Spline(ps, _) | Row::BSpline(_, ps, _) => ps.iter().collect(),
            Row::Cons(r) => r.points(),
            _ => vec![],
        }
    }

    pub fn points_mut(&mut self) -> Vec<&mut Pt> {
        match self {
            Row::Pt(p) => vec![p],
            Row::Line(a, b) => vec![a, b],
            Row::Circle(c, _) => vec![c],
            Row::Arc(c, a, b, _) => vec![c, a, b],
            Row::Ellipse(c, m, _) => vec![c, m],
            Row::Poly(ps, _) | Row::Spline(ps, _) | Row::BSpline(_, ps, _) => ps.iter_mut().collect(),
            Row::Cons(r) => r.points_mut(),
            _ => vec![],
        }
    }

    /// Every reference it makes to the sketch's entities and points, in order.
    pub fn refs(&self) -> Vec<&Ref> {
        let mut out: Vec<&Ref> = self.points().into_iter().filter_map(Pt::named).collect();
        match self {
            Row::Fix(rs) | Row::Hor(rs) | Row::Ver(rs) | Row::Eq(rs) | Row::Dim(_, rs, _) => out.extend(rs),
            Row::Co(a, b)
            | Row::Par(a, b)
            | Row::Perp(a, b)
            | Row::Tan(a, b)
            | Row::Conc(a, b)
            | Row::Colin(a, b)
            | Row::Mid(a, b) => out.extend([a, b]),
            Row::Sym(a, b, c) => out.extend([a, b, c]),
            _ => {}
        }
        out
    }

    pub fn refs_mut(&mut self) -> Vec<&mut Ref> {
        match self {
            Row::Fix(rs) | Row::Hor(rs) | Row::Ver(rs) | Row::Eq(rs) | Row::Dim(_, rs, _) => {
                rs.iter_mut().collect()
            }
            Row::Co(a, b)
            | Row::Par(a, b)
            | Row::Perp(a, b)
            | Row::Tan(a, b)
            | Row::Conc(a, b)
            | Row::Colin(a, b)
            | Row::Mid(a, b) => vec![a, b],
            Row::Sym(a, b, c) => vec![a, b, c],
            r => r
                .points_mut()
                .into_iter()
                .filter_map(|p| match p {
                    Pt::R(r) => Some(r),
                    Pt::At(..) => None,
                })
                .collect(),
        }
    }

    /// Every value it writes, in order: its points' numbers, then its sizes.
    pub fn values(&self) -> Vec<&V> {
        fn pts<'a>(out: &mut Vec<&'a V>, ps: &[&'a Pt]) {
            for p in ps {
                if let Pt::At(x, y) = p {
                    out.extend([x, y]);
                }
            }
        }
        let mut out: Vec<&V> = vec![];
        match self {
            Row::Rect(x, y, w, h) => out.extend([x, y, w, h]),
            Row::Circle(c, r) => {
                pts(&mut out, &[c]);
                out.push(r);
            }
            Row::Ellipse(c, m, b) => {
                pts(&mut out, &[c, m]);
                out.push(b);
            }
            Row::BSpline(d, ps, _) => {
                out.push(d);
                pts(&mut out, &ps.iter().collect::<Vec<_>>());
            }
            Row::Project { approx, .. } => out.extend(approx),
            Row::Dim(_, _, v) => out.extend(v),
            Row::Cons(r) => return r.values(),
            r => pts(&mut out, &r.points()),
        }
        out
    }

    pub fn values_mut(&mut self) -> Vec<&mut V> {
        fn pts<'a>(out: &mut Vec<&'a mut V>, ps: Vec<&'a mut Pt>) {
            for p in ps {
                if let Pt::At(x, y) = p {
                    out.extend([x, y]);
                }
            }
        }
        let mut out: Vec<&mut V> = vec![];
        match self {
            Row::Rect(x, y, w, h) => out.extend([x, y, w, h]),
            Row::Circle(c, r) => {
                pts(&mut out, vec![c]);
                out.push(r);
            }
            Row::Ellipse(c, m, b) => {
                pts(&mut out, vec![c, m]);
                out.push(b);
            }
            Row::BSpline(d, ps, _) => {
                out.push(d);
                pts(&mut out, ps.iter_mut().collect());
            }
            Row::Project { approx, .. } => out.extend(approx),
            Row::Dim(_, _, v) => out.extend(v),
            Row::Cons(r) => return r.values_mut(),
            r => pts(&mut out, r.points_mut()),
        }
        out
    }

    /// A dimension with a value: what it sets.
    pub fn dim(&self) -> Option<(D, &[Ref], &V)> {
        match self {
            Row::Dim(d, rs, Some(v)) => Some((*d, rs, v)),
            _ => None,
        }
    }

    /// The same row in all but its dimension's value.
    fn same_but_value(&self, o: &Row) -> bool {
        match (self, o) {
            (Row::Dim(a, x, Some(_)), Row::Dim(b, y, Some(_))) => a == b && x == y,
            (a, b) => a == b,
        }
    }
}

fn list<T: fmt::Display>(xs: &[T]) -> String {
    xs.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ")
}

/// The Forge it reads as (names left to the block that writes it): what a message quotes.
impl fmt::Display for Row {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let open = |o: bool| if o { ", closed: false" } else { "" };
        match self {
            Row::Pt(p) => write!(f, "pt({p})"),
            Row::Line(a, b) => write!(f, "line({a}, {b})"),
            Row::Poly(ps, o) => write!(f, "poly([{}]{})", list(ps), open(*o)),
            Row::Rect(x, y, w, h) => write!(f, "rect(corner: ({x}, {y}), size: ({w}, {h}))"),
            Row::Circle(c, r) => write!(f, "circle({c}, {r})"),
            Row::Arc(c, a, b, cw) => write!(
                f,
                "arc(center: {c}, from: {a}, to: {b}{})",
                if *cw { ", cw: true" } else { "" }
            ),
            // a through-spline is open unless it says closed
            Row::Spline(ps, o) => write!(
                f,
                "spline([{}]{})",
                list(ps),
                if *o { "" } else { ", closed: true" }
            ),
            Row::BSpline(d, ps, o) => write!(f, "spline([{}], degree: {d}, cv: true{})", list(ps), open(*o)),
            Row::Ellipse(c, m, b) => write!(f, "ellipse(center: {c}, major: {m}, b: {b})"),
            Row::Project { of, approx, .. } => match approx {
                Some(t) => write!(f, "project({of}, approx: {t})"),
                None => write!(f, "project({of})"),
            },
            Row::Cons(r) => write!(f, "{r} (construction)"),
            Row::Fix(rs) => write!(f, "fix({})", list(rs)),
            Row::Co(a, b) => write!(f, "coincident({a}, {b})"),
            Row::Hor(rs) => write!(f, "horizontal({})", list(rs)),
            Row::Ver(rs) => write!(f, "vertical({})", list(rs)),
            Row::Par(a, b) => write!(f, "parallel({a}, {b})"),
            Row::Perp(a, b) => write!(f, "perpendicular({a}, {b})"),
            Row::Tan(a, b) => write!(f, "tangent({a}, {b})"),
            Row::Conc(a, b) => write!(f, "concentric({a}, {b})"),
            Row::Colin(a, b) => write!(f, "collinear({a}, {b})"),
            Row::Eq(rs) => write!(f, "equal({})", list(rs)),
            Row::Mid(a, b) => write!(f, "midpoint({a}, {b})"),
            Row::Sym(a, b, c) => write!(f, "symmetric({a}, {b}, {c})"),
            Row::Dim(d, rs, Some(v)) => write!(f, "{}({}, {v})", d.word(), list(rs)),
            Row::Dim(d, rs, None) => write!(f, "{}({}) (driven)", d.word(), list(rs)),
        }
    }
}

/// What one reference of a `project` row gives: a piece of the part to lay flat, or (a sketch
/// opened from its Forge block) the image the block solved to.
#[derive(Clone, Debug, PartialEq)]
pub enum Src {
    Model(Source),
    Laid(Flat),
}

/// What one reference of a `project` row names: each source with its entity's name, and whether
/// it is always just one (an edge), so rows may name it (`j1`).
#[derive(Clone, Debug, PartialEq)]
pub struct Refd {
    pub sources: Vec<(Name, Src)>,
    pub one: bool,
}

/// Resolves a `project` row's geometry, by what it names (Forge text).
pub type Sources<'a> = dyn Fn(&str) -> R<Vec<Refd>> + 'a;

/// A sketch's rows solved: the geometry later tools read (`Deref`s to it), and what the next
/// solve with only dimension values changed starts from.
#[derive(Clone, Debug, PartialEq)]
pub struct Solved {
    pub sketch: forge_model::Sketch,
    pub rows: Vec<Row>,
    /// Each driven dimension: its row and the value it measures (mm, or radians for an angle).
    pub driven: Vec<(usize, f64)>,
    /// The solver's parameters as written, each dimension row's constraint, each driven row's
    /// entities.
    written: Vec<f64>,
    dims: Vec<(usize, ConstraintId)>,
    driven_of: Vec<(D, Vec<EntityId>)>,
}

impl std::ops::Deref for Solved {
    type Target = forge_model::Sketch;
    fn deref(&self) -> &forge_model::Sketch {
        &self.sketch
    }
}

impl std::ops::DerefMut for Solved {
    fn deref_mut(&mut self) -> &mut forge_model::Sketch {
        &mut self.sketch
    }
}

/// Builds the rows on `plane` and solves them from the coordinates as written. `was`, how they
/// solved before, lends the solve every part of it that did not change.
pub fn build(plane: Frame, rows: Vec<Row>, num: &Num, was: Option<&Solved>) -> R<Solved> {
    let none = |_: &str| Err("this needs a part to project from".to_string());
    build_on(plane, rows, num, &none, was)
}

/// `build`, with what `project` rows name.
pub fn build_on<'n>(
    plane: Frame,
    rows: Vec<Row>,
    num: &Num<'n>,
    src: &Sources<'n>,
    was: Option<&Solved>,
) -> R<Solved> {
    if let Some(s) = was.and_then(|w| values_only(plane, &rows, num, w)) {
        return Ok(Solved { rows, ..s });
    }
    let mut b = B {
        s: forge_sketch::Sketch::new(plane),
        names: Vec::new(),
        made: Vec::new(),
        driven: Vec::new(),
        num,
        src,
        pins: Vec::new(),
        flats: Vec::new(),
    };
    for (k, row) in rows.iter().enumerate() {
        b.row(k, row).map_err(|e| format!("`{row}`: {e}"))?;
    }
    let written = b.s.params().to_vec();
    if b.s.solve_after(was.map(|w| &w.sketch.solved)).status != Status::Solved {
        let a = b.s.analyze();
        let say = |ids: &[ConstraintId]| {
            let mut ks: Vec<usize> = ids
                .iter()
                .filter_map(|c| b.made.iter().find(|m| m.0 == *c).map(|m| m.1))
                .collect();
            ks.sort_unstable();
            ks.dedup();
            ks.iter()
                .map(|&k| format!("`{}`", rows[k]))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let mut msg = format!("the sketch does not solve: {} conflict", say(&a.conflicting));
        let more: Vec<ConstraintId> = a
            .redundant
            .iter()
            .filter(|c| !a.conflicting.contains(c))
            .copied()
            .collect();
        if !more.is_empty() {
            msg += &format!("; with {}", say(&more));
        }
        return Err(msg);
    }
    let implied: Vec<usize> =
        b.s.implied()
            .iter()
            .filter_map(|c| b.made.iter().find(|m| m.0 == *c).map(|m| m.1))
            .collect();
    if let Some(&k) = implied.first() {
        let row = &rows[k];
        let fix = match row {
            Row::Dim(d, rs, Some(_)) => format!(
                "remove it, or make it driven (it then shows its value): `{}`",
                Row::Dim(*d, rs.clone(), None)
            ),
            _ => "remove it".to_string(),
        };
        return Err(format!(
            "`{row}` is implied by the rows above it (over-constrained): {fix}"
        ));
    }
    let driven = b
        .driven
        .iter()
        .map(|(k, d, es)| Ok((*k, b.measure(*d, es).map_err(|e| format!("`{}`: {e}", rows[*k]))?)))
        .collect::<R<_>>()?;
    let dims = b
        .made
        .iter()
        .filter(|m| rows[m.1].dim().is_some())
        .map(|m| (m.1, m.0))
        .collect();
    let driven_of = b.driven.iter().map(|d| (d.1, d.2.clone())).collect();
    Ok(Solved {
        sketch: forge_model::Sketch {
            plane,
            on: None,
            at: None,
            solved: b.s,
            names: b.names,
            note: Default::default(),
        },
        rows,
        driven,
        written,
        dims,
        driven_of,
    })
}

/// `was` again with new dimension values, when nothing else in the rows changed: the
/// dimensions set in place and solved from the coordinates as written, which is what reading
/// the rows afresh gives. None when anything else changed, or it fails (the full read says why).
fn values_only(plane: Frame, rows: &[Row], num: &Num, was: &Solved) -> Option<Solved> {
    if was.plane != plane || was.rows.len() != rows.len() {
        return None;
    }
    // every value but a dimension's is a number, as before; no part's geometry to read again
    for (a, b) in rows.iter().zip(&was.rows) {
        if !a.same_but_value(b) || matches!(a.plain(), Row::Project { .. }) {
            return None;
        }
        let fixed = match a {
            Row::Dim(_, _, Some(_)) => true,
            r => r.values().iter().all(|v| v.num().is_some()),
        };
        if !fixed {
            return None;
        }
    }
    let mut q = was.sketch.solved.clone();
    q.set_params(&was.written).ok()?;
    for &(k, c) in &was.dims {
        let (d, _, v) = rows[k].dim()?;
        let want = if d == D::Ang { Want::Angle } else { Want::Len };
        q.set_value(c, v.read(num, want).ok()?).ok()?;
    }
    if q.solve_after(Some(&was.sketch.solved)).status != Status::Solved || !q.implied().is_empty() {
        return None;
    }
    let b = B {
        s: q,
        names: was.names.clone(),
        made: Vec::new(),
        driven: Vec::new(),
        num,
        src: &|_| Ok(Vec::new()),
        pins: Vec::new(),
        flats: Vec::new(),
    };
    let driven = was
        .driven
        .iter()
        .zip(&was.driven_of)
        .map(|(d, (kw, es))| b.measure(*kw, es).ok().map(|v| (d.0, v)))
        .collect::<Option<Vec<_>>>()?;
    Some(Solved {
        sketch: forge_model::Sketch {
            plane,
            on: None,
            at: None,
            solved: b.s,
            names: b.names,
            note: Default::default(),
        },
        rows: Vec::new(),
        driven,
        written: was.written.clone(),
        dims: was.dims.clone(),
        driven_of: was.driven_of.clone(),
    })
}

struct B<'a, 'n> {
    s: forge_sketch::Sketch,
    names: Vec<(String, EntityId)>,
    /// Each constraint and the row that made it.
    made: Vec<(ConstraintId, usize)>,
    /// Each driven dimension: its row, kind and entities.
    driven: Vec<(usize, D, Vec<EntityId>)>,
    num: &'a Num<'n>,
    src: &'a Sources<'n>,
    /// The projected points so far: where each is held.
    pins: Vec<(Point2, EntityId)>,
    /// The projected lines, circles and arcs so far.
    flats: Vec<EntityId>,
}

use forge_model::Sketch;

fn bad(e: SketchError) -> String {
    match e {
        SketchError::WrongKind(_) => "an entity of the wrong kind for this row".into(),
        SketchError::BadValue => "a value out of range (lengths and radii > 0)".into(),
        e => format!("{e:?}"),
    }
}

/// Whether a projected entity is this one again: the same held points, the same way.
fn same_flat(s: &forge_sketch::Sketch, e: EntityId, kind: Entity, r: f64) -> bool {
    match (s.entity(e), kind) {
        (Some(Entity::Line { a, b }), Entity::Line { a: x, b: y }) => (a, b) == (x, y) || (a, b) == (y, x),
        (Some(Entity::Circle { center, .. }), Entity::Circle { center: c, .. }) => {
            center == c && (s.radius(e) - r).abs() <= tol::LINEAR
        }
        (
            Some(Entity::Arc {
                center,
                start,
                end,
                ccw,
            }),
            Entity::Arc {
                center: c,
                start: x,
                end: y,
                ccw: w,
            },
        ) => center == c && ((start, end, ccw) == (x, y, w) || (start, end, ccw) == (y, x, !w)),
        _ => false,
    }
}

impl B<'_, '_> {
    fn add(&mut self, k: usize, c: C) -> R<()> {
        let id = self.s.add_constraint(c).map_err(bad)?;
        self.made.push((id, k));
        Ok(())
    }

    /// A held point at `p`; one already within LINEAR is that point, so what meets there shares it.
    fn pin(&mut self, k: usize, p: Point2) -> R<EntityId> {
        if let Some(at) = self.pins.iter().find(|q| q.0.distance(p) <= tol::LINEAR) {
            return Ok(at.1);
        }
        let id = self.s.add_point(p);
        self.add(k, C::Fixed(id))?;
        self.pins.push((p, id));
        Ok(id)
    }

    /// The entity for a laid-flat source: the one already made if the same again.
    fn laid(&mut self, k: usize, f: Flat) -> R<EntityId> {
        if let Flat::Spline(pts, closed) = &f {
            let ids = pts.iter().map(|&p| self.pin(k, p)).collect::<R<Vec<_>>>()?;
            return self.s.add_spline(&ids, *closed).map_err(bad);
        }
        if let Flat::Ellipse {
            center,
            major,
            b,
            ends,
        } = f
        {
            let (c, m) = (self.pin(k, center)?, self.pin(k, major)?);
            let ends = match ends {
                Some((p, q, ccw)) => Some((self.pin(k, p)?, self.pin(k, q)?, ccw)),
                None => None,
            };
            let same = |s: &forge_sketch::Sketch, e: EntityId| match s.entity(e) {
                Some(Entity::Ellipse {
                    center,
                    major,
                    b: r,
                    ends: x,
                }) => {
                    let ends_same = match (x, ends) {
                        (None, None) => true,
                        (Some((p, q, w)), Some((p2, q2, w2))) => {
                            (p, q, w) == (p2, q2, w2) || (p, q, w) == (q2, p2, !w2)
                        }
                        _ => false,
                    };
                    center == c && major == m && (s.params()[r] - b).abs() <= tol::LINEAR && ends_same
                }
                _ => false,
            };
            if let Some(&e) = self.flats.iter().find(|&&e| same(&self.s, e)) {
                return Ok(e);
            }
            let e = match ends {
                Some((p, q, ccw)) => self.s.add_ellipse_arc(c, m, b, p, q, ccw),
                None => self.s.add_ellipse(c, m, b),
            }
            .map_err(bad)?;
            // an arc's ends, held, hold its minor axis
            if ends.is_none() {
                self.add(k, C::EllipseMinor(e, b))?;
            }
            self.flats.push(e);
            return Ok(e);
        }
        let r = if let Flat::Circle(_, r) = f { r } else { 0.0 };
        let kind = match f {
            Flat::Point(p) => return self.pin(k, p),
            Flat::Line(a, b) => {
                let (a, b) = (self.pin(k, a)?, self.pin(k, b)?);
                if a == b {
                    return Ok(a);
                }
                Entity::Line { a, b }
            }
            Flat::Circle(c, _) => Entity::Circle {
                center: self.pin(k, c)?,
                r: 0,
            },
            Flat::Arc {
                center,
                start,
                end,
                ccw,
            } => Entity::Arc {
                center: self.pin(k, center)?,
                start: self.pin(k, start)?,
                end: self.pin(k, end)?,
                ccw,
            },
            Flat::Ellipse { .. } | Flat::Spline(..) => unreachable!("made above"),
        };
        if let Some(&e) = self.flats.iter().find(|&&e| same_flat(&self.s, e, kind, r)) {
            return Ok(e);
        }
        let e = match kind {
            Entity::Line { a, b } => self.s.add_line(a, b),
            Entity::Circle { center, .. } => self.s.add_circle(center, r),
            Entity::Arc {
                center,
                start,
                end,
                ccw,
            } => self.s.add_arc(center, start, end, ccw),
            _ => unreachable!("only those are made above"),
        }
        .map_err(bad)?;
        if matches!(kind, Entity::Circle { .. }) {
            self.add(k, C::Radius(e, r))?;
        }
        self.flats.push(e);
        Ok(e)
    }

    /// `project`: each reference's geometry, held where it falls on the plane.
    fn project(&mut self, k: usize, of: &str, many: bool, approx: Option<&V>) -> R<()> {
        // an `approx` tolerance: what has no exact image is fitted within it, and says so
        let approx = match approx {
            Some(v) => {
                let t = v.read(self.num, Want::Len)?;
                if !(t >= 1e-6 && t.is_finite()) {
                    return Err("approx takes a tolerance of 0.000001 mm or more".into());
                }
                Some(t)
            }
            None => None,
        };
        let refs = (self.src)(of)?;
        for r in refs {
            for (n, s) in &r.sources {
                let f = match s {
                    Src::Model(s) => flat(&self.s.plane, s, approx)?,
                    Src::Laid(f) => f.clone(),
                };
                let e = self.laid(k, f)?;
                if r.one && !many {
                    self.name('j', e);
                } else {
                    // how many a face gives can change: no row may name one, and its curves take
                    // their names from the edge each came from
                    self.names.push((hidden(*n), e));
                }
            }
        }
        Ok(())
    }

    fn name(&mut self, kind: char, id: EntityId) {
        let n = self
            .names
            .iter()
            .filter(|(s, _)| s.starts_with(kind) && s != "o")
            .count()
            + 1;
        self.names.push((format!("{kind}{n}"), id));
    }

    /// The entity a name means (`o` made on first use: the fixed origin).
    fn entity(&mut self, t: &str) -> R<EntityId> {
        if let Some((_, id)) = self.names.iter().find(|(n, _)| n == t) {
            return Ok(*id);
        }
        if t == "o" {
            let id = self.s.add_point(Point2::ORIGIN);
            self.s.add_constraint(C::Fixed(id)).map_err(bad)?;
            self.names.push(("o".into(), id));
            return Ok(id);
        }
        let hint = if self.names.iter().any(|n| n.0.starts_with('#')) {
            "; a face's projected curves can change in number, so rows can't name them: project its edges one by one"
        } else {
            ""
        };
        Err(format!("no entity '{t}' above this row{hint}"))
    }

    /// A point: `name.a|b|c`, a point's name, or a circle's or arc's (its centre).
    fn point_ref(&mut self, r: &Ref) -> R<EntityId> {
        let e = self.entity(&r.base)?;
        let kind = self.s.entity(e).expect("named");
        if let Entity::Spline { .. } = kind {
            let pts = self.s.spline_points(e).expect("a spline");
            return match r.part {
                Part::A => Ok(pts[0]),
                Part::B => Ok(pts[pts.len() - 1]),
                Part::K(i) => pts.get(i).copied().ok_or_else(|| {
                    format!(
                        "'{r}' is not a point of the spline ({} points, from 0)",
                        pts.len()
                    )
                }),
                _ => Err(format!("'{r}' is not a point")),
            };
        }
        Ok(match (kind, &r.part) {
            (Entity::Point { .. }, Part::Whole) => e,
            (Entity::Ellipse { center, .. }, Part::Whole | Part::C) => center,
            (Entity::Ellipse { major, .. }, Part::M) => major,
            (
                Entity::Ellipse {
                    ends: Some((a, _, _)),
                    ..
                },
                Part::A,
            ) => a,
            (
                Entity::Ellipse {
                    ends: Some((_, b, _)),
                    ..
                },
                Part::B,
            ) => b,
            (Entity::Line { a, .. }, Part::A) | (Entity::Arc { start: a, .. }, Part::A) => a,
            (Entity::Line { b, .. }, Part::B) | (Entity::Arc { end: b, .. }, Part::B) => b,
            (Entity::Circle { center, .. } | Entity::Arc { center, .. }, Part::Whole | Part::C) => center,
            _ => return Err(format!("'{r}' is not a point")),
        })
    }

    fn row(&mut self, k: usize, row: &Row) -> R<()> {
        let made = self.names.len();
        let (row, cons) = match row {
            Row::Cons(r) => (r.plain(), true),
            r => (r, false),
        };
        match row {
            Row::Pt(p) => {
                let p = self.point(p)?;
                self.name('p', p);
            }
            Row::Line(a, b) => {
                let (a, b) = (self.point(a)?, self.point(b)?);
                let l = self.s.add_line(a, b).map_err(bad)?;
                self.name('l', l);
            }
            Row::Poly(ps, open) => {
                let mut pts = Vec::new();
                for p in ps {
                    pts.push(self.point(p)?);
                }
                if pts.len() < 2 {
                    return Err("poly needs two points or more".into());
                }
                let n = pts.len() - *open as usize;
                for i in 0..n {
                    let l = self.s.add_line(pts[i], pts[(i + 1) % pts.len()]).map_err(bad)?;
                    self.name('l', l);
                }
            }
            Row::Rect(x, y, w, h) => {
                let (x, y) = (self.len(x)?, self.len(y)?);
                let (w, h) = (self.len(w)?, self.len(h)?);
                let p = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
                    .map(|(x, y)| self.s.add_point(Point2::new(x, y)));
                let mut ls = [EntityId(0); 4];
                for i in 0..4 {
                    ls[i] = self.s.add_line(p[i], p[(i + 1) % 4]).map_err(bad)?;
                    self.name('l', ls[i]);
                }
                self.add(k, C::Horizontal(ls[0]))?;
                self.add(k, C::Horizontal(ls[2]))?;
                self.add(k, C::Vertical(ls[1]))?;
                self.add(k, C::Vertical(ls[3]))?;
            }
            Row::Circle(c, r) => {
                let center = self.point(c)?;
                let r = self.len(r)?;
                let e = self.s.add_circle(center, r).map_err(bad)?;
                self.name('c', e);
            }
            Row::Project { of, many, approx } => self.project(k, of, *many, approx.as_ref())?,
            Row::Arc(c, a, b, cw) => {
                let (center, a, b) = (self.point(c)?, self.point(a)?, self.point(b)?);
                let e = self.s.add_arc(center, a, b, !cw).map_err(bad)?;
                self.name('a', e);
            }
            Row::Spline(ps, open) => {
                let mut pts = Vec::new();
                for p in ps {
                    pts.push(self.point(p)?);
                }
                let closed = !open;
                if pts.len() < 2 || (closed && pts.len() < 3) {
                    return Err("spline needs two points, or three to close".into());
                }
                let e = self.s.add_spline(&pts, closed).map_err(bad)?;
                self.name('s', e);
            }
            Row::BSpline(d, ps, open) => {
                let degree = d.read(self.num, Want::Count)? as usize;
                let mut pts = Vec::new();
                for p in ps {
                    pts.push(self.point(p)?);
                }
                let e = self.s.add_bspline(degree, &pts, None, None, !open).map_err(|e| {
                    format!(
                        "{}: a spline on control vertices needs more points than its degree",
                        bad(e)
                    )
                })?;
                self.name('s', e);
            }
            Row::Ellipse(c, m, b) => {
                let (center, major) = (self.point(c)?, self.point(m)?);
                let b = self.len(b)?;
                let e = self.s.add_ellipse(center, major, b).map_err(bad)?;
                self.name('e', e);
            }
            Row::Cons(_) => unreachable!("taken off above"),
            r => {
                if cons {
                    return Err("only geometry is construction".into());
                }
                return self.constraint(k, r);
            }
        }
        if cons {
            for i in made..self.names.len() {
                self.s.set_construction(self.names[i].1, true).map_err(bad)?;
            }
        }
        Ok(())
    }

    fn constraint(&mut self, k: usize, row: &Row) -> R<()> {
        let line = |s: &Self, e: EntityId| matches!(s.s.entity(e), Some(Entity::Line { .. }));
        let point = |s: &Self, e: EntityId| matches!(s.s.entity(e), Some(Entity::Point { .. }));
        match row {
            Row::Dim(d, rs, None) => {
                let mut es = Vec::new();
                for r in rs {
                    es.push(self.any(r)?);
                }
                let want = match d {
                    D::Dist | D::Ang => 1..=2,
                    D::Rad | D::Dia => 1..=1,
                    _ => 2..=2,
                };
                if !want.contains(&es.len()) {
                    return Err(format!(
                        "a driven {} names {} entities",
                        d.word(),
                        if want.start() == want.end() {
                            want.start().to_string()
                        } else {
                            "1 or 2".into()
                        }
                    ));
                }
                self.driven.push((k, *d, es));
            }
            Row::Fix(rs) => {
                for r in rs {
                    let e = if r.is_point() {
                        self.point_ref(r)?
                    } else {
                        self.entity(&r.base)?
                    };
                    let pts = match self.s.entity(e).expect("named") {
                        Entity::Point { .. } => vec![e],
                        Entity::Line { a, b } => vec![a, b],
                        Entity::Circle { center, .. } => {
                            let r = self.s.radius(e);
                            self.add(k, C::Radius(e, r))?;
                            vec![center]
                        }
                        Entity::Arc {
                            center, start, end, ..
                        } => vec![center, start, end],
                        Entity::Spline { .. } => self.s.spline_points(e).expect("a spline").to_vec(),
                        Entity::Ellipse {
                            center, major, ends, ..
                        } => {
                            let mut v = vec![center, major];
                            if let Some((a, b, _)) = ends {
                                v.extend([a, b]);
                            }
                            v
                        }
                    };
                    for p in pts {
                        self.add(k, C::Fixed(p))?;
                    }
                }
            }
            Row::Co(a, b) => {
                let (a, b) = (self.any(a)?, self.any(b)?);
                let (p, q) = if point(self, a) { (a, b) } else { (b, a) };
                if !point(self, p) {
                    return Err("coincident needs a point".into());
                }
                let con = match self.s.entity(q).expect("named") {
                    Entity::Point { .. } => C::Coincident(p, q),
                    Entity::Line { .. } => C::PointOnLine(p, q),
                    Entity::Ellipse { .. } => C::PointOnEllipse(p, q),
                    Entity::Spline { .. } => {
                        return Err(
                            "coincident on a spline: name its point (s1.a, s1.b, s1.0, s1.1 ..)".into(),
                        );
                    }
                    _ => C::PointOnCircle(p, q),
                };
                self.add(k, con)?;
            }
            Row::Hor(rs) | Row::Ver(rs) => {
                let hor = matches!(row, Row::Hor(_));
                let mut es = Vec::new();
                for r in rs {
                    es.push(self.any(r)?);
                }
                if es.len() == 2 && !es.iter().any(|&e| line(self, e)) {
                    let (p, q) = (self.as_point(es[0])?, self.as_point(es[1])?);
                    let con = if hor {
                        C::VerticalDistance(p, q, 0.0)
                    } else {
                        C::HorizontalDistance(p, q, 0.0)
                    };
                    return self.add(k, con);
                }
                if es.is_empty() {
                    return Err("horizontal and vertical need lines, or two points".into());
                }
                for e in es {
                    self.add(k, if hor { C::Horizontal(e) } else { C::Vertical(e) })?;
                }
            }
            Row::Par(a, b) | Row::Perp(a, b) | Row::Tan(a, b) | Row::Conc(a, b) => {
                let (ra, rb) = (a, b);
                let (a, b) = (self.entity_of(ra)?, self.entity_of(rb)?);
                if let Row::Tan(..) = row {
                    let kind = |s: &Self, e: EntityId| s.s.entity(e).expect("named");
                    match (kind(self, a), kind(self, b)) {
                        (Entity::Spline { .. }, _) | (_, Entity::Spline { .. }) => {
                            let (sp, other) = if matches!(kind(self, a), Entity::Spline { .. }) {
                                (a, b)
                            } else {
                                (b, a)
                            };
                            if let Some(id) = self.s.tangent_spline(sp, other).map_err(|_| {
                                "tangent to a spline: a line or arc that ends at the spline's start or end"
                                    .to_string()
                            })? {
                                self.made.push((id, k));
                            }
                            return Ok(());
                        }
                        (Entity::Ellipse { .. }, Entity::Line { .. })
                        | (Entity::Line { .. }, Entity::Ellipse { .. }) => {
                            let (l, e) = if matches!(kind(self, a), Entity::Line { .. }) {
                                (a, b)
                            } else {
                                (b, a)
                            };
                            let ids = self.s.tangent_ellipse(l, e).map_err(bad)?;
                            self.made.extend(ids.map(|id| (id, k)));
                            return Ok(());
                        }
                        _ => {}
                    }
                }
                let con = match row {
                    Row::Par(..) => C::Parallel(a, b),
                    Row::Perp(..) => C::Perpendicular(a, b),
                    Row::Tan(..) => C::Tangent(a, b),
                    _ => C::Concentric(a, b),
                };
                self.add(k, con)?;
            }
            Row::Colin(a, b) => {
                // parallel, and the second's start on the first
                let (a, b) = (self.entity_of(a)?, self.entity_of(b)?);
                let Some(Entity::Line { a: start, .. }) = self.s.entity(b) else {
                    return Err("collinear takes two lines".into());
                };
                if !line(self, a) {
                    return Err("collinear takes two lines".into());
                }
                self.add(k, C::Parallel(a, b))?;
                self.add(k, C::PointOnLine(start, a))?;
            }
            Row::Eq(rs) => {
                let Some((first, rest)) = rs.split_first() else {
                    return Err("equal needs two entities or more".into());
                };
                let a = self.entity_of(first)?;
                if rest.is_empty() {
                    return Err("equal needs two entities or more".into());
                }
                for r in rest {
                    let b = self.entity_of(r)?;
                    self.add(k, C::Equal(a, b))?;
                }
            }
            Row::Mid(p, l) => {
                let p = self.point_ref(p)?;
                let l = self.entity_of(l)?;
                self.add(k, C::Midpoint(p, l))?;
            }
            Row::Sym(p, q, l) => {
                let (p, q) = (self.point_ref(p)?, self.point_ref(q)?);
                let l = self.entity_of(l)?;
                self.add(k, C::Symmetric(p, q, l))?;
            }
            Row::Dim(D::Dist, rs, Some(v)) => match &rs[..] {
                [a, b] => {
                    let (a, b) = (self.any(a)?, self.any(b)?);
                    let d = self.len(v)?;
                    let con = match (line(self, a), line(self, b)) {
                        (false, false) => C::Distance(self.as_point(a)?, self.as_point(b)?, d),
                        (true, false) => C::DistancePointLine(self.as_point(b)?, a, d),
                        (false, true) => C::DistancePointLine(self.as_point(a)?, b, d),
                        _ => {
                            return Err(
                                "a distance between two lines: make them parallel and dimension a point to a line"
                                    .into(),
                            );
                        }
                    };
                    self.add(k, con)?;
                }
                [a] => {
                    let a = self.any(a)?;
                    if !line(self, a) {
                        return Err("a distance on one entity needs a line".into());
                    }
                    let d = self.len(v)?;
                    self.add(k, C::Length(a, d))?;
                }
                _ => return Err("a distance is on a line, or between two".into()),
            },
            Row::Dim(d @ (D::HDist | D::VDist), rs, Some(v)) => {
                let [p, q] = &rs[..] else {
                    return Err("a horizontal or vertical distance is between two points".into());
                };
                let (p, q) = (self.point_ref(p)?, self.point_ref(q)?);
                let x = self.len(v)?;
                let con = if *d == D::HDist {
                    C::HorizontalDistance(p, q, x)
                } else {
                    C::VerticalDistance(p, q, x)
                };
                self.add(k, con)?;
            }
            Row::Dim(d @ (D::Rad | D::Dia), rs, Some(v)) => {
                let [e] = &rs[..] else {
                    return Err("a radius or diameter is of one circle, arc or ellipse".into());
                };
                let e = self.entity_of(e)?;
                let x = self.len(v)?;
                if matches!(self.s.entity(e), Some(Entity::Ellipse { .. })) {
                    let x = if *d == D::Rad { x } else { 0.5 * x };
                    return self.add(k, C::EllipseMinor(e, x));
                }
                self.add(
                    k,
                    if *d == D::Rad {
                        C::Radius(e, x)
                    } else {
                        C::Diameter(e, x)
                    },
                )?;
            }
            Row::Dim(D::Ang, rs, Some(v)) => {
                let [a, b] = &rs[..] else {
                    return Err("an angle is between two lines".into());
                };
                let (a, b) = (self.entity_of(a)?, self.entity_of(b)?);
                let t = v.read(self.num, Want::Angle)?;
                self.add(k, C::Angle(a, b, t))?;
            }
            r => return Err(format!("`{r}` is no constraint")),
        }
        Ok(())
    }

    /// What a driven dimension measures on the solved sketch.
    fn measure(&self, kw: D, es: &[EntityId]) -> R<f64> {
        let q = &self.s;
        let pt = |e: EntityId| self.as_point(e).map(|p| q.point(p));
        let line = |e: EntityId| match q.entity(e) {
            Some(Entity::Line { a, b }) => Some((q.point(a), q.point(b))),
            _ => None,
        };
        let need_line = |e: EntityId| line(e).ok_or_else(|| "a line is needed".to_string());
        Ok(match (kw, es) {
            (D::Dist, [l]) => {
                let (a, b) = need_line(*l)?;
                a.distance(b)
            }
            (D::Dist, [x, y]) => {
                match (line(*x), line(*y)) {
                    (None, None) => pt(*x)?.distance(pt(*y)?),
                    (Some((a, b)), None) | (None, Some((a, b))) => {
                        let p = if line(*x).is_some() { pt(*y)? } else { pt(*x)? };
                        ((b - a).cross(p - a) / a.distance(b)).abs()
                    }
                    _ => return Err(
                        "a distance between two lines: make them parallel and dimension a point to a line"
                            .into(),
                    ),
                }
            }
            (D::HDist, [x, y]) => pt(*y)?.x - pt(*x)?.x,
            (D::VDist, [x, y]) => pt(*y)?.y - pt(*x)?.y,
            (D::Rad | D::Dia, [c]) if matches!(q.entity(*c), Some(Entity::Ellipse { .. })) => {
                let Some(Entity::Ellipse { b, .. }) = q.entity(*c) else {
                    unreachable!("just matched")
                };
                q.params()[b] * if kw == D::Dia { 2.0 } else { 1.0 }
            }
            (D::Rad | D::Dia, [c]) => {
                if !matches!(q.entity(*c), Some(Entity::Circle { .. } | Entity::Arc { .. })) {
                    return Err("a radius or diameter needs a circle or arc".into());
                }
                q.radius(*c) * if kw == D::Dia { 2.0 } else { 1.0 }
            }
            (D::Ang, [x, y]) => {
                let ((a, b), (c, d)) = (need_line(*x)?, need_line(*y)?);
                let (u, v) = (b - a, d - c);
                dmath::atan2(u.cross(v), u.dot(v))
            }
            _ => return Err(format!("a driven {}: wrong entities", kw.word())),
        })
    }

    /// A point named, or a new one there.
    fn point(&mut self, p: &Pt) -> R<EntityId> {
        match p {
            Pt::R(r) => self.point_ref(r),
            Pt::At(x, y) => {
                let (x, y) = (self.len(x)?, self.len(y)?);
                Ok(self.s.add_point(Point2::new(x, y)))
            }
        }
    }

    /// An entity, not a point of one.
    fn entity_of(&mut self, r: &Ref) -> R<EntityId> {
        if r.is_point() {
            return Err(format!("'{r}' is a point; this needs a line, circle or arc"));
        }
        self.entity(&r.base)
    }

    /// An entity, or a point of one (`l1.a`).
    fn any(&mut self, r: &Ref) -> R<EntityId> {
        if r.is_point() {
            self.point_ref(r)
        } else {
            self.entity(&r.base)
        }
    }

    /// A point as a point: a circle or arc stands for its centre.
    fn as_point(&self, e: EntityId) -> R<EntityId> {
        match self.s.entity(e).expect("named") {
            Entity::Point { .. } => Ok(e),
            Entity::Circle { center, .. } | Entity::Arc { center, .. } | Entity::Ellipse { center, .. } => {
                Ok(center)
            }
            Entity::Line { .. } => Err("a line where a point is wanted".into()),
            Entity::Spline { .. } => Err("a spline where a point is wanted: s1.a, s1.b, s1.0 ..".into()),
        }
    }

    fn len(&mut self, v: &V) -> R<f64> {
        v.read(self.num, Want::Len)
    }
}

fn id_of(s: &forge_model::Sketch, t: &str) -> R<EntityId> {
    s.names
        .iter()
        .find(|(n, _)| n == t)
        .map(|n| n.1)
        .ok_or_else(|| format!("the sketch has no '{t}'"))
}

/// The segment id an entity's profile edges carry: from its kind and count (`l3`), so it
/// stays put when other kinds are added or dimensions change.
fn stable(name: &str) -> u64 {
    if let Some(h) = name.strip_prefix('#') {
        return (u64::from_str_radix(h, 16).unwrap_or(0) >> 20) << 2 | 3 | 1 << 46;
    }
    let n: u64 = name[1..].parse().unwrap_or(0);
    let k = match name.as_bytes()[0] {
        b'l' => 0,
        b'a' => 1,
        b's' => 3,
        // two bits ran out: ellipses sit above any other kind's count
        b'e' => return n << 2 | 3 | 1 << 40,
        b'j' => return n << 2 | 3 | 1 << 41,
        _ => 2,
    };
    n << 2 | k
}

/// A profile segment's entity id, what a cut of a kinked or closed curve adds taken off (the
/// piece number stays on the stable id, so piece 0 reads as the entity).
const PIECE: u64 = 0xffff << 48;

fn name_of(s: &forge_model::Sketch, id: u64) -> &str {
    s.names
        .iter()
        .find(|(_, e)| e.0 == id)
        .map_or("?", |n| n.0.as_str())
}

/// The sketch's one region, or the one whose outside loop runs through `through`.
pub fn profile(s: &forge_model::Sketch, through: Option<&str>) -> R<Profile> {
    let ps = forge_sketch::profiles(&s.solved).map_err(|e| {
        let n = |id: EntityId| name_of(s, id.0).to_string();
        match e {
            ExtractError::Empty => "the sketch encloses nothing".into(),
            ExtractError::OpenChain { entity } => format!("the chain through {} is open", n(entity)),
            ExtractError::Branch { entity } => {
                format!("more than two curves meet at an end of {}", n(entity))
            }
            ExtractError::Degenerate { entity } => format!("{} has no length", n(entity)),
            ExtractError::SelfIntersection { a, b } => format!("{} and {} cross", n(a), n(b)),
            ExtractError::Profile(e) => format!("{e:?}"),
            ExtractError::Undecided { inner, outer } => {
                format!(
                    "whether the loop through {} is inside the one through {} is not certain",
                    n(inner),
                    n(outer)
                )
            }
        }
    })?;
    let mut p = match through {
        None if ps.len() == 1 => ps.into_iter().next().expect("one"),
        None => {
            let outs: Vec<&str> = ps
                .iter()
                .map(|p| match name_of(s, p.loops[0].segments[0].1 & !PIECE) {
                    n if n.starts_with('#') => "(a face's projected curve)",
                    n => n,
                })
                .collect();
            return Err(format!(
                "the sketch has {} regions; name one by an entity on its outside: {}",
                ps.len(),
                outs.join(" ")
            ));
        }
        Some(t) => {
            let id = id_of(s, t)?.0;
            ps.into_iter()
                .find(|p| p.loops[0].segments.iter().any(|g| g.1 & !PIECE == id))
                .ok_or_else(|| format!("{t} is not on the outside of a region"))?
        }
    };
    for l in &mut p.loops {
        for g in &mut l.segments {
            let (piece, name) = (g.1 & PIECE, name_of(s, g.1 & !PIECE));
            g.1 = piece | stable(name);
        }
    }
    Ok(p)
}

/// A closed region of a sketch, as a line that reads it would name it.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    /// An entity on its outside: what picks it out of the others.
    pub through: String,
    /// Its outside, then its holes, as points around each.
    pub loops: Vec<Vec<Point2>>,
    /// Its area (less its holes) and the length round every loop, exact from its curves (a
    /// spline's length to its samples).
    pub area: f64,
    pub perimeter: f64,
    /// Each loop's own area and length, in `loops`' order.
    pub loop_areas: Vec<f64>,
    pub loop_lengths: Vec<f64>,
}

/// A profile curve's length: exact for lines, arcs and circles.
fn segment_length(g: &forge_ops::Segment) -> f64 {
    use forge_ops::Segment;
    match g {
        Segment::Line { a, b } => (*b - *a).length(),
        Segment::Arc { radius, sweep, .. } => radius * sweep.abs(),
        Segment::Circle { radius, .. } => core::f64::consts::TAU * radius,
        Segment::Spline(_) => g.samples(512).windows(2).map(|w| (w[1] - w[0]).length()).sum(),
    }
}

/// Points along a curve, about one per `STEP` radians where it bends.
fn along(g: &forge_ops::Segment) -> Vec<Point2> {
    const STEP: f64 = core::f64::consts::TAU / 64.0;
    let arc = |c: Point2, r: f64, t0: f64, sweep: f64| -> Vec<Point2> {
        let n = ((sweep.abs() / STEP).ceil() as usize).max(1);
        (0..=n)
            .map(|k| {
                let t = t0 + sweep * k as f64 / n as f64;
                Point2::new(c.x + r * dmath::cos(t), c.y + r * dmath::sin(t))
            })
            .collect()
    };
    match g {
        forge_ops::Segment::Line { a, b } => vec![*a, *b],
        forge_ops::Segment::Arc {
            center,
            radius,
            start,
            sweep,
        } => arc(*center, *radius, *start, *sweep),
        forge_ops::Segment::Circle { center, radius, .. } => {
            arc(*center, *radius, 0.0, core::f64::consts::TAU)
        }
        forge_ops::Segment::Spline(_) => g.samples(64),
    }
}

/// The closed regions it has now: none if what's drawn doesn't close.
pub fn regions(s: &Sketch) -> Vec<Region> {
    let Ok(ps) = forge_sketch::profiles(&s.solved) else {
        return Vec::new();
    };
    ps.iter()
        .map(|p| Region {
            // one no row can name (a face's projected curves) leaves it to be the sketch's only region
            through: p.loops[0]
                .segments
                .iter()
                .map(|g| name_of(s, g.1 & !PIECE))
                .find(|n| !n.starts_with('#'))
                .unwrap_or("")
                .to_string(),
            loops: p
                .loops
                .iter()
                .map(|l| l.segments.iter().flat_map(|g| along(&g.0)).collect())
                .collect(),
            area: p.area(),
            perimeter: p
                .loops
                .iter()
                .flat_map(|l| &l.segments)
                .map(|g| segment_length(&g.0))
                .sum(),
            loop_areas: p.loops.iter().map(|l| l.signed_area().abs()).collect(),
            loop_lengths: p
                .loops
                .iter()
                .map(|l| l.segments.iter().map(|g| segment_length(&g.0)).sum())
                .collect(),
        })
        .collect()
}

/// What it draws: each curve as points, whether it's construction; and its end points.
pub fn drawing(s: &Sketch) -> (Vec<(Vec<Point2>, bool)>, Vec<Point2>) {
    let q = &s.solved;
    let (mut curves, mut points) = (Vec::new(), Vec::new());
    for id in q.entity_ids() {
        let faint = q.is_construction(id);
        let g = match q.entity(id) {
            Some(Entity::Spline { .. }) => {
                if let Some(pts) = q.spline_points(id) {
                    points.extend(pts.iter().map(|p| q.point(*p)));
                }
                curves.push((q.spline_samples(id, 64), faint));
                continue;
            }
            Some(Entity::Ellipse { center, .. }) => {
                points.push(q.point(center));
                curves.push((q.spline_samples(id, 64), faint));
                continue;
            }
            Some(Entity::Line { a, b }) => {
                points.extend([q.point(a), q.point(b)]);
                forge_ops::Segment::Line {
                    a: q.point(a),
                    b: q.point(b),
                }
            }
            Some(Entity::Circle { center, .. }) => {
                points.push(q.point(center));
                forge_ops::Segment::Circle {
                    center: q.point(center),
                    radius: q.radius(id),
                    ccw: true,
                }
            }
            Some(Entity::Arc {
                center,
                start,
                end,
                ccw,
            }) => {
                let (c, a, b) = (q.point(center), q.point(start), q.point(end));
                points.extend([a, b, c]);
                let t0 = dmath::atan2(a.y - c.y, a.x - c.x);
                let t1 = dmath::atan2(b.y - c.y, b.x - c.x);
                let turn = core::f64::consts::TAU;
                let sweep = if ccw {
                    (t1 - t0).rem_euclid(turn)
                } else {
                    -(t0 - t1).rem_euclid(turn)
                };
                forge_ops::Segment::Arc {
                    center: c,
                    radius: a.distance(c),
                    start: t0,
                    sweep,
                }
            }
            _ => continue,
        };
        curves.push((along(&g), faint));
    }
    (curves, points)
}

/// A point of the sketch (`p1`, `l2.b`, `c1` for its centre), in its plane.
pub fn point(s: &Sketch, t: &str) -> R<Point2> {
    id_of(s, t.split_once('.').map_or(t, |p| p.0))?;
    s.point(t).ok_or_else(|| format!("'{t}' is not a point"))
}

/// A line of the sketch: its ends, in its plane.
pub fn line(s: &Sketch, t: &str) -> R<(Point2, Point2)> {
    match s.solved.entity(id_of(s, t)?) {
        Some(Entity::Line { a, b }) => Ok((s.solved.point(a), s.solved.point(b))),
        _ => Err(format!("'{t}' is not a line")),
    }
}

/// The name of a result of `project` that rows can't name: `#` starts a comment, so none can write it.
fn hidden(n: Name) -> String {
    format!("#{:016x}", n.0)
}

/// Whether a name in a sketch is `project`'s.
pub fn is_projected(name: &str) -> bool {
    name.starts_with(['j', '#'])
}

/// Degrees of freedom left, and the named entities not pinned down.
pub fn freedom(s: &Sketch) -> (usize, Vec<String>) {
    let a = s.solved.analyze();
    let loose = s
        .names
        .iter()
        .filter(|(n, id)| n != "o" && !a.fully_constrained.contains(id))
        .map(|n| n.0.clone())
        .collect();
    (a.dof, loose)
}

/// Rows as the tests write them: values as Forge text, points and entities by name.
#[cfg(test)]
pub(crate) mod dsl {
    use super::{Pt, Ref, V};

    pub fn at(x: &str, y: &str) -> Pt {
        Pt::At(V::of(x), V::of(y))
    }

    pub fn pn(name: &str) -> Pt {
        Pt::of(name)
    }

    pub fn rf(name: &str) -> Ref {
        Ref::name(name)
    }

    pub fn v(t: &str) -> V {
        V::of(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::{Circle, Curve};
    use forge_math::{Interval, Point3};

    fn num(t: &str, _: Want) -> R<f64> {
        t.parse::<f64>().map_err(|e| e.to_string())
    }

    fn seg_ids(s: &Solved) -> Vec<u64> {
        let mut v: Vec<u64> = profile(s, None)
            .unwrap()
            .loops
            .iter()
            .flat_map(|l| l.segments.iter().map(|g| g.1))
            .collect();
        v.sort_unstable();
        v
    }

    #[test]
    fn a_profiles_segments_carry_ids_from_their_entities_names() {
        let rect = Row::Rect(V::N(0.0), V::N(0.0), V::N(10.0), V::N(10.0));
        let s = build(Frame::default(), vec![rect], &num, None).unwrap();
        assert_eq!(seg_ids(&s), [4, 8, 12, 16]);
    }

    fn edge(curve: Curve, range: Interval, start: Point3, end: Point3) -> Source {
        Source::Edge {
            curve,
            range,
            start,
            end,
            closed: false,
        }
    }

    #[test]
    fn an_arc_whose_ends_a_boolean_settled_off_it_still_solves_held() {
        let k = Circle {
            frame: Frame::default(),
            radius: 5.0,
        };
        let e = edge(
            Curve::Circle(k),
            Interval::new(0.0, 1.5),
            Point3::new(5.0 + 4e-6, 0.0, 0.0),
            Point3::new(5.0 * dmath::cos(1.5), 5.0 * dmath::sin(1.5), 0.0),
        );
        let src = |_: &str| {
            Ok(vec![Refd {
                sources: vec![(Name::NONE, Src::Model(e.clone()))],
                one: true,
            }])
        };
        let row = Row::Project {
            of: "e".into(),
            many: false,
            approx: None,
        };
        let s = build_on(Frame::default(), vec![row], &num, &src, None).unwrap();
        assert_eq!(freedom(&s).0, 0);
    }

    #[test]
    fn only_dimension_values_changed_solve_from_how_it_was() {
        let rows = |w: f64| {
            vec![
                Row::Line(Pt::at(0.0, 0.0), Pt::at(10.0, 0.0)),
                Row::Dim(D::Dist, vec![Ref::to("l1")], Some(V::N(w))),
                Row::Dim(D::Dist, vec![Ref::a("l1"), Ref::b("l1")], None),
            ]
        };
        let was = build(Frame::default(), rows(10.0), &num, None).unwrap();
        let now = build(Frame::default(), rows(12.0), &num, Some(&was)).unwrap();
        assert_eq!(now.rows, rows(12.0));
        assert!((now.driven[0].1 - 12.0).abs() < 1e-9, "{:?}", now.driven);
        // what a full read gives
        let fresh = build(Frame::default(), rows(12.0), &num, None).unwrap();
        let (a, b) = line(&now, "l1").unwrap();
        let (c, d) = line(&fresh, "l1").unwrap();
        assert!(a.distance(c) < 1e-9 && b.distance(d) < 1e-9);
    }
}
