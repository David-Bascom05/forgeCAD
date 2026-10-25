use forge_math::{Box3, Point3, Vec3};

/// Indexed triangles, counter-clockwise seen from outside.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TriMesh {
    pub positions: Vec<Point3>,
    pub triangles: Vec<[u32; 3]>,
    /// Which body face each triangle came from (index into the body's faces).
    pub face_of: Vec<u32>,
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

    /// Triangles with (near) zero area.
    pub fn degenerate_triangles(&self, min_area: f64) -> usize {
        self.triangles
            .iter()
            .filter(|&&t| self.triangle_normal(t).length() * 0.5 <= min_area)
            .count()
    }
}
