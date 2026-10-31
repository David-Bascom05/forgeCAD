//! Re-deriving a body's edges and vertices after some of its faces were given new surfaces
//! (offset, tilted, newly inserted): each vertex is solved onto every surface around it, each
//! edge is the exact intersection of its two faces' surfaces between its new vertices.

use forge_geom::curve::{CurveDeriv, ProcCurve, ProceduralCurve};
use forge_geom::intcurve::{IntCurve, refine, trace};
use forge_geom::intersect::{SurfaceSurface, plane_surface};
use forge_geom::{Circle, Curve, CurveGeom, Domain1, Line, Plane, Surface};
use forge_math::{Box3, Frame, Interval, Point3, UnitVec3, Vec3, dmath, tol};
use forge_topo::{Body, EdgeId, FaceId, Name};

#[derive(Clone, Debug, PartialEq)]
pub enum RebuildError {
    /// No point lies on every surface around this vertex near where it was.
    Vertex { vertex: Name },
    /// The two faces at this edge no longer meet between its vertices.
    Edge { edge: Name },
    /// This edge shrank to nothing or turned around: a face would vanish or turn inside out.
    Collapsed { edge: Name },
}

/// Recomputes every vertex and edge touching a face flagged in `changed` (by face index), from
/// the faces' current surfaces. The old geometry is the starting guess.
pub fn rebuild(body: &mut Body, changed: &[bool]) -> Result<(), RebuildError> {
    let mut edges = vec![false; body.edges.len()];
    let mut verts = vec![false; body.vertices.len()];
    for e in body.edge_ids() {
        let edge = body.edge(e);
        if edge.coedges.iter().any(|&c| changed[body.coedge_face(c).index()]) {
            edges[e.index()] = true;
            verts[edge.start.index()] = true;
            verts[edge.end.index()] = true;
        }
    }
    // edges between unchanged faces still end at the moved vertices
    for e in body.edge_ids() {
        let edge = body.edge(e);
        edges[e.index()] |= verts[edge.start.index()] || verts[edge.end.index()];
    }
    rebuild_some(body, &verts, &edges)
}

/// Solves the flagged vertices onto the surfaces around them, then fits the flagged edges between
/// their vertices. Each one's current geometry is the starting guess.
pub fn rebuild_some(body: &mut Body, verts: &[bool], edges: &[bool]) -> Result<(), RebuildError> {
    let mut vedges: Vec<Vec<EdgeId>> = vec![Vec::new(); body.vertices.len()];
    for e in body.edge_ids() {
        let edge = body.edge(e);
        vedges[edge.start.index()].push(e);
        if edge.end != edge.start {
            vedges[edge.end.index()].push(e);
        }
    }
    let faces_of = |b: &Body, e: EdgeId| -> Vec<FaceId> {
        let mut fs: Vec<FaceId> = b.edge(e).coedges.iter().map(|&c| b.coedge_face(c)).collect();
        fs.sort();
        fs.dedup();
        fs
    };
    let old: Vec<(Curve, Interval)> = body.edges.iter().map(|e| (e.curve.clone(), e.range)).collect();
    let old_pts: Vec<Point3> = body.vertices.iter().map(|v| v.point).collect();

    // vertices
    for v in 0..body.vertices.len() {
        if !verts[v] {
            continue;
        }
        let mut faces: Vec<FaceId> = vedges[v].iter().flat_map(|&e| faces_of(body, e)).collect();
        faces.sort();
        faces.dedup();
        let mut cons: Vec<Surface> = Vec::new();
        for &f in &faces {
            let s = &body.face(f).surface;
            if !cons.iter().any(|c| same_surface(c, s)) {
                cons.push(s.clone());
            }
        }
        let p0 = old_pts[v];
        // two faces meeting tangentially (a loft's halves) pin the vertex only once: the plane
        // through their seam and normal holds it along the seam, as a meridian does a revolve's
        for &e in &vedges[v] {
            let fs = faces_of(body, e);
            let [fa, fb] = fs[..] else { continue };
            let (na, nb) = (
                body.face(fa).surface.gradient(p0),
                body.face(fb).surface.gradient(p0),
            );
            let (c, r) = &old[e.index()];
            let along = c.deriv(c.closest(p0, *r)).d1;
            if na.cross(nb).length() <= 1e-6 * na.length() * nb.length()
                && let Some(m) = na.cross(along).normalize()
            {
                cons.push(Surface::Plane(Plane::new(Frame::from_z(p0, m))));
            }
        }
        if cons.len() < 3 {
            for &e in &vedges[v] {
                let fs = faces_of(body, e);
                if fs.len() == 1 {
                    cons.extend(seam_constraints(&body.face(fs[0]).surface, p0, cons.len()));
                    break;
                }
            }
        }
        if cons.len() < 3
            && let Some(s) = cons.first().cloned()
        {
            cons.extend(seam_constraints(&s, p0, cons.len()));
        }
        let mut p = solve_point(&cons, p0).ok_or(RebuildError::Vertex {
            vertex: body.vertices[v].name,
        })?;
        // two faces meeting tangentially do not fix the vertex across their edge: it lies where
        // their old edge, moved onto one of them, meets the others
        for &e in &vedges[v] {
            let fs = faces_of(body, e);
            let [fa, fb] = fs[..] else { continue };
            let (a, b) = (&body.face(fa).surface, &body.face(fb).surface);
            if a.gradient(p).cross(b.gradient(p)).length() > 1e-6 {
                continue;
            }
            let rest: Vec<&Surface> = cons
                .iter()
                .filter(|s| !same_surface(s, a) && !same_surface(s, b))
                .collect();
            let (oc, or) = &old[e.index()];
            if let Some(q) = along_tangent(a, &rest, oc, *or, p0) {
                p = q;
            }
            break;
        }
        body.vertices[v].point = p;
    }

    // edges
    let ids: Vec<EdgeId> = body.edge_ids().collect();
    for e in ids {
        if !edges[e.index()] {
            continue;
        }
        let fs = faces_of(body, e);
        let (oc, or) = old[e.index()].clone();
        let edge = body.edge(e);
        let (vs, ve) = (edge.start, edge.end);
        let name = edge.name;
        let (ps, pe) = (body.vertex(vs).point, body.vertex(ve).point);
        let closed = vs == ve;
        let a = body.face(fs[0]).surface.clone();
        let b = match fs.get(1) {
            Some(&f) => body.face(f).surface.clone(),
            None => meridian(&a, oc.eval(or.mid())).ok_or(RebuildError::Edge { edge: name })?,
        };
        if !closed && ps.distance(pe) <= tol::LINEAR {
            return Err(RebuildError::Collapsed { edge: name });
        }
        let shift = ps
            .distance(old_pts[vs.index()])
            .max(pe.distance(old_pts[ve.index()]));
        let (curve, range) =
            edge_curve(&a, &b, &oc, or, ps, pe, closed, shift).ok_or(RebuildError::Edge { edge: name })?;
        // the edge must still run the way it did
        let (nd, od) = (curve.deriv(range.mid()).d1, oc.deriv(or.mid()).d1);
        if !closed && (nd.dot(od) <= 0.0 || ps.distance(pe) <= tol::LINEAR) {
            return Err(RebuildError::Collapsed { edge: name });
        }
        let edge = &mut body.edges[e.index()];
        edge.curve = curve;
        edge.range = range;
        edge.bbox.clear();
    }
    for f in 0..body.faces.len() {
        body.faces[f].bbox.clear();
        body.faces[f].query.clear();
    }
    Ok(())
}

/// Whether face `f`'s surface is a cone whose apex lies within the face's height (judged from
/// the face's current vertices, any of which may yet move `slack` along the axis): the face would
/// pass through the apex.
pub(crate) fn reaches_apex(body: &Body, f: FaceId, slack: f64) -> bool {
    let Surface::Cone(c) = &body.face(f).surface else {
        return false;
    };
    body.face(f).loops.iter().any(|&l| {
        body.lp(l).coedges.iter().any(|&co| {
            let p = body.vertex(body.coedge_start(co)).point;
            let v = c.frame.z().dot(p - c.frame.origin) - slack;
            c.radius_at(v) <= tol::MIN_LENGTH
        })
    })
}

fn same_surface(a: &Surface, b: &Surface) -> bool {
    a == b
}

/// The axis of a surface of revolution.
fn axis_of(s: &Surface) -> Option<(Point3, UnitVec3)> {
    match s {
        Surface::Cylinder(c) => Some((c.frame.origin, c.frame.z())),
        Surface::Cone(c) => Some((c.frame.origin, c.frame.z())),
        Surface::Sphere(c) => Some((c.frame.origin, c.frame.z())),
        Surface::Torus(c) => Some((c.frame.origin, c.frame.z())),
        _ => None,
    }
}

/// The plane through a revolution surface's axis and `p`.
fn meridian(s: &Surface, p: Point3) -> Option<Surface> {
    let (o, z) = axis_of(s)?;
    let radial = (p - o) - z * z.dot(p - o);
    let n = z.cross(radial).normalize()?;
    Some(Surface::Plane(Plane::new(Frame::from_zx(o, n, radial)?)))
}

/// Extra planes pinning a vertex that the surfaces around it leave free: the meridian of a
/// seam, or two planes through the axis for a pole.
fn seam_constraints(s: &Surface, p: Point3, have: usize) -> Vec<Surface> {
    let Some((o, z)) = axis_of(s) else {
        return Vec::new();
    };
    let radial = (p - o) - z * z.dot(p - o);
    if radial.length() > tol::LINEAR {
        return meridian(s, p).into_iter().take(3 - have.min(3)).collect();
    }
    let x = z.any_perpendicular();
    let y = z.cross(x).normalize().expect("perpendicular");
    [x, y]
        .into_iter()
        .map(|n| Surface::Plane(Plane::new(Frame::from_z(o, n))))
        .take(3 - have.min(3))
        .collect()
}

/// Gauss-Newton (lightly damped) onto every surface's zero set, from `p0`.
pub(crate) fn solve_point(cons: &[Surface], p0: Point3) -> Option<Point3> {
    let mut p = p0;
    let scale = 1.0 + p0.to_vec().abs_max();
    for _ in 0..200 {
        let mut a = [[0.0f64; 3]; 3];
        let mut r = [0.0f64; 3];
        for s in cons {
            let g = s.implicit(p);
            let n = s.gradient(p);
            let n = [n.x, n.y, n.z];
            for i in 0..3 {
                for j in 0..3 {
                    a[i][j] += n[i] * n[j];
                }
                r[i] -= n[i] * g;
            }
        }
        let tr = a[0][0] + a[1][1] + a[2][2];
        for (i, row) in a.iter_mut().enumerate() {
            row[i] += 1e-13 * tr;
        }
        let dp = solve3(a, r)?;
        p += Vec3::new(dp[0], dp[1], dp[2]);
        if dp.iter().fold(0.0f64, |m, x| m.max(x.abs())) <= 1e-14 * scale {
            break;
        }
    }
    cons.iter()
        .all(|s| s.implicit(p).abs() <= 1e-2 * tol::LINEAR)
        .then_some(p)
}

/// Where curve `c` (over `range`, from its end at `p0`) moved onto `a` meets every surface of
/// `rest`: the vertex two tangent faces leave unfixed. None when it does not.
fn along_tangent(a: &Surface, rest: &[&Surface], c: &Curve, range: Interval, p0: Point3) -> Option<Point3> {
    let [s] = rest[..] else { return None };
    let foot = |t: f64| {
        let mut q = c.eval(t);
        for _ in 0..4 {
            q = q - a.gradient(q) * a.implicit(q);
        }
        q
    };
    let mut t = if c.eval(range.lo).distance(p0) <= c.eval(range.hi).distance(p0) {
        range.lo
    } else {
        range.hi
    };
    // secant on the other surface's value along the moved curve
    let h = 1e-3 * range.width();
    let (mut t1, mut f0) = (t + h, s.implicit(foot(t)));
    for _ in 0..40 {
        let f1 = s.implicit(foot(t1));
        if f1.abs() <= 1e-3 * tol::LINEAR || f1 == f0 {
            t = t1;
            break;
        }
        (t, t1, f0) = (t1, t1 - f1 * (t1 - t) / (f1 - f0), f1);
    }
    let q = foot(t);
    (s.implicit(q).abs() <= 1e-2 * tol::LINEAR && a.implicit(q).abs() <= 1e-2 * tol::LINEAR).then_some(q)
}

fn solve3(mut a: [[f64; 3]; 3], mut b: [f64; 3]) -> Option<[f64; 3]> {
    for k in 0..3 {
        let piv = (k..3).max_by(|&i, &j| a[i][k].abs().total_cmp(&a[j][k].abs()))?;
        a.swap(k, piv);
        b.swap(k, piv);
        if a[k][k] == 0.0 {
            return None;
        }
        for i in k + 1..3 {
            let f = a[i][k] / a[k][k];
            let pivot = a[k];
            for (x, p) in a[i].iter_mut().zip(pivot).skip(k) {
                *x -= f * p;
            }
            b[i] -= f * b[k];
        }
    }
    let mut x = [0.0; 3];
    for k in (0..3).rev() {
        let s: f64 = (k + 1..3).map(|j| a[k][j] * x[j]).sum();
        x[k] = (b[k] - s) / a[k][k];
    }
    Some(x)
}

/// The curve running the other way (t becomes -t, or hi - t for a traced chain).
pub(crate) fn reversed(c: &Curve) -> Curve {
    match c {
        Curve::Line(l) => Curve::Line(Line::new(l.origin, -l.dir)),
        Curve::Circle(k) => Curve::Circle(Circle {
            frame: k.frame.flipped(),
            ..*k
        }),
        Curve::Ellipse(k) => Curve::Ellipse(forge_geom::Ellipse {
            frame: k.frame.flipped(),
            ..*k
        }),
        Curve::Parabola(k) => Curve::Parabola(forge_geom::Parabola {
            frame: k.frame.flipped(),
            ..*k
        }),
        Curve::Hyperbola(k) => Curve::Hyperbola(forge_geom::Hyperbola {
            frame: k.frame.flipped(),
            ..*k
        }),
        Curve::Intersection(k) => {
            let mut pts = k.pts.clone();
            let mut tans: Vec<Vec3> = k.tans.iter().map(|&t| -t).collect();
            pts.reverse();
            tans.reverse();
            Curve::Intersection(std::sync::Arc::new(IntCurve::new(
                k.a.clone(),
                k.b.clone(),
                pts,
                tans,
                k.closed,
                k.sag,
            )))
        }
        Curve::Procedural(k) => Curve::Procedural(ProcCurve(std::sync::Arc::new(Backwards(k.clone())))),
    }
}

/// A procedural curve run the other way: t becomes -t.
#[derive(Debug)]
struct Backwards(ProcCurve);

fn flip(r: Interval) -> Interval {
    Interval::new(-r.hi, -r.lo)
}

impl ProceduralCurve for Backwards {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let d = self.0.deriv(-t);
        CurveDeriv {
            p: d.p,
            d1: -d.d1,
            d2: d.d2,
        }
    }
    fn domain(&self) -> Domain1 {
        let d = self.0.domain();
        Domain1 {
            lo: -d.hi,
            hi: -d.lo,
            period: d.period,
        }
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        -self.0.closest(p, flip(range))
    }
    fn bbox(&self, range: Interval) -> Box3 {
        self.0.bbox(flip(range))
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        ProcCurve(std::sync::Arc::new(Backwards(self.0.transformed(f))))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        self.0.speed_bound(flip(range))
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        self.0.segments(flip(range), chord)
    }
    fn kinks(&self, range: Interval) -> Vec<f64> {
        self.0.kinks(flip(range)).into_iter().rev().map(|t| -t).collect()
    }
}

/// Whether the curve over `range` lies on both surfaces.
fn on_both(c: &Curve, range: Interval, a: &Surface, b: &Surface) -> bool {
    (0..=8).all(|i| {
        let p = c.eval(range.lo + range.width() * (i as f64 / 8.0));
        a.implicit(p).abs() <= 0.1 * tol::LINEAR && b.implicit(p).abs() <= 0.1 * tol::LINEAR
    })
}

/// The search range for `closest` on a curve: one period, the traced chain, or far enough.
fn search_range(c: &Curve, ps: Point3, pe: Point3) -> Interval {
    let d = c.domain();
    match (d.period, c) {
        (Some(p), _) => Interval::new(d.lo, d.lo + p),
        (None, Curve::Intersection(_)) => d.interval(),
        _ => {
            let reach = 4.0 * (ps.to_vec().abs_max() + pe.to_vec().abs_max()) + 1e3;
            Interval::new(d.lo.max(-reach), d.hi.min(reach))
        }
    }
}

/// Fits curve `c` to run from `ps` to `pe` (once round from `ps` when closed): its range, or None
/// when the points are not on it or it runs the other way.
fn fit(c: &Curve, ps: Point3, pe: Point3, closed: bool) -> Option<Interval> {
    let search = search_range(c, ps, pe);
    let t0 = c.closest(ps, search);
    if c.eval(t0).distance(ps) > 0.1 * tol::LINEAR {
        return None;
    }
    let period = c.domain().period;
    if closed {
        return period.map(|p| Interval::new(t0, t0 + p));
    }
    let t1 = c.closest(pe, search);
    if c.eval(t1).distance(pe) > 0.1 * tol::LINEAR {
        return None;
    }
    let t1 = match period {
        Some(p) => {
            let w = dmath::wrap_tau((t1 - t0) * dmath::TAU / p) * p / dmath::TAU;
            t0 + w
        }
        None => t1,
    };
    (t1 > t0).then_some(Interval::new(t0, t1))
}

/// The new curve of an edge between surfaces `a` and `b`, running from `ps` to `pe`, picked
/// from the intersection's branches as the one nearest the old edge `oc` over `or`.
#[allow(clippy::too_many_arguments)]
fn edge_curve(
    a: &Surface,
    b: &Surface,
    oc: &Curve,
    or: Interval,
    ps: Point3,
    pe: Point3,
    closed: bool,
    shift: f64,
) -> Option<(Curve, Interval)> {
    // the old curve, if it still lies on both (an edge only trimmed)
    if let Some(r) = fit(oc, ps, pe, closed)
        && on_both(oc, r, a, b)
    {
        return Some((oc.clone(), r));
    }
    // cheap and exact when they hold (and the only sound answer where the faces are tangent)
    if !closed
        && matches!(oc, Curve::Line(_))
        && let Some(l) = Line::through(ps, pe)
    {
        let c = Curve::Line(l);
        let r = Interval::new(0.0, ps.distance(pe));
        if on_both(&c, r, a, b) {
            return Some((c, r));
        }
    }
    if let Curve::Circle(k) = oc {
        let z = k.frame.z();
        let centre = k.frame.origin + z * z.dot(ps - k.frame.origin);
        let radial = ps - centre;
        if radial.length() > tol::MIN_LENGTH
            && let Some(frame) = Frame::from_zx(centre, z, radial)
        {
            let c = Curve::Circle(Circle {
                frame,
                radius: radial.length(),
            });
            if let Some(r) = fit(&c, ps, pe, closed)
                && on_both(&c, r, a, b)
            {
                return Some((c, r));
            }
        }
    }
    // a procedural edge that runs straight (a linear blend's spring between planes) stays straight:
    // past a tangency, where tracing finds nothing
    if !closed
        && matches!(oc, Curve::Procedural(_))
        && let (Some(l), Some(o)) = (
            Line::through(ps, pe),
            Line::through(oc.eval(or.lo), oc.eval(or.hi)),
        )
        && (0..=8).all(|i| {
            let q = oc.eval(or.lo + or.width() * (i as f64 / 8.0));
            o.eval(o.param_of(q)).distance(q) <= 0.1 * tol::LINEAR
        })
    {
        let c = Curve::Line(l);
        let r = Interval::new(0.0, ps.distance(pe));
        if on_both(&c, r, a, b) {
            return Some((c, r));
        }
    }
    let old_mid = oc.eval(or.mid());
    let guess = refine(a, b, old_mid).unwrap_or(old_mid);
    let analytic = match (a, b) {
        (Surface::Plane(p), s) | (s, Surface::Plane(p)) => plane_surface(p, s),
        _ => SurfaceSurface::NotAnalytic,
    };
    let curves: Vec<Curve> = match analytic {
        SurfaceSurface::Curves(cs) => cs.into_iter().map(|c| c.0).collect(),
        SurfaceSurface::NotAnalytic => {
            let region = oc
                .bbox(or)
                .hull(&Box3::from_points([ps, pe, guess]))
                .grow(2.0 * shift + 10.0 * tol::LINEAR);
            trace(a, b, region)?
                .into_iter()
                .map(|c| Curve::Intersection(std::sync::Arc::new(c)))
                .collect()
        }
        _ => return None,
    };
    let mut best: Option<(f64, Curve, Interval)> = None;
    for c in curves {
        for c in [reversed(&c), c] {
            let Some(r) = fit(&c, ps, pe, closed) else {
                continue;
            };
            // a closed edge keeps its turn: a circle its axis, else its tangent where it was
            let turn = match (&c, oc) {
                (Curve::Circle(k), Curve::Circle(o)) => k.frame.z().dot(o.frame.z()),
                _ => c.deriv(r.lo).d1.dot(oc.deriv(oc.closest(ps, or)).d1),
            };
            if closed && turn <= 0.0 {
                continue;
            }
            let score = c.eval(r.mid()).distance(guess);
            if best.as_ref().is_none_or(|b| score < b.0) {
                best = Some((score, c, r));
            }
        }
    }
    best.map(|b| (b.1, b.2))
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::{Cylinder, Sphere};

    #[test]
    fn tangent_pair_and_a_cone() {
        let o = Point3::new(5.7467457927089125, 5.7467457927089125, 0.0);
        let side = Surface::Plane(Plane::new(
            Frame::from_zx(Point3::new(5.75, 0.351, 0.0), -UnitVec3::Y, Vec3::X).unwrap(),
        ));
        let cyl = Surface::Cylinder(Cylinder {
            frame: Frame::WORLD.with_origin(o),
            radius: o.y - 0.351,
        });
        let cone = Surface::Cone(
            forge_geom::Cone::new(
                Frame::WORLD.with_origin(o),
                4.58318276687471,
                0.5 * dmath::FRAC_PI_2,
            )
            .unwrap(),
        );
        let p0 = Point3::new(5.7467457927089125, 0.0, 0.6671650696334661);
        let p = solve_point(&[side, cyl, cone], p0);
        assert!(p.is_some());
    }

    #[test]
    fn three_planes_and_a_tangent_pair() {
        let plane = |o: Point3, n: Vec3| Surface::Plane(Plane::new(Frame::from_z(o, n.normalize().unwrap())));
        let cons = [
            plane(Point3::new(1.0, 0.0, 0.0), Vec3::X),
            plane(Point3::new(0.0, 2.0, 0.0), Vec3::Y),
            plane(Point3::new(0.0, 0.0, 3.0), Vec3::new(0.0, 1.0, 1.0)),
        ];
        let p = solve_point(&cons, Point3::ORIGIN).unwrap();
        assert!(p.distance(Point3::new(1.0, 2.0, 1.0)) < 1e-12, "{p:?}");
        // a plane tangent to a cylinder along a line, cut by a third plane
        let cyl = Surface::Cylinder(Cylinder {
            frame: Frame::WORLD,
            radius: 5.0,
        });
        let cons = [
            cyl,
            plane(Point3::new(5.0, 0.0, 0.0), Vec3::X),
            plane(Point3::new(0.0, 0.0, 2.0), Vec3::Z),
        ];
        let p = solve_point(&cons, Point3::new(5.3, 1.0, 1.0)).unwrap();
        assert!(p.distance(Point3::new(5.0, 0.0, 2.0)) < 1e-6, "{p:?}");
        let pole = [Surface::Sphere(Sphere {
            frame: Frame::WORLD,
            radius: 2.0,
        })];
        let mut cons = pole.to_vec();
        cons.extend(seam_constraints(&pole[0], Point3::new(0.0, 0.0, 3.0), 1));
        let p = solve_point(&cons, Point3::new(0.0, 0.0, 3.0)).unwrap();
        assert!(p.distance(Point3::new(0.0, 0.0, 2.0)) < 1e-12);
    }
}
