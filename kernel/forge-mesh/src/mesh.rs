use forge_math::{Box3, Point3, Vec3};

/// Indexed triangles, counter-clockwise seen from outside.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TriMesh {
    pub positions: Vec<Point3>,
    pub triangles: Vec<[u32; 3]>,
    /// Which body face each triangle came from (index into the body's faces).
    pub face_of: Vec<u32>,
    /// Each body edge's cut points as vertex indices, start to end (index into the body's edges;
    /// empty where the mesh wasn't made from a whole body).
    pub edges: Vec<Vec<u32>>,
}

impl TriMesh {
    pub fn add_vertex(&mut self, p: Point3) -> u32 {
        self.positions.push(p);
        self.positions.len() as u32 - 1
    }

    fn corners(&self, t: [u32; 3]) -> [Point3; 3] {
        t.map(|i| self.positions[i as usize])
    }

    /// Enclosed volume (positive when the mesh is closed and faces outward).
    pub fn volume(&self) -> f64 {
        self.triangles
            .iter()
            .map(|&t| {
                let [a, b, c] = self.corners(t);
                a.to_vec().dot(b.to_vec().cross(c.to_vec()))
            })
            .sum::<f64>()
            / 6.0
    }

    pub fn area(&self) -> f64 {
        self.triangles
            .iter()
            .map(|&t| {
                let [a, b, c] = self.corners(t);
                (b - a).cross(c - a).length() * 0.5
            })
            .sum()
    }

    pub fn bbox(&self) -> Box3 {
        Box3::from_points(self.positions.iter().copied())
    }

    pub fn triangle_normal(&self, t: [u32; 3]) -> Vec3 {
        let [a, b, c] = self.corners(t);
        (b - a).cross(c - a)
    }

    /// Edges not shared by exactly one triangle in each direction. Empty means
    /// the mesh is closed, manifold and consistently oriented.
    pub fn open_edges(&self) -> Vec<(u32, u32)> {
        let mut directed: Vec<(u32, u32)> = self
            .triangles
            .iter()
            .flat_map(|&[a, b, c]| [(a, b), (b, c), (c, a)])
            .collect();
        directed.sort_unstable();
        let mut bad = Vec::new();
        for w in directed.windows(2) {
            if w[0] == w[1] {
                bad.push(w[0]);
            }
        }
        for &(a, b) in &directed {
            if directed.binary_search(&(b, a)).is_err() {
                bad.push((a, b));
            }
        }
        bad.sort_unstable();
        bad.dedup();
        bad
    }

    /// `open_edges` of the mesh of `body` (`tessellate`'s, with `edges`), less each mesh edge along
    /// a sheet's boundary (a body edge used once, by a sheet face) where it is used once: empty means
    /// every other edge is shared by one triangle each way, and the boundary's once.
    pub fn open_edges_of(&self, body: &forge_topo::Body) -> Vec<(u32, u32)> {
        let mut rim: Vec<(u32, u32)> = Vec::new();
        for e in body.edge_ids() {
            let edge = body.edge(e);
            if edge.coedges.len() != 1 || !body.face(body.coedge_face(edge.coedges[0])).sheet {
                continue;
            }
            for w in self.edges.get(e.index()).map_or(&[][..], |v| &v[..]).windows(2) {
                rim.push((w[0].min(w[1]), w[0].max(w[1])));
            }
        }
        rim.sort_unstable();
        let mut directed: Vec<(u32, u32)> = self
            .triangles
            .iter()
            .flat_map(|&[a, b, c]| [(a, b), (b, c), (c, a)])
            .collect();
        directed.sort_unstable();
        self.open_edges()
            .into_iter()
            .filter(|&(a, b)| {
                let on_rim = rim.binary_search(&(a.min(b), a.max(b))).is_ok();
                let once = directed.partition_point(|x| *x < (a, b)) + 1
                    == directed.partition_point(|x| *x <= (a, b));
                !(on_rim && once && directed.binary_search(&(b, a)).is_err())
            })
            .collect()
    }

    /// Triangles with (near) zero area.
    pub fn degenerate_triangles(&self, min_area: f64) -> usize {
        self.triangles
            .iter()
            .filter(|&&t| self.triangle_normal(t).length() * 0.5 <= min_area)
            .count()
    }
}
