//! Scallop (constant stepover) finishing: passes a fixed distance apart measured *on the
//! surface*, as level sets of the distance from the region's boundary.
//!
//! Method (plain, published: geodesic parallels; Kimmel & Sethian 1998, Kim 2007; iso-scallop
//! is reported unclaimed, research unclaimed-methods.md). The distance is Dijkstra's on the grid
//! of CL heights with a 16-neighbour stencil (the knight's moves bring the metrication error
//! under about 2.6%), each edge weighted by its 3D length over the CL surface. The passes are
//! the field's level sets every `stepover · (1 − 0.03)` (the stencil's error taken off, so the
//! spacing on the surface never exceeds the stepover by it), traced by marching squares and then
//! draped (drape.rs): the drape, not the field, makes every move safe.
//!
//! The boundary is where the region ends: grid nodes with nothing under the tool, outside the
//! containment, or (optionally) steeper than a wall slope (passes then run parallel to the
//! walls, Fusion's "from walls" option).

use crate::contain::Containment;
use crate::drape::{self, Drape, DrapeError, Pass};
use crate::drop::DropCutter;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    pub grid: f64,
    /// The spacing on the surface (from scallop::step_for).
    pub stepover: f64,
    /// CL slope (tan) at or above which a node is a wall (the passes start from it); ∞: none.
    pub tan_wall: f64,
}

#[derive(Clone, Copy, PartialEq)]
struct Item(f64, usize);
impl Eq for Item {}
impl PartialOrd for Item {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Item {
    fn cmp(&self, o: &Self) -> Ordering {
        // a min-heap on distance, ties by index (deterministic)
        o.0.total_cmp(&self.0).then(o.1.cmp(&self.1))
    }
}

/// The grid: x0, y0, g, nx, ny, CL heights (NaN outside), and the distance (NaN outside).
pub struct Field {
    pub x0: f64,
    pub y0: f64,
    pub g: f64,
    pub nx: usize,
    pub ny: usize,
    pub z: Vec<f64>,
    pub dist: Vec<f64>,
}

pub fn field(dc: &dyn DropCutter, p: Params, c: &Containment) -> Field {
    let [lo, hi] = dc.extent();
    let g = p.grid;
    let nx = (((hi[0] - lo[0]) / g).ceil() as usize) + 1;
    let ny = (((hi[1] - lo[1]) / g).ceil() as usize) + 1;
    let mut z = vec![f64::NAN; nx * ny];
    let mut j = 0;
    while j < ny {
        let mut i = 0;
        while i < nx {
            let x = lo[0] + g * i as f64;
            let y = lo[1] + g * j as f64;
            let ok = match &c.boundary {
                Some(b) => crate::contain::inside(b, [x, y]),
                None => true,
            };
            if ok && let Some(b) = dc.drop(x, y) {
                let h = b.hi;
                if h >= c.zmin && h <= c.zmax {
                    z[j * nx + i] = h;
                }
            }
            i += 1;
        }
        j += 1;
    }
    // walls: nodes whose slope (central differences) is at least tan_wall leave the region
    if p.tan_wall.is_finite() {
        let mut w = z.clone();
        let mut j = 1;
        while j + 1 < ny {
            let mut i = 1;
            while i + 1 < nx {
                let k = j * nx + i;
                let (a, b, cc, d) = (z[k - 1], z[k + 1], z[k - nx], z[k + nx]);
                if !(a.is_nan() || b.is_nan() || cc.is_nan() || d.is_nan()) {
                    let gx = (b - a) / (2.0 * g);
                    let gy = (d - cc) / (2.0 * g);
                    if (gx * gx + gy * gy).sqrt() >= p.tan_wall {
                        w[k] = f64::NAN;
                    }
                }
                i += 1;
            }
            j += 1;
        }
        z = w;
    }
    // seeds: region nodes with an outside 4-neighbour (or on the grid's edge)
    let mut dist = vec![f64::INFINITY; nx * ny];
    let mut heap = BinaryHeap::new();
    let mut k = 0;
    while k < z.len() {
        if z[k].is_nan() {
            dist[k] = f64::NAN;
        } else {
            let (i, j) = (k % nx, k / nx);
            let edge = i == 0
                || j == 0
                || i + 1 == nx
                || j + 1 == ny
                || z[k - 1].is_nan()
                || z[k + 1].is_nan()
                || z[k - nx].is_nan()
                || z[k + nx].is_nan();
            if edge {
                dist[k] = 0.0;
                heap.push(Item(0.0, k));
            }
        }
        k += 1;
    }
    const NB: [(i64, i64); 16] = [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (1, -1),
        (-1, 1),
        (-1, -1),
        (2, 1),
        (2, -1),
        (-2, 1),
        (-2, -1),
        (1, 2),
        (1, -2),
        (-1, 2),
        (-1, -2),
    ];
    while let Some(Item(d, k)) = heap.pop() {
        if d > dist[k] {
            continue;
        }
        let (i, j) = ((k % nx) as i64, (k / nx) as i64);
        for (di, dj) in NB {
            let (a, b) = (i + di, j + dj);
            if a < 0 || b < 0 || a >= nx as i64 || b >= ny as i64 {
                continue;
            }
            let q = (b as usize) * nx + a as usize;
            if z[q].is_nan() {
                continue;
            }
            // a knight's move must not jump over an outside node
            if di.abs() == 2 || dj.abs() == 2 {
                let mid = ((j + dj / 2) as usize) * nx + (i + di / 2) as usize;
                if z[mid].is_nan() {
                    continue;
                }
            }
            let dx = g * di as f64;
            let dy = g * dj as f64;
            let dz = z[q] - z[k];
            let nd = d + (dx * dx + dy * dy + dz * dz).sqrt();
            if nd < dist[q] {
                dist[q] = nd;
                heap.push(Item(nd, q));
            }
        }
    }
    Field {
        x0: lo[0],
        y0: lo[1],
        g,
        nx,
        ny,
        z,
        dist,
    }
}

/// The closed level sets {dist = level} of the field (outside nodes count as below the level),
/// by marching squares with linear interpolation on the cell edges.
pub fn level_sets(f: &Field, level: f64) -> Vec<Vec<[f64; 2]>> {
    let (nx, ny) = (f.nx, f.ny);
    let above = |i: usize, j: usize| {
        let v = f.dist[j * nx + i];
        !v.is_nan() && v >= level
    };
    let hid = |i: usize, j: usize| 2 * (j * nx + i);
    let vid = |i: usize, j: usize| 2 * (j * nx + i) + 1;
    let mut next = vec![usize::MAX; 2 * nx * ny];
    let mut j = 0;
    while j + 1 < ny {
        let mut i = 0;
        while i + 1 < nx {
            let c = [above(i, j), above(i + 1, j), above(i + 1, j + 1), above(i, j + 1)];
            let e = [hid(i, j), vid(i + 1, j), hid(i, j + 1), vid(i, j)];
            let mut k = 0;
            while k < 4 {
                let prev = (k + 3) % 4;
                if c[k] && !c[prev] {
                    let mut m = k;
                    while c[(m + 1) % 4] && (m + 1) % 4 != k {
                        m = (m + 1) % 4;
                    }
                    next[e[m]] = e[prev];
                }
                k += 1;
            }
            i += 1;
        }
        j += 1;
    }
    let point = |e: usize| -> [f64; 2] {
        let k = e / 2;
        let (i, j) = (k % nx, k / nx);
        let (i2, j2) = if e.is_multiple_of(2) { (i + 1, j) } else { (i, j + 1) };
        let va = f.dist[j * nx + i];
        let vb = f.dist[j2 * nx + i2];
        // an outside node sits at distance 0 (the boundary)
        let va = if va.is_nan() { 0.0 } else { va };
        let vb = if vb.is_nan() { 0.0 } else { vb };
        let t = if vb != va {
            ((level - va) / (vb - va)).clamp(0.0, 1.0)
        } else {
            0.5
        };
        let a = [f.x0 + f.g * i as f64, f.y0 + f.g * j as f64];
        let b = [f.x0 + f.g * i2 as f64, f.y0 + f.g * j2 as f64];
        [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
    };
    let mut seen = vec![false; next.len()];
    let mut out = Vec::new();
    let mut e0 = 0;
    while e0 < next.len() {
        if next[e0] == usize::MAX || seen[e0] {
            e0 += 1;
            continue;
        }
        let mut ring = Vec::new();
        let mut e = e0;
        while e != usize::MAX && !seen[e] {
            seen[e] = true;
            ring.push(point(e));
            e = next[e];
        }
        if ring.len() > 2 {
            ring.push(ring[0]);
            out.push(ring);
        }
        e0 += 1;
    }
    out
}

/// Scallop passes: level sets every stepover (less the stencil's error), innermost last,
/// draped. The first pass is half a grid cell in from the boundary.
pub fn scallop(dc: &dyn DropCutter, p: Params, d: Drape, c: &Containment) -> Result<Vec<Pass>, DrapeError> {
    let f = field(dc, p, c);
    let mut dmax: f64 = 0.0;
    for &v in &f.dist {
        if v.is_finite() {
            dmax = dmax.max(v);
        }
    }
    let step = p.stepover * (1.0 - 0.03);
    let mut out = Vec::new();
    let mut lv = 0.5 * p.grid;
    while lv < dmax {
        for ring in level_sets(&f, lv) {
            out.extend(drape::drape(dc, &ring, d)?);
        }
        lv += step;
    }
    Ok(c.clip(out, None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drop::Analytic;

    #[test]
    fn a_square_floor_gives_nested_rings_a_stepover_apart() {
        let dc = Analytic {
            r: 1.0,
            planes: vec![(0.0, [-10.0, -10.0], [10.0, 10.0])],
            spheres: vec![],
        };
        // the region: a 16 mm square inside the floor
        let c = Containment {
            boundary: Some(vec![[-8.0, -8.0], [8.0, -8.0], [8.0, 8.0], [-8.0, 8.0]]),
            ..Default::default()
        };
        let p = Params {
            grid: 0.25,
            stepover: 1.0,
            tan_wall: f64::INFINITY,
        };
        let d = Drape {
            step: 1.0,
            slack: 0.005,
            leave: 0.01,
            zmin: f64::NEG_INFINITY,
        };
        let ps = scallop(&dc, p, d, &c).unwrap();
        // about 8 rings (half-width 8 mm, 0.97 mm apart)
        assert!(ps.len() >= 7 && ps.len() <= 10, "{}", ps.len());
        // on a flat floor the rings are squares: each one's largest |x| or |y| goes down by
        // about the step from ring to ring
        let mut halves: Vec<f64> = ps
            .iter()
            .map(|p| p.iter().map(|q| q[0].abs().max(q[1].abs())).fold(0.0, f64::max))
            .collect();
        halves.sort_by(|a, b| b.total_cmp(a));
        let mut k = 0;
        while k + 2 < halves.len() {
            let gap = halves[k] - halves[k + 1];
            assert!(gap > 0.85 && gap <= 1.0 + 0.26, "gap {gap} at {k}: {halves:?}");
            k += 1;
        }
    }

    #[test]
    fn on_a_slope_the_xy_spacing_shrinks() {
        // the dome's flank: rings from the dome's foot are closer in xy where it is steep
        let dc = Analytic {
            r: 3.0,
            planes: vec![],
            spheres: vec![([0.0, 0.0, -10.0], 30.0)],
        };
        let p = Params {
            grid: 0.5,
            stepover: 2.0,
            tan_wall: f64::INFINITY,
        };
        let f = field(&dc, p, &Containment::default());
        // distance from the edge of the CL disc (radius 33) inwards along +x: on the surface it
        // grows faster than in xy near the edge
        let i_edge = ((32.0 - f.x0) / f.g).round() as usize;
        let i_in = ((22.0 - f.x0) / f.g).round() as usize;
        let j = ((0.0 - f.y0) / f.g).round() as usize;
        let d_edge = f.dist[j * f.nx + i_edge];
        let d_in = f.dist[j * f.nx + i_in];
        assert!(d_in - d_edge > 10.0 * 1.2, "{d_edge} {d_in}");
    }
}
