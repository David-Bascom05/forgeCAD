//! TRIM, EXTEND, BREAK, JOIN and LENGTHEN, as AutoCAD does them.

use super::curve::{Arc, Curve, Ellipse, Poly, Seg};
use super::geom::{Bbox, TAU, Tol, V2, wrap};
use super::intersect::{Ext, intersect_tol};
use super::nurbs::Nurbs;

fn tol_of(cs: &[&Curve]) -> f64 {
    let b = cs.iter().fold(Bbox::EMPTY, |b, c| b.union(c.bbox()));
    Tol::of_box(&b).lin
}

/// The parameters where `obj` meets the edges (on obj; on the edges, or their extensions when
/// `extend_edges`, AutoCAD's EDGEMODE 1): crossings, touches and the ends of shared stretches.
pub fn cut_params(obj: &Curve, edges: &[Curve], extend_edges: bool) -> Vec<f64> {
    let mut ts = vec![];
    for e in edges {
        let tol = tol_of(&[obj, e]);
        let ext = if extend_edges { Ext::B } else { Ext::None };
        let h = intersect_tol(obj, e, ext, tol);
        for x in h.points {
            if x.on_a && (x.on_b || extend_edges) {
                ts.push(x.ta);
            }
        }
        for o in h.overlaps {
            ts.extend([o.a.0, o.a.1]);
        }
    }
    ts.sort_by(f64::total_cmp);
    ts.dedup_by(|a, b| (*a - *b).abs() <= 1e-12 * (1.0 + b.abs()));
    ts
}

/// The parameter tolerance near `t`.
fn ptol(c: &Curve, t: f64, tol: f64) -> f64 {
    let s = c.deriv(t).len();
    if s > 0.0 { tol / s } else { 1e-12 }
}

/// TRIM: removes the piece of `obj` between the cutting edges' nearest intersections either side
/// of `pick`, or from one to the end when there's none on that side. What is left: none, one or
/// two curves (a closed curve opens into one: a circle into an arc). None when nothing cuts it
/// (an intersection only at an open curve's own end doesn't).
pub fn trim(obj: &Curve, edges: &[Curve], pick: V2, extend_edges: bool) -> Option<Vec<Curve>> {
    let tol = tol_of(&[obj]);
    let (lo, hi) = obj.domain();
    let tp = obj.param_of(pick);
    let mut ts = cut_params(obj, edges, extend_edges);
    if obj.is_closed() {
        ts.retain(|&t| t >= lo && t <= hi);
        // the start and end are one point
        for t in &mut ts {
            if (*t - hi).abs() <= ptol(obj, hi, tol) {
                *t = lo;
            }
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|a, b| (*a - *b).abs() <= ptol(obj, *b, tol));
        if ts.len() < 2 {
            return None;
        }
        let before = ts
            .iter()
            .copied()
            .filter(|&t| t < tp)
            .next_back()
            .unwrap_or(ts[ts.len() - 1]);
        let after = ts.iter().copied().find(|&t| t > tp).unwrap_or(ts[0]);
        return Some(obj.sub(after, before).into_iter().collect());
    }
    // an open curve's own ends don't cut it
    ts.retain(|&t| {
        (lo.is_infinite() || t > lo + ptol(obj, lo, tol)) && (hi.is_infinite() || t < hi - ptol(obj, hi, tol))
    });
    let before = ts.iter().copied().filter(|&t| t < tp).next_back();
    let after = ts.iter().copied().find(|&t| t > tp);
    if before.is_none() && after.is_none() {
        return None;
    }
    let mut out = vec![];
    if let Some(b) = before
        && let Some(c) = obj.sub(lo, b)
    {
        out.push(c);
    }
    if let Some(a) = after
        && let Some(c) = obj.sub(a, hi)
    {
        out.push(c);
    }
    Some(out)
}

/// Which end of an open curve is nearer a pick (along the curve): true for the start.
pub fn near_start(obj: &Curve, pick: V2) -> bool {
    let (lo, hi) = obj.domain();
    if lo.is_infinite() {
        return false;
    }
    if hi.is_infinite() {
        return true;
    }
    let t = obj.param_of(pick);
    obj.length_between(lo, t) <= obj.length_between(t, hi)
}

/// The end segment of an open curve as a line or arc running out of its end (reversed for the
/// start), when it has a natural extension.
fn end_piece(obj: &Curve, start: bool) -> Option<Curve> {
    let c = match obj {
        Curve::Line { .. } | Curve::Arc(_) => obj.clone(),
        Curve::Ellipse(e) if !e.is_full() => obj.clone(),
        Curve::Poly(p) if !p.closed && p.nseg() > 0 => {
            let k = if start { 0 } else { p.nseg() - 1 };
            p.seg(k).curve()
        }
        _ => return None,
    };
    Some(if start { c.reversed() } else { c })
}

/// How far past its end `piece` (a line, arc or elliptical arc) runs to reach `p` on its
/// extension: the line parameter past 1, the angle past the sweep. None when p is not ahead.
fn travel_past_end(piece: &Curve, p: V2, eps: f64) -> Option<f64> {
    match piece {
        Curve::Line { a, b } => {
            let d = *b - *a;
            let t = (p - *a).dot(d) / d.len2();
            (t > 1.0 + eps / d.len()).then_some(t)
        }
        Curve::Arc(a) => {
            let phi = a.param_of_angle((p - a.c).angle());
            (phi > a.len() + eps / a.r && phi < TAU - eps / a.r).then_some(phi)
        }
        Curve::Ellipse(e) => {
            let phi = e.param_of_angle(e.angle_of(p));
            let b = e.b().max(1e-300);
            (phi > e.sweep.abs() + eps / b && phi < TAU - eps / b).then_some(phi)
        }
        _ => None,
    }
}

/// The piece with its end moved to extension parameter `t` (as `travel_past_end` measures).
fn piece_to(piece: &Curve, t: f64) -> Option<Curve> {
    match piece {
        Curve::Line { a, b } => Some(Curve::Line {
            a: *a,
            b: a.lerp(*b, t),
        }),
        Curve::Arc(a) => Some(Curve::Arc(a.sub(0.0, t))),
        Curve::Ellipse(e) => Some(Curve::Ellipse(Ellipse {
            sweep: e.dir() * t,
            ..*e
        })),
        _ => None,
    }
}

/// Puts a changed end piece back into the curve.
fn replace_end(obj: &Curve, start: bool, piece: Curve) -> Option<Curve> {
    let piece = if start { piece.reversed() } else { piece };
    match obj {
        Curve::Poly(p) => {
            let mut segs = p.segs();
            let seg = match piece {
                Curve::Line { a, b } => Seg::Line(a, b),
                Curve::Arc(a) => Seg::Arc(a),
                _ => return None,
            };
            if start {
                segs[0] = seg;
            } else {
                let n = segs.len();
                segs[n - 1] = seg;
            }
            let mut q = Poly::from_segs(&segs, false);
            q.widths = p.widths.clone();
            q.const_width = p.const_width;
            Some(Curve::Poly(q))
        }
        _ => Some(piece),
    }
}

/// EXTEND: the end of `obj` nearer `pick` runs on along its natural extension (a line's
/// direction, an arc's circle, an elliptical arc's ellipse, an open polyline's end segment) to
/// the nearest boundary it meets (the boundaries' extensions too with `extend_bounds`). None
/// when it meets none.
pub fn extend(obj: &Curve, bounds: &[Curve], pick: V2, extend_bounds: bool) -> Option<Curve> {
    if obj.is_closed() {
        return None;
    }
    let start = near_start(obj, pick);
    let piece = end_piece(obj, start)?;
    let mut best: Option<f64> = None;
    for b in bounds {
        let tol = tol_of(&[&piece, b]);
        let ext = if extend_bounds { Ext::Both } else { Ext::A };
        for h in intersect_tol(&piece, b, ext, tol).points {
            if !h.on_b && !extend_bounds {
                continue;
            }
            if let Some(t) = travel_past_end(&piece, h.p, tol)
                && best.is_none_or(|x| t < x)
            {
                best = Some(t);
            }
        }
    }
    replace_end(obj, start, piece_to(&piece, best?)?)
}

/// How LENGTHEN changes the picked end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lengthen {
    /// By a length (negative shortens).
    Delta(f64),
    /// An arc by an angle (radians; negative shortens).
    DeltaAngle(f64),
    /// To a percentage of the length.
    Percent(f64),
    /// To a total length.
    Total(f64),
    /// An arc to a total included angle.
    TotalAngle(f64),
}

/// LENGTHEN: the end nearer `pick` moves so the curve has the new length (lines, arcs, open
/// polylines by their end segment, elliptical arcs; splines shorten only). None when it can't
/// (an arc past a whole turn, a closed curve, a length of zero or less).
pub fn lengthen(obj: &Curve, pick: V2, how: Lengthen) -> Option<Curve> {
    if obj.is_closed() || !obj.is_bounded() {
        return None;
    }
    let len = obj.length();
    let target = match (how, obj) {
        (Lengthen::Delta(d), _) => len + d,
        (Lengthen::Percent(p), _) => len * p / 100.0,
        (Lengthen::Total(t), _) => t,
        (Lengthen::DeltaAngle(a), Curve::Arc(arc)) => len + a * arc.r,
        (Lengthen::TotalAngle(a), Curve::Arc(arc)) => a * arc.r,
        _ => return None,
    };
    if target <= 0.0 || !target.is_finite() {
        return None;
    }
    let start = near_start(obj, pick);
    let c = if start { obj.reversed() } else { obj.clone() };
    let out = lengthen_end(&c, target)?;
    Some(if start { out.reversed() } else { out })
}

fn lengthen_end(c: &Curve, target: f64) -> Option<Curve> {
    let len = c.length();
    let (lo, _) = c.domain();
    if target < len {
        let t = c.param_at_length(target);
        return c.sub(lo, t);
    }
    let extra = target - len;
    match c {
        Curve::Line { a, b } => {
            let d = (*b - *a).unit()?;
            Some(Curve::Line {
                a: *a,
                b: *b + d * extra,
            })
        }
        Curve::Arc(a) => {
            let sw = a.len() + extra / a.r;
            (sw < TAU).then(|| Curve::Arc(a.sub(0.0, sw)))
        }
        Curve::Ellipse(e) => {
            let full = Curve::Ellipse(Ellipse {
                sweep: e.dir() * TAU,
                ..*e
            });
            if target >= full.length() {
                return None;
            }
            let f = |t: f64| full.length_between(0.0, t) - target;
            let t = Nurbs::root(&f, e.sweep.abs(), TAU, 1e-14);
            Some(Curve::Ellipse(Ellipse {
                sweep: e.dir() * t,
                ..*e
            }))
        }
        Curve::Poly(_) => {
            let piece = end_piece(c, false)?;
            let grown = lengthen_end(&piece, piece.length() + extra)?;
            replace_end(c, false, grown)
        }
        _ => None,
    }
}

/// BREAK: removes what lies between two points on `obj` (on a closed curve, from the first
/// to the second in its direction: counter-clockwise on a circle); with one point, splits an
/// open curve there (BREAKATPOINT; a closed curve can't break at one point). What remains.
pub fn break_at(obj: &Curve, p1: V2, p2: Option<V2>) -> Option<Vec<Curve>> {
    let (lo, hi) = obj.domain();
    let t1 = obj.param_of(p1);
    let tol = tol_of(&[obj]);
    match p2 {
        None => {
            if obj.is_closed() {
                return None;
            }
            let e = ptol(obj, t1, tol);
            if t1 <= lo + e || t1 >= hi - e {
                return None;
            }
            Some(vec![obj.sub(lo, t1)?, obj.sub(t1, hi)?])
        }
        Some(p2) => {
            let t2 = obj.param_of(p2);
            if obj.is_closed() {
                if (t1 - t2).abs() <= ptol(obj, t1, tol) {
                    return None;
                }
                return Some(obj.sub(t2, t1).into_iter().collect());
            }
            let (a, b) = (t1.min(t2), t1.max(t2));
            let mut out = vec![];
            if a > lo + ptol(obj, lo, tol)
                && let Some(c) = obj.sub(lo, a)
            {
                out.push(c);
            }
            if b < hi - ptol(obj, hi, tol)
                && let Some(c) = obj.sub(b, hi)
            {
                out.push(c);
            }
            Some(out)
        }
    }
}

/// A JOIN result: the curve and which inputs it is made of (by index, in order).
#[derive(Clone, Debug, PartialEq)]
pub struct Joined {
    pub curve: Curve,
    pub from: Vec<usize>,
}

/// JOIN: collinear lines into one line (gaps and overlaps bridged), arcs on one circle into one
/// arc counter-clockwise from the first (a circle when they close it), elliptical arcs on one
/// ellipse alike, and lines, arcs and polylines meeting end to end (within `tol`, or a part in
/// a million of their size) into polylines. Every input is in exactly one result; one that joins
/// nothing comes back as it was.
pub fn join(curves: &[Curve], tol: Option<f64>) -> Vec<Joined> {
    let bb = curves.iter().fold(Bbox::EMPTY, |b, c| b.union(c.bbox()));
    let tol = tol.unwrap_or_else(|| (bb.size() * 1e-6).max(Tol::of_box(&bb).lin));
    let mut used = vec![false; curves.len()];
    let mut out = vec![];
    // collinear lines
    for i in 0..curves.len() {
        let Curve::Line { a, b } = curves[i] else { continue };
        if used[i] {
            continue;
        }
        let Some(d) = (b - a).unit() else { continue };
        let mut group = vec![i];
        for (j, c) in curves.iter().enumerate().skip(i + 1) {
            if let Curve::Line { a: p, b: q } = *c
                && !used[j]
                && (p - a).cross(d).abs() <= tol
                && (q - a).cross(d).abs() <= tol
            {
                group.push(j);
            }
        }
        if group.len() < 2 {
            continue;
        }
        let (mut s0, mut s1) = (0.0f64, (b - a).dot(d));
        for &j in &group {
            if let Curve::Line { a: p, b: q } = curves[j] {
                for x in [p, q] {
                    let s = (x - a).dot(d);
                    s0 = s0.min(s);
                    s1 = s1.max(s);
                }
            }
            used[j] = true;
        }
        out.push(Joined {
            curve: Curve::Line {
                a: a + d * s0,
                b: a + d * s1,
            },
            from: group,
        });
    }
    // arcs on one circle
    for i in 0..curves.len() {
        let Curve::Arc(a0) = curves[i] else { continue };
        if used[i] {
            continue;
        }
        let src = a0.ccw();
        let mut group = vec![i];
        for (j, c) in curves.iter().enumerate().skip(i + 1) {
            if let Curve::Arc(b) = c
                && !used[j]
                && b.c.dist(src.c) <= tol
                && (b.r - src.r).abs() <= tol
            {
                group.push(j);
            }
        }
        if group.len() < 2 {
            continue;
        }
        let ivs: Vec<(f64, f64)> = group
            .iter()
            .map(|&j| match &curves[j] {
                Curve::Arc(b) => {
                    let b = b.ccw();
                    (b.start, b.sweep)
                }
                _ => (0.0, 0.0),
            })
            .collect();
        for &j in &group {
            used[j] = true;
        }
        let curve = match angular_union(&ivs, tol / src.r) {
            None => Curve::Circle { c: src.c, r: src.r },
            Some(sweep) => Curve::Arc(Arc { sweep, ..src }),
        };
        out.push(Joined { curve, from: group });
    }
    // elliptical arcs on one ellipse
    for i in 0..curves.len() {
        let Curve::Ellipse(e0) = curves[i] else { continue };
        if used[i] || e0.is_full() {
            continue;
        }
        let src = if e0.sweep < 0.0 { e0.reversed() } else { e0 };
        let same = |e: &Ellipse| {
            e.c.dist(src.c) <= tol
                && (e.a() - src.a()).abs() <= tol
                && (e.b() - src.b()).abs() <= tol
                && e.major.unit_or_zero().cross(src.major.unit_or_zero()).abs() <= 1e-9
        };
        let mut group = vec![i];
        for (j, c) in curves.iter().enumerate().skip(i + 1) {
            if let Curve::Ellipse(e) = c
                && !used[j]
                && !e.is_full()
                && same(e)
            {
                group.push(j);
            }
        }
        if group.len() < 2 {
            continue;
        }
        // each arc as an eccentric-angle interval of the source ellipse
        let ivs: Vec<(f64, f64)> = group
            .iter()
            .map(|&j| match &curves[j] {
                Curve::Ellipse(e) => {
                    let e = if e.sweep < 0.0 { e.reversed() } else { *e };
                    let s = src.angle_of(e.point(0.0));
                    (s, e.sweep)
                }
                _ => (0.0, 0.0),
            })
            .collect();
        for &j in &group {
            used[j] = true;
        }
        let rel = ivs[0].0;
        let curve = match angular_union(&ivs, tol / src.b().max(1e-300)) {
            None => Curve::Ellipse(Ellipse {
                start: 0.0,
                sweep: TAU,
                ..src
            }),
            Some(sweep) => Curve::Ellipse(Ellipse {
                start: rel,
                sweep,
                ..src
            }),
        };
        out.push(Joined { curve, from: group });
    }
    // chains end to end
    let chainable = |c: &Curve| match c {
        Curve::Line { .. } | Curve::Arc(_) => true,
        Curve::Poly(p) => !p.closed,
        _ => false,
    };
    for i in 0..curves.len() {
        if used[i] {
            continue;
        }
        used[i] = true;
        if !chainable(&curves[i]) {
            out.push(Joined {
                curve: curves[i].clone(),
                from: vec![i],
            });
            continue;
        }
        let mut segs: Vec<Seg> = segs_of(&curves[i]);
        let mut from = vec![i];
        loop {
            let (s, e) = (segs[0].start(), segs[segs.len() - 1].end());
            if s.dist(e) <= tol && segs.len() > 1 {
                break;
            }
            let mut grew = false;
            for (j, c) in curves.iter().enumerate() {
                if used[j] || !chainable(c) {
                    continue;
                }
                let (cs, ce) = (c.start(), c.end());
                let add: Option<(bool, bool)> = if cs.dist(e) <= tol {
                    Some((true, false))
                } else if ce.dist(e) <= tol {
                    Some((true, true))
                } else if ce.dist(s) <= tol {
                    Some((false, false))
                } else if cs.dist(s) <= tol {
                    Some((false, true))
                } else {
                    None
                };
                let Some((at_end, rev)) = add else { continue };
                let c = if rev { c.reversed() } else { c.clone() };
                let mut more = segs_of(&c);
                if at_end {
                    snap_start(&mut more, e);
                    segs.extend(more);
                    from.push(j);
                } else {
                    let n = more.len();
                    snap_end(&mut more[n - 1], s);
                    more.extend(segs);
                    segs = more;
                    from.insert(0, j);
                }
                used[j] = true;
                grew = true;
                break;
            }
            if !grew {
                break;
            }
        }
        if from.len() == 1 {
            out.push(Joined {
                curve: curves[i].clone(),
                from,
            });
            continue;
        }
        let closed = segs.len() > 1 && segs[0].start().dist(segs[segs.len() - 1].end()) <= tol;
        let poly = Poly::from_segs(&segs, closed);
        out.push(Joined {
            curve: Curve::Poly(poly),
            from,
        });
    }
    out
}

fn segs_of(c: &Curve) -> Vec<Seg> {
    match c {
        Curve::Line { a, b } => vec![Seg::Line(*a, *b)],
        Curve::Arc(a) => vec![Seg::Arc(*a)],
        Curve::Poly(p) => p.segs(),
        _ => vec![],
    }
}

fn snap_start(segs: &mut [Seg], p: V2) {
    if let Some(Seg::Line(a, _)) = segs.first_mut() {
        *a = p;
    }
}

fn snap_end(s: &mut Seg, p: V2) {
    if let Seg::Line(_, b) = s {
        *b = p;
    }
}

/// The counter-clockwise sweep from the first interval's start covering all of them, or None when
/// together they close the circle. Intervals are (start, sweep >= 0).
fn angular_union(ivs: &[(f64, f64)], eps: f64) -> Option<f64> {
    let s0 = ivs[0].0;
    let mut rel: Vec<(f64, f64)> = ivs
        .iter()
        .map(|&(s, w)| (wrap(s - s0), wrap(s - s0) + w))
        .collect();
    // an interval straddling the start counts from just before it
    rel.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut covered = 0.0f64;
    let mut gap = false;
    let mut far = 0.0f64;
    for &(a, b) in &rel {
        if a > covered + eps {
            gap = true;
        }
        if !gap {
            covered = covered.max(b);
        }
        far = far.max(b);
    }
    if !gap && covered >= TAU - eps {
        return None;
    }
    if far >= TAU - eps {
        // the far arcs wrap back onto the start
        return None;
    }
    Some(far)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draft::geom::{PI, v2};

    fn close(a: V2, b: V2) -> bool {
        a.dist(b) < 1e-9
    }

    #[test]
    fn trim_a_circle_between_two_lines() {
        // two vertical lines cut a circle; pick on its right: the right part goes
        let c = Curve::circle(V2::ZERO, 10.0);
        let edges = [
            Curve::line(v2(-5.0, -20.0), v2(-5.0, 20.0)),
            Curve::line(v2(5.0, -20.0), v2(5.0, 20.0)),
        ];
        let r = trim(&c, &edges, v2(10.0, 0.0), false).unwrap();
        assert_eq!(r.len(), 1);
        let Curve::Arc(a) = &r[0] else {
            panic!("{:?}", r[0])
        };
        // from the upper right crossing round the left to the lower right one
        let y = (100.0f64 - 25.0).sqrt();
        assert!(close(a.start_pt(), v2(5.0, y)) && close(a.end_pt(), v2(5.0, -y)));
        assert!(r[0].dist(v2(-10.0, 0.0)) < 1e-9 && r[0].dist(v2(10.0, 0.0)) > 1.0);
    }

    #[test]
    fn trim_lines() {
        let l = Curve::line(v2(0.0, 0.0), v2(10.0, 0.0));
        let edges = [
            Curve::line(v2(3.0, -1.0), v2(3.0, 1.0)),
            Curve::line(v2(7.0, -1.0), v2(7.0, 1.0)),
        ];
        let r = trim(&l, &edges, v2(5.0, 0.2), false).unwrap();
        assert_eq!(r.len(), 2);
        assert!(close(r[0].end(), v2(3.0, 0.0)) && close(r[1].start(), v2(7.0, 0.0)));
        let r = trim(&l, &edges, v2(9.0, 0.0), false).unwrap();
        assert_eq!(r.len(), 1);
        assert!(close(r[0].start(), v2(0.0, 0.0)) && close(r[0].end(), v2(7.0, 0.0)));
        // no edge crosses
        assert!(
            trim(
                &l,
                &[Curve::line(v2(20.0, -1.0), v2(20.0, 1.0))],
                v2(5.0, 0.0),
                false
            )
            .is_none()
        );
        // with the edge's extension (EDGEMODE 1)
        let short = [Curve::line(v2(5.0, 2.0), v2(5.0, 3.0))];
        assert!(trim(&l, &short, v2(8.0, 0.0), false).is_none());
        let r = trim(&l, &short, v2(8.0, 0.0), true).unwrap();
        assert!(close(r[0].end(), v2(5.0, 0.0)));
        // an edge through the line's end doesn't cut it
        let r = trim(
            &l,
            &[Curve::line(v2(10.0, -1.0), v2(10.0, 1.0)), edges[0].clone()],
            v2(8.0, 0.0),
            false,
        );
        assert!(close(r.unwrap()[0].end(), v2(3.0, 0.0)));
    }

    #[test]
    fn trim_a_polyline_across_segments() {
        // a U: cut its bottom near both corners by one line; pick on the left upright
        let p = Curve::poly(
            &[v2(0.0, 10.0), v2(0.0, 0.0), v2(10.0, 0.0), v2(10.0, 10.0)],
            &[],
            false,
        );
        let edge = Curve::line(v2(-1.0, 5.0), v2(11.0, 5.0));
        // pick on the bottom: the piece between the two crossings (across three segments' worth)
        let r = trim(&p, std::slice::from_ref(&edge), v2(5.0, 0.0), false).unwrap();
        assert_eq!(r.len(), 2);
        assert!(close(r[0].start(), v2(0.0, 10.0)) && close(r[0].end(), v2(0.0, 5.0)));
        assert!(close(r[1].start(), v2(10.0, 5.0)) && close(r[1].end(), v2(10.0, 10.0)));
        // a closed polyline opens
        let sq = Curve::poly(
            &[v2(0.0, 0.0), v2(10.0, 0.0), v2(10.0, 10.0), v2(0.0, 10.0)],
            &[],
            true,
        );
        let r = trim(&sq, &[edge], v2(5.0, 10.0), false).unwrap();
        assert_eq!(r.len(), 1);
        let Curve::Poly(q) = &r[0] else { panic!() };
        assert!(!q.closed && q.pts.len() == 4);
        assert!(close(r[0].start(), v2(0.0, 5.0)) && close(r[0].end(), v2(10.0, 5.0)));
        assert!((r[0].length() - 20.0).abs() < 1e-9);
    }

    #[test]
    fn extend_a_line_to_an_arc() {
        let l = Curve::line(v2(0.0, 0.0), v2(5.0, 0.0));
        let arc = Curve::arc(v2(20.0, 0.0), 5.0, PI / 2.0, 3.0 * PI / 2.0);
        let r = extend(&l, std::slice::from_ref(&arc), v2(4.0, 0.0), false).unwrap();
        assert!(close(r.end(), v2(15.0, 0.0)) && close(r.start(), v2(0.0, 0.0)));
        // the start end: nothing there
        assert!(extend(&l, std::slice::from_ref(&arc), v2(1.0, 0.0), false).is_none());
        // an arc extends round its circle to a line
        let a = Curve::arc(V2::ZERO, 10.0, 0.0, PI / 4.0);
        let wall = Curve::line(v2(-20.0, 5.0), v2(20.0, 5.0));
        let r = extend(&a, std::slice::from_ref(&wall), a.end(), false).unwrap();
        assert!((r.end().y - 5.0).abs() < 1e-9 || (r.start().y - 5.0).abs() < 1e-9);
        // an open polyline's end segment
        let p = Curve::poly(&[v2(0.0, 0.0), v2(5.0, 0.0), v2(5.0, 5.0)], &[], false);
        let top = Curve::line(v2(0.0, 12.0), v2(10.0, 12.0));
        let r = extend(&p, &[top], v2(5.0, 4.0), false).unwrap();
        assert!(close(r.end(), v2(5.0, 12.0)));
    }

    #[test]
    fn break_and_lengthen() {
        let c = Curve::circle(V2::ZERO, 1.0);
        let r = break_at(&c, v2(1.0, 0.0), Some(v2(0.0, 1.0))).unwrap();
        // the counter-clockwise quarter from the first point to the second is gone
        let Curve::Arc(a) = &r[0] else { panic!() };
        assert!(close(a.start_pt(), v2(0.0, 1.0)) && close(a.end_pt(), v2(1.0, 0.0)));
        assert!((a.len() - 1.5 * PI).abs() < 1e-12);
        assert!(break_at(&c, v2(1.0, 0.0), None).is_none());
        let l = Curve::line(v2(0.0, 0.0), v2(10.0, 0.0));
        let r = break_at(&l, v2(4.0, 0.0), Some(v2(6.0, 0.0))).unwrap();
        assert_eq!(r.len(), 2);
        let r = break_at(&l, v2(4.0, 0.0), None).unwrap();
        assert!(close(r[0].end(), v2(4.0, 0.0)) && close(r[1].start(), v2(4.0, 0.0)));
        let x = lengthen(&l, v2(9.0, 0.0), Lengthen::Delta(2.0)).unwrap();
        assert!(close(x.end(), v2(12.0, 0.0)));
        let x = lengthen(&l, v2(1.0, 0.0), Lengthen::Total(4.0)).unwrap();
        assert!(close(x.start(), v2(6.0, 0.0)) && close(x.end(), v2(10.0, 0.0)));
        let x = lengthen(&l, v2(9.0, 0.0), Lengthen::Percent(50.0)).unwrap();
        assert!(close(x.end(), v2(5.0, 0.0)));
        let a = Curve::arc(V2::ZERO, 2.0, 0.0, PI / 2.0);
        let x = lengthen(&a, a.end(), Lengthen::DeltaAngle(PI / 2.0)).unwrap();
        assert!(close(x.end(), v2(-2.0, 0.0)));
        let x = lengthen(&a, a.end(), Lengthen::TotalAngle(PI / 4.0)).unwrap();
        assert!((x.length() - PI / 2.0).abs() < 1e-12);
        let e = Curve::Ellipse(Ellipse {
            c: V2::ZERO,
            major: v2(3.0, 0.0),
            ratio: 0.5,
            start: 0.0,
            sweep: PI / 2.0,
        });
        let x = lengthen(&e, e.end(), Lengthen::Delta(1.0)).unwrap();
        assert!((x.length() - e.length() - 1.0).abs() < 1e-9);
        let p = Curve::poly(&[v2(0.0, 0.0), v2(5.0, 0.0), v2(5.0, 5.0)], &[], false);
        let x = lengthen(&p, v2(5.0, 5.0), Lengthen::Delta(1.0)).unwrap();
        assert!(close(x.end(), v2(5.0, 6.0)));
        let x = lengthen(&p, v2(5.0, 5.0), Lengthen::Delta(-7.0)).unwrap();
        assert!(close(x.end(), v2(3.0, 0.0)));
    }

    #[test]
    fn join_lines_arcs_and_chains() {
        // collinear lines with a gap and an overlap
        let ls = [
            Curve::line(v2(0.0, 0.0), v2(3.0, 0.0)),
            Curve::line(v2(5.0, 0.0), v2(8.0, 0.0)),
            Curve::line(v2(7.0, 0.0), v2(10.0, 0.0)),
        ];
        let j = join(&ls, None);
        assert_eq!(j.len(), 1);
        assert_eq!(j[0].curve, Curve::line(v2(0.0, 0.0), v2(10.0, 0.0)));
        // arcs of one circle
        let arcs = [
            Curve::arc(V2::ZERO, 1.0, 0.0, PI / 2.0),
            Curve::arc(V2::ZERO, 1.0, PI / 2.0, PI),
        ];
        let j = join(&arcs, None);
        let Curve::Arc(a) = &j[0].curve else { panic!() };
        assert!((a.sweep - PI).abs() < 1e-12);
        let ring = [
            Curve::arc(V2::ZERO, 1.0, 0.0, PI),
            Curve::arc(V2::ZERO, 1.0, PI, TAU),
        ];
        assert!(matches!(join(&ring, None)[0].curve, Curve::Circle { .. }));
        // lines end to end into a polyline (one reversed), closing
        let tri = [
            Curve::line(v2(0.0, 0.0), v2(4.0, 0.0)),
            Curve::line(v2(4.0, 3.0), v2(4.0, 0.0)),
            Curve::line(v2(4.0, 3.0), v2(0.0, 0.0)),
        ];
        let j = join(&tri, None);
        assert_eq!(j.len(), 1);
        let Curve::Poly(p) = &j[0].curve else { panic!() };
        assert!(p.closed && p.pts.len() == 3);
        assert!((j[0].curve.length() - 12.0).abs() < 1e-12);
        // a line and an arc into an open polyline
        let la = [
            Curve::line(v2(-1.0, -1.0), v2(1.0, 0.0)),
            Curve::arc(V2::ZERO, 1.0, 0.0, PI / 2.0),
        ];
        let j = join(&la, None);
        let Curve::Poly(p) = &j[0].curve else { panic!() };
        assert!(
            !p.closed
                && p.nseg() == 2
                && (p.bulges[1] - forge_math::dmath::sin(PI / 8.0) / forge_math::dmath::cos(PI / 8.0)).abs()
                    < 1e-12
        );
    }
}
