//! A transmit file's curves and surfaces as the kernel's (XT Format Reference 5.2). Lengths are
//! metres in the file, mm here; an instance's transform is applied as each is made.
//!
//! Each surface comes back with whether its natural normal agrees with the file's oriented one
//! (the natural normal times the surface's sense, 5.2.8): a face's normal is then the kernel
//! surface's when that agrees with the face's own sense.

use super::nodes::{Node, Store};
use super::spcurve::OnSurface;
use crate::read::{Refusal, extruded, net_of_curve, turned};
use crate::nurbs::Nurbs;
use forge_geom::blend::{Blend, Pipe, foot, offset};
use forge_geom::intcurve::{IntCurve, direction, refine};
use forge_geom::spline::{Spline, SplineCurve};
use forge_geom::{Circle, Cone, Curve, CurveGeom, Cylinder, Ellipse, Line, Plane, Sphere, Surface, Torus};
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec3, dmath, tol};
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) type R<T> = Result<T, Refusal>;

/// The node type's name in the reference, for a refusal to give.
pub(crate) fn type_name(ty: u16) -> &'static str {
    match ty {
        10 => "ASSEMBLY",
        11 => "INSTANCE",
        12 => "BODY",
        13 => "SHELL",
        14 => "FACE",
        15 => "LOOP",
        16 => "EDGE",
        17 => "FIN",
        18 => "VERTEX",
        19 => "REGION",
        29 => "POINT",
        30 => "LINE",
        31 => "CIRCLE",
        32 => "ELLIPSE",
        38 => "INTERSECTION",
        40 => "CHART",
        41 => "LIMIT",
        45 => "BSPLINE_VERTICES",
        50 => "PLANE",
        51 => "CYLINDER",
        52 => "CONE",
        53 => "SPHERE",
        54 => "TORUS",
        56 => "BLENDED_EDGE",
        59 => "BLEND_BOUND",
        60 => "OFFSET_SURF",
        67 => "SWEPT_SURF",
        68 => "SPUN_SURF",
        100 => "TRANSFORM",
        120 => "PE_SURF",
        124 => "B_SURFACE",
        126 => "NURBS_SURF",
        130 => "PE_CURVE",
        133 => "TRIMMED_CURVE",
        134 => "B_CURVE",
        136 => "NURBS_CURVE",
        137 => "SP_CURVE",
        200 => "POLYLINE",
        201 => "MESH",
        _ => "node",
    }
}

pub(crate) fn refuse<T>(n: u32, ty: u16, why: impl Into<String>) -> R<T> {
    Err(Refusal {
        entity: n,
        kind: type_name(ty).to_string(),
        why: why.into(),
    })
}

/// A placement: p' = k (R p + t), k turning metres to mm and holding the transform's scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Xf {
    pub r: [[f64; 3]; 3],
    pub t: [f64; 3],
    pub k: f64,
}

impl Xf {
    /// Metres to mm, in place.
    pub(crate) const MM: Xf = Xf {
        r: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        t: [0.0; 3],
        k: 1000.0,
    };

    fn rot(&self, v: [f64; 3]) -> [f64; 3] {
        let r = &self.r;
        [
            r[0][0] * v[0] + r[0][1] * v[1] + r[0][2] * v[2],
            r[1][0] * v[0] + r[1][1] * v[1] + r[1][2] * v[2],
            r[2][0] * v[0] + r[2][1] * v[1] + r[2][2] * v[2],
        ]
    }

    pub(crate) fn point(&self, p: [f64; 3]) -> Point3 {
        let q = self.rot(p);
        Point3::new(
            self.k * (q[0] + self.t[0]),
            self.k * (q[1] + self.t[1]),
            self.k * (q[2] + self.t[2]),
        )
    }

    pub(crate) fn vec(&self, v: [f64; 3]) -> Vec3 {
        let q = self.rot(v);
        Vec3::new(q[0], q[1], q[2])
    }

    pub(crate) fn len(&self, x: f64) -> f64 {
        x * self.k
    }

    /// `inner` placed within self: self(inner(p)).
    pub(crate) fn then(&self, inner: &Xf) -> Xf {
        // self(k_i (R_i p + t_i)) = k_s (R_s k_i (R_i p + t_i) + t_s); kept as k (R p + t) with
        // k = k_s k_i: R = R_s R_i, t = R_s t_i + t_s / k_i
        let mut r = [[0.0; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, x) in row.iter_mut().enumerate() {
                *x = (0..3).map(|m| self.r[i][m] * inner.r[m][j]).sum();
            }
        }
        let rt = self.rot(inner.t);
        Xf {
            r,
            t: [
                rt[0] + self.t[0] / inner.k,
                rt[1] + self.t[1] / inner.k,
                rt[2] + self.t[2] / inner.k,
            ],
            k: self.k * inner.k,
        }
    }

    /// The rotation's determinant, near 1 for a rigid placement.
    pub(crate) fn det(&self) -> f64 {
        let r = &self.r;
        r[0][0] * (r[1][1] * r[2][2] - r[1][2] * r[2][1]) - r[0][1] * (r[1][0] * r[2][2] - r[1][2] * r[2][0])
            + r[0][2] * (r[1][0] * r[2][1] - r[1][1] * r[2][0])
    }

    /// Whether R is a rotation: orthonormal, determinant 1, to `eps`.
    pub(crate) fn rigid(&self, eps: f64) -> bool {
        let r = &self.r;
        for i in 0..3 {
            for j in 0..3 {
                let d: f64 = (0..3).map(|m| r[m][i] * r[m][j]).sum();
                if (d - if i == j { 1.0 } else { 0.0 }).abs() > eps {
                    return false;
                }
            }
        }
        (self.det() - 1.0).abs() <= eps
    }
}

/// Curves and surfaces made so far, by node index, under one placement.
pub(crate) struct Geo<'s> {
    pub s: &'s Store,
    pub xf: Xf,
    curves: HashMap<u32, Curve>,
    surfaces: HashMap<u32, (Surface, bool)>,
    blends: HashMap<u32, Blend>,
    depth: u32,
}

const MAX_DEPTH: u32 = 24;

fn sense(n: &Node) -> bool {
    n.c("sense") != b'-'
}

impl<'s> Geo<'s> {
    pub(crate) fn new(s: &'s Store, xf: Xf) -> Geo<'s> {
        Geo {
            s,
            xf,
            curves: HashMap::new(),
            surfaces: HashMap::new(),
            blends: HashMap::new(),
            depth: 0,
        }
    }

    fn node(&self, at: u32, idx: u32, what: &str) -> R<Node<'s>> {
        match self.s.node(idx) {
            Some(n) if idx != 0 => Ok(n),
            _ => Err(Refusal {
                entity: at,
                kind: "node".into(),
                why: format!("{what} {idx} is missing"),
            }),
        }
    }

    fn vec(&self, n: &Node, name: &str) -> R<[f64; 3]> {
        match n.v(name) {
            Some(v) => Ok(v),
            None => refuse(n.index, n.ty(), format!("its {name} is null")),
        }
    }

    fn point(&self, n: &Node, name: &str) -> R<Point3> {
        Ok(self.xf.point(self.vec(n, name)?))
    }

    fn dir(&self, n: &Node, name: &str) -> R<UnitVec3> {
        match self.xf.vec(self.vec(n, name)?).normalize() {
            Some(d) => Ok(d),
            None => refuse(n.index, n.ty(), format!("its {name} has no length")),
        }
    }

    fn real(&self, n: &Node, name: &str) -> R<f64> {
        let x = n.f(name);
        if x.is_finite() { Ok(x) } else { refuse(n.index, n.ty(), format!("its {name} is null")) }
    }

    fn frame(&self, n: &Node, o: &str, z: &str, x: &str) -> R<Frame> {
        let (o, z, x) = (self.point(n, o)?, self.dir(n, z)?, self.xf.vec(self.vec(n, x)?));
        match Frame::from_zx(o, z, x) {
            Some(f) => Ok(f),
            None => refuse(n.index, n.ty(), "its x axis lies along its axis"),
        }
    }

    /// A surface node as the kernel's, and whether the kernel's natural normal agrees with the
    /// file's oriented one.
    pub(crate) fn surface(&mut self, idx: u32) -> R<(Surface, bool)> {
        if let Some(s) = self.surfaces.get(&idx) {
            return Ok(s.clone());
        }
        if self.depth > MAX_DEPTH {
            return refuse(idx, 0, "surfaces defined on one another too deep");
        }
        self.depth += 1;
        let s = self.surface_inner(idx);
        self.depth -= 1;
        let s = s?;
        self.surfaces.insert(idx, s.clone());
        Ok(s)
    }

    fn surface_inner(&mut self, idx: u32) -> R<(Surface, bool)> {
        let n = self.node(idx, idx, "the surface")?;
        let ty = n.ty();
        let k = self.xf.k;
        // whether the kernel's natural normal is the file's natural normal
        let (s, natural) = match ty {
            50 => (
                Surface::Plane(Plane {
                    frame: self.frame(&n, "pvec", "normal", "x_axis")?,
                }),
                true,
            ),
            51 => {
                let radius = self.real(&n, "radius")? * k;
                if radius <= tol::LINEAR {
                    return refuse(idx, ty, "radius not positive");
                }
                (
                    Surface::Cylinder(Cylinder {
                        frame: self.frame(&n, "pvec", "axis", "x_axis")?,
                        radius,
                    }),
                    true,
                )
            }
            52 => {
                // the files widen their cones along the stored axis (the corpus's charts say so;
                // the reference's "away from the half in use" is the PK interface's axis)
                let frame = self.frame(&n, "pvec", "axis", "x_axis")?;
                let radius = self.real(&n, "radius")? * k;
                let half = dmath::atan2(self.real(&n, "sin_half_angle")?, self.real(&n, "cos_half_angle")?);
                match Cone::new(frame, radius.max(0.0), half) {
                    Some(c) => (Surface::Cone(c), true),
                    None => return refuse(idx, ty, format!("radius {radius}, half angle {half}")),
                }
            }
            53 => {
                let radius = self.real(&n, "radius")? * k;
                if radius <= tol::LINEAR {
                    return refuse(idx, ty, "radius not positive");
                }
                (
                    Surface::Sphere(Sphere {
                        frame: self.frame(&n, "centre", "axis", "x_axis")?,
                        radius,
                    }),
                    true,
                )
            }
            54 => {
                let frame = self.frame(&n, "centre", "axis", "x_axis")?;
                let (a, b) = (
                    self.real(&n, "major_radius")? * k,
                    self.real(&n, "minor_radius")? * k,
                );
                // doughnut, apple (a <= b) and lemon (a < 0) alike: C + X(a + b cos v) + b A sin v
                let t = Torus::new(frame, a, b).or_else(|| Torus::spindle(frame, a, b));
                match t {
                    Some(t) => (Surface::Torus(t), true),
                    None => return refuse(idx, ty, format!("major {a}, minor {b}")),
                }
            }
            124 => (self.bsurface(&n)?, true),
            60 => {
                let base = n.p("surface");
                let (b, agree) = self.surface(base)?;
                let d = self.real(&n, "offset")? * k;
                // along the base's oriented normal, which the kernel's natural one is when they agree
                let d = if agree { d } else { -d };
                match offset(&b, d) {
                    Some(o) => {
                        // the offset keeps its base's parameters and natural normal: it agrees as
                        // its base does, by the senses of both
                        let base_sense = self.s.node(base).is_some_and(|b| sense(&b));
                        let natural_base = agree == base_sense;
                        (o, natural_base)
                    }
                    None => return refuse(idx, ty, "the offset closes the surface up"),
                }
            }
            67 => {
                let c = self.curve(n.p("section"))?;
                let dir = self.dir(&n, "sweep")?;
                let net = net_of_curve(&c).ok_or(Refusal {
                    entity: idx,
                    kind: "SWEPT_SURF".into(),
                    why: "its section has no spline form".into(),
                })?;
                match extruded(net, dir.into()) {
                    Some(s) => (s, true),
                    None => return refuse(idx, ty, "the sweep makes no surface"),
                }
            }
            68 => {
                let c = self.curve(n.p("profile"))?;
                let (o, z) = (self.point(&n, "base")?, self.dir(&n, "axis")?);
                let net = net_of_curve(&c).ok_or(Refusal {
                    entity: idx,
                    kind: "SPUN_SURF".into(),
                    why: "its profile has no spline form".into(),
                })?;
                // the kernel's u is the angle and v the profile's: the natural normal turns over
                match turned((o, z.into()), net, 1e-7 * k) {
                    Some(s) => (s, false),
                    None => return refuse(idx, ty, "the spin makes no surface"),
                }
            }
            56 => {
                let (b, natural) = self.blend(&n)?;
                (b.surface.clone(), natural)
            }
            59 => {
                return refuse(
                    idx,
                    ty,
                    "a blend's boundary surface has no form of its own; only its edge curves are read",
                );
            }
            120 => return refuse(idx, ty, "foreign geometry: the file's evaluator is not here"),
            201 => return refuse(idx, ty, "a mesh surface: only exact faces are read"),
            other => return refuse(idx, other, "not a surface this reads"),
        };
        Ok((s, natural == sense(&n)))
    }

    /// Expanded knots from distinct knots and their multiplicities.
    /// `n` distinct knots expanded by their multiplicities. Writers may leave the arrays longer
    /// than `n`, the rest zeros or null.
    fn knots(&self, at: u32, ty: u16, n: f64, mult: u32, knots: u32) -> R<Vec<f64>> {
        let m = self.node(at, mult, "the knot multiplicities")?;
        let kn = self.node(at, knots, "the knots")?;
        let (m, kn) = (m.all("mult"), kn.all("knots"));
        if !(n.is_finite() && n >= 2.0 && n as usize <= m.len().min(kn.len())) {
            return refuse(at, ty, format!("{n} distinct knots, of {} and {} given", kn.len(), m.len()));
        }
        let (m, kn) = (&m[..n as usize], &kn[..n as usize]);
        let total: f64 = m.iter().sum();
        if !(total.is_finite() && total <= 1e7) {
            return refuse(at, ty, "knot multiplicities out of range");
        }
        let mut out = Vec::with_capacity(total as usize);
        for (k, &c) in kn.iter().zip(m) {
            if !(c >= 1.0) || !k.is_finite() {
                return refuse(at, ty, "a knot or multiplicity is null");
            }
            out.extend(std::iter::repeat_n(*k, c as usize));
        }
        Ok(out)
    }

    /// Control points as `dim`-vectors (homogeneous when rational: the last value the weight).
    fn poles(&self, at: u32, ty: u16, v: u32, n: usize, dim: usize) -> R<&'s [f64]> {
        let vx = self.node(at, v, "the control vertices")?;
        let a = vx.all("vertices");
        match a.get(..n * dim) {
            Some(a) if a.iter().all(|x| x.is_finite()) => Ok(a),
            _ => refuse(at, ty, format!("{} control values for {n} vertices of {dim}", a.len())),
        }
    }

    fn bsurface(&mut self, n: &Node) -> R<Surface> {
        let (idx, ty) = (n.index, n.ty());
        let ns = self.node(idx, n.p("nurbs"), "the NURBS data")?;
        let (p, q) = (ns.f("u_degree"), ns.f("v_degree"));
        let (nu, nv) = (ns.f("n_u_vertices"), ns.f("n_v_vertices"));
        let dim = ns.f("vertex_dim");
        let rational = ns.f("rational") == 1.0;
        let ok = |x: f64| x.is_finite() && x >= 1.0 && x <= 1e7;
        if !(ok(p) && ok(q) && ok(nu) && ok(nv) && ok(dim)) {
            return refuse(idx, ty, "degree or vertex count out of range");
        }
        let (p, q, nu, nv, dim) = (p as usize, q as usize, nu as usize, nv as usize, dim as usize);
        if dim != if rational { 4 } else { 3 } {
            return refuse(idx, ty, format!("vertices of {dim} values"));
        }
        let u = self.knots(idx, ty, ns.f("n_u_knots"), ns.p("u_knot_mult"), ns.p("u_knots"))?;
        let v = self.knots(idx, ty, ns.f("n_v_knots"), ns.p("v_knot_mult"), ns.p("v_knots"))?;
        let a = self.poles(idx, ty, ns.p("bspline_vertices"), nu * nv, dim)?;
        // the file's vertices run with v fastest: rows of constant u, as the kernel keeps them
        let mut pts = Vec::with_capacity(nu * nv);
        let mut ws = Vec::with_capacity(nu * nv);
        for c in a.chunks_exact(dim) {
            let w = if rational { c[3] } else { 1.0 };
            if !(w > 0.0) {
                return refuse(idx, ty, "a weight not positive");
            }
            pts.push(self.xf.point([c[0] / w, c[1] / w, c[2] / w]));
            ws.push(w);
        }
        match Nurbs::new((p, q), (u, v), (nu, nv), &pts, rational.then_some(&ws[..])) {
            Some(s) => Ok(s.surface()),
            None => refuse(idx, ty, "knots, poles and degrees do not make a surface"),
        }
    }

    /// A NURBS_CURVE's spline: in space (dim 3, 4 rational) or, for `flat`, a surface's
    /// parameters as x and y (dim 2, 3 rational), unplaced.
    fn nurbs_curve(&self, at: u32, nc_idx: u32, flat: bool) -> R<Spline> {
        let ty = 136;
        let nc = self.node(at, nc_idx, "the NURBS data")?;
        let (deg, n, dim) = (nc.f("degree"), nc.f("n_vertices"), nc.f("vertex_dim"));
        let rational = nc.f("rational") == 1.0;
        let ok = |x: f64| x.is_finite() && x >= 1.0 && x <= 1e7;
        if !(ok(deg) && ok(n) && ok(dim)) {
            return refuse(at, ty, "degree or vertex count out of range");
        }
        let (deg, n, dim) = (deg as usize, n as usize, dim as usize);
        let want = if flat { 2 } else { 3 } + usize::from(rational);
        if dim != want {
            return refuse(at, ty, format!("vertices of {dim} values"));
        }
        let knots = self.knots(at, ty, nc.f("n_knots"), nc.p("knot_mult"), nc.p("knots"))?;
        let a = self.poles(at, ty, nc.p("bspline_vertices"), n, dim)?;
        let mut pts = Vec::with_capacity(n);
        let mut ws = Vec::with_capacity(n);
        for c in a.chunks_exact(dim) {
            let w = if rational { c[dim - 1] } else { 1.0 };
            if !(w > 0.0) {
                return refuse(at, ty, "a weight not positive");
            }
            pts.push(if flat {
                Point3::new(c[0] / w, c[1] / w, 0.0)
            } else {
                self.xf.point([c[0] / w, c[1] / w, c[2] / w])
            });
            ws.push(w);
        }
        match Spline::from_nurbs(deg, &knots, &pts, rational.then_some(&ws[..])) {
            Some(s) => Ok(s),
            None => refuse(at, ty, "knots, poles and degree do not make a curve"),
        }
    }

    pub(crate) fn curve(&mut self, idx: u32) -> R<Curve> {
        if let Some(c) = self.curves.get(&idx) {
            return Ok(c.clone());
        }
        if self.depth > MAX_DEPTH {
            return refuse(idx, 0, "curves defined on one another too deep");
        }
        self.depth += 1;
        let c = self.curve_inner(idx);
        self.depth -= 1;
        let c = c?;
        self.curves.insert(idx, c.clone());
        Ok(c)
    }

    fn curve_inner(&mut self, idx: u32) -> R<Curve> {
        let n = self.node(idx, idx, "the curve")?;
        let ty = n.ty();
        let k = self.xf.k;
        match ty {
            30 => Ok(Curve::Line(Line {
                origin: self.point(&n, "pvec")?,
                dir: self.dir(&n, "direction")?,
            })),
            31 => {
                let radius = self.real(&n, "radius")? * k;
                if radius <= tol::LINEAR {
                    return refuse(idx, ty, "radius not positive");
                }
                Ok(Curve::Circle(Circle {
                    frame: self.frame(&n, "centre", "normal", "x_axis")?,
                    radius,
                }))
            }
            32 => {
                let (a, b) = (
                    self.real(&n, "major_radius")? * k,
                    self.real(&n, "minor_radius")? * k,
                );
                if !(b > tol::LINEAR && a >= b) {
                    return refuse(idx, ty, format!("radii {a} and {b}"));
                }
                Ok(Curve::Ellipse(Ellipse {
                    frame: self.frame(&n, "centre", "normal", "x_axis")?,
                    major: a,
                    minor: b,
                }))
            }
            134 => {
                let s = self.nurbs_curve(idx, n.p("nurbs"), false)?;
                let d = s.domain();
                Ok(SplineCurve::curve(Arc::new(s), d))
            }
            133 => self.curve(n.p("basis_curve")),
            137 => self.sp_curve(&n),
            38 => self.intersection(&n),
            130 => refuse(idx, ty, "foreign geometry: the file's evaluator is not here"),
            200 => refuse(idx, ty, "a polyline: only exact edges are read"),
            other => refuse(idx, other, "not a curve this reads"),
        }
    }

    /// A 2D B-curve in a surface's parameters, as the curve in space it maps to.
    fn sp_curve(&mut self, n: &Node) -> R<Curve> {
        let (idx, ty) = (n.index, n.ty());
        let bc = self.node(idx, n.p("b_curve"), "the 2D B-curve")?;
        if bc.ty() != 134 {
            return refuse(idx, ty, "its 2D curve is not a B-curve");
        }
        let flat = self.nurbs_curve(idx, bc.p("nurbs"), true)?;
        let Some(param) = self.param_of(n.p("surface"))? else {
            return refuse(idx, ty, "a curve in the parameters of a surface this cannot evaluate there");
        };
        match OnSurface::curve(param, flat) {
            Some(c) => Ok(c),
            None => refuse(idx, ty, "the 2D curve leaves its surface"),
        }
    }

    /// How to find a file surface's point at its own parameters: the kernel surface at them
    /// mapped (lengths are metres there, mm here), or the file's definition (swept, spun).
    fn param_of(&mut self, sidx: u32) -> R<Option<Param>> {
        let k = self.xf.k;
        let Some(n) = self.s.node(sidx) else { return Ok(None) };
        let map = match n.ty() {
            // P + uX + vY in metres
            50 => [[k, 0.0], [0.0, k]],
            // u the angle, v along the axis in metres
            51 | 52 => [[1.0, 0.0], [0.0, k]],
            // angles, and a B-surface's own parameters
            53 | 54 | 124 => [[1.0, 0.0], [0.0, 1.0]],
            // an offset keeps its base's parameters
            60 => {
                let base = self.param_of(n.p("surface"))?;
                let (s, _) = self.surface(sidx)?;
                return Ok(match base {
                    Some(Param::Map(_, m)) => Some(Param::Map(s, m)),
                    _ => None,
                });
            }
            67 => {
                let Some((c, cs)) = self.in_file_param(n.p("section"))? else { return Ok(None) };
                let d = self.dir(&n, "sweep")?;
                return Ok(Some(Param::Swept { c, cs, d: d.into(), k }));
            }
            68 => {
                let Some((c, cs)) = self.in_file_param(n.p("profile"))? else { return Ok(None) };
                let (p, a) = (self.point(&n, "base")?, self.dir(&n, "axis")?);
                return Ok(Some(Param::Spun { c, cs, p, a: a.into() }));
            }
            _ => return Ok(None),
        };
        let (s, _) = self.surface(sidx)?;
        Ok(Some(Param::Map(s, map)))
    }

    /// A curve and the factor from the file's parameter on it to the kernel's: lines' run in
    /// metres there and mm here; conics' are angles and splines' their knots in both. None for a
    /// curve whose file parameter the kernel's does not follow.
    fn in_file_param(&mut self, idx: u32) -> R<Option<(Curve, f64)>> {
        let Some(n) = self.s.node(idx) else { return Ok(None) };
        let cs = match n.ty() {
            30 => self.xf.k,
            31 | 32 | 134 => 1.0,
            133 => return self.in_file_param(n.p("basis_curve")),
            _ => return Ok(None),
        };
        Ok(Some((self.curve(idx)?, cs)))
    }

    /// An intersection curve: the chart's points on both surfaces, a terminator past an end
    /// added; or a blend's boundary, where one surface is a BLEND_BOUND: the blend's spring curve.
    fn intersection(&mut self, n: &Node) -> R<Curve> {
        let (idx, ty) = (n.index, n.ty());
        let surf = n.all("surface");
        let (sa, sb) = match surf {
            [a, b] => (*a as u32, *b as u32),
            _ => return refuse(idx, ty, "not two surfaces"),
        };
        let chart = self.node(idx, n.p("chart"), "the chart")?;
        let hv = chart.all("hvec");
        let mut pts: Vec<Point3> = hv
            .chunks_exact(3)
            .filter(|c| c.iter().all(|x| x.is_finite()))
            .map(|c| self.xf.point([c[0], c[1], c[2]]))
            .collect();
        if pts.len() < 2 {
            return refuse(idx, ty, "a chart of fewer than two points");
        }
        let limit = |g: &Self, p: u32| -> Option<(u8, Point3)> {
            let l = g.s.node(p)?;
            let h = l.all("hvec");
            if h.len() < 3 || !h[..3].iter().all(|x| x.is_finite()) {
                return None;
            }
            Some((l.c("type"), g.xf.point([h[0], h[1], h[2]])))
        };
        let (start, end) = (limit(self, n.p("start")), limit(self, n.p("end")));
        for (bound, ix) in [(sa, 0usize), (sb, 1)] {
            if self.s.node(bound).is_some_and(|b| b.ty() == 59) {
                let other = if ix == 0 { sb } else { sa };
                return self.boundary(idx, bound, other, &pts);
            }
        }
        let (a, _) = self.surface(sa)?;
        let (b, _) = self.surface(sb)?;
        let closed = start.is_some_and(|s| s.0 == b'H');
        if closed && pts.len() > 2 && pts[0].distance(pts[pts.len() - 1]) <= 1e-9 * self.xf.k {
            pts.pop();
        }
        // a terminator lies just past the chart's end, where the branch point is
        if let Some((b'T', p)) = start
            && p.distance(pts[0]) > tol::LINEAR * 1e-3
        {
            pts.insert(0, p);
        }
        if let Some((b'T', p)) = end
            && p.distance(pts[pts.len() - 1]) > tol::LINEAR * 1e-3
        {
            pts.push(p);
        }
        let n0 = pts.len();
        let pts = densify(&a, &b, pts, closed);
        if std::env::var_os("XT_TRACE").is_some() {
            eprintln!("intersection {idx}: {n0} -> {} closed {closed} {} {}", pts.len(), a.kind(), b.kind());
        }
        let m = pts.len();
        let mut tans = Vec::with_capacity(m);
        for i in 0..m {
            let chord = if closed {
                pts[(i + 1) % m] - pts[(i + m - 1) % m]
            } else {
                pts[(i + 1).min(m - 1)] - pts[i.saturating_sub(1)]
            };
            let t = match direction(&a, &b, pts[i]) {
                Some(t) if t.dot(chord) < 0.0 => -t,
                Some(t) => t,
                None => chord.normalize().map_or(Vec3::X, |u| u.into()),
            };
            tans.push(t);
        }
        // the chart's own chordal error, or one from the turn between points
        let err = chart.f("chordal_error") * self.xf.k;
        let sag = if err.is_finite() && err > 0.0 {
            err
        } else {
            let mut s: f64 = 0.0;
            for i in 0..m - 1 {
                let l = pts[i].distance(pts[i + 1]);
                let turn = dmath::acos_clamped(tans[i].dot(tans[i + 1]).clamp(-1.0, 1.0));
                s = s.max(l * turn / 4.0);
            }
            s
        };
        Ok(Curve::Intersection(Arc::new(IntCurve::new(a, b, pts, tans, closed, sag))))
    }

    /// A blend's spine. An intersection of offsets is made a C1 cubic through its chart, put
    /// on both surfaces and filled in to within SPINE_TOL: the exact curve finds each point by
    /// searching both offsets, and a blend asks for its spine at every point it is asked for.
    fn spine(&mut self, idx: u32) -> R<Curve> {
        let Some(n) = self.s.of(idx, 38) else { return self.curve(idx) };
        let Curve::Intersection(ic) = self.curve(idx)? else { return self.curve(idx) };
        let _ = n;
        let pts = densify_to(&ic.a, &ic.b, ic.pts.clone(), ic.closed, SPINE_TOL);
        let m = pts.len();
        let mut tans = Vec::with_capacity(m);
        for i in 0..m {
            let chord = if ic.closed {
                pts[(i + 1) % m] - pts[(i + m - 1) % m]
            } else {
                pts[(i + 1).min(m - 1)] - pts[i.saturating_sub(1)]
            };
            tans.push(match direction(&ic.a, &ic.b, pts[i]) {
                Some(t) if t.dot(chord) < 0.0 => -t,
                Some(t) => t,
                None => chord.normalize().map_or(Vec3::X, |u| u.into()),
            });
        }
        match hermite(&pts, &tans, ic.closed) {
            Some(c) => Ok(c),
            None => refuse(idx, 38, "the spine's chart makes no curve"),
        }
    }

    /// The edge where a blend meets one of its supporting surfaces, as the blend's spring.
    fn boundary(&mut self, at: u32, bound: u32, _other: u32, chart: &[Point3]) -> R<Curve> {
        let bb = self.node(at, bound, "the blend boundary")?;
        let side = bb.f("boundary");
        let blend_idx = bb.p("blend");
        let bn = self.node(at, blend_idx, "the blend")?;
        if bn.ty() != 56 {
            return refuse(at, 38, "a blend boundary of something not a blend");
        }
        let (b, _) = self.blend(&bn)?;
        // the supporting surface of the boundary is the blend's surface[1 - boundary]
        let on_a = side == 1.0;
        match b.spring(on_a) {
            Some(c) => {
                let _ = chart;
                Ok(c)
            }
            None => refuse(at, 38, "the blend's spring curve"),
        }
    }

    /// A rolling-ball blend, and whether its kernel surface's natural normal (away from the spine)
    /// is the file's.
    fn blend(&mut self, n: &Node) -> R<(Blend, bool)> {
        let (idx, ty) = (n.index, n.ty());
        if let Some(b) = self.blends.get(&idx) {
            let natural = self.blend_natural(n, b);
            return Ok((b.clone(), natural));
        }
        if n.c("blend_type") != b'R' {
            return refuse(idx, ty, "a cliff-edge blend");
        }
        let surf = n.all("surface");
        let range = n.all("range");
        let (sa, sb) = match surf {
            [a, b] => (*a as u32, *b as u32),
            _ => return refuse(idx, ty, "not two surfaces"),
        };
        let (ra, rb) = match range {
            [a, b] if a.is_finite() && b.is_finite() => (*a, *b),
            _ => return refuse(idx, ty, "its offsets are null"),
        };
        let (a, aa) = self.surface(sa)?;
        let (b, ab) = self.surface(sb)?;
        let k = self.xf.k;
        // offsets along each surface's oriented normal, the kernel's natural one where they agree
        let da = if aa { ra * k } else { -ra * k };
        let db = if ab { rb * k } else { -rb * k };
        let radius = da.abs();
        if (db.abs() - radius).abs() > 1e-9 * k.max(1.0) || radius <= tol::LINEAR {
            return refuse(idx, ty, format!("offsets {ra} and {rb}: not one ball"));
        }
        let spine = self.spine(n.p("spine"))?;
        let near = spine.eval(spine.domain().interval().mid());
        let surface = match &spine {
            Curve::Line(l) => {
                let c = l.origin;
                let x = foot(&a, da, c) - c;
                Surface::Cylinder(Cylinder {
                    frame: Frame::from_zx(c, l.dir, x).unwrap_or_else(|| Frame::from_z(c, l.dir)),
                    radius,
                })
            }
            Curve::Circle(c) if c.radius > radius + tol::LINEAR => Surface::Torus(Torus {
                frame: c.frame,
                major: c.radius,
                minor: radius,
            }),
            _ => match Pipe::build(spine.clone(), a.clone(), b.clone(), da, db, near) {
                Some(p) => Surface::Procedural(p),
                None => return refuse(idx, ty, "the ball along the spine makes no surface"),
            },
        };
        let blend = Blend {
            a,
            b,
            da,
            db,
            radius,
            spine,
            surface,
            var: None,
        };
        self.blends.insert(idx, blend.clone());
        let natural = self.blend_natural(n, &blend);
        Ok((blend, natural))
    }

    /// Whether the file's natural normal of a blend points away from its spine, as the kernel's
    /// does. The file's v runs from the foot on surface[0] to that on surface[1] (the other way
    /// for a spine of negative sense), u along the spine: the normal du x dv at v = 0 is
    /// T x Y, with X the way from the centre to the first foot and Y on towards the second; that
    /// is away from the spine when T . (X x Y) < 0.
    fn blend_natural(&self, n: &Node, b: &Blend) -> bool {
        let spine_sense = self.s.node(n.p("spine")).is_none_or(|s| sense(&s));
        let t = b.spine.domain().interval().mid();
        let d = b.spine.deriv(t);
        let (fa, fb) = (foot(&b.a, b.da, d.p), foot(&b.b, b.db, d.p));
        let (f0, f1) = if spine_sense { (fa, fb) } else { (fb, fa) };
        let x = f0 - d.p;
        let y = f1 - d.p;
        d.d1.dot(x.cross(y)) < 0.0
    }
}

/// How far a chain may stray from its curve between points, in mm: the files' charts are drawn
/// to about a millimetre, too coarse to tell an edge's vertex which part of a curve it is on.
const CHAIN_TOL: f64 = 1e-2;
/// Most points a chart is made up to.
const MAX_CHAIN: usize = 1 << 14;

/// The chart with points put on both surfaces between those too far apart for the curve: each
/// chord's middle pulled onto the curve, until the curve stays within CHAIN_TOL of its chords.
fn densify(a: &Surface, b: &Surface, pts: Vec<Point3>, closed: bool) -> Vec<Point3> {
    densify_to(a, b, pts, closed, CHAIN_TOL)
}

/// How near a blend's spine stays to the true one between its points, about: the cubic's error
/// falls with the fourth power of the step that a chord's middle within this of the curve gives.
const SPINE_TOL: f64 = 1e-3;

/// The cubic through points with the given unit tangents, C1, a piece between each two; closed
/// back to the first.
fn hermite(pts: &[Point3], tans: &[Vec3], closed: bool) -> Option<Curve> {
    let n = pts.len();
    let segs = if closed { n } else { n.checked_sub(1)? };
    if segs == 0 {
        return None;
    }
    let mut ctrl = Vec::with_capacity(3 * segs + 1);
    let mut knots = vec![0.0; 4];
    for i in 0..segs {
        let (p0, p1) = (pts[i], pts[(i + 1) % n]);
        let len = p0.distance(p1);
        let (m0, m1) = (tans[i] * len, tans[(i + 1) % n] * len);
        if i == 0 {
            ctrl.push(p0);
        }
        ctrl.push(p0 + m0 * (1.0 / 3.0));
        ctrl.push(p1 - m1 * (1.0 / 3.0));
        ctrl.push(p1);
        let k = (i + 1) as f64;
        if i + 1 < segs {
            knots.extend([k, k, k]);
        } else {
            knots.extend([k; 4]);
        }
    }
    let s = Spline::from_nurbs(3, &knots, &ctrl, None)?;
    let d = s.domain();
    Some(SplineCurve::curve(Arc::new(s), d))
}

fn densify_to(a: &Surface, b: &Surface, pts: Vec<Point3>, closed: bool, tol: f64) -> Vec<Point3> {
    let mut out = Vec::with_capacity(pts.len() * 2);
    let n = pts.len();
    let segs = if closed { n } else { n - 1 };
    let mut budget = MAX_CHAIN.saturating_sub(n);
    for i in 0..segs {
        let (p, q) = (pts[i], pts[(i + 1) % n]);
        out.push(p);
        // a stack of chords, the left first
        let mut stack = vec![(p, q, 0u32)];
        let mut inner = Vec::new();
        while let Some((x, y, depth)) = stack.pop() {
            let mid = x + (y - x) * 0.5;
            let Some(on) = refine(a, b, mid) else { continue };
            // the chord's own stretch of curve, not another branch the middle was pulled onto
            let (c, len2) = (y - x, (y - x).dot(y - x));
            let f = (on - x).dot(c) / len2.max(1e-300);
            if !(0.05..=0.95).contains(&f) || on.distance(mid) > 0.6 * len2.sqrt() {
                continue;
            }
            if on.distance(mid) <= tol || depth >= 16 || budget == 0 || on.distance(x).min(on.distance(y)) <= 1e-9 {
                continue;
            }
            budget -= 1;
            inner.push((x, on, y, depth));
            stack.push((on, y, depth + 1));
            stack.push((x, on, depth + 1));
        }
        // the inner points in order along the chord: each split's point after its left half's
        let mut ordered: Vec<Point3> = Vec::new();
        fn place(x: Point3, y: Point3, splits: &[(Point3, Point3, Point3, u32)], out: &mut Vec<Point3>) {
            if let Some(&(_, on, _, _)) = splits.iter().find(|s| s.0 == x && s.2 == y) {
                place(x, on, splits, out);
                out.push(on);
                place(on, y, splits, out);
            }
        }
        place(p, q, &inner, &mut ordered);
        out.extend(ordered);
    }
    if !closed {
        out.push(pts[n - 1]);
    }
    out
}

/// How a file surface's point is found at the file's parameters.
#[derive(Debug)]
pub(crate) enum Param {
    /// The kernel surface, the parameters mapped linearly: [u', v'] = M [u, v].
    Map(Surface, [[f64; 2]; 2]),
    /// C(cs u) + k v D.
    Swept { c: Curve, cs: f64, d: Vec3, k: f64 },
    /// The profile C(cs u) turned by v about the axis through p along a.
    Spun { c: Curve, cs: f64, p: Point3, a: Vec3 },
}
