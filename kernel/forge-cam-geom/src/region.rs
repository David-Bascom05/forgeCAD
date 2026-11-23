//! Regions bounded by closed loops of lines and circular arcs, made exact and validated.
//!
//! Input numbers are f64, exact. A line is its two ends. An arc is its two ends, a centre *hint*
//! and its sense: the exact centre is the point on the ends' bisector nearest the hint (a
//! rational point), so the arc passes exactly through both ends and consecutive pieces share
//! their ends exactly. An arc whose ends are the same point is a full circle about the hint.
//!
//! A region is valid when: every loop closes (consecutive ends bit-equal); no piece has zero
//! length; pieces meet only at the ends they share; no vertex turns back on itself (a spike);
//! and each loop's sense matches its nesting: counter-clockwise at even depth, clockwise at odd.
//! The region is then the set on the left of every piece.
use crate::arr::{self, Curve, Piece, Pt};
use crate::real::{Ctx, R};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct P2 {
    pub x: f64,
    pub y: f64,
}

pub fn p2(x: f64, y: f64) -> P2 {
    P2 { x, y }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    Line {
        a: P2,
        b: P2,
    },
    /// From a to b about (near) c; a == b: the full circle.
    Arc {
        a: P2,
        b: P2,
        c: P2,
        ccw: bool,
    },
}

impl Seg {
    pub fn start(&self) -> P2 {
        match *self {
            Seg::Line { a, .. } | Seg::Arc { a, .. } => a,
        }
    }
    pub fn end(&self) -> P2 {
        match *self {
            Seg::Line { b, .. } | Seg::Arc { b, .. } => b,
        }
    }
    pub fn reversed(&self) -> Seg {
        match *self {
            Seg::Line { a, b } => Seg::Line { a: b, b: a },
            Seg::Arc { a, b, c, ccw } => Seg::Arc {
                a: b,
                b: a,
                c,
                ccw: !ccw,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Loop {
    pub segs: Vec<Seg>,
}

impl Loop {
    pub fn reversed(&self) -> Loop {
        let mut v = Vec::with_capacity(self.segs.len());
        let mut i = self.segs.len();
        while i > 0 {
            i -= 1;
            v.push(self.segs[i].reversed());
        }
        Loop { segs: v }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Region {
    pub loops: Vec<Loop>,
}

impl Region {
    pub fn reversed(&self) -> Region {
        let mut v = Vec::with_capacity(self.loops.len());
        let mut i = 0;
        while i < self.loops.len() {
            v.push(self.loops[i].reversed());
            i += 1;
        }
        Region { loops: v }
    }
}

/// Why a region was refused, and where.
#[derive(Clone, Debug, PartialEq)]
pub enum RegionError {
    Empty,
    /// Loop `lp`, piece `seg`: its end is not the next piece's start.
    NotClosed {
        lp: usize,
        seg: usize,
    },
    /// A piece of zero length (or an arc of zero radius).
    Degenerate {
        lp: usize,
        seg: usize,
    },
    /// Two pieces meet away from a shared end.
    Crossing {
        a: (usize, usize),
        b: (usize, usize),
        at: P2,
    },
    /// A vertex where the boundary turns straight back.
    Spike {
        lp: usize,
        seg: usize,
    },
    /// A loop whose sense does not match its nesting depth.
    Sense {
        lp: usize,
        ccw: bool,
        depth: usize,
    },
}

/// One boundary piece, exact.
#[derive(Clone, Copy, Debug)]
pub struct Elem {
    pub piece: Piece,
    pub lp: usize,
    pub seg: usize,
    /// For an arc: the radius ρ and its sense.
    pub rho: Option<R>,
    pub ccw: bool,
}

/// The exact boundary of a region: its pieces, loop by loop.
pub struct Exact {
    pub elems: Vec<Elem>,
    /// For each loop, the range of its elements.
    pub loops: Vec<(usize, usize)>,
}

/// Makes a region's pieces exact (no validation).
pub fn exact(cx: &mut Ctx, rg: &Region) -> Result<Exact, RegionError> {
    let mut elems = Vec::new();
    let mut loops = Vec::new();
    let mut l = 0;
    while l < rg.loops.len() {
        let lp = &rg.loops[l];
        let first = elems.len();
        let mut i = 0;
        while i < lp.segs.len() {
            let sg = lp.segs[i];
            let a = arr::pt(cx, sg.start().x, sg.start().y);
            let b = arr::pt(cx, sg.end().x, sg.end().y);
            match sg {
                Seg::Line { .. } => {
                    let d = arr::sub(cx, b, a);
                    let dd = arr::norm2(cx, d);
                    if cx.sign(dd) != Ordering::Greater {
                        return Err(RegionError::Degenerate { lp: l, seg: i });
                    }
                    elems.push(Elem {
                        piece: Piece {
                            curve: Curve::Line { p: a, d, dd },
                            s: a,
                            e: b,
                            full: false,
                            tag: elems.len() as u32,
                        },
                        lp: l,
                        seg: i,
                        rho: None,
                        ccw: true,
                    });
                }
                Seg::Arc { c, ccw, .. } => {
                    let h = arr::pt(cx, c.x, c.y);
                    let full = sg.start() == sg.end();
                    let cen = if full {
                        h
                    } else {
                        // m + t perp(b − a), t = ((h − m)·perp) / |perp|²
                        let ab = arr::add(cx, a, b);
                        let half = cx.lit(0.5);
                        let m = arr::scale(cx, ab, half);
                        let d = arr::sub(cx, b, a);
                        let dd = arr::norm2(cx, d);
                        if cx.sign(dd) != Ordering::Greater {
                            return Err(RegionError::Degenerate { lp: l, seg: i });
                        }
                        let pp = arr::perp(cx, d);
                        let hm = arr::sub(cx, h, m);
                        let num = arr::dot(cx, hm, pp);
                        let t = cx.div(num, dd);
                        let off = arr::scale(cx, pp, t);
                        arr::add(cx, m, off)
                    };
                    let ra = arr::sub(cx, a, cen);
                    let q = arr::norm2(cx, ra);
                    if cx.sign(q) != Ordering::Greater {
                        return Err(RegionError::Degenerate { lp: l, seg: i });
                    }
                    let rho = cx.sqrt(q);
                    elems.push(Elem {
                        piece: Piece {
                            curve: Curve::Circle {
                                c: cen,
                                rad: rho,
                                rad2: q,
                                ccw,
                            },
                            s: a,
                            e: b,
                            full,
                            tag: elems.len() as u32,
                        },
                        lp: l,
                        seg: i,
                        rho: Some(rho),
                        ccw,
                    });
                }
            }
            i += 1;
        }
        loops.push((first, elems.len()));
        l += 1;
    }
    Ok(Exact { elems, loops })
}

/// The tangent at the start (`at_end` false) or end of an element, in its direction of travel.
pub fn elem_tangent(cx: &mut Ctx, e: &Elem, at_end: bool) -> Pt {
    let x = if at_end { e.piece.e } else { e.piece.s };
    arr::tangent(cx, &e.piece, x)
}

/// Checks that a region is valid (module doc). Returns its exact form.
pub fn validate(cx: &mut Ctx, rg: &Region) -> Result<Exact, RegionError> {
    if rg.loops.is_empty() {
        return Err(RegionError::Empty);
    }
    let mut l = 0;
    while l < rg.loops.len() {
        let s = &rg.loops[l].segs;
        if s.is_empty() {
            return Err(RegionError::Empty);
        }
        let mut i = 0;
        while i < s.len() {
            let next = s[(i + 1) % s.len()];
            if s[i].end() != next.start() {
                return Err(RegionError::NotClosed { lp: l, seg: i });
            }
            if let Seg::Arc { a, b, .. } = s[i]
                && a == b
                && s.len() != 1
            {
                return Err(RegionError::Degenerate { lp: l, seg: i });
            }
            i += 1;
        }
        l += 1;
    }
    let ex = exact(cx, rg)?;
    // pieces meet only at shared ends
    let n = ex.elems.len();
    let mut i = 0;
    while i < n {
        let mut j = i + 1;
        while j < n {
            let (ei, ej) = (ex.elems[i], ex.elems[j]);
            let mut on_i = Vec::new();
            let mut on_j = Vec::new();
            arr::meet(cx, &ei.piece, &ej.piece, &mut on_i, &mut on_j);
            let mut k = 0;
            while k < on_i.len() {
                let x = on_i[k];
                if !shared_end(cx, &ex, i, j, x) {
                    return Err(RegionError::Crossing {
                        a: (ei.lp, ei.seg),
                        b: (ej.lp, ej.seg),
                        at: p2(cx.approx(x.x), cx.approx(x.y)),
                    });
                }
                k += 1;
            }
            j += 1;
        }
        i += 1;
    }
    // no spikes
    let mut l = 0;
    while l < ex.loops.len() {
        let (a, b) = ex.loops[l];
        let mut i = a;
        while i < b {
            let nx = if i + 1 == b { a } else { i + 1 };
            if nx != i {
                let m = cx.mark();
                let tin = elem_tangent(cx, &ex.elems[i], true);
                let tout = elem_tangent(cx, &ex.elems[nx], false);
                let c = arr::cross(cx, tin, tout);
                let d = arr::dot(cx, tin, tout);
                let spike = cx.sign(c) == Ordering::Equal && cx.sign(d) == Ordering::Less;
                cx.release(m);
                if spike {
                    return Err(RegionError::Spike { lp: l, seg: i - a });
                }
            }
            i += 1;
        }
        l += 1;
    }
    // sense against depth
    let mut l = 0;
    while l < ex.loops.len() {
        let ccw = loop_ccw(cx, &ex, l);
        let mut depth = 0;
        let mut k = 0;
        while k < ex.loops.len() {
            if k != l {
                let q = ex.elems[ex.loops[l].0].piece.s;
                if inside_loop(cx, &ex, k, q) {
                    depth += 1;
                }
            }
            k += 1;
        }
        if ccw != (depth % 2 == 0) {
            return Err(RegionError::Sense { lp: l, ccw, depth });
        }
        l += 1;
    }
    Ok(ex)
}

/// Whether x is an end that elements i and j share as neighbours in a loop.
fn shared_end(cx: &mut Ctx, ex: &Exact, i: usize, j: usize, x: Pt) -> bool {
    let (ei, ej) = (&ex.elems[i], &ex.elems[j]);
    if ei.lp != ej.lp {
        return false;
    }
    let (a, b) = ex.loops[ei.lp];
    let next = |k: usize| if k + 1 == b { a } else { k + 1 };
    // i's end is j's start, or j's end is i's start
    (next(i) == j && arr::same(cx, x, ei.piece.e)) || (next(j) == i && arr::same(cx, x, ej.piece.e))
}

/// The sense of loop l, read at its lowest-leftmost point.
pub fn loop_ccw(cx: &mut Ctx, ex: &Exact, l: usize) -> bool {
    let (a, b) = ex.loops[l];
    if b - a == 1 && ex.elems[a].piece.full {
        return ex.elems[a].ccw;
    }
    // candidates: each element's start (a vertex), and each arc's leftmost circle point if inside
    // its span (strictly)
    let mut best: Option<(Pt, usize, bool)> = None; // point, element, is a vertex (start of elem)
    let mut i = a;
    while i < b {
        let e = ex.elems[i];
        let cands: [(Option<Pt>, bool); 2] = [
            (Some(e.piece.s), true),
            (
                match (e.piece.curve, e.rho) {
                    (Curve::Circle { c, .. }, Some(rho)) => {
                        let x = cx.sub(c.x, rho);
                        let lpnt = Pt { x, y: c.y };
                        if arr::on_span(cx, &e.piece, lpnt)
                            && !arr::same(cx, lpnt, e.piece.s)
                            && !arr::same(cx, lpnt, e.piece.e)
                        {
                            Some(lpnt)
                        } else {
                            None
                        }
                    }
                    _ => None,
                },
                false,
            ),
        ];
        let mut k = 0;
        while k < 2 {
            if let (Some(p), vert) = cands[k] {
                let better = match best {
                    None => true,
                    Some((q, _, _)) => {
                        let ox = cx.cmp(p.x, q.x);
                        ox == Ordering::Less || (ox == Ordering::Equal && cx.cmp(p.y, q.y) == Ordering::Less)
                    }
                };
                if better {
                    best = Some((p, i, vert));
                }
            }
            k += 1;
        }
        i += 1;
    }
    let (_, i, vert) = best.unwrap();
    if !vert {
        return ex.elems[i].ccw;
    }
    let prev = if i == a { b - 1 } else { i - 1 };
    let m = cx.mark();
    let tin = elem_tangent(cx, &ex.elems[prev], true);
    let tout = elem_tangent(cx, &ex.elems[i], false);
    let c = arr::cross(cx, tin, tout);
    let s = cx.sign(c);
    cx.release(m);
    match s {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => {
            if ex.elems[i].rho.is_some() {
                ex.elems[i].ccw
            } else {
                ex.elems[prev].ccw
            }
        }
    }
}

/// Whether q (not on loop l) is inside loop l, by a ray to +x with the half-open rule.
pub fn inside_loop(cx: &mut Ctx, ex: &Exact, l: usize, q: Pt) -> bool {
    let (a, b) = ex.loops[l];
    let mut odd = false;
    let mut i = a;
    while i < b {
        let e = ex.elems[i];
        let m = cx.mark();
        match e.piece.curve {
            Curve::Line { .. } => {
                if crosses_line(cx, e.piece.s, e.piece.e, q) {
                    odd = !odd;
                }
            }
            Curve::Circle { c, rad, ccw, .. } => {
                // the y-monotone parts, split at the circle's top and bottom
                let top = Pt {
                    x: c.x,
                    y: cx.add(c.y, rad),
                };
                let bot = Pt {
                    x: c.x,
                    y: cx.sub(c.y, rad),
                };
                let mut cuts: Vec<Pt> = Vec::new();
                let mut k = 0;
                let tb = [top, bot];
                while k < 2 {
                    if arr::on_span(cx, &e.piece, tb[k]) && !arr::same(cx, tb[k], e.piece.s) {
                        cuts.push(tb[k]);
                    }
                    k += 1;
                }
                arr::sort_along(cx, &e.piece, &mut cuts);
                let mut pts = vec![e.piece.s];
                let mut k = 0;
                while k < cuts.len() {
                    pts.push(cuts[k]);
                    k += 1;
                }
                pts.push(e.piece.e);
                let mut k = 0;
                while k + 1 < pts.len() {
                    if crosses_arc_part(cx, c, rad, ccw, pts[k], pts[k + 1], q) {
                        odd = !odd;
                    }
                    k += 1;
                }
            }
        }
        cx.release(m);
        i += 1;
    }
    odd
}

fn crosses_line(cx: &mut Ctx, a: Pt, b: Pt, q: Pt) -> bool {
    let ua = cx.cmp(a.y, q.y) == Ordering::Greater;
    let ub = cx.cmp(b.y, q.y) == Ordering::Greater;
    if ua == ub {
        return false;
    }
    let d = arr::sub(cx, b, a);
    let w = arr::sub(cx, q, a);
    let c = arr::cross(cx, d, w);
    let s = cx.sign(c);
    // upward edge: the crossing is right of q iff q is left of it
    if ub {
        s == Ordering::Greater
    } else {
        s == Ordering::Less
    }
}

fn crosses_arc_part(cx: &mut Ctx, c: Pt, rad: R, ccw: bool, u: Pt, w: Pt, q: Pt) -> bool {
    let uu = cx.cmp(u.y, q.y) == Ordering::Greater;
    let uw = cx.cmp(w.y, q.y) == Ordering::Greater;
    if uu == uw {
        return false;
    }
    // which half of the circle the part is on
    let su = cx.cmp(u.x, c.x);
    let sw = cx.cmp(w.x, c.x);
    let right = if su != Ordering::Equal {
        su == Ordering::Greater
    } else if sw != Ordering::Equal {
        sw == Ordering::Greater
    } else {
        // top to bottom: counter-clockwise goes round the left
        let down = cx.cmp(w.y, u.y) == Ordering::Less;
        down != ccw
    };
    // the crossing's x: c.x ± sqrt(rad² − (q.y − c.y)²)
    let dy = cx.sub(q.y, c.y);
    let dy2 = cx.sqr(dy);
    let r2 = cx.sqr(rad);
    let h2 = cx.sub(r2, dy2);
    let h = cx.sqrt(h2);
    let x = if right { cx.add(c.x, h) } else { cx.sub(c.x, h) };
    cx.cmp(x, q.x) == Ordering::Greater
}

#[cfg(test)]
pub fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Loop {
    let (a, b, c, d) = (p2(x0, y0), p2(x1, y0), p2(x1, y1), p2(x0, y1));
    Loop {
        segs: vec![
            Seg::Line { a, b },
            Seg::Line { a: b, b: c },
            Seg::Line { a: c, b: d },
            Seg::Line { a: d, b: a },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates() {
        let mut cx = Ctx::new();
        let sq = Region {
            loops: vec![rect(0.0, 0.0, 10.0, 10.0)],
        };
        assert!(validate(&mut cx, &sq).is_ok());
        // reversed alone: wrong sense
        assert!(matches!(
            validate(&mut cx, &sq.reversed()),
            Err(RegionError::Sense { .. })
        ));
        // with a clockwise island
        let hole = Loop {
            segs: vec![Seg::Arc {
                a: p2(6.0, 5.0),
                b: p2(6.0, 5.0),
                c: p2(5.0, 5.0),
                ccw: false,
            }],
        };
        let r2 = Region {
            loops: vec![rect(0.0, 0.0, 10.0, 10.0), hole.clone()],
        };
        assert!(validate(&mut cx, &r2).is_ok());
        // an island crossing the outside
        let bad = Loop {
            segs: vec![Seg::Arc {
                a: p2(11.0, 5.0),
                b: p2(11.0, 5.0),
                c: p2(9.0, 5.0),
                ccw: false,
            }],
        };
        assert!(matches!(
            validate(
                &mut cx,
                &Region {
                    loops: vec![rect(0.0, 0.0, 10.0, 10.0), bad]
                }
            ),
            Err(RegionError::Crossing { .. })
        ));
        // a rounded slot: lines and half circles, tangent at every joint
        let slot = Loop {
            segs: vec![
                Seg::Line {
                    a: p2(0.0, 0.0),
                    b: p2(10.0, 0.0),
                },
                Seg::Arc {
                    a: p2(10.0, 0.0),
                    b: p2(10.0, 4.0),
                    c: p2(10.0, 2.0),
                    ccw: true,
                },
                Seg::Line {
                    a: p2(10.0, 4.0),
                    b: p2(0.0, 4.0),
                },
                Seg::Arc {
                    a: p2(0.0, 4.0),
                    b: p2(0.0, 0.0),
                    c: p2(0.0, 2.0),
                    ccw: true,
                },
            ],
        };
        assert!(
            validate(
                &mut cx,
                &Region {
                    loops: vec![slot.clone()]
                }
            )
            .is_ok()
        );
        assert!(matches!(
            validate(
                &mut cx,
                &Region {
                    loops: vec![slot.reversed()]
                }
            ),
            Err(RegionError::Sense { .. })
        ));
    }
}
