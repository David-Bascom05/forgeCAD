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
use std::collections::HashMap;
use std::sync::Mutex;

/// One closed loop of the offset, in f64: each end the exact point rounded to the grid `GRID`
/// (shared bit for bit between neighbours), arcs' centres too. A loop of one zero-length line
/// is an isolated point of C.
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

type Offsets = Result<(Vec<OLoop>, Stats), OffsetError>;

/// Offsets already made in this process, by the region's content and the distance: the offset
/// is a function of those numbers alone (each decision an exact sign of expressions built from
/// them), so the same region at the same distance has the same result, counters included. A
/// pocket's first pass and the opening that records what it cleared are the same offset.
/// Bounded: emptied when full.
static MADE: Mutex<Option<HashMap<(u64, u64), Vec<(Region, Offsets)>>>> = Mutex::new(None);
const MADE_MAX: usize = 512;

/// The offset of a validated region (see the module doc), with its counters.
pub fn offset(rg: &Region, r: f64) -> Offsets {
    let key = (region::content_hash(rg), r.to_bits());
    let known = MADE.lock().ok().and_then(|g| {
        g.as_ref()?
            .get(&key)?
            .iter()
            .find(|(q, _)| q == rg)
            .map(|(_, o)| o.clone())
    });
    if let Some(o) = known {
        return o;
    }
    let o = offset_anew(rg, r);
    if let Ok(mut g) = MADE.lock() {
        let m = g.get_or_insert_with(HashMap::new);
        if m.len() >= MADE_MAX {
            m.clear();
        }
        m.entry(key).or_default().push((rg.clone(), o.clone()));
    }
    o
}

fn offset_anew(rg: &Region, r: f64) -> Offsets {
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
    // a corner arc touches the two pieces it joins at their shared end, and only there (a line
    // tangent to a circle; two circles tangent at that point, their centres apart): those
    // junctions are recorded as they are, never solved for
    let ne = ex.elems.len() as u32;
    let mut by_tag: Vec<Option<usize>> = vec![None; ex.elems.len()];
    let mut k = 0;
    while k < raw.len() {
        if raw[k].tag < ne {
            by_tag[raw[k].tag as usize] = Some(k);
        }
        k += 1;
    }
    let mut joined: Vec<(usize, usize)> = Vec::new();
    let mut k = 0;
    while k < raw.len() {
        if raw[k].tag >= ne {
            let vi = (raw[k].tag - ne) as usize;
            let nx = next_in_loop(&ex, vi);
            if let Some(a) = by_tag[vi]
                && raw[a].e == raw[k].s
            {
                joined.push((a.min(k), a.max(k)));
            }
            if let Some(b) = by_tag[nx]
                && raw[b].s == raw[k].e
            {
                joined.push((b.min(k), b.max(k)));
            }
        }
        k += 1;
    }
    // events
    let n = raw.len();
    let mut on: Vec<Vec<Pt>> = vec![Vec::new(); n];
    let mut i = 0;
    while i < n {
        let mut j = i + 1;
        while j < n {
            if joined.contains(&(i, j)) {
                let v = if raw[i].e == raw[j].s { raw[i].e } else { raw[i].s };
                on[i].push(v);
                on[j].push(v);
                j += 1;
                continue;
            }
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
            // a corner arc's point is exactly r from its vertex (rep_point puts it on the
            // circle: the vertex plus r times a unit vector), the end of the two elements
            // that meet there
            let centre = if pc.tag >= ne {
                Some(ex.elems[(pc.tag - ne) as usize].piece.e)
            } else {
                None
            };
            let ok = far_enough(&mut cx, &ex, q, rr, r2, pc.tag, centre);
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
        if !on_kept(&mut cx, &kept, q) && far_enough(&mut cx, &ex, q, rr, r2, u32::MAX, None) {
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

/// The element after `i` in its loop.
fn next_in_loop(ex: &Exact, i: usize) -> usize {
    let mut l = 0;
    while l < ex.loops.len() {
        let (a, b) = ex.loops[l];
        if i >= a && i < b {
            return if i + 1 == b { a } else { i + 1 };
        }
        l += 1;
    }
    i
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
            Curve::Line { d, dd, .. } => {
                let len = cx.sqrt(dd);
                let k = cx.div(r, len);
                let pp = arr::perp(cx, d);
                let off = arr::scale(cx, pp, k);
                // both ends as vertex + normal: the same expressions the corner arcs and the
                // neighbours' ends are built from, so the pieces' junctions are shared exactly
                let p1 = arr::add(cx, e.piece.s, off);
                let e1 = arr::add(cx, e.piece.e, off);
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
                        // x + r n(x) = c + (x − c)(rad ∓ r)/rad: written from the vertex, like
                        // the corner arcs' ends
                        let ns = unit_normal(cx, &e, false, r);
                        let s = arr::add(cx, e.piece.s, ns);
                        let en = if e.piece.full {
                            s
                        } else {
                            let ne = unit_normal(cx, &e, true, r);
                            arr::add(cx, e.piece.e, ne)
                        };
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
        arr::sort_cyclic(cx, &base, pts);
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
/// Whether q is at least r from every boundary element but `skip` (the one that made the piece).
/// `centre`: a point q is known to lie exactly r from (a corner arc's vertex): an element end
/// that is that very point is r away by construction, not decided (it would be a tie).
fn far_enough(cx: &mut Ctx, ex: &Exact, q: Pt, r: R, r2: R, skip: u32, centre: Option<Pt>) -> bool {
    let mut i = 0;
    while i < ex.elems.len() {
        if i as u32 != skip {
            let m = cx.mark();
            let ok = elem_far(cx, &ex.elems[i], q, r, r2, centre);
            cx.release(m);
            if !ok {
                return false;
            }
        }
        i += 1;
    }
    true
}

fn elem_far(cx: &mut Ctx, e: &Elem, q: Pt, r: R, r2: R, centre: Option<Pt>) -> bool {
    // q's distance from an end that is `centre`: exactly r
    let at_r = |x: Pt| centre == Some(x);
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
                if at_r(p) {
                    return true;
                }
                let n = arr::norm2(cx, v);
                return ge(cx, n, r2);
            }
            if cx.cmp(t, dd) != Ordering::Less {
                if at_r(e.piece.e) {
                    return true;
                }
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
            let far = |cx: &mut Ctx, x: Pt| {
                if at_r(x) {
                    return true;
                }
                let w = arr::sub(cx, q, x);
                let n = arr::norm2(cx, w);
                ge(cx, n, r2)
            };
            far(cx, e.piece.s) && far(cx, e.piece.e)
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

/// The grid every output vertex and centre is rounded to: 2⁻³² mm (about 2.3·10⁻¹⁰ mm). One
/// rule for every region the exact core gives out. It is far below any face's mu (≥ 10⁻⁹), so a
/// clearance or a cleared void stated on it holds against the exact one; and it keeps the
/// regions an op hands to the next one free of the sub-ulp slivers and subnormal coordinates
/// that exact work on rounded inputs leaves (each such literal adds about a thousand bits to the
/// separation bound of every sign it takes part in). Coincident points become one literal.
pub const GRID: f64 = 1.0 / 4294967296.0;

/// `on_grid(cx.approx(a))`, from a's f64 enclosure [lo, hi] where that decides it. on_grid is
/// non-decreasing, and approx(a) lies in [lo, hi]: it is lo when lo = hi, else the common
/// nearest f64 of big-float bounds l ≤ a ≤ h, which is between nearest(lo) = lo and
/// nearest(hi) = hi as rounding is monotone (at the 1024-bit cap, nearest(l) or the
/// enclosure's midpoint, again in [lo, hi]). So when lo and hi round to one grid point, that
/// is the result, and the big-float evaluation is not needed; else as before.
fn grid_of(cx: &Ctx, a: R) -> f64 {
    let iv = cx.iv(a);
    let (l, h) = (on_grid(iv.lo), on_grid(iv.hi));
    if l == h && l.is_finite() {
        return l;
    }
    on_grid(cx.approx(a))
}

/// x rounded to the grid (exact in f64 for |x| < 2²⁰ mm; ties to even, as IEEE rounds).
pub fn on_grid(x: f64) -> f64 {
    let k = x / GRID;
    let r = k.round_ties_even();
    r * GRID
}

fn to_loop(cx: &mut Ctx, kept: &[Piece], chain: &[usize]) -> OLoop {
    // each loop vertex's grid point, once
    let mut verts: Vec<P2> = Vec::with_capacity(chain.len());
    let mut i = 0;
    while i < chain.len() {
        let s = kept[chain[i]].s;
        verts.push(p2(grid_of(cx, s.x), grid_of(cx, s.y)));
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
                c: p2(grid_of(cx, c.x), grid_of(cx, c.y)),
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
    // a loop within a few grid cells is below the grid: it is the point it shrinks to (as a
    // region it has no area; as tool centres, a plunge there clears the tool's disk)
    let (mut x0, mut y0, mut x1, mut y1) =
        (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for v in &verts {
        x0 = x0.min(v.x);
        y0 = y0.min(v.y);
        x1 = x1.max(v.x);
        y1 = y1.max(v.y);
    }
    let full = segs.len() == 1 && matches!(segs[0], Seg::Arc { .. });
    if !full && x1 - x0 <= 4.0 * GRID && y1 - y0 <= 4.0 * GRID {
        return OLoop {
            segs: vec![Seg::Line {
                a: verts[0],
                b: verts[0],
            }],
        };
    }
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
