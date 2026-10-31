//! Constant-radius rolling-ball fillets, all edges of one operation solved
//! together.
//!
//! Each filleted edge becomes a blend face (`forge_geom::blend`) and its two
//! faces are cut back to the blend's spring curves. What happens at each end
//! is decided per vertex, from the cycle of faces around it: a lone blend runs
//! out into the faces across its end, two blends of a tangent chain meet in a
//! shared cross-section, three equal blends meet in a sphere. Every face
//! corner the fillet touches is closed by a path of new edges, so a corner
//! may end in any trim curve.

use forge_geom::blend::{Blend, closed_form, spine_closest};
use forge_geom::intersect::{CurveSurface, curve_surface};
use forge_geom::{Curve, CurveGeom, Sphere, Surface, SurfaceGeom};
use forge_math::{Box3, Frame, Interval, Point3, Vec3, dmath, tol};
use forge_topo::{Body, CoedgeId, EdgeId, FaceId, Name, VertexId};
use std::collections::{BTreeMap, BTreeSet};

/// One edge to round, with its radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilletEdge {
    pub edge: EdgeId,
    pub radius: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FilletError {
    /// The edge is not a live edge between two faces.
    NotAnEdge { edge: EdgeId },
    /// The radius is not a usable length.
    BadRadius { edge: EdgeId },
    /// The two faces are tangent along the edge: there is nothing to round.
    Smooth { edge: EdgeId },
    /// The two faces offset by the radius do not meet near the edge.
    NoBlend { edge: EdgeId },
    /// The blend runs past the edge or face it should end on.
    Overflow { edge: EdgeId },
    /// The blend ends exactly on a vertex or edge: zero-length or zero-thickness result.
    Tangency { edge: EdgeId },
    /// Several blends meet at this vertex in a way that needs a corner patch.
    Corner { vertex: VertexId },
    /// An edge is listed twice.
    Duplicate { edge: EdgeId },
}

type R<T> = Result<T, FilletError>;

/// A filleted edge's blend and the curves it touches its faces along.
struct Rolled {
    edge: EdgeId,
    convex: bool,
    blend: Blend,
    /// Spring curves on the surface of the face using `edge.coedges[i]`; they share the spine's parameter.
    springs: [Curve; 2],
    /// The spine runs the way the edge's curve does.
    aligned: bool,
    /// The faces on each side, and whether each walks the edge forward.
    faces: [FaceId; 2],
    coedges: [CoedgeId; 2],
    forward: [bool; 2],
    name: Name,
}

/// How a filleted edge ends at one of its vertices: per side, the vertex and
/// spring parameter where the spring stops, and the path closing the blend
/// face there (from the side whose face leaves the vertex along the edge to
/// the side whose face arrives).
struct SpringEnd {
    v: [VertexId; 2],
    t: [f64; 2],
    path: Vec<(EdgeId, bool)>,
}

/// Rounds `edges` of `body`, each with its own radius, in one operation.
/// `feature` is the history id the new faces, edges and vertices are named from.
pub fn fillet(body: &Body, edges: &[FilletEdge], feature: u64) -> R<Body> {
    let mut body = if body.dead > 0 {
        body.compact()
    } else {
        body.clone()
    };
    let mut seen = BTreeSet::new();
    for fe in edges {
        if !seen.insert(fe.edge) {
            return Err(FilletError::Duplicate { edge: fe.edge });
        }
        if fe.edge.index() >= body.edges.len() || !body.edge(fe.edge).alive {
            return Err(FilletError::NotAnEdge { edge: fe.edge });
        }
    }
    merge_flush(&mut body, &seen);
    let edges = tangent_chains(&body, edges);
    let seen: BTreeSet<EdgeId> = edges.iter().map(|f| f.edge).collect();
    merge_flush(&mut body, &seen);
    let root = Name::feature(feature);
    let body = body;
    let rolled: Vec<R<Rolled>> = par_map(edges.len(), |i| roll(&body, edges[i], root));
    let rolled: Vec<Rolled> = rolled.into_iter().collect::<R<_>>()?;
    Builder::new(body, rolled, root).build()
}

/// Runs `f` over 0..n on the machine's threads, results in order.
fn par_map<T: Send>(n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let threads = std::thread::available_parallelism().map_or(1, |t| t.get()).min(n);
    if threads <= 1 {
        return (0..n).map(f).collect();
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let parts: Vec<Vec<(usize, T)>> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut got = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if i >= n {
                            break got;
                        }
                        got.push((i, f(i)));
                    }
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|w| w.join().expect("fillet worker"))
            .collect()
    });
    let mut out: Vec<Option<T>> = (0..n).map(|_| None).collect();
    for (i, t) in parts.into_iter().flatten() {
        out[i] = Some(t);
    }
    out.into_iter().map(|t| t.expect("every edge done")).collect()
}

/// The edges with every tangent continuation of each added, at the same
/// radius: a chain rounds as a whole, as a ball rolling along it would.
fn tangent_chains(body: &Body, edges: &[FilletEdge]) -> Vec<FilletEdge> {
    let mut at: BTreeMap<VertexId, Vec<EdgeId>> = BTreeMap::new();
    for e in body.edge_ids() {
        let x = body.edge(e);
        at.entry(x.start).or_default().push(e);
        if x.end != x.start {
            at.entry(x.end).or_default().push(e);
        }
    }
    // unit tangent of e at its end v, pointing out of v
    let out_of = |e: EdgeId, v: VertexId| -> Vec<Vec3> {
        let x = body.edge(e);
        let mut t = Vec::new();
        if x.start == v {
            t.push(x.curve.deriv(x.range.lo).d1);
        }
        if x.end == v {
            t.push(-x.curve.deriv(x.range.hi).d1);
        }
        t.into_iter()
            .filter_map(|d| d.normalize())
            .map(|u| u.get())
            .collect()
    };
    let mut out: Vec<FilletEdge> = edges.to_vec();
    let mut have: BTreeSet<EdgeId> = edges.iter().map(|f| f.edge).collect();
    let mut i = 0;
    while i < out.len() {
        let FilletEdge { edge, radius } = out[i];
        i += 1;
        let x = body.edge(edge);
        for v in [x.start, x.end] {
            let mine = out_of(edge, v);
            let next: Vec<EdgeId> = at[&v]
                .iter()
                .copied()
                .filter(|&o| o != edge && !have.contains(&o) && body.edge(o).coedges.len() == 2)
                .filter(|&o| {
                    let f = &body.edge(o).coedges;
                    body.coedge_face(f[0]) != body.coedge_face(f[1])
                })
                .filter(|&o| {
                    // continuing the chain: leaving v the opposite way, within LINEAR over a radius
                    out_of(o, v)
                        .iter()
                        .any(|t| mine.iter().any(|m| (*m + *t).length() * radius <= tol::LINEAR))
                })
                .collect();
            if let [o] = next[..]
                && !is_smooth(body, o, radius)
            {
                have.insert(o);
                out.push(FilletEdge { edge: o, radius });
            }
        }
    }
    out
}

/// Whether the faces either side of `e` meet tangentially at its middle (to within
/// LINEAR over the radius).
fn is_smooth(body: &Body, e: EdgeId, radius: f64) -> bool {
    let x = body.edge(e);
    let p = x.curve.eval(x.range.mid());
    let n: Vec<_> = x
        .coedges
        .iter()
        .filter_map(|&c| {
            let f = body.coedge_face(c);
            body.face_normal(f, body.face(f).surface.project(p))
        })
        .collect();
    n.len() == 2 && n[0].cross(n[1]).length() * radius <= tol::LINEAR && n[0].dot(n[1]) > 0.0
}

/// Faces on one surface that meet along an edge at a vertex of a filleted edge
/// become one face: a boolean leaves such seams, and a blend must run out
/// across the surface as a whole.
fn merge_flush(body: &mut Body, filleted: &BTreeSet<EdgeId>) {
    let mut verts = BTreeSet::new();
    for &e in filleted {
        verts.insert(body.edge(e).start);
        verts.insert(body.edge(e).end);
    }
    let candidates: Vec<EdgeId> = body
        .edge_ids()
        .filter(|&e| {
            let x = body.edge(e);
            !filleted.contains(&e) && (verts.contains(&x.start) || verts.contains(&x.end))
        })
        .collect();
    for e in candidates {
        let edge = body.edge(e);
        if !edge.alive || edge.coedges.len() != 2 {
            continue;
        }
        let (a, b) = (
            body.coedge_face(edge.coedges[0]),
            body.coedge_face(edge.coedges[1]),
        );
        if a == b || !same_surface(body, a, b) {
            continue;
        }
        merge_faces(body, e);
    }
}

/// Whether two faces lie on one plane with the same outward side. (Curved
/// faces on one surface are left split: their seams keep a periodic face's
/// boundary drawable.)
fn same_surface(body: &Body, a: FaceId, b: FaceId) -> bool {
    let (fa, fb) = (body.face(a), body.face(b));
    match (&fa.surface, &fb.surface) {
        (Surface::Plane(p), Surface::Plane(q)) => {
            let sign = if fa.same_sense == fb.same_sense { 1.0 } else { -1.0 };
            p.frame.z().cross(q.frame.z()).length() <= tol::ANGULAR
                && p.frame.z().dot(q.frame.z()) * sign > 0.0
                && p.signed_distance(q.frame.origin).abs() <= tol::LINEAR
        }
        _ => false,
    }
}

/// Joins the two faces either side of `e` into the first: their loops through
/// `e` become one, the second face's other loops move over, and edges the
/// joined loop walks there and straight back are dropped.
fn merge_faces(body: &mut Body, e: EdgeId) {
    let (ca, cb) = (body.edge(e).coedges[0], body.edge(e).coedges[1]);
    let (a, b) = (body.coedge_face(ca), body.coedge_face(cb));
    let (la, lb) = (body.coedge(ca).lp, body.coedge(cb).lp);
    let uses = |body: &Body, l: forge_topo::LoopId| -> Vec<(EdgeId, bool)> {
        body.lp(l)
            .coedges
            .iter()
            .map(|&c| (body.coedge(c).edge, body.coedge(c).forward))
            .collect()
    };
    let (ua, ub) = (uses(body, la), uses(body, lb));
    let ia = body
        .lp(la)
        .coedges
        .iter()
        .position(|&c| c == ca)
        .expect("in its loop");
    let ib = body
        .lp(lb)
        .coedges
        .iter()
        .position(|&c| c == cb)
        .expect("in its loop");
    let mut joined: Vec<(EdgeId, bool)> = ua[..ia].to_vec();
    joined.extend_from_slice(&ub[ib + 1..]);
    joined.extend_from_slice(&ub[..ib]);
    joined.extend_from_slice(&ua[ia + 1..]);
    // there-and-back pairs (the two faces shared more than one edge in a row)
    let mut changed = true;
    while changed && joined.len() > 1 {
        changed = false;
        let n = joined.len();
        for i in 0..n {
            let j = (i + 1) % n;
            if joined[i].0 == joined[j].0 && joined[i].1 != joined[j].1 {
                let (hi, lo) = (i.max(j), i.min(j));
                joined.remove(hi);
                joined.remove(lo);
                changed = true;
                break;
            }
        }
    }
    let others: Vec<Vec<(EdgeId, bool)>> = body
        .face(b)
        .loops
        .iter()
        .filter(|&&l| l != lb)
        .map(|&l| uses(body, l))
        .collect();
    body.remove_loop(a, la);
    for l in body.face(b).loops.clone() {
        body.remove_loop(b, l);
    }
    body.kill_face(b);
    if !joined.is_empty() {
        body.add_loop(a, &joined);
    }
    for o in others {
        body.add_loop(a, &o);
    }
    let face = &mut body.faces[a.index()];
    face.bbox.clear();
    face.query.clear();
}

/// The blend for one edge, from the faces on either side at its middle.
fn roll(body: &Body, fe: FilletEdge, root: Name) -> R<Rolled> {
    let e = fe.edge;
    if e.index() >= body.edges.len() || !body.edge(e).alive || body.edge(e).coedges.len() != 2 {
        return Err(FilletError::NotAnEdge { edge: e });
    }
    let r = fe.radius;
    if !(r > tol::MIN_LENGTH && r.is_finite()) {
        return Err(FilletError::BadRadius { edge: e });
    }
    let edge = body.edge(e);
    let (c0, c1) = (edge.coedges[0], edge.coedges[1]);
    let (f0, f1) = (body.coedge_face(c0), body.coedge_face(c1));
    if f0 == f1 {
        return Err(FilletError::Smooth { edge: e });
    }
    let d = edge.curve.deriv(edge.range.mid());
    let tangent = d.d1.normalize().ok_or(FilletError::NotAnEdge { edge: e })?;
    let normal = |f: FaceId| {
        let s = &body.face(f).surface;
        body.face_normal(f, s.project(d.p))
    };
    let (n0, n1) = match (normal(f0), normal(f1)) {
        (Some(a), Some(b)) => (a, b),
        _ => return Err(FilletError::NoBlend { edge: e }),
    };
    // the faces turn by less than LINEAR over one radius: tangent
    if n0.cross(n1).length() * r <= tol::LINEAR && n0.dot(n1) > 0.0 {
        return Err(FilletError::Smooth { edge: e });
    }
    // face 1 runs away from the edge along n1 x (its walking direction); convex when that dips below face 0
    let walk1 = if body.coedge(c1).forward {
        tangent.get()
    } else {
        -tangent.get()
    };
    let into1 = n1.cross(walk1);
    let convex = n0.dot(into1) < 0.0;
    let side = if convex { -1.0 } else { 1.0 };
    let (s0, s1) = (&body.face(f0).surface, &body.face(f1).surface);
    let sense = |f: FaceId| if body.face(f).same_sense { 1.0 } else { -1.0 };
    let (d0, d1) = (side * r * sense(f0), side * r * sense(f1));
    let bis = (n0.get() + n1.get()).normalize().map_or(Vec3::ZERO, |u| u.get());
    let half = ((1.0 + n0.dot(n1)) * 0.5).max(1e-6).sqrt();
    let near = d.p + bis * (side * r / half);
    let region = body.edge_box(e).grow(4.0 * r + tol::LINEAR);
    let blend = Blend::new(s0, s1, d0, d1, near, region).ok_or(FilletError::NoBlend { edge: e })?;
    let tm = spine_closest(&blend.spine, d.p);
    if blend.spine.eval(tm).distance(near) > r {
        return Err(FilletError::NoBlend { edge: e });
    }
    let aligned = blend.spine.deriv(tm).d1.dot(tangent.get()) > 0.0;
    let springs = [
        blend.spring(true).ok_or(FilletError::NoBlend { edge: e })?,
        blend.spring(false).ok_or(FilletError::NoBlend { edge: e })?,
    ];
    Ok(Rolled {
        edge: e,
        convex,
        blend,
        springs,
        aligned,
        faces: [f0, f1],
        coedges: [c0, c1],
        forward: [body.coedge(c0).forward, body.coedge(c1).forward],
        name: root.with_str("fillet").with_name(edge.name),
    })
}

/// One face corner at a vertex: the face, the coedge arriving and the one leaving.
#[derive(Clone, Copy, Debug)]
struct Corner {
    face: FaceId,
    cin: CoedgeId,
    cout: CoedgeId,
}

/// A new face closing a corner: surface, same sense, name, its loop.
type Patch = (Surface, bool, Name, Vec<(EdgeId, bool)>);

struct Builder {
    body: Body,
    rolled: Vec<Rolled>,
    root: Name,
    /// Rolled index of each filleted edge.
    which: BTreeMap<EdgeId, usize>,
    /// Position of each coedge in its loop.
    pos: Vec<u32>,
    ends: BTreeMap<(EdgeId, bool), SpringEnd>,
    /// New end (vertex, parameter) of an unfilleted edge cut back at one end (true: its end).
    trims: BTreeMap<(EdgeId, bool), (VertexId, f64)>,
    /// Edges to insert after an arriving coedge, closing that face corner.
    paths: BTreeMap<CoedgeId, Vec<(EdgeId, bool)>>,
    touched: BTreeSet<FaceId>,
    /// New faces other than the blends (corner patches).
    patches: Vec<Patch>,
}

impl Builder {
    fn new(body: Body, rolled: Vec<Rolled>, root: Name) -> Builder {
        let which = rolled.iter().enumerate().map(|(i, r)| (r.edge, i)).collect();
        let mut pos = vec![0u32; body.coedges.len()];
        for l in body.loop_ids() {
            for (k, &c) in body.lp(l).coedges.iter().enumerate() {
                pos[c.index()] = k as u32;
            }
        }
        Builder {
            body,
            rolled,
            root,
            which,
            pos,
            ends: BTreeMap::new(),
            trims: BTreeMap::new(),
            paths: BTreeMap::new(),
            touched: BTreeSet::new(),
            patches: Vec::new(),
        }
    }

    fn build(mut self) -> R<Body> {
        // each vertex with a coedge arriving there
        let mut verts: BTreeMap<VertexId, CoedgeId> = BTreeMap::new();
        for r in &self.rolled {
            let e = self.body.edge(r.edge);
            let c = e.coedges[0];
            let back = self.body.edge(r.edge).coedges[1];
            verts.entry(self.body.coedge_end(c)).or_insert(c);
            verts.entry(self.body.coedge_end(back)).or_insert(back);
        }
        for (v, c) in verts {
            self.resolve(v, c)?;
        }
        let springs = self.make_springs()?;
        self.trim_edges()?;
        self.rebuild_faces(&springs);
        self.add_blends(&springs);
        for (surface, same_sense, name, uses) in std::mem::take(&mut self.patches) {
            let shell = self.body.faces[0].shell;
            let f = self.body.add_face(shell, surface, same_sense, name);
            self.body.add_loop(f, &uses);
        }
        Ok(self.body.compact())
    }

    fn next_in_loop(&self, c: CoedgeId) -> CoedgeId {
        let l = self.body.lp(self.body.coedge(c).lp);
        l.coedges[(self.pos[c.index()] as usize + 1) % l.coedges.len()]
    }

    /// The face corners around `v`, in order: each one's leaving edge is the next one's arriving edge.
    fn corners(&self, start: CoedgeId) -> Option<Vec<Corner>> {
        let b = &self.body;
        let mut out = Vec::new();
        let mut cin = start;
        loop {
            let cout = self.next_in_loop(cin);
            out.push(Corner {
                face: b.coedge_face(cin),
                cin,
                cout,
            });
            cin = b.partner(cout)?;
            if cin == start {
                return Some(out);
            }
            if out.len() > 64 {
                return None;
            }
        }
    }

    fn filleted(&self, c: CoedgeId) -> Option<usize> {
        self.which.get(&self.body.coedge(c).edge).copied()
    }

    /// Which side (index into the edge's coedges) a coedge is.
    fn side(&self, c: CoedgeId) -> usize {
        let e = self.body.edge(self.body.coedge(c).edge);
        usize::from(e.coedges[0] != c)
    }

    fn resolve(&mut self, v: VertexId, start: CoedgeId) -> R<()> {
        let ring = self.corners(start).ok_or(FilletError::Corner { vertex: v })?;
        for k in &ring {
            self.touched.insert(k.face);
        }
        let hot: Vec<usize> = (0..ring.len())
            .filter(|&i| self.filleted(ring[i].cin).is_some())
            .collect();
        match hot[..] {
            [h] => self.run_out(v, &ring, h),
            [a, b] if self.is_joint(&ring, a, b) => self.joint(v, &ring, a, b),
            _ => self.corner(v, &ring),
        }
    }

    /// Whether the blends arriving at ring corners `a` and `b` continue each other
    /// through the vertex: same radius, and every other edge there smooth.
    fn is_joint(&self, ring: &[Corner], a: usize, b: usize) -> bool {
        let k = ring.len();
        let (ra, rb) = (
            &self.rolled[self.filleted(ring[a].cin).expect("hot")],
            &self.rolled[self.filleted(ring[b].cin).expect("hot")],
        );
        if (ra.blend.radius - rb.blend.radius).abs() > tol::LINEAR || ra.convex != rb.convex {
            return false;
        }
        (0..k).filter(|&j| j != a && j != b).all(|j| {
            let c = ring[j].cin;
            let e = self.body.coedge(c).edge;
            let x = self.body.edge(e);
            self.body.coedge_face(x.coedges[0]) == self.body.coedge_face(x.coedges[1])
                || is_smooth(&self.body, e, ra.blend.radius)
        })
    }

    fn vertex_name(&self, v: VertexId, faces: &[Name]) -> Name {
        Name::NONE
            .with_set(faces.to_vec())
            .with_name(self.body.vertex(v).name)
    }

    fn face_name(&self, f: FaceId) -> Name {
        self.body.face(f).name
    }

    /// An edge on `curve` from `a` (at parameter ta) to `b` (at tb), and whether walking
    /// a to b runs with it.
    fn edge_between(
        &mut self,
        curve: Curve,
        ta: f64,
        tb: f64,
        a: VertexId,
        b: VertexId,
        name: Name,
    ) -> (EdgeId, bool) {
        if ta <= tb {
            (self.body.add_edge(curve, Interval::new(ta, tb), a, b, name), true)
        } else {
            (
                self.body.add_edge(curve, Interval::new(tb, ta), b, a, name),
                false,
            )
        }
    }

    /// Where an unfilleted edge, arriving at `v` through coedge `c`, is cut at `p`.
    /// With `extend`, p may lie on the edge carried on past v, over face `extend`:
    /// a blend's end cut by a face it runs out across, beyond that face's corner.
    fn trim(
        &mut self,
        c: CoedgeId,
        p: Point3,
        x: VertexId,
        blend_edge: EdgeId,
        extend: Option<FaceId>,
    ) -> R<()> {
        let co = self.body.coedge(c).clone();
        let e = self.body.edge(co.edge);
        let at_end = co.forward;
        let mid = e.range.mid();
        let window = if at_end {
            Interval::new(mid, e.range.hi)
        } else {
            Interval::new(e.range.lo, mid)
        };
        let mut t = e.curve.closest(p, window);
        if e.curve.eval(t).distance(p) > tol::LINEAR
            && let Some(f) = extend
        {
            let v = if at_end { e.range.hi } else { e.range.lo };
            let w = window.width();
            let past = if at_end {
                Interval::new(v, v + w)
            } else {
                Interval::new(v - w, v)
            };
            let u = e.curve.closest(p, past);
            let ext = if at_end {
                Interval::new(v, u)
            } else {
                Interval::new(u, v)
            };
            if e.curve.eval(u).distance(p) <= tol::LINEAR
                && ext.width() > 0.0
                && inside_face(&self.body, f, &e.curve, ext)
            {
                t = u;
            }
        }
        if e.curve.eval(t).distance(p) > tol::LINEAR {
            return Err(FilletError::Overflow { edge: blend_edge });
        }
        let far = if at_end { e.start } else { e.end };
        if self.body.vertex(far).point.distance(p) <= tol::LINEAR {
            return Err(FilletError::Tangency { edge: blend_edge });
        }
        self.trims.insert((co.edge, at_end), (x, t));
        Ok(())
    }

    /// A lone blend at `v`: it runs out across the faces between its two sides.
    fn run_out(&mut self, v: VertexId, ring: &[Corner], hot: usize) -> R<()> {
        let k = ring.len();
        // corner j of the rotated ring: corner 0 has the blend's edge arriving
        let at = |j: usize| ring[(hot + j) % k];
        let ri = self.filleted(at(0).cin).expect("hot corner");
        let edge = self.rolled[ri].edge;
        let vp = self.body.vertex(v).point;
        let t_ref = spine_closest(&self.rolled[ri].blend.spine, vp);
        let radius = self.rolled[ri].blend.radius;
        let reach = self.body.edge_box(edge).diagonal() + 4.0 * radius;
        // points X[1..k-1]: X[j] on the edge arriving at corner j
        let mut xs: Vec<(Point3, VertexId)> = vec![(vp, v); k];
        let mut tside = [0.0; 2];
        for (j, (spring_c, other)) in [
            (1, (at(0).cin, at(1).face)),
            (k - 1, (at(k - 1).cout, at(k - 2).face)),
        ] {
            let ro = &self.rolled[ri];
            let s = self.side(spring_c);
            let spring = &ro.springs[s];
            let surf = &self.body.face(other).surface;
            let (t, p) = nearest_hit(spring, search_window(spring, t_ref, reach), surf, vp)
                .ok_or(FilletError::Overflow { edge })?;
            tside[s] = t;
            let names = [
                self.face_name(at(j).face),
                self.face_name(at(j - 1).face),
                ro.name,
            ];
            let name = self.vertex_name(v, &names);
            xs[j] = (p, self.body.add_vertex(p, name));
        }
        for (j, slot) in xs.iter_mut().enumerate().take(k - 1).skip(2) {
            let e = self.body.edge(self.body.coedge(at(j).cin).edge);
            let (curve, range) = (e.curve.clone(), e.range);
            let surf = self.rolled[ri].blend.surface.clone();
            let (_, p) = nearest_hit(&curve, range, &surf, vp).ok_or(FilletError::Overflow { edge })?;
            let names = [
                self.face_name(at(j).face),
                self.face_name(at(j - 1).face),
                self.rolled[ri].name,
            ];
            let name = self.vertex_name(v, &names);
            *slot = (p, self.body.add_vertex(p, name));
        }
        for (j, &(p, x)) in xs.clone().iter().enumerate().skip(1) {
            // the edges either side of the blend's own faces may carry on past v
            let extend = match j {
                1 => Some(at(0).face),
                _ if j == k - 1 => Some(at(k - 1).face),
                _ => None,
            };
            self.trim(at(j).cin, p, x, edge, extend)?;
        }
        // cross curves on the faces in between, each from X[j] to X[j+1]
        let mut back: Vec<(EdgeId, bool)> = Vec::new();
        for j in 1..k - 1 {
            let f = at(j).face;
            let ro = &self.rolled[ri];
            let (a, b) = (xs[j], xs[j + 1]);
            let (curve, ta, tb) = cross_curve(&ro.blend, &self.body.face(f).surface, a.0, b.0)
                .ok_or(FilletError::Overflow { edge })?;
            let name = Name::NONE
                .with_set(vec![ro.name, self.face_name(f)])
                .with_name(self.body.vertex(v).name);
            let (eid, fwd) = self.edge_between(curve, ta, tb, a.1, b.1, name);
            self.paths.insert(at(j).cin, vec![(eid, fwd)]);
            back.push((eid, !fwd));
        }
        back.reverse();
        let s_in = self.side(at(0).cin);
        let mut end = SpringEnd {
            v: [v; 2],
            t: tside,
            path: back,
        };
        end.v[s_in] = xs[1].1;
        end.v[1 - s_in] = xs[k - 1].1;
        let at_end = self.arrives_at_end(at(0).cin);
        self.ends.insert((edge, at_end), end);
        Ok(())
    }

    /// Whether coedge `c` (arriving at a vertex) arrives at its edge's end rather than its start.
    fn arrives_at_end(&self, c: CoedgeId) -> bool {
        self.body.coedge(c).forward
    }

    /// Two blends of a tangent chain meet at `v` (arriving at ring corners `ia` and
    /// `ib`), every other edge there smooth: they share the ball's cross-section.
    /// It lies where a spring crosses an edge between faces on different surfaces;
    /// other smooth edges (seams) are cut where the springs cross them, and the
    /// springs between are walked by both the blend and the face corner.
    fn joint(&mut self, v: VertexId, ring: &[Corner], ia: usize, ib: usize) -> R<()> {
        let k = ring.len();
        let err = FilletError::Corner { vertex: v };
        let (ra, rb) = (
            self.filleted(ring[ia].cin).expect("hot"),
            self.filleted(ring[ib].cin).expect("hot"),
        );
        let vp = self.body.vertex(v).point;
        let ea = self.rolled[ra].edge;
        // blend a's and b's spring sides on the P faces (ring ia..ib); the X faces are the rest
        let (sa, sb) = (self.side(ring[ia].cin), self.side(ring[(ib + k - 1) % k].cout));
        let span = (ib + k - ia) % k;
        // the smooth edges on each side, from a's end to b's
        let p_edges: Vec<CoedgeId> = (1..span).map(|d| ring[(ia + d) % k].cin).collect();
        let x_edges: Vec<CoedgeId> = (span + 1..k).rev().map(|d| ring[(ia + d) % k].cin).collect();
        // the section: where a spring crosses an edge between two surfaces, else any smooth edge
        let t0 = spine_closest(&self.rolled[ra].blend.spine, vp);
        let mut ta = None;
        let mut any = None;
        for (edges, s) in [(&p_edges, sa), (&x_edges, 1 - sa)] {
            for &c in edges.iter() {
                let e = self.body.edge(self.body.coedge(c).edge);
                let (f0, f1) = (
                    self.body.coedge_face(e.coedges[0]),
                    self.body.coedge_face(e.coedges[1]),
                );
                let change = f0 != f1 && self.body.face(f0).surface != self.body.face(f1).surface;
                if !change && any.is_some() {
                    continue;
                }
                let t = crossing(&self.rolled[ra].springs[s], t0, &e.curve, e.range).ok_or(err.clone())?;
                if !change {
                    any = Some(t);
                    continue;
                }
                let spine = &self.rolled[ra].blend.spine;
                match ta {
                    Some(u) if spine.eval(u).distance(spine.eval(t)) > tol::LINEAR => return Err(err),
                    Some(_) => {}
                    None => ta = Some(t),
                }
            }
        }
        let ta = ta.or(any).unwrap_or(t0);
        let ca = self.rolled[ra].blend.spine.eval(ta);
        let tb = spine_closest(&self.rolled[rb].blend.spine, ca);
        if ca.distance(self.rolled[rb].blend.spine.eval(tb)) > tol::LINEAR {
            return Err(err);
        }
        let p = self.rolled[ra].springs[sa].eval(ta);
        let x = self.rolled[ra].springs[1 - sa].eval(ta);
        if p.distance(self.rolled[rb].springs[sb].eval(tb)) > tol::LINEAR
            || x.distance(self.rolled[rb].springs[1 - sb].eval(tb)) > tol::LINEAR
        {
            return Err(err);
        }
        let (na, nb) = (self.rolled[ra].name, self.rolled[rb].name);
        let fname = |j: usize| self.face_name(ring[j % k].face);
        let vp_name = self.vertex_name(v, &[fname(ia), fname(ib + k - 1), na, nb]);
        let vx_name = self.vertex_name(v, &[fname(ib), fname(ia + k - 1), na, nb]);
        let pv = self.body.add_vertex(p, vp_name);
        let xv = self.body.add_vertex(x, vx_name);
        let mid = |b: &Builder, ri: usize| {
            let e = b.body.edge(b.rolled[ri].edge);
            spine_closest(&b.rolled[ri].blend.spine, e.curve.eval(e.range.mid()))
        };
        let (mid_a, mid_b) = (mid(self, ra), mid(self, rb));
        let ahead = |b: &Builder, ri: usize, t: f64, from: f64, mid: f64| {
            let period = b.rolled[ri].blend.spine.domain().period;
            along(t - from, period) * along(mid - from, period) > 0.0
        };
        // per side: the spring pieces from a's end to b's, the section foot's place among
        // them, and each smooth edge's
        let mut sides = Vec::with_capacity(2);
        for (edges, s_a, s_b, foot, fv) in [(&p_edges, sa, sb, p, pv), (&x_edges, 1 - sa, 1 - sb, x, xv)] {
            // landings: (blend, side, parameter, vertex); the foot is (None, ..)
            let mut on_a: Vec<(f64, VertexId, usize)> = Vec::new();
            let mut on_b: Vec<(f64, VertexId, usize)> = Vec::new();
            let mut at_foot: Vec<usize> = Vec::new();
            for (m, &c) in edges.iter().enumerate() {
                let e = self.body.edge(self.body.coedge(c).edge);
                let (curve, range, ename) = (e.curve.clone(), e.range, e.name);
                let t = crossing(&self.rolled[ra].springs[s_a], ta, &curve, range).ok_or(err.clone())?;
                let q = self.rolled[ra].springs[s_a].eval(t);
                let name = Name::NONE.with_set(vec![na, nb]).with_name(ename);
                if q.distance(foot) <= tol::LINEAR {
                    self.trim(c, foot, fv, ea, None)?;
                    at_foot.push(m);
                } else if ahead(self, ra, t, ta, mid_a) {
                    let qv = self.body.add_vertex(q, name);
                    self.trim(c, q, qv, ea, None)?;
                    on_a.push((t, qv, m));
                } else {
                    let u = crossing(&self.rolled[rb].springs[s_b], tb, &curve, range).ok_or(err.clone())?;
                    if !ahead(self, rb, u, tb, mid_b) {
                        return Err(err);
                    }
                    let q = self.rolled[rb].springs[s_b].eval(u);
                    let qv = self.body.add_vertex(q, name);
                    self.trim(c, q, qv, ea, None)?;
                    on_b.push((u, qv, m));
                }
            }
            // in ring order the landings must run from a's far end to b's
            let pa = self.rolled[ra].blend.spine.domain().period;
            let pb = self.rolled[rb].blend.spine.domain().period;
            let order: Vec<usize> = on_a.iter().chain(on_b.iter()).map(|l| l.2).collect();
            let dist_a: Vec<f64> = on_a.iter().map(|l| along(l.0 - ta, pa).abs()).collect();
            let dist_b: Vec<f64> = on_b.iter().map(|l| along(l.0 - tb, pb).abs()).collect();
            let a_end = on_a.len();
            let mut placed = order.clone();
            placed.splice(a_end..a_end, at_foot.iter().copied());
            if at_foot.len() > 1
                || !placed.windows(2).all(|w| w[0] < w[1])
                || !dist_a.windows(2).all(|w| w[0] > w[1])
                || !dist_b.windows(2).all(|w| w[0] < w[1])
            {
                return Err(err);
            }
            // nodes from a's end to b's: a's landings, the foot, b's landings
            let mut nodes: Vec<(VertexId, Option<(usize, f64)>)> =
                on_a.iter().map(|l| (l.1, Some((ra, l.0)))).collect();
            nodes.push((fv, None));
            nodes.extend(on_b.iter().map(|l| (l.1, Some((rb, l.0)))));
            let foot_at = a_end;
            let mut segs = Vec::with_capacity(nodes.len() - 1);
            for w in 0..nodes.len() - 1 {
                let (ri, s, tf) = if w < foot_at { (ra, s_a, ta) } else { (rb, s_b, tb) };
                let t_of = |n: &(VertexId, Option<(usize, f64)>)| n.1.map_or(tf, |x| x.1);
                let (t0, t1) = (t_of(&nodes[w]), t_of(&nodes[w + 1]));
                let curve = self.rolled[ri].springs[s].clone();
                let name = Name::NONE
                    .with_set(vec![self.rolled[ri].name])
                    .with_name(self.body.vertex(nodes[w + 1].0).name)
                    .with(w as u64);
                segs.push(self.edge_between(curve, t0, t1, nodes[w].0, nodes[w + 1].0, name));
            }
            // node of each boundary in a-to-b order: a's end, the smooth edges, b's end
            let mut bounds = vec![0usize; edges.len() + 2];
            for (n, l) in on_a.iter().enumerate() {
                bounds[l.2 + 1] = n;
            }
            for &m in &at_foot {
                bounds[m + 1] = foot_at;
            }
            for (n, l) in on_b.iter().enumerate() {
                bounds[l.2 + 1] = foot_at + 1 + n;
            }
            bounds[edges.len() + 1] = nodes.len() - 1;
            let ends = (nodes[0], nodes[nodes.len() - 1]);
            sides.push((segs, bounds, foot_at, ends));
        }
        let rev = |segs: &[(EdgeId, bool)]| -> Vec<(EdgeId, bool)> {
            segs.iter().rev().map(|&(e, f)| (e, !f)).collect()
        };
        // face corners: P ones walk from a to b, X ones from b to a
        for d in 0..span {
            let (segs, bounds, _, _) = &sides[0];
            let path = segs[bounds[d]..bounds[d + 1]].to_vec();
            if !path.is_empty() {
                self.paths.insert(ring[(ia + d) % k].cin, path);
            }
        }
        for d in 0..k - span {
            let (segs, bounds, _, _) = &sides[1];
            let n = bounds.len();
            let path = rev(&segs[bounds[n - 2 - d]..bounds[n - 1 - d]]);
            if !path.is_empty() {
                self.paths.insert(ring[(ib + d) % k].cin, path);
            }
        }
        let (circle, angle) = self.rolled[ra].blend.section(ta).ok_or(err.clone())?;
        // the section runs from the foot on the blend's first surface (its side 0)
        let (from_v, to_v) = if sa == 0 { (pv, xv) } else { (xv, pv) };
        let name = Name::NONE
            .with_set(vec![na, nb])
            .with_str("section")
            .with_name(self.body.vertex(v).name);
        let s = self.body.add_edge(
            Curve::Circle(circle),
            Interval::new(0.0, angle),
            from_v,
            to_v,
            name,
        );
        let x_to_p = from_v == xv;
        let ((p_segs, _, p_foot, p_ends), (x_segs, _, x_foot, x_ends)) = (&sides[0], &sides[1]);
        let t_end = |n: &(VertexId, Option<(usize, f64)>), t: f64| n.1.map_or(t, |x| x.1);
        // blend a, from its X end round through the section to its P end
        let mut path = x_segs[..*x_foot].to_vec();
        path.push((s, x_to_p));
        path.extend(rev(&p_segs[..*p_foot]));
        let mut end_a = SpringEnd {
            v: [v; 2],
            t: [ta; 2],
            path,
        };
        (end_a.v[sa], end_a.t[sa]) = (p_ends.0.0, t_end(&p_ends.0, ta));
        (end_a.v[1 - sa], end_a.t[1 - sa]) = (x_ends.0.0, t_end(&x_ends.0, ta));
        // blend b, from its P end round to its X end
        let mut path = rev(&p_segs[*p_foot..]);
        path.push((s, !x_to_p));
        path.extend(x_segs[*x_foot..].iter().copied());
        let mut end_b = SpringEnd {
            v: [v; 2],
            t: [tb; 2],
            path,
        };
        (end_b.v[sb], end_b.t[sb]) = (p_ends.1.0, t_end(&p_ends.1, tb));
        (end_b.v[1 - sb], end_b.t[1 - sb]) = (x_ends.1.0, t_end(&x_ends.1, tb));
        let eb = self.rolled[rb].edge;
        self.ends.insert((ea, self.arrives_at_end(ring[ia].cin)), end_a);
        self.ends.insert((eb, self.arrives_at_end(ring[ib].cin)), end_b);
        Ok(())
    }

    /// The ball's centre halfway from `v` to the far end of the filleted tangent chain that
    /// leaves it along `e`, or None when the chain comes back to `v`.
    fn chain_middle(&self, v: VertexId, e: EdgeId, spine: &Curve) -> Option<Point3> {
        let blended: BTreeSet<EdgeId> = self.rolled.iter().map(|r| r.edge).collect();
        let other = |e: EdgeId, w: VertexId| {
            let x = self.body.edge(e);
            if x.start == w { x.end } else { x.start }
        };
        // unit tangent of e leaving its end w
        let leaving = |e: EdgeId, w: VertexId| {
            let x = self.body.edge(e);
            let d = if x.start == w {
                x.curve.deriv(x.range.lo).d1
            } else {
                -x.curve.deriv(x.range.hi).d1
            };
            d.normalize().map_or(Vec3::ZERO, |u| u.get())
        };
        let (mut e, mut w) = (e, other(e, v));
        for _ in 0..self.rolled.len() {
            if w == v {
                return None;
            }
            let back = leaving(e, w);
            let next = blended.iter().copied().find(|&o| {
                let x = self.body.edge(o);
                o != e && (x.start == w || x.end == w) && leaving(o, w).dot(back) < -1.0 + 1e-9
            });
            let Some(o) = next else { break };
            (e, w) = (o, other(o, w));
        }
        if w == v {
            return None;
        }
        let half = self.body.vertex(v).point.lerp(self.body.vertex(w).point, 0.5);
        Some(spine.eval(spine_closest(spine, half)))
    }

    /// The corner at `v` as the corner solver sees it: faces counter-clockwise
    /// seen from outside (the ring reversed), the edge between each and the next.
    /// Also the ring corner of each blend.
    fn corner_input(&self, v: VertexId, ring: &[Corner]) -> (CornerInput, Vec<usize>) {
        let k = ring.len();
        let vp = self.body.vertex(v).point;
        let mut faces = Vec::with_capacity(k);
        let mut face_surfaces = Vec::with_capacity(k);
        let mut edges = Vec::with_capacity(k);
        let mut blends = Vec::new();
        let mut blend_ring = Vec::new();
        for i in 0..k {
            let j = (k - i) % k;
            let f = ring[j].face;
            faces.push(f);
            face_surfaces.push((self.body.face(f).surface.clone(), self.body.face(f).same_sense));
            // between faces[i] (ring j) and faces[i + 1] (ring j - 1) lies the edge arriving at ring j
            let c = ring[j].cin;
            // its parameter grows away from v unless the coedge arriving here runs with it
            let away = if self.body.coedge(c).forward { -1.0 } else { 1.0 };
            match self.filleted(c) {
                Some(ri) => {
                    let ro = &self.rolled[ri];
                    let middle = self.chain_middle(v, self.body.coedge(c).edge, &ro.blend.spine);
                    edges.push(CornerEdge::Blend(blends.len()));
                    blends.push(CornerBlend {
                        surface: ro.blend.surface.clone(),
                        same_sense: ro.convex,
                        radius: ro.blend.radius,
                        convex: ro.convex,
                        spine: ro.blend.spine.clone(),
                        out: if ro.aligned { away } else { -away },
                        setback: spine_closest(&ro.blend.spine, vp),
                        middle,
                    });
                    blend_ring.push(j);
                }
                None => edges.push(CornerEdge::Sharp {
                    curve: self.body.edge(self.body.coedge(c).edge).curve.clone(),
                    out: away,
                }),
            }
        }
        (
            CornerInput {
                vertex: v,
                point: vp,
                blends,
                faces,
                face_surfaces,
                edges,
            },
            blend_ring,
        )
    }

    /// Where each blend stops (in corner input order) when one ball touches all
    /// three faces of a corner (equal radii, one kind), and the ball's centre.
    fn ball_setbacks(&self, v: VertexId, ring: &[Corner]) -> Option<(Vec<f64>, Point3)> {
        if ring.len() != 3 {
            return None;
        }
        let ri: Vec<usize> = ring.iter().map(|k| self.filleted(k.cin)).collect::<Option<_>>()?;
        let r0 = &self.rolled[ri[0]];
        let (radius, convex) = (r0.blend.radius, r0.convex);
        if ri.iter().any(|&i| {
            (self.rolled[i].blend.radius - radius).abs() > tol::LINEAR || self.rolled[i].convex != convex
        }) {
            return None;
        }
        // each face's offset, from the blend arriving at its corner
        let offs: Vec<(Surface, f64)> = ring
            .iter()
            .zip(&ri)
            .map(|(k, &i)| {
                let b = &self.rolled[i].blend;
                if self.side(k.cin) == 0 {
                    (b.a.clone(), b.da)
                } else {
                    (b.b.clone(), b.db)
                }
            })
            .collect();
        let vp = self.body.vertex(v).point;
        let guess = ri
            .iter()
            .map(|&i| {
                let s = &self.rolled[i].blend.spine;
                s.eval(spine_closest(s, vp)).to_vec()
            })
            .fold(Vec3::ZERO, |a, b| a + b)
            / 3.0;
        let centre = three_offsets(&offs, Point3::new(guess.x, guess.y, guess.z))?;
        // corner input order is ring 0, 2, 1
        let mut ts = Vec::new();
        for j in [0, 2, 1] {
            let s = &self.rolled[ri[j]].blend.spine;
            let t = spine_closest(s, centre);
            if s.eval(t).distance(centre) > tol::LINEAR {
                return None;
            }
            ts.push(t);
        }
        Some((ts, centre))
    }

    /// A vertex in `pool` within LINEAR of `p`, or a new one.
    fn pooled(&mut self, pool: &mut Vec<(Point3, VertexId)>, p: Point3, name: Name) -> VertexId {
        if let Some(&(_, v)) = pool.iter().find(|q| q.0.distance(p) <= tol::LINEAR) {
            return v;
        }
        let v = self.body.add_vertex(p, name);
        pool.push((p, v));
        v
    }

    /// Several blends meet at `v`, perhaps with sharp edges between them. The
    /// corner solver says where each blend stops and how each face is trimmed,
    /// then patches the hole they leave; without one, a single ball closes a
    /// three-blend corner where it fits.
    fn corner(&mut self, v: VertexId, ring: &[Corner]) -> R<()> {
        let k = ring.len();
        let err = FilletError::Corner { vertex: v };
        let (input, blend_ring) = self.corner_input(v, ring);
        let (setbacks, trims, centre) = match crate::corner::setbacks(&input) {
            Some(s) => {
                let t = crate::corner::face_curves(&input, &s).map_err(|_| err.clone())?;
                (s, t, None)
            }
            None => {
                let (s, c) = self.ball_setbacks(v, ring).ok_or(err.clone())?;
                (s, Vec::new(), Some(c))
            }
        };
        if setbacks.len() != input.blends.len() {
            return Err(err);
        }
        let ccw = |j: usize| (k - j) % k;
        let vname = self.body.vertex(v).name;
        let patch_name = self.root.with_str("corner").with_name(vname);
        let mut pool: Vec<(Point3, VertexId)> = Vec::new();
        // per ring corner j: where its blend stops (parameter, contact on face j, contact on face j - 1)
        let mut stops: Vec<Option<(f64, VertexId, VertexId)>> = vec![None; k];
        for (b, &j) in blend_ring.iter().enumerate() {
            let ri = self.filleted(ring[j].cin).expect("blend");
            let s_in = self.side(ring[j].cin);
            let t = setbacks[b];
            let (pa, pb) = (
                self.rolled[ri].springs[s_in].eval(t),
                self.rolled[ri].springs[1 - s_in].eval(t),
            );
            let fa = self.face_name(ring[j].face);
            let fb = self.face_name(ring[(j + k - 1) % k].face);
            let rn = self.rolled[ri].name;
            let va = self.pooled(&mut pool, pa, Name::NONE.with_set(vec![fa, rn]).with_name(vname));
            let vb = self.pooled(&mut pool, pb, Name::NONE.with_set(vec![fb, rn]).with_name(vname));
            stops[j] = Some((t, va, vb));
        }
        // sharp edges are cut where the face trims beside them end
        let any_blend = self.rolled[self.filleted(ring[blend_ring[0]].cin).expect("blend")].edge;
        let mut cuts: Vec<Option<VertexId>> = vec![None; k];
        for j in 0..k {
            if stops[j].is_some() {
                continue;
            }
            let e = self.body.edge(self.body.coedge(ring[j].cin).edge);
            let (curve, range) = (e.curve.clone(), e.range);
            let on_edge = |p: Point3| curve.eval(curve.closest(p, range)).distance(p) <= tol::LINEAR;
            let m = trims
                .iter()
                .filter(|t| t.face == ccw(j) || t.face == ccw((j + k - 1) % k))
                .flat_map(|t| [t.start, t.end])
                .find(|&p| on_edge(p) && p.distance(input.point) > tol::LINEAR)
                .ok_or(err.clone())?;
            let vm = self.pooled(&mut pool, m, Name::NONE.with_str("cut").with_name(vname));
            self.trim(ring[j].cin, m, vm, any_blend, None)?;
            cuts[j] = Some(vm);
        }
        // face corners: from the arriving side's stop to the leaving side's, along the face's trim
        let mut face_edges: Vec<Option<(EdgeId, bool)>> = vec![None; k];
        for j in 0..k {
            let jn = (j + 1) % k;
            let a = stops[j].map_or(cuts[j], |s| Some(s.1)).ok_or(err.clone())?;
            let b = stops[jn].map_or(cuts[jn], |s| Some(s.2)).ok_or(err.clone())?;
            match trims.iter().find(|t| t.face == ccw(j)) {
                Some(t) => {
                    let (pa, pb) = (self.body.vertex(a).point, self.body.vertex(b).point);
                    let (ta, tb) = (t.curve.closest(pa, t.range), t.curve.closest(pb, t.range));
                    if t.curve.eval(ta).distance(pa) > tol::LINEAR
                        || t.curve.eval(tb).distance(pb) > tol::LINEAR
                    {
                        return Err(err);
                    }
                    let name = Name::NONE.with_set(vec![self.face_name(ring[j].face), patch_name]);
                    let walk = self.edge_between(t.curve.clone(), ta, tb, a, b, name);
                    self.paths.insert(ring[j].cin, vec![walk]);
                    face_edges[j] = Some(walk);
                }
                None if a == b => {}
                None => return Err(err),
            }
        }
        // blend sections, walked by the blend from its leaving side to its arriving side
        let mut sections: Vec<Option<(EdgeId, bool)>> = vec![None; k];
        for j in 0..k {
            let Some((t, va, vb)) = stops[j] else { continue };
            let ri = self.filleted(ring[j].cin).expect("blend");
            let s_in = self.side(ring[j].cin);
            let (circle, angle) = self.rolled[ri].blend.section(t).ok_or(err.clone())?;
            // the section runs from the foot on the blend's side 0
            let (v0, v1) = if s_in == 0 { (va, vb) } else { (vb, va) };
            let name = Name::NONE.with_set(vec![self.rolled[ri].name, patch_name]);
            let e = self
                .body
                .add_edge(Curve::Circle(circle), Interval::new(0.0, angle), v0, v1, name);
            let walk = (e, v0 == vb);
            sections[j] = Some(walk);
            let mut end = SpringEnd {
                v: [v; 2],
                t: [t; 2],
                path: vec![walk],
            };
            end.v[s_in] = va;
            end.v[1 - s_in] = vb;
            let edge = self.rolled[ri].edge;
            self.ends.insert((edge, self.arrives_at_end(ring[j].cin)), end);
        }
        // the hole, walked with the patch on its left: each side the other way to its neighbour
        let mut hole: Vec<HoleSide> = Vec::new();
        let mut hole_edges: Vec<(EdgeId, bool)> = Vec::new();
        for j in std::iter::once(0).chain((1..k).rev()) {
            let jp = (j + k - 1) % k;
            let mut sides = Vec::new();
            if let Some((e, fwd)) = sections[j] {
                let ro = &self.rolled[self.filleted(ring[j].cin).expect("blend")];
                let blend = blend_ring.iter().position(|&x| x == j).expect("blend");
                sides.push((
                    e,
                    !fwd,
                    ro.blend.surface.clone(),
                    ro.convex,
                    SideKind::Section { blend },
                ));
            }
            if let Some((e, fwd)) = face_edges[jp] {
                let face = self.body.face(ring[jp].face);
                let kind = SideKind::FaceCurve { face: ccw(jp) };
                sides.push((e, !fwd, face.surface.clone(), face.same_sense, kind));
            }
            for (e, fwd, neighbour, neighbour_same_sense, kind) in sides {
                let edge = self.body.edge(e);
                let (s, t) = (
                    self.body.vertex(edge.start).point,
                    self.body.vertex(edge.end).point,
                );
                hole.push(HoleSide {
                    curve: edge.curve.clone(),
                    range: edge.range,
                    forward: fwd,
                    start: if fwd { s } else { t },
                    end: if fwd { t } else { s },
                    neighbour,
                    neighbour_same_sense,
                    kind,
                });
                hole_edges.push((e, fwd));
            }
        }
        let patch = match centre {
            None => crate::corner::patch(&input, &hole)
                .ok_or(err.clone())?
                .map_err(|_| err.clone())?,
            Some(c) => ball_patch(&input, &hole, c).ok_or(err.clone())?,
        };
        let mut inner = Vec::with_capacity(patch.inner.len());
        for (n, (curve, range, a, b)) in patch.inner.iter().enumerate() {
            let name = patch_name.with_str("inner").with(n as u64);
            let va = self.pooled(&mut pool, *a, name.with_str("start"));
            let vb = self.pooled(&mut pool, *b, name.with_str("end"));
            inner.push(self.body.add_edge(curve.clone(), *range, va, vb, name));
        }
        for (n, pf) in patch.faces.into_iter().enumerate() {
            let uses = pf
                .uses
                .iter()
                .map(|u| match *u {
                    PatchUse::Side(i) => hole_edges[i],
                    PatchUse::Inner { edge, forward } => (inner[edge], forward),
                })
                .collect();
            let name = if n == 0 {
                patch_name
            } else {
                patch_name.with(n as u64)
            };
            self.patches.push((pf.surface, pf.same_sense, name, uses));
        }
        Ok(())
    }

    /// The spring edges of every blend, indexed like `rolled`.
    fn make_springs(&mut self) -> R<Vec<[EdgeId; 2]>> {
        let mut out = Vec::with_capacity(self.rolled.len());
        for i in 0..self.rolled.len() {
            let e = self.rolled[i].edge;
            let closed = self.body.edge(e).start == self.body.edge(e).end;
            let (s, t) = (&self.ends[&(e, false)], &self.ends[&(e, true)]);
            let mut pair = Vec::with_capacity(2);
            for side in 0..2 {
                let ro = &self.rolled[i];
                let curve = ro.springs[side].clone();
                // the spring runs with the spine: from the edge's start when aligned
                let ((v0, t0), (v1, t1)) = if ro.aligned {
                    ((s.v[side], s.t[side]), (t.v[side], t.t[side]))
                } else {
                    ((t.v[side], t.t[side]), (s.v[side], s.t[side]))
                };
                let t1 = match curve.domain().period {
                    Some(p) => {
                        let w = (t1 - t0).rem_euclid(p);
                        if closed && w * curve.deriv(t0).d1.length() <= tol::LINEAR {
                            t0 + p
                        } else {
                            t0 + w
                        }
                    }
                    None => t1,
                };
                if (!closed && curve.eval(t0).distance(curve.eval(t1)) <= tol::LINEAR) || t1 <= t0 {
                    return Err(FilletError::Overflow { edge: e });
                }
                let name = Name::NONE.with_set(vec![ro.name, self.body.face(ro.faces[side]).name]);
                if !inside_face(&self.body, ro.faces[side], &curve, Interval::new(t0, t1)) {
                    return Err(FilletError::Overflow { edge: e });
                }
                pair.push(self.body.add_edge(curve, Interval::new(t0, t1), v0, v1, name));
            }
            out.push([pair[0], pair[1]]);
        }
        Ok(out)
    }

    fn trim_edges(&mut self) -> R<()> {
        let trims: Vec<((EdgeId, bool), (VertexId, f64))> =
            self.trims.iter().map(|(k, v)| (*k, *v)).collect();
        for ((e, at_end), (v, t)) in trims {
            let edge = &mut self.body.edges[e.index()];
            if at_end {
                edge.end = v;
                edge.range.hi = t;
            } else {
                edge.start = v;
                edge.range.lo = t;
            }
            edge.bbox.clear();
        }
        for &(e, _) in self.trims.keys() {
            let edge = self.body.edge(e);
            // cut back from both ends to one point: two blends meet exactly there
            if self
                .body
                .vertex(edge.start)
                .point
                .distance(self.body.vertex(edge.end).point)
                <= tol::LINEAR
            {
                return Err(FilletError::Tangency { edge: e });
            }
            if edge.range.hi <= edge.range.lo {
                return Err(FilletError::Overflow { edge: e });
            }
        }
        Ok(())
    }

    fn rebuild_faces(&mut self, springs: &[[EdgeId; 2]]) {
        for r in &self.rolled {
            for &c in &self.body.edge(r.edge).coedges {
                self.touched.insert(self.body.coedge_face(c));
            }
        }
        let faces: Vec<FaceId> = self.touched.iter().copied().collect();
        for f in faces {
            let loops = self.body.face(f).loops.clone();
            for l in loops {
                let mut uses = Vec::new();
                for &c in &self.body.lp(l).coedges {
                    let co = self.body.coedge(c).clone();
                    match self.which.get(&co.edge) {
                        Some(&i) => {
                            let s = usize::from(self.rolled[i].coedges[0] != c);
                            uses.push((springs[i][s], co.forward == self.rolled[i].aligned));
                        }
                        None => uses.push((co.edge, co.forward)),
                    }
                    if let Some(p) = self.paths.get(&c) {
                        uses.extend(p.iter().copied());
                    }
                }
                self.body.remove_loop(f, l);
                self.body.add_loop(f, &uses);
            }
            let face = &mut self.body.faces[f.index()];
            face.bbox.clear();
            face.query.clear();
        }
    }

    fn add_blends(&mut self, springs: &[[EdgeId; 2]]) {
        for (i, ro) in self.rolled.iter().enumerate() {
            // side 0 walks the edge from a to b; the blend walks its spring back, crosses at a,
            // walks side 1's spring forward and crosses back at b
            let a_is_end = !ro.forward[0];
            let at_a = &self.ends[&(ro.edge, a_is_end)];
            let at_b = &self.ends[&(ro.edge, !a_is_end)];
            let mut uses = vec![(springs[i][0], ro.forward[0] != ro.aligned)];
            uses.extend(at_a.path.iter().copied());
            uses.push((springs[i][1], ro.forward[1] != ro.aligned));
            uses.extend(at_b.path.iter().copied());
            let shell = self.body.face(ro.faces[0]).shell;
            let f = self
                .body
                .add_face(shell, ro.blend.surface.clone(), ro.convex, ro.name);
            self.body.add_loop(f, &uses);
        }
    }
}

/// A parameter step, the short way round when the curve is periodic.
fn along(d: f64, period: Option<f64>) -> f64 {
    match period {
        Some(p) => {
            let w = d.rem_euclid(p);
            if w > 0.5 * p { w - p } else { w }
        }
        None => d,
    }
}

/// Whether the spring stays on its face (sampled strictly between its ends,
/// against the face as it was): a ball running off the face overflows.
fn inside_face(body: &Body, f: FaceId, c: &Curve, range: Interval) -> bool {
    let loops = forge_topo::uv::face_uv_loops(body, f);
    let surface = &body.face(f).surface;
    let d = surface.domain();
    let shifts = |p: Option<f64>| match p {
        Some(p) => vec![0.0, -p, p],
        None => vec![0.0],
    };
    let n = 16;
    (1..n).all(|i| {
        let uv = surface.project(c.eval(range.lo + range.width() * i as f64 / n as f64));
        shifts(d.u.period).iter().any(|&du| {
            shifts(d.v.period).iter().any(|&dv| {
                forge_topo::uv::point_in_loops(forge_math::Point2::new(uv.x + du, uv.y + dv), &loops)
            })
        })
    })
}

/// A parameter range of `c` around `t` reaching about `reach` along it.
fn search_window(c: &Curve, t: f64, reach: f64) -> Interval {
    let d = c.domain();
    if let Some(p) = d.period {
        return Interval::new(t - 0.5 * p, t + 0.5 * p);
    }
    let speed = c.deriv(t).d1.length().max(1e-12);
    let w = reach / speed;
    Interval::new((t - w).max(d.lo), (t + w).min(d.hi))
}

/// The point where `c` (over `range`) meets `s` nearest `near`, with its parameter.
fn nearest_hit(c: &Curve, range: Interval, s: &Surface, near: Point3) -> Option<(f64, Point3)> {
    match curve_surface(c, range, s) {
        CurveSurface::Hits(h) => h
            .into_iter()
            .min_by(|a, b| a.point.distance(near).total_cmp(&b.point.distance(near)))
            .map(|h| (h.t, h.point)),
        _ => None,
    }
}

/// Whether `p` (on the blend's surface) lies on the arc the ball spans between its contacts.
fn on_contact_arc(blend: &Blend, p: Point3) -> bool {
    let t = spine_closest(&blend.spine, p);
    let Some((circle, angle)) = blend.section(t) else {
        return false;
    };
    let l = circle.frame.point_to_local(p);
    let phi = dmath::atan2(l.y, l.x);
    let slack = tol::LINEAR / blend.radius;
    phi >= -slack && phi <= angle + slack
}

/// The curve where the blend meets surface `s` between `a` and `b` (both on
/// it), with the parameters of a and b; across a period, the arc whose
/// middle lies on the blend's contact arc.
fn cross_curve(blend: &Blend, s: &Surface, a: Point3, b: Point3) -> Option<(Curve, f64, f64)> {
    let region = Box3::from_points([a, b]).grow(2.0 * blend.radius + tol::LINEAR);
    let curves = closed_form(&blend.surface, s).or_else(|| {
        forge_geom::intcurve::trace(&blend.surface, s, region).map(|cs| {
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
        match c.domain().period {
            Some(p) => {
                let up = (tb - ta).rem_euclid(p);
                // a to b going up, or going down
                let mid = c.eval(ta + 0.5 * up);
                if on_contact_arc(blend, mid) {
                    return Some((c, ta, ta + up));
                }
                return Some((c, ta, ta - (p - up)));
            }
            None => return Some((c, ta, tb)),
        }
    }
    None
}

/// One blend arriving at a corner.
#[derive(Clone, Debug)]
pub struct CornerBlend {
    pub surface: Surface,
    pub same_sense: bool,
    pub radius: f64,
    pub convex: bool,
    /// The ball-centre curve.
    pub spine: Curve,
    /// +1 when the spine parameter grows going away from the vertex, else -1.
    pub out: f64,
    /// Where the blend would stop by itself (the section through the vertex).
    pub setback: f64,
    /// The ball's centre halfway along the edge's tangent chain: a stop goes no farther, so
    /// the stops at its two ends never cross. None for a closed chain.
    pub middle: Option<Point3>,
}

/// What lies between two neighbouring faces of a corner.
#[derive(Clone, Debug)]
pub enum CornerEdge {
    /// A blend (index into `CornerInput::blends`).
    Blend(usize),
    /// An edge left sharp; `out` is +1 when its parameter grows away from the vertex.
    Sharp { curve: Curve, out: f64 },
}

/// A vertex where several blends meet, as the corner solver sees it.
#[derive(Clone, Debug)]
pub struct CornerInput {
    pub vertex: VertexId,
    pub point: Point3,
    pub blends: Vec<CornerBlend>,
    /// Counter-clockwise seen from outside.
    pub faces: Vec<FaceId>,
    /// Parallel to `faces`: (surface, same_sense).
    pub face_surfaces: Vec<(Surface, bool)>,
    /// `edges[i]` lies between `faces[i]` and `faces[i + 1]`.
    pub edges: Vec<CornerEdge>,
}

/// Where a face ends at a corner: from one neighbour's stop to the next's.
#[derive(Clone, Debug)]
pub struct FaceTrim {
    /// Index into `CornerInput::faces`.
    pub face: usize,
    pub curve: Curve,
    pub range: Interval,
    pub start: Point3,
    pub end: Point3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SideKind {
    /// A blend's cross-section at its setback.
    Section { blend: usize },
    /// A face's trim (index into `CornerInput::faces`).
    FaceCurve { face: usize },
    /// A trimmed unfilleted edge.
    Edge { edge: EdgeId },
}

/// One side of a corner's hole, in order round it, walked with the patch on
/// its left seen from outside (the other way to its neighbour).
#[derive(Clone, Debug)]
pub struct HoleSide {
    pub curve: Curve,
    pub range: Interval,
    /// The patch walks the curve with its parameter.
    pub forward: bool,
    pub start: Point3,
    pub end: Point3,
    /// G1: along the side the patch's outward normal is the neighbour's.
    pub neighbour: Surface,
    pub neighbour_same_sense: bool,
    pub kind: SideKind,
}

/// The direction square to the side, tangent to the neighbour, pointing into the hole at `t`.
pub fn cross_tangent(side: &HoleSide, t: f64) -> Vec3 {
    let d = side.curve.deriv(t);
    let walk = if side.forward { d.d1 } else { -d.d1 };
    let n = side
        .neighbour
        .normal(side.neighbour.project(d.p))
        .map_or(Vec3::ZERO, |n| {
            if side.neighbour_same_sense {
                n.get()
            } else {
                -n.get()
            }
        });
    n.cross(walk).normalize().map_or(Vec3::ZERO, |u| u.get())
}

/// One face of a corner patch.
#[derive(Clone, Debug)]
pub struct PatchFace {
    pub surface: Surface,
    pub same_sense: bool,
    pub uses: Vec<PatchUse>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PatchUse {
    /// Hole side i, walked as the hole lists it.
    Side(usize),
    /// An edge of the patch's own (index into `CornerPatch::inner`).
    Inner { edge: usize, forward: bool },
}

/// The faces closing a corner's hole and the edges between them.
#[derive(Clone, Debug)]
pub struct CornerPatch {
    /// (curve, range, start point, end point).
    pub inner: Vec<(Curve, Interval, Point3, Point3)>,
    pub faces: Vec<PatchFace>,
}

/// The ball centred at `centre` closing a hole made only of its own sections.
fn ball_patch(input: &CornerInput, hole: &[HoleSide], centre: Point3) -> Option<CornerPatch> {
    let b = input.blends.first()?;
    // poles a quarter turn from the patch's middle, which faces the old vertex
    let mid = (input.point - centre).normalize()?;
    let frame = Frame::from_zx(centre, mid.any_perpendicular(), mid.get())?;
    Some(CornerPatch {
        inner: Vec::new(),
        faces: vec![PatchFace {
            surface: Surface::Sphere(Sphere {
                frame,
                radius: b.radius,
            }),
            same_sense: b.convex,
            uses: (0..hole.len()).map(PatchUse::Side).collect(),
        }],
    })
}

/// The parameter near `t` where curve `a` crosses curve `b` (over `range`),
/// both lying on one surface: alternate nearest points until they meet.
fn crossing(a: &Curve, mut t: f64, b: &Curve, range: Interval) -> Option<f64> {
    let window = search_window(a, t, 1e3);
    for _ in 0..100 {
        let p = a.eval(t);
        let q = b.eval(b.closest(p, range));
        if p.distance(q) <= 1e-3 * tol::LINEAR {
            return Some(t);
        }
        t = a.closest(q, window);
    }
    let p = a.eval(t);
    (b.eval(b.closest(p, range)).distance(p) <= tol::LINEAR).then_some(t)
}

/// The point on all three offset surfaces, by Newton from `p`.
fn three_offsets(offs: &[(Surface, f64)], mut p: Point3) -> Option<Point3> {
    for _ in 0..50 {
        let g: Vec<f64> = offs.iter().map(|(s, d)| s.implicit(p) - d).collect();
        if g.iter().all(|x| x.abs() <= 1e-3 * tol::LINEAR) {
            return Some(p);
        }
        let n: Vec<Vec3> = offs.iter().map(|(s, _)| s.gradient(p)).collect();
        let det = n[0].dot(n[1].cross(n[2]));
        if det.abs() <= 1e-12 {
            return None;
        }
        // Cramer: dp = -J^-1 g with J's rows the gradients
        let dp = (n[1].cross(n[2]) * g[0] + n[2].cross(n[0]) * g[1] + n[0].cross(n[1]) * g[2]) / det;
        p = p - dp;
    }
    let ok = offs.iter().all(|(s, d)| (s.implicit(p) - d).abs() <= tol::LINEAR);
    ok.then_some(p)
}
