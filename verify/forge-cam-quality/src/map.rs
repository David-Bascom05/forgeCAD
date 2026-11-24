//! The stock as a height map over the setup's stock box (a z-up dexel per cell, the cell's
//! centre), lowered by the tool's body as the moves sweep it.
//!
//! This is a measurement model, not a certificate: its answers are within about one cell of the
//! exact swept volume (the cell size is reported with every number made from it).

/// The tool's lower envelope: the height of its body above the tip at radial distance d (mm),
/// out to its radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Profile {
    /// A flat end (corner radius 0) or a bull nose (corner radius re).
    Bull {
        r: f64,
        re: f64,
    },
    Ball {
        r: f64,
    },
    /// A cone from a tip of radius `tip` up at half angle `half` (radians), out to r.
    Cone {
        r: f64,
        tip: f64,
        half: f64,
    },
}

impl Profile {
    pub fn r(&self) -> f64 {
        match *self {
            Profile::Bull { r, .. } | Profile::Ball { r } | Profile::Cone { r, .. } => r,
        }
    }

    /// Height above the tip at radial distance d ≤ r.
    pub fn at(&self, d: f64) -> f64 {
        match *self {
            Profile::Bull { r, re } => {
                let flat = r - re;
                if d <= flat {
                    0.0
                } else {
                    let u = (d - flat).min(re);
                    re - (re * re - u * u).max(0.0).sqrt()
                }
            }
            Profile::Ball { r } => {
                let u = d.min(r);
                r - (r * r - u * u).max(0.0).sqrt()
            }
            Profile::Cone { tip, half, .. } => {
                if d <= tip {
                    0.0
                } else {
                    {
                        let (sn, cs) = forge_math::dmath::sin_cos(half);
                        (d - tip) * cs / sn.max(1e-9)
                    }
                }
            }
        }
    }
}

pub struct HeightMap {
    pub x0: f64,
    pub y0: f64,
    pub h: f64,
    pub nx: usize,
    pub ny: usize,
    pub bottom: f64,
    pub z: Vec<f64>,
}

impl HeightMap {
    /// The box [min, max], cells of size h (whole cells cover the box).
    pub fn new(min: [f64; 3], max: [f64; 3], h: f64) -> HeightMap {
        let nx = (((max[0] - min[0]) / h).ceil() as usize).max(1);
        let ny = (((max[1] - min[1]) / h).ceil() as usize).max(1);
        HeightMap {
            x0: min[0],
            y0: min[1],
            h,
            nx,
            ny,
            bottom: min[2],
            z: vec![max[2]; nx * ny],
        }
    }

    /// The top at a point; outside the box there is no material (−∞).
    pub fn top(&self, x: f64, y: f64) -> f64 {
        let i = ((x - self.x0) / self.h).floor();
        let j = ((y - self.y0) / self.h).floor();
        if i < 0.0 || j < 0.0 || i >= self.nx as f64 || j >= self.ny as f64 {
            return f64::NEG_INFINITY;
        }
        self.z[j as usize * self.nx + i as usize]
    }

    pub fn volume(&self) -> f64 {
        self.z.iter().map(|z| (z - self.bottom).max(0.0)).sum::<f64>() * self.h * self.h
    }

    /// Sweeps the tool's body with its tip on the segment a → b (tip height linear along it):
    /// each cell whose centre lies within r of the segment in xy is lowered to the body there.
    /// Returns the volume removed (mm³).
    pub fn sweep(&mut self, a: [f64; 3], b: [f64; 3], p: &Profile) -> f64 {
        let r = p.r();
        let cell = |v: f64, o: f64| ((v - o) / self.h - 0.5).floor();
        let i0 = cell(a[0].min(b[0]) - r, self.x0).max(0.0) as usize;
        let j0 = cell(a[1].min(b[1]) - r, self.y0).max(0.0) as usize;
        let i1 = ((cell(a[0].max(b[0]) + r, self.x0) + 2.0).max(0.0) as usize).min(self.nx);
        let j1 = ((cell(a[1].max(b[1]) + r, self.y0) + 2.0).max(0.0) as usize).min(self.ny);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let ll = dx * dx + dy * dy;
        let zmin = a[2].min(b[2]);
        let mut removed = 0.0;
        for j in j0..j1 {
            let y = self.y0 + (j as f64 + 0.5) * self.h;
            for i in i0..i1 {
                let k = j * self.nx + i;
                let old = self.z[k];
                if old <= zmin || old <= self.bottom {
                    continue;
                }
                let x = self.x0 + (i as f64 + 0.5) * self.h;
                let t = if ll > 0.0 {
                    (((x - a[0]) * dx + (y - a[1]) * dy) / ll).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let px = a[0] + t * dx;
                let py = a[1] + t * dy;
                let d = ((x - px) * (x - px) + (y - py) * (y - py)).sqrt();
                if d > r {
                    continue;
                }
                let mut zc = a[2] + t * (b[2] - a[2]) + p.at(d);
                // on a ramp the lower end may reach the cell lower than the nearest point does
                for e in [a, b] {
                    let de = ((x - e[0]) * (x - e[0]) + (y - e[1]) * (y - e[1])).sqrt();
                    if de <= r {
                        zc = zc.min(e[2] + p.at(de));
                    }
                }
                let new = zc.max(self.bottom);
                if new < old {
                    removed += old - new;
                    self.z[k] = new;
                }
            }
        }
        removed * self.h * self.h
    }
}

/// What the cutter's side edge meets at one place, the map as it stands (before the piece
/// ending there is swept).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Engage {
    /// Engagement angle, degrees (0 = air).
    pub phi: f64,
    /// Engaged angle right of the direction of travel and left of it, degrees.
    pub right: f64,
    pub left: f64,
    /// max cos θ and mean cos θ over the engaged edge (θ from the direction of travel): the chip
    /// is fz cos θ there.
    pub cos_max: f64,
    pub cos_mean: f64,
    /// Axial depth of the engaged edge (the highest material above the tip).
    pub ap: f64,
    /// Material within 10° of the right extreme (θ = −90°) and of the left (θ = +90°): the
    /// wall a side cut leaves is on that side. Both: a slot.
    pub edge_r: bool,
    pub edge_l: bool,
}

/// Points round the front half of the edge (θ in [−90°, 90°]), 1° apart.
pub const N_EDGE: usize = 181;

/// The engagement at tip c moving along xy direction u (unit), the side edge read at radius
/// r − 3h/4 (a cell whose centre is within r of an earlier pass has every point within r + h/√2
/// of it: a wall cut to exactly r reads as cut): a point is material if the map there is
/// above the tip by more than `eps`.
pub fn engage(m: &HeightMap, c: [f64; 3], u: [f64; 2], r: f64, eps: f64) -> Engage {
    let rr = (r - 0.75 * m.h).max(0.25 * r);
    let step = std::f64::consts::PI / (N_EDGE - 1) as f64;
    let deg = 180.0 / (N_EDGE - 1) as f64;
    let mut e = Engage::default();
    let (mut n, mut sum_cos) = (0usize, 0.0);
    for k in 0..N_EDGE {
        let th = -std::f64::consts::FRAC_PI_2 + k as f64 * step;
        let (s, co) = forge_math::dmath::sin_cos(th);
        // u turned by θ (+ = left, counter-clockwise)
        let dx = u[0] * co - u[1] * s;
        let dy = u[0] * s + u[1] * co;
        let t = m.top(c[0] + rr * dx, c[1] + rr * dy);
        if t > c[2] + eps {
            n += 1;
            sum_cos += co;
            e.cos_max = e.cos_max.max(co);
            e.ap = e.ap.max(t - c[2]);
            if k <= 10 {
                e.edge_r = true;
            }
            if k + 10 >= N_EDGE - 1 {
                e.edge_l = true;
            }
            // the half circle's two end points count half (180 intervals over 181 points)
            let w = if k == 0 || k == N_EDGE - 1 { 0.5 * deg } else { deg };
            if 2 * k < N_EDGE - 1 {
                e.right += w;
            } else if 2 * k > N_EDGE - 1 {
                e.left += w;
            } else {
                e.right += 0.5 * w;
                e.left += 0.5 * w;
            }
        }
    }
    e.phi = e.right + e.left;
    if n > 0 {
        e.cos_mean = sum_cos / n as f64;
    }
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_and_side_cut() {
        // a 20 x 20 x 10 block, a 10 mm flat end mill
        let mut m = HeightMap::new([0.0, 0.0, 0.0], [20.0, 20.0, 10.0], 0.05);
        let p = Profile::Bull { r: 5.0, re: 0.0 };
        // a slot along x at y = 10, 2 deep, ending at x = 10
        let removed = m.sweep([-6.0, 10.0, 8.0], [10.0, 10.0, 8.0], &p);
        let want = 10.0 * 10.0 * 2.0 + 0.5 * std::f64::consts::PI * 25.0 * 2.0;
        assert!((removed - want).abs() / want < 0.02, "{removed} vs {want}");
        // just past the slot's end: the whole front half meets material
        let e = engage(&m, [10.5, 10.0, 8.0], [1.0, 0.0], 5.0, 1e-3);
        assert!((e.phi - 180.0).abs() < 8.0, "{e:?}");
        assert!((e.ap - 2.0).abs() < 1e-9);
        assert!((e.cos_max - 1.0).abs() < 1e-9);
        // the slot run on through the block, then a side cut 1 mm into its right wall:
        // ae/D = 0.1, φ = acos(1 − 2·0.1) ≈ 36.87°
        m.sweep([10.0, 10.0, 8.0], [30.0, 10.0, 8.0], &p);
        let e = engage(&m, [5.0, 9.0, 8.0], [1.0, 0.0], 5.0, 1e-3);
        assert!(e.left < 1e-9, "{e:?}");
        assert!((e.right - 36.87).abs() < 3.0, "{e:?}");
        // the chip there: fz · sin φ = 0.6 fz
        assert!((e.cos_max - 0.6).abs() < 0.03, "{e:?}");
    }
}
