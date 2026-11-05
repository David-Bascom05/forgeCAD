//! The formed part as the thickness model leaves it: the sheet's surface on the tool, trimmed to
//! the trim outline, and the same surface moved off the tool by the wall the model gives at each
//! point, closed round the trim by a rim. It is fitted from the model's grid (a mesh, not a
//! B-rep): for mass, for looking at and for interference, never for dimensions or joints.
use crate::stretch::Stretch;
use forge_math::{Frame, Point2, Point3, Vec3};
use forge_mesh::TriMesh;
use std::collections::HashMap;

/// Which side of the simulated wall a triangle is on (`TriMesh::face_of`).
pub const TOOL_SIDE: u32 = 0;
pub const FREE_SIDE: u32 = 1;
pub const RIM: u32 = 2;

/// Volume, centre and inertia of a closed mesh, unit density.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshMass {
    pub volume: f64,
    pub centroid: Point3,
    /// About the centroid; diagonal the moments, off it the negated products (as forge-topo's).
    pub inertia: [[f64; 3]; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Simulated {
    /// Closed and facing out: the tool side's vertices first, then the free side's, in step.
    pub mesh: TriMesh,
    /// The wall at each tool-side vertex, mm.
    pub wall: Vec<f64>,
    /// The tool side's area, and the integral of the wall over the wall's middle surface (what
    /// the membrane conserves): the mesh's volume is within 0.5 % of it on formed parts.
    pub area: f64,
    pub sheet_volume: f64,
    pub mass: MeshMass,
}

/// The simulated part of `st`, a sheet `t` thick formed with its trim plane `frame` (z the pull)
/// and trimmed to `trim` (on the plane's axes).
pub fn simulated(frame: &Frame, trim: &[Point2], st: &Stretch, t: f64) -> Simulated {
    let poly = clean(trim);
    let convex = is_convex(&poly);
    let local: Vec<Point3> = st.nodes.iter().map(|p| frame.point_to_local(*p)).collect();
    let xy = |k: usize| Point2::new(local[k].x, local[k].y);
    // each node's normal: its triangles' by area, the side the sheet came from (off the tool)
    let mut normal = vec![Vec3::ZERO; st.nodes.len()];
    for v in &st.tris {
        let [a, b, c] = v.map(|i| st.nodes[i as usize]);
        let n = (b - a).cross(c - a);
        for &i in v {
            normal[i as usize] += n;
        }
    }
    let normal: Vec<Vec3> = normal
        .into_iter()
        .map(|n| n.normalize().map_or(frame.z().get(), |u| u.get()))
        .collect();
    // mitred: at a corner the node moves along the mean normal far enough that each side's
    // own wall is the model's, not less
    let mut lean = vec![(0.0, 0.0); st.nodes.len()];
    for v in &st.tris {
        let [a, b, c] = v.map(|i| st.nodes[i as usize]);
        let n = (b - a).cross(c - a);
        let (s, Some(u)) = (n.length(), n.normalize()) else {
            continue;
        };
        for &i in v {
            let l = &mut lean[i as usize];
            l.0 += s * normal[i as usize].dot(u.get());
            l.1 += s;
        }
    }
    let normal: Vec<Vec3> = normal
        .iter()
        .zip(&lean)
        .map(|(n, l)| *n * (1.0 / if l.1 > 0.0 { (l.0 / l.1).max(0.5) } else { 1.0 }))
        .collect();
    let wall: Vec<f64> = st.node_ratio.iter().map(|r| r * t).collect();

    let mut out = Builder::default();
    for v in &st.tris {
        let ids = v.map(|i| i as usize);
        let corners: Vec<Corner> = ids.iter().map(|&k| Corner::Node(k, xy(k))).collect();
        let clipped = if convex {
            clip(corners, &poly)
        } else {
            let c = ids.iter().fold(Point2::new(0.0, 0.0), |s, &k| {
                Point2::new(s.x + xy(k).x / 3.0, s.y + xy(k).y / 3.0)
            });
            if crate::build::inside(&poly, c).0 {
                corners
            } else {
                vec![]
            }
        };
        if clipped.len() < 3 {
            continue;
        }
        let at = |c: &Corner| -> usize {
            match *c {
                Corner::Node(k, _) => out.node(k, st.nodes[k], normal[k], wall[k]),
                Corner::Cut(p) => {
                    // on the triangle, by its corners' weights in the plane
                    let w = weights(p, ids.map(xy));
                    let mix3 =
                        |f: &dyn Fn(usize) -> Vec3| f(ids[0]) * w[0] + f(ids[1]) * w[1] + f(ids[2]) * w[2];
                    let pos = Point3::ORIGIN + mix3(&|k| st.nodes[k] - Point3::ORIGIN);
                    let n = mix3(&|k| normal[k]);
                    let h = wall[ids[0]] * w[0] + wall[ids[1]] * w[1] + wall[ids[2]] * w[2];
                    out.cut(p, pos, n, h)
                }
            }
        };
        let idx: Vec<usize> = clipped.iter().map(at).collect();
        for k in 1..idx.len() - 1 {
            if idx[0] != idx[k] && idx[k] != idx[k + 1] && idx[0] != idx[k + 1] {
                out.tris.push([idx[0], idx[k], idx[k + 1]]);
            }
        }
    }
    out.finish()
}

/// A corner of a clipped triangle: one of the sheet's nodes, or where the trim cuts it.
#[derive(Clone, Copy, Debug)]
enum Corner {
    Node(usize, Point2),
    Cut(Point2),
}

impl Corner {
    fn xy(&self) -> Point2 {
        match *self {
            Corner::Node(_, p) | Corner::Cut(p) => p,
        }
    }
}

/// The polygon without repeated points, counter-clockwise.
fn clean(trim: &[Point2]) -> Vec<Point2> {
    let mut p: Vec<Point2> = vec![];
    for &q in trim {
        if p.last().is_none_or(|l| (*l - q).length() > 1e-9) {
            p.push(q);
        }
    }
    while p.len() > 1 && (p[0] - p[p.len() - 1]).length() <= 1e-9 {
        p.pop();
    }
    let area: f64 = (0..p.len())
        .map(|i| {
            let (a, b) = (p[i], p[(i + 1) % p.len()]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    if area < 0.0 {
        p.reverse();
    }
    p
}

fn cross(o: Point2, a: Point2, b: Point2) -> f64 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

fn is_convex(p: &[Point2]) -> bool {
    p.len() >= 3 && (0..p.len()).all(|i| cross(p[i], p[(i + 1) % p.len()], p[(i + 2) % p.len()]) >= -1e-9)
}

/// Sutherland-Hodgman: the triangle's part inside the convex polygon.
fn clip(mut poly: Vec<Corner>, clipper: &[Point2]) -> Vec<Corner> {
    for i in 0..clipper.len() {
        if poly.is_empty() {
            break;
        }
        let (a, b) = (clipper[i], clipper[(i + 1) % clipper.len()]);
        let side = |p: Point2| cross(a, b, p);
        let input = std::mem::take(&mut poly);
        for k in 0..input.len() {
            let (p, q) = (input[k], input[(k + 1) % input.len()]);
            let (sp, sq) = (side(p.xy()), side(q.xy()));
            if sp >= 0.0 {
                poly.push(p);
            }
            if (sp >= 0.0) != (sq >= 0.0) {
                let f = sp / (sp - sq);
                let (u, v) = (p.xy(), q.xy());
                poly.push(Corner::Cut(Point2::new(
                    u.x + (v.x - u.x) * f,
                    u.y + (v.y - u.y) * f,
                )));
            }
        }
    }
    poly
}

/// A point's weights on a triangle in the plane.
fn weights(p: Point2, [a, b, c]: [Point2; 3]) -> [f64; 3] {
    let d = cross(a, b, c);
    if d.abs() < 1e-18 {
        return [1.0 / 3.0; 3];
    }
    let wa = cross(p, b, c) / d;
    let wb = cross(a, p, c) / d;
    [wa, wb, 1.0 - wa - wb]
}

#[derive(Default)]
struct Builder {
    pos: Vec<Point3>,
    normal: Vec<Vec3>,
    wall: Vec<f64>,
    nodes: HashMap<usize, usize>,
    cuts: HashMap<(i64, i64), usize>,
    tris: Vec<[usize; 3]>,
}

impl Builder {
    fn push(&mut self, p: Point3, n: Vec3, h: f64) -> usize {
        self.pos.push(p);
        self.normal.push(n);
        self.wall.push(h);
        self.pos.len() - 1
    }

    fn node(&mut self, k: usize, p: Point3, n: Vec3, h: f64) -> usize {
        if let Some(&i) = self.nodes.get(&k) {
            return i;
        }
        let i = self.push(p, n, h);
        self.nodes.insert(k, i);
        i
    }

    // two triangles cut on their shared side meet at the same point, to far below a micron
    fn cut(&mut self, at: Point2, p: Point3, n: Vec3, h: f64) -> usize {
        let key = ((at.x * 1e5).round() as i64, (at.y * 1e5).round() as i64);
        if let Some(&i) = self.cuts.get(&key) {
            return i;
        }
        let i = self.push(p, n, h);
        self.cuts.insert(key, i);
        i
    }

    fn finish(self) -> Simulated {
        let n = self.pos.len();
        let mut mesh = TriMesh::default();
        mesh.positions.extend(self.pos.iter().copied());
        mesh.positions
            .extend((0..n).map(|i| self.pos[i] + self.normal[i] * self.wall[i]));
        let u = |i: usize| i as u32;
        let (mut area, mut sheet_volume) = (0.0, 0.0);
        let mut directed: HashMap<(usize, usize), usize> = HashMap::new();
        let mid = |i: usize| self.pos[i] + self.normal[i] * (0.5 * self.wall[i]);
        for &[a, b, c] in &self.tris {
            area += 0.5
                * (self.pos[b] - self.pos[a])
                    .cross(self.pos[c] - self.pos[a])
                    .length();
            // over the wall's middle, where the sheet's area is what the membrane conserves
            let s = 0.5 * (mid(b) - mid(a)).cross(mid(c) - mid(a)).length();
            sheet_volume += s * (self.wall[a] + self.wall[b] + self.wall[c]) / 3.0;
            mesh.triangles.push([u(a), u(c), u(b)]);
            mesh.face_of.push(TOOL_SIDE);
            mesh.triangles.push([u(a + n), u(b + n), u(c + n)]);
            mesh.face_of.push(FREE_SIDE);
            for (p, q) in [(a, b), (b, c), (c, a)] {
                *directed.entry((p, q)).or_default() += 1;
            }
        }
        // the rim: round every side only one triangle has
        let mut rim: Vec<(usize, usize)> = directed
            .keys()
            .filter(|(p, q)| !directed.contains_key(&(*q, *p)))
            .copied()
            .collect();
        rim.sort_unstable();
        for (p, q) in rim {
            mesh.triangles.push([u(p), u(q), u(q + n)]);
            mesh.triangles.push([u(p), u(q + n), u(p + n)]);
            mesh.face_of.extend([RIM, RIM]);
        }
        let mass = mesh_mass(&mesh);
        Simulated {
            mesh,
            wall: self.wall,
            area,
            sheet_volume,
            mass,
        }
    }
}

/// A closed mesh's mass properties: a tetrahedron from a point inside to each triangle.
pub fn mesh_mass(m: &TriMesh) -> MeshMass {
    let bb = m.bbox();
    let o = bb.center();
    let (mut vol, mut first, mut second) = (0.0, Vec3::ZERO, [[0.0f64; 3]; 3]);
    for t in &m.triangles {
        let [a, b, c] = t.map(|i| m.positions[i as usize] - o);
        let v = a.dot(b.cross(c)) / 6.0;
        vol += v;
        let s = a + b + c;
        first += s * (v / 4.0);
        for (i, row) in second.iter_mut().enumerate() {
            for (j, x) in row.iter_mut().enumerate() {
                *x += v / 20.0 * (a[i] * a[j] + b[i] * b[j] + c[i] * c[j] + s[i] * s[j]);
            }
        }
    }
    if vol.abs() < 1e-300 {
        return MeshMass {
            volume: 0.0,
            centroid: o,
            inertia: [[0.0; 3]; 3],
        };
    }
    let g = first * (1.0 / vol);
    let s = |i: usize, j: usize| second[i][j] - vol * g[i] * g[j];
    let tr = s(0, 0) + s(1, 1) + s(2, 2);
    MeshMass {
        volume: vol,
        centroid: o + g,
        inertia: std::array::from_fn(|i| {
            std::array::from_fn(|j| if i == j { tr - s(i, i) } else { -s(i, j) })
        }),
    }
}

/// The blank before forming: the sheet cut to `size` (on the trim plane's axes, round the trim
/// box's middle), `t` thick, lying at `z` along the pull.
pub fn blank(frame: &Frame, middle: Point2, size: (f64, f64), z: f64, t: f64) -> TriMesh {
    let (w, l) = (size.0 * 0.5, size.1 * 0.5);
    let mut m = TriMesh::default();
    for (dz, (dx, dy)) in [0.0, t]
        .iter()
        .flat_map(|&dz| [(-w, -l), (w, -l), (w, l), (-w, l)].map(|c| (dz, c)))
    {
        m.positions
            .push(frame.point_to_world(Point3::new(middle.x + dx, middle.y + dy, z + dz)));
    }
    let quads = [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [1, 2, 6, 5],
        [2, 3, 7, 6],
        [3, 0, 4, 7],
    ];
    for (f, q) in quads.iter().enumerate() {
        m.triangles.push([q[0], q[1], q[2]]);
        m.triangles.push([q[0], q[2], q[3]]);
        m.face_of.extend([f as u32, f as u32]);
    }
    m
}
