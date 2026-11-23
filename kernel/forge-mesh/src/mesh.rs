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
    /// Faces whose refinement stopped at its cap, drawn as far as it got: their triangles are not
    /// certified within the chord. Only a file's body, drawn as it is (`tessellate_tolerant`), has
    /// any; on a body made here an unsettled face is an error (`CdtError::Unsettled`).
    pub unsettled: Vec<u32>,
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

    /// Edges not shared by exactly one triangle in each direction. Empty means the mesh is closed
    /// and consistently oriented, and every edge is manifold; it is a manifold surface only if
    /// `pinched_vertices` is empty too (two tetrahedra sharing one vertex have no open edge).
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

    /// The used vertices whose triangles do not make one fan (walking from triangle to triangle
    /// across the edges at the vertex): a pinch, where two sheets of the surface touch at a point.
    /// Empty, with no `open_edges`, means a closed manifold surface (verify/forge-mesh-check's
    /// `vertex_fans`, proved in its lean/).
    pub fn pinched_vertices(&self) -> Vec<u32> {
        // at each vertex, the step (a, b) its corner in each triangle makes (a, b the other two,
        // in order): a fan walks from the b of one step to the step whose a it is
        let mut at: Vec<Vec<(u32, u32)>> = vec![Vec::new(); self.positions.len()];
        for &[a, b, c] in &self.triangles {
            at[a as usize].push((b, c));
            at[b as usize].push((c, a));
            at[c as usize].push((a, b));
        }
        let mut out = Vec::new();
        for (v, steps) in at.iter().enumerate() {
            let mut seen = vec![false; steps.len()];
            let mut fans = 0;
            for s in 0..steps.len() {
                if seen[s] {
                    continue;
                }
                fans += 1;
                let mut k = s;
                while !seen[k] {
                    seen[k] = true;
                    let to = steps[k].1;
                    match steps.iter().position(|x| x.0 == to) {
                        Some(j) => k = j,
                        None => break,
                    }
                }
            }
            if fans > 1 {
                out.push(v as u32);
            }
        }
        out
    }

    /// Triangles with (near) zero area.
    pub fn degenerate_triangles(&self, min_area: f64) -> usize {
        self.triangles
            .iter()
            .filter(|&&t| self.triangle_normal(t).length() * 0.5 <= min_area)
            .count()
    }
}
