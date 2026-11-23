//! The exact offset of a line/arc region.
//!
//! For r > 0, `offset(R, r)` is the boundary of C = {p ∈ R : d(p, ∂R) ≥ r} = R ⊖ B_r, the
//! tool-centre region of a cutter of radius r inside R. For r < 0 it is the boundary of
//! R ⊕ B_|r|, by the same construction on the complement.
//!
//! The one construction:
//! 1. Raw pieces. Every line gives its parallel at r on its inner (left) side; every arc its
//!    concentric arc at ρ ∓ r (none when that is not positive); every vertex where the boundary
//!    turns right (a reflex corner) gives the arc of radius r about it between the two normals.
//!    No other corner rule exists: at a convex corner the parallels cross, and the crossing is an
//!    event like any other.
//! 2. Events. Every meeting of two raw pieces is found exactly, and each piece is split there.
//! 3. Selection. A split piece is kept iff a point strictly inside it (chosen with a rational
//!    parameter) has d(p, e) ≥ r for every boundary element e. Every point of a raw piece is at
//!    distance exactly r from the element that made it, so a kept piece lies on ∂C; and every
//!    point of ∂C is on a raw piece (its nearest boundary point is inside an edge, or at a reflex
//!    vertex), so nothing of ∂C is lost. A piece changes from kept to dropped only where it meets
//!    another raw piece, which step 2 split.
//! 4. Loops. Kept pieces are linked end to start; where several start at one point, the sharpest
//!    left turn is taken. A one-dimensional part of C (a slot exactly 2r wide) appears as two
//!    coincident pieces of opposite sense: a there-and-back path, which is the tool path it is.
//!
//! Each pass of a pocket is an offset of the original boundary at its own distance, never of the
//! previous pass, so every vertex stays an algebraic number of low degree over the input.
use crate::arr::{self, Curve, Piece, Pt};
use crate::ival::Ival;
use crate::real::{Ctx, R};
use crate::region::{self, Elem, Exact, P2, Region, RegionError, Seg, p2};
use std::cmp::Ordering;

/// One closed loop of the offset, in f64 (each end the nearest f64 to the exact point, shared
/// bit for bit between neighbours). A loop of one zero-length line is an isolated point of C.
#[derive(Clone, Debug, PartialEq)]
pub struct OLoop {
    pub segs: Vec<Seg>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OffsetError {
    Region(RegionError),
    /// The kept pieces do not close into loops: a bug, never a result.
    Open,
}

/// Counters for the trace and the speed gate.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    pub raw: usize,
    pub split: usize,
    pub kept: usize,
    pub exact_evals: u64,
}

/// The offset of a validated region (see the module doc), with its counters.
pub fn offset(rg: &Region, r: f64) -> Result<(Vec<OLoop>, Stats), OffsetError> {
    if r < 0.0 {
        let (loops, st) = offset(&rg.reversed(), -r)?;
        let mut out = Vec::with_capacity(loops.len());
        let mut i = 0;
        while i < loops.len() {
            out.push(OLoop {
                segs: region::Loop {
                    segs: loops[i].segs.clone(),
                }
                .reversed()
                .segs,
            });
            i += 1;
        }
        return Ok((out, st));
    }
    let mut cx = Ctx::new();
    // an outward offset works on the complement: its loops have the opposite senses, which the
    // nesting rule would refuse, so it is validated as given and then reversed
    let ex = match region::validate(&mut cx, rg) {
        Ok(ex) => ex,
        Err(RegionError::Sense { .. }) => {
            let rv = rg.reversed();
            region::validate(&mut cx, &rv).map_err(OffsetError::Region)?;
            region::exact(&mut cx, rg).map_err(OffsetError::Region)?
        }
        Err(e) => return Err(OffsetError::Region(e)),
    };
    if r == 0.0 {
        let mut out = Vec::new();
        let mut i = 0;
        while i < rg.loops.len() {
            out.push(OLoop {
                segs: rg.loops[i].segs.clone(),
            });
            i += 1;
        }
        return Ok((out, Stats::default()));
    }
    let rr = cx.lit(r);
    let r2 = cx.sqr(rr);
    let mut st = Stats::default();
    let (raw, points) = raw_pieces(&mut cx, &ex, rr, r2);
    st.raw = raw.len();
    // events
    let n = raw.len();
    let mut on: Vec<Vec<Pt>> = vec![Vec::new(); n];
    let mut i = 0;
    while i < n {
        let mut j = i + 1;
        while j < n {
            let (mut a, mut b) = (Vec::new(), Vec::new());
            arr::meet(&mut cx, &raw[i], &raw[j], &mut a, &mut b);
            on[i].append(&mut a);
            on[j].append(&mut b);
            j += 1;
        }
        i += 1;
    }
    // split and select
    let mut kept: Vec<Piece> = Vec::new();
    let mut i = 0;
    while i < n {
        let pc = raw[i];
        let mut pts = std::mem::take(&mut on[i]);
        let subs = split(&mut cx, &pc, &mut pts);
        let mut k = 0;
        while k < subs.len() {
            st.split += 1;
            let sp = subs[k];
            let m = cx.mark();
            let q = arr::rep_point(&mut cx, &sp, sp.s, sp.e);
            let ok = far_enough(&mut cx, &ex, q, rr, r2, pc.tag);
            cx.release(m);
            if ok && !duplicate(&mut cx, &kept, &sp) {
                kept.push(sp);
            }
            k += 1;
        }
        i += 1;
    }
    st.kept = kept.len();
    let mut out = link(&mut cx, &kept)?;
    // isolated points: the centres of arcs whose offset collapsed to a point
    let mut k = 0;
    while k < points.len() {
        let q = points[k];
        if !on_kept(&mut cx, &kept, q) && far_enough(&mut cx, &ex, q, rr, r2, u32::MAX) {
            let p = p2(cx.approx(q.x), cx.approx(q.y));
            out.push(OLoop {
                segs: vec![Seg::Line { a: p, b: p }],
            });
        }
        k += 1;
    }
    st.exact_evals = cx.exact_evals;
    Ok((out, st))
}

/// Whether q lies on a kept piece (an isolated point only counts off every loop).
fn on_kept(cx: &mut Ctx, kept: &[Piece], q: Pt) -> bool {
    let mut i = 0;
    while i < kept.len() {
        let k = kept[i];
        let m = cx.mark();
        let on_curve = match k.curve {
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
        let on = on_curve && arr::on_span(cx, &k, q);
        cx.release(m);
        if on {
            return true;
        }
        i += 1;
    }
    false
}

/// The raw pieces, and the points arcs collapse to.
fn raw_pieces(cx: &mut Ctx, ex: &Exact, r: R, r2: R) -> (Vec<Piece>, Vec<Pt>) {
    let mut out = Vec::new();
    let mut pts = Vec::new();
    let mut i = 0;
    while i < ex.elems.len() {
        let e = ex.elems[i];
        let tag = i as u32;
        match e.piece.curve {
            Curve::Line { p, d, dd } => {
                let len = cx.sqrt(dd);
                let k = cx.div(r, len);
                let pp = arr::perp(cx, d);
                let off = arr::scale(cx, pp, k);
                let p1 = arr::add(cx, p, off);
                let e1 = arr::add(cx, p1, d);
                out.push(Piece {
                    curve: Curve::Line { p: p1, d, dd },
                    s: p1,
                    e: e1,
                    full: false,
                    tag,
                });
            }
            Curve::Circle { c, rad, ccw, .. } => {
                let nr = if ccw { cx.sub(rad, r) } else { cx.add(rad, r) };
                match cx.sign(nr) {
                    Ordering::Greater => {
                        let k = cx.div(nr, rad);
                        let vs = arr::sub(cx, e.piece.s, c);
                        let ve = arr::sub(cx, e.piece.e, c);
                        let os = arr::scale(cx, vs, k);
                        let oe = arr::scale(cx, ve, k);
                        let s = arr::add(cx, c, os);
                        let en = if e.piece.full { s } else { arr::add(cx, c, oe) };
                        let q = cx.sqr(nr);
                        out.push(Piece {
                            curve: Curve::Circle {
                                c,
                                rad: nr,
                                rad2: q,
                                ccw,
                            },
                            s,
                            e: en,
                            full: e.piece.full,
                            tag,
                        });
                    }
                    Ordering::Equal => pts.push(c),
                    Ordering::Less => {}
                }
            }
        }
        i += 1;
    }
    // reflex vertices
    let mut l = 0;
    while l < ex.loops.len() {
        let (a, b) = ex.loops[l];
        let mut i = a;
        while i < b {
            let nx = if i + 1 == b { a } else { i + 1 };
            if nx != i || !ex.elems[i].piece.full {
                let ein = ex.elems[i];
                let eout = ex.elems[nx];
                let tin = region::elem_tangent(cx, &ein, true);
                let tout = region::elem_tangent(cx, &eout, false);
                let c = arr::cross(cx, tin, tout);
                if cx.sign(c) == Ordering::Less && !ein.piece.full {
                    let v = ein.piece.e;
                    let nin = unit_normal(cx, &ein, true, r);
                    let nout = unit_normal(cx, &eout, false, r);
                    let s = arr::add(cx, v, nin);
                    let en = arr::add(cx, v, nout);
                    out.push(Piece {
                        curve: Curve::Circle {
                            c: v,
                            rad: r,
                            rad2: r2,
                            ccw: false,
                        },
                        s,
                        e: en,
                        full: false,
                        tag: (ex.elems.len() + i) as u32,
                    });
                }
            }
            i += 1;
        }
        l += 1;
    }
    (out, pts)
}

/// r times the left unit normal of an element at its start or end.
fn unit_normal(cx: &mut Ctx, e: &Elem, at_end: bool, r: R) -> Pt {
    match e.piece.curve {
        Curve::Line { d, dd, .. } => {
            let len = cx.sqrt(dd);
            let k = cx.div(r, len);
            let pp = arr::perp(cx, d);
            arr::scale(cx, pp, k)
        }
        Curve::Circle { c, rad, ccw, .. } => {
            let x = if at_end { e.piece.e } else { e.piece.s };
            let v = if ccw {
                arr::sub(cx, c, x)
            } else {
                arr::sub(cx, x, c)
            };
            let k = cx.div(r, rad);
            arr::scale(cx, v, k)
        }
    }
}

/// The piece split at the given points (on its curve, within its span).
pub(crate) fn split(cx: &mut Ctx, pc: &Piece, pts: &mut Vec<Pt>) -> Vec<Piece> {
    let mut out = Vec::new();
    if pc.full {
        if pts.is_empty() {
            out.push(*pc);
            return out;
        }
        // restart the circle at the first split point
        let mut base = *pc;
        base.s = pts[0];
        base.e = pts[0];
        arr::sort_along(cx, &base, pts);
        // a point met twice (three curves through one point) splits once
        dedup_same(cx, pts);
        while pts.len() > 1 && arr::same(cx, pts[pts.len() - 1], pts[0]) {
            pts.pop();
        }
        let mut k = 0;
        while k < pts.len() {
            let a = pts[k];
            let b = if k + 1 < pts.len() { pts[k + 1] } else { pts[0] };
            let mut sp = base;
            sp.s = a;
            sp.e = b;
            sp.full = pts.len() == 1;
            out.push(sp);
            k += 1;
        }
        return out;
    }
    let mut all = vec![pc.s];
    let mut k = 0;
    while k < pts.len() {
        all.push(pts[k]);
        k += 1;
    }
    all.push(pc.e);
    arr::sort_along(cx, pc, &mut all);
    // no piece of zero length: a split point exactly at an end, or met twice, splits once
    dedup_same(cx, &mut all);
    let mut k = 0;
    while k + 1 < all.len() {
        let mut sp = *pc;
        sp.s = all[k];
        sp.e = all[k + 1];
        out.push(sp);
        k += 1;
    }
    out
}

/// Drops each point exactly equal to the one before it (the points sorted along a curve).
fn dedup_same(cx: &mut Ctx, pts: &mut Vec<Pt>) {
    let mut out: Vec<Pt> = Vec::with_capacity(pts.len());
    let mut k = 0;
    while k < pts.len() {
        if out.is_empty() || !arr::same(cx, out[out.len() - 1], pts[k]) {
            out.push(pts[k]);
        }
        k += 1;
    }
    *pts = out;
}

/// d(q, e) ≥ r for every boundary element e but `skip` (the one that made the piece, whose
/// distance is r by construction).
fn far_enough(cx: &mut Ctx, ex: &Exact, q: Pt, r: R, r2: R, skip: u32) -> bool {
    let mut i = 0;
    while i < ex.elems.len() {
        if i as u32 != skip {
            let m = cx.mark();
            let ok = elem_far(cx, &ex.elems[i], q, r, r2);
            cx.release(m);
            if !ok {
                return false;
            }
        }
        i += 1;
    }
    true
}

fn elem_far(cx: &mut Ctx, e: &Elem, q: Pt, r: R, r2: R) -> bool {
    // the cheap reject: the box of the element grown by r misses q
    let (x0, y0, x1, y1) = arr::piece_box(cx, &e.piece);
    let (qx, qy, rv) = (cx.iv(q.x), cx.iv(q.y), cx.iv(r));
    // strictly more than r away along an axis: (lower box edge − r) above q, and so on
    if qx.hi < Ival::pt(x0).sub(rv).lo
        || qx.lo > Ival::pt(x1).add(rv).hi
        || qy.hi < Ival::pt(y0).sub(rv).lo
        || qy.lo > Ival::pt(y1).add(rv).hi
    {
        return true;
    }
    match e.piece.curve {
        Curve::Line { p, d, dd } => {
            let v = arr::sub(cx, q, p);
            let t = arr::dot(cx, v, d);
            if cx.sign(t) != Ordering::Greater {
                let n = arr::norm2(cx, v);
                return ge(cx, n, r2);
            }
            if cx.cmp(t, dd) != Ordering::Less {
                let w = arr::sub(cx, q, e.piece.e);
                let n = arr::norm2(cx, w);
                return ge(cx, n, r2);
            }
            let c = arr::cross(cx, d, v);
            let c2 = cx.sqr(c);
            let rd = cx.mul(r2, dd);
            ge(cx, c2, rd)
        }
        Curve::Circle { c, rad, .. } => {
            let v = arr::sub(cx, q, c);
            let n = arr::norm2(cx, v);
            let in_span = if cx.sign(n) == Ordering::Equal {
                true
            } else {
                // the circle point nearest q, c + rad v/|v|, within the arc
                let len = cx.sqrt(n);
                let k = cx.div(rad, len);
                let off = arr::scale(cx, v, k);
                let foot = arr::add(cx, c, off);
                arr::on_span(cx, &e.piece, foot)
            };
            if in_span {
                let len = cx.sqrt(n);
                let dlt = cx.sub(len, rad);
                let d2 = cx.sqr(dlt);
                return ge(cx, d2, r2);
            }
            let (ws, we) = (arr::sub(cx, q, e.piece.s), arr::sub(cx, q, e.piece.e));
            let (ns, ne) = (arr::norm2(cx, ws), arr::norm2(cx, we));
            ge(cx, ns, r2) && ge(cx, ne, r2)
        }
    }
}

fn ge(cx: &mut Ctx, a: R, b: R) -> bool {
    cx.cmp(a, b) != Ordering::Less
}

/// Whether the same piece (same curve, sense and ends) is already kept.
fn duplicate(cx: &mut Ctx, kept: &[Piece], sp: &Piece) -> bool {
    let mut i = 0;
    while i < kept.len() {
        let k = kept[i];
        if arr::same(cx, k.s, sp.s) && arr::same(cx, k.e, sp.e) && k.full == sp.full {
            let m = cx.mark();
            let ts = arr::tangent(cx, &k, k.s);
            let tp = arr::tangent(cx, sp, sp.s);
            let c = arr::cross(cx, ts, tp);
            let d = arr::dot(cx, ts, tp);
            let parallel = cx.sign(c) == Ordering::Equal && cx.sign(d) == Ordering::Greater;
            let kind = matches!(
                (k.curve, sp.curve),
                (Curve::Line { .. }, Curve::Line { .. }) | (Curve::Circle { .. }, Curve::Circle { .. })
            );
            cx.release(m);
            if parallel && kind {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Links kept pieces into closed loops.
pub(crate) fn link(cx: &mut Ctx, kept: &[Piece]) -> Result<Vec<OLoop>, OffsetError> {
    let n = kept.len();
    let mut used = vec![false; n];
    // the f64 of each exact end, computed once per piece end and shared with the next piece
    let mut loops = Vec::new();
    let mut start = 0;
    while start < n {
        if used[start] {
            start += 1;
            continue;
        }
        used[start] = true;
        let mut chain = vec![start];
        if !kept[start].full {
            let mut cur = start;
            loop {
                if arr::same(cx, kept[cur].e, kept[start].s) && closes_here(cx, kept, &used, cur, start) {
                    break;
                }
                let nx = next_piece(cx, kept, &used, cur);
                match nx {
                    Some(j) => {
                        used[j] = true;
                        chain.push(j);
                        cur = j;
                    }
                    None => return Err(OffsetError::Open),
                }
            }
        }
        loops.push(to_loop(cx, kept, &chain));
        start += 1;
    }
    Ok(loops)
}

/// At the loop's start point: close it, unless another unused piece starting here is a sharper
/// left turn than the loop's own first piece (then the walk goes on through it first).
fn closes_here(cx: &mut Ctx, kept: &[Piece], used: &[bool], cur: usize, start: usize) -> bool {
    match next_piece_with(cx, kept, used, cur, Some(start)) {
        Some(j) => j == start,
        None => true,
    }
}

fn next_piece(cx: &mut Ctx, kept: &[Piece], used: &[bool], cur: usize) -> Option<usize> {
    next_piece_with(cx, kept, used, cur, None)
}

/// The unused piece starting at cur's end with the sharpest left turn (or `also`, if given).
fn next_piece_with(
    cx: &mut Ctx,
    kept: &[Piece],
    used: &[bool],
    cur: usize,
    also: Option<usize>,
) -> Option<usize> {
    let end = kept[cur].e;
    let mut cands: Vec<usize> = Vec::new();
    let mut j = 0;
    while j < kept.len() {
        if (!used[j] || Some(j) == also) && j != cur && arr::same(cx, kept[j].s, end) {
            cands.push(j);
        }
        j += 1;
    }
    if cands.len() <= 1 {
        return cands.first().copied();
    }
    // the reverse of the incoming tangent is the reference; the sharpest left turn is the
    // outgoing direction nearest it going clockwise
    let m = cx.mark();
    let tin = arr::tangent(cx, &kept[cur], end);
    let back = Pt {
        x: cx.neg(tin.x),
        y: cx.neg(tin.y),
    };
    let mut best = cands[0];
    let mut k = 1;
    while k < cands.len() {
        let c = cands[k];
        let tb = arr::tangent(cx, &kept[best], end);
        let tc = arr::tangent(cx, &kept[c], end);
        let o = arr::angle_cmp(cx, back, tc, tb, false);
        let better = match o {
            Ordering::Less => true,
            Ordering::Greater => false,
            Ordering::Equal => curvier_left(cx, &kept[c], &kept[best]),
        };
        if better {
            best = c;
        }
        k += 1;
    }
    cx.release(m);
    Some(best)
}

/// Of two pieces leaving a point in the same direction, whether a turns further left.
fn curvier_left(cx: &mut Ctx, a: &Piece, b: &Piece) -> bool {
    // signed curvature: + for counter-clockwise circles, 0 for lines
    let ka = curvature_sign_and_rad(a);
    let kb = curvature_sign_and_rad(b);
    match (ka, kb) {
        ((sa, Some(ra)), (sb, Some(rb))) => {
            if sa != sb {
                return sa > sb;
            }
            // same sign: the smaller radius curves more
            let o = cx.cmp(ra, rb);
            if sa > 0 {
                o == Ordering::Less
            } else {
                o == Ordering::Greater
            }
        }
        ((sa, _), (sb, _)) => sa > sb,
    }
}

fn curvature_sign_and_rad(p: &Piece) -> (i32, Option<R>) {
    match p.curve {
        Curve::Line { .. } => (0, None),
        Curve::Circle { rad, ccw, .. } => (if ccw { 1 } else { -1 }, Some(rad)),
    }
}

fn to_loop(cx: &mut Ctx, kept: &[Piece], chain: &[usize]) -> OLoop {
    // each loop vertex's f64, once
    let mut verts: Vec<P2> = Vec::with_capacity(chain.len());
    let mut i = 0;
    while i < chain.len() {
        let s = kept[chain[i]].s;
        verts.push(p2(cx.approx(s.x), cx.approx(s.y)));
        i += 1;
    }
    let mut segs = Vec::with_capacity(chain.len());
    let mut i = 0;
    while i < chain.len() {
        let pc = kept[chain[i]];
        let a = verts[i];
        let b = if pc.full { a } else { verts[(i + 1) % chain.len()] };
        let sg = match pc.curve {
            Curve::Line { .. } => Seg::Line { a, b },
            Curve::Circle { c, ccw, .. } => Seg::Arc {
                a,
                b,
                c: p2(cx.approx(c.x), cx.approx(c.y)),
                ccw,
            },
        };
        segs.push(sg);
        i += 1;
    }
    // a piece whose two ends round to one f64 point (an arc of radius near zero, a sliver) is
    // dropped: its neighbours share that vertex, so the loop stays closed, and the region moves
    // by less than the rounding of its vertices; kept, it would read as a full circle or a
    // zero-length line. A loop that rounds to one point stays one (an isolated point).
    if segs.len() > 1 {
        let all = segs.clone();
        segs.retain(|s| match *s {
            Seg::Line { a, b } | Seg::Arc { a, b, .. } => a != b,
        });
        if segs.is_empty() {
            let a = all[0].start();
            segs.push(Seg::Line { a, b: a });
        }
    }
    OLoop { segs }
}
