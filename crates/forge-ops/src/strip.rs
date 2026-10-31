//! Fillets that take a face away. A face fillet rounds between two sets of faces: where they
//! meet it rounds the edges between, and where a strip face (a chamfer, a land) joins them it
//! rolls one ball touching both sides and the strip goes. A full round takes a face between
//! two sides away with the ball that touches all three, its radius changing where the sides
//! close in.

use crate::fillet::{FilletEdge, FilletError, Radius, chain_loops, fillet, inside_face, nearest_hit};
use forge_geom::blend::{Blend, closed_form, foot, offset, spine, spine_closest};
use forge_geom::varblend::{Guide, VarBlend};
use forge_geom::{Circle, Curve, CurveGeom, Cylinder, Surface, SurfaceGeom, Torus};
use forge_math::{Box3, Frame, Interval, Point3, Vec3, dmath, tol};
use forge_topo::{Body, CoedgeId, EdgeId, FaceId, LoopId, Name, VertexId};
use std::collections::BTreeSet;

type R<T> = Result<T, FilletError>;

/// Rounds between faces `first` and faces `second` with radius `radius`: the edges where the
/// two sets meet, or else the one strip face joining them, taken away by the ball.
pub fn face_fillet(body: &Body, first: &[FaceId], second: &[FaceId], radius: f64, feature: u64) -> R<Body> {
    let body = body.compact_if_dead();
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
    // a strip face: neither set's, touching both
    let touches = |f: FaceId, set: &BTreeSet<FaceId>| {
        body.face(f).loops.iter().any(|&l| {
            body.lp(l).coedges.iter().any(|&c| {
                body.partner(c)
                    .is_some_and(|p| set.contains(&body.coedge_face(p)))
            })
        })
    };
    // a strip: four sides, the two sets across from each other
    let across = |f: FaceId| {
        let l = &body.face(f).loops;
        let c = &body.lp(l[0]).coedges;
        let side = |i: usize, set: &BTreeSet<FaceId>| {
            body.partner(c[i])
                .is_some_and(|p| set.contains(&body.coedge_face(p)))
        };
        l.len() == 1
            && c.len() == 4
            && (0..2).any(|i| (side(i, &a) && side(i + 2, &b)) || (side(i, &b) && side(i + 2, &a)))
    };
    let strips: Vec<FaceId> = body
        .face_ids()
        .filter(|f| !a.contains(f) && !b.contains(f) && touches(*f, &a) && touches(*f, &b) && across(*f))
        .collect();
    let [m] = strips[..] else {
        return Err(FilletError::NoBlend { edge: EdgeId(0) });
    };
    let side = |set: &BTreeSet<FaceId>| {
        body.lp(body.face(m).loops[0])
            .coedges
            .iter()
            .filter_map(|&c| body.partner(c))
            .map(|p| body.coedge_face(p))
            .find(|f| set.contains(f))
    };
    let (Some(fa), Some(fb)) = (side(&a), side(&b)) else {
        return Err(FilletError::NoBlend { edge: EdgeId(0) });
    };
    strip(&body, fa, m, fb, Some(radius), feature)
}

/// Takes face `middle` away between faces `side` and `other` with the ball touching all three:
/// one blend face over it, a cylinder or torus where the radius holds along it, else a canal
/// surface whose radius is solved with each ball. A middle closing round on itself (a ring's
/// top) rounds all the way round, with constant radius.
pub fn full_round(body: &Body, side: FaceId, middle: FaceId, other: FaceId, feature: u64) -> R<Body> {
    strip(&body.compact_if_dead(), side, middle, other, None, feature)
}

trait CompactIfDead {
    fn compact_if_dead(&self) -> Body;
}

impl CompactIfDead for Body {
    fn compact_if_dead(&self) -> Body {
        if self.dead > 0 {
            self.compact()
        } else {
            self.clone()
        }
    }
}

/// One end of the strip: the edge to its end face, the coedges either side of it there, and
/// the strip's corners on side a and side b.
struct End {
    face: FaceId,
    /// The end face's coedge on the strip's edge, and the end face's coedges on its edges to a and b.
    on_strip: CoedgeId,
    to_a: CoedgeId,
    to_b: CoedgeId,
    va: VertexId,
    vb: VertexId,
}

fn strip(body: &Body, fa: FaceId, m: FaceId, fb: FaceId, radius: Option<f64>, feature: u64) -> R<Body> {
    let ids_ok = |f: FaceId| f.index() < body.faces.len() && body.face(f).alive;
    if !ids_ok(fa) || !ids_ok(fb) || !ids_ok(m) {
        return Err(FilletError::NoBlend { edge: EdgeId(0) });
    }
    match body.face(m).loops.len() {
        1 => {}
        2 => return closed_strip(body, fa, m, fb, radius, feature),
        _ => return Err(FilletError::NoBlend { edge: EdgeId(0) }),
    }
    let ring = body.lp(body.face(m).loops[0]).coedges.clone();
    let face_across = |c: CoedgeId| body.partner(c).map(|p| body.coedge_face(p));
    // the strip's loop: its edge to a, an end, its edge to b, an end
    let n = ring.len();
    let ia = (0..n).find(|&i| face_across(ring[i]) == Some(fa));
    let ib = (0..n).find(|&i| face_across(ring[i]) == Some(fb));
    let (Some(ia), Some(ib)) = (ia, ib) else {
        return Err(FilletError::NoBlend { edge: EdgeId(0) });
    };
    let err = FilletError::NoBlend {
        edge: body.coedge(ring[ia]).edge,
    };
    if n != 4 || (ib + 4 - ia) % 4 != 2 {
        return Err(err);
    }
    let (ca, cb) = (ring[ia], ring[ib]);
    let (ea, eb) = (body.coedge(ca).edge, body.coedge(cb).edge);
    let ends: Vec<End> = [ring[(ia + 1) % 4], ring[(ia + 3) % 4]]
        .iter()
        .map(|&c| {
            let p = body.partner(c).ok_or(err.clone())?;
            let face = body.coedge_face(p);
            let lp = &body.lp(body.coedge(p).lp).coedges;
            let k = lp.iter().position(|&x| x == p).expect("in its loop");
            let (prev, next) = (lp[(k + lp.len() - 1) % lp.len()], lp[(k + 1) % lp.len()]);
            let across = |x: CoedgeId| face_across(x);
            let (to_a, to_b) = match (across(prev), across(next)) {
                (Some(x), Some(y)) if x == fa && y == fb => (prev, next),
                (Some(x), Some(y)) if x == fb && y == fa => (next, prev),
                _ => return Err(err.clone()),
            };
            let shared = |x: EdgeId, y: EdgeId| {
                let (x, y) = (body.edge(x), body.edge(y));
                [x.start, x.end]
                    .into_iter()
                    .find(|v| *v == y.start || *v == y.end)
            };
            let e = body.coedge(c).edge;
            Ok(End {
                face,
                on_strip: p,
                to_a,
                to_b,
                va: shared(e, ea).ok_or(err.clone())?,
                vb: shared(e, eb).ok_or(err.clone())?,
            })
        })
        .collect::<R<_>>()?;
    let mid = |e: EdgeId| {
        let x = body.edge(e);
        x.curve.eval(x.range.mid())
    };
    let (pa, pb) = (mid(ea), mid(eb));
    let (convex, radius, blend) = round_blend(
        body,
        [fa, m, fb],
        [ea, eb],
        [pa, body.face_box(m).center(), pb],
        radius,
    )?;
    let springs = [
        blend.spring(true).ok_or(err.clone())?,
        blend.spring(false).ok_or(err.clone())?,
    ];
    let mut out = body.clone();
    let root = Name::feature(feature);
    let name = root.with_str("round").with_name(body.face(m).name);
    // per end: where each spring meets the end face, and the cut across it
    let region = body.face_box(m).hull(&body.face_box(fa)).hull(&body.face_box(fb));
    let reach = region.diagonal() + 4.0 * radius;
    let mut xs = Vec::new();
    for (k, end) in ends.iter().enumerate() {
        let es = &body.face(end.face).surface;
        let mut at = [(0.0, Point3::ORIGIN, VertexId(0)); 2];
        for (j, (spring, v, c)) in [(&springs[0], end.va, end.to_a), (&springs[1], end.vb, end.to_b)]
            .into_iter()
            .enumerate()
        {
            let vp = body.vertex(v).point;
            let t = spring.closest(vp, spring.full_range(1e6));
            let speed = spring.deriv(t).d1.length().max(1e-12);
            let d = spring.domain();
            let w = match d.period {
                Some(_) => Interval::new(t - reach / speed, t + reach / speed),
                None => Interval::new((t - reach / speed).max(d.lo), (t + reach / speed).min(d.hi)),
            };
            let (t, p) = nearest_hit(spring, w, es, vp).ok_or(FilletError::Overflow { edge: ea })?;
            let e = body.edge(body.coedge(c).edge);
            if e.curve.eval(e.curve.closest(p, e.range)).distance(p) > tol::LINEAR {
                return Err(FilletError::Overflow { edge: ea });
            }
            let vn = Name::NONE
                .with_set(vec![name, body.face(end.face).name])
                .with(j as u64);
            at[j] = (t, p, out.add_vertex(p, vn));
        }
        // the cut on the end face from a's spring to b's: the arc through the strip's end
        let strip_mid = mid(body.coedge(end.on_strip).edge);
        let cut = match &blend.var {
            Some(v) => v.cut(es, at[0].1, at[1].1),
            None => cut_through(&blend.surface, es, at[0].1, at[1].1, strip_mid),
        };
        let (curve, ta, tb) = cut.ok_or(FilletError::Overflow { edge: ea })?;
        let en = name.with_str("end").with(k as u64);
        let cut = if ta <= tb {
            (
                out.add_edge(curve, Interval::new(ta, tb), at[0].2, at[1].2, en),
                true,
            )
        } else {
            (
                out.add_edge(curve, Interval::new(tb, ta), at[1].2, at[0].2, en),
                false,
            )
        };
        xs.push((at, cut));
    }
    // the springs between the ends, each the stretch through the strip's edge on its side
    let mut spring_edges = Vec::new();
    for (j, (curve, e, f)) in [(&springs[0], ea, fa), (&springs[1], eb, fb)]
        .into_iter()
        .enumerate()
    {
        let (t0, v0) = (xs[0].0[j].0, xs[0].0[j].2);
        let (t1, v1) = (xs[1].0[j].0, xs[1].0[j].2);
        let (lo, hi) = span(curve, t0, t1, mid(e)).ok_or(FilletError::Overflow { edge: ea })?;
        let (va, vb) = if (lo - t0).abs() <= (lo - t1).abs() {
            (v0, v1)
        } else {
            (v1, v0)
        };
        let range = Interval::new(lo, hi);
        if !inside_face(body, f, curve, range) {
            return Err(FilletError::Overflow { edge: ea });
        }
        let sn = Name::NONE.with_set(vec![name, body.face(f).name]);
        spring_edges.push(out.add_edge(curve.clone(), range, va, vb, sn));
    }
    // the end faces' edges to a and b cut back to the springs
    for (end, (at, _)) in ends.iter().zip(&xs) {
        for (c, v, x) in [(end.to_a, end.va, at[0]), (end.to_b, end.vb, at[1])] {
            let e = body.coedge(c).edge;
            let edge = &mut out.edges[e.index()];
            let t = edge.curve.closest(x.1, edge.range);
            if edge.start == v {
                edge.start = x.2;
                edge.range.lo = t;
            } else {
                edge.end = x.2;
                edge.range.hi = t;
            }
            edge.bbox.clear();
            if edge.range.width() <= 0.0 {
                return Err(FilletError::Overflow { edge: ea });
            }
        }
    }
    // the faces round the strip walk the new edges where they walked the strip's
    let swap: Vec<(CoedgeId, EdgeId)> = vec![
        (body.partner(ca).ok_or(err.clone())?, spring_edges[0]),
        (body.partner(cb).ok_or(err.clone())?, spring_edges[1]),
        (ends[0].on_strip, xs[0].1.0),
        (ends[1].on_strip, xs[1].1.0),
    ];
    let mut blend_uses = Vec::new();
    for (c, new) in &swap {
        let f = body.coedge_face(*c);
        let l = body.coedge(*c).lp;
        // walking the new edge the way the old one ran: from the end nearest its start
        let from = body.vertex(body.coedge_start(*c)).point;
        let ne = out.edge(*new);
        let fwd = out.vertex(ne.start).point.distance(from) <= out.vertex(ne.end).point.distance(from);
        let uses: Vec<(EdgeId, bool)> = body
            .lp(l)
            .coedges
            .iter()
            .map(|&x| {
                if x == *c {
                    (*new, fwd)
                } else {
                    (body.coedge(x).edge, body.coedge(x).forward)
                }
            })
            .collect();
        out.remove_loop(f, l);
        out.add_loop(f, &uses);
        let face = &mut out.faces[f.index()];
        face.bbox.clear();
        face.query.clear();
        blend_uses.push((*new, !fwd));
    }
    out.remove_loop(m, body.face(m).loops[0]);
    out.kill_face(m);
    let shell = body.face(fa).shell;
    let f = out.add_face(shell, blend.surface.clone(), convex, name);
    match &chain_loops(&out, &blend_uses)[..] {
        [lp] if lp.len() == 4 => {
            out.add_loop(f, lp);
        }
        _ => return Err(err),
    }
    Ok(out.compact())
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
    let lone = |l: LoopId| match body.lp(l).coedges[..] {
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
            .flat_map(|&l| body.lp(l).coedges.iter().map(|&c| body.coedge(c).edge))
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
        .with_name(body.face(m).name);
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
            Name::NONE.with_set(vec![name, body.face(f).name]).with_str("at"),
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
    let sn = |f: FaceId| Name::NONE.with_set(vec![name, body.face(f).name]);
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
            .coedges
            .iter()
            .flat_map(|&x| {
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
    Ok(out.compact())
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
