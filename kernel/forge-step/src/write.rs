//! Part 21 from a body: an AP214 (or AP242) advanced B-rep.

use crate::fit::{self, DEFAULT_TOL, Fit};
use forge_geom::{Curve, Surface};
use forge_math::{Box2, Frame, Interval, Point2, Point3, Vec3};
use forge_topo::uv::{default_segments, face_uv_loops_with};
use forge_topo::{Body, EdgeId, FaceId, ShellId, VertexId};
use std::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Schema {
    Ap214,
    Ap242,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub schema: Schema,
    /// How far B-splines standing in for procedural surfaces and curves may stray, in mm.
    pub tol: f64,
    /// The product's name in the file.
    pub name: String,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            schema: Schema::Ap214,
            tol: DEFAULT_TOL,
            name: "part".into(),
        }
    }
}

/// A written file and what in it is not exact.
#[derive(Clone, Debug, PartialEq)]
pub struct Export {
    pub text: String,
    /// The distance accuracy the file states: `Options::tol`, or more where a fitted edge lies
    /// further than that off a fitted face.
    pub uncertainty: f64,
    pub solids: usize,
    pub faces: usize,
    pub edges: usize,
    /// Faces and edges written as B-spline fits, each with the most any strayed.
    pub fit_faces: Fit,
    pub fit_edges: Fit,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExportError {
    /// A number in the body that a file cannot hold.
    NotFinite {
        face: Option<FaceId>,
    },
    NoSolid,
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::NotFinite { face } => write!(f, "a non-finite number in the body ({face:?})"),
            ExportError::NoSolid => write!(f, "the body has no faces"),
        }
    }
}

/// A real as part 21 has it: always with a point, shortest digits that read back the same bits.
fn real(out: &mut String, x: f64) {
    let a = x.abs();
    if a != 0.0 && !(1e-4..1e16).contains(&a) {
        let s = format!("{x:E}");
        match s.split_once('E') {
            Some((m, e)) if !m.contains('.') => {
                let _ = write!(out, "{m}.E{e}");
            }
            _ => out.push_str(&s),
        }
    } else {
        let at = out.len();
        let _ = write!(out, "{x}");
        if !out[at..].contains('.') {
            out.push('.');
        }
    }
}

struct W {
    data: String,
    next: u64,
    finite: bool,
}

impl W {
    /// Starts instance #n; finish its text then call `end`.
    fn begin(&mut self) -> u64 {
        let id = self.next;
        self.next += 1;
        let _ = write!(self.data, "#{id}=");
        id
    }
    fn end(&mut self) {
        self.data.push_str(";\n");
    }
    fn s(&mut self, t: &str) -> &mut Self {
        self.data.push_str(t);
        self
    }
    fn r(&mut self, x: f64) -> &mut Self {
        self.finite &= x.is_finite();
        real(&mut self.data, x);
        self
    }
    fn id(&mut self, i: u64) -> &mut Self {
        let _ = write!(self.data, "#{i}");
        self
    }
    fn ids(&mut self, l: &[u64]) -> &mut Self {
        self.data.push('(');
        for (k, &i) in l.iter().enumerate() {
            if k > 0 {
                self.data.push(',');
            }
            self.id(i);
        }
        self.data.push(')');
        self
    }
    fn reals(&mut self, l: &[f64]) -> &mut Self {
        self.data.push('(');
        for (k, &x) in l.iter().enumerate() {
            if k > 0 {
                self.data.push(',');
            }
            self.r(x);
        }
        self.data.push(')');
        self
    }
    fn ints(&mut self, l: &[u32]) -> &mut Self {
        let s: Vec<String> = l.iter().map(u32::to_string).collect();
        let _ = write!(self.data, "({})", s.join(","));
        self
    }
    fn point(&mut self, p: Point3) -> u64 {
        let i = self.begin();
        self.s("CARTESIAN_POINT('',").reals(&[p.x, p.y, p.z]).s(")");
        self.end();
        i
    }
    fn dir(&mut self, v: Vec3) -> u64 {
        let i = self.begin();
        self.s("DIRECTION('',").reals(&[v.x, v.y, v.z]).s(")");
        self.end();
        i
    }
    fn placement(&mut self, f: &Frame) -> u64 {
        let (p, z, x) = (
            self.point(f.origin),
            self.dir(f.z().into()),
            self.dir(f.x().into()),
        );
        let i = self.begin();
        self.s("AXIS2_PLACEMENT_3D('',")
            .id(p)
            .s(",")
            .id(z)
            .s(",")
            .id(x)
            .s(")");
        self.end();
        i
    }

    fn curve(&mut self, c: &Curve, e: EdgeId, body: &Body, tol: f64, fits: &mut Fit) -> u64 {
        match c {
            Curve::Line(l) => {
                let (p, d) = (self.point(l.origin), self.dir(l.dir.into()));
                let v = self.begin();
                self.s("VECTOR('',").id(d).s(",1.)");
                self.end();
                let i = self.begin();
                self.s("LINE('',").id(p).s(",").id(v).s(")");
                self.end();
                i
            }
            Curve::Circle(k) => {
                let a = self.placement(&k.frame);
                let i = self.begin();
                self.s("CIRCLE('',").id(a).s(",").r(k.radius).s(")");
                self.end();
                i
            }
            Curve::Ellipse(k) => {
                let a = self.placement(&k.frame);
                let i = self.begin();
                self.s("ELLIPSE('',")
                    .id(a)
                    .s(",")
                    .r(k.major)
                    .s(",")
                    .r(k.minor)
                    .s(")");
                self.end();
                i
            }
            // STEP's parabola is (focal t^2, 2 focal t): the same points as ours, and the edge
            // holds them by its end vertices
            Curve::Parabola(k) => {
                let a = self.placement(&k.frame);
                let i = self.begin();
                self.s("PARABOLA('',").id(a).s(",").r(k.focal).s(")");
                self.end();
                i
            }
            Curve::Hyperbola(k) => {
                let a = self.placement(&k.frame);
                let i = self.begin();
                self.s("HYPERBOLA('',").id(a).s(",").r(k.a).s(",").r(k.b).s(")");
                self.end();
                i
            }
            Curve::Intersection(_) | Curve::Procedural(_) => {
                let f = fit::fit_curve(c, body.edge(e).range, tol);
                fits.count += 1;
                fits.error = fits.error.max(f.error);
                let pts: Vec<u64> = f.ctrl.iter().map(|&p| self.point(p)).collect();
                let i = self.begin();
                self.s("B_SPLINE_CURVE_WITH_KNOTS('',3,").ids(&pts);
                self.s(",.UNSPECIFIED.,.F.,.F.,")
                    .ints(&f.mults)
                    .s(",")
                    .reals(&f.knots)
                    .s(",.UNSPECIFIED.)");
                self.end();
                i
            }
        }
    }

    fn surface(&mut self, body: &Body, f: FaceId, tol: f64, fits: &mut Fit) -> u64 {
        match &body.face(f).surface {
            Surface::Plane(s) => {
                let a = self.placement(&s.frame);
                let i = self.begin();
                self.s("PLANE('',").id(a).s(")");
                self.end();
                i
            }
            Surface::Cylinder(s) => {
                let a = self.placement(&s.frame);
                let i = self.begin();
                self.s("CYLINDRICAL_SURFACE('',").id(a).s(",").r(s.radius).s(")");
                self.end();
                i
            }
            Surface::Cone(s) => {
                let a = self.placement(&s.frame);
                let half = forge_math::dmath::atan2(s.sin_a(), s.cos_a());
                let i = self.begin();
                self.s("CONICAL_SURFACE('',")
                    .id(a)
                    .s(",")
                    .r(s.radius)
                    .s(",")
                    .r(half)
                    .s(")");
                self.end();
                i
            }
            Surface::Sphere(s) => {
                let a = self.placement(&s.frame);
                let i = self.begin();
                self.s("SPHERICAL_SURFACE('',").id(a).s(",").r(s.radius).s(")");
                self.end();
                i
            }
            Surface::Torus(s) => {
                let a = self.placement(&s.frame);
                let i = self.begin();
                self.s("TOROIDAL_SURFACE('',")
                    .id(a)
                    .s(",")
                    .r(s.major)
                    .s(",")
                    .r(s.minor)
                    .s(")");
                self.end();
                i
            }
            s @ Surface::Procedural(_) => {
                let mut fit = fit::fit_surface(s, face_box(body, f), tol, &face_mask(body, f));
                fit::close_net(&mut fit, seams(body, f));
                fits.count += 1;
                fits.error = fits.error.max(fit.error);
                let rows: Vec<Vec<u64>> = fit
                    .ctrl
                    .iter()
                    .map(|r| r.iter().map(|&p| self.point(p)).collect())
                    .collect();
                let i = self.begin();
                self.s("B_SPLINE_SURFACE_WITH_KNOTS('',3,3,(");
                for (k, r) in rows.iter().enumerate() {
                    if k > 0 {
                        self.s(",");
                    }
                    self.ids(r);
                }
                let flag = |b: bool| if b { ".T." } else { ".F." };
                self.s("),.UNSPECIFIED.,")
                    .s(flag(fit.closed.0))
                    .s(",")
                    .s(flag(fit.closed.1))
                    .s(",.F.,")
                    .ints(&fit.u.1)
                    .s(",")
                    .ints(&fit.v.1)
                    .s(",");
                self.reals(&fit.u.0).s(",").reals(&fit.v.0).s(",.UNSPECIFIED.)");
                self.end();
                i
            }
        }
    }
}

/// A box of parameters that holds the face's loops, with a margin so its edges lie inside the
/// fit and not on the rim, kept in the surface's domain where that has ends.
/// Whether a seam (an edge the face uses twice) joins the face's parameter box in u and in v: its
/// two uses are at the box's two ends in that direction.
fn seams(body: &Body, f: FaceId) -> (bool, bool) {
    let loops = face_uv_loops_with(body, f, default_segments);
    let (mut lo, mut hi) = (
        Point2::new(f64::INFINITY, f64::INFINITY),
        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for p in loops.iter().flat_map(|l| l.points.iter()) {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    let mut out = (false, false);
    for l in &loops {
        for (k, &c) in l.coedges.iter().enumerate() {
            let e = body.edge(body.coedge(c).edge);
            if e.coedges.len() < 2 || body.coedge_face(e.coedges[0]) != body.coedge_face(e.coedges[1]) {
                continue;
            }
            let (i, j) = (
                l.coedge_first[k],
                l.coedge_first.get(k + 1).copied().unwrap_or(l.points.len()),
            );
            let run = &l.points[i..j];
            let along = |x: fn(&Point2) -> f64, a: f64, b: f64| {
                let eps = 1e-6 * (b - a).abs().max(1e-9);
                run.iter().all(|q| (x(q) - a).abs() <= eps) || run.iter().all(|q| (x(q) - b).abs() <= eps)
            };
            out.0 |= along(|q| q.x, lo.x, hi.x);
            out.1 |= along(|q| q.y, lo.y, hi.y);
        }
    }
    out
}

pub fn face_box(body: &Body, f: FaceId) -> Box2 {
    let loops = face_uv_loops_with(body, f, default_segments);
    let (mut lo, mut hi) = (
        Point2::new(f64::INFINITY, f64::INFINITY),
        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for p in loops.iter().flat_map(|l| l.points.iter()) {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    let d = forge_geom::SurfaceGeom::domain(&body.face(f).surface);
    let margin = |a: f64, b: f64, dom: forge_geom::Domain1| {
        let m = 0.02 * (b - a).max(1e-6);
        if dom.period.is_some() {
            (a - m, b + m)
        } else {
            // a face may run past the domain a procedural surface states, onto its carried-on part
            ((a - m).max(dom.lo.min(a)), (b + m).min(dom.hi.max(b)))
        }
    };
    let (su, sv) = seams(body, f);
    let (ul, uh) = if su { (lo.x, hi.x) } else { margin(lo.x, hi.x, d.u) };
    let (vl, vh) = if sv { (lo.y, hi.y) } else { margin(lo.y, hi.y, d.v) };
    Box2 {
        u: Interval::new(ul, uh),
        v: Interval::new(vl, vh),
    }
}

/// Which part of the face's box the face takes up, on a raster: `any` asks of a parameter box in
/// a few additions, as fits ask it for each cell.
pub struct FaceMask {
    b: Box2,
    /// Summed-area table of the raster, (N + 1) squared.
    sum: Vec<u32>,
    loops: Vec<Vec<Point2>>,
}

const MASK_N: usize = 512;

impl FaceMask {
    /// Whether the point is on the face by the loops themselves, not the raster.
    pub fn inside(&self, p: Point2) -> bool {
        let mut odd = false;
        for l in &self.loops {
            for (i, q) in l.iter().enumerate() {
                let r = l[(i + 1) % l.len()];
                if (q.y > p.y) != (r.y > p.y) && p.x < q.x + (p.y - q.y) / (r.y - q.y) * (r.x - q.x) {
                    odd = !odd;
                }
            }
        }
        odd
    }

    /// Points along the loops, about `n` across the box.
    pub fn boundary(&self, n: usize) -> Vec<Point2> {
        let mut out = Vec::new();
        let step = self.b.u.width().max(self.b.v.width()) / n as f64;
        for l in &self.loops {
            for (i, q) in l.iter().enumerate() {
                let r = l[(i + 1) % l.len()];
                let k = (((r.x - q.x).abs().max((r.y - q.y).abs())) / step)
                    .ceil()
                    .max(1.0) as usize;
                out.extend((0..k).map(|a| {
                    let t = a as f64 / k as f64;
                    Point2::new(q.x + (r.x - q.x) * t, q.y + (r.y - q.y) * t)
                }));
            }
        }
        out
    }

    pub fn any(&self, u0: f64, u1: f64, v0: f64, v1: f64) -> bool {
        let at = |x: f64, i: forge_math::Interval| {
            ((x - i.lo) / i.width() * MASK_N as f64).clamp(0.0, MASK_N as f64)
        };
        // a point asks of the cell it is in
        let (a, b) = (
            at(u0, self.b.u).floor() as usize,
            (at(u1, self.b.u).floor() as usize + 1).min(MASK_N),
        );
        let (c, d) = (
            at(v0, self.b.v).floor() as usize,
            (at(v1, self.b.v).floor() as usize + 1).min(MASK_N),
        );
        let (a, c) = (a.min(MASK_N - 1), c.min(MASK_N - 1));
        let s = |i: usize, j: usize| i64::from(self.sum[i * (MASK_N + 1) + j]);
        s(b, d) - s(a, d) - s(b, c) + s(a, c) > 0
    }
}

/// The face on a raster over its box. A loop open in the parameter
/// plane (the rim of a seamless cylinder) has no inside to fill: the whole box counts.
pub fn face_mask(body: &Body, f: FaceId) -> FaceMask {
    let b = face_box(body, f);
    let loops: Vec<Vec<Point2>> = face_uv_loops_with(body, f, default_segments)
        .into_iter()
        .map(|l| l.points)
        .collect();
    let n = MASK_N;
    let (du, dv) = (b.u.width() / n as f64, b.v.width() / n as f64);
    let open = loops.iter().any(|l| match (l.first(), l.last()) {
        (Some(p), Some(q)) => (p.x - q.x).abs() > 0.5 * b.u.width() || (p.y - q.y).abs() > 0.5 * b.v.width(),
        _ => true,
    });
    let mut cells = vec![open; n * n];
    if !open {
        for j in 0..n {
            let v = b.v.lo + (j as f64 + 0.5) * dv;
            let mut xs: Vec<f64> = Vec::new();
            for l in &loops {
                for (i, q) in l.iter().enumerate() {
                    let r = l[(i + 1) % l.len()];
                    if (q.y > v) != (r.y > v) {
                        xs.push(q.x + (v - q.y) / (r.y - q.y) * (r.x - q.x));
                    }
                }
            }
            xs.sort_by(f64::total_cmp);
            for w in xs.as_chunks::<2>().0 {
                let a = (((w[0] - b.u.lo) / du).floor().max(0.0) as usize).min(n);
                let c = (((w[1] - b.u.lo) / du).ceil().max(0.0) as usize).min(n);
                for x in a..c {
                    cells[x * n + j] = true;
                }
            }
        }
        // the loops themselves, a segment at a time
        for l in &loops {
            for (i, q) in l.iter().enumerate() {
                let r = l[(i + 1) % l.len()];
                let steps =
                    (((r.x - q.x).abs() / du).max((r.y - q.y).abs() / dv).ceil() as usize + 1).min(4 * n);
                for k in 0..=steps {
                    let t = k as f64 / steps as f64;
                    let a = (((q.x + (r.x - q.x) * t - b.u.lo) / du).max(0.0) as usize).min(n - 1);
                    let c = (((q.y + (r.y - q.y) * t - b.v.lo) / dv).max(0.0) as usize).min(n - 1);
                    cells[a * n + c] = true;
                }
            }
        }
    }
    let at = |a: usize, c: usize| cells[a.min(n - 1) * n + c.min(n - 1)];
    let mut sum = vec![0u32; (n + 1) * (n + 1)];
    for a in 0..n {
        for c in 0..n {
            sum[(a + 1) * (n + 1) + c + 1] =
                u32::from(at(a, c)) + sum[a * (n + 1) + c + 1] + sum[(a + 1) * (n + 1) + c]
                    - sum[a * (n + 1) + c];
        }
    }
    FaceMask { b, sum, loops }
}

/// The body as a file. One solid per outer shell, its voids with it.
pub fn export(body: &Body, opt: &Options) -> Result<Export, ExportError> {
    let body = if body.dead > 0 {
        body.compact()
    } else {
        body.clone()
    };
    if body.faces.is_empty() {
        return Err(ExportError::NoSolid);
    }
    let mut w = W {
        data: String::new(),
        next: 1,
        finite: true,
    };
    let (mut face_fits, mut edge_fits) = (Fit::default(), Fit::default());

    let verts: Vec<u64> = body
        .vertices
        .iter()
        .map(|v| {
            let p = w.point(v.point);
            let i = w.begin();
            w.s("VERTEX_POINT('',").id(p).s(")");
            w.end();
            i
        })
        .collect();
    let vid = |v: VertexId| verts[v.index()];
    // surfaces first: the edges on fitted ones say where on them they lie
    let surfaces: Vec<u64> = (0..body.faces.len() as u32)
        .map(|f| w.surface(&body, FaceId(f), opt.tol, &mut face_fits))
        .collect();
    let anchors = coedge_anchors(&body);
    let ctx2 = w.begin();
    w.s("(GEOMETRIC_REPRESENTATION_CONTEXT(2)PARAMETRIC_REPRESENTATION_CONTEXT()REPRESENTATION_CONTEXT('2D SPACE',''))");
    w.end();
    let edges: Vec<u64> = body
        .edge_ids()
        .map(|e| {
            let ed = body.edge(e);
            let mut c = w.curve(&ed.curve, e, &body, opt.tol, &mut edge_fits);
            if ed
                .coedges
                .iter()
                .any(|&co| matches!(body.face(body.coedge_face(co)).surface, Surface::Procedural(_)))
            {
                let on: Vec<u64> = ed
                    .coedges
                    .iter()
                    .map(|&co| {
                        let f = body.coedge_face(co);
                        let s = surfaces[f.index()];
                        if !matches!(body.face(f).surface, Surface::Procedural(_)) {
                            return s;
                        }
                        let fit =
                            fit::fit_pcurve(&ed.curve, &anchors[co.index()], &body.face(f).surface, opt.tol);
                        let pts: Vec<u64> = fit
                            .ctrl
                            .iter()
                            .map(|p| {
                                let i = w.begin();
                                w.s("CARTESIAN_POINT('',").reals(&[p.x, p.y]).s(")");
                                w.end();
                                i
                            })
                            .collect();
                        let b = w.begin();
                        w.s("B_SPLINE_CURVE_WITH_KNOTS('',3,").ids(&pts);
                        w.s(",.UNSPECIFIED.,.F.,.F.,")
                            .ints(&fit.mults)
                            .s(",")
                            .reals(&fit.knots)
                            .s(",.UNSPECIFIED.)");
                        w.end();
                        let d = w.begin();
                        w.s("DEFINITIONAL_REPRESENTATION('',(")
                            .id(b)
                            .s("),")
                            .id(ctx2)
                            .s(")");
                        w.end();
                        let i = w.begin();
                        w.s("PCURVE('',").id(s).s(",").id(d).s(")");
                        w.end();
                        i
                    })
                    .collect();
                let seam =
                    body.coedge_face(ed.coedges[0]) == body.coedge_face(ed.coedges[ed.coedges.len() - 1]);
                let i = w.begin();
                w.s(if seam {
                    "SEAM_CURVE('',".to_string()
                } else {
                    "SURFACE_CURVE('',".to_string()
                }
                .as_str());
                w.id(c).s(",").ids(&on).s(",.PCURVE_S1.)");
                w.end();
                c = i;
            }
            let i = w.begin();
            w.s("EDGE_CURVE('',")
                .id(vid(ed.start))
                .s(",")
                .id(vid(ed.end))
                .s(",")
                .id(c)
                .s(",.T.)");
            w.end();
            i
        })
        .collect();
    let mut faces = vec![0u64; body.faces.len()];
    for f in body.face_ids() {
        let mut bounds = Vec::new();
        for (k, &l) in body.face(f).loops.iter().enumerate() {
            let uses: Vec<u64> = body
                .lp(l)
                .coedges()
                .map(|c| {
                    let co = body.coedge(c);
                    let i = w.begin();
                    w.s("ORIENTED_EDGE('',*,*,").id(edges[co.edge.index()]);
                    w.s(if co.forward { ",.T.)" } else { ",.F.)" });
                    w.end();
                    i
                })
                .collect();
            let el = w.begin();
            w.s("EDGE_LOOP('',").ids(&uses).s(")");
            w.end();
            let b = w.begin();
            w.s(if k == 0 {
                "FACE_OUTER_BOUND('',".to_string()
            } else {
                "FACE_BOUND('',".to_string()
            }
            .as_str());
            w.id(el).s(",.T.)");
            w.end();
            bounds.push(b);
        }
        let i = w.begin();
        w.s("ADVANCED_FACE('',")
            .ids(&bounds)
            .s(",")
            .id(surfaces[f.index()]);
        w.s(if body.face(f).same_sense { ",.T.)" } else { ",.F.)" });
        w.end();
        faces[f.index()] = i;
    }

    let shells = shell_kinds(&body);
    let mut solids = Vec::new();
    for (s, _) in shells.iter().filter(|x| x.1.outer) {
        let list = |w: &mut W, s: ShellId, kind: &str| {
            let fs: Vec<u64> = body.shell(s).faces.iter().map(|&f| faces[f.index()]).collect();
            let i = w.begin();
            w.s(kind).s("('',").ids(&fs).s(")");
            w.end();
            i
        };
        let outer = list(&mut w, *s, "CLOSED_SHELL");
        let voids: Vec<u64> = shells
            .iter()
            .filter(|x| x.1.within == Some(*s))
            .map(|x| {
                let c = list(&mut w, x.0, "CLOSED_SHELL");
                let i = w.begin();
                w.s("ORIENTED_CLOSED_SHELL('',*,").id(c).s(",.F.)");
                w.end();
                i
            })
            .collect();
        let i = w.begin();
        if voids.is_empty() {
            w.s("MANIFOLD_SOLID_BREP('',").id(outer).s(")");
        } else {
            w.s("BREP_WITH_VOIDS('',").id(outer).s(",").ids(&voids).s(")");
        }
        w.end();
        solids.push(i);
    }

    let units = {
        let m = w.begin();
        w.s("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))");
        w.end();
        let a = w.begin();
        w.s("(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.))");
        w.end();
        let s = w.begin();
        w.s("(NAMED_UNIT(*)SI_UNIT($,.STERADIAN.)SOLID_ANGLE_UNIT())");
        w.end();
        let u = w.begin();
        w.s("UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(")
            .r(forge_math::tol::LINEAR.max(opt.tol));
        w.s("),")
            .id(m)
            .s(",'distance_accuracy_value','confusion accuracy')");
        w.end();
        (m, a, s, u)
    };
    let ctx = w.begin();
    w.s("(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((")
        .id(units.3);
    w.s("))GLOBAL_UNIT_ASSIGNED_CONTEXT((")
        .id(units.0)
        .s(",")
        .id(units.1)
        .s(",")
        .id(units.2);
    w.s("))REPRESENTATION_CONTEXT('',''))");
    w.end();
    let brep = w.begin();
    w.s("ADVANCED_BREP_SHAPE_REPRESENTATION('',")
        .ids(&solids)
        .s(",")
        .id(ctx)
        .s(")");
    w.end();

    let (context, protocol, year) = match opt.schema {
        Schema::Ap214 => ("automotive design", "automotive_design", 2001),
        Schema::Ap242 => (
            "managed model based 3d engineering",
            "ap242_managed_model_based_3d_engineering",
            2014,
        ),
    };
    let name = quoted(&opt.name);
    let app = w.begin();
    let _ = write!(w.data, "APPLICATION_CONTEXT('{context}')");
    w.end();
    let i = w.begin();
    let _ = write!(
        w.data,
        "APPLICATION_PROTOCOL_DEFINITION('international standard','{protocol}',{year},#{app})"
    );
    w.end();
    let _ = i;
    let pc = w.begin();
    w.s("PRODUCT_CONTEXT('',").id(app).s(",'mechanical')");
    w.end();
    let prod = w.begin();
    let _ = write!(w.data, "PRODUCT('{name}','{name}','',(#{pc}))");
    w.end();
    let form = w.begin();
    w.s("PRODUCT_DEFINITION_FORMATION('','',").id(prod).s(")");
    w.end();
    let pdc = w.begin();
    w.s("PRODUCT_DEFINITION_CONTEXT('part definition',")
        .id(app)
        .s(",'design')");
    w.end();
    let pd = w.begin();
    w.s("PRODUCT_DEFINITION('design','',")
        .id(form)
        .s(",")
        .id(pdc)
        .s(")");
    w.end();
    let pds = w.begin();
    w.s("PRODUCT_DEFINITION_SHAPE('','',").id(pd).s(")");
    w.end();
    let sdr = w.begin();
    w.s("SHAPE_DEFINITION_REPRESENTATION(")
        .id(pds)
        .s(",")
        .id(brep)
        .s(")");
    w.end();
    let _ = sdr;
    if !w.finite {
        return Err(ExportError::NotFinite { face: None });
    }

    let schema = match opt.schema {
        Schema::Ap214 => "AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }",
        Schema::Ap242 => "AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 1 1 4 }",
    };
    let mut text = String::with_capacity(w.data.len() + 512);
    let _ = write!(
        text,
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ForgeCAD part'),'2;1');\n\
         FILE_NAME('{name}','2000-01-01T00:00:00',(''),(''),'ForgeCAD','ForgeCAD','');\n\
         FILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n{}ENDSEC;\nEND-ISO-10303-21;\n",
        w.data
    );
    // what the file says of itself is what its fits came to: a fitted edge and a fitted face can
    // miss each other by more than either strays from what it stands for
    let stated = forge_math::tol::LINEAR.max(opt.tol);
    let mut uncertainty = stated;
    // (nor can it say less than a fit left: one short of `tol` on its cells)
    let gap = if face_fits.count + edge_fits.count > 0 {
        crate::read::curve_off_surface(text.as_bytes(), &crate::part21::Limits::default())
            .unwrap_or(0.0)
            .max(face_fits.error)
            .max(edge_fits.error)
    } else {
        0.0
    };
    if gap > stated {
        uncertainty = 1.01 * gap;
        let (mut a, mut b) = (String::from("LENGTH_MEASURE("), String::from("LENGTH_MEASURE("));
        real(&mut a, stated);
        real(&mut b, uncertainty);
        text = text.replacen(&a, &b, 1);
    }
    Ok(Export {
        text,
        uncertainty,
        solids: solids.len(),
        faces: body.faces.len(),
        edges: edges.len(),
        fit_faces: face_fits,
        fit_edges: edge_fits,
    })
}

/// For each coedge of a fitted face, its edge's parameters and where its loop draws them, by
/// increasing parameter (a seam's two uses get their two periods).
fn coedge_anchors(body: &Body) -> Vec<Vec<(f64, Point2)>> {
    let mut out = vec![Vec::new(); body.coedges.len()];
    for f in body.face_ids() {
        if !matches!(body.face(f).surface, Surface::Procedural(_)) {
            continue;
        }
        for l in face_uv_loops_with(body, f, default_segments) {
            let total = l.sample_of.iter().copied().max().map_or(0, |m| m + 1);
            for (k, &c) in l.coedges.iter().enumerate() {
                let co = body.coedge(c);
                let e = body.edge(co.edge);
                let (a, b) = (
                    l.coedge_first[k],
                    l.coedge_first.get(k + 1).copied().unwrap_or(total),
                );
                let n = b - a;
                let mut row: Vec<(f64, Point2)> = (0..=n)
                    .map(|i| {
                        let at = if a + i == total { 0 } else { a + i };
                        let s = if co.forward { i } else { n - i };
                        let t = (e.range.lo + e.range.width() * (s as f64 / n as f64)).min(e.range.hi);
                        (t, l.point_of_sample(at))
                    })
                    .collect();
                // the loop's closing sample is drawn at its start: carry it to this end by the
                // period, where a closed loop winds
                if a + n == total && n > 0 {
                    let d = forge_geom::SurfaceGeom::domain(&body.face(f).surface);
                    let prev = row[n - 1].1;
                    let fix = |x: f64, r: f64, p: Option<f64>| p.map_or(x, |p| x + p * ((r - x) / p).round());
                    let q = row[n].1;
                    row[n].1 = Point2::new(fix(q.x, prev.x, d.u.period), fix(q.y, prev.y, d.v.period));
                }
                if !co.forward {
                    row.reverse();
                }
                out[c.index()] = row;
            }
        }
    }
    out
}

fn quoted(s: &str) -> String {
    // part 21 strings are ASCII here; a quote doubles
    s.chars()
        .map(|c| if c.is_ascii_graphic() || c == ' ' { c } else { '_' })
        .collect::<String>()
        .replace('\'', "''")
}

struct ShellKind {
    outer: bool,
    within: Option<ShellId>,
}

/// Each shell: bounding material, or a void in which outer shell. A void's sense is its signed
/// volume; it belongs to the smallest outer shell whose box holds it.
fn shell_kinds(body: &Body) -> Vec<(ShellId, ShellKind)> {
    let shells: Vec<ShellId> = (0..body.shells.len() as u32)
        .map(ShellId)
        .filter(|&s| !body.shell(s).faces.is_empty())
        .collect();
    let boxes: Vec<forge_math::Box3> = shells
        .iter()
        .map(|&s| {
            body.shell(s)
                .faces
                .iter()
                .fold(forge_math::Box3::EMPTY, |b, &f| b.hull(&body.face_box(f)))
        })
        .collect();
    let outer: Vec<bool> = shells
        .iter()
        .map(|&s| shells.len() == 1 || forge_topo::mass::shell_volume(body, s) >= 0.0)
        .collect();
    shells
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let within = if outer[i] {
                None
            } else {
                (0..shells.len())
                    .filter(|&j| outer[j] && holds(&boxes[j], &boxes[i]))
                    .min_by(|&a, &b| boxes[a].diagonal().total_cmp(&boxes[b].diagonal()))
                    .map(|j| shells[j])
            };
            (
                s,
                ShellKind {
                    outer: outer[i],
                    within,
                },
            )
        })
        .collect()
}

fn holds(a: &forge_math::Box3, b: &forge_math::Box3) -> bool {
    let inside = |p: forge_math::Interval, q: forge_math::Interval| p.lo <= q.lo && p.hi >= q.hi;
    inside(a.x, b.x) && inside(a.y, b.y) && inside(a.z, b.z)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: f64) -> String {
        let mut s = String::new();
        real(&mut s, x);
        s
    }

    #[test]
    fn reals_have_a_point_and_read_back_the_same_bits() {
        assert_eq!(r(1.0), "1.");
        assert_eq!(r(-0.5), "-0.5");
        assert_eq!(r(1e-7), "1.E-7");
        assert_eq!(r(1.5e20), "1.5E20");
        for x in [0.1, 1.0 / 3.0, 1e-300, 1.7976931348623157e308, -2.5e-5, 12345.678] {
            let t = r(x);
            assert!(t.contains('.'), "{t}");
            assert_eq!(t.parse::<f64>().unwrap().to_bits(), x.to_bits(), "{t}");
        }
    }

    #[test]
    fn a_box_is_one_exact_solid() {
        use forge_ops::{Profile, extrude};
        let p = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::new(0.0, 0.0), 10.0, 20.0, 0)],
        };
        let b = extrude(&p, 5.0, 1).unwrap();
        let e = export(&b, &Options::default()).unwrap();
        assert_eq!((e.solids, e.faces, e.edges), (1, 6, 12));
        assert_eq!((e.fit_faces.count, e.fit_edges.count), (0, 0));
        assert!(e.text.starts_with("ISO-10303-21;"));
        assert!(e.text.ends_with("END-ISO-10303-21;\n"));
        assert!(e.text.contains("AUTOMOTIVE_DESIGN"));
        assert_eq!(e.text.matches("ADVANCED_FACE").count(), 6);
        assert!(e.text.contains("MANIFOLD_SOLID_BREP"));
        let again = export(&b, &Options::default()).unwrap();
        assert_eq!(e.text, again.text);
        let a242 = export(
            &b,
            &Options {
                schema: Schema::Ap242,
                ..Options::default()
            },
        )
        .unwrap();
        assert!(
            a242.text
                .contains("AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF")
        );
    }
}
