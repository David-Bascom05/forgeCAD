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
    pub schema: String,
    pub system: String,
    pub stats: Stats,
}

#[derive(Debug)]
pub enum ImportError {
    Parse(ParseError),
    /// A `.brep` that does not read as one.
    Brep,
    NoSolids,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::Parse(e) => write!(f, "{e}"),
            ImportError::Brep => write!(f, "not a readable OpenCascade .brep file"),
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

fn positive(x: f64) -> bool {
    x > 0.0
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
        if !forge_math::tol::in_model_box(p) {
            return refuse(id, "CARTESIAN_POINT", "outside the 1 km model box");
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
        let a = self.args_of(id, "AXIS2_PLACEMENT_3D")?;
        let origin = self.point(self.reference(id, "AXIS2_PLACEMENT_3D", a.get(1))?)?;
        let z = match a.get(2) {
            Some(Value::Ref(r)) => Some(self.direction(*r)?),
            _ => None,
        };
        let x = match a.get(3) {
            Some(Value::Ref(r)) => Some(self.direction(*r)?),
            _ => None,
        };
        let z = z.unwrap_or(UnitVec3::Z);
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
                let origin = self.point(self.reference(id, "LINE", a.get(1))?)?;
                let v = self.reference(id, "LINE", a.get(2))?;
                let va = self.args_of(v, "VECTOR")?;
                let dir = self.direction(self.reference(v, "VECTOR", va.get(1))?)?;
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
                Ok(Curve::Ellipse(Ellipse { frame, major, minor }))
            }
            "PARABOLA" => {
                let a = self.args_of(id, "PARABOLA")?;
                let frame = self.placement(self.reference(id, "PARABOLA", a.get(1))?)?;
                let focal = self.real(id, "PARABOLA", a.get(2))? * m;
                if focal == 0.0 {
                    return refuse(id, "PARABOLA", "no focal length");
                }
                Ok(Curve::Parabola(Parabola { frame, focal }))
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
            "PLANE" => {
                let a = self.args_of(id, name)?;
                Ok(Surface::Plane(Plane {
                    frame: self.placement(self.reference(id, name, a.get(1))?)?,
                }))
            }
            "CYLINDRICAL_SURFACE" => {
                let a = self.args_of(id, name)?;
                let frame = self.placement(self.reference(id, name, a.get(1))?)?;
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
                // a spindle torus: its outer or inner part, a circle turned about the axis
                let outer = a.get(4).and_then(|v| logical(f, v)).unwrap_or(true);
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
    stats: &'b mut Stats,
    salt: Name,
}

impl Build<'_, '_> {
    fn vertex(&mut self, id: u32) -> R<usize> {
        if let Some(&v) = self.verts.get(&id) {
            return Ok(v);
        }
        let a = self.r.args_of(id, "VERTEX_POINT")?;
        let p = self.r.point(self.r.reference(id, "VERTEX_POINT", a.get(1))?)?;
        self.p.verts.push((p, self.salt.with(u64::from(id))));
        let v = self.p.verts.len() - 1;
        self.verts.insert(id, v);
        Ok(v)
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
        let a = self.r.args_of(id, "CLOSED_SHELL")?;
        let faces = self.r.list(id, "CLOSED_SHELL", a.get(1))?;
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
fn edge_range(c: &Curve, a: Point3, b: Point3, closed: bool) -> (Interval, f64, bool) {
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
                let t = |x: Point3| {
                    if near(x, e0) {
                        d.lo
                    } else if near(x, e1) {
                        d.hi
                    } else {
                        c.closest(x, whole)
                    }
                };
                let (t0, t1) = (t(a), t(b));
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
    for e in body.edge_ids().collect::<Vec<_>>() {
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
        let measure = |n: usize| {
            let pts = body.edge_points(e, n);
            faces
                .iter()
                .map(|&f| {
                    let s = &body.face(f).surface;
                    // each point from the one before, where the walk holds
                    let mut near = None;
                    let mut worst = 0.0_f64;
                    for &p in &pts {
                        let uv = near.map_or_else(|| s.project(p), |uv| s.project_near(p, uv));
                        near = Some(uv);
                        worst = worst.max(s.eval(uv).distance(p));
                    }
                    worst
                })
                .fold(0.0, f64::max)
        };
        let dev = measure(16);
        let dev = if free_form || dev > LINEAR {
            dev.max(measure(256))
        } else {
            dev
        };
        if dev > LINEAR {
            body.edges[e.index()].tol = dev;
            stats.tolerant_edges += 1;
            stats.max_edge_tol = stats.max_edge_tol.max(dev);
        }
    }
}

/// How far healing may move an edge: from a file's stated uncertainty, but never much of the part.
/// A normalised part keeps the figure of a far larger original, as much as a tenth of its size, and
/// an edge then went to where two other planes meet, further off than it was long.
fn heal_limit(uncertainty: f64, size: f64) -> f64 {
    (uncertainty.max(1e-3) * 20.0).min(2e-3 * size)
}

const TOO_LONG: &str = "its surfaces would take too long to measure";

/// A body whose integrals and mesh disagree is refused: on a file's geometry the integrals can go
/// wrong with nothing to show for it (`research/step/README.md`). So is one that cannot be meshed
/// even coarsely, with nothing to check it by and nothing to show, and so is one whose integrals
/// cannot answer.
fn measures_agree(body: &Body) -> Result<(), String> {
    let m = forge_topo::mass::mass_rel(body, 1e-6);
    if forge_geom::budget::over() {
        return Err(TOO_LONG.into());
    }
    // nothing to check it by, or to tell its mass by: no pass
    let m = m.map_err(|e| format!("its volume cannot be worked out: {}", e.why))?;
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
    let (mesh, chord) = match mesh_at(5e-4 * diagonal) {
        Ok(x) => (x, 5e-4 * diagonal),
        Err(_) => match mesh_at(5e-3 * diagonal) {
            Ok(x) => (x, 5e-3 * diagonal),
            Err(_) if forge_geom::budget::over() => return Err(TOO_LONG.into()),
            Err(e) => return Err(format!("it cannot be meshed: {e:?}")),
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
    if (m.volume - volume).abs() > 0.01 * volume.abs() + 0.5 * curved * chord
        || (m.area - area).abs() > 0.01 * area
    {
        return Err(format!(
            "its volume and area integrate to {} and {}, its mesh says {volume} and {area}",
            m.volume, m.area
        ));
    }
    Ok(())
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

/// How far a file's geometry misses itself, before anything is changed.
fn measure_gaps(proto: &Proto, stats: &mut Stats) {
    for fa in &proto.faces {
        for u in fa.loops.iter().flatten() {
            let e = &proto.edges[u.0];
            let w = e.range.width();
            let worst = (0..=8)
                .map(|k| fa.surface.distance(e.curve.eval(e.range.lo + w * k as f64 / 8.0)))
                .fold(0.0, f64::max);
            stats.curve_off_surface = stats.curve_off_surface.max(worst);
            if worst > forge_math::tol::LINEAR {
                stats.edges_off += 1;
            }
            let kind = |s: &str| s.split(['(', ' ']).next().unwrap_or("").to_string();
            let key = format!(
                "{}/{}",
                kind(&format!("{:?}", e.curve).chars().take(12).collect::<String>()),
                kind(&format!("{:?}", fa.surface).chars().take(12).collect::<String>())
            );
            let en = stats.pairs.entry(key).or_insert((0, 0, 0.0));
            en.0 += 1;
            en.1 += usize::from(worst > forge_math::tol::LINEAR);
            en.2 = en.2.max(worst);
        }
    }
}

/// From a read file's topology to a body: seams, edges tightened, checked; healing and the check on
/// one budget. `uncertainty`: what the file says of itself.
pub(crate) fn finish(
    mut proto: Proto,
    uncertainty: f64,
    lim: &Limits,
    stats: &mut Stats,
) -> Result<Body, String> {
    if proto.faces.is_empty() {
        return Err("a solid with no faces".into());
    }
    measure_gaps(&proto, stats);
    forge_geom::budget::begin(lim.max_evals);
    proto.split_closed_splines();
    if let Err(why) = proto.add_seams() {
        forge_geom::budget::end();
        return Err(why);
    }
    let size = Box3::from_points(proto.verts.iter().map(|v| v.0)).diagonal();
    proto.tighten(heal_limit(uncertainty, size));
    stats.tangent_off += proto.tangent;
    proto.split_fans();
    proto.outer_first();
    let mut body = proto.into_body();
    record_deviations(&mut body, stats);
    let agree = measures_agree(&body);
    forge_geom::budget::end();
    agree?;
    Ok(body)
}

/// Reads a whole file. Parts that cannot be read become refusals; the others are returned.
pub fn import(text: &[u8], lim: &Limits) -> Result<Imported, ImportError> {
    if crate::brep::is_brep(text) {
        return crate::brep::import(text, lim);
    }
    read_file(text, lim, false)
}

/// The most any edge of a file lies off a face it bounds, by the same measure a read reports.
pub(crate) fn curve_off_surface(text: &[u8], lim: &Limits) -> Result<f64, ImportError> {
    Ok(read_file(text, lim, true)?.stats.curve_off_surface)
}

/// `only_measure`: as far as what the file's geometry misses by, and no bodies.
fn read_file(text: &[u8], lim: &Limits, only_measure: bool) -> Result<Imported, ImportError> {
    let f = parse(text, lim).map_err(ImportError::Parse)?;
    let mut stats = Stats::default();
    let mut refusals = Vec::new();
    let mut solids = Vec::new();
    // each solid's context: from the representation that lists it
    let mut ctx_of: BTreeMap<u32, u32> = BTreeMap::new();
    let mut any_ctx = None;
    let mut roots = Vec::new();
    for &(id, at, n) in &f.ids {
        let recs = &f.records[at as usize..(at + n) as usize];
        for r in recs {
            let name = f.name(r);
            if name == "MANIFOLD_SOLID_BREP" || name == "BREP_WITH_VOIDS" {
                roots.push((id, *r));
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
    if let Some(u) = any_ctx.and_then(|c| units_of(&f, c)) {
        stats.assembly = assembly_asks(&f, &Reader { f: &f, u, depth: 0 });
    }
    for (id, r) in roots {
        let ctx = ctx_of.get(&id).copied().or(any_ctx);
        let Some(u) = ctx.and_then(|c| units_of(&f, c)) else {
            refusals.push(Refusal {
                entity: id,
                kind: f.name(&r).to_string(),
                why: "its units cannot be read".into(),
            });
            continue;
        };
        stats.uncertainty = u.uncertainty;
        stats.units_mm = u.mm;
        let mut reader = Reader { f: &f, u, depth: 0 };
        let name = match f.args(&r).first() {
            Some(Value::Str(a, n)) => f.str_of(*a, *n).to_string(),
            _ => String::new(),
        };
        let mut b = Build {
            r: &mut reader,
            p: Proto {
                salt: Name::feature(u64::from(id)).with_str("step"),
                ..Proto::default()
            },
            verts: BTreeMap::new(),
            edges: BTreeMap::new(),
            stats: &mut stats,
            salt: Name::feature(u64::from(id)).with_str("step"),
        };
        let built = (|| {
            let a = f.args(&r);
            let outer = b.r.reference(id, "solid", a.get(1))?;
            b.shell(outer, false)?;
            if f.name(&r) == "BREP_WITH_VOIDS" {
                for v in b.r.list(id, "solid", a.get(2))? {
                    let vid = b.r.reference(id, "solid", Some(v))?;
                    b.shell(vid, false)?;
                }
            }
            Ok(())
        })();
        let proto = b.p;
        match built {
            Ok(()) => {
                if only_measure {
                    measure_gaps(&proto, &mut stats);
                    continue;
                }
                match finish(proto, u.uncertainty, lim, &mut stats) {
                    Ok(mut body) => {
                        body.unplaced = stats.assembly > 0;
                        solids.push(Solid {
                            entity: id,
                            name,
                            body,
                        })
                    }
                    Err(why) => refusals.push(Refusal {
                        entity: id,
                        kind: f.name(&r).to_string(),
                        why,
                    }),
                }
            }
            Err(x) => refusals.push(x),
        }
    }
    if solids.is_empty() && refusals.is_empty() && !only_measure {
        return Err(ImportError::NoSolids);
    }
    Ok(Imported {
        solids,
        refusals,
        schema: f.schema.clone(),
        system: f.system.clone(),
        stats,
    })
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
