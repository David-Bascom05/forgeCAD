use crate::body::{Body, CoedgeId, Edge, FaceId, LoopId};
use forge_geom::{Curve, CurveGeom, Domain2, SurfaceGeom};
use forge_math::{Box3, Interval, Point2, Point3, dmath};

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

/// How near the default drawing of a procedural edge stays to the curve.
pub const PROC_SAG: f64 = 1e-4;

/// An even number of segments, so each edge's parameter midpoint is a sample.
pub fn default_segments(e: &Edge) -> usize {
    let n = match e.curve {
        Curve::Line(_) => 2,
        Curve::Circle(_) | Curve::Ellipse(_) => (e.range.width() / (dmath::PI / 16.0)).ceil() as usize,
        Curve::Parabola(_) | Curve::Hyperbola(_) => 16,
        // follow the chain: a sample per chain segment or two
        Curve::Intersection(_) => (e.range.width().ceil() as usize).clamp(4, 256),
        Curve::Procedural(ref c) => c.segments(e.range, PROC_SAG).clamp(4, 256),
    };
    (n.max(2) + 1) & !1
}

/// Shifts `uv` by whole periods so it is as close as possible to `reference`.
pub fn unwrap_near(body: &Body, face: FaceId, uv: Point2, reference: Point2) -> Point2 {
    unwrap_in(&body.face(face).surface.domain(), uv, reference)
}

fn unwrap_in(d: &Domain2, uv: Point2, reference: Point2) -> Point2 {
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
    let coedges = &body.lp(lp).coedges;
    let ns: Vec<usize> = coedges
        .iter()
        .map(|&c| segments(body.edge(body.coedge(c).edge)))
        .collect();
    let mut pts = Vec::with_capacity(ns.iter().sum());
    let mut first = Vec::with_capacity(coedges.len());
    for (&c, &n) in coedges.iter().zip(&ns) {
        first.push(pts.len());
        let co = body.coedge(c);
        let e = body.edge(co.edge);
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
    // a procedural face's samples are each found from the one before
    draw_loop_near(body, face, samples)
}

/// `draw_loop` with each sample projected from the one before (`Surface::project_near`), for
/// meshing: the same points to within rounding, where the surface can start from there.
pub fn draw_loop_near(body: &Body, face: FaceId, samples: &[Point3]) -> (Vec<Point2>, Vec<usize>) {
    let surface = &body.face(face).surface;
    // an analytic surface's projection is closed form already
    if !matches!(surface, forge_geom::Surface::Procedural(_)) {
        return draw(body, face, samples.iter().map(|&p| surface.project(p)).collect());
    }
    let mut raw: Vec<Point2> = Vec::with_capacity(samples.len());
    for &p in samples {
        let uv = match raw.last() {
            Some(&near) => surface.project_near(p, near),
            None => surface.project(p),
        };
        raw.push(uv);
    }
    draw(body, face, raw)
}

/// `draw_loop` from the samples' parameters `raw`.
fn draw(body: &Body, face: FaceId, mut raw: Vec<Point2>) -> (Vec<Point2>, Vec<usize>) {
    let surface = &body.face(face).surface;
    let domain = surface.domain();
    let n = raw.len();
    if !raw.iter().any(|&uv| surface.singular(uv)) {
        // one run, unwrapped from the first sample
        for i in 1..n {
            raw[i] = unwrap_in(&domain, raw[i], raw[i - 1]);
        }
        return (raw, (0..n).collect());
    }
    let singular: Vec<bool> = raw.iter().map(|&uv| surface.singular(uv)).collect();
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
            Some(r) => unwrap_in(&domain, raw[i], r),
            None => raw[i],
        };
        runs.last_mut().expect("at least one run").push((p, i));
        prev = Some(p);
    }
    let period = domain.u.period;
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
        inside ^= forge_math::predicates::inside_polygon(p, &l.points);
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

/// Shifts every loop after the first by whole periods so the middle of its span
/// falls in the first loop's span.
pub fn align_loops<'a>(body: &Body, face: FaceId, mut loops: impl Iterator<Item = &'a mut Vec<Point2>>) {
    // spans, not sample centroids: a full-turn loop whose samples bunch near one end
    // would otherwise pull the others a period away
    let span = |pts: &[Point2]| {
        pts.iter().fold(
            (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY),
            |a, q| (a.0.min(q.x), a.1.max(q.x), a.2.min(q.y), a.3.max(q.y)),
        )
    };
    let Some(first) = loops.next() else { return };
    let r = span(first);
    let d = body.face(face).surface.domain();
    // the shift that puts `c` inside [lo, hi], else the one nearest its middle
    let fit = |c: f64, lo: f64, hi: f64, period: Option<f64>| match period {
        Some(per) => [0.0, -per, per, -2.0 * per, 2.0 * per]
            .into_iter()
            .find(|k| c + k >= lo - 1e-9 && c + k <= hi + 1e-9)
            .unwrap_or_else(|| per * ((0.5 * (lo + hi) - c) / per).round()),
        None => 0.0,
    };
    for l in loops {
        let s = span(l);
        let du = fit(0.5 * (s.0 + s.1), r.0, r.1, d.u.period);
        let dv = fit(0.5 * (s.2 + s.3), r.2, r.3, d.v.period);
        for p in l.iter_mut() {
            *p = Point2::new(p.x + du, p.y + dv);
        }
    }
}

/// A procedural face's surface box: over the stretch of the domain its loops span, not all
/// of it, column by column in u, over only the v the face holds there (a cut askew across a
/// blend's end leaves the end columns part of the arc). Kept out of `Body::face_box`,
/// which every face goes through.
#[cold]
#[inline(never)]
pub(crate) fn procedural_box(body: &Body, f: FaceId, pr: &forge_geom::surface::Proc) -> Box3 {
    let d = pr.domain();
    let whole = forge_math::Box2::new(d.u.interval(), d.v.interval());
    let loops = if d.u.period.is_none() && d.v.period.is_none() {
        face_uv_loops(body, f)
    } else {
        Vec::new()
    };
    let mut su = Interval::EMPTY;
    loops.iter().flat_map(|l| &l.points).for_each(|p| su.include(p.x));
    if su.is_empty() || su.width() <= 0.0 {
        return pr.bbox(whole);
    }
    let mut b = Box3::EMPTY;
    let pad = |i: Interval, w: Interval| {
        let g = 0.05 * i.width() + 1e-9 * (1.0 + i.lo.abs().max(i.hi.abs()));
        Interval::new((i.lo - g).max(w.lo), (i.hi + g).min(w.hi))
    };
    // eight columns; one the boundary crosses askew (a run-out's cut) in eight
    // more, twice
    const G: usize = 8;
    let mut todo = vec![(su.lo, su.hi, 0)];
    while let Some((lo, hi, depth)) = todo.pop() {
        let (runs, askew) = column_runs(&loops, lo, hi);
        if depth == 0 || (depth < 3 && askew) {
            let at = |k: usize| lo + (hi - lo) * k as f64 / G as f64;
            todo.extend((0..G).map(|k| (at(k), at(k + 1), depth + 1)));
            continue;
        }
        let u = pad(Interval::new(lo, hi), whole.u);
        for v in runs {
            b = b.hull(&pr.bbox(forge_math::Box2::new(u, pad(v, whole.v))));
        }
    }
    b
}

/// The v a face holds over u in [lo, hi], as runs, from its uv loops: each stretch of boundary
/// inside the column, and between the boundary's crossings of the column's middle; and whether
/// the boundary moves much in v while moving in u there (a cut askew).
fn column_runs(loops: &[UvLoop], lo: f64, hi: f64) -> (Vec<Interval>, bool) {
    let mid = 0.5 * (lo + hi);
    let mut runs: Vec<Interval> = Vec::new();
    let mut cross: Vec<f64> = Vec::new();
    let (mut moves, mut span) = (0.0f64, Interval::EMPTY);
    for l in loops {
        let n = l.points.len();
        let mut piece = Interval::EMPTY;
        for i in 0..n {
            let (p, q) = (l.points[i], l.points[(i + 1) % n]);
            if (p.x <= mid) != (q.x <= mid) {
                cross.push(p.y + (q.y - p.y) * (mid - p.x) / (q.x - p.x));
            }
            let (a, c) = (p.x.min(q.x).max(lo), p.x.max(q.x).min(hi));
            if a > c {
                if !piece.is_empty() {
                    runs.push(piece);
                    piece = Interval::EMPTY;
                }
                continue;
            }
            let at = |u: f64| p.y + (q.y - p.y) * ((u - p.x) / (q.x - p.x)).clamp(0.0, 1.0);
            let (va, vc) = if q.x == p.x { (p.y, q.y) } else { (at(a), at(c)) };
            if q.x != p.x {
                moves += (va - vc).abs();
            }
            piece.include(va);
            piece.include(vc);
            span.include(va);
            span.include(vc);
        }
        if !piece.is_empty() {
            runs.push(piece);
        }
    }
    cross.sort_by(f64::total_cmp);
    runs.extend(cross.as_chunks::<2>().0.iter().map(|&[a, b]| Interval::new(a, b)));
    // merged where they touch or overlap
    runs.sort_by(|x, y| x.lo.total_cmp(&y.lo));
    let mut out: Vec<Interval> = Vec::new();
    for r in runs {
        match out.last_mut() {
            Some(last) if r.lo <= last.hi => last.hi = last.hi.max(r.hi),
            _ => out.push(r),
        }
    }
    (out, moves > 0.05 * span.width())
}
