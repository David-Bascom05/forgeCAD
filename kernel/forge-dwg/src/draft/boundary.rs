//! BHATCH's boundary: the region round a picked point, found from the curves near it the way
//! AutoCAD's "pick an internal point" does. The curves are cut where they cross into edges,
//! loose ends are pruned, a ray from the point finds the nearest edge, and the face holding the
//! point is walked edge by edge, always taking the sharpest left. Closed curves wholly inside it
//! (islands) come back too, so an even-odd fill leaves them clear.

use super::curve::Curve;
use super::entity::explode;
use super::geom::{Bbox, TAU, V2, v2};
use super::intersect::{Ext, intersect};

/// A closed loop of curves, end to end in order.
pub type Loop = Vec<Curve>;

/// The boundary found: the loop round the point (counter-clockwise), and the islands inside it
/// (clockwise).
#[derive(Clone, Debug, PartialEq)]
pub struct Boundary {
    pub outer: Loop,
    pub islands: Vec<Loop>,
}

struct Edge {
    c: Curve,
    v: [usize; 2],
    alive: bool,
}

/// A direction leaving vertex `v` along edge `e` (forward from its start, or backward from its
/// end): a point a little along it, so a tangent line and arc are told apart by which way the
/// arc bends.
fn out_dir(e: &Edge, fwd: bool) -> V2 {
    let (t0, t1) = e.c.domain();
    let len = e.c.length().max(1e-300);
    let step = (len * 1e-3).min(len * 0.5);
    let (at, p) = if fwd {
        (e.c.start(), e.c.point(e.c.param_at_length(step).clamp(t0, t1)))
    } else {
        (e.c.end(), e.c.point(e.c.param_at_length(len - step).clamp(t0, t1)))
    };
    (p - at).unit().unwrap_or(v2(1.0, 0.0))
}

fn cw_from(r: V2, d: V2) -> f64 {
    let a = r.angle() - d.angle();
    let a = a.rem_euclid(TAU);
    if a <= 1e-12 { TAU } else { a }
}

/// Points along a curve for areas and inside tests.
fn samples(c: &Curve, fwd: bool, out: &mut Vec<V2>) {
    let (t0, t1) = c.domain();
    let n = match c {
        Curve::Line { .. } => 1,
        _ => 32,
    };
    for k in 0..n {
        let f = k as f64 / n as f64;
        let t = if fwd { t0 + (t1 - t0) * f } else { t1 - (t1 - t0) * f };
        out.push(c.point(t));
    }
}

fn area(pts: &[V2]) -> f64 {
    let n = pts.len();
    (0..n).map(|k| pts[k].cross(pts[(k + 1) % n])).sum::<f64>() * 0.5
}

/// Even-odd: is p inside the ring?
pub fn inside(ring: &[V2], p: V2) -> bool {
    let mut odd = false;
    let n = ring.len();
    for k in 0..n {
        let (a, b) = (ring[k], ring[(k + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) {
            let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
            if p.x < x {
                odd = !odd;
            }
        }
    }
    odd
}

struct Graph {
    edges: Vec<Edge>,
    /// Each vertex's leaving half-edges: (edge, forward).
    at: Vec<Vec<(usize, bool)>>,
}

impl Graph {
    fn build(curves: &[Curve], tol: f64) -> Graph {
        // every bounded piece, polylines as their segments
        let mut cs: Vec<Curve> = vec![];
        for c in curves {
            if !c.is_bounded() {
                continue;
            }
            cs.extend(explode(c));
        }
        let boxes: Vec<Bbox> = cs.iter().map(|c| c.bbox().grow(tol)).collect();
        let mut cuts: Vec<Vec<f64>> = vec![vec![]; cs.len()];
        // pairs whose boxes meet, by a sweep along x
        let mut order: Vec<usize> = (0..cs.len()).collect();
        order.sort_by(|&a, &b| boxes[a].min.x.total_cmp(&boxes[b].min.x));
        for (k, &i) in order.iter().enumerate() {
            for &j in &order[k + 1..] {
                if boxes[j].min.x > boxes[i].max.x {
                    break;
                }
                if !boxes[i].overlaps(&boxes[j]) {
                    continue;
                }
                let h = intersect(&cs[i], &cs[j], Ext::None);
                for x in h.points.iter().filter(|x| x.on_both()) {
                    cuts[i].push(x.ta);
                    cuts[j].push(x.tb);
                }
            }
        }
        let mut pieces: Vec<Curve> = vec![];
        for (c, ts) in cs.iter().zip(cuts.iter_mut()) {
            ts.sort_by(f64::total_cmp);
            ts.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
            if ts.is_empty() {
                pieces.push(c.clone());
            } else {
                pieces.extend(c.split_at(ts).into_iter().filter(|p| p.length() > tol));
            }
        }
        // vertices: ends within the tolerance are one
        let mut verts: Vec<V2> = vec![];
        let mut cells: std::collections::HashMap<(i64, i64), Vec<usize>> = Default::default();
        let cell = (tol * 2.0).max(1e-300);
        let mut vert = |p: V2, verts: &mut Vec<V2>| -> usize {
            let (cx, cy) = ((p.x / cell).floor() as i64, (p.y / cell).floor() as i64);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if let Some(vs) = cells.get(&(cx + dx, cy + dy)) {
                        for &v in vs {
                            if verts[v].dist(p) <= tol {
                                return v;
                            }
                        }
                    }
                }
            }
            verts.push(p);
            cells.entry((cx, cy)).or_default().push(verts.len() - 1);
            verts.len() - 1
        };
        let mut edges = vec![];
        for c in pieces {
            let a = vert(c.start(), &mut verts);
            let b = vert(c.end(), &mut verts);
            edges.push(Edge {
                c,
                v: [a, b],
                alive: true,
            });
        }
        // loose ends can't bound anything: prune them, again and again
        let mut deg = vec![0usize; verts.len()];
        for e in &edges {
            deg[e.v[0]] += 1;
            deg[e.v[1]] += 1;
        }
        loop {
            let mut changed = false;
            for e in edges.iter_mut() {
                if e.alive && (deg[e.v[0]] < 2 || deg[e.v[1]] < 2) && e.v[0] != e.v[1] {
                    e.alive = false;
                    deg[e.v[0]] -= 1;
                    deg[e.v[1]] -= 1;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let mut at = vec![vec![]; verts.len()];
        for (i, e) in edges.iter().enumerate() {
            if e.alive {
                at[e.v[0]].push((i, true));
                at[e.v[1]].push((i, false));
            }
        }
        Graph { edges, at }
    }

    /// The face on the left of half-edge `start`, walked round: its half-edges.
    fn walk(&self, start: (usize, bool)) -> Option<Vec<(usize, bool)>> {
        let mut out = vec![start];
        let mut cur = start;
        let limit = self.edges.len() * 2 + 2;
        for _ in 0..limit {
            let e = &self.edges[cur.0];
            let v = if cur.1 { e.v[1] } else { e.v[0] };
            // back along where we came from
            let r = out_dir(e, !cur.1);
            let next = self.at[v]
                .iter()
                .copied()
                .min_by(|a, b| {
                    let da = cw_from(r, out_dir(&self.edges[a.0], a.1));
                    let db = cw_from(r, out_dir(&self.edges[b.0], b.1));
                    da.total_cmp(&db)
                })?;
            if next == start {
                return Some(out);
            }
            if out.contains(&next) {
                return None;
            }
            out.push(next);
            cur = next;
        }
        None
    }

    fn ring(&self, hs: &[(usize, bool)]) -> Vec<V2> {
        let mut pts = vec![];
        for &(e, f) in hs {
            samples(&self.edges[e].c, f, &mut pts);
        }
        pts
    }

    fn curves(&self, hs: &[(usize, bool)]) -> Loop {
        hs.iter()
            .map(|&(e, f)| {
                let c = &self.edges[e].c;
                if f { c.clone() } else { c.reversed() }
            })
            .collect()
    }

    /// The vertices joined to each, numbered by component.
    fn components(&self) -> Vec<usize> {
        let n = self.at.len();
        let mut comp = vec![usize::MAX; n];
        let mut k = 0;
        for s in 0..n {
            if comp[s] != usize::MAX || self.at[s].is_empty() {
                continue;
            }
            let mut stack = vec![s];
            comp[s] = k;
            while let Some(v) = stack.pop() {
                for &(e, _) in &self.at[v] {
                    for w in self.edges[e].v {
                        if comp[w] == usize::MAX {
                            comp[w] = k;
                            stack.push(w);
                        }
                    }
                }
            }
            k += 1;
        }
        comp
    }
}

/// The boundary round `p` made by `curves` (in any order, crossing or not), or None when the
/// point isn't closed in. `tol` is the gap that still closes (ends nearer than it meet).
pub fn find(curves: &[Curve], p: V2, tol: f64) -> Option<Boundary> {
    let g = Graph::build(curves, tol);
    let bb = g
        .edges
        .iter()
        .filter(|e| e.alive)
        .fold(Bbox::EMPTY, |b, e| b.union(e.c.bbox()));
    if bb.is_empty() || !bb.contains(p) {
        return None;
    }
    let far = p + v2(bb.size() * 4.0 + 1.0, 0.0);
    let ray = Curve::line(p, far);
    // the edges the ray crosses, nearest first
    let mut hits: Vec<(f64, usize, bool)> = vec![];
    for (i, e) in g.edges.iter().enumerate() {
        if !e.alive || e.c.bbox().max.x < p.x || e.c.bbox().min.y > p.y || e.c.bbox().max.y < p.y {
            continue;
        }
        for h in intersect(&ray, &e.c, Ext::None).points {
            if !h.on_both() || h.tangent {
                continue;
            }
            let d = e.c.deriv(h.tb);
            if d.y == 0.0 {
                continue;
            }
            // p on the left: the edge goes up where the ray meets it
            hits.push((h.p.dist(p), i, d.y > 0.0));
        }
    }
    hits.sort_by(|a, b| a.0.total_cmp(&b.0));
    let comp = g.components();
    let mut tried: Vec<usize> = vec![];
    let mut outer = None;
    for (_, e, fwd) in hits {
        let c = comp[g.edges[e].v[0]];
        if tried.contains(&c) {
            continue;
        }
        let Some(hs) = g.walk((e, fwd)) else {
            continue;
        };
        let ring = g.ring(&hs);
        if area(&ring) > 0.0 && inside(&ring, p) {
            outer = Some((hs, ring, c));
            break;
        }
        // the outside of something the point is not in: look further along
        tried.push(c);
    }
    let (hs, ring, oc) = outer?;
    let in_outer: std::collections::BTreeSet<usize> = hs.iter().map(|h| h.0).collect();
    // islands: every other component inside the outer loop, by its own outline
    let mut islands = vec![];
    let mut seen: Vec<usize> = vec![oc];
    for (i, e) in g.edges.iter().enumerate() {
        if !e.alive || in_outer.contains(&i) {
            continue;
        }
        let c = comp[e.v[0]];
        if seen.contains(&c) {
            continue;
        }
        seen.push(c);
        // the component's leftmost point, on which edge
        let mut best: Option<(f64, usize, f64)> = None;
        for (j, f) in g.edges.iter().enumerate() {
            if !f.alive || comp[f.v[0]] != c {
                continue;
            }
            let (t0, t1) = f.c.domain();
            for k in 0..=32 {
                let t = t0 + (t1 - t0) * k as f64 / 32.0;
                let q = f.c.point(t);
                if best.is_none_or(|b| q.x < b.0) {
                    best = Some((q.x, j, t));
                }
            }
        }
        let Some((_, j, t)) = best else { continue };
        let q = g.edges[j].c.point(t);
        if !inside(&ring, q) {
            continue;
        }
        // round the outside: the outside (to -x there) on the left, going up
        let fwd = g.edges[j].c.deriv(t).y >= 0.0;
        if let Some(ih) = g.walk((j, fwd)) {
            let ir = g.ring(&ih);
            if !inside(&ir, p) {
                islands.push(g.curves(&ih));
            }
        }
    }
    Some(Boundary {
        outer: g.curves(&hs),
        islands,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draft::geom::v2;

    fn sq(x: f64, y: f64, s: f64) -> Vec<Curve> {
        let p = [v2(x, y), v2(x + s, y), v2(x + s, y + s), v2(x, y + s)];
        (0..4).map(|k| Curve::line(p[k], p[(k + 1) % 4])).collect()
    }

    #[test]
    fn a_square_from_crossing_lines() {
        // four lines that overshoot their corners, a dangling line, a circle island
        let mut cs = vec![
            Curve::line(v2(-1.0, 0.0), v2(11.0, 0.0)),
            Curve::line(v2(10.0, -1.0), v2(10.0, 11.0)),
            Curve::line(v2(11.0, 10.0), v2(-1.0, 10.0)),
            Curve::line(v2(0.0, 11.0), v2(0.0, -1.0)),
            Curve::line(v2(5.0, 10.0), v2(5.0, 7.0)),
            Curve::circle(v2(7.0, 3.0), 1.0),
        ];
        cs.extend(sq(30.0, 30.0, 5.0));
        let b = find(&cs, v2(2.0, 2.0), 1e-9).unwrap();
        // the top cut where the loose line met it
        assert_eq!(b.outer.len(), 5);
        let len: f64 = b.outer.iter().map(Curve::length).sum();
        assert!((len - 40.0).abs() < 1e-9, "{len}");
        assert_eq!(b.islands.len(), 1);
        // inside the island: the circle itself
        let c = find(&cs, v2(7.0, 3.2), 1e-9).unwrap();
        assert_eq!(c.outer.len(), 1);
        assert!(c.islands.is_empty());
        // outside everything
        assert!(find(&cs, v2(20.0, 20.0), 1e-9).is_none());
    }

    #[test]
    fn a_region_cut_by_an_arc() {
        // a square with an arc across its corner: the point in the corner's lens
        let mut cs = sq(0.0, 0.0, 10.0);
        cs.push(Curve::arc(v2(10.0, 10.0), 4.0, 0.0, TAU));
        let b = find(&cs, v2(9.0, 9.0), 1e-9).unwrap();
        assert_eq!(b.outer.len(), 3);
        let ring: Vec<V2> = {
            let mut v = vec![];
            for c in &b.outer {
                samples(c, true, &mut v);
            }
            v
        };
        let a = area(&ring);
        assert!((a - std::f64::consts::PI * 4.0).abs() < 0.05, "{a}");
        // the rest of the square: an L round the lens
        let r = find(&cs, v2(2.0, 2.0), 1e-9).unwrap();
        assert_eq!(r.outer.len(), 5);
    }
}
