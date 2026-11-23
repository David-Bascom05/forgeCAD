//! Exact booleans of line/arc regions: union, difference, intersection.
//!
//! The arrangement of the two boundaries (offset.rs's machinery): every meet found exactly, every
//! piece split there, and each split piece kept or dropped by where a rational point inside it
//! lies against the other region: inside, outside, or on its boundary (then by whether the two
//! boundaries run the same way there). The kept pieces link into loops with the result on their
//! left. One rule per case, no tolerance:
//!
//! | op | a piece of A is kept when it is | a piece of B is kept when it is |
//! |---|---|---|
//! | A ∪ B | outside B, or on ∂B the same way | outside A |
//! | A ∩ B | inside B, or on ∂B the same way | inside A |
//! | A − B | outside B, or on ∂B the opposite way | inside A, reversed |
use crate::arr::{self, Curve, Piece, Pt};
use crate::offset::{OLoop, OffsetError};
use crate::real::Ctx;
use crate::region::{self, Exact, Region, RegionError};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Union,
    Inter,
    Diff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Where {
    In,
    Out,
    Same,
    Opposite,
}

fn on_curve(cx: &mut Ctx, pc: &Piece, q: Pt) -> bool {
    let m = cx.mark();
    let on = match pc.curve {
        Curve::Line { p, d, .. } => {
            let v = arr::sub(cx, q, p);
            let c = arr::cross(cx, d, v);
            cx.sign(c) == Ordering::Equal
        }
        Curve::Circle { c, rad2, .. } => {
            let v = arr::sub(cx, q, c);
            let n = arr::norm2(cx, v);
            cx.cmp(n, rad2) == Ordering::Equal
        }
    };
    let r = on && arr::on_span(cx, pc, q);
    cx.release(m);
    r
}

/// Where q (a point inside the piece `pc`) is against the region `ex`.
fn locate(cx: &mut Ctx, ex: &Exact, q: Pt, pc: &Piece) -> Where {
    let mut i = 0;
    while i < ex.elems.len() {
        let e = ex.elems[i].piece;
        // the same circle (one centre, one squared radius, as expressions): q, on pc, is on e's
        // circle by construction, and the pieces run the same way exactly when their senses
        // agree; only the span is left to decide
        if let (
            Curve::Circle {
                c: c1,
                rad2: q1,
                ccw: w1,
                ..
            },
            Curve::Circle {
                c: c2,
                rad2: q2,
                ccw: w2,
                ..
            },
        ) = (pc.curve, e.curve)
            && c1 == c2
            && q1 == q2
        {
            if arr::on_span(cx, &e, q) {
                return if w1 == w2 { Where::Same } else { Where::Opposite };
            }
            i += 1;
            continue;
        }
        if on_curve(cx, &e, q) {
            let m = cx.mark();
            let (t1, t2) = (arr::tangent(cx, pc, q), arr::tangent(cx, &e, q));
            let d = arr::dot(cx, t1, t2);
            let s = cx.sign(d);
            cx.release(m);
            return if s == Ordering::Greater {
                Where::Same
            } else {
                Where::Opposite
            };
        }
        i += 1;
    }
    let mut odd = false;
    let mut l = 0;
    while l < ex.loops.len() {
        if region::inside_loop(cx, ex, l, q) {
            odd = !odd;
        }
        l += 1;
    }
    if odd { Where::In } else { Where::Out }
}

fn reversed(cx: &mut Ctx, p: &Piece) -> Piece {
    match p.curve {
        Curve::Line { d, dd, .. } => {
            let nd = Pt {
                x: cx.neg(d.x),
                y: cx.neg(d.y),
            };
            Piece {
                curve: Curve::Line { p: p.e, d: nd, dd },
                s: p.e,
                e: p.s,
                full: p.full,
                tag: p.tag,
            }
        }
        Curve::Circle { c, rad, rad2, ccw } => Piece {
            curve: Curve::Circle {
                c,
                rad,
                rad2,
                ccw: !ccw,
            },
            s: p.e,
            e: p.s,
            full: p.full,
            tag: p.tag,
        },
    }
}

/// The boolean of two valid regions (their loops with the region on the left), as loops.
pub fn boolean(a: &Region, b: &Region, op: Op) -> Result<Vec<OLoop>, OffsetError> {
    let mut cx = Ctx::new();
    let ea = region::validate(&mut cx, a).map_err(OffsetError::Region)?;
    let eb = region::validate(&mut cx, b).map_err(OffsetError::Region)?;
    let na = ea.elems.len();
    let mut pieces: Vec<Piece> = Vec::new();
    let mut i = 0;
    while i < na {
        pieces.push(ea.elems[i].piece);
        i += 1;
    }
    let mut j = 0;
    while j < eb.elems.len() {
        pieces.push(eb.elems[j].piece);
        j += 1;
    }
    let n = pieces.len();
    let mut on: Vec<Vec<Pt>> = vec![Vec::new(); n];
    let mut i = 0;
    while i < na {
        let mut j = na;
        while j < n {
            let (mut x, mut y) = (Vec::new(), Vec::new());
            arr::meet(&mut cx, &pieces[i], &pieces[j], &mut x, &mut y);
            on[i].append(&mut x);
            on[j].append(&mut y);
            j += 1;
        }
        i += 1;
    }
    let mut kept: Vec<Piece> = Vec::new();
    let mut i = 0;
    while i < n {
        let from_a = i < na;
        let pc = pieces[i];
        let mut pts = std::mem::take(&mut on[i]);
        let subs = crate::offset::split(&mut cx, &pc, &mut pts);
        let mut k = 0;
        while k < subs.len() {
            let sp = subs[k];
            let m = cx.mark();
            let q = arr::rep_point(&mut cx, &sp, sp.s, sp.e);
            let w = if from_a {
                locate(&mut cx, &eb, q, &sp)
            } else {
                locate(&mut cx, &ea, q, &sp)
            };
            cx.release(m);
            let keep = match (op, from_a, w) {
                (Op::Union, true, Where::Out | Where::Same) => 1,
                (Op::Union, false, Where::Out) => 1,
                (Op::Inter, true, Where::In | Where::Same) => 1,
                (Op::Inter, false, Where::In) => 1,
                (Op::Diff, true, Where::Out | Where::Opposite) => 1,
                (Op::Diff, false, Where::In) => -1,
                _ => 0,
            };
            if keep == 1 {
                kept.push(sp);
            } else if keep == -1 {
                let r = reversed(&mut cx, &sp);
                kept.push(r);
            }
            k += 1;
        }
        i += 1;
    }
    crate::offset::link(&mut cx, &kept)
}

/// The loops as a region (each loop's pieces as given). A loop that is one point is left out:
/// a region is its area, and a point has none.
pub fn region_of(loops: &[OLoop]) -> Region {
    Region {
        loops: loops
            .iter()
            .filter(|l| !(l.segs.len() == 1 && matches!(l.segs[0], region::Seg::Line { a, b } if a == b)))
            .map(|l| region::Loop { segs: l.segs.clone() })
            .collect(),
    }
}

#[allow(dead_code)]
fn _unused(_: RegionError) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::region::{Loop, Seg, p2, rect};

    fn circle(x: f64, y: f64, r: f64, ccw: bool) -> Loop {
        let a = p2(x + r, y);
        Loop {
            segs: vec![Seg::Arc {
                a,
                b: a,
                c: p2(x, y),
                ccw,
            }],
        }
    }

    #[test]
    fn square_minus_disc_and_shared_edges() {
        let sq = Region {
            loops: vec![rect(0.0, 0.0, 10.0, 10.0)],
        };
        let hole = Region {
            loops: vec![circle(5.0, 5.0, 2.0, true)],
        };
        let d = boolean(&sq, &hole, Op::Diff).unwrap();
        assert_eq!(d.len(), 2, "{d:?}");
        // a disc over the edge: the square with a bite
        let bite = Region {
            loops: vec![circle(10.0, 5.0, 2.0, true)],
        };
        let d = boolean(&sq, &bite, Op::Diff).unwrap();
        assert_eq!(d.len(), 1);
        assert!(
            d[0].segs.iter().any(|s| matches!(s, Seg::Arc { ccw: false, .. })),
            "{d:?}"
        );
        // two squares sharing an edge: their union is one rectangle's worth of loop
        let s2 = Region {
            loops: vec![rect(10.0, 0.0, 20.0, 10.0)],
        };
        let u = boolean(&sq, &s2, Op::Union).unwrap();
        assert_eq!(u.len(), 1, "{u:?}");
        let i = boolean(&sq, &s2, Op::Inter).unwrap();
        assert!(i.is_empty(), "{i:?}");
        // a square minus itself: nothing
        assert!(boolean(&sq, &sq, Op::Diff).unwrap().is_empty());
        let x = boolean(
            &sq,
            &Region {
                loops: vec![rect(5.0, 5.0, 15.0, 15.0)],
            },
            Op::Inter,
        )
        .unwrap();
        assert_eq!(x.len(), 1);
        assert_eq!(x[0].segs.len(), 4, "{x:?}");
    }
}
