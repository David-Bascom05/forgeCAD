use crate::name::Name;
use forge_geom::{Curve, CurveGeom, Surface, SurfaceGeom};
use forge_math::{Box3, Interval, Point2, Point3, UnitVec3};

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub u32);
        impl $name {
            pub fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}
id_type!(VertexId);
id_type!(EdgeId);
id_type!(CoedgeId);
id_type!(LoopId);
id_type!(FaceId);
id_type!(ShellId);

#[derive(Clone, Debug, PartialEq)]
pub struct Vertex {
    pub point: Point3,
    pub name: Name,
    /// The radius of the ball every curve and surface through the vertex meets: LINEAR, or
    /// more where a boolean could not settle the vertex within LINEAR of all of them.
    pub tol: f64,
}

/// A bounded piece of a curve. `start` is at `range.lo` and `end` at
/// `range.hi`: an edge always runs the way its curve's parameter increases.
/// A closed edge (a full circle) has start == end.
#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub curve: Curve,
    pub range: Interval,
    pub start: VertexId,
    pub end: VertexId,
    /// The two uses of this edge, one by each face it bounds (both by the same
    /// face for a seam).
    pub coedges: Vec<CoedgeId>,
    pub name: Name,
    /// False once no face uses the edge (after an in-place edit).
    pub alive: bool,
    /// A box holding the edge, worked out on first use.
    pub bbox: Cached<Box3>,
}

/// One use of an edge by a loop. `forward` means the loop walks the edge from
/// start to end.
#[derive(Clone, Debug, PartialEq)]
pub struct Coedge {
    pub edge: EdgeId,
    pub forward: bool,
    pub lp: LoopId,
}

/// A closed chain of coedges bounding a face. Walking it with the face's
/// outward normal pointing at you, the face is on the left.
#[derive(Clone, Debug, PartialEq)]
pub struct Loop {
    pub coedges: Vec<CoedgeId>,
    pub face: FaceId,
}

/// A bounded region of a surface. `same_sense` says whether the face's outward
/// normal (away from the solid's material) is the surface's natural normal.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub surface: Surface,
    pub same_sense: bool,
    pub loops: Vec<LoopId>,
    pub shell: ShellId,
    pub name: Name,
    /// A box holding the face, worked out on first use.
    pub bbox: Cached<Box3>,
    /// A point-in-face index, kept by whoever built it (the booleans) across edits that leave
    /// the face alone or only add loops to it; compaction drops it (coedge ids change there).
    pub query: Cached<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
    /// False once the face is removed by an in-place edit.
    pub alive: bool,
}

/// Data derived from an entity and kept with it so it is worked out once.
/// It takes no part in equality.
#[derive(Clone)]
pub struct Cached<T>(std::sync::OnceLock<T>);

impl<T> Default for Cached<T> {
    fn default() -> Self {
        Cached(std::sync::OnceLock::new())
    }
}

impl<T> Cached<T> {
    pub fn get_or_init(&self, f: impl FnOnce() -> T) -> &T {
        self.0.get_or_init(f)
    }
    pub fn get(&self) -> Option<&T> {
        self.0.get()
    }
    pub fn set(&self, v: T) {
        let _ = self.0.set(v);
    }
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.0.get_mut()
    }
    pub fn clear(&mut self) {
        self.0.take();
    }
}

impl<T> PartialEq for Cached<T> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl<T> std::fmt::Debug for Cached<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Cached")
    }
}

/// A connected, closed set of faces: the boundary between material and space.
#[derive(Clone, Debug, PartialEq)]
pub struct Shell {
    pub faces: Vec<FaceId>,
}

/// A solid: one or more closed shells (an outer skin, plus inner ones for voids).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Body {
    /// Faces removed by in-place edits and not yet compacted away.
    pub dead: usize,
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub coedges: Vec<Coedge>,
    pub loops: Vec<Loop>,
    pub faces: Vec<Face>,
    pub shells: Vec<Shell>,
    /// Built on first use, kept up to date from then on.
    pub hulls: Cached<FaceHulls>,
}

/// Boxes over runs of face ids (a chunk of 16 faces, a block of 16 chunks), each the hull of
/// its live faces' boxes: worked out on first use, dropped when a face in the run comes or goes.
#[derive(Clone)]
pub struct FaceHulls {
    chunks: Vec<Cached<Chunk>>,
    blocks: Vec<Cached<Box3>>,
}

#[derive(Clone)]
struct Chunk {
    hull: Box3,
    /// Bit i: face 16c + i is alive.
    live: u16,
    boxes: [Box3; RUN],
}

const RUN: usize = 16;

impl Body {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn vertex(&self, id: VertexId) -> &Vertex {
        &self.vertices[id.index()]
    }
    pub fn edge(&self, id: EdgeId) -> &Edge {
        &self.edges[id.index()]
    }
    pub fn coedge(&self, id: CoedgeId) -> &Coedge {
        &self.coedges[id.index()]
    }
    pub fn lp(&self, id: LoopId) -> &Loop {
        &self.loops[id.index()]
    }
    pub fn face(&self, id: FaceId) -> &Face {
        &self.faces[id.index()]
    }
    pub fn shell(&self, id: ShellId) -> &Shell {
        &self.shells[id.index()]
    }

    pub fn vertex_ids(&self) -> impl Iterator<Item = VertexId> + '_ {
        (0..self.vertices.len() as u32).map(VertexId)
    }
    pub fn edge_ids(&self) -> impl Iterator<Item = EdgeId> + '_ {
        (0..self.edges.len() as u32)
            .map(EdgeId)
            .filter(|e| self.edges[e.index()].alive)
    }
    pub fn face_ids(&self) -> impl Iterator<Item = FaceId> + '_ {
        (0..self.faces.len() as u32)
            .map(FaceId)
            .filter(|f| self.faces[f.index()].alive)
    }
    pub fn loop_ids(&self) -> impl Iterator<Item = LoopId> + '_ {
        (0..self.loops.len() as u32)
            .map(LoopId)
            .filter(|l| self.faces[self.loops[l.index()].face.index()].alive)
    }

    /// Removes a face (and its loops) in place. Edges left with no uses die
    /// with it; vertices are left for `compact` to drop.
    pub fn kill_face(&mut self, f: FaceId) {
        if !self.faces[f.index()].alive {
            return;
        }
        self.faces[f.index()].alive = false;
        self.dead += 1;
        if let Some(h) = self.hulls.get_mut() {
            h.chunks[f.index() / RUN].clear();
            h.blocks[f.index() / (RUN * RUN)].clear();
        }
        // shell face lists go stale here; compact() regroups shells
        for li in 0..self.faces[f.index()].loops.len() {
            let l = self.faces[f.index()].loops[li];
            for ci in 0..self.loops[l.index()].coedges.len() {
                let c = self.loops[l.index()].coedges[ci];
                let e = self.coedges[c.index()].edge;
                let edge = &mut self.edges[e.index()];
                edge.coedges.retain(|&x| x != c);
                if edge.coedges.is_empty() {
                    edge.alive = false;
                }
            }
        }
    }

    /// Regroups live faces into shells by connectivity (after an in-place edit
    /// joins or separates pieces).
    pub fn rebuild_shells(&mut self) {
        let group = self.connected();
        self.shells.clear();
        let mut shell_of: Vec<Option<ShellId>> = vec![None; self.faces.len()];
        for f in 0..self.faces.len() {
            if !self.faces[f].alive {
                continue;
            }
            let s = match shell_of[group[f]] {
                Some(s) => s,
                None => {
                    self.shells.push(Shell { faces: Vec::new() });
                    let s = ShellId(self.shells.len() as u32 - 1);
                    shell_of[group[f]] = Some(s);
                    s
                }
            };
            self.faces[f].shell = s;
            self.shells[s.index()].faces.push(FaceId(f as u32));
        }
    }

    /// For each face, the lowest face index connected to it through live edges.
    fn connected(&self) -> Vec<usize> {
        let n = self.faces.len();
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(p: &mut [usize], mut i: usize) -> usize {
            while p[i] != i {
                p[i] = p[p[i]];
                i = p[i];
            }
            i
        }
        for e in &self.edges {
            if !e.alive {
                continue;
            }
            for w in e.coedges.windows(2) {
                let (a, b) = (self.coedge_face(w[0]).index(), self.coedge_face(w[1]).index());
                let (x, y) = (find(&mut parent, a), find(&mut parent, b));
                if x != y {
                    parent[x.max(y)] = x.min(y);
                }
            }
        }
        (0..n).map(|f| find(&mut parent, f)).collect()
    }

    /// A copy holding only live entities, densely numbered, with shells
    /// regrouped by connectivity.
    pub fn compact(&self) -> Body {
        let group = self.connected();
        let mut out = Body::new();
        let mut vmap: Vec<Option<VertexId>> = vec![None; self.vertices.len()];
        let mut emap: Vec<Option<EdgeId>> = vec![None; self.edges.len()];
        let mut smap: Vec<Option<ShellId>> = vec![None; self.faces.len()];
        for f in self.face_ids() {
            let face = self.face(f);
            let shell = *smap[group[f.index()]].get_or_insert_with(|| out.add_shell());
            let nf = out.add_face(shell, face.surface.clone(), face.same_sense, face.name);
            if let Some(b) = face.bbox.get() {
                out.faces[nf.index()].bbox.set(*b);
            }
            for &l in &face.loops {
                let mut uses = Vec::with_capacity(self.lp(l).coedges.len());
                for &c in &self.lp(l).coedges {
                    let co = self.coedge(c);
                    let e = match emap[co.edge.index()] {
                        Some(e) => e,
                        None => {
                            let edge = self.edge(co.edge);
                            let mut vert = |v: VertexId| {
                                *vmap[v.index()].get_or_insert_with(|| {
                                    let x = self.vertex(v);
                                    out.add_tolerant_vertex(x.point, x.tol, x.name)
                                })
                            };
                            let (a, b) = (vert(edge.start), vert(edge.end));
                            let e = out.add_edge(edge.curve.clone(), edge.range, a, b, edge.name);
                            emap[co.edge.index()] = Some(e);
                            e
                        }
                    };
                    uses.push((e, co.forward));
                }
                out.add_loop(nf, &uses);
            }
        }
        out
    }

    /// Adds `other`'s entities after this body's, renumbered, its shells kept apart.
    pub fn append(&mut self, other: &Body) {
        let n = |x: usize| x as u32;
        let (v, e, c, l, f, s) = (
            n(self.vertices.len()),
            n(self.edges.len()),
            n(self.coedges.len()),
            n(self.loops.len()),
            n(self.faces.len()),
            n(self.shells.len()),
        );
        self.vertices.extend(other.vertices.iter().cloned());
        self.edges.extend(other.edges.iter().map(|x| Edge {
            start: VertexId(x.start.0 + v),
            end: VertexId(x.end.0 + v),
            coedges: x.coedges.iter().map(|k| CoedgeId(k.0 + c)).collect(),
            ..x.clone()
        }));
        self.coedges.extend(other.coedges.iter().map(|x| Coedge {
            edge: EdgeId(x.edge.0 + e),
            forward: x.forward,
            lp: LoopId(x.lp.0 + l),
        }));
        self.loops.extend(other.loops.iter().map(|x| Loop {
            coedges: x.coedges.iter().map(|k| CoedgeId(k.0 + c)).collect(),
            face: FaceId(x.face.0 + f),
        }));
        self.faces.extend(other.faces.iter().map(|x| Face {
            surface: x.surface.clone(),
            same_sense: x.same_sense,
            loops: x.loops.iter().map(|k| LoopId(k.0 + l)).collect(),
            shell: ShellId(x.shell.0 + s),
            name: x.name,
            bbox: x.bbox.clone(),
            query: Cached::default(),
            alive: x.alive,
        }));
        self.shells.extend(other.shells.iter().map(|x| Shell {
            faces: x.faces.iter().map(|k| FaceId(k.0 + f)).collect(),
        }));
        self.dead += other.dead;
    }

    pub fn add_vertex(&mut self, point: Point3, name: Name) -> VertexId {
        self.add_tolerant_vertex(point, forge_math::tol::LINEAR, name)
    }

    /// A vertex with its own tolerance (at least LINEAR).
    pub fn add_tolerant_vertex(&mut self, point: Point3, tol: f64, name: Name) -> VertexId {
        let tol = tol.max(forge_math::tol::LINEAR);
        self.vertices.push(Vertex { point, name, tol });
        VertexId(self.vertices.len() as u32 - 1)
    }

    pub fn add_edge(
        &mut self,
        curve: Curve,
        range: Interval,
        start: VertexId,
        end: VertexId,
        name: Name,
    ) -> EdgeId {
        self.edges.push(Edge {
            curve,
            range,
            start,
            end,
            coedges: Vec::new(),
            name,
            alive: true,
            bbox: Cached::default(),
        });
        EdgeId(self.edges.len() as u32 - 1)
    }

    pub fn add_shell(&mut self) -> ShellId {
        self.shells.push(Shell { faces: Vec::new() });
        ShellId(self.shells.len() as u32 - 1)
    }

    pub fn add_face(&mut self, shell: ShellId, surface: Surface, same_sense: bool, name: Name) -> FaceId {
        let id = FaceId(self.faces.len() as u32);
        self.faces.push(Face {
            surface,
            same_sense,
            loops: Vec::new(),
            shell,
            name,
            bbox: Cached::default(),
            query: Cached::default(),
            alive: true,
        });
        self.shells[shell.index()].faces.push(id);
        // the new face is the last: so are its chunk and block
        if let Some(h) = self.hulls.get_mut() {
            h.chunks.resize_with(id.index() / RUN + 1, Cached::default);
            h.blocks
                .resize_with(id.index() / (RUN * RUN) + 1, Cached::default);
            h.chunks[id.index() / RUN].clear();
            h.blocks[id.index() / (RUN * RUN)].clear();
        }
        id
    }

    /// Adds a loop to `face` made of the given (edge, forward) uses, in order.
    /// Takes loop `lp` off face `f`: its edges lose those uses (an edge left unused dies);
    /// the loop and its coedges stay in the arenas until compaction.
    pub fn remove_loop(&mut self, f: FaceId, lp: LoopId) {
        self.faces[f.index()].loops.retain(|&l| l != lp);
        for ci in 0..self.loops[lp.index()].coedges.len() {
            let c = self.loops[lp.index()].coedges[ci];
            let edge = &mut self.edges[self.coedges[c.index()].edge.index()];
            edge.coedges.retain(|&x| x != c);
            if edge.coedges.is_empty() {
                edge.alive = false;
            }
        }
    }

    pub fn add_loop(&mut self, face: FaceId, uses: &[(EdgeId, bool)]) -> LoopId {
        let lp = LoopId(self.loops.len() as u32);
        let mut coedges = Vec::with_capacity(uses.len());
        for &(edge, forward) in uses {
            let c = CoedgeId(self.coedges.len() as u32);
            self.coedges.push(Coedge { edge, forward, lp });
            self.edges[edge.index()].coedges.push(c);
            self.edges[edge.index()].alive = true;
            coedges.push(c);
        }
        self.loops.push(Loop { coedges, face });
        self.faces[face.index()].loops.push(lp);
        lp
    }

    pub fn coedge_start(&self, c: CoedgeId) -> VertexId {
        let co = self.coedge(c);
        let e = self.edge(co.edge);
        if co.forward { e.start } else { e.end }
    }
    pub fn coedge_end(&self, c: CoedgeId) -> VertexId {
        let co = self.coedge(c);
        let e = self.edge(co.edge);
        if co.forward { e.end } else { e.start }
    }
    pub fn coedge_face(&self, c: CoedgeId) -> FaceId {
        self.lp(self.coedge(c).lp).face
    }

    /// The other use of the same edge.
    pub fn partner(&self, c: CoedgeId) -> Option<CoedgeId> {
        let e = self.edge(self.coedge(c).edge);
        e.coedges.iter().copied().find(|&o| o != c)
    }

    /// The face's outward normal at surface parameters `uv`.
    pub fn face_normal(&self, f: FaceId, uv: Point2) -> Option<UnitVec3> {
        let face = self.face(f);
        let n = face.surface.normal(uv)?;
        Some(if face.same_sense { n } else { -n })
    }

    /// Points along an edge, `n + 1` of them, evenly spaced in parameter.
    pub fn edge_points(&self, e: EdgeId, n: usize) -> Vec<Point3> {
        let edge = self.edge(e);
        (0..=n)
            .map(|i| {
                let t = edge.range.lo + edge.range.width() * (i as f64 / n as f64);
                edge.curve.eval(t.min(edge.range.hi))
            })
            .collect()
    }

    /// A box holding face `f`, from its edges alone. A face on a plane,
    /// cylinder or cone lies in the hull of its boundary (through any point of
    /// it runs a straight line of the surface that meets the boundary on both
    /// sides); spheres, tori and surfaces defined by construction can bulge past
    /// their edges, so their whole surface box is added. Cached on the face.
    /// A box holding edge `e` (cached on the edge).
    pub fn edge_box(&self, e: EdgeId) -> Box3 {
        let edge = self.edge(e);
        *edge.bbox.get_or_init(|| edge.curve.bbox(edge.range))
    }

    fn hulls(&self) -> &FaceHulls {
        self.hulls.get_or_init(|| FaceHulls {
            chunks: (0..self.faces.len().div_ceil(RUN))
                .map(|_| Cached::default())
                .collect(),
            blocks: (0..self.faces.len().div_ceil(RUN * RUN))
                .map(|_| Cached::default())
                .collect(),
        })
    }

    fn chunk(&self, c: usize) -> &Chunk {
        self.hulls().chunks[c].get_or_init(|| {
            let mut ch = Chunk {
                hull: Box3::EMPTY,
                live: 0,
                boxes: [Box3::EMPTY; RUN],
            };
            for i in 0..RUN.min(self.faces.len() - c * RUN) {
                let f = FaceId((c * RUN + i) as u32);
                if self.face(f).alive {
                    ch.boxes[i] = self.face_box(f);
                    ch.hull = ch.hull.hull(&ch.boxes[i]);
                    ch.live |= 1 << i;
                }
            }
            ch
        })
    }

    fn block_box(&self, k: usize) -> Box3 {
        let h = self.hulls();
        *h.blocks[k].get_or_init(|| {
            (k * RUN..(k * RUN + RUN).min(h.chunks.len()))
                .fold(Box3::EMPTY, |b, c| b.hull(&self.chunk(c).hull))
        })
    }

    /// A box holding every live face.
    pub fn faces_box(&self) -> Box3 {
        (0..self.hulls().blocks.len()).fold(Box3::EMPTY, |b, k| b.hull(&self.block_box(k)))
    }

    /// `face_box` of a live face, empty for a removed one, as kept with the hulls.
    pub fn live_face_box(&self, f: FaceId) -> Box3 {
        self.chunk(f.index() / RUN).boxes[f.index() % RUN]
    }

    /// The live faces whose boxes `hit` accepts, in id order. A run of faces is passed over when
    /// `hit` refuses the hull of their boxes, so `hit` must accept every box holding one it accepts.
    pub fn faces_where<'a>(
        &'a self,
        hit: impl Fn(&Box3) -> bool + Copy + 'a,
    ) -> impl Iterator<Item = FaceId> + 'a {
        let h = self.hulls();
        (0..h.blocks.len())
            .filter(move |&k| hit(&self.block_box(k)))
            .flat_map(move |k| k * RUN..(k * RUN + RUN).min(h.chunks.len()))
            .map(move |c| (c, self.chunk(c)))
            .filter(move |(_, ch)| hit(&ch.hull))
            .flat_map(move |(c, ch)| {
                (0..RUN)
                    .filter(move |&i| ch.live >> i & 1 == 1 && hit(&ch.boxes[i]))
                    .map(move |i| FaceId((c * RUN + i) as u32))
            })
    }

    pub fn face_box(&self, f: FaceId) -> Box3 {
        let face = self.face(f);
        *face.bbox.get_or_init(|| {
            let mut b = Box3::EMPTY;
            for &l in &face.loops {
                for &c in &self.lp(l).coedges {
                    b = b.hull(&self.edge_box(self.coedge(c).edge));
                }
            }
            match &face.surface {
                Surface::Sphere(sp) => b = b.hull(&Box3::from_point(sp.frame.origin).grow(sp.radius)),
                Surface::Torus(t) => b = b.hull(&Box3::from_point(t.frame.origin).grow(t.major + t.minor)),
                Surface::Procedural(pr) => {
                    // over the stretch of the domain the face's loops span, not all of it: column
                    // by column in u, over only the v the face holds there (a cut askew across a
                    // blend's end leaves the end columns part of the arc)
                    let d = pr.domain();
                    let whole = forge_math::Box2::new(d.u.interval(), d.v.interval());
                    let loops = if d.u.period.is_none() && d.v.period.is_none() {
                        crate::uv::face_uv_loops(self, f)
                    } else {
                        Vec::new()
                    };
                    let mut su = Interval::EMPTY;
                    loops.iter().flat_map(|l| &l.points).for_each(|p| su.include(p.x));
                    if su.is_empty() || su.width() <= 0.0 {
                        b = b.hull(&pr.bbox(whole));
                    } else {
                        let pad = |i: Interval, w: Interval| {
                            let g = 0.05 * i.width() + 1e-9 * (1.0 + i.lo.abs().max(i.hi.abs()));
                            Interval::new((i.lo - g).max(w.lo), (i.hi + g).min(w.hi))
                        };
                        // eight columns; one the boundary crosses askew (a run-out's cut) in eight
                        // more, twice
                        const G: usize = 8;
                        let mut todo = vec![(su.lo, su.hi, 0)];
                        while let Some((lo, hi, depth)) = todo.pop() {
                            let (runs, askew) = column_runs(&loops, lo, hi);
                            if depth == 0 || (depth < 3 && askew) {
                                let at = |k: usize| lo + (hi - lo) * k as f64 / G as f64;
                                todo.extend((0..G).map(|k| (at(k), at(k + 1), depth + 1)));
                                continue;
                            }
                            let u = pad(Interval::new(lo, hi), whole.u);
                            for v in runs {
                                b = b.hull(&pr.bbox(forge_math::Box2::new(u, pad(v, whole.v))));
                            }
                        }
                    }
                }
                _ => {}
            }
            b.grow(forge_math::tol::LINEAR)
        })
    }

    pub fn face_by_name(&self, name: Name) -> Option<FaceId> {
        self.face_ids().find(|&f| self.face(f).name == name)
    }
    pub fn edge_by_name(&self, name: Name) -> Option<EdgeId> {
        self.edge_ids().find(|&e| self.edge(e).name == name)
    }
}

/// The v a face holds over u in [lo, hi], as runs, from its uv loops: each stretch of boundary
/// inside the column, and between the boundary's crossings of the column's middle; and whether
/// the boundary moves much in v while moving in u there (a cut askew).
fn column_runs(loops: &[crate::uv::UvLoop], lo: f64, hi: f64) -> (Vec<Interval>, bool) {
    let mid = 0.5 * (lo + hi);
    let mut runs: Vec<Interval> = Vec::new();
    let mut cross: Vec<f64> = Vec::new();
    let (mut moves, mut span) = (0.0f64, Interval::EMPTY);
    for l in loops {
        let n = l.points.len();
        let mut piece = Interval::EMPTY;
        for i in 0..n {
            let (p, q) = (l.points[i], l.points[(i + 1) % n]);
            if (p.x <= mid) != (q.x <= mid) {
                cross.push(p.y + (q.y - p.y) * (mid - p.x) / (q.x - p.x));
            }
            let (a, c) = (p.x.min(q.x).max(lo), p.x.max(q.x).min(hi));
            if a > c {
                if !piece.is_empty() {
                    runs.push(piece);
                    piece = Interval::EMPTY;
                }
                continue;
            }
            let at = |u: f64| p.y + (q.y - p.y) * ((u - p.x) / (q.x - p.x)).clamp(0.0, 1.0);
            let (va, vc) = if q.x == p.x { (p.y, q.y) } else { (at(a), at(c)) };
            if q.x != p.x {
                moves += (va - vc).abs();
            }
            piece.include(va);
            piece.include(vc);
            span.include(va);
            span.include(vc);
        }
        if !piece.is_empty() {
            runs.push(piece);
        }
    }
    cross.sort_by(f64::total_cmp);
    runs.extend(cross.as_chunks::<2>().0.iter().map(|&[a, b]| Interval::new(a, b)));
    // merged where they touch or overlap
    runs.sort_by(|x, y| x.lo.total_cmp(&y.lo));
    let mut out: Vec<Interval> = Vec::new();
    for r in runs {
        match out.last_mut() {
            Some(last) if r.lo <= last.hi => last.hi = last.hi.max(r.hi),
            _ => out.push(r),
        }
    }
    (out, moves > 0.05 * span.width())
}
