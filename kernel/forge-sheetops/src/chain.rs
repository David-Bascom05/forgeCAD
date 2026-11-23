//! A sheet's chains checked (`sheet_chains`): a module of its own, with its own copies of the few
//! profile helpers an open chain needs, so the profile's path is compiled as it was.

use forge_math::predicates::orient2d;
use forge_math::{Frame, Interval, Point2, dmath, tol};
use forge_ops::profile::{Profile, ProfileError, ProfileLoop, Segment};
use std::sync::Arc;

/// A chain's segments, and whether it closes.
pub(crate) type Chain = (Vec<(Segment, u64)>, bool);

/// A sheet's chains checked, each with whether it closes (its last end on its first start): segments
/// end to end, none degenerate, ids unique, a circle alone, none crossing itself or another (the later
/// chain's index in `SelfIntersecting`). A loop comes back counter-clockwise; splines are cut at their
/// corners, a loop's lone closed spline in two, as a profile's are.
pub(crate) fn sheet_chains(chains: &[Vec<(Segment, u64)>]) -> Result<Vec<Chain>, ProfileError> {
    if chains.is_empty() || chains.iter().any(|c| c.is_empty()) {
        return Err(ProfileError::Empty);
    }
    let mut ids: Vec<u64> = chains.iter().flat_map(|c| c.iter().map(|s| s.1)).collect();
    ids.sort();
    if let Some(w) = ids.windows(2).find(|w| w[0] == w[1]) {
        return Err(ProfileError::DuplicateId { id: w[0] });
    }
    let mut out = Vec::with_capacity(chains.len());
    for (ci, c) in chains.iter().enumerate() {
        let n = c.len();
        for (i, (s, id)) in c.iter().enumerate() {
            if sheet_degenerate(s) {
                return Err(ProfileError::Degenerate { id: *id });
            }
            if matches!(s, Segment::Circle { .. }) && n != 1 {
                return Err(ProfileError::CircleNotAlone { id: *id });
            }
            if i + 1 < n {
                let gap = s.end().distance(c[i + 1].0.start());
                if gap > tol::LINEAR {
                    return Err(ProfileError::Open { id: *id, gap });
                }
            }
        }
        let closed = matches!(c[..], [(Segment::Circle { .. }, _)])
            || c[n - 1].0.end().distance(c[0].0.start()) <= tol::LINEAR;
        let l = ProfileLoop::new(c.clone());
        // a loop: checked, turned counter-clockwise and cut as a profile's outline is
        if closed {
            let p = Profile {
                plane: Frame::WORLD,
                loops: vec![l],
            }
            .normalized_touching()
            .map_err(|e| match e {
                ProfileError::SelfIntersecting { .. } => ProfileError::SelfIntersecting { loop_index: ci },
                e => e,
            })?;
            out.push((p.loops.into_iter().next().expect("one loop"), true));
            continue;
        }
        let mut poly = sheet_outline(&l);
        poly.push(l.segments[l.segments.len() - 1].0.end());
        if crosses_open(&poly) {
            return Err(ProfileError::SelfIntersecting { loop_index: ci });
        }
        let l = if l.segments.iter().any(|(s, _)| matches!(s, Segment::Spline(_))) {
            sheet_pieces(&l, false).ok_or(ProfileError::Degenerate { id: l.segments[0].1 })?
        } else {
            l
        };
        out.push((l, closed));
    }
    // two chains crossing or touching: the polylines of both, joined end to end at a gap, cross
    for i in 1..out.len() {
        for j in 0..i {
            if chains_meet(&out[i].0, out[i].1, &out[j].0, out[j].1) {
                return Err(ProfileError::SelfIntersecting { loop_index: i });
            }
        }
    }
    Ok(out.into_iter().map(|(l, closed)| (l.segments, closed)).collect())
}

/// Whether two chains' sampled polylines cross or touch, edge against edge, exactly.
fn chains_meet(a: &ProfileLoop, a_closed: bool, b: &ProfileLoop, b_closed: bool) -> bool {
    let poly = |l: &ProfileLoop, closed: bool| {
        let mut p = sheet_outline(l);
        p.push(if closed {
            p[0]
        } else {
            l.segments[l.segments.len() - 1].0.end()
        });
        p
    };
    let (p, q) = (poly(a, a_closed), poly(b, b_closed));
    let apart = |x: f64, y: f64| (x > 0.0 && y < 0.0) || (x < 0.0 && y > 0.0);
    let on = |p: Point2, q: Point2, x: Point2| {
        x.x >= p.x.min(q.x) && x.x <= p.x.max(q.x) && x.y >= p.y.min(q.y) && x.y <= p.y.max(q.y)
    };
    for u in p.windows(2) {
        let (a, b) = (u[0], u[1]);
        for v in q.windows(2) {
            let (c, d) = (v[0], v[1]);
            if a.x.max(b.x) < c.x.min(d.x)
                || c.x.max(d.x) < a.x.min(b.x)
                || a.y.max(b.y) < c.y.min(d.y)
                || c.y.max(d.y) < a.y.min(b.y)
            {
                continue;
            }
            let (o1, o2, o3, o4) = (
                orient2d(a, b, c),
                orient2d(a, b, d),
                orient2d(c, d, a),
                orient2d(c, d, b),
            );
            if (apart(o1, o2) && apart(o3, o4))
                || (o1 == 0.0 && on(a, b, c))
                || (o2 == 0.0 && on(a, b, d))
                || (o3 == 0.0 && on(c, d, a))
                || (o4 == 0.0 && on(c, d, b))
            {
                return true;
            }
        }
    }
    false
}

/// `crosses_itself` of an open polyline: its last point not joined back to its first (a sheet's chain).
fn crosses_open(poly: &[Point2]) -> bool {
    let n = poly.len();
    let seg = |i: usize| (poly[i], poly[(i + 1) % n]);
    let mut order: Vec<usize> = (0..n - 1).collect();
    order.sort_unstable_by(|&i, &j| {
        let (a, b) = (seg(i), seg(j));
        a.0.x.min(a.1.x).total_cmp(&b.0.x.min(b.1.x))
    });
    // strictly opposite sides (signs, not products: they could underflow)
    let apart = |x: f64, y: f64| (x > 0.0 && y < 0.0) || (x < 0.0 && y > 0.0);
    for (k, &i) in order.iter().enumerate() {
        let (a, b) = seg(i);
        let right = a.x.max(b.x);
        for &j in &order[k + 1..] {
            let (c, d) = seg(j);
            if c.x.min(d.x) > right {
                break;
            }
            if (i + 1) % n == j
                || (j + 1) % n == i
                || a.y.max(b.y) < c.y.min(d.y)
                || c.y.max(d.y) < a.y.min(b.y)
            {
                continue;
            }
            let (o1, o2, o3, o4) = (
                orient2d(a, b, c),
                orient2d(a, b, d),
                orient2d(c, d, a),
                orient2d(c, d, b),
            );
            if apart(o1, o2) && apart(o3, o4) {
                return true;
            }
            let on = |p: Point2, q: Point2, x: Point2| {
                x.x >= p.x.min(q.x) && x.x <= p.x.max(q.x) && x.y >= p.y.min(q.y) && x.y <= p.y.max(q.y)
            };
            if (o1 == 0.0 && on(a, b, c))
                || (o2 == 0.0 && on(a, b, d))
                || (o3 == 0.0 && on(c, d, a))
                || (o4 == 0.0 && on(c, d, b))
            {
                return true;
            }
        }
    }
    false
}

/// `Segment::degenerate`, for a sheet's chains (its own copy: the profile's path is compiled as it was).
fn sheet_degenerate(s: &Segment) -> bool {
    match *s {
        Segment::Spline(ref c) => {
            let (b, r) = (c.c.bbox(c.c.domain()), tol::MIN_LENGTH);
            !(b.x.width() > r || b.y.width() > r) || !b.diagonal().is_finite() || !c.c.is_regular()
        }
        // written to be true for NaN or infinite sizes too
        Segment::Line { a, b } => !(a.distance(b) > tol::MIN_LENGTH && a.distance(b) < f64::INFINITY),
        Segment::Arc { radius, sweep, .. } => {
            !(radius > tol::MIN_LENGTH
                && radius < f64::INFINITY
                && (radius * sweep).abs() > tol::MIN_LENGTH
                && sweep.abs() < dmath::TAU - tol::ANGULAR)
        }
        Segment::Circle { radius, .. } => !(radius > tol::MIN_LENGTH && radius < f64::INFINITY),
    }
}

/// `ProfileLoop::smooth_pieces` for a sheet's chain, open or closed (an open chain's lone curve is left
/// whole).
fn sheet_pieces(l: &ProfileLoop, closed: bool) -> Option<ProfileLoop> {
    let mut out = Vec::with_capacity(l.segments.len());
    let alone = closed && l.segments.len() == 1;
    for (s, id) in &l.segments {
        let Segment::Spline(c) = s else {
            out.push((s.clone(), *id));
            continue;
        };
        let d = c.c.domain();
        let mut cuts = c.c.corners();
        if cuts.is_empty() && alone {
            // a joint where the pieces meet, else the middle
            let b = c.c.breaks();
            cuts.push(if b.len() > 2 { b[b.len() / 2] } else { d.mid() });
        }
        if cuts.is_empty() {
            out.push((s.clone(), *id));
            continue;
        }
        let mut lo = d.lo;
        for (k, &hi) in cuts.iter().chain([&d.hi]).enumerate() {
            let part = c.part(Interval::new(lo, hi))?;
            out.push((Segment::Spline(Arc::new(part)), *id ^ ((k as u64) << 48)));
            lo = hi;
        }
    }
    Some(ProfileLoop::new(out))
}

/// `ProfileLoop::outline`, for a sheet's chain.
fn sheet_outline(l: &ProfileLoop) -> Vec<Point2> {
    l.segments
        .iter()
        .flat_map(|(s, _)| s.samples(if matches!(s, Segment::Line { .. }) { 1 } else { 16 }))
        .collect()
}
