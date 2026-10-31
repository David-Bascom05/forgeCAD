//! Whether an operation left faces crossing each other: an offset or tilt that ran one face
//! through another, or a trimmed face whose loops cross.

use forge_geom::intcurve::trace;
use forge_geom::intersect::{SurfaceSurface, plane_surface};
use forge_geom::{Curve, CurveGeom, Surface, SurfaceGeom};
use forge_math::predicates::orient2d;
use forge_math::{Box3, Interval, Point2, Point3, tol};
use forge_topo::uv::{UvLoop, face_uv_loops, point_in_loops, unwrap_near};
use forge_topo::{Body, FaceId, Name, VertexId};

/// The first two faces found crossing, at least one of them flagged (by face index): faces that
/// share no vertex yet meet, or a flagged face whose own loops cross.
pub(crate) fn crossing(body: &Body, flagged: &[bool]) -> Option<(Name, Name)> {
    let n = body.faces.len();
    let mut verts: Vec<Vec<VertexId>> = vec![Vec::new(); n];
    for f in body.face_ids() {
        for &l in &body.face(f).loops {
            for &c in &body.lp(l).coedges {
                verts[f.index()].push(body.coedge_start(c));
            }
        }
        verts[f.index()].sort();
        verts[f.index()].dedup();
    }
    let loops: Vec<Vec<UvLoop>> = (0..n as u32)
        .map(FaceId)
        .map(|f| {
            if body.face(f).alive {
                face_uv_loops(body, f)
            } else {
                Vec::new()
            }
        })
        .collect();
    for f in body.face_ids() {
        if flagged[f.index()] && loops_cross(&loops[f.index()]) {
            let name = body.face(f).name;
            return Some((name, name));
        }
    }
    // a surface defined by construction is held far tighter by the boxes of an 8 x 8 grid of its
    // parameters than by one box: faces whose pieces' boxes all miss each other can't meet
    let pieces: Vec<Vec<Box3>> = (0..n as u32)
        .map(FaceId)
        .map(|f| {
            let face = body.face(f);
            match &face.surface {
                Surface::Procedural(pr) if face.alive => {
                    let d = pr.domain();
                    let (u, v) = (d.u.interval(), d.v.interval());
                    if !(u.width().is_finite() && v.width().is_finite()) {
                        return vec![body.face_box(f)];
                    }
                    let at = |x: Interval, k: usize| {
                        Interval::new(
                            x.lo + x.width() * k as f64 / 8.0,
                            x.lo + x.width() * (k + 1) as f64 / 8.0,
                        )
                    };
                    (0..64)
                        .map(|k| {
                            pr.bbox(forge_math::Box2::new(at(u, k / 8), at(v, k % 8)))
                                .grow(tol::LINEAR)
                        })
                        .collect()
                }
                _ => vec![body.face_box(f)],
            }
        })
        .collect();
    let apart = |f: FaceId, g: FaceId| {
        !pieces[f.index()]
            .iter()
            .any(|a| pieces[g.index()].iter().any(|b| a.overlaps(b)))
    };
    let ids: Vec<FaceId> = body.face_ids().collect();
    for (k, &f) in ids.iter().enumerate() {
        for &g in &ids[k + 1..] {
            if !(flagged[f.index()] || flagged[g.index()]) {
                continue;
            }
            let (bf, bg) = (body.face_box(f), body.face_box(g));
            if !bf.overlaps(&bg) {
                continue;
            }
            let (vf, vg) = (&verts[f.index()], &verts[g.index()]);
            if vf.iter().any(|v| vg.binary_search(v).is_ok()) || apart(f, g) {
                continue;
            }
            let region = Box3 {
                x: meet(bf.x, bg.x),
                y: meet(bf.y, bg.y),
                z: meet(bf.z, bg.z),
            };
            let spacing = (bf.diagonal().min(bg.diagonal()) / 32.0).max(10.0 * tol::LINEAR);
            let inside = |p: Point3| {
                region.contains(p)
                    && in_face(body, f, &loops[f.index()], p)
                    && in_face(body, g, &loops[g.index()], p)
            };
            if surfaces_meet(
                &body.face(f).surface,
                &body.face(g).surface,
                region,
                spacing,
                inside,
            ) {
                return Some((body.face(f).name, body.face(g).name));
            }
        }
    }
    None
}

fn meet(a: Interval, b: Interval) -> Interval {
    Interval::new(a.lo.max(b.lo), a.hi.min(b.hi))
}

fn in_face(body: &Body, f: FaceId, loops: &[UvLoop], p: Point3) -> bool {
    let Some(first) = loops.first().filter(|l| !l.points.is_empty()) else {
        return false;
    };
    let (mut lo, mut hi) = (first.points[0], first.points[0]);
    for q in &first.points {
        lo = Point2::new(lo.x.min(q.x), lo.y.min(q.y));
        hi = Point2::new(hi.x.max(q.x), hi.y.max(q.y));
    }
    let uv = body.face(f).surface.project(p);
    point_in_loops(unwrap_near(body, f, uv, lo.lerp(hi, 0.5)), loops)
}

/// Whether any point of the surfaces' intersection inside `region` passes `inside`, sampled
/// `spacing` apart along each branch.
fn surfaces_meet(
    a: &Surface,
    b: &Surface,
    region: Box3,
    spacing: f64,
    inside: impl Fn(Point3) -> bool,
) -> bool {
    let ss = match (a, b) {
        (Surface::Plane(p), s) | (s, Surface::Plane(p)) => plane_surface(p, s),
        _ => SurfaceSurface::NotAnalytic,
    };
    let curves: Vec<Curve> = match ss {
        SurfaceSurface::Curves(cs) => cs.into_iter().map(|c| c.0).collect(),
        SurfaceSurface::Points(ps) => return ps.into_iter().any(inside),
        SurfaceSurface::NotAnalytic => match trace(a, b, region.grow(tol::LINEAR)) {
            Some(cs) => cs
                .into_iter()
                .map(|c| Curve::Intersection(std::sync::Arc::new(c)))
                .collect(),
            None => return false,
        },
        SurfaceSurface::Empty | SurfaceSurface::Coincident => return false,
    };
    curves.iter().any(|c| {
        let Some(range) = span(c, &region) else {
            return false;
        };
        let len =
            c.eval(range.lo).distance(c.eval(range.mid())) + c.eval(range.mid()).distance(c.eval(range.hi));
        let steps = ((len / spacing).ceil() as usize).clamp(8, 2048);
        (0..=steps).any(|i| inside(c.eval(range.lo + range.width() * (i as f64 / steps as f64))))
    })
}

/// The parameters over which a curve can be inside `region`.
fn span(c: &Curve, region: &Box3) -> Option<Interval> {
    let d = c.domain();
    if let Some(p) = d.period {
        return Some(Interval::new(d.lo, d.lo + p));
    }
    if let Curve::Line(l) = c {
        let (mut lo, mut hi) = (f64::NEG_INFINITY, f64::INFINITY);
        let o = l.origin;
        let v = l.dir.get();
        for (a, s, w) in [(o.x, region.x, v.x), (o.y, region.y, v.y), (o.z, region.z, v.z)] {
            if w.abs() < 1e-300 {
                if a < s.lo || a > s.hi {
                    return None;
                }
                continue;
            }
            let (t0, t1) = ((s.lo - a) / w, (s.hi - a) / w);
            lo = lo.max(t0.min(t1));
            hi = hi.min(t0.max(t1));
        }
        return (hi >= lo).then_some(Interval::new(lo, hi));
    }
    if d.lo.is_finite() && d.hi.is_finite() {
        return Some(d.interval());
    }
    // a conic branch: as far as the region's corners reach along it
    let reach = Interval::new(-1e6, 1e6);
    let mut r = Interval::point(c.closest(region.center(), reach));
    for x in [region.x.lo, region.x.hi] {
        for y in [region.y.lo, region.y.hi] {
            for z in [region.z.lo, region.z.hi] {
                r.include(c.closest(Point3::new(x, y, z), reach));
            }
        }
    }
    Some(r)
}

/// Whether face `f`'s loops cross each other.
pub(crate) fn face_loops_cross(body: &Body, f: FaceId) -> bool {
    loops_cross(&face_uv_loops(body, f))
}

/// Whether two segments of a face's loops cross properly (not at a shared end).
fn loops_cross(loops: &[UvLoop]) -> bool {
    let segs: Vec<(Point2, Point2)> = loops
        .iter()
        .flat_map(|l| {
            let n = l.points.len();
            (0..n).map(move |i| (l.points[i], l.points[(i + 1) % n]))
        })
        .filter(|s| s.0 != s.1)
        .collect();
    // swept along u: only segments whose u spans overlap are compared
    let mut segs = segs;
    segs.sort_by(|p, q| p.0.x.min(p.1.x).total_cmp(&q.0.x.min(q.1.x)));
    let opposite = |p: f64, q: f64| (p < 0.0 && q > 0.0) || (p > 0.0 && q < 0.0);
    for i in 0..segs.len() {
        let reach = segs[i].0.x.max(segs[i].1.x);
        for j in i + 1..segs.len() {
            if segs[j].0.x.min(segs[j].1.x) > reach {
                break;
            }
            let ((a, b), (c, d)) = (segs[i], segs[j]);
            if a == c || a == d || b == c || b == d {
                continue;
            }
            let apart = |p: f64, q: f64, r: f64, t: f64| p.max(q) < r.min(t) || r.max(t) < p.min(q);
            if apart(a.x, b.x, c.x, d.x) || apart(a.y, b.y, c.y, d.y) {
                continue;
            }
            if opposite(orient2d(a, b, c), orient2d(a, b, d))
                && opposite(orient2d(c, d, a), orient2d(c, d, b))
            {
                return true;
            }
        }
    }
    false
}
