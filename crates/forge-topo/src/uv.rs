use crate::body::{Body, CoedgeId, Edge, FaceId, LoopId};
use forge_geom::{Curve, CurveGeom, SurfaceGeom};
use forge_math::{Point2, dmath};

/// A face loop drawn in its surface's parameter plane, with periodic
/// directions unwrapped so the polygon is continuous.
#[derive(Clone, Debug)]
pub struct UvLoop {
    pub points: Vec<Point2>,
    /// For each coedge of the loop, the index of its first point in `points`.
    pub coedge_first: Vec<usize>,
    pub coedges: Vec<CoedgeId>,
}

/// An even number of segments, so each edge's parameter midpoint is a sample.
pub fn default_segments(e: &Edge) -> usize {
    let n = match e.curve {
        Curve::Line(_) => 2,
        Curve::Circle(_) | Curve::Ellipse(_) => (e.range.width() / (dmath::PI / 16.0)).ceil() as usize,
        Curve::Parabola(_) | Curve::Hyperbola(_) => 16,
    };
    (n.max(2) + 1) & !1
}

/// Shifts `uv` by whole periods so it is as close as possible to `reference`.
pub fn unwrap_near(body: &Body, face: FaceId, uv: Point2, reference: Point2) -> Point2 {
    let d = body.face(face).surface.domain();
    let fix = |x: f64, r: f64, period: Option<f64>| match period {
        Some(p) => x + p * ((r - x) / p).round(),
        None => x,
    };
    Point2::new(
        fix(uv.x, reference.x, d.u.period),
        fix(uv.y, reference.y, d.v.period),
    )
}

pub fn uv_loop(body: &Body, lp: LoopId, segments: impl Fn(&Edge) -> usize) -> UvLoop {
    let l = body.lp(lp);
    let surface = &body.face(l.face).surface;
    let mut points: Vec<Point2> = Vec::new();
    let mut coedge_first = Vec::with_capacity(l.coedges.len());
    for &c in &l.coedges {
        coedge_first.push(points.len());
        let co = body.coedge(c);
        let e = body.edge(co.edge);
        let n = segments(e);
        for i in 0..n {
            let k = if co.forward { i } else { n - i };
            let t = (e.range.lo + e.range.width() * (k as f64 / n as f64)).min(e.range.hi);
            let uv = surface.project(e.curve.eval(t));
            let uv = match points.last() {
                Some(&prev) => unwrap_near(body, l.face, uv, prev),
                None => uv,
            };
            points.push(uv);
        }
    }
    UvLoop {
        points,
        coedge_first,
        coedges: l.coedges.clone(),
    }
}

/// Signed area of a closed polygon (positive when counter-clockwise).
pub fn signed_area(poly: &[Point2]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| poly[i].to_vec().cross(poly[(i + 1) % n].to_vec()))
        .sum::<f64>()
        * 0.5
}

/// Even-odd test against all of a face's loops.
pub fn point_in_loops(p: Point2, loops: &[UvLoop]) -> bool {
    let mut inside = false;
    for l in loops {
        let pts = &l.points;
        let n = pts.len();
        for i in 0..n {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            if (a.y > p.y) != (b.y > p.y) {
                let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
                if x > p.x {
                    inside = !inside;
                }
            }
        }
    }
    inside
}

/// All loops of a face, later loops shifted by whole periods into the first
/// loop's parameter window.
pub fn face_uv_loops(body: &Body, face: FaceId) -> Vec<UvLoop> {
    face_uv_loops_with(body, face, default_segments)
}

pub fn face_uv_loops_with(body: &Body, face: FaceId, segments: impl Fn(&Edge) -> usize) -> Vec<UvLoop> {
    let mut loops: Vec<UvLoop> = body
        .face(face)
        .loops
        .iter()
        .map(|&l| uv_loop(body, l, &segments))
        .collect();
    if let Some((first, rest)) = loops.split_first_mut() {
        let centre = |pts: &[Point2]| {
            let n = pts.len().max(1) as f64;
            let s = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.x, a.1 + p.y));
            Point2::new(s.0 / n, s.1 / n)
        };
        let reference = centre(&first.points);
        for l in rest {
            let c = centre(&l.points);
            let moved = unwrap_near(body, face, c, reference);
            let (du, dv) = (moved.x - c.x, moved.y - c.y);
            for p in &mut l.points {
                *p = Point2::new(p.x + du, p.y + dv);
            }
        }
    }
    loops
}
