//! A plain list of placed parts, and what of it a frame draws: frustum culling and level of detail.

use crate::camera::View;

/// A part's rows of a rigid transform (row major, translation in the last column) and its colour.
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub part: u32,
    pub rows: [[f32; 4]; 3],
    pub tint: [f32; 4],
}

impl Placement {
    pub fn at(part: u32, pos: [f32; 3]) -> Placement {
        Placement {
            part,
            rows: [
                [1.0, 0.0, 0.0, pos[0]],
                [0.0, 1.0, 0.0, pos[1]],
                [0.0, 0.0, 1.0, pos[2]],
            ],
            tint: [0.62, 0.65, 0.69, 1.0],
        }
    }
}

/// One draw: placements `first..first + count` of the frame's list, all one part at one level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Draw {
    pub part: u32,
    pub lod: u32,
    pub first: u32,
    pub count: u32,
}

/// Each part's chords (finest first), its sphere, and every placement's sphere in columns: the
/// frame's one loop over the placements runs over these.
pub struct Scene {
    pub placements: Vec<Placement>,
    chords: Vec<Vec<f32>>,
    x: Vec<f32>,
    y: Vec<f32>,
    z: Vec<f32>,
    r: Vec<f32>,
    /// Filled by `cull`: the visible placements' indices, grouped as `draws` says.
    pub visible: Vec<[u8; 4]>,
    pub draws: Vec<Draw>,
    picked: Vec<(u32, u32)>,
    counts: Vec<u32>,
}

/// A level per part at most (tessellated and clustered).
pub const LODS: usize = 8;

impl Scene {
    pub fn new(parts: &[([f32; 3], f32, Vec<f32>)], placements: Vec<Placement>) -> Scene {
        let (mut x, mut y, mut z, mut r) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for p in &placements {
            let (c, radius, _) = &parts[p.part as usize];
            let at = |row: [f32; 4]| row[0] * c[0] + row[1] * c[1] + row[2] * c[2] + row[3];
            x.push(at(p.rows[0]));
            y.push(at(p.rows[1]));
            z.push(at(p.rows[2]));
            r.push(*radius);
        }
        Scene {
            placements,
            chords: parts.iter().map(|p| p.2.clone()).collect(),
            x,
            y,
            z,
            r,
            visible: Vec::new(),
            draws: Vec::new(),
            picked: Vec::new(),
            counts: vec![0; parts.len() * LODS],
        }
    }

    /// A sphere holding the whole scene.
    pub fn bounds(&self) -> ([f32; 3], f32) {
        let n = self.x.len().max(1) as f32;
        let c = [
            self.x.iter().sum::<f32>() / n,
            self.y.iter().sum::<f32>() / n,
            self.z.iter().sum::<f32>() / n,
        ];
        let mut r = 0f32;
        for i in 0..self.x.len() {
            let (a, b, d) = (self.x[i] - c[0], self.y[i] - c[1], self.z[i] - c[2]);
            r = r.max((a * a + b * b + d * d).sqrt() + self.r[i]);
        }
        (c, r)
    }

    /// Works out the frame's draws: finest levels first, which draws near things first.
    /// A level is the coarsest whose chord stays under `max_px` pixels on screen.
    pub fn cull(&mut self, v: &View, max_px: f32) {
        let parts = self.chords.len();
        self.picked.clear();
        self.counts.fill(0);
        let [a, b, c, d, e] = v.planes;
        for i in 0..self.x.len() {
            let (x, y, z, r) = (self.x[i], self.y[i], self.z[i], -self.r[i]);
            let side = |p: [f32; 4]| p[0] * x + p[1] * y + p[2] * z + p[3] < r;
            if side(a) | side(b) | side(c) | side(d) | side(e) {
                continue;
            }
            let (dx, dy, dz) = (x - v.eye[0], y - v.eye[1], z - v.eye[2]);
            let closest = ((dx * dx + dy * dy + dz * dz).sqrt() + r).max(v.near);
            let allowed = max_px * closest / v.px_mm;
            let part = self.placements[i].part as usize;
            let chords = &self.chords[part];
            let lod = chords.partition_point(|&c| c <= allowed).saturating_sub(1);
            let bin = lod * parts + part;
            self.counts[bin] += 1;
            self.picked.push((bin as u32, i as u32));
        }
        self.draws.clear();
        let mut at = 0;
        for (bin, count) in self.counts.iter_mut().enumerate() {
            if *count > 0 {
                let n = *count;
                self.draws.push(Draw {
                    part: (bin % parts) as u32,
                    lod: (bin / parts) as u32,
                    first: at,
                    count: n,
                });
                *count = at;
                at += n;
            }
        }
        self.visible.resize(at as usize, [0; 4]);
        for &(bin, i) in &self.picked {
            let slot = &mut self.counts[bin as usize];
            self.visible[*slot as usize] = i.to_le_bytes();
            *slot += 1;
        }
    }
}

/// `n` placements of `parts` kinds in a cube, `spacing` apart on a jittered grid, turned at
/// random (fixed seed, so every run sees the same scene).
pub fn stress(parts: usize, n: usize, spacing: f32) -> Vec<Placement> {
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 40) as f32 / (1u64 << 24) as f32
    };
    let side = (n as f32).cbrt().ceil() as usize;
    let half = side as f32 * spacing * 0.5;
    (0..n)
        .map(|i| {
            let (ix, iy, iz) = (i % side, i / side % side, i / side / side);
            let jitter = |g: usize, r: f32| g as f32 * spacing - half + (r - 0.5) * spacing * 0.6;
            let pos = [jitter(ix, next()), jitter(iy, next()), jitter(iz, next())];
            // a uniform random rotation (Shoemake)
            let (u1, u2, u3) = (
                next(),
                next() * core::f32::consts::TAU,
                next() * core::f32::consts::TAU,
            );
            let (a, b) = ((1.0 - u1).sqrt(), u1.sqrt());
            let (w, x, y, z) = (a * u2.sin(), a * u2.cos(), b * u3.sin(), b * u3.cos());
            let part = (next() * parts as f32) as u32 % parts as u32;
            let hue = part as f32 / parts as f32 * core::f32::consts::TAU;
            let tint = [
                0.62 + 0.10 * hue.cos(),
                0.64 + 0.04 * hue.sin(),
                0.68 - 0.10 * hue.cos(),
                1.0,
            ];
            Placement {
                part,
                rows: [
                    [
                        1.0 - 2.0 * (y * y + z * z),
                        2.0 * (x * y - w * z),
                        2.0 * (x * z + w * y),
                        pos[0],
                    ],
                    [
                        2.0 * (x * y + w * z),
                        1.0 - 2.0 * (x * x + z * z),
                        2.0 * (y * z - w * x),
                        pos[1],
                    ],
                    [
                        2.0 * (x * z - w * y),
                        2.0 * (y * z + w * x),
                        1.0 - 2.0 * (x * x + y * y),
                        pos[2],
                    ],
                ],
                tint,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;

    fn scene(places: Vec<Placement>) -> Scene {
        Scene::new(&[([0.0; 3], 1.0, vec![0.01, 0.1, 1.0])], places)
    }

    #[test]
    fn a_part_behind_the_camera_is_culled_and_one_ahead_kept() {
        let cam = Camera::fit([0.0; 3], 1.0);
        let v = cam.view(800, 600);
        let behind = [0, 1, 2].map(|k| v.eye[k] * 3.0);
        let mut s = scene(vec![Placement::at(0, behind), Placement::at(0, [0.0; 3])]);
        s.cull(&v, 1.0);
        assert_eq!(s.visible, vec![1u32.to_le_bytes()]);
        assert_eq!(s.draws.len(), 1);
    }

    #[test]
    fn farther_parts_take_coarser_levels() {
        let cam = Camera::fit([0.0; 3], 1.0);
        let v = cam.view(800, 600);
        let dir = [0, 1, 2].map(|k| -v.eye[k]);
        let at = |t: f32| Placement::at(0, dir.map(|d| d * t));
        let mut s = scene(vec![at(1.0), at(1000.0)]);
        s.cull(&v, 1.0);
        assert_eq!(s.draws.len(), 2);
        assert!(s.draws[0].lod < s.draws[1].lod);
    }

    #[test]
    fn stress_rotations_are_rigid_and_the_scene_is_repeatable() {
        let a = stress(15, 100, 300.0);
        let b = stress(15, 100, 300.0);
        for (p, q) in a.iter().zip(&b) {
            assert_eq!(p.rows, q.rows);
            let dot = |i: usize, j: usize| (0..3).map(|k| p.rows[i][k] * p.rows[j][k]).sum::<f32>();
            assert!((dot(0, 0) - 1.0).abs() < 1e-4 && dot(0, 1).abs() < 1e-4 && dot(1, 2).abs() < 1e-4);
        }
    }
}
