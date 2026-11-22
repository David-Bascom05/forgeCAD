//! Cliff overflow of a closed round: where the spring of a ball rolling between two faces runs
//! over a sharp rim of one of them and comes back, the ball rests on the rim between (the other
//! face and the rim's curve, a tube round it) and on the faces again after. Decided on the
//! geometry alone: the spring is followed round the whole closed spine, the rim's crossings found
//! wherever they lie, and seams, which are no boundaries, are moved out of the way afterwards.

use super::*;

/// How a closed edge's round goes over a rim and back: the edge's parameters where its ball comes
/// onto the rim and leaves it, in order along the edge, the rim and its parameters there.
#[derive(Clone, Debug)]
pub(super) struct Cliff {
    pub edge: EdgeId,
    pub at: [f64; 2],
    pub rim: EdgeId,
    pub rim_at: [f64; 2],
    /// The rim's parameter at a point of the stretch between: which way round a closed rim it runs.
    pub rim_mid: f64,
}

/// `fillet_with`'s last try, every other refused: with closed rounds rolled over rims and back.
/// The round, where it builds and is valid.
pub(super) fn retry(body: &Body, edges: &[FilletEdge], opts: &FilletOptions, feature: u64) -> Option<Body> {
    if let Some(b) = retry_whole(body, edges, opts, feature) {
        return Some(b);
    }
    // a closed round seams cut into pieces: whole first
    let (whole, picks) = joined_loop(body, edges)?;
    retry_whole(&whole, &picks, opts, feature)
}

/// `retry` with the picks as they are.
fn retry_whole(body: &Body, edges: &[FilletEdge], opts: &FilletOptions, feature: u64) -> Option<Body> {
    // only a closed concave edge's round goes over a rim and back here: none picked, nothing to try
    // (a refusal the overlapping rounds take up costs no more for it)
    let closed = |f: &FilletEdge| {
        f.edge.index() < body.edges.len()
            && body.edge(f.edge).alive
            && body.edge(f.edge).start == body.edge(f.edge).end
            && fillet_kind(body, f.edge) == Some(false)
    };
    if !edges.iter().any(closed) {
        return None;
    }
    // what the caller asked, not the retry it had come to (fillet_with makes its own)
    let tried = FilletOptions {
        rims: true,
        cliff: true,
        across: false,
        seams: false,
        walk: false,
        ..opts.clone()
    };
    // the seams left where they are where they are out of the way, then all moved where the
    // geometry puts them (which way builds depends on the joints the seam leaves)
    [false, true].into_iter().find_map(|all| {
        let tried = FilletOptions {
            cliff_seams: all,
            ..tried.clone()
        };
        fillet_with(body, edges, &tried, feature)
            .ok()
            .filter(|b| forge_topo::check(b).is_empty() && forge_topo::mass::mass(b).is_ok())
    })
}

/// Each closed round of `by_edge` whose spring runs over a rim and back cut where the ball comes
/// onto the rim and leaves it, its seam moved off that stretch first where its vertex lies there
/// (`seam_off`; every seam it can be with `move_all`); the pieces join `all` and `picks`, the rims
/// `rims` and the pieces `pieces`.
/// Whether any was cut (all is then to be rolled again).
pub(super) fn split(
    move_all: bool,
    body: &mut Body,
    by_edge: &BTreeMap<EdgeId, Rolled>,
    all: &mut Vec<(EdgeId, f64)>,
    picks: &mut BTreeMap<EdgeId, &Radius>,
    (rims, pieces): (&mut BTreeSet<EdgeId>, &mut BTreeSet<EdgeId>),
) -> R<bool> {
    let cliffs: Vec<Cliff> = by_edge.values().filter_map(|r| find(body, r)).collect();
    for c in &cliffs {
        // a seam in the cliff's stretch moves to the middle of the stretch rolled on the faces
        let c = if c.at[1] <= c.at[0] || move_all {
            match seam_off(body, c) {
                Some(moved) => moved,
                None if c.at[1] <= c.at[0] => return Err(FilletError::Overflow { edge: c.edge }),
                None => c.clone(),
            }
        } else {
            c.clone()
        };
        let c = if rim_fits(body, &c) {
            c
        } else {
            rim_seam_off(body, &c)
                .filter(|c| rim_fits(body, c))
                .ok_or(FilletError::Overflow { edge: c.edge })?
        };
        rims.insert(c.rim);
        let other = split_edge(body, c.edge, c.at[0]);
        let last = split_edge(body, other, c.at[1]);
        pieces.extend([c.edge, other, last]);
        let at = all.iter().position(|f| f.0 == c.edge).expect("rolled");
        all.insert(at + 1, (other, all[at].1));
        all.insert(at + 2, (last, all[at].1));
        if let Some(&r) = picks.get(&c.edge) {
            picks.insert(other, r);
            picks.insert(last, r);
        }
    }
    Ok(!cliffs.is_empty())
}

/// Each piece of a round cut for a cliff, all along it: a cliff's ball clear of the face whose
/// rim it rests on (it would lean back into it where the round's spring comes back onto that face
/// between samples), the others' springs on their faces (one may leave for a stretch the samples
/// missed). Nothing here builds a ball touching the rim, its face and the other face at once.
pub(super) fn clear(body: &Body, by_edge: &BTreeMap<EdgeId, Rolled>, pieces: &BTreeSet<EdgeId>) -> R<()> {
    for r in by_edge.values().filter(|r| pieces.contains(&r.edge)) {
        let err = FilletError::Overflow { edge: r.edge };
        let x = body.edge(r.edge);
        let spine = &r.blend.spine;
        let t0 = spine_closest(spine, x.curve.eval(x.range.lo) + r.lean);
        let t1 = t0
            + along(
                spine_closest(spine, x.curve.eval(x.range.hi) + r.lean) - t0,
                spine.domain().period,
            );
        let at = |k: usize| t0 + (t1 - t0) * k as f64 / 64.0;
        match r.cliff {
            Some(s) => {
                // the rim's face and the faces round it (the one beyond the rim among them)
                let f = r.faces[s];
                let mut near: Vec<FaceId> = body
                    .face(f)
                    .loops
                    .iter()
                    .flat_map(|&l| body.lp(l).coedges())
                    .filter_map(|c| body.partner(c).map(|p| body.coedge_face(p)))
                    .filter(|&g| g != r.faces[1 - s])
                    .collect();
                near.push(f);
                near.sort_unstable();
                near.dedup();
                for k in 0..=64 {
                    let c = spine.eval(at(k));
                    for &g in &near {
                        let surface = &body.face(g).surface;
                        let foot = surface.eval(surface.project(c));
                        if foot.distance(c) < r.blend.radius - 10.0 * tol::LINEAR && on_face(body, g, foot) {
                            return Err(err);
                        }
                    }
                }
            }
            // (the ends lie on the rim itself)
            None => {
                for k in 1..64 {
                    if (0..2).any(|s| !on_face(body, r.faces[s], r.springs[s].eval(at(k)))) {
                        return Err(err);
                    }
                }
            }
        }
    }
    Ok(())
}

/// Samples of a closed spring round its spine.
const SAMPLES: usize = 256;

/// The edge's parameter at the section of `spine` at t: the edge's point in the plane square to
/// the spine there.
pub(super) fn section_u(x: &forge_topo::Edge, spine: &Curve, t: f64) -> f64 {
    let cs = spine.deriv(t);
    let mut u = x.curve.closest(cs.p, x.range);
    for _ in 0..20 {
        let d = x.curve.deriv(u);
        let step = (d.p - cs.p).dot(cs.d1) / d.d1.dot(cs.d1);
        u -= step;
        if step.abs() <= 1e-15 * (1.0 + u.abs()) {
            break;
        }
    }
    u
}

/// For closed concave rounds, where a spring runs over a sharp rim of its face and back: one
/// stretch on one rim. None where the round is not closed, its spring stays on its face, or
/// leaves it any other way.
pub(super) fn find(body: &Body, r: &Rolled) -> Option<Cliff> {
    let x = body.edge(r.edge);
    if x.start != x.end
        || r.convex
        || r.var.is_some()
        || r.cliff.is_some()
        || r.over.iter().any(Option::is_some)
    {
        return None;
    }
    let spine = &r.blend.spine;
    let period = spine.domain().period?;
    let t0 = spine_closest(spine, body.vertex(x.start).point);
    let tt = |i: usize| t0 + period * i as f64 / SAMPLES as f64;
    let ts: Vec<f64> = (0..SAMPLES).map(tt).collect();
    // each stretch off a rim, with its spine parameters
    let mut found: Vec<(Cliff, f64, f64)> = Vec::new();
    for s in 0..2 {
        let f = r.faces[s];
        let spring = &r.springs[s];
        // each sharp rim of the face, and the surface beyond it with its outward normal
        for &lp in &body.face(f).loops {
            for c in body.lp(lp).coedges() {
                let g = body.coedge(c).edge;
                if g == r.edge {
                    continue;
                }
                let Some(p) = body.partner(c) else { continue };
                let beyond = body.coedge_face(p);
                // a seam is no boundary
                if beyond == f || body.face(beyond).surface == body.face(f).surface {
                    continue;
                }
                let gx = body.edge(g);
                if !matches!(gx.curve, Curve::Line(_) | Curve::Circle(_)) {
                    continue;
                }
                // past the rim: on the far side of the surface beyond, near the rim
                let bs = &body.face(beyond).surface;
                let over = |t: f64| -> Option<f64> {
                    let q = spring.eval(t);
                    let uv = bs.project(q);
                    let foot = bs.eval(uv);
                    let n = body.face_normal(beyond, uv)?;
                    let rim = gx.curve.eval(gx.curve.closest(q, gx.range));
                    (rim.distance(q) <= 4.0 * r.blend.radius).then(|| (q - foot).dot(n.get()))
                };
                let h: Vec<Option<f64>> = ts.iter().map(|&t| over(t)).collect();
                let off: Vec<bool> = h.iter().map(|v| v.is_some_and(|v| v > 0.0)).collect();
                let n_off = off.iter().filter(|&&o| o).count();
                if n_off == 0 || n_off == SAMPLES {
                    continue;
                }
                // one stretch off: from the last sample on to the first off, round to the last off
                let starts: Vec<usize> = (0..SAMPLES)
                    .filter(|&i| off[i] && !off[(i + SAMPLES - 1) % SAMPLES])
                    .collect();
                let [i0] = starts[..] else { return None };
                let mut i1 = i0;
                while off[(i1 + 1) % SAMPLES] {
                    i1 += 1;
                }
                // the crossings, by bisection on the side of the surface beyond
                let refine = |a: f64, b: f64| -> Option<f64> {
                    let (mut a, mut b) = (a, b);
                    let ha = over(a)?;
                    for _ in 0..80 {
                        let m = 0.5 * (a + b);
                        let hm = over(m)?;
                        if (hm > 0.0) == (ha > 0.0) {
                            a = m;
                        } else {
                            b = m;
                        }
                    }
                    Some(0.5 * (a + b))
                };
                let step = period / SAMPLES as f64;
                let ta = refine(tt(i0) - step, tt(i0))?;
                let tb = refine(tt(i1), tt(i1 + 1))?;
                let on_rim = |t: f64| {
                    let q = spring.eval(t);
                    gx.curve.eval(gx.curve.closest(q, gx.range)).distance(q) <= tol::LINEAR
                };
                if !on_rim(ta) || !on_rim(tb) {
                    return None;
                }
                let rim_mid = gx.curve.closest(spring.eval(0.5 * (ta + tb)), gx.range);
                let (ua, ub) = (section_u(x, spine, ta), section_u(x, spine, tb));
                let (qa, qb) = (spring.eval(ta), spring.eval(tb));
                let (ra, rb) = (gx.curve.closest(qa, gx.range), gx.curve.closest(qb, gx.range));
                // along the edge as it runs
                let w = x.range.width();
                let lo = x.range.lo;
                let wrap = |u: f64| lo + (u - lo).rem_euclid(w);
                let (ua, ub, ra, rb) = if r.aligned {
                    (ua, ub, ra, rb)
                } else {
                    (ub, ua, rb, ra)
                };
                found.push((
                    Cliff {
                        edge: r.edge,
                        at: [wrap(ua), wrap(ub)],
                        rim: g,
                        rim_at: [ra, rb],
                        rim_mid,
                    },
                    ta,
                    tb,
                ));
            }
        }
    }
    // springs off both faces: the ball comes onto the rim of the stretch begun first, and what it
    // does after is that cliff's own (its other foot need not leave its face at all)
    match &found[..] {
        [(c, ..)] => Some(c.clone()),
        [(a, a0, a1), (b, b0, b1)] if a0 <= b0 && b1 <= a1 => Some(a.clone()),
        [(a, a0, a1), (b, b0, b1)] if b0 <= a0 && a1 <= b1 => Some(b.clone()),
        _ => None,
    }
}

/// Whether the rim can be cut where the ball comes onto it and leaves: the stretch between holds
/// none of the rim's own vertices.
pub(super) fn rim_fits(body: &Body, c: &Cliff) -> bool {
    let x = body.edge(c.rim);
    let (lo, hi) = (c.rim_at[0].min(c.rim_at[1]), c.rim_at[0].max(c.rim_at[1]));
    let near = |a: f64, b: f64| (a - b).abs() * x.curve.deriv(a).d1.length() <= 4.0 * tol::LINEAR;
    c.rim_mid > lo && c.rim_mid < hi && !near(lo, x.range.lo) && !near(hi, x.range.hi) && !near(lo, hi)
}

/// Splits edge `e` at parameter `t` at vertex `v` (lying there): `e` keeps the first part, the
/// returned edge is the second, and every loop through it walks both.
pub(super) fn split_at(body: &mut Body, e: EdgeId, t: f64, v: VertexId) -> EdgeId {
    let x = body.edge(e).clone();
    let other = body.add_edge(
        x.curve.clone(),
        Interval::new(t, x.range.hi),
        v,
        x.end,
        x.name.with_str("cliff"),
    );
    let edge = &mut body.edges[e.index()];
    edge.range.hi = t;
    edge.end = v;
    edge.bbox.clear();
    let mut loops: Vec<(FaceId, forge_topo::LoopId)> = x
        .coedges
        .iter()
        .map(|&c| (body.coedge_face(c), body.coedge(c).lp))
        .collect();
    loops.dedup();
    for (f, l) in loops {
        let mut uses: Vec<(EdgeId, bool)> = Vec::new();
        for d in body.lp(l).coedges() {
            let co = body.coedge(d);
            match (co.edge == e, co.forward) {
                (true, true) => uses.extend([(e, true), (other, true)]),
                (true, false) => uses.extend([(other, false), (e, false)]),
                _ => uses.push((co.edge, co.forward)),
            }
        }
        body.replace_loop(f, l, &uses);
    }
    other
}

/// A closed edge whose own vertex lies on the stretch its ball rests on the rim: where that vertex
/// only ends a seam of a round face beside the edge (a cylinder's or cone's line running to the
/// face's closed circular rim), the seam is no boundary of anything, and is moved round to the
/// middle of the stretch where the ball rests on the faces. The edge, the seam and that rim keep
/// their ids; their vertices move along them. The cliff, its parameters on the edges as they now run.
pub(super) fn seam_off(body: &mut Body, c: &Cliff) -> Option<Cliff> {
    let e = c.edge;
    let x = body.edge(e).clone();
    let period = x.curve.domain().period?;
    if x.start != x.end || (period - x.range.width()).abs() > 1e-9 * period {
        return None;
    }
    let v = x.start;
    let at_v: Vec<EdgeId> = body
        .edge_ids()
        .filter(|&g| g != e && (body.edge(g).start == v || body.edge(g).end == v))
        .collect();
    let [s] = at_v[..] else { return None };
    let sx = body.edge(s).clone();
    let [c0, c1] = sx.coedges[..] else { return None };
    let f = body.coedge_face(c0);
    if body.coedge_face(c1) != f || !matches!(sx.curve, Curve::Line(_)) {
        return None;
    }
    let surface = body.face(f).surface.clone();
    if !matches!(surface, Surface::Cylinder(_) | Surface::Cone(_)) || body.face(f).loops.len() != 1 {
        return None;
    }
    let w = if sx.start == v { sx.end } else { sx.start };
    let uses: Vec<EdgeId> = body
        .lp(body.face(f).loops[0])
        .coedges()
        .map(|k| body.coedge(k).edge)
        .collect();
    let rims: Vec<EdgeId> = uses.iter().copied().filter(|&g| g != e && g != s).collect();
    let [g] = rims[..] else { return None };
    let gx = body.edge(g).clone();
    if uses.len() != 4 || gx.start != w || gx.end != w || !matches!(gx.curve, Curve::Circle(_)) {
        return None;
    }
    // the middle of the stretch off the rim, round from the cliff's end to its start
    let (a, b) = (c.at[0], c.at[1]);
    let lo = x.range.lo;
    let wrap = |u: f64, from: f64| from + (u - from).rem_euclid(period);
    let mid = wrap(b + 0.5 * (a - b).rem_euclid(period), lo);
    let p = x.curve.eval(mid);
    let su = surface.project(p).x;
    let q0 = surface.eval(forge_math::Point2::new(
        su,
        surface.project(body.vertex(w).point).y,
    ));
    let full = Interval::new(gx.range.lo, gx.range.lo + dmath::TAU);
    let tw = gx.curve.closest(q0, full);
    let q = gx.curve.eval(tw);
    // the rim a circle of the surface's: the seam's line meets it there
    if q.distance(q0) > tol::LINEAR {
        return None;
    }
    let dir = if sx.start == v { q - p } else { p - q };
    let len = dir.length();
    let dir = dir.normalize()?;
    let start = if sx.start == v { p } else { q };
    body.vertices[v.index()].point = p;
    body.vertices[w.index()].point = q;
    let xe = &mut body.edges[e.index()];
    xe.range = Interval::new(mid, mid + period);
    xe.bbox.clear();
    let width = gx.range.width();
    let ge = &mut body.edges[g.index()];
    ge.range = Interval::new(tw, tw + width);
    ge.bbox.clear();
    let se = &mut body.edges[s.index()];
    se.curve = Curve::Line(forge_geom::Line::new(start, dir));
    se.range = Interval::new(0.0, len);
    se.bbox.clear();
    for k in [
        f,
        body.coedge_face(body.edge(g).coedges[0]),
        body.coedge_face(body.edge(g).coedges[1]),
    ] {
        body.face_changed(k);
    }
    let mut out = c.clone();
    out.at = [wrap(a, mid), wrap(b, mid)];
    if c.rim == g {
        let turn = gx.curve.domain().period?;
        let wrap_g = |u: f64| tw + (u - tw).rem_euclid(turn);
        out.rim_at = [wrap_g(c.rim_at[0]), wrap_g(c.rim_at[1])];
        out.rim_mid = wrap_g(c.rim_mid);
    }
    Some(out)
}

/// A closed circular rim whose own vertex lies on the stretch the ball rests on it: where that
/// vertex ends a seam of the round face the rim bounds, running to its other closed rim, the seam
/// is moved round that face (`move_seam`, to the middle of what its other loops leave free). The
/// cliff, its rim parameters as the rim now runs.
fn rim_seam_off(body: &mut Body, c: &Cliff) -> Option<Cliff> {
    let gx = body.edge(c.rim).clone();
    let turn = gx.curve.domain().period?;
    if gx.start != gx.end || !matches!(gx.curve, Curve::Circle(_)) {
        return None;
    }
    let f = gx
        .coedges
        .iter()
        .map(|&k| body.coedge_face(k))
        .find(|&f| !seams_at(body, f, gx.start).is_empty())?;
    move_seam(body, f)?;
    let lo = body.edge(c.rim).range.lo;
    let wrap = |u: f64| lo + (u - lo).rem_euclid(turn);
    let mut out = c.clone();
    out.rim_at = [wrap(c.rim_at[0]), wrap(c.rim_at[1])];
    out.rim_mid = wrap(c.rim_mid);
    Some(out)
}

/// The seams of face `f` (edges it holds on both sides) ending at vertex `v`.
pub(super) fn seams_at(body: &Body, f: FaceId, v: VertexId) -> Vec<EdgeId> {
    let mut out: Vec<EdgeId> = body
        .face(f)
        .loops
        .iter()
        .flat_map(|&l| body.lp(l).coedges())
        .map(|c| body.coedge(c).edge)
        .filter(|&e| {
            let x = body.edge(e);
            (x.start == v || x.end == v) && x.coedges.iter().all(|&c| body.coedge_face(c) == f)
        })
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Whether a body's counts pass `forge_topo::check`'s Euler rule (V - E + 2F - L even, and at
/// most 2 a shell): what a round's caller asks of it before keeping it, at no cost. A body with
/// tombstones is not judged here.
pub(super) fn counts_sound(b: &Body) -> bool {
    let value =
        b.vertices.len() as i64 - b.edges.len() as i64 + 2 * b.faces.len() as i64 - b.loops.len() as i64;
    b.dead > 0 || (value % 2 == 0 && value <= 2 * b.shells.len() as i64)
}

/// A closed round that seams cut into pieces: the picked edges between the same two faces running
/// round in one loop. The body with them one closed edge (the faces' meet traced whole), each seam
/// of a round face that ran across the loop moved round off it (the middle of what the loop leaves
/// free), and the picks with that edge for the pieces. None where the pieces are no such loop.
pub(super) fn joined_loop(body: &Body, edges: &[FilletEdge]) -> Option<(Body, Vec<FilletEdge>)> {
    // (nothing copied before the picks are seen to be such a loop)
    if body.dead > 0
        || edges
            .iter()
            .any(|f| f.edge.index() >= body.edges.len() || !body.edge(f.edge).alive)
    {
        return None;
    }
    let b = body;
    let picked: Vec<EdgeId> = edges.iter().map(|f| f.edge).collect();
    let faces_of = |b: &Body, e: EdgeId| -> Option<(FaceId, FaceId)> {
        let x = b.edge(e);
        let [c0, c1] = x.coedges[..] else { return None };
        let (f0, f1) = (b.coedge_face(c0), b.coedge_face(c1));
        (f0 != f1).then(|| if f0 < f1 { (f0, f1) } else { (f1, f0) })
    };
    // the first loop of open pieces between one pair of faces, all picked
    let (fa, fb) = picked.iter().find_map(|&e| {
        let x = b.edge(e);
        (x.start != x.end).then(|| faces_of(&b, e)).flatten()
    })?;
    let chain: Vec<EdgeId> = picked
        .iter()
        .copied()
        .filter(|&e| b.edge(e).start != b.edge(e).end && faces_of(&b, e) == Some((fa, fb)))
        .collect();
    if chain.len() < 2 {
        return None;
    }
    // each vertex of the pieces two of them: a loop
    let mut count: BTreeMap<VertexId, usize> = BTreeMap::new();
    for &e in &chain {
        *count.entry(b.edge(e).start).or_default() += 1;
        *count.entry(b.edge(e).end).or_default() += 1;
    }
    if count.values().any(|&n| n != 2) {
        return None;
    }
    let radius = edges.iter().find(|f| f.edge == chain[0])?.radius.clone();
    if edges
        .iter()
        .filter(|f| chain.contains(&f.edge))
        .any(|f| f.radius != radius)
    {
        return None;
    }
    // every other edge at a vertex of the loop a seam of one of the two faces
    let others: Vec<EdgeId> = b
        .edge_ids()
        .filter(|&g| !chain.contains(&g))
        .filter(|&g| count.contains_key(&b.edge(g).start) || count.contains_key(&b.edge(g).end))
        .collect();
    for &g in &others {
        let x = b.edge(g);
        let [c0, c1] = x.coedges[..] else { return None };
        let f = b.coedge_face(c0);
        if b.coedge_face(c1) != f || (f != fa && f != fb) || !matches!(x.curve, Curve::Line(_)) {
            return None;
        }
    }
    let mut b = body.clone();
    // a face whose seam the loop splits (pieces of it at two of the loop's vertices): its seam
    // moved to a line clear of the loop, between its two rims
    let seam_face: Vec<(EdgeId, FaceId)> = others
        .iter()
        .map(|&g| (g, b.coedge_face(b.edge(g).coedges[0])))
        .collect();
    for f in [fa, fb] {
        let pieces: Vec<EdgeId> = seam_face.iter().filter(|x| x.1 == f).map(|x| x.0).collect();
        if pieces.len() >= 2 {
            reseam_across(&mut b, f, &pieces, &chain)?;
        }
    }
    // the meet traced whole, the loop's vertex the one a seam still ends at (else the first's start)
    let (sa, sb) = (b.face(fa).surface.clone(), b.face(fb).surface.clone());
    let mut region = Box3::EMPTY;
    for &e in &chain {
        region = region.hull(&b.edge_box(e));
    }
    let region = region.grow(0.1 * region.diagonal() + tol::LINEAR);
    let mid = {
        let x = b.edge(chain[0]);
        x.curve.eval(x.range.mid())
    };
    let traced = forge_geom::intcurve::trace(&sa, &sb, region)?;
    let ic = traced.into_iter().find(|c| {
        c.closed && {
            let t = c.closest(mid, Interval::new(0.0, c.domain_hi()));
            c.eval(t).distance(mid) <= tol::LINEAR
        }
    })?;
    let curve = Curve::Intersection(Arc::new(ic));
    let keep = count
        .keys()
        .copied()
        .find(|&v| {
            b.edge_ids().any(|g| {
                !chain.contains(&g) && b.edge(g).alive && (b.edge(g).start == v || b.edge(g).end == v)
            })
        })
        .unwrap_or(b.edge(chain[0]).start);
    let period = curve.domain().period?;
    let t0 = curve.closest(b.vertex(keep).point, Interval::new(0.0, period));
    if curve.eval(t0).distance(b.vertex(keep).point) > tol::LINEAR {
        return None;
    }
    // along the pieces as the first one runs
    let first = b.edge(chain[0]).clone();
    let along = curve.deriv(curve.closest(mid, Interval::new(0.0, period))).d1;
    let fwd_first = first.curve.deriv(first.range.mid()).d1;
    let same = along.dot(fwd_first) > 0.0;
    let name = chain.iter().map(|&e| b.edge(e).name).min()?;
    let whole = b.add_edge(curve, Interval::new(t0, t0 + period), keep, keep, name);
    // each face's loop through the pieces walks the one edge instead, the first piece's way
    for f in [fa, fb] {
        let loops = b.face(f).loops.to_vec();
        for l in loops {
            let uses: Vec<(EdgeId, bool)> = b
                .lp(l)
                .coedges()
                .map(|c| (b.coedge(c).edge, b.coedge(c).forward))
                .collect();
            let Some(at) = uses.iter().position(|u| u.0 == chain[0]) else {
                continue;
            };
            let dir = uses[at].1 == same;
            let rest: Vec<(EdgeId, bool)> = uses.iter().copied().filter(|u| !chain.contains(&u.0)).collect();
            // the pieces lie in one run of the loop: the one edge goes where that run began
            let start = (0..uses.len())
                .find(|&k| {
                    chain.contains(&uses[k].0) && !chain.contains(&uses[(k + uses.len() - 1) % uses.len()].0)
                })
                .unwrap_or(0);
            let before = uses[..start].iter().filter(|u| !chain.contains(&u.0)).count();
            let mut new = rest.clone();
            new.insert(before, (whole, dir));
            if uses.iter().filter(|u| chain.contains(&u.0)).count() != chain.len() {
                return None;
            }
            b.replace_loop(f, l, &new);
        }
    }
    let b = b.compact();
    let joined = b
        .edge_ids()
        .find(|&e| b.edge(e).name == name && b.edge(e).start == b.edge(e).end)?;
    let mut picks: Vec<FilletEdge> = Vec::new();
    for f in edges {
        let n = body.edge(f.edge).name;
        if chain.iter().any(|&c| body.edge(c).name == n) {
            continue;
        }
        let e = b.edge_ids().find(|&e| b.edge(e).name == n)?;
        picks.push(FilletEdge {
            edge: e,
            radius: f.radius.clone(),
        });
    }
    picks.push(FilletEdge { edge: joined, radius });
    Some((b, picks))
}

/// Face `f`'s seam, in pieces from its rims to the loop `chain` runs round, made one line between
/// the rims clear of the loop: the rims' vertices moved round to the middle of the widest stretch
/// the loop leaves free, the pieces gone. Its loops: the rims and the new seam, and the loop apart.
fn reseam_across(b: &mut Body, f: FaceId, pieces: &[EdgeId], chain: &[EdgeId]) -> Option<()> {
    let surface = b.face(f).surface.clone();
    if !matches!(surface, Surface::Cylinder(_) | Surface::Cone(_)) || b.face(f).loops.len() != 1 {
        return None;
    }
    let lp = b.face(f).loops[0];
    let uses: Vec<(EdgeId, bool)> = b
        .lp(lp)
        .coedges()
        .map(|c| (b.coedge(c).edge, b.coedge(c).forward))
        .collect();
    let ends = |b: &Body, u: (EdgeId, bool)| {
        let x = b.edge(u.0);
        if u.1 { (x.start, x.end) } else { (x.end, x.start) }
    };
    // the rims: closed circles of the face's loop
    let rims: Vec<EdgeId> = uses
        .iter()
        .map(|u| u.0)
        .filter(|&e| !pieces.contains(&e) && !chain.contains(&e))
        .collect();
    let [r0, r1] = rims[..] else { return None };
    for r in [r0, r1] {
        let x = b.edge(r);
        if x.start != x.end || !matches!(x.curve, Curve::Circle(_)) {
            return None;
        }
    }
    // the angle round the face clear of the loop
    let mut taken: Vec<f64> = Vec::new();
    for &e in chain {
        let x = b.edge(e);
        for k in 0..=32 {
            let p = x.curve.eval(x.range.lo + x.range.width() * k as f64 / 32.0);
            taken.push(surface.project(p).x.rem_euclid(dmath::TAU));
        }
    }
    taken.sort_by(f64::total_cmp);
    let (mut gap, mut at) = (
        taken[0] + dmath::TAU - taken[taken.len() - 1],
        taken[taken.len() - 1],
    );
    for w in taken.windows(2) {
        if w[1] - w[0] > gap {
            (gap, at) = (w[1] - w[0], w[0]);
        }
    }
    let u = at + 0.5 * gap;
    let (v0, v1) = (b.edge(r0).start, b.edge(r1).start);
    let move_to = |b: &Body, v: VertexId| {
        let uv = surface.project(b.vertex(v).point);
        surface.eval(forge_math::Point2::new(u, uv.y))
    };
    let (p0, p1) = (move_to(b, v0), move_to(b, v1));
    for (r, v, p) in [(r0, v0, p0), (r1, v1, p1)] {
        let x = b.edge(r).clone();
        let t = x
            .curve
            .closest(p, Interval::new(x.range.lo, x.range.lo + dmath::TAU));
        if x.curve.eval(t).distance(p) > tol::LINEAR {
            return None;
        }
        let e = &mut b.edges[r.index()];
        e.range = Interval::new(t, t + x.range.width());
        e.bbox.clear();
        b.vertices[v.index()].point = p;
    }
    // the seam up from v0 to v1, its pieces' direction kept: the first piece leaving v0's rim
    let up = uses
        .iter()
        .copied()
        .find(|&u| pieces.contains(&u.0) && ends(b, u).0 == v0)?;
    let x = b.edge(up.0).clone();
    let dir = (p1 - p0).normalize()?;
    let len = (p1 - p0).length();
    let seam = b.add_edge(
        Curve::Line(forge_geom::Line::new(p0, dir)),
        Interval::new(0.0, len),
        v0,
        v1,
        x.name,
    );
    // the outer loop: each rim with the seam between, as the old loop ran round; the chain's
    // coedges, in the order they came, the loop apart
    let mut outer: Vec<(EdgeId, bool)> = Vec::new();
    let mut hole: Vec<(EdgeId, bool)> = Vec::new();
    let n = uses.len();
    let s = uses.iter().position(|u| u.0 == r0)?;
    for k in 0..n {
        let u = uses[(s + k) % n];
        if chain.contains(&u.0) {
            hole.push(u);
        } else if u.0 == r0 || u.0 == r1 {
            outer.push(u);
            let (_, e) = ends(b, u);
            outer.push((seam, e == v0));
        }
    }
    if outer.len() != 4 || hole.is_empty() {
        return None;
    }
    // the hole's coedges in a closed run: rotated so each starts where the last ended
    b.replace_loop(f, lp, &outer);
    b.add_loop(f, &hole);
    b.face_changed(f);
    Some(())
}
