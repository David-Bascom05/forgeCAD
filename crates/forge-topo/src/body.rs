use crate::name::Name;
use forge_geom::{Curve, CurveGeom, Surface, SurfaceGeom};
use forge_math::{Interval, Point2, Point3, UnitVec3};

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
}

/// A connected, closed set of faces: the boundary between material and space.
#[derive(Clone, Debug, PartialEq)]
pub struct Shell {
    pub faces: Vec<FaceId>,
}

/// A solid: one or more closed shells (an outer skin, plus inner ones for voids).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Body {
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
        (0..self.edges.len() as u32).map(EdgeId)
    }
    pub fn face_ids(&self) -> impl Iterator<Item = FaceId> + '_ {
        (0..self.faces.len() as u32).map(FaceId)
    }
    pub fn loop_ids(&self) -> impl Iterator<Item = LoopId> + '_ {
        (0..self.loops.len() as u32).map(LoopId)
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
        });
        self.shells[shell.index()].faces.push(id);
        id
    }

    /// Adds a loop to `face` made of the given (edge, forward) uses, in order.
    pub fn add_loop(&mut self, face: FaceId, uses: &[(EdgeId, bool)]) -> LoopId {
        let lp = LoopId(self.loops.len() as u32);
        let mut coedges = Vec::with_capacity(uses.len());
        for &(edge, forward) in uses {
            let c = CoedgeId(self.coedges.len() as u32);
            self.coedges.push(Coedge { edge, forward, lp });
            self.edges[edge.index()].coedges.push(c);
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

    pub fn face_by_name(&self, name: Name) -> Option<FaceId> {
        self.face_ids().find(|&f| self.face(f).name == name)
    }
    pub fn edge_by_name(&self, name: Name) -> Option<EdgeId> {
        self.edge_ids().find(|&e| self.edge(e).name == name)
    }
}
