//! Unify same domain: faces on one surface that meet along an edge become one face, and edges
//! on one curve that meet at a vertex nothing else ends at become one edge. A merged entity
//! takes the least of its pieces' names, a piece a boolean split off counting as the face or edge
//! it came from; the others are kept as aliases (`Body::joins`) so a reference to any piece still
//! finds it.
use forge_geom::{Curve, CurveGeom, Surface, SurfaceGeom};
use forge_math::{Interval, Point2, tol};
use forge_topo::{Body, CoedgeId, EdgeId, FaceId, LoopId};

/// Joins, all over `body`, faces on one plane or cylinder that meet along an edge (an edge left
/// between pieces of one face is dropped, or kept as the seam of a face round the cylinder), then
/// edges on one line or circle that meet at a vertex nothing else ends at. Removed entities stay
/// as tombstones (counted in `Body::dead`) until `Body::compact`.
pub fn unify(body: &mut Body) {
    // each face's kind and a number one surface gives all its faces (a plane's distance from the
    // origin, a cylinder's radius), so most edges are passed over without reading their faces
    let keys: Vec<(u8, f64)> = body
        .faces
        .iter()
        .map(|f| match &f.surface {
            Surface::Plane(p) => (1, p.frame.z().dot(p.frame.origin.to_vec()).abs()),
            Surface::Cylinder(c) => (2, c.radius),
            _ => (0, 0.0),
        })
        .collect();
    let mut cands: Vec<EdgeId> = (0..body.edges.len() as u32)
        .map(EdgeId)
        .filter(|&e| between_same(body, &keys, e))
        .collect();
    // by name, so the seam a wrapped face keeps does not depend on storage order
    cands.sort_unstable_by_key(|&e| body.edge(e).name);
    let mut faces: Vec<FaceId> = Vec::new();
    // (the face that went, the face it went into)
    let mut merged: Vec<(u32, u32)> = Vec::new();
    let mut w = Work::new(body);
    for &e in &cands {
        if let Some((keep, gone)) = remove_edge(body, &mut w, e) {
            faces.push(keep);
            if gone != keep {
                merged.push((gone.0, keep.0));
            }
        }
    }
    w.finish(body);
    faces.retain(|&f| body.face(f).alive);
    faces.sort_unstable();
    faces.dedup();
    // a loop left walking edges there and back (a slit between faces now one) bounds nothing
    for &f in &faces {
        let ls: Vec<LoopId> = body.face(f).loops.iter().copied().collect();
        for l in ls {
            let cs = body.lp(l).coedges();
            if cs
                .clone()
                .all(|c| body.partner(c).is_some_and(|p| body.coedge(p).lp == l))
            {
                let edges = cs.len() / 2;
                body.remove_loop(f, l);
                body.dead += edges;
            }
        }
    }
    let joined = join_through(body);
    rename_wholes(body, &merged, &joined);
    for f in faces {
        body.face_changed(f);
        outer_first(body, f);
    }
}

/// Names each face or edge joined here after what its pieces came from, so the sequential
/// regen (which splits faces and edges the n-ary one never does) and the n-ary one name a joined
/// entity alike: a face takes the least of its pieces' wholes (a piece's whole if a boolean split
/// it off, else its own name); an edge likewise, or if no piece of it was split off an edge (the
/// pieces of where two surfaces meet), a name from the faces it now lies between. A name two
/// would take leaves each the least of its pieces' own. Every piece's name is kept as an alias of
/// the whole's. `faces` and `edges` are (gone, went into) pairs, in the order joined.
fn rename_wholes(body: &mut Body, faces: &[(u32, u32)], edges: &[(u32, u32)]) {
    use forge_topo::Name;
    // (each one left, its new name, the least of its pieces' own), (each piece's name, its group)
    type Renamed = (Vec<(u32, Name, Name)>, Vec<(Name, u32)>);
    if faces.is_empty() && edges.is_empty() {
        return;
    }
    // each piece's whole, by name (open addressing on a name's low bits: names are hashes);
    // a piece recorded twice takes the lesser whole
    let np = body.joins.pieces.len();
    let mask = (2 * np + 1).next_power_of_two() - 1;
    let mut slots = vec![(Name::NONE, Name::NONE); mask + 1];
    for &(p, w) in &body.joins.pieces {
        let mut i = p.0 as usize & mask;
        while slots[i].0 != Name::NONE && slots[i].0 != p {
            i = (i + 1) & mask;
        }
        if slots[i].0 == Name::NONE || w < slots[i].1 {
            slots[i] = (p, w);
        }
    }
    let up = |n: Name| {
        let mut i = n.0 as usize & mask;
        loop {
            match slots[i] {
                (Name::NONE, _) => return None,
                (p, w) if p == n => return Some(w),
                _ => i = (i + 1) & mask,
            }
        }
    };
    // the whole's whole, and so on, back to the face or edge no boolean split: the sequential
    // regen splits pieces again where the n-ary one splits the original once, so only the
    // original is the same both ways
    let whole = |n: Name| {
        let mut w = up(n)?;
        for _ in 0..np {
            match up(w) {
                Some(x) => w = x,
                None => break,
            }
        }
        Some(w)
    };
    // per entity joined into: (the one left, its name, whether a piece had a whole), worked out
    // over the pairs and their roots; then the pieces' names to the one left's as aliases
    let rename = |pairs: &[(u32, u32)],
                  n: usize,
                  name: &dyn Fn(u32) -> Name,
                  other: &dyn Fn(u32) -> Option<Name>|
     -> Renamed {
        const NONE: u32 = u32::MAX;
        let mut parent = vec![NONE; n];
        for &(gone, kept) in pairs {
            parent[gone as usize] = kept;
        }
        let root = |mut x: u32| {
            while parent[x as usize] != NONE {
                x = parent[x as usize];
            }
            x
        };
        // (root, least whole-or-own name, least own name, any whole)
        let mut slot = vec![NONE; n];
        let mut gs: Vec<(u32, Name, Name, bool)> = Vec::new();
        let mut members: Vec<(Name, u32)> = Vec::with_capacity(pairs.len());
        let mut add = |r: u32, m: Name, gs: &mut Vec<(u32, Name, Name, bool)>| {
            let w = whole(m);
            let k = &mut slot[r as usize];
            if *k == NONE {
                *k = gs.len() as u32;
                gs.push((r, Name(u64::MAX), Name(u64::MAX), false));
            }
            let g = &mut gs[*k as usize];
            g.1 = g.1.min(w.unwrap_or(m));
            g.2 = g.2.min(m);
            g.3 |= w.is_some();
            *k
        };
        for &(gone, kept) in pairs {
            let r = root(kept);
            let k = add(r, name(gone), &mut gs);
            members.push((name(gone), k));
        }
        for k in 0..gs.len() {
            let r = gs[k].0;
            add(r, name(r), &mut gs);
            members.push((name(r), k as u32));
        }
        let mut to: Vec<(u32, Name, Name)> = gs
            .iter()
            .map(|&(r, w, own, any)| (r, if any { w } else { other(r).unwrap_or(w) }, own))
            .collect();
        // a name two would take: each keeps the least of its own
        let mut taken: Vec<Name> = to.iter().map(|t| t.1).collect();
        taken.sort_unstable();
        let twice: Vec<Name> = taken.windows(2).filter(|w| w[0] == w[1]).map(|w| w[0]).collect();
        for t in &mut to {
            if twice.binary_search(&t.1).is_ok() {
                t.1 = t.2;
            }
        }
        (to, members)
    };
    let apply = |body: &mut Body, (to, members): Renamed, faces: bool| {
        let aliases = members.iter().map(|&(n, k)| (n, to[k as usize].1));
        body.joins.aliases.extend(aliases.filter(|a| a.0 != a.1));
        for (i, n, _) in to {
            match faces {
                true => body.faces[i as usize].name = n,
                false => body.edges[i as usize].name = n,
            }
        }
    };
    let r = rename(faces, body.faces.len(), &|f| body.faces[f as usize].name, &|_| {
        None
    });
    apply(body, r, true);
    // an edge named from its faces takes their new names
    let joined = Name::NONE.with_str("joined").with(0x5e7);
    let sides = |e: u32| {
        let x = &body.edges[e as usize];
        let side = |c: CoedgeId| body.face(body.coedge_face(c)).name;
        let (a, b) = (side(x.coedges[0]), side(x.coedges[1]));
        Some(joined.with_name(a.min(b)).with_name(a.max(b)))
    };
    let r = rename(edges, body.edges.len(), &|e| body.edges[e as usize].name, &sides);
    apply(body, r, false);
}

/// Whether live edge `e` lies between two faces on one surface, facing the same way (`keys` as
/// `unify` makes them).
fn between_same(body: &Body, keys: &[(u8, f64)], e: EdgeId) -> bool {
    let x = body.edge(e);
    if !x.alive || x.coedges.len() != 2 {
        return false;
    }
    let (a, b) = (body.coedge_face(x.coedges[0]), body.coedge_face(x.coedges[1]));
    let (ka, kb) = (keys[a.index()], keys[b.index()]);
    // (far looser than the test below: faces on one surface differ by tolerances here)
    if a == b || ka.0 == 0 || ka.0 != kb.0 || (ka.1 - kb.1).abs() > 1e-3 {
        return false;
    }
    let (fa, fb) = (body.face(a), body.face(b));
    // on one surface, facing the same way: a plane's outward normal is its z or -z; a cylinder's
    // natural normal points away from its axis whichever way its frame turns
    let parallel = |u: forge_math::UnitVec3, v: forge_math::UnitVec3| {
        let c = u.cross(v);
        c.dot(c) <= tol::ANGULAR * tol::ANGULAR
    };
    let same = match (&fa.surface, &fb.surface) {
        (Surface::Plane(p), Surface::Plane(q)) => {
            parallel(p.frame.z(), q.frame.z())
                && (p.frame.z().dot(q.frame.z()) > 0.0) == (fa.same_sense == fb.same_sense)
                && p.frame.z().dot(q.frame.origin - p.frame.origin).abs() <= tol::LINEAR
        }
        (Surface::Cylinder(p), Surface::Cylinder(q)) => {
            let off = q.frame.origin - p.frame.origin;
            fa.same_sense == fb.same_sense
                && (p.radius - q.radius).abs() <= tol::LINEAR
                && parallel(p.frame.z(), q.frame.z())
                && {
                    let c = p.frame.z().cross(off);
                    c.dot(c) <= tol::LINEAR * tol::LINEAR
                }
        }
        _ => false,
    };
    // unless a feature split the face there on purpose
    same && !body
        .joins
        .kept
        .iter()
        .any(|&k| x.name == k || x.name.is_piece_of(k))
}

/// Whether the closed chain of edge uses `chain` goes round face `f`'s periodic surface.
fn winds(body: &Body, f: FaceId, chain: &[(EdgeId, bool)]) -> bool {
    let s = &body.face(f).surface;
    let d = s.domain();
    let mut pts: Vec<Point2> = Vec::new();
    for &(e, fwd) in chain {
        let x = body.edge(e);
        // enough samples that no step goes half way round
        let n = match x.curve {
            Curve::Line(_) => 1,
            Curve::Circle(_) | Curve::Ellipse(_) => 4,
            _ => forge_topo::uv::default_segments(x),
        };
        for i in 0..n {
            let k = if fwd { i } else { n - i };
            let t = x.range.lo + x.range.width() * (k as f64 / n as f64);
            pts.push(s.project(x.curve.eval(t.min(x.range.hi))));
        }
    }
    // round the chain and back to its start, each step unwrapped
    let mut turn = (0.0, 0.0);
    for k in 0..pts.len() {
        let (p, q) = (pts[k], pts[(k + 1) % pts.len()]);
        let q = forge_topo::uv::unwrap_near(body, f, q, p);
        turn = (turn.0 + q.x - p.x, turn.1 + q.y - p.y);
    }
    let half = |p: Option<f64>| p.map_or(f64::INFINITY, |p| 0.5 * p);
    turn.0.abs() > half(d.u.period) || turn.1.abs() > half(d.v.period)
}

fn periodic(s: &Surface) -> bool {
    let d = s.domain();
    d.u.period.is_some() || d.v.period.is_some()
}

/// The loops faces are being joined in, held apart from the body's (whose loops are each a run
/// of coedges) until every join is done: each coedge's loop now, and the coedges of each loop
/// changed. Loops made here have ids from `MADE` on, clear of any the body gives out meanwhile.
struct Work {
    lp: Vec<u32>,
    loops: Vec<Option<Vec<CoedgeId>>>,
    /// The face and coedges of each loop made here.
    made: Vec<(FaceId, Vec<CoedgeId>)>,
    /// The edges taken out.
    dropped: Vec<EdgeId>,
}

const MADE: u32 = 1 << 31;

impl Work {
    fn new(body: &Body) -> Work {
        Work {
            lp: body.coedges.iter().map(|c| c.lp.0).collect(),
            loops: vec![None; body.loops.len()],
            made: Vec::new(),
            dropped: Vec::new(),
        }
    }

    fn face(&self, body: &Body, l: u32) -> FaceId {
        match l.checked_sub(MADE) {
            Some(k) => self.made[k as usize].0,
            None => body.loops[l as usize].face,
        }
    }

    fn set_face(&mut self, body: &mut Body, l: u32, f: FaceId) {
        match l.checked_sub(MADE) {
            Some(k) => self.made[k as usize].0 = f,
            None => body.loops[l as usize].face = f,
        }
    }

    /// Loop `l`'s coedges, copied from the body the first time.
    fn get(&mut self, body: &Body, l: u32) -> &mut Vec<CoedgeId> {
        match l.checked_sub(MADE) {
            Some(k) => &mut self.made[k as usize].1,
            None => self.loops[l as usize].get_or_insert_with(|| body.lp(LoopId(l)).coedges().collect()),
        }
    }

    /// A new loop of face `f`, last among its loops.
    fn add(&mut self, body: &mut Body, f: FaceId, cs: Vec<CoedgeId>) {
        let l = MADE + self.made.len() as u32;
        for &c in &cs {
            self.lp[c.index()] = l;
        }
        self.made.push((f, cs));
        body.faces[f.index()].loops.push(LoopId(l));
    }

    /// Each loop changed made in the body, in its place among its face's loops (a loop no face
    /// holds now had its coedges taken by others, or only those of edges taken out).
    fn finish(self, body: &mut Body) {
        for (l, cs) in self.loops.iter().enumerate() {
            let Some(cs) = cs else { continue };
            let id = LoopId(l as u32);
            let f = body.loops[l].face;
            if body.faces[f.index()].loops.contains(&id) {
                body.relink_loop(Some(id), f, cs);
            }
        }
        for (k, (f, cs)) in self.made.iter().enumerate() {
            let id = LoopId(MADE + k as u32);
            if let Some(at) = body.faces[f.index()].loops.iter().position(|&x| x == id) {
                body.faces[f.index()].loops[at] = body.relink_loop(None, *f, cs);
            }
        }
        for e in self.dropped {
            body.edges[e.index()].coedges.retain(|_| false);
        }
    }
}

/// Removes edge `e`, which lies between faces on one surface (or is now used twice by one face),
/// if that leaves a valid face; returns the face that holds both sides afterwards and the one
/// that went into it (the same face if there was one). Loops are spliced in `w`: only the two
/// uses of `e` (and of edges walked there and straight back) go.
fn remove_edge(body: &mut Body, w: &mut Work, e: EdgeId) -> Option<(FaceId, FaceId)> {
    let x = body.edge(e);
    if !x.alive || x.coedges.len() != 2 {
        return None;
    }
    let (ca, cb) = (x.coedges[0], x.coedges[1]);
    let (la, lb) = (w.lp[ca.index()], w.lp[cb.index()]);
    let (fa, fb) = (w.face(body, la), w.face(body, lb));
    let wraps = periodic(&body.face(fa).surface);
    let use_of = |c: CoedgeId| (body.coedge(c).edge, body.coedge(c).forward);
    if fa == fb {
        if la != lb {
            return None;
        }
        let cs = w.get(body, la);
        let n = cs.len();
        let pos = |c: CoedgeId| cs.iter().position(|&d| d == c).expect("in its loop");
        let (ia, ib) = (pos(ca), pos(cb));
        let (i, j) = (ia.min(ib), ia.max(ib));
        if j == i + 1 || (i == 0 && j == n - 1) {
            // walked there and straight back: a spur
            if n == 2 {
                return None;
            }
            cs.retain(|&c| c != ca && c != cb);
        } else {
            let inner = cs[i + 1..j].to_vec();
            let chain: Vec<(EdgeId, bool)> = inner.iter().map(|&c| use_of(c)).collect();
            if wraps && winds(body, fa, &chain) {
                // the seam of a face round a periodic surface: kept
                return None;
            }
            // a bridge between two loops of the face
            cs.drain(i..=j);
            w.add(body, fa, inner);
        }
        body.edges[e.index()].alive = false;
        w.dropped.push(e);
        body.dead += 1;
        return Some((fa, fa));
    }
    let sb = w.get(body, lb).clone();
    let sa = w.get(body, la);
    let (ia, ib) = (
        sa.iter().position(|&d| d == ca).expect("in its loop"),
        sb.iter().position(|&d| d == cb).expect("in its loop"),
    );
    // b's loop from just after its use of e round to just before it, then a's likewise
    let mut joined: Vec<CoedgeId> = Vec::with_capacity(sa.len() + sb.len() - 2);
    joined.extend_from_slice(&sb[ib + 1..]);
    joined.extend_from_slice(&sb[..ib]);
    let mut mid = joined.len();
    joined.extend_from_slice(&sa[ia + 1..]);
    joined.extend_from_slice(&sa[..ia]);
    // edges walked there and straight back where the two loops join (the faces shared more
    // than one edge in a row): at the middle and round the wrap
    let mut gone_edges = vec![e];
    let back = |c: CoedgeId, d: CoedgeId| {
        let (u, v) = (use_of(c), use_of(d));
        (u.0 == v.0 && u.1 != v.1).then_some(u.0)
    };
    while mid > 0 && mid < joined.len() {
        let Some(d) = back(joined[mid - 1], joined[mid]) else {
            break;
        };
        gone_edges.push(d);
        joined.drain(mid - 1..=mid);
        mid -= 1;
    }
    while joined.len() > 1 {
        let Some(d) = back(joined[joined.len() - 1], joined[0]) else {
            break;
        };
        gone_edges.push(d);
        joined.pop();
        joined.remove(0);
    }
    if joined.is_empty() && wraps {
        // the faces meet only along a whole loop round the surface: one face would have no seam
        return None;
    }
    // nor a face between a thread's turns, where it winds round more than once: none along a
    // swept surface
    if wraps
        && joined.iter().any(|&c| {
            body.partner(c).is_some_and(|p| {
                matches!(
                    body.face(w.face(body, w.lp[p.index()])).surface,
                    Surface::Procedural(_)
                )
            })
        })
    {
        return None;
    }
    // the face with more loops holds the merge
    let (keep, gone, lk, lg) = if body.face(fa).loops.len() >= body.face(fb).loops.len() {
        (fa, fb, la, lb)
    } else {
        (fb, fa, lb, la)
    };
    for &d in &gone_edges {
        body.edges[d.index()].alive = false;
    }
    w.dropped.extend(gone_edges);
    let moved: Vec<LoopId> = body.faces[gone.index()]
        .loops
        .iter()
        .copied()
        .filter(|&l| l.0 != lg)
        .collect();
    for &l in &moved {
        w.set_face(body, l.0, keep);
        body.faces[keep.index()].loops.push(l);
    }
    body.faces[gone.index()].loops.retain(|_| false);
    if joined.is_empty() {
        body.faces[keep.index()].loops.retain(|&l| l.0 != lk);
    } else {
        for &c in &joined {
            w.lp[c.index()] = lk;
        }
    }
    *w.get(body, lk) = joined;
    w.get(body, lg).clear();
    body.faces[gone.index()].alive = false;
    body.dead += 1;
    Some((keep, gone))
}

/// Puts plane face `f`'s outer loop first (the boolean's local split relies on it).
fn outer_first(body: &mut Body, f: FaceId) {
    let face = body.face(f);
    let Surface::Plane(p) = &face.surface else { return };
    let at = |q: forge_math::Point3| {
        let d = q - p.frame.origin;
        (p.frame.x().dot(d), p.frame.y().dot(d))
    };
    // twice the signed area about the plane's z: chords, plus the bulge of each circular arc
    let area = |l: LoopId| -> f64 {
        let mut a = 0.0;
        for c in body.lp(l).coedges() {
            let co = body.coedge(c);
            let x = body.edge(co.edge);
            let (s0, s1) = (
                at(body.vertex(body.coedge_start(c)).point),
                at(body.vertex(body.coedge_end(c)).point),
            );
            if let Curve::Line(_) | Curve::Circle(_) = x.curve {
                a += s0.0 * s1.1 - s1.0 * s0.1;
            } else {
                let mut pts: Vec<(f64, f64)> = body
                    .edge_points(co.edge, forge_topo::uv::default_segments(x))
                    .into_iter()
                    .map(at)
                    .collect();
                if !co.forward {
                    pts.reverse();
                }
                a += pts
                    .windows(2)
                    .map(|w| w[0].0 * w[1].1 - w[1].0 * w[0].1)
                    .sum::<f64>();
            }
            if let Curve::Circle(ci) = &x.curve {
                let w = x.range.width();
                let ccw = (ci.frame.z().dot(p.frame.z()) > 0.0) == co.forward;
                let bulge = ci.radius * ci.radius * (w - forge_math::dmath::sin(w));
                a += if ccw { bulge } else { -bulge };
            }
        }
        a
    };
    let outer = |l: LoopId| (area(l) > 0.0) == face.same_sense;
    if face.loops.len() < 2 || outer(face.loops[0]) {
        return;
    }
    if let Some(k) = face.loops.iter().position(|&l| outer(l)) {
        body.faces[f.index()].loops.swap(0, k);
    }
}

/// Joins each two edges that are all that ends at a vertex, where they lie on one curve (a vertex
/// a joined face left, or one a boolean left on a line or circle). Returns (gone, went into) pairs.
fn join_through(body: &mut Body) -> Vec<(u32, u32)> {
    // the edges ending at each vertex, while there are at most two
    let mut at: Vec<(u8, [EdgeId; 2])> = vec![(0, [EdgeId(0); 2]); body.vertices.len()];
    for (i, x) in body.edges.iter().enumerate().filter(|x| x.1.alive) {
        let e = EdgeId(i as u32);
        for v in [x.start, x.end] {
            let a = &mut at[v.index()];
            if a.0 < 2 {
                a.1[a.0 as usize] = e;
            }
            a.0 = a.0.saturating_add(1);
        }
    }
    let mut joined = Vec::new();
    // the loops that held an edge joined away
    let mut gone_loops: Vec<LoopId> = Vec::new();
    for v in 0..at.len() {
        let (2, [e1, e2]) = at[v] else { continue };
        if e1 == e2 {
            continue;
        }
        let Some(&c) = body
            .edge(e1)
            .coedges
            .iter()
            .find(|&&c| body.coedge_end(c).index() == v)
        else {
            continue;
        };
        let n = next(body, c);
        if body.coedge(n).edge != e2 || !join(body, c, n) {
            continue;
        }
        joined.push((e2.0, e1.0));
        gone_loops.extend(body.edge(e2).coedges.iter().map(|&d| body.coedge(d).lp));
        // e2 went into e1: its far vertex now ends e1
        let x = body.edge(e1);
        for w in [x.start, x.end] {
            for e in &mut at[w.index()].1 {
                if *e == e2 {
                    *e = e1;
                }
            }
        }
    }
    gone_loops.sort_unstable();
    gone_loops.dedup();
    for l in gone_loops {
        body.retain_loop(l, |b, c| b.edge(b.coedge(c).edge).alive);
        let f = body.lp(l).face;
        body.faces[f.index()].query.clear();
    }
    joined
}

/// Joins the edges of coedge `c` and the next one in its loop, `n`, at the vertex between them,
/// when nothing else ends there and they lie on one curve. The first edge carries on; the
/// second goes.
fn join(body: &mut Body, c: CoedgeId, n: CoedgeId) -> bool {
    let v = body.coedge_end(c);
    let (e1, e2) = (body.coedge(c).edge, body.coedge(n).edge);
    if e1 == e2 {
        return false;
    }
    let (x1, x2) = (body.edge(e1), body.edge(e2));
    if x1.start == x1.end || x2.start == x2.end || x1.coedges.len() != 2 || x2.coedges.len() != 2 {
        return false;
    }
    // on one line or circle (cheap tests first; the walk round v and the evaluation settle it)
    let same_kind = match (&x1.curve, &x2.curve) {
        (Curve::Line(a), Curve::Line(b)) => {
            let c = a.dir.cross(b.dir);
            c.dot(c) <= tol::ANGULAR * tol::ANGULAR
        }
        (Curve::Circle(a), Curve::Circle(b)) => {
            (a.radius - b.radius).abs() <= tol::LINEAR
                && a.frame.origin.distance(b.frame.origin) <= tol::LINEAR
        }
        // pieces of one other curve: only where their parameter ranges abut, not across the
        // start of a closed curve
        (a, b) => {
            let abut = |t: f64, u: f64| (t - u).abs() <= 1e-9 * (1.0 + t.abs());
            a == b
                && if x1.end == v {
                    x2.start == v && abut(x2.range.lo, x1.range.hi)
                } else {
                    x2.end == v && abut(x2.range.hi, x1.range.lo)
                }
        }
    };
    if !same_kind {
        return false;
    }
    // v has only these two edges when the other side also passes straight through it
    let (Some(pc), Some(pn)) = (body.partner(c), body.partner(n)) else {
        return false;
    };
    if next(body, pn) != pc {
        return false;
    }
    // a pole stays a vertex: a face's loop is drawn round it there
    let at = body.vertex(v).point;
    if [c, pc].iter().any(|&d| {
        let s = &body.face(body.coedge_face(d)).surface;
        !matches!(s, Surface::Plane(_) | Surface::Cylinder(_)) && s.singular(s.project(at))
    }) {
        return false;
    }
    // e1's curve carried on past v by e2's length (both unit-speed lines, or circles of one radius)
    let far = if x2.start == v { x2.end } else { x2.start };
    let w = x2.range.width();
    let (range, t_mid) = if x1.end == v {
        (Interval::new(x1.range.lo, x1.range.hi + w), x1.range.hi + 0.5 * w)
    } else {
        (Interval::new(x1.range.lo - w, x1.range.hi), x1.range.lo - 0.5 * w)
    };
    let t_far = if x1.end == v { range.hi } else { range.lo };
    let reach = body.vertex(far).tol.max(tol::LINEAR);
    // (one curve, its ranges meeting at v, needs no evaluating)
    let gap = if x1.end == v {
        x2.range.lo - x1.range.hi
    } else {
        x1.range.lo - x2.range.hi
    };
    let one = x1.curve == x2.curve
        && (x1.end == v) == (x2.start == v)
        && (gap == 0.0 || x1.curve.domain().period.is_some_and(|p| gap.abs() == p));
    if !one
        && (x1.curve.eval(t_far).distance(body.vertex(far).point) > reach
            || x1.curve.eval(t_mid).distance(x2.curve.eval(x2.range.mid())) > tol::LINEAR)
    {
        return false;
    }
    let other = if x1.end == v { x1.start } else { x1.end };
    // nor an edge from a point back to it through two vertices there (a pinch's)
    let pinch = body.vertex(far).tol.max(body.vertex(other).tol).max(tol::LINEAR);
    if far != other && body.vertex(far).point.distance(body.vertex(other).point) <= pinch {
        return false;
    }
    if far == other
        && x1
            .curve
            .domain()
            .period
            .is_none_or(|p| (range.width() - p).abs() > tol::ANGULAR)
    {
        return false;
    }
    let edge = &mut body.edges[e1.index()];
    edge.range = range;
    if edge.end == v {
        edge.end = far;
    } else {
        edge.start = far;
    }
    edge.bbox.clear();
    // its uses leave their loops once every join is done
    body.edges[e2.index()].alive = false;
    body.dead += 1;
    true
}

/// The coedge after `c` in its loop, passing uses of edges joined away.
fn next(body: &Body, c: CoedgeId) -> CoedgeId {
    let l = body.lp(body.coedge(c).lp);
    let (k, n) = ((c.0 - l.coedge(0).0) as usize, l.coedges().len());
    (1..n)
        .map(|i| l.coedge((k + i) % n))
        .find(|&d| body.edge(body.coedge(d).edge).alive)
        .unwrap_or(c)
}
