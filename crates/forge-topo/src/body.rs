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
}

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
        let n = self.faces.len();
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(p: &mut [usize], mut i: usize) -> usize {
            while p[i] != i {
                p[i] = p[p[i]];
                i = p[i];
            }
            i
        }
        for e in 0..self.edges.len() {
            if !self.edges[e].alive {
                continue;
            }
            let faces: Vec<usize> = self.edges[e]
                .coedges
                .iter()
                .map(|&c| self.coedge_face(c).index())
                .collect();
            for w in faces.windows(2) {
                let (x, y) = (find(&mut parent, w[0]), find(&mut parent, w[1]));
                if x != y {
                    parent[x.max(y)] = x.min(y);
                }
            }
        }
        self.shells.clear();
        let mut shell_of: Vec<Option<ShellId>> = vec![None; n];
        for f in 0..n {
            if !self.faces[f].alive {
                continue;
            }
            let r = find(&mut parent, f);
            let s = match shell_of[r] {
                Some(s) => s,
                None => {
                    self.shells.push(Shell { faces: Vec::new() });
                    let s = ShellId(self.shells.len() as u32 - 1);
                    shell_of[r] = Some(s);
                    s
                }
            };
            self.faces[f].shell = s;
            self.shells[s.index()].faces.push(FaceId(f as u32));
        }
    }

    /// A copy holding only live entities, densely numbered, with shells
    /// regrouped by connectivity.
    pub fn compact(&self) -> Body {
        let mut grouped = self.clone();
        grouped.rebuild_shells();
        grouped.compact_grouped()
    }

    fn compact_grouped(&self) -> Body {
        let mut out = Body::new();
        let mut vmap: Vec<Option<VertexId>> = vec![None; self.vertices.len()];
        let mut emap: Vec<Option<EdgeId>> = vec![None; self.edges.len()];
        let mut smap: Vec<Option<ShellId>> = vec![None; self.shells.len()];
        for f in self.face_ids() {
            let face = self.face(f);
            let shell = *smap[face.shell.index()].get_or_insert_with(|| out.add_shell());
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
                                    out.add_vertex(self.vertex(v).point, self.vertex(v).name)
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

    pub fn add_vertex(&mut self, point: Point3, name: Name) -> VertexId {
        self.vertices.push(Vertex { point, name });
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
    /// sides); spheres and tori can bulge past their edges, so their whole
    /// surface box is added. Cached on the face.
    /// A box holding edge `e` (cached on the edge).
    pub fn edge_box(&self, e: EdgeId) -> Box3 {
        let edge = self.edge(e);
        *edge.bbox.get_or_init(|| edge.curve.bbox(edge.range))
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
