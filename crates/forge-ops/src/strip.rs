//! Fillets that take a face away. A face fillet rounds between two sets of faces: where they
//! meet it rounds the edges between, and where a band of strip faces (a chamfer, a land, a
//! bevel of several facets) joins them it rolls one ball touching both sides and the band goes. A full round takes a face between
//! two sides away with the ball that touches all three, its radius changing where the sides
//! close in.

use crate::fillet::{FilletEdge, FilletError, Radius, chain_loops, fillet, inside_face, nearest_hit};
use forge_geom::blend::{Blend, closed_form, foot, offset, spine, spine_closest};
use forge_geom::varblend::{Guide, VarBlend};
use forge_geom::{Circle, Curve, CurveGeom, Cylinder, Surface, SurfaceGeom, Torus};
use forge_math::{Box3, Frame, Interval, Point3, Vec3, dmath, tol};
use forge_topo::{Body, CoedgeId, EdgeId, FaceId, LoopId, Name, VertexId};
use std::collections::{BTreeMap, BTreeSet};

type R<T> = Result<T, FilletError>;

/// Rounds between faces `first` and faces `second` with radius `radius`: the edges where the
/// two sets meet, or else the band of strip faces joining them (one face of each set along
/// it), taken away by the ball.
pub fn face_fillet(body: &Body, first: &[FaceId], second: &[FaceId], radius: f64, feature: u64) -> R<Body> {
    let (body, mut first) = compacted(body, &[first, second].concat());
    let second = first.split_off(first.len() - second.len());
    let (a, b): (BTreeSet<FaceId>, BTreeSet<FaceId>) =
        (first.iter().copied().collect(), second.iter().copied().collect());
    let faces_of = |e: EdgeId| -> Vec<FaceId> {
        body.edge(e)
            .coedges
            .iter()
            .map(|&c| body.coedge_face(c))
            .collect()
    };
    let between: Vec<EdgeId> = body
        .edge_ids()
        .filter(|&e| {
            let f = faces_of(e);
            f.len() == 2
                && ((a.contains(&f[0]) && b.contains(&f[1])) || (b.contains(&f[0]) && a.contains(&f[1])))
        })
        .collect();
    if !between.is_empty() {
        let edges: Vec<FilletEdge> = between
            .into_iter()
            .map(|edge| FilletEdge {
                edge,
                radius: Radius::Constant(radius),
            })
            .collect();
        return fillet(&body, &edges, feature);
    }
    // the band joining them: four-sided faces in neither set, in a row from a across to b
    // (a chamfer and a land, a bevel of several facets) or side by side along between them
    let sides = |f: FaceId| -> Option<Vec<CoedgeId>> {
        let l = &body.face(f).loops;
        (!a.contains(&f) && !b.contains(&f) && l.len() == 1 && body.lp(l[0]).coedges().len() == 4)
            .then(|| body.lp(l[0]).coedges().collect())
    };
    let across = |c: CoedgeId| body.partner(c);
    let face_of = |c: CoedgeId| body.coedge_face(c);
    // how far the surface turns crossing coedge c's edge onto the face across
    let turn = |c: CoedgeId| {
        let x = body.edge(body.coedge(c).edge);
        let p = x.curve.eval(x.range.mid());
        let n = |f: FaceId| {
            body.face_normal(f, body.face(f).surface.project(p))
                .map(|u| u.get())
        };
        match (n(face_of(c)), across(c).and_then(|q| n(face_of(q)))) {
            (Some(u), Some(v)) => dmath::atan2(u.cross(v).length(), u.dot(v)),
            _ => dmath::PI,
        }
    };
    // each band with how far the surface turns over it from a to b: the band between is the
    // one that turns least (round the far side of a bar turns further)
    let mut bands: BTreeMap<Vec<FaceId>, f64> = BTreeMap::new();
    for f in body.face_ids() {
        let Some(cs) = sides(f) else { continue };
        for i in 0..4 {
            // a row: in from a at side i, out across side i + 2, until b
            if !across(cs[i]).is_some_and(|p| a.contains(&face_of(p))) {
                continue;
            }
            let (mut row, mut out) = (vec![f], cs[(i + 2) % 4]);
            let mut turned = turn(cs[i]);
            while let Some(p) = across(out) {
                let g = face_of(p);
                turned += turn(out);
                if b.contains(&g) {
                    row.sort();
                    bands.insert(row, turned);
                    break;
                }
                let Some(next) = sides(g).filter(|_| !row.contains(&g) && row.len() < 16) else {
                    break;
                };
                let k = next.iter().position(|&x| x == p).expect("in its loop");
                out = next[(k + 2) % 4];
                row.push(g);
            }
        }
    }
    // side by side: faces with a and b across from each other, joined at their ends
    let between = |f: FaceId| {
        let cs = sides(f)?;
        let on = |c: CoedgeId, set: &BTreeSet<FaceId>| across(c).is_some_and(|p| set.contains(&face_of(p)));
        (0..2)
            .find(|&i| (on(cs[i], &a) && on(cs[i + 2], &b)) || (on(cs[i], &b) && on(cs[i + 2], &a)))
            .map(|i| turn(cs[i]) + turn(cs[i + 2]))
    };
    let mut left: BTreeSet<FaceId> = body.face_ids().filter(|&f| between(f).is_some()).collect();
    while let Some(&f) = left.iter().next() {
        let mut part = vec![f];
        left.remove(&f);
        let mut k = 0;
        while k < part.len() {
            let next: Vec<FaceId> = sides(part[k])
                .expect("a strip")
                .iter()
                .filter_map(|&c| across(c).map(face_of))
                .filter(|g| left.contains(g))
                .collect();
            for g in next {
                left.remove(&g);
                part.push(g);
            }
            k += 1;
        }
        part.sort();
        let turned = between(part[0]).expect("a strip between");
        bands.insert(part, turned);
    }
    let fail = FilletError::NoBlend { edge: EdgeId(0) };
    let mut ranked: Vec<(f64, Vec<FaceId>)> = bands.into_iter().map(|(k, t)| (t, k)).collect();
    ranked.sort_by(|x, y| x.0.total_cmp(&y.0));
    let band = match &ranked[..] {
        [(_, band)] => band,
        [(t0, band), (t1, _), ..] if *t1 > t0 + 1e-6 => band,
        _ => return Err(fail),
    };
    // the faces of each set along it
    let side = |set: &BTreeSet<FaceId>| {
        band.iter()
            .flat_map(|&f| body.lp(body.face(f).loops[0]).coedges())
            .filter_map(|c| across(c).map(face_of))
            .find(|f| set.contains(f))
    };
    let (Some(fa), Some(fb)) = (side(&a), side(&b)) else {
        return Err(fail);
    };
    strip(&body, fa, band, fb, Some(radius), feature)
}

/// Takes face `middle` away between faces `side` and `other` with the ball touching all three:
/// one blend face over it, a cylinder or torus where the radius holds along it, else a canal
/// surface whose radius is solved with each ball. A middle closing round on itself (a ring's
/// top) rounds all the way round, with constant radius.
pub fn full_round(body: &Body, side: FaceId, middle: FaceId, other: FaceId, feature: u64) -> R<Body> {
    let (body, f) = compacted(body, &[side, middle, other]);
    strip(&body, f[0], &[f[1]], f[2], None, feature)
}

/// The body without its dead entities, and the faces in it (found again by name: compacting
/// renumbers them; a face not there gets an id past the end).
fn compacted(body: &Body, faces: &[FaceId]) -> (Body, Vec<FaceId>) {
    if body.dead == 0 {
        return (body.clone(), faces.to_vec());
    }
    let out = body.compact();
    let ids = faces
        .iter()
        .map(|&f| {
            (f.index() < body.faces.len() && body.face(f).alive)
                .then(|| out.face_by_name(body.face(f).name))
                .flatten()
                .unwrap_or(FaceId(u32::MAX))
        })
        .collect();
    (out, ids)
}

/// One end of the band, from side a to side b: the faces across it in order, each with its
/// coedges along the band; the edges to a and b there and the band's corners on them; and
/// between faces, the edge they share and the band's corner on it.
struct End {
    faces: Vec<(FaceId, Vec<CoedgeId>)>,
    to_a: (EdgeId, VertexId),
    to_b: (EdgeId, VertexId),
    splits: Vec<(EdgeId, VertexId)>,
}

/// Takes the `band` of faces away between faces `fa` and `fb` (they bound it along two sides,
/// the faces across its ends the other two) with the ball rolling between fa and fb: of
/// `radius`, else touching the band's one face too. The springs replace the band's edges to fa
/// and fb, the cuts across each end face its edges there, and the edges between end faces
/// and to fa and fb are cut back to the blend.
fn strip(body: &Body, fa: FaceId, band: &[FaceId], fb: FaceId, radius: Option<f64>, feature: u64) -> R<Body> {
    let ids_ok = |f: FaceId| f.index() < body.faces.len() && body.face(f).alive;
    let fail = FilletError::NoBlend { edge: EdgeId(0) };
    if !ids_ok(fa) || !ids_ok(fb) || band.is_empty() || !band.iter().all(|&f| ids_ok(f)) {
        return Err(fail);
    }
    if let [m] = band[..]
        && body.face(m).loops.len() == 2
    {
        return closed_strip(body, fa, m, fb, radius, feature);
    }
    if band.iter().any(|&f| body.face(f).loops.len() != 1) {
        return Err(fail);
    }
    let inside: BTreeSet<FaceId> = band.iter().copied().collect();
    let across = |c: CoedgeId| body.partner(c).map(|p| body.coedge_face(p));
    let end_of = |c: CoedgeId| {
        let x = body.edge(body.coedge(c).edge);
        if body.coedge(c).forward { x.end } else { x.start }
    };
    // the band's border, chained round
    let border: Vec<CoedgeId> = band
        .iter()
        .flat_map(|&f| body.lp(body.face(f).loops[0]).coedges())
        .filter(|&c| !across(c).is_some_and(|g| inside.contains(&g)))
        .collect();
    let mut order = vec![border[0]];
    while order.len() < border.len() {
        let v = end_of(*order.last().expect("non-empty"));
        let next: Vec<CoedgeId> = border
            .iter()
            .copied()
            .filter(|&c| body.coedge_start(c) == v)
            .collect();
        match next[..] {
            [c] if c != order[0] => order.push(c),
            _ => return Err(fail),
        }
    }
    if end_of(order[order.len() - 1]) != body.coedge_start(order[0]) {
        return Err(fail);
    }
    // its runs: along a, across an end, along b, across the other end
    let side = |c: CoedgeId| match across(c) {
        Some(f) if f == fa => 0,
        Some(f) if f == fb => 1,
        _ => 2,
    };
    let n = order.len();
    let Some(k0) = (0..n).find(|&i| side(order[i]) == 0 && side(order[(i + n - 1) % n]) != 0) else {
        return Err(fail);
    };
    order.rotate_left(k0);
    let mut runs: Vec<(usize, Vec<CoedgeId>)> = Vec::new();
    for &c in &order {
        match runs.last_mut() {
            Some((s, r)) if *s == side(c) => r.push(c),
            _ => runs.push((side(c), vec![c])),
        }
    }
    if runs.iter().map(|r| r.0).collect::<Vec<_>>() != [0, 2, 1, 2]
        || order.iter().any(|&c| across(c).is_none())
    {
        return Err(fail);
    }
    let pick = |r: &[CoedgeId]| body.coedge(r[r.len() / 2]).edge;
    let (ea, eb) = (pick(&runs[0].1), pick(&runs[2].1));
    let err = FilletError::NoBlend { edge: ea };
    let lp_of = |c: CoedgeId| body.lp(body.coedge(c).lp).coedges();
    let step = |c: CoedgeId, by: usize| {
        let lp = body.lp(body.coedge(c).lp);
        let k = lp.coedges().position(|x| x == c).expect("in its loop");
        lp.coedge((k + by) % lp.coedges().len())
    };
    let partner = |c: CoedgeId| body.partner(c).ok_or(err.clone());
    // each end walked from the side it leaves to the side it reaches, then put a to b
    let mut ends = Vec::new();
    for (run, from) in [(&runs[1].1, fa), (&runs[3].1, fb)] {
        let mut faces: Vec<(FaceId, Vec<CoedgeId>)> = Vec::new();
        for &c in run {
            let (f, p) = (across(c).expect("checked"), partner(c)?);
            match faces.last_mut() {
                Some((g, ps)) if *g == f => ps.push(p),
                _ => faces.push((f, vec![p])),
            }
        }
        // in an end face's loop its band coedges run backwards: the edge after the first
        // comes from where the end starts, the one before the last reaches where it stops
        let edge_to = |c: CoedgeId, by: usize, want: FaceId| -> R<EdgeId> {
            let x = step(partner(c)?, by);
            (across(x) == Some(want))
                .then_some(body.coedge(x).edge)
                .ok_or(err.clone())
        };
        let mut splits = Vec::new();
        let mut k = 0;
        for w in faces.windows(2) {
            k += w[0].1.len();
            let last = run[k - 1];
            let lp = lp_of(partner(last)?);
            splits.push((edge_to(last, lp.len() - 1, w[1].0)?, end_of(last)));
        }
        let (first, last) = (run[0], run[run.len() - 1]);
        let to = if from == fa { fb } else { fa };
        let lead = (edge_to(first, 1, from)?, body.coedge_start(first));
        let tail = (edge_to(last, lp_of(partner(last)?).len() - 1, to)?, end_of(last));
        ends.push(if from == fa {
            End {
                faces,
                to_a: lead,
                to_b: tail,
                splits,
            }
        } else {
            faces.reverse();
            splits.reverse();
            End {
                faces,
                to_a: tail,
                to_b: lead,
                splits,
            }
        });
    }
    let mid = |e: EdgeId| {
        let x = body.edge(e);
        x.curve.eval(x.range.mid())
    };
    let m = band[0];
    let region = band
        .iter()
        .fold(body.face_box(fa).hull(&body.face_box(fb)), |b, &f| {
            b.hull(&body.face_box(f))
        });
    let pm = band
        .iter()
        .fold(Box3::EMPTY, |b, &f| b.hull(&body.face_box(f)))
        .center();
    let (convex, radius, blend) = round_blend(body, [fa, m, fb], [ea, eb], [mid(ea), pm, mid(eb)], radius)?;
    let springs = [
        blend.spring(true).ok_or(err.clone())?,
        blend.spring(false).ok_or(err.clone())?,
    ];
    let mut out = body.clone();
    let mark = match band {
        [m] => body.face(*m).name.whole(),
        _ => Name::NONE.with_set(&band.iter().map(|&f| body.face(f).name.whole()).collect::<Vec<_>>()),
    };
    let name = Name::feature(feature).with_str("round").with_name(mark);
    let overflow = FilletError::Overflow { edge: ea };
    // named from the faces as wholes: a split elsewhere on one renames nothing here
    let whole = |f: FaceId| body.face(f).name.whole();
    let alike = ends[0].faces.iter().any(|a| ends[1].faces.iter().any(|b| whole(a.0) == whole(b.0)));
    let reach = region.diagonal() + 4.0 * radius;
    // an edge's end at v moved to where the blend crosses it, and the band's corner v with it
    let mut moved: BTreeMap<VertexId, VertexId> = BTreeMap::new();
    let mut trim = |out: &mut Body, e: EdgeId, v: VertexId, (p, q): (Point3, VertexId)| -> R<()> {
        moved.insert(v, q);
        let edge = &mut out.edges[e.index()];
        let t = edge.curve.closest(p, edge.range);
        if edge.start == v {
            (edge.start, edge.range.lo) = (q, t);
        } else {
            (edge.end, edge.range.hi) = (q, t);
        }
        edge.bbox.clear();
        if edge.range.width() <= 0.0 {
            return Err(overflow.clone());
        }
        Ok(())
    };
    // per end: where each spring meets the end faces, where the blend crosses the edges between
    // them, and the cuts across them
    let mut hits = Vec::new();
    let mut cuts = Vec::new();
    for (k, end) in ends.iter().enumerate() {
        let mut at = [(0.0, Point3::ORIGIN, VertexId(0)); 2];
        let sides = [
            (end.to_a, &end.faces[0]),
            (end.to_b, &end.faces[end.faces.len() - 1]),
        ];
        for (j, (spring, ((e, v), (ef, _)))) in springs.iter().zip(sides).enumerate() {
            let vp = body.vertex(v).point;
            let t = spring.closest(vp, spring.full_range(1e6));
            let speed = spring.deriv(t).d1.length().max(1e-12);
            let d = spring.domain();
            let w = match d.period {
                Some(_) => Interval::new(t - reach / speed, t + reach / speed),
                None => Interval::new((t - reach / speed).max(d.lo), (t + reach / speed).min(d.hi)),
            };
            let (t, p) = nearest_hit(spring, w, &body.face(*ef).surface, vp).ok_or(overflow.clone())?;
            let x = body.edge(e);
            if x.curve.eval(x.curve.closest(p, x.range)).distance(p) > tol::LINEAR {
                return Err(overflow);
            }
            let vn = Name::NONE
                .with_set(&[name, whole(*ef)])
                .with(j as u64)
                .with(if alike { k as u64 } else { 0 });
            at[j] = (t, p, out.add_vertex(p, vn));
            trim(&mut out, e, v, (p, at[j].2))?;
        }
        let mut xs = vec![(at[0].1, at[0].2)];
        for (i, &(e, v)) in end.splits.iter().enumerate() {
            let x = body.edge(e);
            let (_, p) = nearest_hit(&x.curve, x.range, &blend.surface, body.vertex(v).point)
                .ok_or(overflow.clone())?;
            let names = [name, whole(end.faces[i].0), whole(end.faces[i + 1].0)];
            let q = out.add_vertex(p, Name::NONE.with_set(&names));
            trim(&mut out, e, v, (p, q))?;
            xs.push((p, q));
        }
        xs.push((at[1].1, at[1].2));
        // the cut across each end face, through the band's edge there
        for (i, (f, ps)) in end.faces.iter().enumerate() {
            let es = &body.face(*f).surface;
            let (a, b) = (xs[i], xs[i + 1]);
            let cut = match &blend.var {
                Some(v) => v.cut(es, a.0, b.0),
                None => cut_through(&blend.surface, es, a.0, b.0, mid(body.coedge(ps[0]).edge)),
            };
            let (curve, ta, tb) = cut.ok_or(overflow.clone())?;
            let en = name
                .with_str("end")
                .with_name(whole(*f))
                .with(if alike { k as u64 } else { 0 })
                .with(i as u64);
            let e = if ta <= tb {
                out.add_edge(curve, Interval::new(ta, tb), a.1, b.1, en)
            } else {
                out.add_edge(curve, Interval::new(tb, ta), b.1, a.1, en)
            };
            cuts.push((*f, ps.clone(), e));
        }
        hits.push(at);
    }
    // the springs between the ends, each the stretch through the band's edge on its side
    let mut spliced = Vec::new();
    for (j, (curve, (e, f))) in springs.iter().zip([(ea, fa), (eb, fb)]).enumerate() {
        let (t0, v0) = (hits[0][j].0, hits[0][j].2);
        let (t1, v1) = (hits[1][j].0, hits[1][j].2);
        let (lo, hi) = span(curve, t0, t1, mid(e)).ok_or(overflow.clone())?;
        let (va, vb) = if (lo - t0).abs() <= (lo - t1).abs() {
            (v0, v1)
        } else {
            (v1, v0)
        };
        let range = Interval::new(lo, hi);
        if !inside_face(body, f, curve, range) {
            return Err(overflow);
        }
        let sn = Name::NONE.with_set(&[name, whole(f)]);
        let new = out.add_edge(curve.clone(), range, va, vb, sn);
        let run: Vec<CoedgeId> = runs[2 * j].1.iter().map(|&c| partner(c)).collect::<R<_>>()?;
        spliced.push((f, run, new));
    }
    spliced.extend(cuts);
    // the faces round the band walk the new edges where they walked its border
    let mut blend_uses = Vec::new();
    for (f, run, new) in &spliced {
        let l = body.coedge(run[0]).lp;
        let lp: Vec<CoedgeId> = body.lp(l).coedges().collect();
        // the run's first coedge round the loop: walk the new edge from where its start moved
        let first = *run
            .iter()
            .find(|&&c| !run.contains(&step(c, lp.len() - 1)))
            .ok_or(err.clone())?;
        let fwd = moved.get(&body.coedge_start(first)) == Some(&out.edge(*new).start);
        let uses: Vec<(EdgeId, bool)> = lp
            .iter()
            .filter(|&x| !run.contains(x) || *x == first)
            .map(|&x| {
                if x == first {
                    (*new, fwd)
                } else {
                    (body.coedge(x).edge, body.coedge(x).forward)
                }
            })
            .collect();
        out.remove_loop(*f, l);
        out.add_loop(*f, &uses);
        let face = &mut out.faces[f.index()];
        face.bbox.clear();
        face.query.clear();
        blend_uses.push((*new, !fwd));
    }
    for &g in band {
        out.remove_loop(g, body.face(g).loops[0]);
        out.kill_face(g);
    }
    let f = out.add_face(body.face(fa).shell, blend.surface.clone(), convex, name);
    match &chain_loops(&out, &blend_uses)[..] {
        [lp] if lp.len() == blend_uses.len() => {
            out.add_loop(f, lp);
        }
        _ => return Err(err),
    }
    Ok(out.into_compact())
}

/// A strip closing round on itself (a ring's top between two coaxial walls): each of its two
/// loops one closed edge, to side a and to side b, whose vertex the side's seam alone meets.
/// The springs (circles) replace those edges, the seams are cut back to them, and the blend
/// face takes a seam of its own: the section through side a's seam crossing.
fn closed_strip(
    body: &Body,
    fa: FaceId,
    m: FaceId,
    fb: FaceId,
    radius: Option<f64>,
    feature: u64,
) -> R<Body> {
    let ml = &body.face(m).loops;
    // M's coedge on each loop, the loop's edge closed and to one side
    let lone = |l: LoopId| match body.lp(l).coedges().collect::<Vec<_>>()[..] {
        [c] if body.edge(body.coedge(c).edge).start == body.edge(body.coedge(c).edge).end => {
            body.partner(c).map(|p| (c, body.coedge_face(p)))
        }
        _ => None,
    };
    let fail = FilletError::NoBlend { edge: EdgeId(0) };
    let (Some(x), Some(y)) = (lone(ml[0]), lone(ml[1])) else {
        return Err(fail);
    };
    let (ca, cb) = match (x.1, y.1) {
        (p, q) if p == fa && q == fb => (x.0, y.0),
        (p, q) if p == fb && q == fa => (y.0, x.0),
        _ => return Err(fail),
    };
    let (ea, eb) = (body.coedge(ca).edge, body.coedge(cb).edge);
    let err = FilletError::NoBlend { edge: ea };
    let (xa, xb) = (body.edge(ea), body.edge(eb));
    let pa = xa.curve.eval(xa.range.mid());
    let pb = xb.curve.eval(xb.curve.closest(pa, xb.range));
    let sm = &body.face(m).surface;
    let pm = sm.eval(sm.project(pa.lerp(pb, 0.5)));
    let (convex, r, blend) = round_blend(body, [fa, m, fb], [ea, eb], [pa, pm, pb], radius)?;
    let (Some(Curve::Circle(ka)), Some(Curve::Circle(kb))) = (blend.spring(true), blend.spring(false)) else {
        return Err(FilletError::BadRadius { edge: ea });
    };
    // each side's seam: the one other edge at its vertex, on its face both ways
    let seam = |e: EdgeId, f: FaceId| -> R<EdgeId> {
        let v = body.edge(e).start;
        let at: BTreeSet<EdgeId> = body
            .face(f)
            .loops
            .iter()
            .flat_map(|&l| body.lp(l).coedges().map(|c| body.coedge(c).edge))
            .filter(|&x| x != e && (body.edge(x).start == v || body.edge(x).end == v))
            .collect();
        match at.iter().collect::<Vec<_>>()[..] {
            [&s] if body.edge(s).coedges.iter().all(|&c| body.coedge_face(c) == f) => Ok(s),
            _ => Err(err.clone()),
        }
    };
    let (sa, sb) = (seam(ea, fa)?, seam(eb, fb)?);
    let mut out = body.clone();
    let name = Name::feature(feature)
        .with_str("round")
        .with_name(body.face(m).name.whole());
    // where each seam crosses its spring's plane: the seam cut back there
    let mut cross = |s: EdgeId, e: EdgeId, k: &Circle, f: FaceId| -> R<VertexId> {
        let v = body.edge(e).start;
        let plane = Surface::Plane(forge_geom::Plane { frame: k.frame });
        let x = body.edge(s);
        let (t, p) = nearest_hit(&x.curve, x.range, &plane, body.vertex(v).point).ok_or(err.clone())?;
        if k.eval(k.closest(p, Interval::new(0.0, dmath::TAU))).distance(p) > tol::LINEAR {
            return Err(FilletError::Overflow { edge: ea });
        }
        let q = out.add_vertex(
            p,
            Name::NONE.with_set(&[name, body.face(f).name.whole()]).with_str("at"),
        );
        let edge = &mut out.edges[s.index()];
        if edge.start == v {
            (edge.start, edge.range.lo) = (q, t);
        } else {
            (edge.end, edge.range.hi) = (q, t);
        }
        edge.bbox.clear();
        if edge.range.width() <= tol::LINEAR {
            return Err(FilletError::Overflow { edge: ea });
        }
        Ok(q)
    };
    let (qa, qb) = (cross(sa, ea, &ka, fa)?, cross(sb, eb, &kb, fb)?);
    // the blend's own seam: the section through qa, over the middle face, to b's spring
    let qp = out.vertex(qa).point;
    let t = spine_closest(&blend.spine, qp);
    let (c, (_, fb_at)) = (blend.spine.eval(t), blend.feet(t));
    let up = sm.eval(sm.project(c)) - c;
    let x = qp - c;
    let z = x.cross(up).normalize().ok_or(err.clone())?;
    let sec = Circle {
        frame: Frame::from_zx(c, z, x).ok_or(err.clone())?,
        radius: r,
    };
    let l = sec.frame.point_to_local(fb_at);
    let phi = dmath::wrap_tau(dmath::atan2(l.y, l.x));
    let full = |k: &Circle, p: Point3| k.closest(p, Interval::new(0.0, dmath::TAU));
    // spring b from qb round to qb, through a vertex where the seam lands unless that is qb
    let qbp = out.vertex(qb).point;
    let (tb, tp) = (full(&kb, qbp), full(&kb, fb_at));
    let sn = |f: FaceId| Name::NONE.with_set(&[name, body.face(f).name.whole()]);
    let tau = dmath::TAU;
    let (pv, b_arcs) = if qbp.distance(fb_at) <= tol::LINEAR {
        let e = out.add_edge(Curve::Circle(kb), Interval::new(tb, tb + tau), qb, qb, sn(fb));
        (qb, vec![e])
    } else {
        let pv = out.add_vertex(fb_at, sn(fb).with_str("at").with(1));
        let tp = tb + (tp - tb).rem_euclid(tau);
        let e1 = out.add_edge(Curve::Circle(kb), Interval::new(tb, tp), qb, pv, sn(fb));
        let e2 = out.add_edge(
            Curve::Circle(kb),
            Interval::new(tp, tb + tau),
            pv,
            qb,
            sn(fb).with(1),
        );
        (pv, vec![e1, e2])
    };
    let ta = full(&ka, qp);
    let spa = out.add_edge(Curve::Circle(ka), Interval::new(ta, ta + tau), qa, qa, sn(fa));
    let bseam = out.add_edge(
        Curve::Circle(sec),
        Interval::new(0.0, phi),
        qa,
        pv,
        name.with_str("seam"),
    );
    // the walls walk their springs the way they walked the strip's edges
    let along = |c: CoedgeId, k: &Circle, t: f64| {
        let x = body.edge(body.coedge(c).edge);
        let d = x.curve.deriv(x.range.lo).d1 * if body.coedge(c).forward { 1.0 } else { -1.0 };
        d.dot(k.deriv(t).d1) > 0.0
    };
    let (pa_c, pb_c) = (
        body.partner(ca).ok_or(err.clone())?,
        body.partner(cb).ok_or(err.clone())?,
    );
    let (da, db) = (along(pa_c, &ka, ta), along(pb_c, &kb, tb));
    // spring b's arcs from `from` round to it, one way or the other
    let b_chain = |fwd: bool, from_qb: bool| -> Vec<(EdgeId, bool)> {
        let mut v: Vec<(EdgeId, bool)> = b_arcs.iter().map(|&e| (e, fwd)).collect();
        if !fwd {
            v.reverse();
        }
        if !from_qb {
            v.rotate_left(1);
        }
        v
    };
    for (c, chain) in [(pa_c, vec![(spa, da)]), (pb_c, b_chain(db, true))] {
        let f = body.coedge_face(c);
        let l = body.coedge(c).lp;
        let uses: Vec<(EdgeId, bool)> = body
            .lp(l)
            .coedges()
            .flat_map(|x| {
                if x == c {
                    chain.clone()
                } else {
                    vec![(body.coedge(x).edge, body.coedge(x).forward)]
                }
            })
            .collect();
        out.remove_loop(f, l);
        out.add_loop(f, &uses);
        let face = &mut out.faces[f.index()];
        face.bbox.clear();
        face.query.clear();
    }
    for &l in ml {
        out.remove_loop(m, l);
    }
    out.kill_face(m);
    let f = out.add_face(body.face(fa).shell, blend.surface.clone(), convex, name);
    let mut uses = vec![(spa, !da), (bseam, true)];
    uses.extend(b_chain(!db, false));
    uses.push((bseam, false));
    out.add_loop(f, &uses);
    Ok(out.into_compact())
}

/// The ball rolling between faces `fa` and `fb` over the strip `m` between their edges `ea`
/// and `eb` (points `pa`, `pb` on them, `pm` on or near m): with `radius`, else touching all
/// three. Whether it is convex, its (largest) radius and the blend.
fn round_blend(
    body: &Body,
    [fa, m, fb]: [FaceId; 3],
    [ea, eb]: [EdgeId; 2],
    [pa, pm, pb]: [Point3; 3],
    radius: Option<f64>,
) -> R<(bool, f64, Blend)> {
    let err = FilletError::NoBlend { edge: ea };
    let (sa, sb, sm) = (
        body.face(fa).surface.clone(),
        body.face(fb).surface.clone(),
        body.face(m).surface.clone(),
    );
    let sense = |f: FaceId| if body.face(f).same_sense { 1.0 } else { -1.0 };
    let normal = |f: FaceId, p: Point3| body.face_normal(f, body.face(f).surface.project(p));
    let na = normal(fa, pa).ok_or(err.clone())?;
    // convex: side b lies behind side a, the ball inside the material
    let convex = (pb - pa).dot(na.get()) < 0.0;
    let s = if convex { -1.0 } else { 1.0 };
    let region = body.face_box(m).hull(&body.face_box(fa)).hull(&body.face_box(fb));
    let (radius, sp) = match radius {
        Some(r) => {
            if !(r > tol::MIN_LENGTH && r.is_finite()) {
                return Err(FilletError::BadRadius { edge: ea });
            }
            let nb = normal(fb, pb).ok_or(err.clone())?;
            let bis = (na.get() + nb.get()).normalize().map_or(Vec3::ZERO, |u| u.get());
            let near = pm + bis * (s * r);
            let blend = Blend::new(
                &sa,
                &sb,
                s * r * sense(fa),
                s * r * sense(fb),
                near,
                region.grow(4.0 * r),
            )
            .ok_or(err.clone())?;
            (r, blend)
        }
        None => {
            // the ball touching all three at the strip's middle
            let sides = [s * sense(fa), s * sense(m), s * sense(fb)];
            let (cb, r) = touching_ball(
                &[(&sa, sides[0]), (&sb, sides[2]), (&sm, sides[1])],
                pm,
                body.face_box(m).diagonal(),
            )
            .ok_or(err.clone())?;
            let (da, db, dm) = (s * r * sense(fa), s * r * sense(fb), s * r * sense(m));
            let (oa, om, ob) = (
                offset(&sa, da).ok_or(err.clone())?,
                offset(&sm, dm).ok_or(err.clone())?,
                offset(&sb, db).ok_or(err.clone())?,
            );
            let c0 = pm - normal(m, pm).ok_or(err.clone())?.get() * (s * -r);
            let spine = spine(&oa, &om, c0, region.grow(4.0 * r)).ok_or(err.clone())?;
            // the same ball all along when the spine runs on side b's offset too; else the
            // radius changes along (the sides close in or open out)
            let range = spine.full_range(1e6);
            let t0 = spine.closest(c0, range);
            if [t0 - 1.0, t0, t0 + 1.0]
                .iter()
                .any(|&t| ob.implicit(spine.eval(t)).abs() > tol::LINEAR)
            {
                let blend = varying_round(body, [ea, eb], [&sa, &sm, &sb], sides, (cb, r))?;
                (blend.radius, blend)
            } else {
                let surface = match &spine {
                    Curve::Line(l) => Surface::Cylinder(Cylinder {
                        frame: Frame::from_zx(l.origin, l.dir, foot(&sa, da, l.origin) - l.origin)
                            .unwrap_or_else(|| Frame::from_z(l.origin, l.dir)),
                        radius: r,
                    }),
                    Curve::Circle(k) if k.radius > r + tol::LINEAR => Surface::Torus(Torus {
                        frame: k.frame,
                        major: k.radius,
                        minor: r,
                    }),
                    // the centres' circle is smaller than the ball: it shrinks as the ball
                    // grows (it never gets here growing), k - (r' - r) > r' up to their mean
                    Curve::Circle(k) => {
                        return Err(FilletError::TooLarge {
                            edge: ea,
                            max: 0.5 * (k.radius + r),
                        });
                    }
                    _ => return Err(err),
                };
                (
                    r,
                    Blend {
                        a: sa.clone(),
                        b: sb.clone(),
                        da,
                        db,
                        radius: r,
                        spine,
                        surface,
                        var: None,
                    },
                )
            }
        }
    };
    Ok((convex, radius, sp))
}

/// The full round whose radius changes along the strip: one canal surface whose balls touch
/// both sides and the middle (`faces` and the balls' `sides` of them, in a, m, b order), guided
/// along the strip (between its straight edges `es` to a and b, else by the one to a) and
/// carried on past its ends to run out; `ball` is one of them. Shorter run-outs where the
/// balls shrink away past the ends.
fn varying_round(
    body: &Body,
    es: [EdgeId; 2],
    faces: [&Surface; 3],
    sides: [f64; 3],
    ball: (Point3, f64),
) -> R<Blend> {
    let (x, y) = (body.edge(es[0]), body.edge(es[1]));
    let ends = |e: &forge_topo::Edge| [body.vertex(e.start).point, body.vertex(e.end).point];
    let ([a0, a1], [b0, b1]) = (ends(x), ends(y));
    let (b0, b1) = if a0.distance(b0) <= a0.distance(b1) {
        (b0, b1)
    } else {
        (b1, b0)
    };
    let (m0, m1) = (a0.lerp(b0, 0.5), a1.lerp(b1, 0.5));
    let path = match (&x.curve, &y.curve, forge_geom::Line::through(m0, m1)) {
        (Curve::Line(_), Curve::Line(_), Some(l)) => {
            (Curve::Line(l), Interval::new(0.0, m0.distance(m1)), true)
        }
        _ => (x.curve.clone(), x.range, true),
    };
    for reach in [4.0, 2.0, 1.0, 0.5].map(|k| k * ball.1) {
        let Some((guide, _, whole)) = Guide::through_edges(std::slice::from_ref(&path), 8, reach) else {
            break;
        };
        if let Ok(vb) = VarBlend::round(faces, sides, guide, whole, ball) {
            return Ok(Blend::variable(faces[0], faces[2], vb));
        }
    }
    Err(FilletError::BadRadius { edge: es[0] })
}

/// The ball touching every surface (each offset on its side, +1 or -1) near `p`: its centre
/// and radius, by Newton on the centre and radius from p and a guess of `size` / 4.
fn touching_ball(surfaces: &[(&Surface, f64)], p: Point3, size: f64) -> Option<(Point3, f64)> {
    // three surfaces and four unknowns: the ball nearest p, its centre square to p's offset
    let (mut c, mut r) = (p, 0.25 * size);
    for _ in 0..60 {
        let f: Vec<f64> = surfaces
            .iter()
            .map(|(s, side)| s.implicit(c) - side * r)
            .collect();
        let g: Vec<Vec3> = surfaces.iter().map(|(s, _)| s.gradient(c)).collect();
        // least change: solve the 3x4 system J [dc; dr] = -f with the step square to J's null space
        let rows: Vec<[f64; 4]> = (0..3).map(|i| [g[i].x, g[i].y, g[i].z, -surfaces[i].1]).collect();
        let jjt: [[f64; 3]; 3] =
            std::array::from_fn(|i| std::array::from_fn(|j| (0..4).map(|k| rows[i][k] * rows[j][k]).sum()));
        let y = solve3(jjt, [-f[0], -f[1], -f[2]])?;
        let d: [f64; 4] = std::array::from_fn(|k| (0..3).map(|i| rows[i][k] * y[i]).sum());
        c += Vec3::new(d[0], d[1], d[2]);
        r += d[3];
        if f.iter().all(|x| x.abs() <= 1e-3 * tol::LINEAR)
            && d.iter().all(|x| x.abs() <= 1e-12 * (1.0 + size))
        {
            break;
        }
    }
    let ok = r > tol::MIN_LENGTH
        && surfaces
            .iter()
            .all(|(s, side)| (s.implicit(c) - side * r).abs() <= tol::LINEAR);
    ok.then_some((c, r))
}

/// The parameters (low, high) of the stretch of `c` between t0 and t1 (either way round a
/// periodic curve) whose middle lies nearest `through`.
fn span(c: &Curve, t0: f64, t1: f64, through: Point3) -> Option<(f64, f64)> {
    let (lo, hi) = (t0.min(t1), t0.max(t1));
    match c.domain().period {
        None => (hi > lo).then_some((lo, hi)),
        Some(p) => {
            let w = (hi - lo).rem_euclid(p);
            let a = (lo, lo + w);
            let b = (hi, hi + (p - w));
            let d = |s: (f64, f64)| c.eval(0.5 * (s.0 + s.1)).distance(through);
            Some(if d(a) <= d(b) { a } else { b })
        }
    }
}

/// The curve where surface `blend` meets `s` from `a` to `b`, with their parameters, through
/// the stretch nearer `through`.
fn cut_through(
    blend: &Surface,
    s: &Surface,
    a: Point3,
    b: Point3,
    through: Point3,
) -> Option<(Curve, f64, f64)> {
    let region = Box3::from_points([a, b, through]).grow(a.distance(b) + tol::LINEAR);
    let curves = closed_form(blend, s).or_else(|| {
        forge_geom::intcurve::trace(blend, s, region).map(|cs| {
            cs.into_iter()
                .map(|c| Curve::Intersection(std::sync::Arc::new(c)))
                .collect()
        })
    })?;
    for c in curves {
        let full = c.full_range(1e6);
        let (ta, tb) = (c.closest(a, full), c.closest(b, full));
        if c.eval(ta).distance(a) > tol::LINEAR || c.eval(tb).distance(b) > tol::LINEAR {
            continue;
        }
        let (lo, hi) = span(&c, ta, tb, through)?;
        // back to a-to-b order
        let a_first = (c.eval(lo).distance(a)) <= tol::LINEAR;
        return Some(if a_first { (c, lo, hi) } else { (c, hi, lo) });
    }
    None
}

/// a x = b for a 3x3 matrix, by Cramer's rule; None when it is singular.
fn solve3(a: [[f64; 3]; 3], b: [f64; 3]) -> Option<[f64; 3]> {
    let det = |m: [[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let d = det(a);
    if d.abs() <= 1e-300 {
        return None;
    }
    Some(std::array::from_fn(|k| {
        let mut m = a;
        for (row, &x) in m.iter_mut().zip(&b) {
            row[k] = x;
        }
        det(m) / d
    }))
}
