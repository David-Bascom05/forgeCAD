use crate::name::Name;
use crate::small::Small;
use forge_geom::{Curve, CurveGeom, Surface, SurfaceGeom};
use forge_math::{Box3, Interval, Point2, Point3, UnitVec3};

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    pub coedges: Small<CoedgeId, 2>,
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
/// outward normal pointing at you, the face is on the left. A loop's coedges are numbered
/// consecutively, so it holds only the first and the count.
#[derive(Clone, Debug, PartialEq)]
pub struct Loop {
    first: u32,
    len: u32,
    pub face: FaceId,
}

impl Loop {
    pub fn coedges(&self) -> Coedges {
        Coedges {
            at: self.first,
            end: self.first + self.len,
        }
    }
    /// The loop's `i`th coedge.
    pub fn coedge(&self, i: usize) -> CoedgeId {
        assert!(i < self.len as usize);
        CoedgeId(self.first + i as u32)
    }
}

/// A loop's coedges, in order.
#[derive(Clone, Copy, Debug)]
pub struct Coedges {
    at: u32,
    end: u32,
}

impl Iterator for Coedges {
    type Item = CoedgeId;
    fn next(&mut self) -> Option<CoedgeId> {
        (self.at < self.end).then(|| {
            self.at += 1;
            CoedgeId(self.at - 1)
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.end - self.at) as usize;
        (n, Some(n))
    }
}

impl ExactSizeIterator for Coedges {}

impl DoubleEndedIterator for Coedges {
    fn next_back(&mut self) -> Option<CoedgeId> {
        (self.at < self.end).then(|| {
            self.end -= 1;
            CoedgeId(self.end)
        })
    }
}

/// A bounded region of a surface. `same_sense` says whether the face's outward
/// normal (away from the solid's material) is the surface's natural normal.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub surface: Surface,
    pub same_sense: bool,
    pub loops: Small<LoopId, 2>,
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

/// Names of faces and edges merged into another (alias, the name it went to), so a reference to
/// a merged piece still finds the whole; the face or edge each piece a boolean split off came
/// from (piece, whole), so pieces joined again take the whole's name; and edges between faces on
/// one surface that a feature made on purpose (a split face), which unifying leaves.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Joins {
    pub aliases: Vec<(Name, Name)>,
    pub pieces: Vec<(Name, Name)>,
    pub kept: Vec<Name>,
}

impl Joins {
    pub fn extend(&mut self, other: &Joins) {
        self.aliases.extend_from_slice(&other.aliases);
        self.pieces.extend_from_slice(&other.pieces);
        self.kept.extend_from_slice(&other.kept);
    }

    /// Every name passed through `rename`.
    pub fn renamed(&self, rename: impl Fn(Name) -> Name) -> Joins {
        Joins {
            aliases: self
                .aliases
                .iter()
                .map(|&(x, y)| (rename(x), rename(y)))
                .collect(),
            pieces: self.pieces.iter().map(|&(x, y)| (rename(x), rename(y))).collect(),
            kept: self.kept.iter().map(|&n| rename(n)).collect(),
        }
    }
}

/// A solid: one or more closed shells (an outer skin, plus inner ones for voids).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Body {
    /// Faces (and edges) removed by in-place edits and not yet compacted away.
    pub dead: usize,
    /// What `forge_bool::unify` must know beyond the entities: merged names and kept edges.
    pub joins: Joins,
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

    /// Drops what is cached of a face whose loops changed: its box, its queries and its run's hulls.
    pub fn face_changed(&mut self, f: FaceId) {
        let face = &mut self.faces[f.index()];
        face.bbox.clear();
        face.query.clear();
        if let Some(h) = self.hulls.get_mut() {
            h.chunks[f.index() / RUN].clear();
            h.blocks[f.index() / (RUN * RUN)].clear();
        }
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
            for c in self.loops[l.index()].coedges() {
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
        self.compact_by(&self.connected())
    }

    /// `compact` of a body no longer needed: the arenas' lists are reused, not copied.
    pub fn into_compact(self) -> Body {
        let group = self.connected();
        self.compact_owned(|f, _| group[f], group.len())
    }

    /// `into_compact`, each face kept in its shell, for edits that join faces of one shell but
    /// never part or join shells.
    pub fn compact_in_shells(self) -> Body {
        let n = self.shells.len();
        self.compact_owned(|_, s| s, n)
    }

    /// The live entities densely numbered, faces with one `group` value (of `groups`, from a
    /// face's index and its shell's) in one shell, made from this body's own lists.
    fn compact_owned(mut self, group: impl Fn(usize, usize) -> usize, groups: usize) -> Body {
        const NONE: u32 = u32::MAX;
        let mut vmap = vec![NONE; self.vertices.len()];
        let mut emap = vec![NONE; self.edges.len()];
        let mut smap = vec![NONE; groups];
        let mut out = Body {
            dead: 0,
            joins: std::mem::take(&mut self.joins),
            vertices: Vec::with_capacity(self.vertices.len()),
            edges: Vec::with_capacity(self.edges.len()),
            coedges: Vec::with_capacity(self.coedges.len()),
            loops: Vec::with_capacity(self.loops.len()),
            faces: Vec::with_capacity(self.faces.len()),
            shells: Vec::new(),
            hulls: Cached::default(),
        };
        for (i, mut face) in std::mem::take(&mut self.faces).into_iter().enumerate() {
            if !face.alive {
                continue;
            }
            let nf = FaceId(out.faces.len() as u32);
            let g = group(i, face.shell.index());
            if smap[g] == NONE {
                smap[g] = out.shells.len() as u32;
                out.shells.push(Shell { faces: Vec::new() });
            }
            out.shells[smap[g] as usize].faces.push(nf);
            face.shell = ShellId(smap[g]);
            face.query = Cached::default();
            for l in face.loops.iter_mut() {
                let cs = self.loops[l.index()].coedges();
                let nl = LoopId(out.loops.len() as u32);
                let first = out.coedges.len() as u32;
                for c in cs {
                    let Coedge { edge, forward, .. } = self.coedges[c.index()];
                    let oe = edge.index();
                    if emap[oe] == NONE {
                        emap[oe] = out.edges.len() as u32;
                        let x = &mut self.edges[oe];
                        let mut vert = |v: VertexId| {
                            if vmap[v.index()] == NONE {
                                vmap[v.index()] = out.vertices.len() as u32;
                                let w = &self.vertices[v.index()];
                                out.vertices.push(Vertex {
                                    tol: w.tol.max(forge_math::tol::LINEAR),
                                    ..w.clone()
                                });
                            }
                            VertexId(vmap[v.index()])
                        };
                        let (start, end) = (vert(x.start), vert(x.end));
                        out.edges.push(Edge {
                            curve: x.curve.clone(),
                            range: x.range,
                            start,
                            end,
                            coedges: Small::default(),
                            name: x.name,
                            alive: true,
                            bbox: Cached::default(),
                        });
                    }
                    let nc = CoedgeId(out.coedges.len() as u32);
                    out.coedges.push(Coedge {
                        edge: EdgeId(emap[oe]),
                        forward,
                        lp: nl,
                    });
                    out.edges[emap[oe] as usize].coedges.push(nc);
                }
                out.loops.push(Loop {
                    first,
                    len: cs.len() as u32,
                    face: nf,
                });
                *l = nl;
            }
            out.faces.push(face);
        }
        out
    }

    /// A compacted copy, faces with one `group` value in one shell.
    fn compact_by(&self, group: &[usize]) -> Body {
        let mut out = Body::new();
        out.joins = self.joins.clone();
        out.vertices.reserve(self.vertices.len());
        out.edges.reserve(self.edges.len());
        out.coedges.reserve(self.coedges.len());
        out.loops.reserve(self.loops.len());
        out.faces.reserve(self.faces.len());
        let mut uses = Vec::new();
        let mut vmap: Vec<Option<VertexId>> = vec![None; self.vertices.len()];
        let mut emap: Vec<Option<EdgeId>> = vec![None; self.edges.len()];
        let mut smap: Vec<Option<ShellId>> = vec![None; self.faces.len().max(self.shells.len())];
        for f in self.face_ids() {
            let face = self.face(f);
            let shell = *smap[group[f.index()]].get_or_insert_with(|| out.add_shell());
            let nf = out.add_face(shell, face.surface.clone(), face.same_sense, face.name);
            if let Some(b) = face.bbox.get() {
                out.faces[nf.index()].bbox.set(*b);
            }
            for &l in &face.loops {
                uses.clear();
                for c in self.lp(l).coedges() {
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
            first: x.first + c,
            len: x.len,
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
        self.joins.extend(&other.joins);
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
            coedges: Small::default(),
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
            loops: Small::default(),
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

    /// Loop `lp` (or, with None, a new loop of face `f` the caller puts on it) made of coedges
    /// `cs`, each now in some loop being remade or this one, as a new run of coedges: each edge's
    /// use moves to the copy. The old run stays in the arena until compaction.
    pub fn relink_loop(&mut self, lp: Option<LoopId>, f: FaceId, cs: &[CoedgeId]) -> LoopId {
        let l = lp.unwrap_or_else(|| {
            self.loops.push(Loop {
                first: 0,
                len: 0,
                face: f,
            });
            LoopId(self.loops.len() as u32 - 1)
        });
        let first = self.coedges.len() as u32;
        for &c in cs {
            let n = CoedgeId(self.coedges.len() as u32);
            let Coedge { edge, forward, .. } = self.coedges[c.index()];
            self.coedges.push(Coedge { edge, forward, lp: l });
            for x in self.edges[edge.index()].coedges.iter_mut() {
                if *x == c {
                    *x = n;
                }
            }
        }
        self.loops[l.index()] = Loop {
            first,
            len: cs.len() as u32,
            face: f,
        };
        l
    }

    /// Keeps only the coedges of loop `lp` that `keep` says, in order, in place: those after a
    /// dropped one move down its run (their edges told), the dropped ones leave their edges.
    pub fn retain_loop(&mut self, lp: LoopId, keep: impl Fn(&Body, CoedgeId) -> bool) {
        let Loop { first, len, .. } = self.loops[lp.index()];
        let mut to = first;
        for at in first..first + len {
            let c = CoedgeId(at);
            let e = self.coedges[at as usize].edge.index();
            if !keep(self, c) {
                self.edges[e].coedges.retain(|&x| x != c);
                continue;
            }
            if to != at {
                self.coedges[to as usize] = self.coedges[at as usize].clone();
                for x in self.edges[e].coedges.iter_mut() {
                    if *x == c {
                        *x = CoedgeId(to);
                    }
                }
            }
            to += 1;
        }
        self.loops[lp.index()].len = to - first;
    }

    /// Adds a loop to `face` made of the given (edge, forward) uses, in order.
    /// Takes loop `lp` off face `f`: its edges lose those uses (an edge left unused dies);
    /// the loop and its coedges stay in the arenas until compaction.
    pub fn remove_loop(&mut self, f: FaceId, lp: LoopId) {
        self.faces[f.index()].loops.retain(|&l| l != lp);
        for c in self.loops[lp.index()].coedges() {
            let edge = &mut self.edges[self.coedges[c.index()].edge.index()];
            edge.coedges.retain(|&x| x != c);
            if edge.coedges.is_empty() {
                edge.alive = false;
            }
        }
    }

    /// Replaces loop `old` of face `f` by a new one of `uses`, in its place among the face's loops.
    pub fn replace_loop(&mut self, f: FaceId, old: LoopId, uses: &[(EdgeId, bool)]) -> LoopId {
        let at = self
            .face(f)
            .loops
            .iter()
            .position(|&x| x == old)
            .expect("its face's loop");
        self.remove_loop(f, old);
        let nl = self.add_loop(f, uses);
        self.faces[f.index()].loops[at..].rotate_right(1);
        nl
    }

    pub fn add_loop(&mut self, face: FaceId, uses: &[(EdgeId, bool)]) -> LoopId {
        let lp = LoopId(self.loops.len() as u32);
        let first = self.coedges.len() as u32;
        for &(edge, forward) in uses {
            let c = CoedgeId(self.coedges.len() as u32);
            self.coedges.push(Coedge { edge, forward, lp });
            self.edges[edge.index()].coedges.push(c);
            self.edges[edge.index()].alive = true;
        }
        self.loops.push(Loop {
            first,
            len: uses.len() as u32,
            face,
        });
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

    /// The live coedges leaving each vertex.
    pub fn leaving(&self) -> Vec<Vec<CoedgeId>> {
        let mut out = vec![Vec::new(); self.vertices.len()];
        // the edges' own coedges are the live ones (a loop taken off a face drops its own)
        for e in self.edge_ids() {
            for &c in &self.edge(e).coedges {
                out[self.coedge_start(c).index()].push(c);
            }
        }
        out
    }

    /// The fans of faces round a vertex that coedges `from` (all leaving it) lie in: from one
    /// leaving it, back along its loop to the one arriving, then across to that one's partner,
    /// which leaves it again. A closed 2-manifold has one fan at every vertex. `prev` caches each
    /// coedge's predecessor in its loop (filled a loop at a time, as loops are reached).
    pub fn fans(&self, from: &[CoedgeId], prev: &mut Vec<CoedgeId>) -> Vec<Vec<CoedgeId>> {
        const NONE: CoedgeId = CoedgeId(u32::MAX);
        prev.resize(self.coedges.len(), NONE);
        let mut before = |c: CoedgeId| {
            if prev[c.index()] == NONE {
                let lp = self.lp(self.coedge(c).lp);
                let n = lp.coedges().len();
                for (i, x) in lp.coedges().enumerate() {
                    prev[x.index()] = lp.coedge((i + n - 1) % n);
                }
            }
            prev[c.index()]
        };
        let mut seen: Vec<CoedgeId> = Vec::new();
        let mut fans = Vec::new();
        for &c0 in from {
            if seen.contains(&c0) {
                continue;
            }
            let mut fan = Vec::new();
            let mut c = c0;
            while !seen.contains(&c) {
                seen.push(c);
                fan.push(c);
                match self.partner(before(c)) {
                    Some(n) => c = n,
                    None => break,
                }
            }
            fans.push(fan);
        }
        fans
    }

    /// Gives each fan past the first of those coedges `from` (all leaving vertex `v`) lie in a
    /// vertex of its own at the same point: solids touching at a point stay manifold, as
    /// Parasolid and ACIS keep them.
    pub fn split_pinched(&mut self, v: VertexId, from: &[CoedgeId], prev: &mut Vec<CoedgeId>) {
        for (k, fan) in self.fans(from, prev).iter().enumerate().skip(1) {
            let old = self.vertices[v.index()].clone();
            let nv = self.add_tolerant_vertex(old.point, old.tol, old.name.with(k as u64));
            for &c in fan {
                let (edge, forward) = (self.coedge(c).edge, self.coedge(c).forward);
                let e = &mut self.edges[edge.index()];
                if forward {
                    e.start = nv;
                } else {
                    e.end = nv;
                }
            }
        }
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
                for c in self.lp(l).coedges() {
                    b = b.hull(&self.edge_box(self.coedge(c).edge));
                }
            }
            match &face.surface {
                Surface::Sphere(sp) => b = b.hull(&Box3::from_point(sp.frame.origin).grow(sp.radius)),
                Surface::Torus(t) => b = b.hull(&Box3::from_point(t.frame.origin).grow(t.major + t.minor)),
                Surface::Procedural(pr) => b = b.hull(&crate::uv::procedural_box(self, f, pr)),
                _ => {}
            }
            b.grow(forge_math::tol::LINEAR)
        })
    }

    pub fn face_by_name(&self, name: Name) -> Option<FaceId> {
        self.by_name(name, |n| self.face_ids().find(|&f| self.face(f).name == n))
    }
    pub fn edge_by_name(&self, name: Name) -> Option<EdgeId> {
        self.by_name(name, |n| self.edge_ids().find(|&e| self.edge(e).name == n))
    }

    /// `find` of `name`, else of the names it went to, in turn.
    fn by_name<T>(&self, mut name: Name, find: impl Fn(Name) -> Option<T>) -> Option<T> {
        for _ in 0..=self.joins.aliases.len() {
            if let Some(x) = find(name) {
                return Some(x);
            }
            name = self.alias(name)?;
        }
        None
    }

    /// The name a merged entity's `name` went to.
    pub fn alias(&self, name: Name) -> Option<Name> {
        self.joins.aliases.iter().rev().find(|a| a.0 == name).map(|a| a.1)
    }
}

/// A body's entities in the order they are made, its loops not yet linked to their edges: what
/// a boolean can read of a tool without building the Body (`forge_ops::extrude_unlinked`).
#[derive(Clone, Debug, Default)]
pub struct Unlinked {
    pub vertices: Vec<(Point3, Name)>,
    pub edges: Vec<(Curve, Interval, VertexId, VertexId, Name)>,
    pub faces: Vec<(Surface, bool, Name)>,
    /// Each loop's face and the end of its uses in `uses` (they start at the previous loop's end).
    pub loops: Vec<(FaceId, u32)>,
    pub uses: Vec<(EdgeId, bool)>,
}

impl Unlinked {
    pub fn add_vertex(&mut self, p: Point3, name: Name) -> VertexId {
        self.vertices.push((p, name));
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
        self.edges.push((curve, range, start, end, name));
        EdgeId(self.edges.len() as u32 - 1)
    }
    pub fn add_face(&mut self, surface: Surface, same_sense: bool, name: Name) -> FaceId {
        self.faces.push((surface, same_sense, name));
        FaceId(self.faces.len() as u32 - 1)
    }
    pub fn add_loop(&mut self, face: FaceId, uses: &[(EdgeId, bool)]) {
        self.uses.extend_from_slice(uses);
        self.loops.push((face, self.uses.len() as u32));
    }

    /// The loops of face `f`, each as its uses.
    pub fn loops_of(&self, f: FaceId) -> impl Iterator<Item = &[(EdgeId, bool)]> {
        let starts = std::iter::once(0).chain(self.loops.iter().map(|l| l.1));
        self.loops
            .iter()
            .zip(starts)
            .filter(move |(l, _)| l.0 == f)
            .map(|(l, at)| &self.uses[at as usize..l.1 as usize])
    }

    pub fn into_body(self) -> Body {
        let mut body = Body::new();
        let shell = body.add_shell();
        for (p, name) in self.vertices {
            body.add_vertex(p, name);
        }
        for (curve, range, start, end, name) in self.edges {
            body.add_edge(curve, range, start, end, name);
        }
        for (surface, same_sense, name) in self.faces {
            body.add_face(shell, surface, same_sense, name);
        }
        let mut at = 0;
        for (f, end) in self.loops {
            body.add_loop(f, &self.uses[at..end as usize]);
            at = end as usize;
        }
        body
    }
}
