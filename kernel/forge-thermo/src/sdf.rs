//! The tool as a 3-D signed distance field over the clamp window, on the trim plane's axes: the
//! distance from any point to the tool's surface (the shape's faces the sheet meets, and the table
//! or the tool's top round them), positive on the sheet's side. Exact at the nodes of a narrow band
//! of cells either side of the surface, trilinear between them; the gradient is the surface's
//! normal. A height field (field.rs) knows only the height above each cell: no normal, steep walls
//! smeared over a cell, no undercut. This knows all three.
use crate::field::Field;
use forge_math::{Frame, Point3, Vec3};
use forge_mesh::TriMesh;
use std::collections::HashMap;

/// A brick's nodes along each axis.
const B: usize = 8;
/// Within this many node spacings of the surface a distance is worked out exactly.
const NEAR: f64 = 3.0;

pub struct Sdf {
    /// The trim plane: x, y the field's axes, z the pull; every point here is on its axes.
    pub frame: Frame,
    origin: Vec3,
    /// The node spacing, and how far either side of the surface the nodes are exact.
    pub h: f64,
    pub band: f64,
    bricks: HashMap<(i32, i32, i32), Box<[f32; B * B * B]>>,
    surface: Surface,
}

/// The tool's surface on the plane's axes: its triangles (each facing the sheet's side) with
/// pseudo-normals at its edges and corners, a grid of them, and the table (z = 0) where the shape
/// does not stand on it.
struct Surface {
    pos: Vec<Vec3>,
    tris: Vec<[u32; 3]>,
    normal: Vec<Vec3>,
    vertex_normal: Vec<Vec3>,
    edge_normal: HashMap<(u32, u32), Vec3>,
    cell: f64,
    cells: HashMap<(i64, i64, i64), Vec<u32>>,
    table: Field,
}

/// The nearest point of the surface: where, how far, and which side.
#[derive(Clone, Copy, Debug)]
pub struct Nearest {
    pub at: Vec3,
    /// Signed: positive on the sheet's side.
    pub distance: f64,
}

impl Surface {
    fn new(frame: &Frame, mesh: &TriMesh, tris: &[usize], male: bool, table: Field, reach: f64) -> Surface {
        let mut index: HashMap<u32, u32> = HashMap::new();
        let mut pos = vec![];
        let mut out = vec![];
        for &k in tris {
            let mut t = mesh.triangles[k].map(|i| {
                *index.entry(i).or_insert_with(|| {
                    let p = frame.point_to_local(mesh.positions[i as usize]);
                    pos.push(Vec3::new(p.x, p.y, p.z));
                    (pos.len() - 1) as u32
                })
            });
            // a male tool's faces face the sheet; a female's (the cavity's) face into the tool
            if !male {
                t.swap(1, 2);
            }
            out.push(t);
        }
        let normal: Vec<Vec3> = out
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| pos[i as usize]);
                (b - a).cross(c - a).normalize().map_or(Vec3::ZERO, |u| u.get())
            })
            .collect();
        // angle-weighted at a corner, the two faces' at a side (Baerentzen & Aanaes 2005): the sign
        // of the distance is then right wherever the nearest point is
        let mut vertex_normal = vec![Vec3::ZERO; pos.len()];
        let mut edge_normal: HashMap<(u32, u32), Vec3> = HashMap::new();
        for (t, n) in out.iter().zip(&normal) {
            for k in 0..3 {
                let (a, b, c) = (t[k], t[(k + 1) % 3], t[(k + 2) % 3]);
                let (u, v) = (
                    pos[b as usize] - pos[a as usize],
                    pos[c as usize] - pos[a as usize],
                );
                let angle = match (u.normalize(), v.normalize()) {
                    (Some(u), Some(v)) => forge_math::dmath::acos(u.get().dot(v.get()).clamp(-1.0, 1.0)),
                    _ => 0.0,
                };
                vertex_normal[a as usize] += *n * angle;
                *edge_normal.entry((a.min(b), a.max(b))).or_insert(Vec3::ZERO) += *n;
            }
        }
        let cell = reach.max(0.5);
        let key = |v: f64| (v / cell).floor() as i64;
        let mut cells: HashMap<(i64, i64, i64), Vec<u32>> = HashMap::new();
        for (k, t) in out.iter().enumerate() {
            let [a, b, c] = t.map(|i| pos[i as usize]);
            let lo = [
                a.x.min(b.x).min(c.x),
                a.y.min(b.y).min(c.y),
                a.z.min(b.z).min(c.z),
            ];
            let hi = [
                a.x.max(b.x).max(c.x),
                a.y.max(b.y).max(c.y),
                a.z.max(b.z).max(c.z),
            ];
            for i in key(lo[0] - reach)..=key(hi[0] + reach) {
                for j in key(lo[1] - reach)..=key(hi[1] + reach) {
                    for l in key(lo[2] - reach)..=key(hi[2] + reach) {
                        cells.entry((i, j, l)).or_default().push(k as u32);
                    }
                }
            }
        }
        Surface {
            pos,
            tris: out,
            normal,
            vertex_normal,
            edge_normal,
            cell,
            cells,
            table,
        }
    }

    /// Whether the table is there under a point: the window's cells the shape does not stand on.
    fn table_at(&self, p: Vec3) -> bool {
        let f = &self.table;
        let i = ((p.x - f.x0) / f.h).floor();
        let j = ((p.y - f.y0) / f.h).floor();
        if i < 0.0 || j < 0.0 || i >= f.nx as f64 || j >= f.ny as f64 {
            return true;
        }
        !f.under[j as usize * f.nx + i as usize]
    }

    /// The nearest point within reach (None: none), signed.
    fn nearest(&self, p: Vec3) -> Option<Nearest> {
        let key = |v: f64| (v / self.cell).floor() as i64;
        let mut best: Option<(f64, Vec3, Vec3)> = None;
        for &k in self
            .cells
            .get(&(key(p.x), key(p.y), key(p.z)))
            .into_iter()
            .flatten()
        {
            let t = self.tris[k as usize];
            let [a, b, c] = t.map(|i| self.pos[i as usize]);
            let (q, w) = closest(p, a, b, c);
            let d = (p - q).length_sq();
            if best.is_some_and(|x| x.0 <= d) {
                continue;
            }
            // the feature the point is nearest: a corner, a side or the face
            let small = 1e-9;
            let zero: Vec<usize> = (0..3).filter(|&i| w[i] < small).collect();
            let n = match zero.len() {
                2 => {
                    let i = (0..3).find(|i| !zero.contains(i)).unwrap_or(0);
                    self.vertex_normal[t[i] as usize]
                }
                1 => {
                    let (x, y) = (t[(zero[0] + 1) % 3], t[(zero[0] + 2) % 3]);
                    self.edge_normal
                        .get(&(x.min(y), x.max(y)))
                        .copied()
                        .unwrap_or(self.normal[k as usize])
                }
                _ => self.normal[k as usize],
            };
            best = Some((d, q, n));
        }
        let tri = best.map(|(d, q, n)| Nearest {
            at: q,
            distance: d.sqrt() * if (p - q).dot(n) < 0.0 { -1.0 } else { 1.0 },
        });
        // the table, where the shape does not stand on it: above it is the sheet's side
        let table = self.table_at(p).then(|| Nearest {
            at: Vec3::new(p.x, p.y, 0.0),
            distance: p.z,
        });
        match (tri, table) {
            (Some(a), Some(b)) => Some(if a.distance.abs() <= b.distance.abs() {
                a
            } else {
                b
            }),
            (a, b) => a.or(b),
        }
    }
}

/// The point of triangle abc nearest p, and its weights on the corners.
fn closest(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> (Vec3, [f64; 3]) {
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return (a, [1.0, 0.0, 0.0]);
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return (b, [0.0, 1.0, 0.0]);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return (a + ab * v, [1.0 - v, v, 0.0]);
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return (c, [0.0, 0.0, 1.0]);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return (a + ac * w, [1.0 - w, 0.0, w]);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (b + (c - b) * w, [0.0, 1.0 - w, w]);
    }
    let den = 1.0 / (va + vb + vc);
    let (v, w) = (vb * den, vc * den);
    (a + ab * v + ac * w, [1.0 - v - w, v, w])
}

impl Sdf {
    /// The field of the tool's faces `tris` of `mesh` (world) and the table `table` marks, over
    /// the window `lo`..`hi` on `frame`'s axes, nodes `h` apart, exact within `band` of the
    /// surface. `male`: the faces face the sheet; else they face into the tool.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        frame: Frame,
        mesh: &TriMesh,
        tris: &[usize],
        male: bool,
        table: Field,
        lo: forge_math::Point2,
        hi: forge_math::Point2,
        h: f64,
        band: f64,
    ) -> Sdf {
        let surface = Surface::new(&frame, mesh, tris, male, table, band + 2.0 * h);
        let zmin = surface.pos.iter().map(|p| p.z).fold(0.0, f64::min) - band - h;
        let origin = Vec3::new(lo.x - band - h, lo.y - band - h, zmin);
        let mut sdf = Sdf {
            frame,
            origin,
            h,
            band,
            bricks: HashMap::new(),
            surface,
        };
        let node = |v: f64, o: f64| ((v - o) / h).floor() as i64;
        let mut want: std::collections::HashSet<(i64, i64, i64)> = Default::default();
        // the nodes within the band of each face
        let reach = band + h;
        for t in &sdf.surface.tris {
            let [a, b, c] = t.map(|i| sdf.surface.pos[i as usize]);
            let lo = Vec3::new(
                a.x.min(b.x).min(c.x),
                a.y.min(b.y).min(c.y),
                a.z.min(b.z).min(c.z),
            );
            let hi = Vec3::new(
                a.x.max(b.x).max(c.x),
                a.y.max(b.y).max(c.y),
                a.z.max(b.z).max(c.z),
            );
            for i in node(lo.x - reach, origin.x)..=node(hi.x + reach, origin.x) + 1 {
                for j in node(lo.y - reach, origin.y)..=node(hi.y + reach, origin.y) + 1 {
                    for k in node(lo.z - reach, origin.z)..=node(hi.z + reach, origin.z) + 1 {
                        want.insert((i, j, k));
                    }
                }
            }
        }
        // and of the table over the window
        let (ni, nj) = (node(hi.x + reach, origin.x) + 1, node(hi.y + reach, origin.y) + 1);
        for i in 0..=ni {
            for j in 0..=nj {
                for k in node(-reach, origin.z)..=node(reach, origin.z) + 1 {
                    want.insert((i, j, k));
                }
            }
        }
        for (i, j, k) in want {
            let p = origin + Vec3::new(i as f64 * h, j as f64 * h, k as f64 * h);
            let Some(d) = sdf.surface.nearest(p) else { continue };
            if d.distance.abs() > reach + h {
                continue;
            }
            let b = (
                i.div_euclid(B as i64) as i32,
                j.div_euclid(B as i64) as i32,
                k.div_euclid(B as i64) as i32,
            );
            let at = |v: i64| v.rem_euclid(B as i64) as usize;
            let brick = sdf
                .bricks
                .entry(b)
                .or_insert_with(|| Box::new([f32::NAN; B * B * B]));
            brick[(at(k) * B + at(j)) * B + at(i)] = d.distance as f32;
        }
        sdf
    }

    fn node(&self, i: i64, j: i64, k: i64) -> Option<f64> {
        let b = (
            i.div_euclid(B as i64) as i32,
            j.div_euclid(B as i64) as i32,
            k.div_euclid(B as i64) as i32,
        );
        let at = |v: i64| v.rem_euclid(B as i64) as usize;
        let v = self.bricks.get(&b)?[(at(k) * B + at(j)) * B + at(i)];
        (!v.is_nan()).then_some(v as f64)
    }

    /// The signed distance at a point on the plane's axes: trilinear in the band, exact near the
    /// surface (where contact is decided) and past the band; past that the band, on the side the
    /// table says.
    pub fn distance(&self, p: Vec3) -> f64 {
        let d = self.interpolated(p);
        if d.abs() < NEAR * self.h {
            return self.surface.nearest(p).map_or(d, |n| n.distance);
        }
        d
    }

    fn interpolated(&self, p: Vec3) -> f64 {
        let q = (p - self.origin) * (1.0 / self.h);
        let (i, j, k) = (q.x.floor(), q.y.floor(), q.z.floor());
        let (u, v, w) = (q.x - i, q.y - j, q.z - k);
        let (i, j, k) = (i as i64, j as i64, k as i64);
        let mut sum = 0.0;
        for (di, fu) in [(0, 1.0 - u), (1, u)] {
            for (dj, fv) in [(0, 1.0 - v), (1, v)] {
                for (dk, fw) in [(0, 1.0 - w), (1, w)] {
                    match self.node(i + di, j + dj, k + dk) {
                        Some(d) => sum += d * fu * fv * fw,
                        None => return self.far(p),
                    }
                }
            }
        }
        sum
    }

    fn far(&self, p: Vec3) -> f64 {
        match self.surface.nearest(p) {
            Some(n) => n.distance,
            None => {
                self.band
                    * if self.surface.table_at(p) && p.z < 0.0 {
                        -1.0
                    } else {
                        1.0
                    }
            }
        }
    }

    /// The exact signed distance (what the nodes hold), for checking the field.
    pub fn exact(&self, p: Vec3) -> Option<f64> {
        self.surface.nearest(p).map(|n| n.distance)
    }

    /// The surface's normal at a point near it: the field's gradient, the sheet's side out.
    pub fn normal(&self, p: Vec3) -> Vec3 {
        let e = 0.25 * self.h;
        let d = |x: Vec3| self.distance(p + x) - self.distance(p - x);
        let g = Vec3::new(
            d(Vec3::new(e, 0.0, 0.0)),
            d(Vec3::new(0.0, e, 0.0)),
            d(Vec3::new(0.0, 0.0, e)),
        );
        g.normalize().map_or(Vec3::new(0.0, 0.0, 1.0), |u| u.get())
    }

    /// How many nodes the band holds.
    pub fn nodes(&self) -> usize {
        self.bricks
            .values()
            .map(|b| b.iter().filter(|v| !v.is_nan()).count())
            .sum()
    }

    /// A point on the plane's axes in the world.
    pub fn world(&self, p: Vec3) -> Point3 {
        self.frame.point_to_world(Point3::new(p.x, p.y, p.z))
    }
}
