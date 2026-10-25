use crate::body::{Body, CoedgeId, Edge, FaceId, LoopId};
use forge_geom::{Curve, CurveGeom, SurfaceGeom};
use forge_math::{Point2, Point3, dmath};

/// A face loop drawn in its surface's parameter plane, with periodic
/// directions unwrapped so the polygon is continuous.
#[derive(Clone, Debug)]
pub struct UvLoop {
    pub points: Vec<Point2>,
    /// For each point, which loop sample it came from. A sample at a singular
    /// point (sphere pole, cone apex) appears twice: once at each neighbour's u.
    pub sample_of: Vec<usize>,
    /// For each coedge of the loop, the index of its first sample.
    pub coedge_first: Vec<usize>,
    pub coedges: Vec<CoedgeId>,
}

impl UvLoop {
    /// The first point drawn for a sample.
    pub fn point_of_sample(&self, sample: usize) -> Point2 {
        let i = self
            .sample_of
            .iter()
            .position(|&s| s == sample)
            .expect("every sample is drawn");
        self.points[i]
    }
}

/// An even number of segments, so each edge's parameter midpoint is a sample.
pub fn default_segments(e: &Edge) -> usize {
    let n = match e.curve {
        Curve::Line(_) => 2,
        Curve::Circle(_) | Curve::Ellipse(_) => (e.range.width() / (dmath::PI / 16.0)).ceil() as usize,
        Curve::Parabola(_) | Curve::Hyperbola(_) => 16,
        // follow the chain: a sample per chain segment or two
        Curve::Intersection(_) => (e.range.width().ceil() as usize).clamp(4, 256),
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

/// Points of a loop in traversal order, each coedge contributing its samples
/// minus its last point (the next coedge starts there).
pub fn loop_samples(body: &Body, lp: LoopId, segments: impl Fn(&Edge) -> usize) -> (Vec<Point3>, Vec<usize>) {
    let mut pts = Vec::new();
    let mut first = Vec::new();
    for &c in &body.lp(lp).coedges {
        first.push(pts.len());
        let co = body.coedge(c);
        let e = body.edge(co.edge);
        let n = segments(e);
        for i in 0..n {
            let k = if co.forward { i } else { n - i };
            let t = (e.range.lo + e.range.width() * (k as f64 / n as f64)).min(e.range.hi);
            pts.push(e.curve.eval(t));
        }
    }
    (pts, first)
}

/// Draws a loop's samples in the face's parameter plane.
///
/// Periodic parameters are unwrapped for continuity. At a singular sample the
/// surface's u is undefined, so it is drawn twice, at the u of the samples on
/// either side. Where the loop passes through singular points the unwrapping
/// alone cannot tell which way round the face goes (a half sphere bounded by
/// two meridians), so the choice is the one whose orientation matches the
/// face: counter-clockwise in (u, v) when the face normal is the surface's.
pub fn draw_loop(body: &Body, face: FaceId, samples: &[Point3]) -> (Vec<Point2>, Vec<usize>) {
    let surface = &body.face(face).surface;
    let n = samples.len();
    let raw: Vec<Point2> = samples.iter().map(|&p| surface.project(p)).collect();
    let singular: Vec<bool> = raw.iter().map(|&uv| surface.normal(uv).is_none()).collect();
    // begin just after a singular sample when there is one, so each run below is one
    // whole stretch between singular points (a stretch split at the start would be
    // unwrapped as two runs and could be shifted apart)
    let Some(start) = (0..n)
        .find(|&i| !singular[i] && singular[(i + n - 1) % n])
        .or_else(|| (0..n).find(|&i| !singular[i]))
    else {
        return (raw, (0..n).collect());
    };
    let next_regular = |i: usize| (1..=n).map(|k| (i + k) % n).find(|&j| !singular[j]).unwrap_or(i);

    // runs of continuously unwrapped points; a new run starts after each singular sample
    let mut runs: Vec<Vec<(Point2, usize)>> = vec![Vec::new()];
    // after the last singular sample the loop comes back to its start: same run as the first
    let mut tail: Vec<(Point2, usize)> = Vec::new();
    let mut prev: Option<Point2> = None;
    for k in 0..n {
        let i = (start + k) % n;
        if singular[i] {
            let before = prev.expect("loop starts at a regular sample");
            runs.last_mut()
                .expect("at least one run")
                .push((Point2::new(before.x, raw[i].y), i));
            if next_regular(i) == start {
                tail.push((Point2::new(runs[0][0].0.x, raw[i].y), i));
                continue;
            }
            let after = raw[next_regular(i)];
            let p = Point2::new(after.x, raw[i].y);
            runs.push(vec![(p, i)]);
            prev = Some(p);
            continue;
        }
        let p = match prev {
            Some(r) => unwrap_near(body, face, raw[i], r),
            None => raw[i],
        };
        runs.last_mut().expect("at least one run").push((p, i));
        prev = Some(p);
    }
    let period = surface.domain().u.period;
    let mut best: Vec<(Point2, usize)> = runs.concat();
    best.extend(tail.iter().copied());
    if runs.len() > 1
        && let Some(period) = period
    {
        let want_ccw = body.face(face).same_sense;
        let count = 3usize.pow(runs.len() as u32 - 1);
        let mut best_score = f64::INFINITY;
        for combo in 0..count {
            let mut c = combo;
            let mut pts: Vec<(Point2, usize)> = runs[0].clone();
            for run in &runs[1..] {
                let shift = (c % 3) as f64 - 1.0;
                c /= 3;
                pts.extend(
                    run.iter()
                        .map(|&(p, i)| (Point2::new(p.x + shift * period, p.y), i)),
                );
            }
            pts.extend(tail.iter().copied());
            let poly: Vec<Point2> = pts.iter().map(|q| q.0).collect();
            let area = signed_area(&poly);
            if (area > 0.0) == want_ccw && area.abs() < best_score {
                best_score = area.abs();
                best = pts;
            }
        }
    }
    best.into_iter().unzip()
}

pub fn uv_loop(body: &Body, lp: LoopId, segments: impl Fn(&Edge) -> usize) -> UvLoop {
    let face = body.lp(lp).face;
    let (samples, coedge_first) = loop_samples(body, lp, segments);
    let (points, sample_of) = draw_loop(body, face, &samples);
    UvLoop {
        points,
        sample_of,
        coedge_first,
        coedges: body.lp(lp).coedges.clone(),
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
    align_loops(body, face, loops.iter_mut().map(|l| &mut l.points));
    loops
}

/// Shifts every loop after the first by whole periods so its centre falls in
/// the first loop's window.
pub fn align_loops<'a>(body: &Body, face: FaceId, mut loops: impl Iterator<Item = &'a mut Vec<Point2>>) {
    let centre = |pts: &[Point2]| {
        let n = pts.len().max(1) as f64;
        let s = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.x, a.1 + p.y));
        Point2::new(s.0 / n, s.1 / n)
    };
    let Some(first) = loops.next() else { return };
    let reference = centre(first);
    for l in loops {
        let c = centre(l);
        let moved = unwrap_near(body, face, c, reference);
        let (du, dv) = (moved.x - c.x, moved.y - c.y);
        for p in l.iter_mut() {
            *p = Point2::new(p.x + du, p.y + dv);
        }
    }
}
