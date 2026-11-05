//! Exact hidden-line removal, after OpenCascade's `HLRBRep_Algo`: every edge and the outline of
//! every curved face is projected, every curve is split where it crosses another on the view, and
//! each piece is seen or hidden by one ray from it toward the eye against the exact faces.

use crate::geom2::{Geom2, conic_steps};
use crate::grid::Grid;
use crate::view::View;
use forge_bool::{Index, Where};
use forge_geom::intersect::{Contact, LineSurface, line_surface};
use forge_geom::{Curve, CurveGeom, Line, Surface, SurfaceGeom};
use forge_math::{Frame, Interval, Point2, Point3, Vec2, Vec3, dmath};
use forge_topo::uv::face_uv_loops;
use forge_topo::{Body, EdgeId, FaceId, Name};

/// One solid placed in the view.
#[derive(Clone, Copy)]
pub struct Instance<'a> {
    pub body: &'a Body,
    pub pose: Frame,
}

#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Draw tangent edges (between faces meeting smoothly). Off, Inventor's default.
    pub tangent: bool,
    /// How far a drawn curve may stray from the exact one, in model millimetres.
    pub chord: f64,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            tangent: false,
            chord: 0.01,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Class {
    /// An edge between faces at an angle.
    Sharp,
    /// An edge between faces meeting smoothly.
    Tangent,
    /// Where a curved face turns away from the eye.
    Outline,
}

/// What a piece is drawn from: an edge or a face's outline, by the kernel's persistent name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    Edge(Name),
    Outline(Name),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub inst: usize,
    pub source: Source,
    pub class: Class,
    pub visible: bool,
    pub geom: Geom2,
}

/// A curve in model space (the view's world): exact where it has a closed form.
#[derive(Clone, Debug)]
enum Path3 {
    /// From the first point (t = 0) to the second (t = 1).
    Line(Point3, Point3),
    /// `c + cos t u + sin t v` for t in `t0..t1`.
    Conic {
        c: Point3,
        u: Vec3,
        v: Vec3,
        t0: f64,
        t1: f64,
    },
    /// Points on the curve; t is the index.
    Poly(Vec<Point3>),
}

impl Path3 {
    fn range(&self) -> (f64, f64) {
        match self {
            Path3::Line(..) => (0.0, 1.0),
            Path3::Conic { t0, t1, .. } => (*t0, *t1),
            Path3::Poly(p) => (0.0, p.len().saturating_sub(1) as f64),
        }
    }

    fn at(&self, t: f64) -> Point3 {
        match self {
            Path3::Line(a, b) => a.lerp(*b, t),
            Path3::Conic { c, u, v, .. } => {
                let (s, co) = dmath::sin_cos(t);
                *c + *u * co + *v * s
            }
            Path3::Poly(p) => {
                if p.len() == 1 {
                    return p[0];
                }
                let i = (t.floor().max(0.0) as usize).min(p.len() - 2);
                p[i].lerp(p[i + 1], t - i as f64)
            }
        }
    }

    fn sub(&self, a: f64, b: f64) -> Path3 {
        match self {
            Path3::Line(..) => Path3::Line(self.at(a), self.at(b)),
            Path3::Conic { c, u, v, .. } => Path3::Conic {
                c: *c,
                u: *u,
                v: *v,
                t0: a,
                t1: b,
            },
            Path3::Poly(p) => {
                let mut out = vec![self.at(a)];
                let (i, j) = (a.floor() as usize + 1, (b.ceil() as usize).min(p.len()));
                out.extend(p.iter().take(j).skip(i).copied());
                out.push(self.at(b));
                out.dedup();
                Path3::Poly(out)
            }
        }
    }

    fn placed(&self, f: &Frame) -> Path3 {
        match self {
            Path3::Line(a, b) => Path3::Line(f.point_to_world(*a), f.point_to_world(*b)),
            Path3::Conic { c, u, v, t0, t1 } => Path3::Conic {
                c: f.point_to_world(*c),
                u: f.vec_to_world(*u),
                v: f.vec_to_world(*v),
                t0: *t0,
                t1: *t1,
            },
            Path3::Poly(p) => Path3::Poly(p.iter().map(|q| f.point_to_world(*q)).collect()),
        }
    }

    /// Parameters whose points, joined, stay within `chord` of the curve.
    fn ts(&self, chord: f64) -> Vec<f64> {
        let (t0, t1) = self.range();
        match self {
            Path3::Line(..) => vec![0.0, 1.0],
            Path3::Conic { u, v, .. } => {
                let n = conic_steps(u.length().max(v.length()), t1 - t0, chord);
                (0..=n).map(|k| t0 + (t1 - t0) * k as f64 / n as f64).collect()
            }
            Path3::Poly(p) => (0..p.len()).map(|k| k as f64).collect(),
        }
    }

    /// The projection of the part between `a < b`.
    fn geom(&self, view: &View, a: f64, b: f64) -> Geom2 {
        match self {
            Path3::Line(..) => Geom2::Line {
                a: view.project(self.at(a)),
                b: view.project(self.at(b)),
            },
            Path3::Conic { c, u, v, .. } => {
                let (u2, v2) = (view.project_vec(*u), view.project_vec(*v));
                if flat(u2, v2, u.length().max(v.length())) {
                    Geom2::Line {
                        a: view.project(self.at(a)),
                        b: view.project(self.at(b)),
                    }
                } else {
                    Geom2::Conic {
                        c: view.project(*c),
                        u: u2,
                        v: v2,
                        t0: a,
                        t1: b,
                    }
                }
            }
            Path3::Poly(_) => match self.sub(a, b) {
                Path3::Poly(p) => Geom2::Poly(p.into_iter().map(|q| view.project(q)).collect()),
                _ => unreachable!("a polyline's part is a polyline"),
            },
        }
    }
}

/// Whether a conic with these projected semi-diameters is seen edge-on (drawn as a line).
fn flat(u2: Vec2, v2: Vec2, r: f64) -> bool {
    u2.cross(v2).abs() <= 1e-9 * r * r
}

struct Curve3 {
    inst: usize,
    source: Source,
    class: Class,
    path: Path3,
}

/// Points of an exact curve over `range`, close enough that the chords between them stay within
/// `chord` of it.
fn sample(f: &dyn Fn(f64) -> Point3, lo: f64, hi: f64, chord: f64) -> Vec<Point3> {
    #[allow(clippy::too_many_arguments)]
    fn go(
        f: &dyn Fn(f64) -> Point3,
        a: f64,
        pa: Point3,
        b: f64,
        pb: Point3,
        chord: f64,
        depth: u32,
        out: &mut Vec<Point3>,
    ) {
        let m = 0.5 * (a + b);
        let pm = f(m);
        let (q1, q3) = (f(a + 0.25 * (b - a)), f(a + 0.75 * (b - a)));
        let sag = pm
            .distance(pa.lerp(pb, 0.5))
            .max(q1.distance(pa.lerp(pm, 0.5)))
            .max(q3.distance(pm.lerp(pb, 0.5)));
        if depth < 14 && sag > chord {
            go(f, a, pa, m, pm, chord, depth + 1, out);
            go(f, m, pm, b, pb, chord, depth + 1, out);
        } else {
            out.push(pb);
        }
    }
    let n = 16;
    let mut out = vec![f(lo)];
    for k in 0..n {
        let (a, b) = (
            lo + (hi - lo) * k as f64 / n as f64,
            lo + (hi - lo) * (k + 1) as f64 / n as f64,
        );
        let pa = *out.last().expect("started with a point");
        go(f, a, pa, b, f(b), chord, 0, &mut out);
    }
    out
}

fn edge_path(body: &Body, e: EdgeId, chord: f64) -> Path3 {
    let edge = body.edge(e);
    let r = edge.range;
    match &edge.curve {
        Curve::Line(l) => Path3::Line(l.eval(r.lo), l.eval(r.hi)),
        Curve::Circle(c) => Path3::Conic {
            c: c.frame.origin,
            u: c.frame.x().get() * c.radius,
            v: c.frame.y().get() * c.radius,
            t0: r.lo,
            t1: r.hi,
        },
        Curve::Ellipse(c) => Path3::Conic {
            c: c.frame.origin,
            u: c.frame.x().get() * c.major,
            v: c.frame.y().get() * c.minor,
            t0: r.lo,
            t1: r.hi,
        },
        curve => Path3::Poly(sample(&|t| curve.eval(t), r.lo, r.hi, chord)),
    }
}

/// Whether the faces either side of an edge meet smoothly along it.
fn smooth(body: &Body, e: EdgeId, faces: [FaceId; 2]) -> bool {
    let edge = body.edge(e);
    [0.25, 0.5, 0.75].iter().all(|&k| {
        let p = edge.curve.eval(edge.range.lo + k * edge.range.width());
        let n = faces.map(|f| body.face_normal(f, body.face(f).surface.project(p)));
        match n {
            [Some(a), Some(b)] => a.dot(b.get()) > dmath::cos(1.0f64.to_radians()),
            _ => false,
        }
    })
}

/// A face's parameter window, from its loops.
fn window(body: &Body, f: FaceId) -> Option<(Interval, Interval)> {
    let mut u = Interval::EMPTY;
    let mut v = Interval::EMPTY;
    for l in face_uv_loops(body, f) {
        for p in l.points {
            u.include(p.x);
            v.include(p.y);
        }
    }
    (!u.is_empty() && !v.is_empty()).then_some((u, v))
}

/// The roots of `g` on `lo..hi` (sign changes on `n` steps, then bisected).
fn roots(g: &dyn Fn(f64) -> f64, lo: f64, hi: f64, n: usize) -> Vec<f64> {
    let mut out = Vec::new();
    let mut a = lo;
    let mut ga = g(a);
    for k in 1..=n {
        let b = lo + (hi - lo) * k as f64 / n as f64;
        let gb = g(b);
        if ga == 0.0 {
            out.push(a);
        } else if ga * gb < 0.0 {
            let (mut x, mut y, mut gx) = (a, b, ga);
            for _ in 0..60 {
                let m = 0.5 * (x + y);
                let gm = g(m);
                if gm * gx > 0.0 {
                    x = m;
                    gx = gm;
                } else {
                    y = m;
                }
            }
            out.push(0.5 * (x + y));
        }
        a = b;
        ga = gb;
    }
    out
}

/// The runs of `path` inside face `f`: sampled, the ends bisected on the face's exact test.
fn clip(path: &Path3, idx: &Index, f: FaceId, out: &mut Vec<Path3>) {
    let (t0, t1) = path.range();
    let n = match path {
        Path3::Line(..) => 64,
        Path3::Conic { .. } => 128,
        Path3::Poly(p) => 2 * p.len(),
    };
    let ts: Vec<f64> = (0..=n).map(|k| t0 + (t1 - t0) * k as f64 / n as f64).collect();
    let inside = |t: f64| idx.point_in_face(f, path.at(t)) != Where::Outside;
    let ins: Vec<bool> = ts.iter().map(|&t| inside(t)).collect();
    let edge = |a: f64, b: f64, ia: bool| {
        let (mut a, mut b) = (a, b);
        for _ in 0..50 {
            let m = 0.5 * (a + b);
            if inside(m) == ia {
                a = m;
            } else {
                b = m;
            }
        }
        0.5 * (a + b)
    };
    let mut runs: Vec<(f64, f64)> = Vec::new();
    let mut start = ins[0].then_some(t0);
    for k in 1..ts.len() {
        if ins[k] != ins[k - 1] {
            let x = edge(ts[k - 1], ts[k], ins[k - 1]);
            match start.take() {
                Some(s) => runs.push((s, x)),
                None => start = Some(x),
            }
        }
    }
    if let Some(s) = start {
        runs.push((s, t1));
    }
    // a gap shorter than GAP is the face test's own doubt (near a sphere's pole), not
    // the face's boundary
    let gap = |a: f64, b: f64| path.at(a).distance(path.at(b)) <= GAP;
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for r in runs {
        match merged.last_mut() {
            Some(m) if gap(m.1, r.0) => m.1 = r.1,
            _ => merged.push(r),
        }
    }
    if let (Path3::Conic { .. }, true) = (path, (t1 - t0 - dmath::TAU).abs() <= 1e-12) {
        if merged.len() >= 2 && gap(merged[merged.len() - 1].1, t1) && gap(t0, merged[0].0) {
            let last = merged.pop().expect("two runs");
            merged[0].0 = last.0 - dmath::TAU;
        }
        if merged.len() == 1 && gap(merged[0].0, t0) && gap(merged[0].1, t1) {
            merged[0] = (t0, t1);
        }
    }
    for (a, b) in merged {
        out.push(path.sub(a, b));
    }
}

/// Runs of an outline closer than this are one.
const GAP: f64 = 1e-2;

/// The outline of face `f` seen from `d` (in the body's own space): where its normal is square to
/// `d`, inside the face.
fn outlines(body: &Body, idx: &Index, f: FaceId, d: Vec3, chord: f64) -> Vec<Path3> {
    let surface = &body.face(f).surface;
    let mut out = Vec::new();
    let mut paths = Vec::new();
    match surface {
        Surface::Plane(_) => return out,
        Surface::Cylinder(_) | Surface::Cone(_) => {
            let Some((_, v)) = window(body, f) else { return out };
            let vm = 0.5 * (v.lo + v.hi);
            let g = |u: f64| surface.normal(Point2::new(u, vm)).map_or(0.0, |n| n.dot(d));
            for u in roots(&g, 0.0, dmath::TAU, 96) {
                let pad = 1e-3 * (v.hi - v.lo);
                let (a, b) = (
                    surface.eval(Point2::new(u, v.lo - pad)),
                    surface.eval(Point2::new(u, v.hi + pad)),
                );
                paths.push(Path3::Line(a, b));
            }
        }
        Surface::Sphere(s) => {
            let Some(z) = d.normalize() else { return out };
            let fr = Frame::from_z(s.frame.origin, z);
            paths.push(Path3::Conic {
                c: s.frame.origin,
                u: fr.x().get() * s.radius,
                v: fr.y().get() * s.radius,
                t0: 0.0,
                t1: dmath::TAU,
            });
        }
        Surface::Torus(t) => {
            let fr = &t.frame;
            let (dx, dy, dz) = (d.dot(fr.x().get()), d.dot(fr.y().get()), d.dot(fr.z().get()));
            let at = |u: f64, v: f64| surface.eval(Point2::new(u, v));
            if dx.abs() <= 1e-9 * d.length() && dy.abs() <= 1e-9 * d.length() {
                // seen along its axis: the inner and outer equators
                for rho in [t.major + t.minor, t.major - t.minor] {
                    paths.push(Path3::Conic {
                        c: fr.origin,
                        u: fr.x().get() * rho,
                        v: fr.y().get() * rho,
                        t0: 0.0,
                        t1: dmath::TAU,
                    });
                }
            } else if dz.abs() <= 1e-9 * d.length() {
                // seen square to its axis: the top and bottom rims and two meridians
                let (r, z) = (t.major, fr.z().get());
                for s in [-1.0, 1.0] {
                    paths.push(Path3::Conic {
                        c: fr.origin + z * (s * t.minor),
                        u: fr.x().get() * r,
                        v: fr.y().get() * r,
                        t0: 0.0,
                        t1: dmath::TAU,
                    });
                }
                let phi = dmath::atan2(dy, dx);
                for u in [phi + dmath::FRAC_PI_2, phi - dmath::FRAC_PI_2] {
                    let (s, c) = dmath::sin_cos(u);
                    let w = fr.x().get() * c + fr.y().get() * s;
                    paths.push(Path3::Conic {
                        c: fr.origin + w * t.major,
                        u: w * t.minor,
                        v: z * t.minor,
                        t0: 0.0,
                        t1: dmath::TAU,
                    });
                }
            } else {
                for branch in [0.0, dmath::PI] {
                    let p = move |u: f64| {
                        let (s, c) = dmath::sin_cos(u);
                        at(u, dmath::atan2(-(dx * c + dy * s), dz) + branch)
                    };
                    paths.push(Path3::Poly(sample(&p, 0.0, dmath::TAU, chord)));
                }
            }
        }
        Surface::Procedural(_) => {
            let Some((u, v)) = window(body, f) else { return out };
            paths = traced(surface, d, u, v, chord);
        }
    }
    for p in &paths {
        clip(p, idx, f, &mut out);
    }
    out
}

/// The outline of a surface with no closed form, traced: `n . d = 0` found on a grid over the
/// window, each crossing settled on its grid edge, and the crossings chained.
fn traced(s: &Surface, d: Vec3, u: Interval, v: Interval, chord: f64) -> Vec<Path3> {
    let len = |a: Point2, b: Point2| {
        (0..8)
            .map(|k| {
                s.eval(a.lerp(b, k as f64 / 8.0))
                    .distance(s.eval(a.lerp(b, (k + 1) as f64 / 8.0)))
            })
            .sum::<f64>()
    };
    let (um, vm) = (0.5 * (u.lo + u.hi), 0.5 * (v.lo + v.hi));
    let lu = len(Point2::new(u.lo, vm), Point2::new(u.hi, vm));
    let lv = len(Point2::new(um, v.lo), Point2::new(um, v.hi));
    let cells = |l: f64| ((l / (40.0 * chord).max(0.25)).ceil() as usize).clamp(12, 300);
    let (nu, nv) = (cells(lu), cells(lv));
    let at = |i: usize, j: usize| {
        Point2::new(
            u.lo + u.width() * i as f64 / nu as f64,
            v.lo + v.width() * j as f64 / nv as f64,
        )
    };
    let g = |p: Point2| s.normal(p).map(|n| n.dot(d));
    let vals: Vec<Option<f64>> = (0..=nv)
        .flat_map(|j| (0..=nu).map(move |i| (i, j)))
        .map(|(i, j)| g(at(i, j)))
        .collect();
    let val = |i: usize, j: usize| vals[j * (nu + 1) + i];
    // a crossing on the grid edge from node a to node b, keyed by the edge
    let cross = |a: (usize, usize), b: (usize, usize)| -> Option<Point2> {
        let (ga, gb) = (val(a.0, a.1)?, val(b.0, b.1)?);
        if ga * gb > 0.0 || (ga == 0.0 && gb == 0.0) {
            return None;
        }
        let (mut x, mut y, mut gx) = (at(a.0, a.1), at(b.0, b.1), ga);
        for _ in 0..50 {
            let m = x.lerp(y, 0.5);
            let gm = g(m)?;
            if gm * gx > 0.0 {
                x = m;
                gx = gm;
            } else {
                y = m;
            }
        }
        Some(x.lerp(y, 0.5))
    };
    // edges: horizontal (i, j)-(i+1, j) as 2 * (j * (nu + 1) + i), vertical (i, j)-(i, j+1) as + 1
    let key = |i: usize, j: usize, vert: bool| 2 * (j * (nu + 1) + i) + usize::from(vert);
    let mut segs: Vec<(usize, usize)> = Vec::new();
    let mut pts: std::collections::HashMap<usize, Point2> = std::collections::HashMap::new();
    for j in 0..nv {
        for i in 0..nu {
            let es = [
                (key(i, j, false), (i, j), (i + 1, j)),
                (key(i + 1, j, true), (i + 1, j), (i + 1, j + 1)),
                (key(i, j + 1, false), (i, j + 1), (i + 1, j + 1)),
                (key(i, j, true), (i, j), (i, j + 1)),
            ];
            let mut hit = Vec::new();
            for (k, a, b) in es {
                if let Some(p) = pts.get(&k) {
                    hit.push((k, *p));
                } else if let Some(p) = cross(a, b) {
                    pts.insert(k, p);
                    hit.push((k, p));
                }
            }
            match hit.len() {
                2 => segs.push((hit[0].0, hit[1].0)),
                4 => {
                    segs.push((hit[0].0, hit[1].0));
                    segs.push((hit[2].0, hit[3].0));
                }
                _ => {}
            }
        }
    }
    // chain the segments into polylines
    let mut by: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for (k, &(a, b)) in segs.iter().enumerate() {
        by.entry(a).or_default().push(k);
        by.entry(b).or_default().push(k);
    }
    let mut used = vec![false; segs.len()];
    let mut out = Vec::new();
    for s0 in 0..segs.len() {
        if used[s0] {
            continue;
        }
        used[s0] = true;
        let mut chain = std::collections::VecDeque::from([segs[s0].0, segs[s0].1]);
        for front in [false, true] {
            loop {
                let end = if front { chain[0] } else { chain[chain.len() - 1] };
                let Some(&next) = by[&end].iter().find(|&&k| !used[k]) else {
                    break;
                };
                used[next] = true;
                let (a, b) = segs[next];
                let other = if a == end { b } else { a };
                if front {
                    chain.push_front(other);
                } else {
                    chain.push_back(other);
                }
            }
        }
        let p: Vec<Point3> = chain.iter().map(|k| s.eval(pts[k])).collect();
        if p.len() >= 2 {
            out.push(Path3::Poly(p));
        }
    }
    out
}

/// The faces of every instance, binned by their boxes on the view, for the visibility rays.
struct Scene<'a> {
    insts: &'a [Instance<'a>],
    idx: Vec<Index<'a>>,
    view: View,
    faces: Vec<(usize, FaceId, Point2, Point2, f64)>,
    /// For a face with no closed form, its mesh: a ray whose point is farther than `COVER` from
    /// all of its triangles on the view misses the face; one nearer starts Newton's method there.
    cover: Vec<Option<Cover>>,
    grid: Grid,
    eps: f64,
}

/// How far a procedural face's mesh may be from it on the view, with room to spare.
const COVER: f64 = 0.1;

/// A procedural face's mesh: each triangle in the body's space, on the view, and in the face's
/// parameters (its corners projected onto the surface).
type Cover = Vec<([Point3; 3], [Point2; 3], [Point2; 3])>;

/// A body's procedural faces' meshes in its own space, with their corners' parameters, by face
/// index: the same for every occurrence of the body, so made once for it.
type Local = Vec<Option<Vec<([Point3; 3], [Point2; 3])>>>;

fn local_covers(body: &Body) -> Local {
    let mut out = vec![None; body.faces.len()];
    if !body
        .face_ids()
        .any(|f| matches!(body.face(f).surface, Surface::Procedural(_)))
    {
        return out;
    }
    let q = forge_mesh::Quality {
        chord: 0.02,
        max_angle: dmath::PI / 8.0,
    };
    let Ok(mesh) = forge_mesh::tessellate(body, &q) else {
        return out;
    };
    // each corner's parameters once, on the face its triangle is of
    let mut uv: std::collections::HashMap<(u32, u32), Point2> = Default::default();
    for (t, &f) in mesh.triangles.iter().zip(&mesh.face_of) {
        let surface = &body.faces[f as usize].surface;
        if !matches!(surface, Surface::Procedural(_)) {
            continue;
        }
        let tri = t.map(|k| mesh.positions[k as usize]);
        let at = t.map(|k| {
            *uv.entry((f, k))
                .or_insert_with(|| surface.project(mesh.positions[k as usize]))
        });
        out[f as usize].get_or_insert_with(Vec::new).push((tri, at));
    }
    out
}

/// `local_covers` placed at `pose` and seen on the view.
fn covers(local: &Local, pose: &Frame, view: &View) -> Vec<Option<Cover>> {
    local
        .iter()
        .map(|c| {
            c.as_ref().map(|tris| {
                tris.iter()
                    .map(|(tri, uv)| (*tri, tri.map(|p| view.project(pose.point_to_world(p))), *uv))
                    .collect()
            })
        })
        .collect()
}

/// Where the ray `o + t d` meets `s`, by Newton's method on (u, v, t) from parameters `uv` near
/// the point `seed`, at `t_seed`: the ray's parameter and the point; None where it does not settle.
fn newton_ray(s: &Surface, o: Point3, d: Vec3, uv: Point2, t_seed: f64) -> Option<(f64, Point3)> {
    let mut uv = uv;
    let mut t = t_seed;
    for _ in 0..20 {
        let sd = s.deriv1(uv);
        let r = sd.p - (o + d * t);
        if r.length() <= 1e-10 {
            return Some((t, sd.p));
        }
        // [du dv -d] (x, y, z) = -r
        let (a, b, c) = (sd.du, sd.dv, d * -1.0);
        let det = a.dot(b.cross(c));
        if det.abs() <= 1e-300 {
            return None;
        }
        let x = -r.dot(b.cross(c)) / det;
        let y = -a.dot(r.cross(c)) / det;
        let z = -a.dot(b.cross(r)) / det;
        uv = Point2::new(uv.x + x, uv.y + y);
        t += z;
    }
    None
}

/// How far `q` is from the triangle (0 inside).
fn tri_distance(q: Point2, t: &[Point2; 3]) -> f64 {
    let area = (t[1] - t[0]).cross(t[2] - t[0]);
    let side = |a: Point2, b: Point2| (b - a).cross(q - a) * area.signum();
    if area.abs() > 1e-300 && side(t[0], t[1]) >= 0.0 && side(t[1], t[2]) >= 0.0 && side(t[2], t[0]) >= 0.0 {
        return 0.0;
    }
    let seg = |a: Point2, b: Point2| {
        let ab = b - a;
        let x = ((q - a).dot(ab) / ab.dot(ab).max(1e-300)).clamp(0.0, 1.0);
        q.distance(a + ab * x)
    };
    seg(t[0], t[1]).min(seg(t[1], t[2])).min(seg(t[2], t[0]))
}

fn box_on_view(view: &View, pose: &Frame, b: &forge_math::Box3) -> (Point2, Point2, f64) {
    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut near = f64::NEG_INFINITY;
    for k in 0..8 {
        let p = Point3::new(
            if k & 1 == 0 { b.x.lo } else { b.x.hi },
            if k & 2 == 0 { b.y.lo } else { b.y.hi },
            if k & 4 == 0 { b.z.lo } else { b.z.hi },
        );
        let w = pose.point_to_world(p);
        let q = view.project(w);
        lo = Point2::new(lo.x.min(q.x), lo.y.min(q.y));
        hi = Point2::new(hi.x.max(q.x), hi.y.max(q.y));
        near = near.max(view.depth(w));
    }
    (lo, hi, near)
}

impl<'a> Scene<'a> {
    fn new(insts: &'a [Instance<'a>], view: View) -> Scene<'a> {
        let idx: Vec<Index> = insts.iter().map(|i| Index::new(i.body)).collect();
        let mut faces = Vec::new();
        let mut cover: Vec<Option<Cover>> = Vec::new();
        let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
        let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        let mut locals: std::collections::HashMap<usize, Local> = Default::default();
        for (k, inst) in insts.iter().enumerate() {
            let local = locals
                .entry(inst.body as *const Body as usize)
                .or_insert_with(|| local_covers(inst.body));
            let mut tris = covers(local, &inst.pose, &view);
            for f in inst.body.face_ids() {
                cover.push(tris[f.index()].take());
                let (a, b, near) = box_on_view(&view, &inst.pose, &idx[k].face_box(f));
                lo = Point2::new(lo.x.min(a.x), lo.y.min(a.y));
                hi = Point2::new(hi.x.max(b.x), hi.y.max(b.y));
                faces.push((k, f, a, b, near));
            }
        }
        let mut grid = Grid::new(lo, hi, faces.len());
        for (k, f) in faces.iter().enumerate() {
            grid.insert(k as u32, f.2, f.3);
        }
        let size = (hi.x - lo.x).max(hi.y - lo.y).max(1.0);
        Scene {
            insts,
            idx,
            view,
            faces,
            cover,
            grid,
            eps: 1e-7 * size + 1e-5,
        }
    }

    /// Whether face `k` (of `faces`) lies between `p` and the eye.
    fn face_blocks(&self, k: usize, p: Point3, q: Point2, depth: f64) -> bool {
        let (i, f, lo, hi, near) = self.faces[k];
        if q.x < lo.x || q.x > hi.x || q.y < lo.y || q.y > hi.y || near <= depth + self.eps {
            return false;
        }
        match &self.cover[k] {
            Some(tris) => self.blocks_meshed(i, f, p, q, tris),
            None => self.blocks(i, f, p),
        }
    }

    /// Whether a face lies between `p` and the eye, trying first the face that hid the last point
    /// asked about (`last`, kept up to date): along a curve one face mostly hides the next point
    /// too. Each face's test stands alone, so the order they are tried in does not change the answer.
    fn hidden_from(&self, p: Point3, last: &mut Option<u32>) -> bool {
        let q = self.view.project(p);
        let depth = self.view.depth(p);
        if let Some(k) = *last
            && self.face_blocks(k as usize, p, q, depth)
        {
            return true;
        }
        // faces with a closed form first: a hit there spares the costly ones
        for costly in [false, true] {
            for &k in self.grid.at(q) {
                if self.cover[k as usize].is_some() != costly || Some(k) == *last {
                    continue;
                }
                if self.face_blocks(k as usize, p, q, depth) {
                    *last = Some(k);
                    return true;
                }
            }
        }
        false
    }

    /// `blocks` for a procedural face: the ray's exact meeting with the surface found by Newton's
    /// method from each triangle of its mesh it passes near; the subdivision search only where
    /// that does not settle.
    fn blocks_meshed(&self, i: usize, f: FaceId, p: Point3, q: Point2, tris: &Cover) -> bool {
        let pose = &self.insts[i].pose;
        let surface = &self.insts[i].body.face(f).surface;
        let o = pose.point_to_local(p);
        let d = pose.vec_to_local(self.view.toward.get());
        let mut tried: Vec<f64> = Vec::new();
        for (t3, t2, tuv) in tris {
            if tri_distance(q, t2) > COVER {
                continue;
            }
            // the ray's point over the triangle, from q's barycentric place (clamped)
            let area = (t2[1] - t2[0]).cross(t2[2] - t2[0]);
            let (b1, b2) = if area.abs() > 1e-12 {
                let b1 = ((q - t2[0]).cross(t2[2] - t2[0]) / area).clamp(0.0, 1.0);
                let b2 = ((t2[1] - t2[0]).cross(q - t2[0]) / area).clamp(0.0, 1.0 - b1);
                (b1, b2)
            } else {
                (0.0, 0.0)
            };
            let seed = t3[0] + (t3[1] - t3[0]) * b1 + (t3[2] - t3[0]) * b2;
            // the corners' parameters carried the same way: a start as near as the mesh, with no
            // search of the surface
            let uv = tuv[0] + (tuv[1] - tuv[0]) * b1 + (tuv[2] - tuv[0]) * b2;
            let t_seed = (seed - o).dot(d);
            if tried.iter().any(|x| (x - t_seed).abs() <= 1e-3) {
                continue;
            }
            tried.push(t_seed);
            match newton_ray(surface, o, d, uv, t_seed) {
                Some((t, at)) => {
                    if t > self.eps && self.idx[i].point_in_face(f, at) == Where::Inside {
                        return true;
                    }
                }
                None => return self.blocks(i, f, p),
            }
        }
        false
    }

    /// Whether face `f` of instance `i` lies between `p` and the eye.
    fn blocks(&self, i: usize, f: FaceId, p: Point3) -> bool {
        let pose = &self.insts[i].pose;
        let Some(dir) = pose.vec_to_local(self.view.toward.get()).normalize() else {
            return false;
        };
        let line = Line::new(pose.point_to_local(p), dir);
        let LineSurface::Hits(hits) = line_surface(&line, &self.insts[i].body.face(f).surface) else {
            return false;
        };
        hits.iter().any(|h| {
            h.t > self.eps
                && h.contact == Contact::Crossing
                && self.idx[i].point_in_face(f, h.point) == Where::Inside
        })
    }
}

/// The view of the instances: every edge (tangent ones if asked) and every outline, split where
/// they cross on the view, each piece seen or hidden; a hidden piece lying on a seen one, and a
/// second copy of one, are left out.
pub fn hlr(insts: &[Instance], view: &View, opts: &Options) -> Vec<Piece> {
    use forge_math::trace::span;
    let _all = span("hlr");
    let t = span("hlr scene");
    let scene = Scene::new(insts, *view);
    drop(t);
    let t = span("hlr curves");
    // each part's curves on a core of its own, put together in part order
    // a part's edges and outlines in its own space depend on the part and the view's direction
    // there alone: made once per such pair, placed at each occurrence
    let keys: Vec<(usize, [u64; 3])> = insts
        .iter()
        .map(|inst| {
            let d = inst.pose.vec_to_local(view.toward.get());
            (
                inst.body as *const Body as usize,
                [d.x.to_bits(), d.y.to_bits(), d.z.to_bits()],
            )
        })
        .collect();
    let mut first: std::collections::HashMap<(usize, [u64; 3]), usize> = Default::default();
    let mut slot = Vec::with_capacity(insts.len());
    let mut reps: Vec<usize> = Vec::new();
    for (i, k) in keys.iter().enumerate() {
        let n = reps.len();
        let s = *first.entry(*k).or_insert(n);
        if s == n {
            reps.push(i);
        }
        slot.push(s);
    }
    let local: Vec<Vec<(Source, Class, Path3)>> = forge_math::par::map(reps.len(), |r| {
        let i = reps[r];
        let inst = &insts[i];
        let body = inst.body;
        let d = inst.pose.vec_to_local(view.toward.get());
        let mut out = Vec::new();
        for e in body.edge_ids() {
            let edge = body.edge(e);
            let faces: Vec<FaceId> = edge.coedges.iter().map(|&c| body.coedge_face(c)).collect();
            let class = match faces[..] {
                [a, b] if a == b => continue,
                [a, b] if smooth(body, e, [a, b]) => Class::Tangent,
                _ => Class::Sharp,
            };
            if class == Class::Tangent && !opts.tangent {
                continue;
            }
            out.push((Source::Edge(edge.name), class, edge_path(body, e, opts.chord)));
        }
        for f in body.face_ids() {
            for path in outlines(body, &scene.idx[i], f, d, opts.chord) {
                out.push((Source::Outline(body.face(f).name), Class::Outline, path));
            }
        }
        out
    });
    let per_inst: Vec<Vec<Curve3>> = forge_math::par::map(insts.len(), |i| {
        let mut curves: Vec<Curve3> = Vec::new();
        for (source, class, path) in &local[slot[i]] {
            push_curve(&mut curves, view, i, *source, *class, path.placed(&insts[i].pose));
        }
        curves
    });
    let curves: Vec<Curve3> = per_inst.into_iter().flatten().collect();
    drop(t);
    let t = span("hlr crossings");
    let splits = crossings(&curves, view, opts.chord);
    drop(t);
    let t = span("hlr visibility");
    let runs_of: Vec<Vec<(f64, f64, bool)>> = forge_math::par::map(curves.len(), |k| {
        let c = &curves[k];
        let mut runs: Vec<(f64, f64, bool)> = Vec::new();
        let mut last = None;
        let ts = &splits[k];
        for w in ts.windows(2) {
            let (a, b) = (w[0], w[1]);
            if b - a <= 1e-12 * (1.0 + b.abs()) {
                continue;
            }
            for (x, y, vis) in visibility(&scene, &c.path, a, b, &mut last) {
                match runs.last_mut() {
                    Some(r) if r.2 == vis && (r.1 - x).abs() <= 1e-12 * (1.0 + x.abs()) => r.1 = y,
                    _ => runs.push((x, y, vis)),
                }
            }
        }
        runs
    });
    let mut pieces: Vec<(usize, Path3, bool)> = Vec::new();
    for (k, runs) in runs_of.into_iter().enumerate() {
        for (a, b, vis) in runs {
            pieces.push((k, curves[k].path.sub(a, b), vis));
        }
    }
    drop(t);
    let t = span("hlr unique");
    let geoms: Vec<Geom2> = pieces
        .iter()
        .map(|(_, p, _)| {
            let (a, b) = p.range();
            p.geom(view, a, b)
        })
        .collect();
    let order: Vec<usize> = {
        let mut o: Vec<usize> = (0..pieces.len()).collect();
        // seen pieces win over hidden ones, then the earlier piece over the later
        o.sort_by_key(|&k| (!pieces[k].2, k));
        o
    };
    let curve: Vec<usize> = pieces.iter().map(|p| p.0).collect();
    let kept = unique(&geoms, &curve, &order, opts.chord);

    drop(t);
    let mut out = Vec::new();
    for (k, runs) in kept.into_iter().enumerate() {
        let c = &curves[pieces[k].0];
        for geom in runs {
            out.push(Piece {
                inst: c.inst,
                source: c.source,
                class: c.class,
                visible: pieces[k].2,
                geom,
            });
        }
    }
    out
}

/// Lines drawn by convention among the parts (a thread's minor diameter, its end): each polyline,
/// in model space, as its runs seen (true) and hidden on the view.
pub fn seen_runs(insts: &[Instance], view: &View, lines: &[Vec<Point3>]) -> Vec<Vec<(Geom2, bool)>> {
    if lines.is_empty() {
        return vec![];
    }
    let scene = Scene::new(insts, *view);
    forge_math::par::map(lines.len(), |k| {
        if lines[k].len() < 2 {
            return vec![];
        }
        let path = Path3::Poly(lines[k].clone());
        let (a, b) = path.range();
        visibility(&scene, &path, a, b, &mut None)
            .into_iter()
            .map(|(x, y, vis)| {
                let p = path.sub(x, y);
                let (a, b) = p.range();
                (p.geom(view, a, b), vis)
            })
            .collect()
    })
}

/// Adds a curve, split where it turns back on the view (a circle seen edge-on is drawn there and
/// back on one line) and left out if it is only a point on the view.
fn push_curve(curves: &mut Vec<Curve3>, view: &View, inst: usize, source: Source, class: Class, path: Path3) {
    let (t0, t1) = path.range();
    let mut cuts = vec![t0, t1];
    if let Path3::Conic { u, v, .. } = &path {
        let (u2, v2) = (view.project_vec(*u), view.project_vec(*v));
        if flat(u2, v2, u.length().max(v.length())) {
            let w = if u2.length() >= v2.length() { u2 } else { v2 };
            let Some(w) = w.normalize() else { return };
            let t = dmath::atan2(v2.dot(w), u2.dot(w));
            for k in -4..=4 {
                let x = t + k as f64 * dmath::PI;
                if x > t0 + 1e-12 && x < t1 - 1e-12 {
                    cuts.push(x);
                }
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    for w in cuts.windows(2) {
        let p = path.sub(w[0], w[1]);
        let (a, b) = p.range();
        // only a point on the view: its length summed till it passes the least that is drawn
        let least = 1e-9 * (1.0 + view.project(p.at(a)).to_vec().length());
        let mut len = 0.0;
        for x in p.ts(1e-3).windows(2) {
            len += view.project(p.at(x[0])).distance(view.project(p.at(x[1])));
            if len > least {
                break;
            }
        }
        if len <= least {
            continue;
        }
        let _ = b;
        curves.push(Curve3 {
            inst,
            source,
            class,
            path: p,
        });
    }
}

/// Every curve's split parameters: its ends and where it crosses another curve on the view.
fn crossings(curves: &[Curve3], view: &View, chord: f64) -> Vec<Vec<f64>> {
    let samples: Vec<(Vec<f64>, Vec<Point2>)> = forge_math::par::map(curves.len(), |k| {
        let c = &curves[k];
        let ts = c.path.ts(chord);
        let ps = ts.iter().map(|&t| view.project(c.path.at(t))).collect();
        (ts, ps)
    });
    let mut segs: Vec<(u32, u32)> = Vec::new();
    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for (c, (_, ps)) in samples.iter().enumerate() {
        for (s, p) in ps.iter().enumerate() {
            lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
            if s + 1 < ps.len() {
                segs.push((c as u32, s as u32));
            }
        }
    }
    let mut out: Vec<Vec<f64>> = curves
        .iter()
        .map(|c| {
            let (a, b) = c.path.range();
            vec![a, b]
        })
        .collect();
    if segs.is_empty() {
        return out;
    }
    let seg = |k: usize| {
        let (c, s) = segs[k];
        let ps = &samples[c as usize].1;
        (ps[s as usize], ps[s as usize + 1])
    };
    let mut grid = Grid::new(lo, hi, segs.len());
    for k in 0..segs.len() {
        let (a, b) = seg(k);
        grid.insert(
            k as u32,
            Point2::new(a.x.min(b.x), a.y.min(b.y)),
            Point2::new(a.x.max(b.x), a.y.max(b.y)),
        );
    }
    // each segment's box, side by side: the test most pairs fail, without going to the samples
    let boxes: Vec<[f64; 4]> = (0..segs.len())
        .map(|k| {
            let (a, b) = seg(k);
            [a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y)]
        })
        .collect();
    // every crossing, each pair of segments taken once: in the cell holding the low corner of
    // where their boxes meet, a run of cells to a core at a time (the splits are sorted below,
    // so the order they are found in does not matter)
    const CELLS: usize = 256;
    let found: Vec<Vec<(u32, f64, u32, f64)>> = forge_math::par::map(grid.cells().div_ceil(CELLS), |n| {
        let mut got = Vec::new();
        for cell_k in n * CELLS..((n + 1) * CELLS).min(grid.cells()) {
            let (cell, ids) = grid.cell_at(cell_k);
            for (x, &k) in ids.iter().enumerate() {
                let k = k as usize;
                let bk = boxes[k];
                let (a, b) = seg(k);
                for &j in &ids[x + 1..] {
                    let j = j as usize;
                    let bj = boxes[j];
                    if bj[2] < bk[0] || bj[0] > bk[2] || bj[3] < bk[1] || bj[1] > bk[3] {
                        continue;
                    }
                    let ((ci, si), (cj, sj)) = (segs[k], segs[j]);
                    if ci == cj && si.abs_diff(sj) <= 1 {
                        continue;
                    }
                    let corner = Point2::new(bk[0].max(bj[0]), bk[1].max(bj[1]));
                    if grid.cell(corner) != cell {
                        continue;
                    }
                    let (c, d) = seg(j);
                    let Some((x, y)) = seg_cross(a, b, c, d) else {
                        continue;
                    };
                    let (ti, tj) = (&samples[ci as usize].0, &samples[cj as usize].0);
                    let ta = ti[si as usize] + x * (ti[si as usize + 1] - ti[si as usize]);
                    let tb = tj[sj as usize] + y * (tj[sj as usize + 1] - tj[sj as usize]);
                    let (ta, tb) = refine(&curves[ci as usize].path, &curves[cj as usize].path, view, ta, tb);
                    got.push((ci, ta, cj, tb));
                }
            }
        }
        got
    });
    for (ci, ta, cj, tb) in found.into_iter().flatten() {
        out[ci as usize].push(ta);
        out[cj as usize].push(tb);
    }
    // where a curve ends on another (an outline ending where it touches a rim, which it meets
    // tangentially, so no crossing shows it): split the other there
    let size = (hi.x - lo.x).max(hi.y - lo.y).max(1.0);
    let touch = 1e-7 * size + 1e-7;
    let ends: Vec<Vec<(u32, f64)>> = forge_math::par::map(samples.len(), |c| {
        let ps = &samples[c].1;
        let mut got = Vec::new();
        for e in [ps[0], ps[ps.len() - 1]] {
            let r = chord + touch;
            // a segment in two of the cells is seen twice: the same split twice, which the
            // sort and dedup below make one
            grid.each_near(
                Point2::new(e.x - r, e.y - r),
                Point2::new(e.x + r, e.y + r),
                |k| {
                    let bk = boxes[k as usize];
                    if bk[2] < e.x - r || bk[0] > e.x + r || bk[3] < e.y - r || bk[1] > e.y + r {
                        return;
                    }
                    let (ck, sk) = segs[k as usize];
                    if ck as usize == c {
                        return;
                    }
                    let (a, b) = seg(k as usize);
                    let ab = b - a;
                    let x = ((e - a).dot(ab) / ab.dot(ab).max(1e-300)).clamp(0.0, 1.0);
                    if e.distance(a + ab * x) > r {
                        return;
                    }
                    let ts = &samples[ck as usize].0;
                    let guess = ts[sk as usize] + x * (ts[sk as usize + 1] - ts[sk as usize]);
                    let path = &curves[ck as usize].path;
                    let t = nearest_on_view(path, view, e, guess);
                    if view.project(path.at(t)).distance(e) <= touch {
                        got.push((ck, t));
                    }
                },
            );
        }
        got
    });
    for (ck, t) in ends.into_iter().flatten() {
        out[ck as usize].push(t);
    }
    for (k, ts) in out.iter_mut().enumerate() {
        let (a, b) = curves[k].path.range();
        ts.retain(|t| *t >= a && *t <= b);
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|x, y| (*x - *y).abs() <= 1e-12 * (1.0 + x.abs()));
    }
    out
}

/// The parameter of `path`'s point nearest `q` on the view, by Gauss-Newton steps from `guess`.
fn nearest_on_view(path: &Path3, view: &View, q: Point2, guess: f64) -> f64 {
    let (lo, hi) = path.range();
    if let Path3::Poly(_) = path {
        return guess;
    }
    let h = 1e-7 * (hi - lo);
    let mut t = guess;
    for _ in 0..8 {
        let p = |t: f64| view.project(path.at(t));
        let d1 = (p(t + h) - p(t - h)) * (0.5 / h);
        let r = p(t) - q;
        let den = d1.dot(d1);
        if den.abs() <= 1e-300 {
            break;
        }
        let next = (t - r.dot(d1) / den).clamp(lo, hi);
        // a step that changes nothing: every step after it would be the same one
        if next == t {
            break;
        }
        t = next;
    }
    t
}

/// Where segments `ab` and `cd` cross: the fraction along each; None for parallel ones.
fn seg_cross(a: Point2, b: Point2, c: Point2, d: Point2) -> Option<(f64, f64)> {
    let (r, s) = (b - a, d - c);
    let den = r.cross(s);
    if den.abs() <= 1e-300 {
        return None;
    }
    let w = c - a;
    let (x, y) = (w.cross(s) / den, w.cross(r) / den);
    ((0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y)).then_some((x, y))
}

/// The exact crossing near `(ta, tb)` by Newton's method on the projected curves; the guess where
/// it does not settle.
fn refine(p: &Path3, q: &Path3, view: &View, ta: f64, tb: f64) -> (f64, f64) {
    if matches!(p, Path3::Poly(_)) && matches!(q, Path3::Poly(_)) {
        return (ta, tb);
    }
    let (pa, pb) = (p.range(), q.range());
    let f = |a: f64, b: f64| view.project(p.at(a)) - view.project(q.at(b));
    let (mut a, mut b) = (ta, tb);
    let (ha, hb) = (1e-7 * (pa.1 - pa.0), 1e-7 * (pb.1 - pb.0));
    for _ in 0..8 {
        let r = f(a, b);
        let da = (view.project(p.at(a + ha)) - view.project(p.at(a - ha))) * (0.5 / ha);
        let db = (view.project(q.at(b + hb)) - view.project(q.at(b - hb))) * (-0.5 / hb);
        let det = da.cross(db);
        if det.abs() <= 1e-300 {
            return (ta, tb);
        }
        // solve [da db] (x, y) = -r
        let x = -(r.cross(db)) / det;
        let y = -(da.cross(r)) / det;
        let (na, nb) = (a + x, b + y);
        // a step that changes nothing: every step after it would be the same one
        if na == a && nb == b {
            break;
        }
        (a, b) = (na, nb);
    }
    let ok = f(a, b).length() <= 1e-9 * (1.0 + view.project(p.at(a)).to_vec().length())
        && a >= pa.0
        && a <= pa.1
        && b >= pb.0
        && b <= pb.1
        && (a - ta).abs() <= 0.1 * (pa.1 - pa.0)
        && (b - tb).abs() <= 0.1 * (pb.1 - pb.0);
    if ok { (a, b) } else { (ta, tb) }
}

/// Seen or hidden along `path` from `a` to `b`: a quarter, the middle and three quarters looked at,
/// and where they disagree the change found by bisection.
fn visibility(scene: &Scene, path: &Path3, a: f64, b: f64, last: &mut Option<u32>) -> Vec<(f64, f64, bool)> {
    let ks = [0.25, 0.5, 0.75];
    let mut ts: Vec<f64> = ks.iter().map(|k| a + k * (b - a)).collect();
    // a polyline's points lie on the curve, its chords do not (inside a thin wire by their sag)
    if let Path3::Poly(_) = path {
        let mut at: Vec<f64> = ts
            .iter()
            .map(|t| t.round())
            .filter(|t| *t > a && *t < b)
            .collect();
        at.dedup();
        if !at.is_empty() {
            ts = at;
        }
    }
    let vis: Vec<bool> = ts.iter().map(|&t| !scene.hidden_from(path.at(t), last)).collect();
    if vis.iter().all(|&v| v == vis[0]) {
        return vec![(a, b, vis[0])];
    }
    let mut out = Vec::new();
    let mut start = a;
    for k in 1..ts.len() {
        if vis[k] != vis[k - 1] {
            let (mut x, mut y) = (ts[k - 1], ts[k]);
            for _ in 0..50 {
                let m = 0.5 * (x + y);
                // x and y side by side: every step after would be this one
                if m == x || m == y {
                    break;
                }
                if !scene.hidden_from(path.at(m), last) == vis[k - 1] {
                    x = m;
                } else {
                    y = m;
                }
            }
            let m = 0.5 * (x + y);
            out.push((start, m, vis[k - 1]));
            start = m;
        }
    }
    out.push((start, b, vis[ts.len() - 1]));
    out
}

/// What is left of each piece once the parts lying on a piece before it in `order` (and of another
/// curve) are taken out. Pieces too short to draw cover nothing.
fn unique(geoms: &[Geom2], curve: &[usize], order: &[usize], chord: f64) -> Vec<Vec<Geom2>> {
    let fine = chord.min(1e-3);
    let samples: Vec<(Vec<f64>, Vec<Point2>)> = forge_math::par::map(geoms.len(), |k| {
        let g = &geoms[k];
        if g.length() <= 1e-6 {
            return (Vec::new(), Vec::new());
        }
        let ts = g.params(fine);
        let ps = ts.iter().map(|&t| g.at(t)).collect();
        (ts, ps)
    });
    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut segs: Vec<(u32, u32)> = Vec::new();
    for (g, (_, ps)) in samples.iter().enumerate() {
        for (s, p) in ps.iter().enumerate() {
            lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
            if s + 1 < ps.len() {
                segs.push((g as u32, s as u32));
            }
        }
    }
    if segs.is_empty() {
        return vec![Vec::new(); geoms.len()];
    }
    let size = (hi.x - lo.x).max(hi.y - lo.y).max(1.0);
    // how near a curve must lie to another to be the same line on the sheet
    let tol = 1e-7 * size + 1e-6;
    let reach = tol + fine;
    let mut rank = vec![0usize; geoms.len()];
    for (r, &k) in order.iter().enumerate() {
        rank[k] = r;
    }
    let mut grid = Grid::new(lo, hi, segs.len());
    for (k, &(g, s)) in segs.iter().enumerate() {
        let ps = &samples[g as usize].1;
        let (a, b) = (ps[s as usize], ps[s as usize + 1]);
        grid.insert(
            k as u32,
            Point2::new(a.x.min(b.x) - reach, a.y.min(b.y) - reach),
            Point2::new(a.x.max(b.x) + reach, a.y.max(b.y) + reach),
        );
    }
    // every piece's ends, so a run between two samples is not missed
    let mut ends = Grid::new(lo, hi, 2 * geoms.len());
    for (k, g) in geoms.iter().enumerate() {
        if !samples[k].0.is_empty() {
            for p in [g.start(), g.end()] {
                ends.insert(k as u32, p, p);
            }
        }
    }
    forge_math::par::map(geoms.len(), |k| {
        let geom = &geoms[k];
        if samples[k].0.is_empty() {
            return Vec::new();
        }
        let mut near = Vec::new();
        let (ts, ps) = &samples[k];
        // the earlier pieces of other curves that come within reach of this one: only they can
        // cover it (a point of it is within `fine` of its chords), so the test below looks at them
        // alone; none, and it is kept whole
        // the segments of earlier pieces of other curves that can cover a point of this one: within
        // reach of a point of it (so within reach and its chords' sag of a chord), and for two lines
        // only one along it
        let sag = match geom {
            Geom2::Conic { u, v, t0, t1, .. } => {
                let r = u.length().max(v.length());
                let n = crate::geom2::conic_steps(r, t1 - t0, fine);
                r * (1.0 - dmath::cos(0.5 * (t1 - t0).abs() / n as f64))
            }
            _ => 0.0,
        };
        let m = reach + 2.0 * sag.max(fine) + 1e-9;
        let line_dir = match geom {
            Geom2::Line { .. } => Some(geom.tangent(0.0)),
            _ => None,
        };
        let mut cand: Vec<u32> = Vec::new();
        let chords: Vec<(Point2, Point2)> = if ps.len() == 1 {
            vec![(ps[0], ps[0])]
        } else {
            ps.windows(2).map(|w| (w[0], w[1])).collect()
        };
        // the chords a block at a time: one look in the grid for each run of them
        let pad_box = |a: Point2, b: Point2| {
            (
                Point2::new(a.x.min(b.x) - m, a.y.min(b.y) - m),
                Point2::new(a.x.max(b.x) + m, a.y.max(b.y) + m),
            )
        };
        let mut items: Vec<u32> = Vec::new();
        let mut marked = vec![false; chords.len()];
        for (b, block) in chords.chunks(16).enumerate() {
            let first = 16 * b;
            let boxes: Vec<(Point2, Point2)> = block.iter().map(|&(a, b)| pad_box(a, b)).collect();
            let lo = boxes
                .iter()
                .fold(boxes[0].0, |l, x| Point2::new(l.x.min(x.0.x), l.y.min(x.0.y)));
            let hi = boxes
                .iter()
                .fold(boxes[0].1, |h, x| Point2::new(h.x.max(x.1.x), h.y.max(x.1.y)));
            items.clear();
            // each once: in the cell holding the low corner of where its box (as it went in)
            // meets the look's; one whose box misses the look's is near none of the chords
            grid.each_near_in(lo, hi, |s, cell| {
                let (g, i) = segs[s as usize];
                if curve[g as usize] == curve[k] || rank[g as usize] >= rank[k] {
                    return;
                }
                let op = &samples[g as usize].1;
                let (c, d) = (op[i as usize], op[i as usize + 1]);
                let (bl, bh) = (
                    Point2::new(c.x.min(d.x) - reach, c.y.min(d.y) - reach),
                    Point2::new(c.x.max(d.x) + reach, c.y.max(d.y) + reach),
                );
                if bh.x < lo.x || bl.x > hi.x || bh.y < lo.y || bl.y > hi.y {
                    return;
                }
                if grid.cell(Point2::new(lo.x.max(bl.x), lo.y.max(bl.y))) == cell {
                    items.push(s);
                }
            });
            for &s in &items {
                let (g, i) = segs[s as usize];
                let op = &samples[g as usize].1;
                let (c, d) = (op[i as usize], op[i as usize + 1]);
                let other = &geoms[g as usize];
                if let (Some(dir), Geom2::Line { .. }) = (line_dir, other) {
                    match (dir, other.tangent(0.0)) {
                        (Some(dir), Some(d)) if d.cross(dir).abs() <= 0.05 => {}
                        _ => continue,
                    }
                }
                let mut near_one = false;
                for (q, (&(a, b), &(lo, hi))) in block.iter().zip(&boxes).enumerate() {
                    if !(c.x.max(d.x) < lo.x
                        || c.x.min(d.x) > hi.x
                        || c.y.max(d.y) < lo.y
                        || c.y.min(d.y) > hi.y)
                        && seg_distance(a, b, c, d) <= m
                    {
                        // which chords it comes near: a point of the piece within reach of it lies
                        // within its chord's sag of one of those, so only there can it be covered
                        marked[first + q] = true;
                        near_one = true;
                    }
                }
                if near_one {
                    cand.push(s);
                }
            }
        }
        cand.sort_unstable();
        cand.dedup();
        let alone = cand.is_empty();
        if alone {
            let (t0, t1) = geom.range();
            return if t1 > t0 { vec![geom.sub(t0, t1)] } else { vec![] }
                .into_iter()
                .filter(|g| g.length() > 10.0 * tol)
                .collect();
        }
        // a line covered by lines alone: what each covers of it is where it passes within `tol`
        // of that segment, an interval found in closed form, and what is left is kept
        if let Geom2::Line { a: pa, b: pb } = *geom {
            let lines: Option<Vec<(Point2, Point2)>> = cand
                .iter()
                .map(|&s| match geoms[segs[s as usize].0 as usize] {
                    Geom2::Line { a, b } => Some((a, b)),
                    _ => None,
                })
                .collect();
            if let Some(mut lines) = lines {
                lines.sort_by(|x, y| {
                    (x.0.x, x.0.y, x.1.x, x.1.y)
                        .partial_cmp(&(y.0.x, y.0.y, y.1.x, y.1.y))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                lines.dedup();
                let mut cov: Vec<(f64, f64)> = lines
                    .iter()
                    .filter_map(|&(c, d)| near_segment(pa, pb, c, d, tol))
                    .filter_map(|(x, y)| {
                        let (x, y) = (x.max(0.0), y.min(1.0));
                        (y > x).then_some((x, y))
                    })
                    .collect();
                cov.sort_by(|x, y| x.0.total_cmp(&y.0));
                let mut out = Vec::new();
                let mut at = 0.0;
                for (x, y) in cov {
                    if x > at {
                        out.push(geom.sub(at, x));
                    }
                    at = at.max(y);
                }
                if at < 1.0 {
                    out.push(geom.sub(at, 1.0));
                }
                return out.into_iter().filter(|g| g.length() > 10.0 * tol).collect();
            }
        }
        let mut also = Vec::new();
        for (i, w) in ps.windows(2).enumerate() {
            let (a, b) = (w[0], w[1]);
            ends.near(
                Point2::new(a.x.min(b.x) - reach, a.y.min(b.y) - reach),
                Point2::new(a.x.max(b.x) + reach, a.y.max(b.y) + reach),
                &mut near,
            );
            for &j in &near {
                if j as usize == k {
                    continue;
                }
                for e in [geoms[j as usize].start(), geoms[j as usize].end()] {
                    let ab = b - a;
                    let x = ((e - a).dot(ab) / ab.dot(ab).max(1e-300)).clamp(0.0, 1.0);
                    if e.distance(a + ab * x) <= reach {
                        let t = geom.nearest(e, ts[i] + x * (ts[i + 1] - ts[i]));
                        if geom.at(t).distance(e) <= tol {
                            also.push(t);
                        }
                    }
                }
            }
        }
        let maybe = |t: f64| {
            let i = ts.partition_point(|x| *x < t);
            [i.wrapping_sub(1), i]
                .iter()
                .any(|&w| marked.get(w).is_some_and(|m| *m))
        };
        // many candidates (a circle on another, each of its chords one): binned by their boxes
        // grown by the reach, so a point looks only at those whose box it is in, the only ones
        // its test can pass
        let local = (cand.len() > 32).then(|| {
            let (mut lo, mut hi) = (
                Point2::new(f64::INFINITY, f64::INFINITY),
                Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
            );
            let boxes: Vec<(Point2, Point2)> = cand
                .iter()
                .map(|&s| {
                    let (g, i) = segs[s as usize];
                    let op = &samples[g as usize].1;
                    let (a, b) = (op[i as usize], op[i as usize + 1]);
                    let (l, h) = (
                        Point2::new(a.x.min(b.x) - reach, a.y.min(b.y) - reach),
                        Point2::new(a.x.max(b.x) + reach, a.y.max(b.y) + reach),
                    );
                    lo = Point2::new(lo.x.min(l.x), lo.y.min(l.y));
                    hi = Point2::new(hi.x.max(h.x), hi.y.max(h.y));
                    (l, h)
                })
                .collect();
            let mut g = Grid::new(lo, hi, cand.len());
            for (n, (l, h)) in boxes.iter().enumerate() {
                g.insert(n as u32, *l, *h);
            }
            (g, lo, hi)
        });
        let covered = |t: f64| {
            if !maybe(t) {
                return false;
            }
            let q = geom.at(t);
            let Some(dir) = geom.tangent(t) else { return false };
            let test = |s: u32| {
                let (g, i) = segs[s as usize];
                if curve[g as usize] == curve[k] || rank[g as usize] > rank[k] {
                    return false;
                }
                let (ts, ps) = &samples[g as usize];
                let (a, b) = (ps[i as usize], ps[i as usize + 1]);
                let ab = b - a;
                let l2 = ab.dot(ab);
                if l2 <= 1e-300 {
                    return false;
                }
                let x = ((q - a).dot(ab) / l2).clamp(0.0, 1.0);
                if q.distance(a + ab * x) > reach {
                    return false;
                }
                let other = &geoms[g as usize];
                let (t0, t1) = (ts[i as usize], ts[i as usize + 1]);
                let at = other.nearest(q, t0 + x * (t1 - t0));
                other.at(at).distance(q) <= tol
                    && other.tangent(at).is_some_and(|d| d.cross(dir).abs() <= 0.05)
            };
            match &local {
                // a point off the grid is within reach of none (the grid holds every grown box)
                Some((g, lo, hi)) => {
                    !(q.x < lo.x || q.x > hi.x || q.y < lo.y || q.y > hi.y)
                        && g.at(q).iter().any(|&n| test(cand[n as usize]))
                }
                None => cand.iter().any(|&s| test(s)),
            }
        };
        geom.runs_where_also(chord, &also, |t| !covered(t))
            .into_iter()
            .filter(|g| g.length() > 10.0 * tol)
            .collect()
    })
}

/// The distance between segments `ab` and `cd`.
fn seg_distance(a: Point2, b: Point2, c: Point2, d: Point2) -> f64 {
    if seg_cross(a, b, c, d).is_some() {
        return 0.0;
    }
    let to = |q: Point2, a: Point2, b: Point2| {
        let ab = b - a;
        let x = ((q - a).dot(ab) / ab.dot(ab).max(1e-300)).clamp(0.0, 1.0);
        q.distance(a + ab * x)
    };
    to(a, c, d).min(to(b, c, d)).min(to(c, a, b)).min(to(d, a, b))
}

/// Where the line `a + t (b - a)` passes within `tol` of segment `cd`: the interval of t (the
/// segment's capsule is convex, so one interval, the hull of its two end discs' and its band's).
fn near_segment(a: Point2, b: Point2, c: Point2, d: Point2, tol: f64) -> Option<(f64, f64)> {
    let u = b - a;
    let uu = u.dot(u);
    if uu <= 1e-300 {
        return None;
    }
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    let mut take = |x: f64, y: f64| {
        if x <= y {
            lo = lo.min(x);
            hi = hi.max(y);
        }
    };
    for e in [c, d] {
        let w = a - e;
        let (p, q) = (u.dot(w) / uu, (w.dot(w) - tol * tol) / uu);
        let disc = p * p - q;
        if disc >= 0.0 {
            let r = disc.sqrt();
            take(-p - r, -p + r);
        }
    }
    let v = d - c;
    let vv = v.dot(v);
    if vv > 1e-300 {
        let vl = vv.sqrt();
        // f(t) = f0 + t f1 kept within [l, h]
        let within = |f0: f64, f1: f64, l: f64, h: f64| -> Option<(f64, f64)> {
            if f1.abs() <= 1e-300 {
                return (f0 >= l && f0 <= h).then_some((f64::NEG_INFINITY, f64::INFINITY));
            }
            let (x, y) = ((l - f0) / f1, (h - f0) / f1);
            Some((x.min(y), x.max(y)))
        };
        let w = a - c;
        let s = within(w.dot(v) / vv, u.dot(v) / vv, 0.0, 1.0);
        let n = within(v.cross(w) / vl, v.cross(u) / vl, -tol, tol);
        if let (Some(s), Some(n)) = (s, n) {
            take(s.0.max(n.0), s.1.min(n.1));
        }
    }
    (lo <= hi).then_some((lo, hi))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `near_segment` against the test `unique` makes of each point: the segment's nearest point
    /// within `tol`. Lines along it, across it, ending near it, short and long.
    #[test]
    fn a_lines_cover_by_another_is_its_closed_form() {
        let tol = 1e-3;
        let mut seed = 0x9e37_79b9_7f4a_7c15u64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut covered_some = 0;
        for _ in 0..4000 {
            let a = Point2::new(10.0 * rnd() - 5.0, 0.0);
            let b = Point2::new(10.0 * rnd() - 5.0, 2e-3 * (rnd() - 0.5));
            let c = Point2::new(10.0 * rnd() - 5.0, 3e-3 * (rnd() - 0.5));
            let d = Point2::new(10.0 * rnd() - 5.0, 3e-3 * (rnd() - 0.5));
            let line = Geom2::Line { a: c, b: d };
            let got = near_segment(a, b, c, d, tol);
            let inside = |t: f64| {
                let q = a.lerp(b, t);
                let at = line.nearest(q, 0.0);
                line.at(at).distance(q) <= tol
            };
            for k in 0..=400 {
                let t = k as f64 / 400.0;
                let want = inside(t);
                let have = got.is_some_and(|(x, y)| t >= x && t <= y);
                // at an end of the interval the two may round apart
                let edge = got.is_some_and(|(x, y)| (t - x).abs() < 1e-9 || (t - y).abs() < 1e-9);
                assert!(
                    want == have || edge,
                    "t {t}: {want} {have} {a:?} {b:?} {c:?} {d:?} {got:?}"
                );
                covered_some += want as usize;
            }
        }
        assert!(covered_some > 1000, "the cases cover something: {covered_some}");
    }
}
