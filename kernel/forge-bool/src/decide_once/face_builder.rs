//! A8's face arrangement (research/decide-once/face-arrangement.md §1a), built directly: R3.
//!
//! Per near class plane `C`, its *segments*: every boundary edge of a face of `C` (on the line
//! `C ∩ H`, `H` the class of the face across it), and on each other class line `C ∩ K` the
//! stretches inside some face of `C` and (for a face class `K`) inside some face of `K`; a wall's
//! line is cut where it crosses the faces of `C`. Segments split where they meet; the planar
//! graph's faces are the candidate facets. A facet's holders (the faces of `C` it lies in) are
//! found by propagation: crossing a segment changes membership in exactly the faces whose
//! boundary it lies on. No interior point is needed for that.
//!
//! Milestone 1 (this file so far): segments, vertices, the graph's faces and their holders, per
//! plane, as counts, for comparison with the plane ledger's coarsening (`v2check::coarse_counts`).
//! Exact predicates throughout; speed comes later. Where a face of `C` runs past the box, its far
//! edges are dropped and the walls stand in for them: a wall's stretch inside a face bounds that
//! face (inside of the box on the bounded side), and the box's corners end the stretches.

use super::{Classes, In, NONE, Pts, crosses};
use forge_bool_core::Sgn;
use forge_math::exact::{DyPlane, DyV3, HPoint, meet3};
use forge_topo::Body;

/// A segment on a line of plane `C`: its two end points (exact), the class of the other plane
/// of its line, and for a boundary edge the face of `C` it bounds and its sense (the face on its
/// left, walking `a` to `b` with `C`'s normal toward you, when `+1`).
#[derive(Clone)]
struct Seg {
    a: HPoint,
    b: HPoint,
    line: u32,
    /// (face index in `fs`, +1 / -1), or (usize::MAX, 0) for a crossing
    bound: (usize, i32),
}

/// The axis to project plane `p` along (its normal's largest component), and whether that
/// component is negative (the projection then reverses orientation).
fn axis_of(p: &DyPlane) -> (usize, bool) {
    let n = [p.n.x.to_f64(), p.n.y.to_f64(), p.n.z.to_f64()];
    let a = (0..3)
        .max_by(|&i, &j| n[i].abs().total_cmp(&n[j].abs()).then(j.cmp(&i)))
        .unwrap();
    (a, n[a] < 0.0)
}

/// The direction of line `C ∩ K`: `n_C × n_K`.
fn dir(c: &DyPlane, k: &DyPlane) -> DyV3 {
    c.n.cross(&k.n)
}

/// The sign of `(p − q) · d` (where along a line of direction `d` `p` lies against `q`).
fn along(p: &HPoint, q: &HPoint, d: &DyV3) -> i32 {
    // (P/Wp − Q/Wq)·d, times Wp·Wq's sign: P·d Wq − Q·d Wp
    let pd = p.p.dot(d).mul(&q.w);
    let qd = q.p.dot(d).mul(&p.w);
    pd.sub(&qd).signum() * p.w.signum() * q.w.signum()
}

/// The exact mean of two points.
fn mid(a: &HPoint, b: &HPoint) -> HPoint {
    HPoint::mean(&[a, b])
}

/// Is `q` (on face `g`'s plane) inside face `g`?
fn inside(q: &HPoint, g: usize, fs: &[In], pts: &[Pts], snapped: &[DyPlane]) -> bool {
    let (a, _) = axis_of(&snapped[g]);
    super::in_face(q, &fs[g], &pts[fs[g].body], snapped, (a + 1) % 3, (a + 2) % 3)
}

/// Per class plane: (segments, distinct vertices, graph faces, faces with holders), summed over
/// near class planes; for the milestone's counts.
pub(crate) fn counts(
    bodies: &[&Body],
    fs: &[In],
    cls: &Classes,
    pts: &[Pts],
    snapped: &[DyPlane],
) -> (usize, usize, usize) {
    let nc = cls.planes.len();
    // the faces of each class, in arena order
    let mut on: Vec<Vec<usize>> = vec![Vec::new(); nc];
    for (i, &c) in cls.of_face.iter().enumerate() {
        if c != NONE {
            on[c as usize].push(i);
        }
    }
    // the face of `fs` by (body, FaceId)
    let mut gidx: Vec<Vec<usize>> = bodies.iter().map(|b| vec![usize::MAX; b.faces.len()]).collect();
    for (i, f) in fs.iter().enumerate() {
        gidx[f.body][f.face.index()] = i;
    }
    let is_wall = |k: u32| cls.walls.contains(&k);
    let (mut nseg, mut nvert, mut nfacet) = (0, 0, 0);
    for c in 0..nc {
        if on[c].is_empty() {
            continue;
        }
        let pc = &cls.planes[c];
        let mut segs: Vec<Seg> = Vec::new();
        // 1. boundary edges of the faces of C, each on C ∩ (the class across it)
        for &f in &on[c] {
            let b = bodies[fs[f].body];
            let face = b.face(fs[f].face);
            for &lp in face.loops.iter() {
                for co in b.lp(lp).coedges() {
                    let (va, vb) = (b.coedge_start(co), b.coedge_end(co));
                    let across = b
                        .partner(co)
                        .map(|p| gidx[fs[f].body][b.coedge_face(p).index()])
                        .filter(|&g| g != usize::MAX)
                        .map(|g| cls.of_face[g])
                        .unwrap_or(NONE);
                    if across == NONE {
                        // (an edge to a far face lies outside the region: the walls cut it off)
                        continue;
                    }
                    let (ha, hb) = (pts[fs[f].body].ph(va, snapped), pts[fs[f].body].ph(vb, snapped));
                    // (the loop runs with the face on its left about the face's outward normal;
                    // about C's normal that is +1 when the face is C's sense)
                    let along_c = fs[f].plane.n.dot(&pc.n).signum();
                    let sense = if b.face(fs[f].face).same_sense {
                        along_c
                    } else {
                        -along_c
                    };
                    segs.push(Seg {
                        a: ha,
                        b: hb,
                        line: across,
                        bound: (f, sense),
                    });
                }
            }
        }
        // 2. on each other line C ∩ K, the stretches inside the faces of C and of K
        for k in 0..nc as u32 {
            if k as usize == c {
                continue;
            }
            let pk = &cls.planes[k as usize];
            let d = dir(pc, pk);
            if d.is_zero() {
                continue; // parallel
            }
            // the points where the line crosses a boundary edge of a face of C or of K
            let mut ps: Vec<HPoint> = Vec::new();
            let cross_edges = |faces: &[usize], plane: &DyPlane, ps: &mut Vec<HPoint>| {
                for &f in faces {
                    let b = bodies[fs[f].body];
                    for &lp in b.face(fs[f].face).loops.iter() {
                        for co in b.lp(lp).coedges() {
                            let across = b
                                .partner(co)
                                .map(|p| gidx[fs[f].body][b.coedge_face(p).index()])
                                .filter(|&g| g != usize::MAX)
                                .map(|g| cls.of_face[g])
                                .unwrap_or(NONE);
                            let (ha, hb) = (
                                pts[fs[f].body].ph(b.coedge_start(co), snapped),
                                pts[fs[f].body].ph(b.coedge_end(co), snapped),
                            );
                            // an end on the line
                            for h in [&ha, &hb] {
                                if pk.side(h) == 0 && pc.side(h) == 0 {
                                    ps.push(h.clone());
                                }
                            }
                            if across == NONE || across == k {
                                continue;
                            }
                            let ph = &cls.planes[across as usize];
                            if let Some(x) = meet3(pc, pk, ph) {
                                // within the edge: between its ends along the edge's line
                                let de = plane.n.cross(&ph.n);
                                let (sa, sb) = (along(&x, &ha, &de), along(&x, &hb, &de));
                                if sa * sb < 0 {
                                    ps.push(x);
                                }
                            }
                        }
                    }
                }
            };
            cross_edges(&on[c], pc, &mut ps);
            // the box's corners on the line (a face of C may run past the walls)
            for &w in &cls.walls {
                if w != k
                    && let Some(x) = meet3(pc, pk, &cls.planes[w as usize])
                {
                    ps.push(x);
                }
            }
            if !is_wall(k) {
                cross_edges(&on[k as usize], pk, &mut ps);
            }
            // sorted along the line, repeats dropped
            ps.sort_by(|p, q| along(p, q, &d).cmp(&0));
            ps.dedup_by(|p, q| p.same(q));
            for w in ps.windows(2) {
                let m = mid(&w[0], &w[1]);
                let in_c = on[c].iter().any(|&f| inside(&m, f, fs, pts, snapped));
                let in_k = is_wall(k) || on[k as usize].iter().any(|&g| inside(&m, g, fs, pts, snapped));
                if in_c && in_k {
                    if is_wall(k) {
                        // a wall bounds the faces of C it cuts: the inside of the box on its left
                        // is +1
                        let wi = cls.walls.iter().position(|&w| w == k).unwrap();
                        let left = pc.n.cross(&d).dot(&pk.n).signum() * along(&w[1], &w[0], &d);
                        let sense = if (left > 0) == (cls.wall_in[wi] == Sgn::Pos) {
                            1
                        } else {
                            -1
                        };
                        for &f in &on[c] {
                            if inside(&m, f, fs, pts, snapped) {
                                segs.push(Seg {
                                    a: w[0].clone(),
                                    b: w[1].clone(),
                                    line: k,
                                    bound: (f, sense),
                                });
                            }
                        }
                    } else {
                        segs.push(Seg {
                            a: w[0].clone(),
                            b: w[1].clone(),
                            line: k,
                            bound: (usize::MAX, 0),
                        });
                    }
                }
            }
        }
        nseg += segs.len();
        // 3. the vertices: ends and crossings (distinct points)
        let mut vs: Vec<HPoint> = Vec::new();
        let add = |h: HPoint, vs: &mut Vec<HPoint>| {
            if !vs.iter().any(|x| x.same(&h)) {
                vs.push(h);
            }
        };
        for s in &segs {
            add(s.a.clone(), &mut vs);
            add(s.b.clone(), &mut vs);
        }
        for i in 0..segs.len() {
            for j in i + 1..segs.len() {
                let (s, t) = (&segs[i], &segs[j]);
                if s.line == t.line {
                    continue;
                }
                let (ks, kt) = (&cls.planes[s.line as usize], &cls.planes[t.line as usize]);
                if let Some(x) = meet3(pc, ks, kt) {
                    let (ds, dt) = (dir(pc, ks), dir(pc, kt));
                    let within = |a: &HPoint, b: &HPoint, d: &DyV3| along(&x, a, d) * along(&x, b, d) <= 0;
                    if within(&s.a, &s.b, &ds) && within(&t.a, &t.b, &dt) {
                        add(x, &mut vs);
                    }
                }
            }
        }
        nvert += vs.len();
        if std::env::var_os("FORGE_A8_SEGS").is_some() {
            for sg in &segs {
                let (a, b) = (sg.a.to_f64(), sg.b.to_f64());
                eprintln!(
                    "  c{c} seg line {} bound {:?}: {:.3?} -> {:.3?}",
                    sg.line, sg.bound, a, b
                );
            }
        }
        let k = plane_facets(c, pc, &segs, &vs, cls);
        if std::env::var_os("FORGE_A8_DETAIL").is_some() {
            eprintln!(
                "a8 built class {c}: {k} facets, {} segs, {} verts",
                segs.len(),
                vs.len()
            );
        }
        nfacet += k;
    }
    (nseg, nvert, nfacet)
}

/// A sub-edge of plane `C`'s graph: its ends (vertex indices), its line's class, and the faces
/// of `C` whose boundary it lies on with their sense along `u → w`.
struct Sub {
    u: usize,
    w: usize,
    line: u32,
    bounds: Vec<(usize, i32)>,
}

/// The 2D direction of half-edge `u → w` on line `C ∩ K`, in `C`'s projection (s, t), turned so
/// that counter-clockwise is about `C`'s normal.
fn dir2(
    pc: &DyPlane,
    pk: &DyPlane,
    u: &HPoint,
    w: &HPoint,
    s: usize,
    t: usize,
    flip: bool,
) -> (forge_math::exact::Dy, forge_math::exact::Dy) {
    let d = dir(pc, pk);
    let sg = along(w, u, &d);
    let (mut x, mut y) = (d.get(s).clone(), d.get(t).clone());
    if sg < 0 {
        x = x.neg();
        y = y.neg();
    }
    if flip {
        y = y.neg();
    }
    (x, y)
}

/// The half-plane of a 2D direction: 0 for angles in [0, π), 1 for [π, 2π).
fn half(v: &(forge_math::exact::Dy, forge_math::exact::Dy)) -> i32 {
    let (x, y) = (v.0.signum(), v.1.signum());
    if y > 0 || (y == 0 && x > 0) { 0 } else { 1 }
}

/// Counter-clockwise angle order of two 2D directions: -1, 0, 1.
fn cmp_angle(
    a: &(forge_math::exact::Dy, forge_math::exact::Dy),
    b: &(forge_math::exact::Dy, forge_math::exact::Dy),
) -> i32 {
    let (ha, hb) = (half(a), half(b));
    if ha != hb {
        return if ha < hb { -1 } else { 1 };
    }
    // a before b if a × b > 0
    let cr = a.0.mul(&b.1).sub(&a.1.mul(&b.0)).signum();
    -cr
}

/// Plane `C`'s graph from its segments: sub-edges between consecutive vertices on each segment,
/// half-edges ordered by angle at each vertex, the face cycles, holes given to the faces round
/// them, and the faces' holders by propagation from the outside. The number of faces holding a
/// face of `C`, inside the region.
#[allow(clippy::needless_range_loop)]
fn plane_facets(_c: usize, pc: &DyPlane, segs: &[Seg], vs: &[HPoint], cls: &Classes) -> usize {
    let (a, flip) = axis_of(pc);
    let (s, t) = ((a + 1) % 3, (a + 2) % 3);
    // 1. sub-edges: each segment cut at the vertices on it
    let mut subs: Vec<Sub> = Vec::new();
    for sg in segs {
        let pk = &cls.planes[sg.line as usize];
        let d = dir(pc, pk);
        let mut on: Vec<usize> = (0..vs.len())
            .filter(|&i| pk.side(&vs[i]) == 0 && along(&vs[i], &sg.a, &d) * along(&vs[i], &sg.b, &d) <= 0)
            .collect();
        // (ordered from a to b)
        let fwd = along(&sg.b, &sg.a, &d);
        on.sort_by(|&i, &j| (along(&vs[i], &vs[j], &d) * fwd).cmp(&0));
        for w in on.windows(2) {
            let (u, x) = (w[0], w[1]);
            // the same sub-edge from another segment: merge its bounds
            let hit = subs
                .iter()
                .position(|q| q.line == sg.line && ((q.u, q.w) == (u, x) || (q.u, q.w) == (x, u)));
            let i = match hit {
                Some(i) => i,
                None => {
                    subs.push(Sub {
                        u,
                        w: x,
                        line: sg.line,
                        bounds: Vec::new(),
                    });
                    subs.len() - 1
                }
            };
            if sg.bound.0 != usize::MAX {
                let sense = if subs[i].u == u { sg.bound.1 } else { -sg.bound.1 };
                if !subs[i].bounds.contains(&(sg.bound.0, sense)) {
                    subs[i].bounds.push((sg.bound.0, sense));
                }
            }
        }
    }
    let ns = subs.len();
    // 2. half-edges 2k (u → w) and 2k + 1 (w → u); per vertex, outgoing ones by angle
    let he_from = |h: usize| {
        if h.is_multiple_of(2) {
            subs[h / 2].u
        } else {
            subs[h / 2].w
        }
    };
    let he_to = |h: usize| {
        if h.is_multiple_of(2) {
            subs[h / 2].w
        } else {
            subs[h / 2].u
        }
    };
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); vs.len()];
    for h in 0..2 * ns {
        out[he_from(h)].push(h);
    }
    let d2 = |h: usize| {
        let sb = &subs[h / 2];
        dir2(
            pc,
            &cls.planes[sb.line as usize],
            &vs[he_from(h)],
            &vs[he_to(h)],
            s,
            t,
            flip,
        )
    };
    for o in out.iter_mut() {
        o.sort_by(|&x, &y| cmp_angle(&d2(x), &d2(y)).cmp(&0));
    }
    // next(h): at h's head, the outgoing edge just clockwise of h's twin (the face on h's left)
    let mut next = vec![0usize; 2 * ns];
    for h in 0..2 * ns {
        let v = he_to(h);
        let tw = h ^ 1;
        let o = &out[v];
        let i = o.iter().position(|&x| x == tw).unwrap();
        next[h] = o[(i + o.len() - 1) % o.len()];
    }
    // 3. cycles
    let mut cyc = vec![usize::MAX; 2 * ns];
    let mut cycles: Vec<Vec<usize>> = Vec::new();
    for h in 0..2 * ns {
        if cyc[h] != usize::MAX {
            continue;
        }
        let mut c = Vec::new();
        let mut x = h;
        while cyc[x] == usize::MAX {
            cyc[x] = cycles.len();
            c.push(x);
            x = next[x];
        }
        cycles.push(c);
    }
    // a cycle is counter-clockwise (an outer boundary of a bounded face) when it turns left at its
    // least vertex (by s, then t)
    let ccw: Vec<bool> = cycles
        .iter()
        .map(|c| {
            let k = (0..c.len())
                .min_by(|&i, &j| {
                    let (p, q) = (&vs[he_from(c[i])], &vs[he_from(c[j])]);
                    p.cmp_axis(q, s).cmp(&0).then(p.cmp_axis(q, t).cmp(&0))
                })
                .unwrap();
            let prev = c[(k + c.len() - 1) % c.len()];
            let (a0, a1, a2) = (&vs[he_from(prev)], &vs[he_from(c[k])], &vs[he_to(c[k])]);
            let o = forge_math::exact::orient2(a0, a1, a2, s, t);
            (if flip { -o } else { o }) > 0
        })
        .collect();
    // 4. components (by vertices), and each component's outer cycle given to the innermost
    // counter-clockwise cycle of another component round it (or to the outside)
    let mut comp: Vec<usize> = (0..vs.len()).collect();
    fn root(p: &mut [usize], x: usize) -> usize {
        let mut x = x;
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    for sb in &subs {
        let (a, b) = (root(&mut comp, sb.u), root(&mut comp, sb.w));
        comp[a.max(b)] = a.min(b);
    }
    let in_cycle = |p: &HPoint, c: &[usize]| {
        let mut k = 0;
        for &h in c {
            if crosses(p, &vs[he_from(h)], &vs[he_to(h)], s, t) {
                k += 1;
            }
        }
        k % 2 == 1
    };
    // per cycle, its face: a counter-clockwise cycle is its own face; a clockwise one belongs to
    // the face round it
    let nf = cycles.len();
    let mut face_of = vec![usize::MAX; nf];
    const OUTSIDE: usize = usize::MAX - 1;
    for (i, c) in cycles.iter().enumerate() {
        if ccw[i] {
            face_of[i] = i;
            continue;
        }
        let ci = root(&mut comp, he_from(c[0]));
        let p = &vs[he_from(c[0])];
        // the counter-clockwise cycles of other components holding the point; the innermost
        let holding: Vec<usize> = (0..nf)
            .filter(|&j| {
                ccw[j] && root(&mut comp.clone(), he_from(cycles[j][0])) != ci && in_cycle(p, &cycles[j])
            })
            .collect();
        let inner = holding.iter().copied().find(|&j| {
            holding
                .iter()
                .all(|&k| k == j || in_cycle(&vs[he_from(cycles[j][0])], &cycles[k]))
        });
        face_of[i] = inner.unwrap_or(OUTSIDE);
    }
    // 5. holders, by propagation from the outside across sub-edges
    let mut holders: Vec<Option<Vec<usize>>> = vec![None; nf];
    let mut outside_set: Vec<usize> = Vec::new();
    let mut queue: Vec<(usize, Vec<usize>)> = Vec::new();
    // seed: every half-edge whose cycle belongs to the outside
    let face = |h: usize| face_of[cyc[h]];
    for h in 0..2 * ns {
        if face(h) == OUTSIDE {
            let g = face(h ^ 1);
            let sb = &subs[h / 2];
            // crossing h's sub-edge from its left (outside) to its right
            let mut set = outside_set.clone();
            toggle(&mut set, sb, h);
            if g != OUTSIDE {
                queue.push((g, set));
            }
        }
    }
    outside_set.clear();
    while let Some((f, set)) = queue.pop() {
        if holders[f].is_some() {
            continue;
        }
        holders[f] = Some(set.clone());
        for h in 0..2 * ns {
            if face(h) == f {
                let g = face(h ^ 1);
                if g != OUTSIDE && g != f && holders[g].is_none() {
                    let mut s2 = set.clone();
                    toggle(&mut s2, &subs[h / 2], h);
                    queue.push((g, s2));
                }
            }
        }
    }
    // 6. facets: faces holding a face of C, inside the region
    let wall_ok = |f: usize| {
        cycles[f].iter().all(|&h| {
            let p = &vs[he_from(h)];
            cls.walls.iter().zip(&cls.wall_in).all(|(&w, &si)| {
                let x = cls.planes[w as usize].side(p);
                x == 0 || (x > 0) == (si == Sgn::Pos)
            })
        })
    };
    (0..nf)
        .filter(|&f| ccw[f] && holders[f].as_ref().is_some_and(|h| !h.is_empty()) && wall_ok(f))
        .count()
}

/// Crossing half-edge `h`'s sub-edge from its left to its right: leave each face of `C` whose
/// boundary it is with that face on the left, enter each with it on the right.
fn toggle(set: &mut Vec<usize>, sb: &Sub, h: usize) {
    let dir = if h.is_multiple_of(2) { 1 } else { -1 };
    for &(f, sense) in &sb.bounds {
        if sense * dir > 0 {
            set.retain(|&x| x != f);
        } else if !set.contains(&f) {
            set.push(f);
        }
    }
}
