//! Exact planar arrangements of line segments and circular arcs: where two pieces meet, whether a
//! point lies on a piece's span, and the order of points along a piece. Every decision is an
//! exact sign (`real::Ctx::sign`).
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
        Curve::Circle { c, rad, .. } => {
            let (x, y, r) = (cx.iv(c.x), cx.iv(c.y), cx.iv(rad));
            (
                (x.lo - r.hi).next_down(),
                (y.lo - r.hi).next_down(),
                (x.hi + r.hi).next_up(),
                (y.hi + r.hi).next_up(),
            )
        }
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
            // a dyadic direction near w, checked strictly inside (a, b)
            let mut prec = 53u64;
            loop {
                let (wx, wy) = (dy_near(cx, w.x, prec), dy_near(cx, w.y, prec));
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
                if ok {
                    let (nx, ny) = (cx.dy(wx), cx.dy(wy));
                    let nv = Pt { x: nx, y: ny };
                    let n2 = norm2(cx, nv);
                    let n = cx.sqrt(n2);
                    let k = cx.div(rad, n);
                    let off = scale(cx, nv, k);
                    return add(cx, c, off);
                }
                prec *= 2;
                assert!(prec < (1 << 20), "rep_point: empty sub-span");
            }
        }
    }
}

/// A dyadic within about 2^-prec (relative) of the node's value.
fn dy_near(cx: &mut Ctx, a: R, prec: u64) -> Dy {
    if prec <= 53 {
        return Dy::from_f64(cx.approx(a));
    }
    let v = cx.eval(a, prec);
    match (v.lo, v.hi) {
        (Some(l), Some(h)) => l.add(&h).half().round(prec, false),
        _ => Dy::from_f64(cx.approx(a)),
    }
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
