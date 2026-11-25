//! An axis-aligned plane face (spanned by two coordinate axes: a workholding box's, a plate's,
//! the stock's) against a tool body on a line move, or a horizontal arc: a lower bound on their
//! distance in closed form, decided in exact arithmetic (real.rs), so a distance exactly at the
//! limit is proven (ForgeCam.AxisPlane.axisplane_clear, axisplane_clear_below).
//!
//! The branch and bound proves a clearance only above its limit by a margin its intervals leave;
//! a tool exactly at a fixture's clearance, or touching a modelled plate (distance 0 against a
//! limit of 0), stays undecided there. Here the bound is a separation along one coordinate axis
//! k, on one side: the body's near extent along k against the face box's far end. A separation
//! g ≥ 0 along one coordinate bounds the distance by g; the tip is linear in a line's parameter,
//! so each gap is too, and its least is at an end. The best over axes and sides is the bound.
//!
//! The model the lemma's hypotheses stand for (what REMOVED-CHECKS' "code = model" owes):
//! - the body (hp): a solid of revolution about the +z axis through the tip, as every `Body` is
//!   (a (ρ, z) profile; there is no other axis in the type: CAM is 3-axis). Every point lies
//!   within ρ ≤ rmax of the axis (rmax includes an arc's bulge, `Body::new`) and at heights
//!   tip + [z0, z1]; so along x or y within tip ± rmax, along z within tip + [z0, z1];
//! - the face (hq): its points lie in the box o + [p] u + [q] v (a trim lies within its
//!   parameter box, check.rs `Face`), each end computed exactly here; the true object lies
//!   within mu of the face, so along k within that box's extent ± mu;
//! - limit ≥ 0 and mu ≥ 0 (refused otherwise): a separation is a distance bound only when it is
//!   not negative (best − mu ≥ limit ≥ 0).
//!
//! Non-finite data is refused. Only ever a Clear: `None` passes the pair on (as `contact::clear`
//! does).
use crate::check::{Face, Move, Surf, Verdict};
use crate::real::Ctx;
use crate::tool::Body;
use std::cmp::Ordering;

/// Which axis a unit vector is (0, 1, 2) and its sign, if it is exactly ±e_k.
fn axis_of(v: [f64; 3]) -> Option<(usize, f64)> {
    let nz: Vec<usize> = (0..3).filter(|&k| v[k] != 0.0).collect();
    match nz.as_slice() {
        [k] if v[*k].abs() == 1.0 => Some((*k, v[*k])),
        _ => None,
    }
}

/// The face's box, if it is a plane spanned by two coordinate axes, each end exact (in `cx`):
/// along u's axis o + s·p over p's range, along v's o + s·q over q's, along the normal o. The face
/// (its trim within the parameter box) lies in it.
fn face_box(cx: &mut Ctx, f: &Face) -> Option<[[crate::real::R; 2]; 3]> {
    let Surf::Plane { o, u, v } = f.surf else {
        return None;
    };
    let (ku, su) = axis_of(u)?;
    let (kv, sv) = axis_of(v)?;
    if ku == kv
        || ![f.p.lo, f.p.hi, f.q.lo, f.q.hi]
            .iter()
            .chain(o.iter())
            .all(|x| x.is_finite())
    {
        return None;
    }
    let k = 3 - ku - kv;
    let mut span = |c: f64, s: f64, lo: f64, hi: f64| {
        let cc = cx.lit(c);
        let (l, h) = (cx.lit(s * lo), cx.lit(s * hi));
        let (a, b) = (cx.add(cc, l), cx.add(cc, h));
        // s is ±1, so s·lo and s·hi are exact; the smaller end first
        if s > 0.0 { [a, b] } else { [b, a] }
    };
    let bu = span(o[ku], su, f.p.lo, f.p.hi);
    let bv = span(o[kv], sv, f.q.lo, f.q.hi);
    let on = cx.lit(o[k]);
    let mut b = [[on, on]; 3];
    b[ku] = bu;
    b[kv] = bv;
    Some(b)
}

/// A proven Clear for `body` moved by `mv` against `f` at `limit`, or None.
///
/// The body at the tip t occupies at most the cylinder |xy − t.xy| ≤ rmax, t.z + z0 ≤ z ≤ t.z + z1;
/// the face lies in its box B. Along each coordinate axis, on each side, the gap between the
/// two (B's near end to the body's far one) is a lower bound on their distance wherever it is
/// positive; on a line the tip is linear in the move's parameter, so each gap is too and its
/// least over the move is at an end. The best such least is a lower bound for the whole move.
pub fn clear(body: &Body, f: &Face, mv: &Move, limit: f64) -> Option<Verdict> {
    let mut cx = Ctx::new();
    let fb = face_box(&mut cx, f)?;
    // limit ≥ 0 and mu ≥ 0, as the lemma has them: a separation is a distance bound only when
    // it is not negative (best − mu ≥ limit ≥ 0)
    if !(limit.is_finite()
        && limit >= 0.0
        && f.mu.is_finite()
        && f.mu >= 0.0
        && body.z0.is_finite()
        && body.z1.is_finite()
        && body.rmax.is_finite())
    {
        return None;
    }
    // the tip's ends; an arc only at one height, so only its height's gap (z) is linear
    let (a, b, axes): ([f64; 3], [f64; 3], &[usize]) = match *mv {
        Move::Line { a, b } => (a, b, &[0, 1, 2]),
        // (horizontal only: its plane's axes have no height, so every point is at c's)
        Move::Arc { c, x, y, .. } if x[2] == 0.0 && y[2] == 0.0 => (c, c, &[2]),
        _ => return None,
    };
    if !a.iter().chain(b.iter()).all(|x| x.is_finite()) {
        return None;
    }
    // the body's extent along axis k from the tip: [lo, hi] offsets
    let ext = |k: usize| -> (f64, f64) {
        if k == 2 {
            (body.z0, body.z1)
        } else {
            (-body.rmax, body.rmax)
        }
    };
    let mut best: Option<crate::real::R> = None;
    for &k in axes {
        let (lo, hi) = ext(k);
        for side in [0, 1] {
            // side 0: the face below/left of the body: gap = (t + lo) − face_hi
            // side 1: the face above/right:              gap = face_lo − (t + hi)
            let g = |cx: &mut Ctx, t: f64| {
                let tt = cx.lit(t);
                if side == 0 {
                    let l = cx.lit(lo);
                    let bl = cx.add(tt, l);
                    cx.sub(bl, fb[k][1])
                } else {
                    let h = cx.lit(hi);
                    let bh = cx.add(tt, h);
                    cx.sub(fb[k][0], bh)
                }
            };
            let ga = g(&mut cx, a[k]);
            let gb = g(&mut cx, b[k]);
            let d = cx.sub(ga, gb);
            let least = if cx.sign(d) == Ordering::Greater { gb } else { ga };
            best = Some(match best {
                None => least,
                Some(x) => {
                    let e = cx.sub(least, x);
                    if cx.sign(e) == Ordering::Greater { least } else { x }
                }
            });
        }
    }
    let best = best?;
    let need = {
        let l = cx.lit(limit);
        let m = cx.lit(f.mu);
        cx.add(l, m)
    };
    let over = cx.sub(best, need);
    if cx.sign(over) == Ordering::Less {
        return None;
    }
    // the reported lower bound: a float no more than the exact least − mu, and at least limit
    let m = cx.lit(f.mu);
    let lm = cx.sub(best, m);
    let lo = cx.iv(lm).lo;
    Some(Verdict::Clear { lower: lo.max(limit) })
}
