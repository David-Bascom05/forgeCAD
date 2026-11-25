//! A part 21 file's boundary representations into bodies. Refusals name the entity and say why;
//! nothing in a file can panic this or make it loop.

use crate::heal::{PEdge, PFace, Proto};
use crate::nurbs::{self, Nurbs};
use crate::part21::{File, Limits, ParseError, Record, Value, parse};
use forge_geom::SurfaceGeom;
use forge_geom::spline::{Spline, SplineCurve};
use forge_geom::{
    Circle, Cone, Curve, CurveGeom, Cylinder, Ellipse, Hyperbola, Line, Parabola, Plane, Sphere, Surface,
    Torus,
};
use forge_math::{Box3, Frame, Interval, Point3, UnitVec3, Vec3, dmath};
use forge_topo::{Body, Name};
use std::collections::BTreeMap;

/// A body read from a file, with the entity it came from.
pub struct Solid {
    pub entity: u32,
    pub name: String,
    pub body: Body,
    /// The body's integrals as the read worked them out to check it (to 1e-6), where it could.
    pub mass: Option<forge_topo::mass::MassProps>,
    /// The body drawn as [`Solid::display_mesh`] draws it, where the read's own check drew
    /// exactly that (the same bits), so it is not drawn twice; with [`import_drawn`], wherever
    /// it can be drawn.
    pub mesh: Option<forge_mesh::TriMesh>,
}

impl Solid {
    /// The chord a file's body is drawn to for display: 5e-4 of its diagonal.
    pub fn display_quality(body: &Body) -> forge_mesh::Quality {
        forge_mesh::Quality {
            chord: 5e-4 * body.faces_box().diagonal(),
            ..forge_mesh::Quality::default()
        }
    }

    /// The body drawn as the app draws a file's body (`tessellate_tolerant` at
    /// [`Solid::display_quality`]): the read's own drawing where it kept it, else drawn now.
    pub fn display_mesh(&self) -> Result<std::borrow::Cow<'_, forge_mesh::TriMesh>, forge_mesh::TessError> {
        match &self.mesh {
            Some(m) => Ok(std::borrow::Cow::Borrowed(m)),
            None => {
                forge_mesh::tessellate::tessellate_tolerant(&self.body, &Self::display_quality(&self.body))
                    .map(std::borrow::Cow::Owned)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Refusal {
    pub entity: u32,
    pub kind: String,
    pub why: String,
}

/// What a read found. `deviation` is how far the file's geometry misses itself, measured before
/// any healing.
#[derive(Debug, Default)]
pub struct Stats {
    pub faces: usize,
    pub edges: usize,
    pub vertex_off_curve: f64,
    pub curve_off_surface: f64,
    /// Over which vertices and edges miss by more than `forge_math::tol::LINEAR`.
    pub vertices_off: usize,
    pub edges_off: usize,
    pub uncertainty: f64,
    pub units_mm: f64,
    /// Edges by curve kind and surface kind: how many, how many off by more than LINEAR, the most.
    pub pairs: BTreeMap<String, (usize, usize, f64)>,
    /// Edges left off a surface because their faces touch along them.
    pub tangent_off: usize,
    /// Edges that keep a tolerance above the kernel's, and the largest.
    pub tolerant_edges: usize,
    pub max_edge_tol: f64,
    /// What an assembly asks that is not done (`Body::unplaced`): parts moved from where they stand,
    /// and parts used more than once.
    pub assembly: usize,
}

pub struct Imported {
    pub solids: Vec<Solid>,
    pub refusals: Vec<Refusal>,
    /// Each use of a solid in the file's assembly, where it has one (`occurrences`).
    pub occurrences: Vec<Occurrence>,
    pub schema: String,
    pub system: String,
    pub stats: Stats,
}

#[derive(Debug)]
pub enum ImportError {
    Parse(ParseError),
    /// A `.brep` that does not read as one.
    Brep,
    /// An IGES file that does not read as one.
    Iges,
    NoSolids,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::Parse(e) => write!(f, "{e}"),
            ImportError::Brep => write!(f, "not a readable OpenCascade .brep file"),
            ImportError::Iges => write!(f, "not a readable IGES file"),
            ImportError::NoSolids => write!(f, "the file holds no solid"),
        }
    }
}

type R<T> = Result<T, Refusal>;

fn refuse<T>(entity: u32, kind: &str, why: impl Into<String>) -> R<T> {
    Err(Refusal {
        entity,
        kind: kind.to_string(),
        why: why.into(),
    })
}

/// Units in the shape's representation context: mm per length unit, radians per angle unit.
#[derive(Clone, Copy, Debug)]
struct Units {
    mm: f64,
    rad: f64,
    uncertainty: f64,
}

struct Reader<'a> {
    f: &'a File<'a>,
    u: Units,
    /// How deep a chain of wrapped curves has gone, which a hostile file could make circular.
    depth: u32,
}

const MAX_WRAP: u32 = 16;

/// A curve as degree, knots, points and weights.
pub(crate) type Net = (usize, Vec<f64>, Vec<Point3>, Vec<f64>);

fn kw<'x>(f: &File<'x>, id: u32) -> Option<&'x str> {
    let r = f.records_of(id)?;
    Some(f.name(r.first()?))
}

fn rec(f: &File, id: u32, name: &str) -> Option<Record> {
    f.records_of(id)?.iter().find(|r| f.name(r) == name).copied()
}

/// The numeric value inside a plain or typed value.
fn number(f: &File, v: &Value) -> Option<f64> {
    match *v {
        Value::Real(x) => Some(x),
        Value::Int(i) => Some(i as f64),
        Value::Typed(_, at) => f.values.get(at as usize).and_then(|x| number(f, x)),
        _ => None,
    }
}

/// Positive and finite: a size read from the file, after its unit (a finite number times the unit
/// can overflow to infinity).
fn positive(x: f64) -> bool {
    x > 0.0 && x.is_finite()
}

fn enum_of<'x>(f: &File<'x>, v: &Value) -> Option<&'x str> {
    match *v {
        Value::Enum(a, n) => Some(f.str_of(a, n)),
        _ => None,
    }
}

fn logical(f: &File, v: &Value) -> Option<bool> {
    match enum_of(f, v)? {
        "T" => Some(true),
        "F" => Some(false),
        _ => None,
    }
}

/// How many mm or radians one of a named unit is.
fn unit_scale(f: &File, id: u32, depth: u32) -> Option<f64> {
    if depth > 8 {
        return None;
    }
    let rs = f.records_of(id)?;
    for r in rs {
        match f.name(r) {
            "SI_UNIT" => {
                let a = f.args(r);
                // SI_UNIT(prefix, name) in its complex form, 2 args
                let n = a.len();
                let (pre, name) = (a.get(n.wrapping_sub(2))?, a.get(n.wrapping_sub(1))?);
                let k = match enum_of(f, pre) {
                    None => 1.0,
                    Some("MILLI") => 1e-3,
                    Some("CENTI") => 1e-2,
                    Some("DECI") => 1e-1,
                    Some("KILO") => 1e3,
                    Some("MICRO") => 1e-6,
                    Some("NANO") => 1e-9,
                    Some(_) => return None,
                };
                return match enum_of(f, name)? {
                    "METRE" => Some(k * 1000.0),
                    "RADIAN" | "STERADIAN" => Some(k),
                    _ => None,
                };
            }
            "CONVERSION_BASED_UNIT" => {
                let a = f.args(r);
                let Value::Ref(m) = *a.get(1)? else { return None };
                let mr = f.records_of(m)?;
                for x in mr {
                    let nm = f.name(x);
                    if nm == "LENGTH_MEASURE_WITH_UNIT"
                        || nm == "PLANE_ANGLE_MEASURE_WITH_UNIT"
                        || nm == "MEASURE_WITH_UNIT"
                    {
                        let b = f.args(x);
                        let k = number(f, b.first()?)?;
                        let Value::Ref(base) = *b.get(1)? else { return None };
                        return Some(k * unit_scale(f, base, depth + 1)?);
                    }
                }
                return None;
            }
            _ => {}
        }
    }
    None
}

fn units_of(f: &File, ctx: u32) -> Option<Units> {
    let rs = f.records_of(ctx)?;
    let (mut mm, mut rad, mut unc) = (None, None, None);
    for r in rs {
        match f.name(r) {
            "GLOBAL_UNIT_ASSIGNED_CONTEXT" => {
                let Value::List(a, n) = *f.args(r).first()? else {
                    return None;
                };
                for v in f.run(a, n) {
                    let Value::Ref(u) = *v else { continue };
                    let names: Vec<&str> = f.records_of(u)?.iter().map(|x| f.name(x)).collect();
                    if names.contains(&"LENGTH_UNIT") {
                        mm = unit_scale(f, u, 0);
                    } else if names.contains(&"PLANE_ANGLE_UNIT") {
                        rad = unit_scale(f, u, 0);
                    }
                }
            }
            "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT" => {
                let Value::List(a, n) = *f.args(r).first()? else {
                    continue;
                };
                for v in f.run(a, n) {
                    let Value::Ref(u) = *v else { continue };
                    let Some(m) = rec(f, u, "UNCERTAINTY_MEASURE_WITH_UNIT") else {
                        continue;
                    };
                    let b = f.args(&m);
                    let Some(k) = b.first().and_then(|x| number(f, x)) else {
                        continue;
                    };
                    let Some(Value::Ref(unit)) = b.get(1) else {
                        continue;
                    };
                    if let Some(s) = unit_scale(f, *unit, 0) {
                        unc = Some(unc.unwrap_or(f64::INFINITY).min(k * s));
                    }
                }
            }
            _ => {}
        }
    }
    Some(Units {
        mm: mm?,
        rad: rad.unwrap_or(1.0),
        uncertainty: unc.unwrap_or(1e-3),
    })
}

impl<'a> Reader<'a> {
    fn args_of(&self, id: u32, name: &str) -> R<&'a [Value]> {
        let f = self.f;
        match f
            .records_of(id)
            .and_then(|rs| rs.iter().find(|r| f.name(r) == name))
        {
            Some(r) => Ok(f.args(r)),
            None => refuse(id, name, "the entity is missing or of another kind"),
        }
    }

    fn real(&self, id: u32, kind: &str, v: Option<&Value>) -> R<f64> {
        match v.and_then(|v| number(self.f, v)) {
            Some(x) if x.is_finite() => Ok(x),
            _ => refuse(id, kind, "a number is missing"),
        }
    }

    fn reference(&self, id: u32, kind: &str, v: Option<&Value>) -> R<u32> {
        match v {
            Some(Value::Ref(r)) => Ok(*r),
            _ => refuse(id, kind, "a reference is missing"),
        }
    }

    fn list(&self, id: u32, kind: &str, v: Option<&Value>) -> R<&'a [Value]> {
        match v {
            Some(Value::List(a, n)) => Ok(self.f.run(*a, *n)),
            _ => refuse(id, kind, "a list is missing"),
        }
    }

    fn point(&self, id: u32) -> R<Point3> {
        let p = self.point_anywhere(id)?;
        if !forge_math::tol::in_model_box(p) {
            return refuse(id, "CARTESIAN_POINT", "outside the 1 km model box");
        }
        Ok(p)
    }

    /// A point, wherever it lies (a line's origin may be any point of it).
    fn point_anywhere(&self, id: u32) -> R<Point3> {
        let a = self.args_of(id, "CARTESIAN_POINT")?;
        let c = self.list(id, "CARTESIAN_POINT", a.get(1))?;
        if c.len() != 3 {
            return refuse(id, "CARTESIAN_POINT", format!("{} coordinates, not 3", c.len()));
        }
        let m = self.u.mm;
        let p = Point3::new(
            self.real(id, "CARTESIAN_POINT", c.first())? * m,
            self.real(id, "CARTESIAN_POINT", c.get(1))? * m,
            self.real(id, "CARTESIAN_POINT", c.get(2))? * m,
        );
        if !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite() {
            return refuse(id, "CARTESIAN_POINT", "a coordinate is not finite");
        }
        Ok(p)
    }

    fn direction(&self, id: u32) -> R<UnitVec3> {
        let a = self.args_of(id, "DIRECTION")?;
        let c = self.list(id, "DIRECTION", a.get(1))?;
        if c.len() != 3 {
            return refuse(id, "DIRECTION", "not three ratios");
        }
        let v = Vec3::new(
            self.real(id, "DIRECTION", c.first())?,
            self.real(id, "DIRECTION", c.get(1))?,
            self.real(id, "DIRECTION", c.get(2))?,
        );
        match v.normalize() {
            Some(u) => Ok(u),
            None => refuse(id, "DIRECTION", "zero length"),
        }
    }

    fn placement(&self, id: u32) -> R<Frame> {
        self.placement_sliding(id, |_, _| None)
    }

    /// A placement whose origin, when it lies outside the model box, `slide` may move to an
    /// equivalent point (given the far origin and the z axis; None: refused as it stands).
    fn placement_sliding(&self, id: u32, slide: impl Fn(Point3, UnitVec3) -> Option<Point3>) -> R<Frame> {
        let a = self.args_of(id, "AXIS2_PLACEMENT_3D")?;
        let pid = self.reference(id, "AXIS2_PLACEMENT_3D", a.get(1))?;
        let far = self.point_anywhere(pid)?;
        let z = match a.get(2) {
            Some(Value::Ref(r)) => Some(self.direction(*r)?),
            _ => None,
        };
        let x = match a.get(3) {
            Some(Value::Ref(r)) => Some(self.direction(*r)?),
            _ => None,
        };
        let z = z.unwrap_or(UnitVec3::Z);
        let origin = if forge_math::tol::in_model_box(far) {
            far
        } else {
            match slide(far, z).filter(|&o| forge_math::tol::in_model_box(o)) {
                Some(o) => o,
                None => return refuse(pid, "CARTESIAN_POINT", "outside the 1 km model box"),
            }
        };
        // the standard's default for a missing ref direction: world x, or y along it
        let hint: Vec3 = match x {
            Some(x) => x.into(),
            None => {
                if z.dot(Vec3::X).abs() > 0.99 {
                    Vec3::Y
                } else {
                    Vec3::X
                }
            }
        };
        Ok(Frame::from_zx(origin, z, hint).unwrap_or_else(|| Frame::from_z(origin, z)))
    }

    fn knots(&self, id: u32, kind: &str, mults: &Value, values: &Value) -> R<Vec<f64>> {
        let (m, k) = (
            self.list(id, kind, Some(mults))?,
            self.list(id, kind, Some(values))?,
        );
        if m.len() != k.len() {
            return refuse(id, kind, "knots and multiplicities differ in number");
        }
        let mut out = Vec::new();
        for (a, b) in m.iter().zip(k) {
            let Some(c) = number(self.f, a) else {
                return refuse(id, kind, "a bad multiplicity");
            };
            let x = self.real(id, kind, Some(b))?;
            if !(1.0..=64.0).contains(&c) {
                return refuse(id, kind, format!("multiplicity {c}"));
            }
            out.extend(std::iter::repeat_n(x, c as usize));
            if out.len() > nurbs::MAX_CTRL {
                return refuse(id, kind, "too many knots");
            }
        }
        Ok(out)
    }

    /// Knots a curve or surface without any states: clamped, even. For BEZIER_ and QUASI_UNIFORM_
    /// the standard fixes them; UNIFORM_ is unclamped and even.
    fn implied(&self, id: u32, kind: &str, deg: usize, n: usize) -> R<Vec<f64>> {
        let names = self
            .f
            .records_of(id)
            .map(|r| r.iter().map(|x| self.f.name(x)).collect::<Vec<_>>())
            .unwrap_or_default();
        let has = |s: &str| names.iter().any(|x| x.starts_with(s));
        if has("BEZIER_") {
            if n != deg + 1 {
                // pieces of `deg` joined end to end
                if !(n - 1).is_multiple_of(deg) {
                    return refuse(id, kind, "a Bezier with the wrong number of points");
                }
                let mut k = vec![0.0; deg + 1];
                let pieces = (n - 1) / deg;
                for i in 1..pieces {
                    k.extend(std::iter::repeat_n(i as f64, deg));
                }
                k.extend(std::iter::repeat_n(pieces as f64, deg + 1));
                return Ok(k);
            }
            let mut k = vec![0.0; deg + 1];
            k.extend(std::iter::repeat_n(1.0, deg + 1));
            return Ok(k);
        }
        if has("UNIFORM_") {
            return Ok((0..n + deg + 1).map(|i| i as f64 - deg as f64).collect());
        }
        if has("QUASI_UNIFORM_") {
            let mut k = vec![0.0; deg + 1];
            let spans = n - deg;
            for i in 1..spans {
                k.push(i as f64);
            }
            k.extend(std::iter::repeat_n(spans as f64, deg + 1));
            return Ok(k);
        }
        refuse(id, kind, "a B-spline with no knots")
    }

    fn weights(&self, id: u32, kind: &str, rec_name: &str) -> R<Option<&'a [Value]>> {
        match self
            .f
            .records_of(id)
            .and_then(|rs| rs.iter().find(|r| self.f.name(r) == rec_name))
        {
            Some(r) => Ok(Some(self.list(id, kind, self.f.args(r).first())?)),
            None => Ok(None),
        }
    }

    fn degree(&self, id: u32, kind: &str, v: Option<&Value>, max: usize) -> R<usize> {
        let d = self.real(id, kind, v)?;
        if !(1.0..=max as f64).contains(&d) {
            return refuse(id, kind, format!("degree {d}"));
        }
        Ok(d as usize)
    }

    fn bspline_curve(&self, id: u32) -> R<Curve> {
        let kind = "B_SPLINE_CURVE";
        let f = self.f;
        let rs = f.records_of(id).unwrap_or(&[]);
        let base = rs.iter().find(|r| {
            let n = f.name(r);
            n == "B_SPLINE_CURVE"
                || (rs.len() == 1 && n.starts_with(['B', 'U', 'Q']) && n.ends_with("_CURVE")
                    || n == "B_SPLINE_CURVE_WITH_KNOTS")
        });
        let Some(base) = base else {
            return refuse(id, kind, "no B-spline record");
        };
        let a = f.args(base);
        // simple form leads with its name; a complex form's B_SPLINE_CURVE has none
        let at = usize::from(matches!(a.first(), Some(Value::Str(..))));
        let deg = self.degree(id, kind, a.get(at), forge_geom::spline::MAX_DEGREE)?;
        let ctrl = self.list(id, kind, a.get(at + 1))?;
        if ctrl.len() > nurbs::MAX_CTRL {
            return refuse(id, kind, "too many control points");
        }
        let mut pts = Vec::with_capacity(ctrl.len());
        for c in ctrl {
            pts.push(self.point(self.reference(id, kind, Some(c))?)?);
        }
        let knots = match rs.iter().find(|r| f.name(r) == "B_SPLINE_CURVE_WITH_KNOTS") {
            Some(r) => {
                let k = f.args(r);
                // simple: name, degree, points, form, closed, intersect, mults, knots, spec
                let (m, v) = if k.len() >= 8 {
                    (k.get(k.len() - 3), k.get(k.len() - 2))
                } else {
                    (k.first(), k.get(1))
                };
                match (m, v) {
                    (Some(m), Some(v)) => self.knots(id, kind, m, v)?,
                    _ => return refuse(id, kind, "no knots"),
                }
            }
            None => self.implied(id, kind, deg, pts.len())?,
        };
        let weights = match self.weights(id, kind, "RATIONAL_B_SPLINE_CURVE")? {
            Some(w) => {
                let mut out = Vec::with_capacity(w.len());
                for x in w {
                    out.push(self.real(id, kind, Some(x))?);
                }
                if out.iter().any(|&x| x <= 0.0) {
                    return refuse(id, kind, "a weight not positive");
                }
                Some(out)
            }
            None => None,
        };
        let Some(s) = Spline::from_nurbs(deg, &knots, &pts, weights.as_deref()) else {
            return refuse(id, kind, "knots, points and degree do not make a curve");
        };
        let dom = s.domain();
        Ok(SplineCurve::curve(std::sync::Arc::new(s), dom))
    }

    fn curve(&mut self, id: u32) -> R<Curve> {
        self.depth += 1;
        if self.depth > MAX_WRAP {
            self.depth -= 1;
            return refuse(id, "curve", "curves wrapped too deep, or in a circle");
        }
        let c = self.curve_inner(id);
        self.depth -= 1;
        c
    }

    fn curve_inner(&mut self, id: u32) -> R<Curve> {
        let f = self.f;
        let Some(name) = kw(f, id) else {
            return refuse(id, "curve", "the entity is missing");
        };
        let m = self.u.mm;
        let is_spline = f.records_of(id).is_some_and(|rs| {
            rs.iter().any(|r| {
                let n = f.name(r);
                n == "B_SPLINE_CURVE"
                    || n == "B_SPLINE_CURVE_WITH_KNOTS"
                    || n == "BEZIER_CURVE"
                    || n == "UNIFORM_CURVE"
                    || n == "QUASI_UNIFORM_CURVE"
            })
        });
        if is_spline {
            return self.bspline_curve(id);
        }
        match name {
            "LINE" => {
                let a = self.args_of(id, "LINE")?;
                let pid = self.reference(id, "LINE", a.get(1))?;
                let far = self.point_anywhere(pid)?;
                let v = self.reference(id, "LINE", a.get(2))?;
                let va = self.args_of(v, "VECTOR")?;
                let dir = self.direction(self.reference(v, "VECTOR", va.get(1))?)?;
                // a line's origin is any point of it: one far off (SolidWorks writes them 500 m
                // out) is moved along it to the point nearest the model's origin
                let origin = if forge_math::tol::in_model_box(far) {
                    far
                } else {
                    let o = far - dir.get() * (far - Point3::ORIGIN).dot(dir.get());
                    if !forge_math::tol::in_model_box(o) {
                        return refuse(pid, "CARTESIAN_POINT", "outside the 1 km model box");
                    }
                    o
                };
                Ok(Curve::Line(Line { origin, dir }))
            }
            "CIRCLE" => {
                let a = self.args_of(id, "CIRCLE")?;
                let frame = self.placement(self.reference(id, "CIRCLE", a.get(1))?)?;
                let radius = self.real(id, "CIRCLE", a.get(2))? * m;
                if !positive(radius) {
                    return refuse(id, "CIRCLE", "radius not positive");
                }
                Ok(Curve::Circle(Circle { frame, radius }))
            }
            "ELLIPSE" => {
                let a = self.args_of(id, "ELLIPSE")?;
                let frame = self.placement(self.reference(id, "ELLIPSE", a.get(1))?)?;
                let (major, minor) = (
                    self.real(id, "ELLIPSE", a.get(2))? * m,
                    self.real(id, "ELLIPSE", a.get(3))? * m,
                );
                if !(positive(major) && positive(minor)) {
                    return refuse(id, "ELLIPSE", "a semi-axis not positive");
                }
                Ok(Curve::Ellipse(Ellipse::new(frame, major, minor)))
            }
            "PARABOLA" => {
                let a = self.args_of(id, "PARABOLA")?;
                let frame = self.placement(self.reference(id, "PARABOLA", a.get(1))?)?;
                let focal = self.real(id, "PARABOLA", a.get(2))? * m;
                if focal == 0.0 || !focal.is_finite() {
                    return refuse(id, "PARABOLA", "no finite focal length");
                }
                Ok(Curve::Parabola(Parabola::new(frame, focal)))
            }
            "HYPERBOLA" => {
                let a = self.args_of(id, "HYPERBOLA")?;
                let frame = self.placement(self.reference(id, "HYPERBOLA", a.get(1))?)?;
                let (x, y) = (
                    self.real(id, "HYPERBOLA", a.get(2))? * m,
                    self.real(id, "HYPERBOLA", a.get(3))? * m,
                );
                if !(positive(x) && positive(y)) {
                    return refuse(id, "HYPERBOLA", "a semi-axis not positive");
                }
                Ok(Curve::Hyperbola(Hyperbola { frame, a: x, b: y }))
            }
            "SURFACE_CURVE" | "SEAM_CURVE" | "BOUNDED_SURFACE_CURVE" | "INTERSECTION_CURVE" => {
                let a = self.args_of(id, name)?;
                let inner = self.reference(id, name, a.get(1))?;
                self.curve(inner)
            }
            "TRIMMED_CURVE" => {
                // the edge's vertices bound it; only the curve matters
                let a = self.args_of(id, name)?;
                let inner = self.reference(id, name, a.get(1))?;
                self.curve(inner)
            }
            other => refuse(id, other, "this curve is not read yet"),
        }
    }

    fn surface(&mut self, id: u32) -> R<Surface> {
        self.depth += 1;
        if self.depth > MAX_WRAP {
            self.depth -= 1;
            return refuse(id, "surface", "surfaces wrapped too deep, or in a circle");
        }
        let s = self.surface_inner(id);
        self.depth -= 1;
        s
    }

    fn surface_inner(&mut self, id: u32) -> R<Surface> {
        let f = self.f;
        let Some(name) = kw(f, id) else {
            return refuse(id, "surface", "the entity is missing");
        };
        let m = self.u.mm;
        let is_spline = f.records_of(id).is_some_and(|rs| {
            rs.iter().any(|r| {
                let n = f.name(r);
                n == "B_SPLINE_SURFACE"
                    || n == "B_SPLINE_SURFACE_WITH_KNOTS"
                    || n == "BEZIER_SURFACE"
                    || n == "UNIFORM_SURFACE"
                    || n == "QUASI_UNIFORM_SURFACE"
            })
        });
        if is_spline {
            return self.bspline_surface(id);
        }
        match name {
            // a plane's or cylinder's origin far off (SolidWorks writes them 500 m out) moved
            // within the plane, or along the axis, to the point nearest the model's origin
            "PLANE" => {
                let a = self.args_of(id, name)?;
                Ok(Surface::Plane(Plane {
                    frame: self.placement_sliding(self.reference(id, name, a.get(1))?, |o, z| {
                        Some(Point3::ORIGIN + z.get() * (o - Point3::ORIGIN).dot(z.get()))
                    })?,
                }))
            }
            "CYLINDRICAL_SURFACE" => {
                let a = self.args_of(id, name)?;
                let frame = self.placement_sliding(self.reference(id, name, a.get(1))?, |o, z| {
                    Some(o - z.get() * (o - Point3::ORIGIN).dot(z.get()))
                })?;
                let radius = self.real(id, name, a.get(2))? * m;
                if !positive(radius) {
                    return refuse(id, name, "radius not positive");
                }
                Ok(Surface::Cylinder(Cylinder { frame, radius }))
            }
            "CONICAL_SURFACE" => {
                let a = self.args_of(id, name)?;
                let frame = self.placement(self.reference(id, name, a.get(1))?)?;
                let radius = self.real(id, name, a.get(2))? * m;
                let half = self.real(id, name, a.get(3))? * self.u.rad;
                match Cone::new(frame, radius, half) {
                    Some(c) => Ok(Surface::Cone(c)),
                    // a half angle within a millionth of a radian of a right angle (some files write a flat
                    // ring so, ABC 00012071: 1.5707956859) is the plane square to the axis through its
                    // circle, to within that angle times the face's reach; its edges are measured against
                    // it as any edge is. Such a cone's normal is against its axis
                    None if radius >= 0.0 && (half - dmath::FRAC_PI_2).abs() <= 1e-6 => {
                        Ok(Surface::Plane(Plane {
                            frame: frame.flipped(),
                        }))
                    }
                    None => refuse(id, name, format!("radius {radius}, half angle {half}")),
                }
            }
            "SPHERICAL_SURFACE" => {
                let a = self.args_of(id, name)?;
                let frame = self.placement(self.reference(id, name, a.get(1))?)?;
                let radius = self.real(id, name, a.get(2))? * m;
                if !positive(radius) {
                    return refuse(id, name, "radius not positive");
                }
                Ok(Surface::Sphere(Sphere { frame, radius }))
            }
            "TOROIDAL_SURFACE" | "DEGENERATE_TOROIDAL_SURFACE" => {
                let a = self.args_of(id, name)?;
                let frame = self.placement(self.reference(id, name, a.get(1))?)?;
                let (major, minor) = (
                    self.real(id, name, a.get(2))? * m,
                    self.real(id, name, a.get(3))? * m,
                );
                if name == "TOROIDAL_SURFACE"
                    && let Some(t) = Torus::new(frame, major, minor)
                {
                    return Ok(Surface::Torus(t));
                }
                // a spindle torus: its outer part, or the inner part across the axis, as a revolve makes
                // them; a circle turned about the axis only for radii too near to be one
                let outer = a.get(4).and_then(|v| logical(f, v)).unwrap_or(true);
                if let Some(t) = Torus::spindle(frame, if outer { major } else { -major }, minor) {
                    return Ok(Surface::Torus(t));
                }
                self.spindle(id, name, frame, major, minor, outer)
            }
            "SURFACE_OF_LINEAR_EXTRUSION" => self.extrusion(id),
            "SURFACE_OF_REVOLUTION" => self.revolution(id),
            "RECTANGULAR_TRIMMED_SURFACE" => {
                let a = self.args_of(id, name)?;
                let inner = self.reference(id, name, a.get(1))?;
                self.surface(inner)
            }
            other => refuse(id, other, "this surface is not read yet"),
        }
    }

    fn bspline_surface(&self, id: u32) -> R<Surface> {
        let kind = "B_SPLINE_SURFACE";
        let f = self.f;
        let rs = f.records_of(id).unwrap_or(&[]);
        let Some(base) = rs.iter().find(|r| {
            let n = f.name(r);
            n == "B_SPLINE_SURFACE"
                || (rs.len() == 1 && n.ends_with("_SURFACE"))
                || n == "B_SPLINE_SURFACE_WITH_KNOTS"
        }) else {
            return refuse(id, kind, "no B-spline record");
        };
        let a = f.args(base);
        let at = usize::from(matches!(a.first(), Some(Value::Str(..))));
        let p = self.degree(id, kind, a.get(at), nurbs::MAX_DEGREE)?;
        let q = self.degree(id, kind, a.get(at + 1), nurbs::MAX_DEGREE)?;
        let rows = self.list(id, kind, a.get(at + 2))?;
        let nu = rows.len();
        if nu == 0 {
            return refuse(id, kind, "no control points");
        }
        let mut pts = Vec::new();
        let mut nv = 0;
        for r in rows {
            let row = self.list(id, kind, Some(r))?;
            if nv != 0 && row.len() != nv {
                return refuse(id, kind, "rows of control points differ in length");
            }
            nv = row.len();
            if pts.len() + nv > nurbs::MAX_CTRL {
                return refuse(id, kind, "too many control points");
            }
            for c in row {
                pts.push(self.point(self.reference(id, kind, Some(c))?)?);
            }
        }
        let (u, v) = match rs.iter().find(|r| f.name(r) == "B_SPLINE_SURFACE_WITH_KNOTS") {
            Some(r) => {
                let k = f.args(r);
                // simple: .. mults u, mults v, knots u, knots v, spec; complex: those alone
                let n = k.len();
                if n >= 5 {
                    let (mu, mv, ku, kv) = (&k[n - 5], &k[n - 4], &k[n - 3], &k[n - 2]);
                    (self.knots(id, kind, mu, ku)?, self.knots(id, kind, mv, kv)?)
                } else {
                    return refuse(id, kind, "no knots");
                }
            }
            None => (self.implied(id, kind, p, nu)?, self.implied(id, kind, q, nv)?),
        };
        let weights = match rs.iter().find(|r| f.name(r) == "RATIONAL_B_SPLINE_SURFACE") {
            Some(r) => {
                let w = self.list(id, kind, f.args(r).first())?;
                let mut out = Vec::with_capacity(pts.len());
                for row in w {
                    for x in self.list(id, kind, Some(row))? {
                        out.push(self.real(id, kind, Some(x))?);
                    }
                }
                Some(out)
            }
            None => None,
        };
        match Nurbs::new((p, q), (u, v), (nu, nv), &pts, weights.as_deref()) {
            Some(n) => Ok(n.surface()),
            None => refuse(id, kind, "knots, points and weights do not make a surface"),
        }
    }

    /// A curve's control net, or the error, for surfaces built by moving it.
    fn generator(&mut self, id: u32, kind: &str, curve: u32) -> R<Net> {
        let c = self.curve(curve)?;
        match net_of_curve(&c) {
            Some(n) => Ok(n),
            None => refuse(id, kind, "its curve cannot be moved exactly"),
        }
    }

    /// Linear in v, over a range past any part.
    fn extrusion(&mut self, id: u32) -> R<Surface> {
        let name = "SURFACE_OF_LINEAR_EXTRUSION";
        let a = self.args_of(id, name)?;
        let basis = self.reference(id, name, a.get(1))?;
        let v = self.reference(id, name, a.get(2))?;
        let va = self.args_of(v, "VECTOR")?;
        let dir: Vec3 = self.direction(self.reference(v, "VECTOR", va.get(1))?)?.into();
        let net = self.generator(id, name, basis)?;
        match extruded(net, dir) {
            Some(n) => Ok(n),
            None => refuse(id, name, "no surface from its curve"),
        }
    }

    fn revolution(&mut self, id: u32) -> R<Surface> {
        let name = "SURFACE_OF_REVOLUTION";
        let a = self.args_of(id, name)?;
        let basis = self.reference(id, name, a.get(1))?;
        let ax = self.reference(id, name, a.get(2))?;
        let aa = self.args_of(ax, "AXIS1_PLACEMENT")?;
        let o = self.point(self.reference(ax, "AXIS1_PLACEMENT", aa.get(1))?)?;
        let z: Vec3 = match aa.get(2) {
            Some(Value::Ref(r)) => self.direction(*r)?.into(),
            _ => Vec3::Z,
        };
        let g = self.generator(id, name, basis)?;
        match turned((o, z), g, self.u.uncertainty) {
            Some(n) => Ok(n),
            None => refuse(id, name, "no surface from its curve"),
        }
    }

    /// The spindle torus's outer or inner arc of its tube turned about the axis.
    fn spindle(&self, id: u32, name: &str, frame: Frame, major: f64, minor: f64, outer: bool) -> R<Surface> {
        if !positive(minor) {
            return refuse(id, name, "minor radius not positive");
        }
        if major > minor + forge_math::tol::MIN_LENGTH {
            return refuse(
                id,
                name,
                format!("radii {major} and {minor}: a ring torus with too thin a tube"),
            );
        }
        // the tube's circle meets the axis at tube angle +-half from the side away from it
        let h = (minor * minor - major * major).max(0.0).sqrt();
        let half = dmath::atan2(h, -major);
        let (x, z): (Vec3, Vec3) = (frame.x().into(), frame.z().into());
        let c = frame.origin + x * major;
        let s = if outer {
            Spline::ellipse(c, x * minor, z * minor, -half, 2.0 * half)
        } else {
            Spline::ellipse(c, x * minor, z * minor, half, 2.0 * (dmath::PI - half))
        };
        match s {
            Some(s) => match turned((frame.origin, z), net_of(&s), self.u.uncertainty) {
                Some(n) => Ok(n),
                None => refuse(id, name, "no surface from its curve"),
            },
            None => refuse(id, name, "no arc for its tube"),
        }
    }
}

/// A curve's control net, for surfaces built by moving it. Conics become rational splines (exact)
/// through `Spline::ellipse`; a line, two points.
pub(crate) fn net_of_curve(c: &Curve) -> Option<Net> {
    match c {
        Curve::Line(l) => {
            // long enough for any part in the model box
            let reach = 2.0 * forge_math::tol::MODEL_HALF_SIZE;
            let (a, b) = (
                l.origin + Vec3::from(l.dir) * -reach,
                l.origin + Vec3::from(l.dir) * reach,
            );
            Some((1, vec![-reach, -reach, reach, reach], vec![a, b], vec![1.0, 1.0]))
        }
        Curve::Circle(k) => {
            let (x, y) = (
                Vec3::from(k.frame.x()) * k.radius,
                Vec3::from(k.frame.y()) * k.radius,
            );
            Some(net_of(&Spline::ellipse(k.frame.origin, x, y, 0.0, dmath::TAU)?))
        }
        Curve::Ellipse(k) => {
            let (x, y) = (
                Vec3::from(k.frame.x()) * k.major,
                Vec3::from(k.frame.y()) * k.minor,
            );
            Some(net_of(&Spline::ellipse(k.frame.origin, x, y, 0.0, dmath::TAU)?))
        }
        other => Some(net_of(&forge_geom::spline::spline_of(other)?.c)),
    }
}

/// A curve's net swept linearly, in v over a range past any part.
pub(crate) fn extruded((deg, knots, pts, w): Net, dir: Vec3) -> Option<Surface> {
    let reach = 2.0 * forge_math::tol::MODEL_HALF_SIZE;
    let mut net = Vec::with_capacity(pts.len() * 2);
    let mut ws = Vec::with_capacity(pts.len() * 2);
    for (i, p) in pts.iter().enumerate() {
        net.push(*p + dir * -reach);
        net.push(*p + dir * reach);
        ws.push(w[i]);
        ws.push(w[i]);
    }
    let n = Nurbs::new(
        (deg, 1),
        (knots, vec![-reach, -reach, reach, reach]),
        (pts.len(), 2),
        &net,
        Some(&ws),
    )?;
    Some(n.surface())
}

/// A curve's net turned about an axis by rational quadratic quarter-turns. A generator ending
/// within `on_axis` of the axis ends on it: a pole.
pub(crate) fn turned((o, z): (Point3, Vec3), (deg, knots, pts, w): Net, on_axis: f64) -> Option<Surface> {
    // nine points round: four quarters, weights 1 and sqrt(1/2)
    let s = 0.5_f64.sqrt();
    let mut net = Vec::with_capacity(pts.len() * 9);
    let mut ws = Vec::with_capacity(pts.len() * 9);
    // as the standard has it: u the angle, v along the curve (the natural normal depends on it)
    for k in 0..9 {
        let (ca, cb, wt) = match k {
            0 | 8 => (1.0, 0.0, 1.0),
            1 => (1.0, 1.0, s),
            2 => (0.0, 1.0, 1.0),
            3 => (-1.0, 1.0, s),
            4 => (-1.0, 0.0, 1.0),
            5 => (-1.0, -1.0, s),
            6 => (0.0, -1.0, 1.0),
            _ => (1.0, -1.0, s),
        };
        for (i, p) in pts.iter().enumerate() {
            let c = o + z * z.dot(*p - o);
            let r = *p - c;
            let r = if r.length() <= on_axis { Vec3::ZERO } else { r };
            net.push(c + r * ca + z.cross(r) * cb);
            ws.push(w[i] * wt);
        }
    }
    let pi = dmath::PI;
    let angle = vec![
        0.0,
        0.0,
        0.0,
        0.5 * pi,
        0.5 * pi,
        pi,
        pi,
        1.5 * pi,
        1.5 * pi,
        2.0 * pi,
        2.0 * pi,
        2.0 * pi,
    ];
    Some(Nurbs::new((2, deg), (angle, knots), (9, pts.len()), &net, Some(&ws))?.surface())
}

/// A spline's Bezier pieces as a clamped B-spline's knots, points and weights.
pub(crate) fn net_of(s: &Spline) -> Net {
    let (deg, breaks, h) = s.bezier_data();
    let mut knots = vec![breaks[0]; deg + 1];
    for b in &breaks[1..breaks.len() - 1] {
        knots.extend(std::iter::repeat_n(*b, deg));
    }
    knots.extend(std::iter::repeat_n(breaks[breaks.len() - 1], deg + 1));
    let (mut pts, mut w) = (Vec::new(), Vec::new());
    for (i, c) in h.iter().enumerate() {
        // pieces share their end points
        if i > 0 && i % (deg + 1) == 0 {
            continue;
        }
        // (each point its own, not times its weight: `bezier_data`)
        pts.push(Point3::new(c[0], c[1], c[2]));
        w.push(c[3]);
    }
    (deg, knots, pts, w)
}

struct Build<'a, 'b> {
    r: &'b mut Reader<'a>,
    p: Proto,
    verts: BTreeMap<u32, usize>,
    /// Each EDGE_CURVE's edge and whether the edge runs against the entity's direction.
    edges: BTreeMap<u32, (usize, bool)>,
    /// The edges that end at each vertex reference, built when a reference must be recovered.
    ends: Option<BTreeMap<u32, Vec<u32>>>,
    stats: &'b mut Stats,
    salt: Name,
}

impl Build<'_, '_> {
    fn vertex(&mut self, id: u32) -> R<usize> {
        if let Some(&v) = self.verts.get(&id) {
            return Ok(v);
        }
        let p = match self.r.args_of(id, "VERTEX_POINT") {
            Ok(a) => self.r.point(self.r.reference(id, "VERTEX_POINT", a.get(1))?)?,
            Err(e) => self.vertex_from_curves(id).ok_or(e)?,
        };
        self.p.verts.push((p, self.salt.with(u64::from(id))));
        let v = self.p.verts.len() - 1;
        self.verts.insert(id, v);
        Ok(v)
    }

    /// Where a vertex must be when the file's reference to it names something else (SolidWorks
    /// 2021 has written an ORIENTED_EDGE's id in a vertex's place): the point the curves of the
    /// edges that end there share, found by projecting onto each in turn, if they meet to within
    /// 1e-4 of their size and 0.01 mm.
    fn vertex_from_curves(&mut self, id: u32) -> Option<Point3> {
        let f = self.r.f;
        let ends = self.ends.get_or_insert_with(|| {
            let mut m: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
            for &(eid, at, n) in &f.ids {
                for r in &f.records[at as usize..(at + n) as usize] {
                    if f.name(r) == "EDGE_CURVE" {
                        let a = f.args(r);
                        for v in [a.get(1), a.get(2)] {
                            if let Some(Value::Ref(x)) = v {
                                m.entry(*x).or_default().push(eid);
                            }
                        }
                    }
                }
            }
            m
        });
        let edges = ends.get(&id)?.clone();
        let (mut curves, mut others) = (Vec::new(), Vec::new());
        for e in edges {
            let a = self.r.args_of(e, "EDGE_CURVE").ok()?;
            let Some(Value::Ref(c)) = a.get(3) else {
                return None;
            };
            curves.push(self.r.curve(*c).ok()?);
            for v in [a.get(1), a.get(2)] {
                if let Some(Value::Ref(x)) = v
                    && *x != id
                    && let Ok(va) = self.r.args_of(*x, "VERTEX_POINT")
                    && let Some(Value::Ref(pp)) = va.get(1)
                    && let Ok(p) = self.r.point(*pp)
                {
                    others.push(p);
                }
            }
        }
        if curves.len() < 2 || others.is_empty() {
            return None;
        }
        let range = |c: &Curve| {
            let d = c.domain();
            Interval::new(d.lo.max(-1e7), d.hi.min(1e7))
        };
        let miss = |p: Point3| {
            curves
                .iter()
                .map(|c| c.eval(c.closest(p, range(c))).distance(p))
                .fold(0.0, f64::max)
        };
        // starts: the edges' other ends, their middle, and where two of the lines come nearest
        let n = others.len() as f64;
        let mut starts = others.clone();
        starts.push(others.iter().fold(Point3::ORIGIN, |s, q| s + q.to_vec() / n));
        let lines: Vec<&Line> = curves
            .iter()
            .filter_map(|c| if let Curve::Line(l) = c { Some(l) } else { None })
            .collect();
        for (i, a) in lines.iter().enumerate() {
            for b in &lines[i + 1..] {
                let (u, v, w) = (a.dir.get(), b.dir.get(), a.origin - b.origin);
                let (uv, uw, vw) = (u.dot(v), u.dot(w), v.dot(w));
                let den = 1.0 - uv * uv;
                if den > 1e-6 {
                    let s = (uv * vw - uw) / den;
                    let t = (vw - uv * uw) / den;
                    starts.push(
                        Point3::ORIGIN + ((a.origin + u * s).to_vec() + (b.origin + v * t).to_vec()) / 2.0,
                    );
                }
            }
        }
        // and around each closed curve, and on each line nearest the others
        for c in &curves {
            let d = c.domain();
            if d.period.is_some() {
                starts.extend((0..8).map(|k| c.eval(d.lo + (d.hi - d.lo) * k as f64 / 8.0)));
            }
        }
        let anchors: Vec<Point3> = starts.clone();
        for l in &lines {
            for q in &anchors {
                starts.push(l.origin + l.dir.get() * (*q - l.origin).dot(l.dir.get()));
            }
        }
        let mut best: Option<(f64, Point3)> = None;
        for mut p in starts {
            for _ in 0..2000 {
                let mut s = Vec3::ZERO;
                for c in &curves {
                    s += c.eval(c.closest(p, range(c))).to_vec();
                }
                let q = Point3::ORIGIN + s / curves.len() as f64;
                let moved = q.distance(p);
                p = q;
                if moved < 1e-13 {
                    break;
                }
            }
            let m = miss(p);
            if best.is_none_or(|(b, _)| m < b) {
                best = Some((m, p));
            }
        }
        let (m, p) = best?;
        let size = others.iter().map(|q| q.distance(p)).fold(0.0, f64::max);
        (m <= (1e-4 * size).clamp(forge_math::tol::LINEAR, 0.01) && forge_math::tol::in_model_box(p))
            .then_some(p)
    }

    fn edge(&mut self, id: u32) -> R<(usize, bool)> {
        if let Some(&e) = self.edges.get(&id) {
            return Ok(e);
        }
        let k = "EDGE_CURVE";
        let a = self.r.args_of(id, k)?;
        let (v0, v1) = (
            self.r.reference(id, k, a.get(1))?,
            self.r.reference(id, k, a.get(2))?,
        );
        let curve_id = self.r.reference(id, k, a.get(3))?;
        let Some(same) = a.get(4).and_then(|v| logical(self.r.f, v)) else {
            return refuse(id, k, "same_sense is missing");
        };
        let (start, end) = (self.vertex(v0)?, self.vertex(v1)?);
        let curve = self.r.curve(curve_id)?;
        // the edge runs the way its curve's parameter increases
        let (s, t, flipped) = if same {
            (start, end, false)
        } else {
            (end, start, true)
        };
        let (range, off, against) = edge_range(&curve, self.p.verts[s].0, self.p.verts[t].0, s == t);
        // a file's same_sense can be wrong: the vertices say which way the edge really runs
        let (s, t, flipped) = if against {
            (t, s, !flipped)
        } else {
            (s, t, flipped)
        };
        self.stats.vertex_off_curve = self.stats.vertex_off_curve.max(off);
        if off > forge_math::tol::LINEAR {
            self.stats.vertices_off += 1;
        }
        self.p.edges.push(PEdge {
            curve,
            range,
            start: s,
            end: t,
            name: self.salt.with(u64::from(id)),
        });
        let e = self.p.edges.len() - 1;
        self.stats.edges += 1;
        self.edges.insert(id, (e, flipped));
        Ok((e, flipped))
    }

    /// One ADVANCED_FACE, `flip` when its shell is used reversed.
    fn face(&mut self, id: u32, shell: usize, flip: bool) -> R<()> {
        let f = self.r.f;
        let k = "ADVANCED_FACE";
        let a = self.r.args_of(id, k)?;
        let bounds = self.r.list(id, k, a.get(1))?;
        let surf = self.r.reference(id, k, a.get(2))?;
        let Some(same) = a.get(3).and_then(|v| logical(f, v)) else {
            return refuse(id, k, "same_sense is missing");
        };
        let surface = self.r.surface(surf)?;
        let name = self.salt.with(u64::from(id));
        self.stats.faces += 1;
        let mut loops = Vec::with_capacity(bounds.len());
        for b in bounds {
            let bid = self.r.reference(id, k, Some(b))?;
            let bk = kw(f, bid).unwrap_or("");
            if bk != "FACE_BOUND" && bk != "FACE_OUTER_BOUND" {
                return refuse(bid, bk, "not a face bound");
            }
            let ba = self.r.args_of(bid, bk)?;
            let lp = self.r.reference(bid, bk, ba.get(1))?;
            let Some(ori) = ba.get(2).and_then(|v| logical(f, v)) else {
                return refuse(bid, bk, "orientation is missing");
            };
            match kw(f, lp) {
                Some("EDGE_LOOP") => {}
                // a cone's apex or a sphere's pole: seams reach it in healing
                Some("VERTEX_LOOP") => continue,
                other => return refuse(lp, other.unwrap_or("?"), "this loop is not read yet"),
            }
            let la = self.r.args_of(lp, "EDGE_LOOP")?;
            let uses = self.r.list(lp, "EDGE_LOOP", la.get(1))?;
            let mut out: Vec<(usize, bool)> = Vec::with_capacity(uses.len());
            for u in uses {
                let uid = self.r.reference(lp, "EDGE_LOOP", Some(u))?;
                let oa = self.r.args_of(uid, "ORIENTED_EDGE")?;
                let ec = self.r.reference(uid, "ORIENTED_EDGE", oa.get(3))?;
                let Some(fw) = oa.get(4).and_then(|v| logical(f, v)) else {
                    return refuse(uid, "ORIENTED_EDGE", "orientation is missing");
                };
                let (e, flipped) = self.edge(ec)?;
                out.push((e, fw != flipped));
            }
            // the loop must have the face on its left, seen against the face's outward normal
            if ori == flip {
                out.reverse();
                for u in &mut out {
                    u.1 = !u.1;
                }
            }
            if out.is_empty() {
                return refuse(lp, "EDGE_LOOP", "no edges");
            }
            loops.push(out);
        }

        self.p.faces.push(PFace {
            surface,
            same_sense: same != flip,
            name,
            shell,
            loops,
        });
        self.p.face_ids.push(id);
        Ok(())
    }

    fn shell(&mut self, id: u32, flip: bool) -> R<()> {
        let k = kw(self.r.f, id).unwrap_or("");
        let (id, flip) = if k == "ORIENTED_CLOSED_SHELL" {
            let a = self.r.args_of(id, k)?;
            let inner = self.r.reference(id, k, a.get(2))?;
            let Some(o) = a.get(3).and_then(|v| logical(self.r.f, v)) else {
                return refuse(id, k, "orientation is missing");
            };
            (inner, flip != !o)
        } else {
            (id, flip)
        };
        let kind = if kw(self.r.f, id) == Some("OPEN_SHELL") {
            "OPEN_SHELL"
        } else {
            "CLOSED_SHELL"
        };
        let a = self.r.args_of(id, kind)?;
        let faces = self.r.list(id, kind, a.get(1))?;
        let shell = self.p.shells;
        self.p.shells += 1;
        for x in faces {
            let fid = self.r.reference(id, "CLOSED_SHELL", Some(x))?;
            self.face(fid, shell, flip)?;
        }
        Ok(())
    }
}

/// The edge's parameter range from its end points on its curve, how far they lie from it, and
/// whether they come the other way along it than the file says.
pub(crate) fn edge_range(c: &Curve, a: Point3, b: Point3, closed: bool) -> (Interval, f64, bool) {
    let d = c.domain();
    match d.period {
        Some(p) => {
            let t0 = c.closest(a, Interval::new(d.lo, d.hi));
            let off0 = c.eval(t0).distance(a);
            if closed {
                return (Interval::new(t0, t0 + p), off0, false);
            }
            let mut t1 = c.closest(b, Interval::new(d.lo, d.hi));
            let off1 = c.eval(t1).distance(b);
            while t1 <= t0 {
                t1 += p;
            }
            (Interval::new(t0, t1), off0.max(off1), false)
        }
        None => {
            if d.lo.is_finite() {
                let whole = Interval::new(d.lo, d.hi);
                // the curve's ends first: a curve may pass its end point elsewhere
                let (e0, e1) = (c.eval(d.lo), c.eval(d.hi));
                let near = |x: Point3, y: Point3| x.distance(y) <= 1e-9;
                if closed {
                    return (whole, a.distance(e0), false);
                }
                // on a curve closed in fact (a closed B-spline), its closing point is both ends: the
                // edge starts there at lo and ends there at hi, as it runs the way the parameter grows
                let shut = e0.distance(e1) <= forge_math::tol::LINEAR;
                let at_seam =
                    |x: Point3| shut && x.distance(e0).min(x.distance(e1)) <= forge_math::tol::LINEAR;
                let t = |x: Point3, end: bool| {
                    if at_seam(x) {
                        if end { d.hi } else { d.lo }
                    } else if near(x, e0) {
                        d.lo
                    } else if near(x, e1) {
                        d.hi
                    } else {
                        c.closest(x, whole)
                    }
                };
                let (t0, t1) = (t(a, false), t(b, true));
                let off = c.eval(t0).distance(a).max(c.eval(t1).distance(b));
                return (Interval::new(t0.min(t1), t0.max(t1)), off, t1 < t0);
            }
            let far = 4.0 * forge_math::tol::MODEL_HALF_SIZE;
            let whole = Interval::new(-far, far);
            let (t0, t1) = (c.closest(a, whole), c.closest(b, whole));
            let off = c.eval(t0).distance(a).max(c.eval(t1).distance(b));
            (Interval::new(t0.min(t1), t0.max(t1)), off, t1 < t0)
        }
    }
}

/// Each edge's measured distance from its faces, where that is more than the kernel's tolerance:
/// at the points `check` and `certify` will ask about, so they read what was measured here.
fn record_deviations(body: &mut Body, stats: &mut Stats) {
    use forge_math::tol::LINEAR;
    let ids: Vec<forge_topo::EdgeId> = body.edge_ids().collect();
    // each edge's measure reads no edge's tolerance and tests no budget: all of them on all cores,
    // what they spend together charged (`budget::map_counted`), then the tolerances set in edge order
    let devs = {
        let body = &*body;
        forge_geom::budget::map_counted(ids.len(), |k| edge_deviation(body, ids[k]))
    };
    for (e, dev) in ids.into_iter().zip(devs) {
        if dev > LINEAR {
            // with room over what was measured: `check` measures again, after any rigid move and,
            // once a boolean has trimmed the edge, at other points, where a gap that peaks between
            // these samples comes out larger (by 1.7e-5 of it on a spline edge of GM2-5AX, by
            // 1.3e-3 on a piece of TRT210's cylinder-cylinder intersection a boolean split off): a
            // tolerance of exactly the gap fails it. A hundredth of the gap is room for both
            let dev = dev * (1.0 + 1e-2) + 1e-9;
            body.edges[e.index()].tol = dev;
            stats.tolerant_edges += 1;
            stats.max_edge_tol = stats.max_edge_tol.max(dev);
        }
    }
}

/// How far an edge's points lie off its faces: 16 points, or 256 where its curve or a face is
/// free-form or the 16 find it off by more than LINEAR.
fn edge_deviation(body: &Body, e: forge_topo::EdgeId) -> f64 {
    use forge_math::tol::LINEAR;
    let faces: Vec<_> = body
        .edge(e)
        .coedges
        .iter()
        .map(|&c| body.coedge_face(c))
        .collect();
    let free_form = matches!(body.edge(e).curve, Curve::Procedural(_) | Curve::Intersection(_))
        || faces
            .iter()
            .any(|&f| matches!(body.face(f).surface, Surface::Procedural(_)));
    let measure = |n: usize, refine: bool| {
        let x = body.edge(e);
        let at = |i: usize| x.range.lo + x.range.width() * (i as f64 / n as f64);
        let pts = body.edge_points(e, n);
        faces
            .iter()
            .map(|&f| {
                let s = &body.face(f).surface;
                // each point from the one before, where the walk holds
                let mut near = None;
                let mut gaps: Vec<(f64, forge_math::Point2)> = Vec::with_capacity(pts.len());
                for &p in &pts {
                    let uv = near.map_or_else(|| s.project(p), |uv| s.project_near(p, uv));
                    near = Some(uv);
                    gaps.push((s.eval(uv).distance(p), uv));
                }
                let mut worst = gaps.iter().fold(0.0_f64, |m, g| m.max(g.0));
                if refine && gaps.len() > 2 {
                    // a gap can peak sharply between samples (a free-form edge's came out 1.15%
                    // over 256 samples' on MM 2024): around the three highest local peaks, 32
                    // more each, what 2048 evenly would find at a twentieth of the work
                    let mut peaks: Vec<usize> = (0..gaps.len())
                        .filter(|&i| {
                            (i == 0 || gaps[i].0 >= gaps[i - 1].0)
                                && (i + 1 == gaps.len() || gaps[i].0 >= gaps[i + 1].0)
                        })
                        .collect();
                    peaks.sort_by(|&a, &b| gaps[b].0.total_cmp(&gaps[a].0));
                    for &i in peaks.iter().take(3) {
                        let (lo, hi) = (at(i.saturating_sub(1)), at((i + 1).min(n)));
                        let mut uv = gaps[i].1;
                        for k in 0..=32 {
                            let t = lo + (hi - lo) * (k as f64 / 32.0);
                            let p = x.curve.eval(t.min(x.range.hi));
                            uv = s.project_near(p, uv);
                            worst = worst.max(s.eval(uv).distance(p));
                        }
                    }
                }
                worst
            })
            .fold(0.0, f64::max)
    };
    let dev = measure(16, false);
    if free_form || dev > LINEAR {
        let fine = measure(256, false);
        // an edge off its faces: its peaks looked at closely, as `check` will ask between these
        // samples once a boolean has trimmed it
        if fine > LINEAR {
            dev.max(measure(256, true))
        } else {
            dev.max(fine)
        }
    } else {
        dev
    }
}

/// How far healing may move an edge: from a file's stated uncertainty, but never much of the part.
/// A normalised part keeps the figure of a far larger original, as much as a tenth of its size, and
/// an edge then went to where two other planes meet, further off than it was long.
fn heal_limit(uncertainty: f64, size: f64) -> f64 {
    (uncertainty.max(1e-3) * 20.0).min(2e-3 * size)
}

const TOO_LONG: &str = "its surfaces would take too long to measure";

/// What a body as read cannot be checked by, in plain words: its integrals and mesh disagree (on a
/// file's geometry the integrals can go wrong with nothing to show for it, `research/step/README.md`),
/// or one of them cannot be worked out. Such a body is imported with the warning, not refused; only a
/// budget run out refuses it.
type Measured<'a> = (
    Option<String>,
    Option<forge_topo::mass::MassProps>,
    Option<std::borrow::Cow<'a, forge_mesh::TriMesh>>,
);

/// `pre`: the display mesh and what drawing it spent, where it ran no budget out: the check's
/// mesh where it is the same computation ([`finish_checked`]).
fn measures_agree<'a>(
    body: &Body,
    pre: Option<(&'a forge_mesh::TriMesh, u64)>,
) -> Result<Measured<'a>, String> {
    let m = times::timed(5, || forge_topo::mass::mass_rel(body, 1e-6));
    if forge_geom::budget::over() {
        return Err(TOO_LONG.into());
    }
    let m = match m {
        Ok(m) => m,
        Err(e) => {
            return Ok((
                Some(format!(
                    "its volume cannot be worked out ({}), so nothing it measures is checked",
                    e.why
                )),
                None,
                None,
            ));
        }
    };
    let (warning, mesh) = times::timed(6, || mesh_agrees(body, &m, pre))?;
    Ok((warning, Some(m), mesh))
}

/// `measures_agree` past the integrals: the mesh's volume and area against them.
/// With the mesh at 5e-4 of the diagonal where it is the one `display_mesh` asks for, bit for bit:
/// made with no budget test true (some left after it), and on less than `tessellate::MESH` (what a
/// mesh outside an import is drawn within), so drawn again within that, it would spend the same
/// and meet no test true either: the same computation.
type Checked<'a> = (Option<String>, Option<std::borrow::Cow<'a, forge_mesh::TriMesh>>);

fn mesh_agrees<'a>(
    body: &Body,
    m: &forge_topo::mass::MassProps,
    pre: Option<(&'a forge_mesh::TriMesh, u64)>,
) -> Result<Checked<'a>, String> {
    use std::borrow::Cow;
    let mesh_at = |chord: f64| {
        forge_mesh::tessellate::tessellate_tolerant(
            body,
            &forge_mesh::Quality {
                chord,
                ..forge_mesh::Quality::default()
            },
        )
    };
    let diagonal = body.faces_box().diagonal();
    let before = forge_geom::budget::left();
    // the display mesh where drawing it here would be the same: it spent less than is left
    let first = match pre {
        Some((m, spent)) if spent < before => {
            forge_geom::budget::spend_many(spent);
            Ok(Cow::Borrowed(m))
        }
        _ => mesh_at(5e-4 * diagonal).map(Cow::Owned),
    };
    let (mesh, chord, display) = match first {
        Ok(x) => {
            let left = forge_geom::budget::left();
            let display = left > 0 && before - left < forge_mesh::tessellate::MESH;
            (x, 5e-4 * diagonal, display)
        }
        Err(_) => match mesh_at(5e-3 * diagonal) {
            Ok(x) => (Cow::Owned(x), 5e-3 * diagonal, false),
            Err(_) if forge_geom::budget::over() => return Err(TOO_LONG.into()),
            Err(forge_mesh::TessError::Face { error, .. }) => {
                use forge_mesh::CdtError as E;
                let why = match error {
                    E::ConstraintsIntersect { .. } => "its edges cross",
                    E::ConstraintThroughPoint { .. } => "an edge runs through a corner",
                    E::DuplicatePoint { .. } => "two of its corners fall together",
                    E::BadLoop { .. } => "a loop of it has no area",
                    _ => "its boundary does not close",
                };
                return Ok((
                    Some(format!(
                        "a face of it cannot be drawn ({why}), so its volume is not checked against a drawing"
                    )),
                    None,
                ));
            }
        },
    };
    let (mut volume, mut area) = (0.0, 0.0);
    for t in &mesh.triangles {
        let [a, b, c] = t.map(|i| mesh.positions[i as usize] - Point3::ORIGIN);
        let n = (b - a).cross(c - a);
        area += 0.5 * n.length();
        volume += a.dot(n) / 6.0;
    }
    // a mesh strays by its chord at most, and only over what is curved
    let curved: f64 = m
        .faces
        .iter()
        .filter(|(f, _)| !matches!(body.face(*f).surface, forge_geom::Surface::Plane(_)))
        .map(|x| x.1)
        .sum();
    let mesh = display.then_some(mesh);
    if (m.volume - volume).abs() > 0.01 * volume.abs() + 0.5 * curved * chord
        || (m.area - area).abs() > 0.01 * area
    {
        return Ok((
            Some(format!(
                "its volume and area work out to {:.6e} and {:.6e}, but drawn they come to {volume:.6e} \
                 and {area:.6e}: one of the two is wrong, and which is not known",
                m.volume, m.area
            )),
            mesh,
        ));
    }
    Ok((None, mesh))
}

/// How many placements a file gives that move a part, and how many extra uses of a part or of a
/// mapped shape, none of which are applied. A placement onto itself (Onshape wraps every part in one)
/// asks for nothing.
fn assembly_asks(f: &File<'_>, reader: &Reader<'_>) -> usize {
    let moves = |a: Option<&Value>, b: Option<&Value>| match (a, b) {
        (Some(Value::Ref(a)), Some(Value::Ref(b))) => match (reader.placement(*a), reader.placement(*b)) {
            (Ok(p), Ok(q)) => {
                let near = |x: Vec3, y: Vec3| x.dot(y) >= 1.0 - 1e-12;
                p.origin.distance(q.origin) > 1e-9 * (1.0 + (p.origin - Point3::ORIGIN).abs_max())
                    || !near(p.x().into(), q.x().into())
                    || !near(p.z().into(), q.z().into())
            }
            // a part placed by something not read: moved, for all that can be told
            _ => true,
        },
        _ => true,
    };
    let (mut asks, mut used) = (0, BTreeMap::<u32, usize>::new());
    for &(_, at, n) in &f.ids {
        for r in &f.records[at as usize..(at + n) as usize] {
            let a = f.args(r);
            match f.name(r) {
                "NEXT_ASSEMBLY_USAGE_OCCURRENCE" => {
                    if let Some(Value::Ref(part)) = a.get(4) {
                        *used.entry(*part).or_default() += 1;
                    }
                }
                "ITEM_DEFINED_TRANSFORMATION" => asks += usize::from(moves(a.get(2), a.get(3))),
                "MAPPED_ITEM" => {
                    if let Some(Value::Ref(map)) = a.get(1) {
                        *used.entry(*map).or_default() += 1;
                        let origin = f.records_of(*map).and_then(|x| x.first()).map(|x| f.args(x));
                        asks += usize::from(moves(origin.as_ref().and_then(|o| o.first()), a.get(2)));
                    }
                }
                _ => {}
            }
        }
    }
    asks + used.values().map(|&k| k - 1).sum::<usize>()
}

/// The first n chars of x's Debug text, stopping there (a spline's whole text is megabytes).
fn debug_head(x: &impl std::fmt::Debug, n: usize) -> String {
    struct Head(String, usize);
    impl std::fmt::Write for Head {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            for c in s.chars() {
                if self.1 == 0 {
                    return Err(std::fmt::Error);
                }
                self.0.push(c);
                self.1 -= 1;
            }
            Ok(())
        }
    }
    let mut h = Head(String::new(), n);
    let _ = std::fmt::write(&mut h, format_args!("{x:?}"));
    h.0
}

/// A face's edges lie off its surface by more than both of these (mm, and of the part's size)
/// only where the file gives the face a surface somewhere else: the solid is refused, saying so.
const FAR_FACE_MM: f64 = 1.0;
const FAR_FACE_PART: f64 = 0.01;

/// A distance in mm, in words.
fn far(mm: f64) -> String {
    if mm >= 1000.0 {
        format!("{:.1} m", mm / 1000.0)
    } else {
        format!("{mm:.1} mm")
    }
}

/// How far a file's geometry misses itself, before anything is changed; and each face's largest
/// gap, by face.
fn measure_gaps(proto: &Proto, stats: &mut Stats) -> Vec<f64> {
    let mut by_face = vec![0.0_f64; proto.faces.len()];
    for (fi, fa) in proto.faces.iter().enumerate() {
        for u in fa.loops.iter().flatten() {
            let e = &proto.edges[u.0];
            let w = e.range.width();
            let worst = (0..=8)
                .map(|k| fa.surface.distance(e.curve.eval(e.range.lo + w * k as f64 / 8.0)))
                .fold(0.0, f64::max);
            stats.curve_off_surface = stats.curve_off_surface.max(worst);
            by_face[fi] = by_face[fi].max(worst);
            if worst > forge_math::tol::LINEAR {
                stats.edges_off += 1;
            }
            let kind = |s: &str| s.split(['(', ' ']).next().unwrap_or("").to_string();
            let key = format!(
                "{}/{}",
                kind(&debug_head(&e.curve, 12)),
                kind(&debug_head(&fa.surface, 12))
            );
            let en = stats.pairs.entry(key).or_insert((0, 0, 0.0));
            en.0 += 1;
            en.1 += usize::from(worst > forge_math::tol::LINEAR);
            en.2 = en.2.max(worst);
        }
    }
    by_face
}

/// From a read file's topology to a body, and its integrals where the check worked them out: seams,
/// edges tightened, checked; healing and the check on one budget. `uncertainty`: what the file says of itself.
/// Whether every shell but the largest bounds negative volume.
fn voids_inward(body: &Body) -> bool {
    let v: Vec<f64> = (0..body.shells.len())
        .map(|i| forge_topo::mass::shell_volume(body, forge_topo::ShellId(i as u32)))
        .collect();
    let most = v.iter().fold(0.0f64, |m, x| m.max(x.abs()));
    v.iter().filter(|x| x.abs() < most).all(|&x| x < 0.0)
}

/// What reading one solid root gave: the solid or its refusal (none: only measured), what it
/// added to the stats, its units, and whether a `finish` ran (which ends its thread's budget).
struct RootOut {
    got: Option<Result<Solid, Refusal>>,
    stats: Stats,
    units: Option<Units>,
    finished: bool,
}

impl Stats {
    /// Adds what one solid's read counted (`assembly`, `uncertainty` and `units_mm` are the
    /// file's, set by the caller).
    fn add(&mut self, o: &Stats) {
        self.faces += o.faces;
        self.edges += o.edges;
        self.vertex_off_curve = self.vertex_off_curve.max(o.vertex_off_curve);
        self.curve_off_surface = self.curve_off_surface.max(o.curve_off_surface);
        self.vertices_off += o.vertices_off;
        self.edges_off += o.edges_off;
        for (k, v) in &o.pairs {
            let en = self.pairs.entry(k.clone()).or_insert((0, 0, 0.0));
            en.0 += v.0;
            en.1 += v.1;
            en.2 = en.2.max(v.2);
        }
        self.tangent_off += o.tangent_off;
        self.tolerant_edges += o.tolerant_edges;
        self.max_edge_tol = self.max_edge_tol.max(o.max_edge_tol);
    }
}

/// One solid root of a file read, as far as its topology: its units and the proto body its
/// shell makes (voids as they stand), with what that counted. Then [`finish_root`].
struct Built {
    out: RootOut,
    name: String,
    /// `None` where the units could not be read (the refusal is in `out`).
    built: Option<(Units, Result<Proto, Refusal>)>,
}

impl Built {
    /// How long the rest will take, about: its faces.
    fn size(&self) -> usize {
        match &self.built {
            Some((_, Ok(p))) => p.faces.len(),
            _ => 0,
        }
    }
}

/// A root's shell (and voids, each `void_flip` or as it stands) to a proto body.
fn build_shell(
    f: &File<'_>,
    id: u32,
    r: Record,
    u: Units,
    void_flip: bool,
    stats: &mut Stats,
) -> Result<Proto, Refusal> {
    let mut reader = Reader { f, u, depth: 0 };
    let mut b = Build {
        r: &mut reader,
        p: Proto {
            salt: Name::feature(u64::from(id)).with_str("step"),
            ..Proto::default()
        },
        verts: BTreeMap::new(),
        edges: BTreeMap::new(),
        ends: None,
        stats,
        salt: Name::feature(u64::from(id)).with_str("step"),
    };
    let built = (|| {
        if f.name(&r) == "OPEN_SHELL" {
            return b.shell(id, false);
        }
        let a = f.args(&r);
        let outer = b.r.reference(id, "solid", a.get(1))?;
        b.shell(outer, false)?;
        if f.name(&r) == "BREP_WITH_VOIDS" {
            for v in b.r.list(id, "solid", a.get(2))? {
                let vid = b.r.reference(id, "solid", Some(v))?;
                b.shell(vid, void_flip)?;
            }
        }
        Ok(())
    })();
    let proto = b.p;
    built?;
    // an open shell is a solid only if it closes: every edge between two faces' uses
    if f.name(&r) == "OPEN_SHELL" {
        let mut uses = vec![0u32; proto.edges.len()];
        for face in &proto.faces {
            for l in &face.loops {
                for &(e, _) in l {
                    uses[e] += 1;
                }
            }
        }
        match uses.iter().filter(|&&n| n != 2).count() {
            0 => {}
            n => {
                return refuse(
                    id,
                    "OPEN_SHELL",
                    format!("a sheet, not a solid: {n} edges not between two faces"),
                );
            }
        }
    }
    Ok(proto)
}

/// One solid root of a file read, as far as its topology (as `read_file` did each in turn).
fn build_root(f: &File<'_>, id: u32, r: Record, ctx: Option<u32>, assembly: usize) -> Built {
    let mut out = RootOut {
        got: None,
        stats: Stats::default(),
        units: None,
        finished: false,
    };
    out.stats.assembly = assembly;
    let Some(u) = ctx.and_then(|c| units_of(f, c)) else {
        out.got = Some(Err(Refusal {
            entity: id,
            kind: f.name(&r).to_string(),
            why: "its units cannot be read".into(),
        }));
        return Built {
            out,
            name: String::new(),
            built: None,
        };
    };
    out.units = Some(u);
    let name = match f.args(&r).first() {
        Some(Value::Str(a, n)) => f.str_of(*a, *n).to_string(),
        _ => String::new(),
    };
    // a void's faces first as they stand (the files of every system measured, research/step-voids)
    let built = build_shell(f, id, r, u, true, &mut out.stats);
    Built {
        out,
        name,
        built: Some((u, built)),
    }
}

/// The rest of a root's read after [`build_root`]: the body finished and checked.
fn finish_root(f: &File<'_>, id: u32, r: Record, b: Built, lim: &Limits, only_measure: bool) -> RootOut {
    let Built { mut out, name, built } = b;
    let Some((u, mut built)) = built else {
        return out;
    };
    let stats = &mut out.stats;
    let mut void_flip = true;
    loop {
        match built {
            Ok(proto) => {
                if only_measure {
                    let _ = measure_gaps(&proto, stats);
                    break;
                }
                out.finished = true;
                let done = finish(proto, u.uncertainty, lim, stats);
                // files differ on what a void's .F. turns: a void must bound negative volume
                if void_flip
                    && f.name(&r) == "BREP_WITH_VOIDS"
                    && !done.as_ref().is_ok_and(|(b, _, _)| voids_inward(b))
                {
                    void_flip = false;
                    built = build_shell(f, id, r, u, void_flip, stats);
                    continue;
                }
                out.got = Some(match done {
                    Ok((mut body, mass, mesh)) => {
                        body.unplaced = stats.assembly > 0;
                        Ok(Solid {
                            entity: id,
                            name,
                            body,
                            mass,
                            mesh,
                        })
                    }
                    Err(why) => Err(Refusal {
                        entity: id,
                        kind: f.name(&r).to_string(),
                        why,
                    }),
                });
            }
            Err(x) => out.got = Some(Err(x)),
        }
        break;
    }
    out
}

/// A body read, its integrals where worked out, and its display mesh where the check drew it.
pub(crate) type Finished = (
    Body,
    Option<forge_topo::mass::MassProps>,
    Option<forge_mesh::TriMesh>,
);

pub(crate) fn finish(
    proto: Proto,
    uncertainty: f64,
    lim: &Limits,
    stats: &mut Stats,
) -> Result<Finished, String> {
    let (mut body, budget) = times::timed(2, || healed(proto, uncertainty, lim, stats))?;
    if times::timed(3, || laid_out_in_a_speck(&body)) {
        return Err(SPECK.into());
    }
    times::timed(4, || record_deviations(&mut body, stats));
    let agree = measures_agree(&body, None);
    drop(budget);
    let (unverified, mass, mesh) = agree?;
    body.unverified = unverified;
    Ok((body, mass, mesh.map(std::borrow::Cow::into_owned)))
}

const SPECK: &str = "a face laid out over a span of its parameters below 2^-433";

/// `finish` as far as the body healed: what is checked of the file's edges and faces, the gaps
/// measured, and the body made, under the read's budget (returned live: the rest spends it).
fn healed(
    mut proto: Proto,
    uncertainty: f64,
    lim: &Limits,
    stats: &mut Stats,
) -> Result<(Body, forge_geom::budget::Budget), String> {
    if proto.faces.is_empty() {
        return Err("a solid with no faces".into());
    }
    // an edge runs at most once round a closed curve, over a finite range (a file's range of
    // many turns made a check sample it 65536 times over itself, and every pair of those crossed)
    for e in &proto.edges {
        let r = e.range;
        if !(r.lo.is_finite() && r.hi.is_finite()) {
            return Err("an edge's range is not finite".into());
        }
        if let Some(p) = e.curve.domain().period
            && r.width() > p * (1.0 + 1e-9)
        {
            return Err("an edge runs more than once round its curve".into());
        }
    }
    // a face goes along an edge at most once each way (a seam: once each way); loops that repeat
    // an edge are no boundary, and meshing them cut the repeats finer until memory ran out
    for f in &proto.faces {
        let mut used: Vec<(usize, bool)> = f.loops.iter().flatten().copied().collect();
        used.sort_unstable();
        if used.windows(2).any(|w| w[0] == w[1]) {
            return Err("a face's loops go along one edge twice the same way".into());
        }
    }
    let gaps = measure_gaps(&proto, stats);
    // a face whose edges lie far off its surface has the wrong surface in the file (VS-3 #478011:
    // a B-spline patch 5 m from its face's edges): nothing in the file says where it is
    let size = Box3::from_points(proto.verts.iter().map(|v| v.0)).diagonal();
    if let Some((fi, &gap)) = gaps
        .iter()
        .enumerate()
        .find(|&(_, &g)| g > FAR_FACE_MM && g > FAR_FACE_PART * size)
    {
        let face = proto.face_ids.get(fi).map_or(String::new(), |id| format!(" #{id}"));
        return Err(format!("face{face} lies {} from its edges: the file gives it another surface", far(gap)));
    }
    let budget = forge_geom::budget::scoped(lim.max_evals);
    // a loop running out and back along two edges that lie on each other (a slit of no width)
    proto.cancel_slits(uncertainty.max(forge_math::tol::LINEAR));
    proto.split_closed_splines();
    proto.add_seams()?;
    let size = Box3::from_points(proto.verts.iter().map(|v| v.0)).diagonal();
    proto.tighten(heal_limit(uncertainty, size));
    stats.tangent_off += proto.tangent;
    proto.split_fans();
    proto.outer_first();
    Ok((proto.into_body(), budget))
}

/// A body as far as it is shown before its checks ([`open`]): its display mesh, as
/// [`Solid::display_mesh`] makes it (none: it cannot be drawn, or not within what the check's mesh
/// could have at most).
pub(crate) struct Shown {
    mesh: Option<forge_mesh::TriMesh>,
}

/// `finish`'s first part, for showing a body at once: healed under the read's budget, as
/// `finish` heals it; then the edges' tolerances, which spend but test no budget (each edge's
/// answer is the same at any budget: what they spend is counted, for the checks to charge); then
/// the display mesh, under a budget of its own as `display_mesh` makes it (what it spends counted
/// too). The speck check and the measures come after ([`finish_checked`]).
pub(crate) fn finish_shown(
    proto: Proto,
    uncertainty: f64,
    lim: &Limits,
    stats: &mut Stats,
) -> Result<Shown, String> {
    use forge_geom::budget;
    let (mut body, healing) = healed(proto, uncertainty, lim, stats)?;
    let left = budget::left();
    drop(healing);
    let mut dev = Stats::default();
    let dev_spent = {
        // a budget no read runs out, counting
        let _b = budget::scoped(u64::MAX);
        record_deviations(&mut body, &mut dev);
        u64::MAX - budget::left()
    };
    // the display mesh, drawn within no more than its own budget (MESH) or what the check's mesh
    // could have at most (what healing left, less the tolerances' spend): a drawing that runs that
    // out is not kept (the check's mesh would run out too: such a body is refused as too long to
    // measure, or drawn after its checks), and one that does not is MESH's, bit for bit
    let most = forge_mesh::tessellate::MESH.min(left.saturating_sub(dev_spent));
    let (mesh, mesh_spent) = {
        let _b = budget::scoped(most);
        let m = forge_mesh::tessellate::tessellate_tolerant(&body, &Solid::display_quality(&body)).ok();
        match budget::left() {
            0 => (None, None),
            l => (m, Some(most - l)),
        }
    };
    let _ = mesh_spent;
    Ok(Shown { mesh })
}

/// Whether a face's parameter loops, drawn as finely as `forge_topo::check` draws them, hold a
/// coordinate that is not 0 but below 2^-433 in size: a surface's frame 1e308 from its face
/// lays the face out over a span of 1e-308. The predicates' filters (`orient2d`'s debug
/// precondition, `forge_math::predicates::check_filter`) hold for coordinates that are 0 or at
/// least 2^-433: their differences are multiples of 2^-485, so each product of two is exact,
/// and a nonzero sum of such products times the filter's bound (about 1.5 * 2^-52) is at least
/// 2^-1022, normal. This covers the loops' points only, not every point projected into them.
fn laid_out_in_a_speck(body: &Body) -> bool {
    // 2^-433, exactly
    const SPECK: f64 = f64::from_bits(((1023 - 433) as u64) << 52);
    let tiny = |x: f64| x != 0.0 && x.abs() < SPECK;
    let speck = |f: forge_topo::FaceId| {
        forge_topo::uv::face_uv_loops_with(body, f, |e| 8 * forge_topo::uv::default_segments(e))
            .iter()
            .any(|l| l.points.iter().any(|p| tiny(p.x) || tiny(p.y)))
    };
    // every face on all cores (`budget::map_counted`: a face's loops test no budget, so each answer
    // is one by one's, and so is what they spend together; one by one stopped at the first speck,
    // and spent less, but then the body is refused and nothing more is spent)
    let ids: Vec<forge_topo::FaceId> = body.face_ids().collect();
    forge_geom::budget::map_counted(ids.len(), |k| speck(ids[k]))
        .into_iter()
        .any(|x| x)
}

/// Reads a whole file. Parts that cannot be read become refusals; the others are returned.
pub fn import(text: &[u8], lim: &Limits) -> Result<Imported, ImportError> {
    if crate::brep::is_brep(text) {
        return crate::brep::import(text, lim);
    }
    if crate::iges::is_iges(text) {
        return crate::iges::import(text, lim);
    }
    read_file(text, lim, false)
}

/// [`import`], and every solid drawn for display ([`Solid::display_mesh`], kept in
/// [`Solid::mesh`] wherever it can be drawn): for a viewer, which draws every solid. The read
/// itself as `import` (drawn in its order: a drawing fills caches whose first filler pays their
/// evaluations, and a read's refusals as too long to measure count them), each solid the read's
/// check did not draw drawn after it, on all cores.
pub fn import_drawn(text: &[u8], lim: &Limits) -> Result<Imported, ImportError> {
    let mut imp = import(text, lim)?;
    let drawn = forge_math::par::map_eager(imp.solids.len(), |k| {
        let s = &imp.solids[k];
        s.mesh
            .is_none()
            .then(|| times::timed(7, || s.display_mesh().ok().map(std::borrow::Cow::into_owned)))
    });
    for (s, m) in imp.solids.iter_mut().zip(drawn) {
        if let Some(m) = m {
            s.mesh = m;
        }
    }
    Ok(imp)
}

/// The most any edge of a file lies off a face it bounds, by the same measure a read reports.
pub(crate) fn curve_off_surface(text: &[u8], lim: &Limits) -> Result<f64, ImportError> {
    Ok(read_file(text, lim, true)?.stats.curve_off_surface)
}

/// A file's solid roots, each with its representation's context, and what an assembly asks.
struct Head<'a> {
    f: File<'a>,
    roots: Vec<(u32, Record, Option<u32>)>,
    assembly: usize,
    /// Each use of a solid in the file's assembly (none read when only measuring).
    occurrences: Vec<Occurrence>,
}

fn head<'a>(text: &'a [u8], lim: &Limits, only_measure: bool) -> Result<Head<'a>, ImportError> {
    let f = times::timed(0, || parse(text, lim)).map_err(ImportError::Parse)?;
    // each solid's context: from the representation that lists it
    let mut ctx_of: BTreeMap<u32, u32> = BTreeMap::new();
    let mut any_ctx = None;
    let mut roots = Vec::new();
    // open shells read as solids, and the surface model each is in (for its context)
    let mut open_of: Vec<(u32, u32)> = Vec::new();
    for &(id, at, n) in &f.ids {
        let recs = &f.records[at as usize..(at + n) as usize];
        for r in recs {
            let name = f.name(r);
            if name == "MANIFOLD_SOLID_BREP" || name == "BREP_WITH_VOIDS" {
                roots.push((id, *r));
            }
            // a surface model's shells: SolidWorks writes some whole machines so (TRT160), each
            // body a shell of faces that closes; read as a solid when it does
            if name == "SHELL_BASED_SURFACE_MODEL"
                && let Some(Value::List(l, ln)) = f.args(r).get(1)
            {
                for v in f.run(*l, *ln) {
                    if let Value::Ref(x) = v
                        && kw(&f, *x) == Some("OPEN_SHELL")
                        && let Some(sr) = f.records_of(*x).and_then(|rs| rs.first())
                    {
                        roots.push((*x, *sr));
                        open_of.push((*x, id));
                    }
                }
            }
            if name.ends_with("_REPRESENTATION") || name == "SHAPE_REPRESENTATION" {
                let a = f.args(r);
                if let (Some(Value::List(l, ln)), Some(Value::Ref(c))) = (a.get(1), a.get(2)) {
                    any_ctx.get_or_insert(*c);
                    for v in f.run(*l, *ln) {
                        if let Value::Ref(x) = v {
                            ctx_of.entry(*x).or_insert(*c);
                        }
                    }
                }
            }
        }
    }
    for &(shell, model) in &open_of {
        if let Some(&c) = ctx_of.get(&model) {
            ctx_of.entry(shell).or_insert(c);
        }
    }
    let (mut assembly, mut occurrences) = (0, Vec::new());
    if let Some(u) = any_ctx.and_then(|c| units_of(&f, c)) {
        assembly = assembly_asks(&f, &Reader { f: &f, u, depth: 0 });
        if !only_measure {
            occurrences = self::occurrences(&f, &Reader { f: &f, u, depth: 0 });
        }
    }
    let roots = roots
        .into_iter()
        .map(|(id, r)| (id, r, ctx_of.get(&id).copied().or(any_ctx)))
        .collect();
    Ok(Head {
        f,
        roots,
        assembly,
        occurrences,
    })
}

/// Every root's topology, on all cores, and the order to read the rest in: the largest first (the
/// longest: started last, it would run alone at the end).
fn built(h: &Head<'_>) -> (Vec<std::sync::Mutex<Option<Built>>>, Vec<usize>) {
    let built = forge_math::par::map(h.roots.len(), |k| {
        let (id, r, ctx) = h.roots[k];
        std::sync::Mutex::new(Some(times::timed(1, || build_root(&h.f, id, r, ctx, h.assembly))))
    });
    let sizes: Vec<usize> = built
        .iter()
        .map(|b| {
            b.lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
                .map_or(0, Built::size)
        })
        .collect();
    let mut order: Vec<usize> = (0..h.roots.len()).collect();
    order.sort_by_key(|&k| std::cmp::Reverse(sizes[k]));
    (built, order)
}

/// `f(k, item k)` for each item, taken once each, on all cores in `order`, the answers in item
/// order.
fn each_in<T: Send, U: Send>(
    items: Vec<std::sync::Mutex<Option<T>>>,
    order: &[usize],
    f: impl Fn(usize, T) -> U + Sync,
) -> Vec<U> {
    let done = forge_math::par::map_eager(order.len(), |i| {
        let k = order[i];
        let t = items[k]
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .expect("each item taken once");
        (k, f(k, t))
    });
    let mut out: Vec<Option<U>> = (0..items.len()).map(|_| None).collect();
    for (k, u) in done {
        out[k] = Some(u);
    }
    out.into_iter().map(|u| u.expect("every item done")).collect()
}

/// The solids' reads put together in the file's order: the same as reading them one after another.
fn gathered(h: Head<'_>, outs: Vec<RootOut>, only_measure: bool) -> Result<Imported, ImportError> {
    let mut stats = Stats {
        assembly: h.assembly,
        ..Stats::default()
    };
    let (mut solids, mut refusals) = (Vec::new(), Vec::new());
    let mut finished = false;
    for out in outs {
        if let Some(u) = out.units {
            stats.uncertainty = u.uncertainty;
            stats.units_mm = u.mm;
        }
        stats.add(&out.stats);
        finished |= out.finished;
        match out.got {
            Some(Ok(s)) => solids.push(s),
            Some(Err(x)) => refusals.push(x),
            None => {}
        }
    }
    // a `finish` ends the budget of the thread it runs on: so it ends here, as it did when every
    // solid was read on this thread
    if finished {
        drop(forge_geom::budget::scoped(0));
    }
    if solids.is_empty() && refusals.is_empty() && !only_measure {
        return Err(ImportError::NoSolids);
    }
    Ok(Imported {
        solids,
        refusals,
        occurrences: h.occurrences,
        schema: h.f.schema.clone(),
        system: h.f.system.clone(),
        stats,
    })
}

/// `only_measure`: as far as what the file's geometry misses by, and no bodies.
fn read_file(text: &[u8], lim: &Limits, only_measure: bool) -> Result<Imported, ImportError> {
    let h = head(text, lim, only_measure)?;
    // each solid read on its own (its own stats, its own budget in `finish`), on all cores, and
    // put together in the file's order: the same as reading them one after another
    let (built, order) = built(&h);
    let outs = each_in(built, &order, |k, b| {
        let (id, r, _) = h.roots[k];
        finish_root(&h.f, id, r, b, lim, only_measure)
    });
    gathered(h, outs, only_measure)
}

/// A root read as far as shown ([`finish_shown`]), none where it has no body to show.
struct RootShown {
    first: Option<Result<Shown, String>>,
}

fn show_root(b: Built, lim: &Limits) -> RootShown {
    let Built { mut out, built, .. } = b;
    let first = match built {
        Some((u, Ok(proto))) => Some(finish_shown(proto, u.uncertainty, lim, &mut out.stats)),
        _ => None,
    };
    RootShown { first }
}

/// A file read as far as its solids can be shown ([`open`]); [`Opened::finish`] reads the rest.
pub struct Opened<'a> {
    text: &'a [u8],
    lim: Limits,
    inner: OpenedAs<'a>,
}

enum OpenedAs<'a> {
    /// A STEP file: its roots, each shown or refused.
    Step { h: Head<'a>, roots: Vec<RootShown> },
    /// Another format, read whole and drawn.
    Read(Imported),
}

/// Opens a file to show it at once: each solid read as far as its body, healed, and its display
/// mesh ([`Solid::display_mesh`]'s), on all cores, before the checks and measures that decide
/// whether a solid is refused. The solids shown are those not refused so far: the checks may
/// refuse some yet. [`Opened::finish`] gives what [`import_drawn`] gives.
pub fn open<'a>(text: &'a [u8], lim: &Limits) -> Result<Opened<'a>, ImportError> {
    if crate::brep::is_brep(text) || crate::iges::is_iges(text) {
        return Ok(Opened {
            text,
            lim: *lim,
            inner: OpenedAs::Read(import_drawn(text, lim)?),
        });
    }
    let h = head(text, lim, false)?;
    let (built, order) = built(&h);
    let roots = each_in(built, &order, |_, b| show_root(b, lim));
    Ok(Opened {
        text,
        lim: *lim,
        inner: OpenedAs::Step { h, roots },
    })
}

impl Opened<'_> {
    /// What can be shown now: each solid not refused so far, by its entity, drawn, in the file's
    /// order.
    pub fn shown(&self) -> Vec<(u32, &forge_mesh::TriMesh)> {
        match &self.inner {
            OpenedAs::Step { h, roots, .. } => h
                .roots
                .iter()
                .zip(roots)
                .filter_map(|(&(id, _, _), rs)| match &rs.first {
                    Some(Ok(sh)) => sh.mesh.as_ref().map(|m| (id, m)),
                    _ => None,
                })
                .collect(),
            OpenedAs::Read(imp) => imp
                .solids
                .iter()
                .filter_map(|s| s.mesh.as_ref().map(|m| (s.entity, m)))
                .collect(),
        }
    }

    /// The whole read: what [`import_drawn`] gives. It reads the file again, in `import`'s
    /// order: the shown part drew each body before its checks, and a drawing fills caches whose
    /// first filler pays their evaluations, which a read's refusals as too long count.
    pub fn finish(self) -> Result<Imported, ImportError> {
        match self.inner {
            OpenedAs::Read(imp) => Ok(imp),
            OpenedAs::Step { .. } => import_drawn(self.text, &self.lim),
        }
    }
}

/// One use of a solid in a file's assembly: the solid's entity, the product names from the top
/// assembly down to the part, and where the part stands in the top assembly's coordinates (mm).
#[derive(Clone, Debug)]
pub struct Occurrence {
    pub solid: u32,
    pub path: Vec<String>,
    pub frame: Frame,
}

/// The assembly a file's products make (AP203/AP214 PDM usage, §4): from each top product down
/// its next_assembly_usage_occurrences, each placed by its context dependent shape
/// representation's item defined transformation, to the solids each product's shape holds. Empty
/// where the file has no product structure; a use whose placement cannot be read is left out.
fn occurrences(f: &File<'_>, reader: &Reader<'_>) -> Vec<Occurrence> {
    let refs = |r: &Record| -> Vec<u32> {
        f.args(r)
            .iter()
            .filter_map(|v| if let Value::Ref(x) = v { Some(*x) } else { None })
            .collect()
    };
    let text = |v: Option<&Value>| match v {
        Some(Value::Str(a, n)) => f.str_of(*a, *n).trim().to_string(),
        _ => String::new(),
    };
    let mut solids_in: BTreeMap<u32, Vec<u32>> = BTreeMap::new(); // representation -> solids it lists
    let mut linked: BTreeMap<u32, Vec<u32>> = BTreeMap::new(); // representations related untransformed
    let mut shape_of: BTreeMap<u32, u32> = BTreeMap::new(); // product definition shape -> definition
    let mut sdr: Vec<(u32, u32)> = Vec::new(); // (product definition shape, representation)
    let mut nauo: Vec<(u32, u32, u32)> = Vec::new(); // (id, parent definition, child definition)
    let mut cdsr: Vec<(u32, u32)> = Vec::new(); // (relationship, product definition shape)
    let mut rep_name: BTreeMap<u32, String> = BTreeMap::new();
    let mut pd_formation: BTreeMap<u32, u32> = BTreeMap::new();
    let mut formation_product: BTreeMap<u32, u32> = BTreeMap::new();
    let mut product_name: BTreeMap<u32, String> = BTreeMap::new();
    let is_solid = |x: u32| matches!(kw(f, x), Some("MANIFOLD_SOLID_BREP" | "BREP_WITH_VOIDS"));
    for &(id, at, n) in &f.ids {
        let recs = &f.records[at as usize..(at + n) as usize];
        let has = |k: &str| recs.iter().any(|r| f.name(r) == k);
        for r in recs {
            let a = f.args(r);
            match f.name(r) {
                "PRODUCT_DEFINITION_SHAPE" => {
                    if let Some(Value::Ref(d)) = a.get(2) {
                        shape_of.insert(id, *d);
                    }
                }
                "SHAPE_DEFINITION_REPRESENTATION" => {
                    if let (Some(Value::Ref(p)), Some(Value::Ref(s))) = (a.first(), a.get(1)) {
                        sdr.push((*p, *s));
                    }
                }
                "NEXT_ASSEMBLY_USAGE_OCCURRENCE" => {
                    if let (Some(Value::Ref(p)), Some(Value::Ref(c))) = (a.get(3), a.get(4)) {
                        nauo.push((id, *p, *c));
                    }
                }
                "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION" => {
                    if let (Some(Value::Ref(rel)), Some(Value::Ref(p))) = (a.first(), a.get(1)) {
                        cdsr.push((*rel, *p));
                    }
                }
                "REPRESENTATION_RELATIONSHIP" | "SHAPE_REPRESENTATION_RELATIONSHIP"
                    if !has("REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION") =>
                {
                    if let (Some(Value::Ref(x)), Some(Value::Ref(y))) = (a.get(2), a.get(3)) {
                        linked.entry(*x).or_default().push(*y);
                        linked.entry(*y).or_default().push(*x);
                    }
                }
                "PRODUCT_DEFINITION" => {
                    if let Some(Value::Ref(x)) = a.get(2) {
                        pd_formation.insert(id, *x);
                    }
                }
                "PRODUCT_DEFINITION_FORMATION" | "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE" => {
                    if let Some(Value::Ref(x)) = a.get(2) {
                        formation_product.insert(id, *x);
                    }
                }
                "PRODUCT" => {
                    let name = text(a.get(1));
                    product_name.insert(id, if name.is_empty() { text(a.first()) } else { name });
                }
                k if k.ends_with("REPRESENTATION") && !k.contains("RELATIONSHIP") => {
                    if let Some(Value::List(l, ln)) = a.get(1) {
                        let s: Vec<u32> = f
                            .run(*l, *ln)
                            .iter()
                            .filter_map(|v| if let Value::Ref(x) = v { Some(*x) } else { None })
                            .filter(|&x| is_solid(x))
                            .collect();
                        if !s.is_empty() {
                            solids_in.entry(id).or_default().extend(s);
                        }
                        rep_name.insert(id, text(a.first()));
                    }
                }
                _ => {}
            }
        }
    }
    if nauo.is_empty() && sdr.is_empty() {
        return Vec::new();
    }
    // each definition's representations; a representation's solids through untransformed links
    let mut reps_of: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &(p, s) in &sdr {
        if let Some(&d) = shape_of.get(&p) {
            reps_of.entry(d).or_default().push(s);
        }
    }
    let rep_solids = |start: u32| -> Vec<u32> {
        let (mut seen, mut todo, mut out) = (vec![start], vec![start], Vec::new());
        while let Some(x) = todo.pop() {
            out.extend(solids_in.get(&x).into_iter().flatten().copied());
            for &y in linked.get(&x).into_iter().flatten() {
                if !seen.contains(&y) {
                    seen.push(y);
                    todo.push(y);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    };
    let name_of = |d: u32| -> String {
        let p = pd_formation
            .get(&d)
            .and_then(|x| formation_product.get(x))
            .and_then(|x| product_name.get(x));
        match p {
            Some(n) if !n.is_empty() => n.clone(),
            _ => reps_of
                .get(&d)
                .and_then(|r| r.iter().find_map(|x| rep_name.get(x).filter(|n| !n.is_empty())))
                .cloned()
                .unwrap_or_else(|| format!("#{d}")),
        }
    };
    // each use: its child, and the child's placement in the parent
    let place_of: BTreeMap<u32, u32> = cdsr
        .iter()
        .filter_map(|&(rel, p)| Some((*shape_of.get(&p)?, rel)))
        .collect();
    let placement = |use_id: u32, child: u32| -> Option<Frame> {
        let rel = *place_of.get(&use_id)?;
        let rs = f.records_of(rel)?;
        let rr = rs
            .iter()
            .find(|r| f.name(r).ends_with("REPRESENTATION_RELATIONSHIP"))?;
        let rw = rs
            .iter()
            .find(|r| f.name(r) == "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION")?;
        let (rep1, _rep2) = match (f.args(rr).get(2), f.args(rr).get(3)) {
            (Some(Value::Ref(a)), Some(Value::Ref(b))) => (*a, *b),
            _ => return None,
        };
        let Some(Value::Ref(idt)) = f.args(rw).first() else {
            return None;
        };
        let t = refs(&rec(f, *idt, "ITEM_DEFINED_TRANSFORMATION")?);
        let (a1, a2) = (
            reader.placement(*t.first()?).ok()?,
            reader.placement(*t.get(1)?).ok()?,
        );
        // item 1 is in rep 1 and goes onto item 2 in rep 2; the child is whichever rep is its own
        let child_first = reps_of.get(&child).is_none_or(|r| r.contains(&rep1));
        Some(if child_first {
            a2.compose(&a1.inverse())
        } else {
            a1.compose(&a2.inverse())
        })
    };
    let children: BTreeMap<u32, Vec<(u32, u32)>> = nauo.iter().fold(BTreeMap::new(), |mut m, &(id, p, c)| {
        m.entry(p).or_default().push((id, c));
        m
    });
    let used: std::collections::BTreeSet<u32> = nauo.iter().map(|x| x.2).collect();
    let mut tops: Vec<u32> = reps_of
        .keys()
        .chain(children.keys())
        .copied()
        .filter(|d| !used.contains(d))
        .collect();
    tops.sort_unstable();
    tops.dedup();
    let mut out = Vec::new();
    // (definition, path, frame), depth bounded: a file can make its uses circular
    let mut todo: Vec<(u32, Vec<String>, Frame)> = tops
        .into_iter()
        .map(|d| (d, vec![name_of(d)], Frame::WORLD))
        .collect();
    while let Some((d, path, frame)) = todo.pop() {
        if out.len() > 1_000_000 || path.len() > 64 {
            break;
        }
        for &r in reps_of.get(&d).into_iter().flatten() {
            for s in rep_solids(r) {
                out.push(Occurrence {
                    solid: s,
                    path: path.clone(),
                    frame,
                });
            }
        }
        for &(id, c) in children.get(&d).into_iter().flatten() {
            let Some(p) = placement(id, c) else { continue };
            let mut q = path.clone();
            q.push(name_of(c));
            todo.push((c, q, frame.compose(&p)));
        }
    }
    out
}

/// Per-phase time of reads, summed over solids and threads, with the `times` feature (else
/// nothing is counted, and `timed` is its expression alone): research/import-speed/CHECKS.md.
pub mod times {
    /// The phases, in the order `finish` runs them.
    pub const NAMES: [&str; 8] = ["parse", "topology", "heal", "speck", "deviations", "mass", "mesh check", "display mesh"];
    /// Whether the clock is read at all (`counting`).
    #[cfg(feature = "times")]
    pub static ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

    /// Starts or stops counting (off until asked: the clock reads themselves cost something).
    pub fn counting(on: bool) {
        #[cfg(feature = "times")]
        ON.store(on, std::sync::atomic::Ordering::Relaxed);
        #[cfg(not(feature = "times"))]
        let _ = on;
    }

    #[cfg(feature = "times")]
    pub static NS: [std::sync::atomic::AtomicU64; 8] = [const { std::sync::atomic::AtomicU64::new(0) }; 8];

    /// `f()`, its wall time on this thread added to phase `k`.
    #[inline(always)]
    pub fn timed<T>(k: usize, f: impl FnOnce() -> T) -> T {
        #[cfg(feature = "times")]
        {
            if !ON.load(std::sync::atomic::Ordering::Relaxed) {
                return f();
            }
            let t = std::time::Instant::now();
            let x = f();
            NS[k].fetch_add(t.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);
            x
        }
        #[cfg(not(feature = "times"))]
        {
            let _ = k;
            f()
        }
    }

    /// Each phase's seconds so far, and the counts set back to 0.
    pub fn take() -> [f64; 8] {
        #[cfg(feature = "times")]
        {
            NS.each_ref().map(|n| n.swap(0, std::sync::atomic::Ordering::Relaxed) as f64 * 1e-9)
        }
        #[cfg(not(feature = "times"))]
        {
            [0.0; 8]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healing_never_moves_an_edge_far_on_a_small_part() {
        assert_eq!(heal_limit(0.005, 1.0), 0.002);
        assert_eq!(heal_limit(0.005, 1000.0), 0.1);
        assert_eq!(heal_limit(1e-7, 1000.0), 0.02);
    }

    /// A rational curve's net is the curve's: its points each their own, as the weights expect.
    #[test]
    fn an_arcs_net_stays_on_its_circle() {
        let (r, a) = (6.0, 0.5);
        let s = Spline::ellipse(Point3::ORIGIN, Vec3::X * r, Vec3::Y * r, a, a).unwrap();
        let (deg, knots, pts, w) = net_of(&s);
        let back = Spline::from_nurbs(deg, &knots, &pts, Some(&w)).unwrap();
        let c = SplineCurve::curve(std::sync::Arc::new(back), s.domain());
        for k in 0..=16 {
            let p = c.eval(s.domain().lo + s.domain().width() * k as f64 / 16.0);
            assert!((Vec3::new(p.x, p.y, p.z).length() - r).abs() < 1e-12, "{p:?}");
        }
    }
}
