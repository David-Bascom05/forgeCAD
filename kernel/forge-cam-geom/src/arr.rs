//! Exact planar arrangements of line segments and circular arcs: where two pieces meet, whether a
//! point lies on a piece's span, and the order of points along a piece. Every decision is an
//! exact sign (`real::Ctx::sign`).
use crate::ival::Ival;
use crate::real::{Ctx, Dy, R};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pt {
    pub x: R,
    pub y: R,
}

/// The curve a piece lies on.
#[derive(Clone, Copy, Debug)]
pub enum Curve {
    /// p + t d, t in [0, 1] for the piece made from it.
    Line { p: Pt, d: Pt, dd: R },
    /// The circle about c of radius `rad` (rad2 = rad²), traversed counter-clockwise or not.
    Circle { c: Pt, rad: R, rad2: R, ccw: bool },
}

/// A piece of a curve from `s` to `e` (along the line, or around the circle in its sense;
/// `full`: the whole circle, s = e). `tag` names what made it.
#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub curve: Curve,
    pub s: Pt,
    pub e: Pt,
    pub full: bool,
    pub tag: u32,
}

pub fn pt(cx: &mut Ctx, x: f64, y: f64) -> Pt {
    Pt {
        x: cx.lit(x),
        y: cx.lit(y),
    }
}
pub fn sub(cx: &mut Ctx, a: Pt, b: Pt) -> Pt {
    Pt {
        x: cx.sub(a.x, b.x),
        y: cx.sub(a.y, b.y),
    }
}
pub fn add(cx: &mut Ctx, a: Pt, b: Pt) -> Pt {
    Pt {
        x: cx.add(a.x, b.x),
        y: cx.add(a.y, b.y),
    }
}
pub fn scale(cx: &mut Ctx, a: Pt, k: R) -> Pt {
    Pt {
        x: cx.mul(a.x, k),
        y: cx.mul(a.y, k),
    }
}
pub fn dot(cx: &mut Ctx, a: Pt, b: Pt) -> R {
    let (p, q) = (cx.mul(a.x, b.x), cx.mul(a.y, b.y));
    cx.add(p, q)
}
pub fn cross(cx: &mut Ctx, a: Pt, b: Pt) -> R {
    let (p, q) = (cx.mul(a.x, b.y), cx.mul(a.y, b.x));
    cx.sub(p, q)
}
/// The left normal (−y, x).
pub fn perp(cx: &mut Ctx, a: Pt) -> Pt {
    Pt {
        x: cx.neg(a.y),
        y: a.x,
    }
}
pub fn norm2(cx: &mut Ctx, a: Pt) -> R {
    dot(cx, a, a)
}

/// Exact equality of two points.
pub fn same(cx: &mut Ctx, a: Pt, b: Pt) -> bool {
    if a == b {
        return true;
    }
    // either coordinate's enclosures apart decides it at once (before an exact tie in the
    // other, which costs the most a sign can): the same answer, the conjunction's order aside
    let apart = |x: R, y: R, cx: &Ctx| {
        let (ix, iy) = (cx.iv(x), cx.iv(y));
        ix.hi < iy.lo || ix.lo > iy.hi
    };
    if apart(a.x, b.x, cx) || apart(a.y, b.y, cx) {
        return false;
    }
    cx.cmp(a.x, b.x) == Ordering::Equal && cx.cmp(a.y, b.y) == Ordering::Equal
}

fn sgn(cx: &mut Ctx, a: R) -> i32 {
    match cx.sign(a) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

/// The half-turn `v` lies in, measured from `u` in the given sense: 0 for [0, π), 1 for [π, 2π).
fn half(cx: &mut Ctx, u: Pt, v: Pt, ccw: bool) -> u8 {
    let m = cx.mark();
    let c = cross(cx, u, v);
    let mut s = sgn(cx, c);
    if !ccw {
        s = -s;
    }
    let h = if s > 0 {
        0
    } else if s < 0 {
        1
    } else {
        let d = dot(cx, u, v);
        if sgn(cx, d) > 0 { 0 } else { 1 }
    };
    cx.release(m);
    h
}

/// The order of directions v1, v2 by their angle from u in the given sense, in [0, 2π).
pub fn angle_cmp(cx: &mut Ctx, u: Pt, v1: Pt, v2: Pt, ccw: bool) -> Ordering {
    let (h1, h2) = (half(cx, u, v1, ccw), half(cx, u, v2, ccw));
    if h1 != h2 {
        return h1.cmp(&h2);
    }
    let m = cx.mark();
    let c = cross(cx, v1, v2);
    let mut s = sgn(cx, c);
    cx.release(m);
    if !ccw {
        s = -s;
    }
    // v1 before v2 when v2 is further round: cross(v1, v2) > 0 in the sense
    if s > 0 {
        Ordering::Less
    } else if s < 0 {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

/// Where a point known to be on the piece's curve sits along it, compared with another such point.
pub fn along_cmp(cx: &mut Ctx, pc: &Piece, a: Pt, b: Pt) -> Ordering {
    if a == b {
        return Ordering::Equal;
    }
    match pc.curve {
        Curve::Line { p, d, .. } => {
            let m = cx.mark();
            let (va, vb) = (sub(cx, a, p), sub(cx, b, p));
            let (ta, tb) = (dot(cx, va, d), dot(cx, vb, d));
            let o = cx.cmp(ta, tb);
            cx.release(m);
            o
        }
        Curve::Circle { c, ccw, .. } => {
            let m = cx.mark();
            let u = sub(cx, pc.s, c);
            let (va, vb) = (sub(cx, a, c), sub(cx, b, c));
            let o = angle_cmp(cx, u, va, vb, ccw);
            cx.release(m);
            o
        }
    }
}

/// Whether a point known to be on the piece's curve is within its span (ends included).
pub fn on_span(cx: &mut Ctx, pc: &Piece, x: Pt) -> bool {
    // one of its own ends (the same expression): within, with no sign to decide
    if x == pc.s || x == pc.e {
        return true;
    }
    match pc.curve {
        Curve::Line { p, d, dd } => {
            let m = cx.mark();
            let v = sub(cx, x, p);
            let t = dot(cx, v, d);
            let ok = sgn(cx, t) >= 0 && cx.cmp(t, dd) != Ordering::Greater;
            cx.release(m);
            ok
        }
        Curve::Circle { .. } => {
            if pc.full {
                return true;
            }
            along_cmp(cx, pc, x, pc.e) != Ordering::Greater
        }
    }
}

/// A point within this many f64 of an interval box: the cheap reject.
fn boxes_apart(cx: &Ctx, a: &Piece, b: &Piece) -> bool {
    let ba = piece_box(cx, a);
    let bb = piece_box(cx, b);
    ba.0 > bb.2 || bb.0 > ba.2 || ba.1 > bb.3 || bb.1 > ba.3
}

/// An f64 box (xmin, ymin, xmax, ymax) holding the piece.
pub fn piece_box(cx: &Ctx, pc: &Piece) -> (f64, f64, f64, f64) {
    match pc.curve {
        Curve::Line { .. } => {
            let (sx, sy, ex, ey) = (cx.iv(pc.s.x), cx.iv(pc.s.y), cx.iv(pc.e.x), cx.iv(pc.e.y));
            (
                sx.lo.min(ex.lo),
                sy.lo.min(ey.lo),
                sx.hi.max(ex.hi),
                sy.hi.max(ey.hi),
            )
        }
        Curve::Circle { c, rad, ccw, .. } => {
            let (x, y, r) = (cx.iv(c.x), cx.iv(c.y), cx.iv(rad));
            // the circle's box: west, south, east, north
            let circle = [
                (x.lo - r.hi).next_down(),
                (y.lo - r.hi).next_down(),
                (x.hi + r.hi).next_up(),
                (y.hi + r.hi).next_up(),
            ];
            if pc.full {
                return (circle[0], circle[1], circle[2], circle[3]);
            }
            // an arc: its ends' box, grown to each of the circle's extremes the span may hold
            // (the extent in a direction is an end's unless the extreme point is inside the
            // span); undecided in intervals, the extreme is taken, so the box only grows
            let (sx, sy, ex, ey) = (cx.iv(pc.s.x), cx.iv(pc.s.y), cx.iv(pc.e.x), cx.iv(pc.e.y));
            let mut b = [
                sx.lo.min(ex.lo),
                sy.lo.min(ey.lo),
                sx.hi.max(ex.hi),
                sy.hi.max(ey.hi),
            ];
            let us = (sx.sub(x), sy.sub(y));
            let ue = (ex.sub(x), ey.sub(y));
            // west (-1, 0), south (0, -1), east (1, 0), north (0, 1): which box side each grows
            let dirs: [((f64, f64), usize); 4] = [
                ((-1.0, 0.0), 0),
                ((0.0, -1.0), 1),
                ((1.0, 0.0), 2),
                ((0.0, 1.0), 3),
            ];
            for (d, k) in dirs {
                if span_may_hold(us, ue, d, ccw) {
                    b[k] = circle[k];
                }
            }
            (b[0], b[1], b[2], b[3])
        }
    }
}

/// Whether direction d (a unit axis vector) may lie strictly inside the arc's span from us to
/// ue (in the arc's sense): false only when interval signs prove it outside.
fn span_may_hold(us: (Ival, Ival), ue: (Ival, Ival), d: (f64, f64), ccw: bool) -> bool {
    let dv = (Ival::pt(d.0), Ival::pt(d.1));
    let sense = if ccw { 1 } else { -1 };
    let cross = |a: (Ival, Ival), b: (Ival, Ival)| a.0.mul(b.1).sub(a.1.mul(b.0)).sign() * sense;
    let dot = |a: (Ival, Ival), b: (Ival, Ival)| a.0.mul(b.0).add(a.1.mul(b.1)).sign();
    // the half-turn of v from us in the arc's sense: 0 for [0, π), 1 for [π, 2π); None undecided
    let half = |v: (Ival, Ival)| -> Option<u8> {
        match cross(us, v) {
            1 => Some(0),
            -1 => Some(1),
            _ => match dot(us, v) {
                // on us's line: along it (angle 0) or against it (π)
                1 => Some(0),
                -1 => Some(1),
                _ => None,
            },
        }
    };
    let (Some(hd), Some(he)) = (half(dv), half(ue)) else {
        return true;
    };
    // d outside: further round than ue (or at angle 0, at us, an end: covered by the ends' box)
    if hd != he {
        return hd < he;
    }
    match cross(dv, ue) {
        // ue further round than d: d inside
        1 => true,
        -1 => false,
        _ => true,
    }
}

/// The points where two pieces meet, pushed onto each one's list. Where they overlap (the same
/// line or circle), the ends of each that lie on the other are pushed.
pub fn meet(cx: &mut Ctx, a: &Piece, b: &Piece, on_a: &mut Vec<Pt>, on_b: &mut Vec<Pt>) {
    if boxes_apart(cx, a, b) {
        return;
    }
    let mut cand: Vec<Pt> = Vec::new();
    let mut overlap = false;
    // an end the two pieces share (the same expression, as neighbours in a loop have): it is one
    // of the meets, and the other follows rationally from it, with no root to take and no tie
    // to prove between the vertex and a recomputed copy of it
    let shared = [a.s, a.e]
        .into_iter()
        .find(|&x| (x == b.s || x == b.e) && !a.full && !b.full);
    let both = !a.full && !b.full && ((a.s == b.s && a.e == b.e) || (a.s == b.e && a.e == b.s));
    if let Some(v) = shared
        && let Some(other) = other_meet(cx, a, b, v)
    {
        cand.push(v);
        match other {
            // both ends shared: those are the two meets (distinct curves meet at most twice)
            Some(_) if both => cand.push(if v == a.s { a.e } else { a.s }),
            Some(o) => cand.push(o),
            None => {}
        }
        let mut i = 0;
        while i < cand.len() {
            let x = cand[i];
            if on_span(cx, a, x) && on_span(cx, b, x) {
                on_a.push(x);
                on_b.push(x);
            }
            i += 1;
        }
        return;
    }
    match (a.curve, b.curve) {
        (Curve::Line { p: p1, d: d1, .. }, Curve::Line { p: p2, d: d2, .. }) => {
            let den = cross(cx, d1, d2);
            let w = sub(cx, p2, p1);
            if sgn(cx, den) != 0 {
                let num = cross(cx, w, d2);
                let t = cx.div(num, den);
                let off = scale(cx, d1, t);
                cand.push(add(cx, p1, off));
            } else {
                let c = cross(cx, w, d1);
                if sgn(cx, c) == 0 {
                    overlap = true;
                }
            }
        }
        (Curve::Line { .. }, Curve::Circle { .. }) => line_circle(cx, a, b, &mut cand),
        (Curve::Circle { .. }, Curve::Line { .. }) => line_circle(cx, b, a, &mut cand),
        (Curve::Circle { c: c1, rad2: q1, .. }, Curve::Circle { c: c2, rad2: q2, .. }) => {
            let dv = sub(cx, c2, c1);
            let dd = norm2(cx, dv);
            if sgn(cx, dd) == 0 {
                if cx.cmp(q1, q2) == Ordering::Equal {
                    overlap = true;
                }
            } else {
                // c1 + k dv ± h perp(dv): k = (D + r1² − r2²) / 2D, h² = (r1² − k² D) / D
                let s1 = cx.add(dd, q1);
                let num = cx.sub(s1, q2);
                let two = cx.lit(2.0);
                let den = cx.mul(two, dd);
                let k = cx.div(num, den);
                let k2 = cx.sqr(k);
                let k2d = cx.mul(k2, dd);
                let hn = cx.sub(q1, k2d);
                let h2 = cx.div(hn, dd);
                let s = sgn(cx, h2);
                if s >= 0 {
                    let kd = scale(cx, dv, k);
                    let base = add(cx, c1, kd);
                    if s == 0 {
                        cand.push(base);
                    } else {
                        let h = cx.sqrt(h2);
                        let pv = perp(cx, dv);
                        let off = scale(cx, pv, h);
                        cand.push(add(cx, base, off));
                        cand.push(sub(cx, base, off));
                    }
                }
            }
        }
    }
    let mut i = 0;
    while i < cand.len() {
        let x = cand[i];
        if on_span(cx, a, x) && on_span(cx, b, x) {
            on_a.push(x);
            on_b.push(x);
        }
        i += 1;
    }
    if overlap {
        let ends_b = [b.s, b.e];
        let ends_a = [a.s, a.e];
        let mut k = 0;
        while k < 2 {
            if on_span(cx, a, ends_b[k]) {
                on_a.push(ends_b[k]);
            }
            if on_span(cx, b, ends_a[k]) {
                on_b.push(ends_a[k]);
            }
            k += 1;
        }
    }
}

/// Given a point v on both curves, the other meet: Some(None) for two lines (they meet once),
/// Some(Some(w)) for a line and a circle (Vieta: t_v + t_w = −2 b / dd) or two circles (v
/// mirrored in the line of centres), w = v when they touch there. None when the curves
/// coincide (the caller then treats the overlap in full).
fn other_meet(cx: &mut Ctx, a: &Piece, b: &Piece, v: Pt) -> Option<Option<Pt>> {
    match (a.curve, b.curve) {
        (Curve::Line { d: d1, .. }, Curve::Line { d: d2, .. }) => {
            let den = cross(cx, d1, d2);
            if sgn(cx, den) == 0 { None } else { Some(None) }
        }
        (Curve::Line { .. }, Curve::Circle { .. }) => Some(Some(vieta(cx, a, b, v))),
        (Curve::Circle { .. }, Curve::Line { .. }) => Some(Some(vieta(cx, b, a, v))),
        (Curve::Circle { c: c1, .. }, Curve::Circle { c: c2, .. }) => {
            let dv = sub(cx, c2, c1);
            let dd = norm2(cx, dv);
            if sgn(cx, dd) == 0 {
                // concentric through a common point: the same circle
                return None;
            }
            // w = 2 (c1 + k dv) − v, k = ((v − c1)·dv) / dd
            let w = sub(cx, v, c1);
            let num = dot(cx, w, dv);
            let k = cx.div(num, dd);
            let kd = scale(cx, dv, k);
            let foot = add(cx, c1, kd);
            let two = cx.lit(2.0);
            let f2 = scale(cx, foot, two);
            Some(Some(sub(cx, f2, v)))
        }
    }
}

/// The line's other meet with the circle, given one at v.
fn vieta(cx: &mut Ctx, l: &Piece, c: &Piece, v: Pt) -> Pt {
    let (p, d, dd) = match l.curve {
        Curve::Line { p, d, dd } => (p, d, dd),
        _ => unreachable!(),
    };
    let cc = match c.curve {
        Curve::Circle { c, .. } => c,
        _ => unreachable!(),
    };
    // t_v + t_w = −2 (p − c)·d / dd, t_v = (v − p)·d / dd
    let f = sub(cx, p, cc);
    let b = dot(cx, f, d);
    let vp = sub(cx, v, p);
    let tv = dot(cx, vp, d);
    let two = cx.lit(2.0);
    let b2 = cx.mul(two, b);
    let s = cx.add(b2, tv);
    let ns = cx.neg(s);
    let t = cx.div(ns, dd);
    let off = scale(cx, d, t);
    add(cx, p, off)
}

fn line_circle(cx: &mut Ctx, l: &Piece, c: &Piece, out: &mut Vec<Pt>) {
    let (p, d, dd) = match l.curve {
        Curve::Line { p, d, dd } => (p, d, dd),
        _ => unreachable!(),
    };
    let (cc, q) = match c.curve {
        Curve::Circle { c, rad2, .. } => (c, rad2),
        _ => unreachable!(),
    };
    // |p + t d − c|² = q: dd t² + 2 b t + (f·f − q) = 0 with f = p − c, b = f·d
    let f = sub(cx, p, cc);
    let b = dot(cx, f, d);
    let ff = norm2(cx, f);
    let c0 = cx.sub(ff, q);
    let b2 = cx.sqr(b);
    let ac = cx.mul(dd, c0);
    let disc = cx.sub(b2, ac);
    let s = sgn(cx, disc);
    if s < 0 {
        return;
    }
    let nb = cx.neg(b);
    if s == 0 {
        let t = cx.div(nb, dd);
        let off = scale(cx, d, t);
        out.push(add(cx, p, off));
        return;
    }
    let r = cx.sqrt(disc);
    let n1 = cx.add(nb, r);
    let n2 = cx.sub(nb, r);
    let t1 = cx.div(n1, dd);
    let t2 = cx.div(n2, dd);
    let o1 = scale(cx, d, t1);
    let o2 = scale(cx, d, t2);
    out.push(add(cx, p, o1));
    out.push(add(cx, p, o2));
}

/// Sorts points on a full circle into their order round it starting from `pts[0]`, dropping
/// repeats: the order `sort_along` gives from pts[0], found by the angle from a fixed direction
/// off every axis (the cyclic order is the same from any reference), then turned to start at
/// pts[0]'s place. From pts[0] itself, a point opposite it is at a cross product exactly 0 (a
/// tie: the costliest sign); from the fixed direction two points tie only if one lies on its
/// line, and opposite points fall in opposite half turns, decided at once.
pub fn sort_cyclic(cx: &mut Ctx, pc: &Piece, pts: &mut Vec<Pt>) {
    let (c, ccw) = match pc.curve {
        Curve::Circle { c, ccw, .. } => (c, ccw),
        Curve::Line { .. } => unreachable!("sort_cyclic takes a circle"),
    };
    if pts.is_empty() {
        return;
    }
    let p0 = pts[0];
    let m = cx.mark();
    // 1 and 0.4142135623730951 (≈ tan 22.5°, an f64): no axis, no diagonal
    let g = Pt {
        x: cx.lit(1.0),
        y: cx.lit(0.4142135623730951),
    };
    let mut i = 1;
    while i < pts.len() {
        let mut j = i;
        while j > 0 && {
            let (a, b) = (sub(cx, pts[j], c), sub(cx, pts[j - 1], c));
            angle_cmp(cx, g, a, b, ccw) == Ordering::Less
        } {
            pts.swap(j, j - 1);
            j -= 1;
        }
        i += 1;
    }
    let mut out: Vec<Pt> = Vec::with_capacity(pts.len());
    let mut k = 0;
    while k < pts.len() {
        let keep = match out.last() {
            None => true,
            Some(&l) => {
                l != pts[k] && {
                    let (a, b) = (sub(cx, l, c), sub(cx, pts[k], c));
                    angle_cmp(cx, g, a, b, ccw) != Ordering::Equal
                }
            }
        };
        if keep {
            out.push(pts[k]);
        }
        k += 1;
    }
    // turned to start where p0 is (the first kept point at its place)
    let mut start = 0;
    while start < out.len() {
        let q = out[start];
        if q == p0 || {
            let (a, b) = (sub(cx, q, c), sub(cx, p0, c));
            angle_cmp(cx, g, a, b, ccw) == Ordering::Equal
        } {
            break;
        }
        start += 1;
    }
    cx.release(m);
    let n = out.len().max(1);
    out.rotate_left(start % n);
    *pts = out;
}

/// Sorts points on a piece along it (insertion sort, exact comparisons) and drops repeats.
pub fn sort_along(cx: &mut Ctx, pc: &Piece, pts: &mut Vec<Pt>) {
    let mut i = 1;
    while i < pts.len() {
        let mut j = i;
        while j > 0 && along_cmp(cx, pc, pts[j], pts[j - 1]) == Ordering::Less {
            pts.swap(j, j - 1);
            j -= 1;
        }
        i += 1;
    }
    let mut out: Vec<Pt> = Vec::with_capacity(pts.len());
    let mut k = 0;
    while k < pts.len() {
        let keep = match out.last() {
            None => true,
            Some(&l) => along_cmp(cx, pc, l, pts[k]) != Ordering::Equal,
        };
        if keep {
            out.push(pts[k]);
        }
        k += 1;
    }
    *pts = out;
}

/// A point strictly inside the piece's span between a and b (a before b along it; for a circle
/// a = b means the whole circle from a).
pub fn rep_point(cx: &mut Ctx, pc: &Piece, a: Pt, b: Pt) -> Pt {
    match pc.curve {
        Curve::Line { p, d, dd } => {
            let (va, vb) = (sub(cx, a, p), sub(cx, b, p));
            let (ta, tb) = (dot(cx, va, d), dot(cx, vb, d));
            let m = cx.between(ta, tb);
            let mn = cx.dy(m);
            let t = cx.div(mn, dd);
            let off = scale(cx, d, t);
            add(cx, p, off)
        }
        Curve::Circle { c, rad, ccw, .. } => {
            let (ua, ub) = (sub(cx, a, c), sub(cx, b, c));
            // the bisecting direction of the sub-span (in the piece's sense)
            let cr = cross(cx, ua, ub);
            let mut s = sgn(cx, cr);
            if !ccw {
                s = -s;
            }
            let w = if same(cx, a, b) {
                Pt {
                    x: cx.neg(ua.x),
                    y: cx.neg(ua.y),
                }
            } else if s > 0 {
                add(cx, ua, ub)
            } else if s < 0 {
                let w0 = add(cx, ua, ub);
                Pt {
                    x: cx.neg(w0.x),
                    y: cx.neg(w0.y),
                }
            } else {
                // exactly a half turn: the perpendicular in the sense of travel
                let pp = perp(cx, ua);
                if ccw {
                    pp
                } else {
                    Pt {
                        x: cx.neg(pp.x),
                        y: cx.neg(pp.y),
                    }
                }
            };
            // a dyadic direction near w (from the f64 enclosures, a low-degree point), checked
            // strictly inside (a, b)
            let (wx, wy) = (dy_near(cx, w.x), dy_near(cx, w.y));
            let mk = cx.mark();
            let (nx, ny) = (cx.dy(wx.clone()), cx.dy(wy.clone()));
            let nv = Pt { x: nx, y: ny };
            let n2 = norm2(cx, nv);
            let ok = sgn(cx, n2) > 0 && {
                let after_a = angle_cmp(cx, ua, ua, nv, ccw) == Ordering::Less;
                let before_b = same(cx, a, b) || angle_cmp(cx, ua, nv, ub, ccw) == Ordering::Less;
                after_a && before_b
            };
            cx.release(mk);
            // else w itself: strictly inside by construction (the bisector of a sub-span short
            // of a half turn, its opposite for one beyond, the perpendicular in the sense of
            // travel for a half turn, the opposite of a for the whole circle; w ≠ 0 in each),
            // where a tiny sub-span (a near-tangent corner's arc) would take a dyadic of
            // ever more bits
            let nv = if ok {
                let (nx, ny) = (cx.dy(wx), cx.dy(wy));
                Pt { x: nx, y: ny }
            } else {
                w
            };
            let n2 = norm2(cx, nv);
            let n = cx.sqrt(n2);
            let k = cx.div(rad, n);
            let off = scale(cx, nv, k);
            add(cx, c, off)
        }
    }
}

/// A dyadic near the node's value: its f64 enclosure's midpoint (no big-float evaluation; the
/// caller checks what it makes), or the nearest f64 if the enclosure is unbounded.
fn dy_near(cx: &mut Ctx, a: R) -> Dy {
    let iv = cx.iv(a);
    if iv.lo.is_finite() && iv.hi.is_finite() {
        return Dy::from_f64(iv.mid());
    }
    Dy::from_f64(cx.approx(a))
}

/// The unit tangent's direction (not normalised) at a point of the piece's curve.
pub fn tangent(cx: &mut Ctx, pc: &Piece, x: Pt) -> Pt {
    match pc.curve {
        Curve::Line { d, .. } => d,
        Curve::Circle { c, ccw, .. } => {
            let v = sub(cx, x, c);
            let p = perp(cx, v);
            if ccw {
                p
            } else {
                Pt {
                    x: cx.neg(p.x),
                    y: cx.neg(p.y),
                }
            }
        }
    }
}

#[cfg(test)]
mod cyclic_tests {
    use super::*;

    /// sort_cyclic gives sort_along's order from pts[0] round a full circle (each start, each
    /// sense), with opposite points, repeats (equal points of other expressions) and points on
    /// the axes: points on the circle of radius 5 at the Pythagorean (±3, ±4), (±4, ±3), (±5, 0),
    /// (0, ±5).
    #[test]
    fn the_cyclic_order_is_sort_alongs() {
        let mut cx = Ctx::new();
        let xy = [
            (3.0, 4.0),
            (-3.0, -4.0),
            (4.0, -3.0),
            (5.0, 0.0),
            (-5.0, 0.0),
            (0.0, 5.0),
            (-4.0, 3.0),
            (0.0, -5.0),
            (-3.0, 4.0),
            (3.0, -4.0),
        ];
        for ccw in [true, false] {
            for first in 0..xy.len() {
                let mut pts: Vec<Pt> = vec![];
                for k in 0..xy.len() {
                    let (x, y) = xy[(first + k) % xy.len()];
                    pts.push(pt(&mut cx, x, y));
                }
                // a repeat of the third point, as another expression: ((3x)/3, (3y)/3)
                let (x, y) = xy[(first + 2) % xy.len()];
                let three = cx.lit(3.0);
                let (tx, ty) = (cx.lit(3.0 * x), cx.lit(3.0 * y));
                let rx = cx.div(tx, three);
                let ry = cx.div(ty, three);
                assert!(rx != pts[2].x || x == 0.0);
                pts.push(Pt { x: rx, y: ry });
                let c = pt(&mut cx, 0.0, 0.0);
                let rad = cx.lit(5.0);
                let rad2 = cx.lit(25.0);
                let pc = Piece {
                    curve: Curve::Circle { c, rad, rad2, ccw },
                    s: pts[0],
                    e: pts[0],
                    full: true,
                    tag: 0,
                };
                let mut a = pts.clone();
                sort_along(&mut cx, &pc, &mut a);
                let mut b = pts.clone();
                sort_cyclic(&mut cx, &pc, &mut b);
                assert_eq!(a.len(), b.len());
                for (p, q) in a.iter().zip(&b) {
                    assert!(same(&mut cx, *p, *q), "ccw {ccw} first {first}");
                }
            }
        }
    }
}
