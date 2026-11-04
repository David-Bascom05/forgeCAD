//! The tool's surface as a height field over the trim plane: the height the sheet meets above each
//! cell (male: the mould's top, the table elsewhere; female: the cavity's floor, the tool's top
//! elsewhere). The draw ratios, the webbing scan and the vents' places are read from it.
use forge_math::{Frame, Point2, Point3};
use forge_mesh::TriMesh;

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// The trim plane: x, y the field's axes, z the pull.
    pub frame: Frame,
    pub x0: f64,
    pub y0: f64,
    /// The cell size.
    pub h: f64,
    pub nx: usize,
    pub ny: usize,
    /// Height along the pull over each cell centre, row by row; NaN outside the trim.
    pub z: Vec<f64>,
    /// Cells under the mould (male) or over the cavity (female).
    pub under: Vec<bool>,
}

/// A valley between raised parts of the tool, along one scan line: how deep it is below the lower
/// of the two crests and how wide it is at that level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Valley {
    pub at: Point2,
    /// The height of its floor there.
    pub floor: f64,
    pub depth: f64,
    pub width: f64,
}

/// One region the mould stands on (male) or a cavity opens through (female): its deepest draw and
/// the widest circle its outline holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    pub at: Point2,
    pub depth: f64,
    pub width: f64,
}

/// At most this many cells either way (one more where the span does not divide).
const MAX_CELLS: f64 = 240.0;

impl Field {
    /// The field of the triangles `tris` of `mesh` (the tool's contact faces), inside `inside`
    /// (the trim). `male`: the highest surface over a cell, the table (0) where there is none; else
    /// the lowest, the tool's top (0) where there is none.
    pub fn new(
        frame: Frame,
        lo: Point2,
        hi: Point2,
        mesh: &TriMesh,
        tris: &[usize],
        male: bool,
        inside: impl Fn(Point2) -> bool,
    ) -> Field {
        let span = (hi.x - lo.x).max(hi.y - lo.y);
        let h = (span / MAX_CELLS).max(0.25);
        let nx = ((hi.x - lo.x) / h).ceil().max(1.0) as usize;
        let ny = ((hi.y - lo.y) / h).ceil().max(1.0) as usize;
        let (x0, y0) = (lo.x, lo.y);
        let mut z = vec![f64::NAN; nx * ny];
        let mut under = vec![false; nx * ny];
        for &t in tris {
            let [a, b, c] = mesh.triangles[t].map(|i| frame.point_to_local(mesh.positions[i as usize]));
            let area = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
            if area.abs() < 1e-12 {
                continue;
            }
            let cell = |v: f64, o: f64, n: usize| (((v - o) / h - 0.5).max(0.0) as usize).min(n - 1);
            let (i0, i1) = (
                cell(a.x.min(b.x).min(c.x), x0, nx),
                cell(a.x.max(b.x).max(c.x) + h, x0, nx),
            );
            let (j0, j1) = (
                cell(a.y.min(b.y).min(c.y), y0, ny),
                cell(a.y.max(b.y).max(c.y) + h, y0, ny),
            );
            for j in j0..=j1 {
                for i in i0..=i1 {
                    let (px, py) = (x0 + (i as f64 + 0.5) * h, y0 + (j as f64 + 0.5) * h);
                    let wa = ((b.x - px) * (c.y - py) - (b.y - py) * (c.x - px)) / area;
                    let wb = ((c.x - px) * (a.y - py) - (c.y - py) * (a.x - px)) / area;
                    let wc = 1.0 - wa - wb;
                    if wa < -1e-12 || wb < -1e-12 || wc < -1e-12 {
                        continue;
                    }
                    let v = wa * a.z + wb * b.z + wc * c.z;
                    let k = j * nx + i;
                    if !under[k] || (male && v > z[k]) || (!male && v < z[k]) {
                        z[k] = v;
                    }
                    under[k] = true;
                }
            }
        }
        for j in 0..ny {
            for i in 0..nx {
                let k = j * nx + i;
                let p = Point2::new(x0 + (i as f64 + 0.5) * h, y0 + (j as f64 + 0.5) * h);
                if !inside(p) {
                    z[k] = f64::NAN;
                    under[k] = false;
                } else if !under[k] {
                    z[k] = 0.0;
                } else {
                    // the table or the tool's top bounds what the sheet meets
                    z[k] = if male { z[k].max(0.0) } else { z[k].min(0.0) };
                }
            }
        }
        Field {
            frame,
            x0,
            y0,
            h,
            nx,
            ny,
            z,
            under,
        }
    }

    pub fn centre(&self, i: usize, j: usize) -> Point2 {
        Point2::new(
            self.x0 + (i as f64 + 0.5) * self.h,
            self.y0 + (j as f64 + 0.5) * self.h,
        )
    }

    /// A point of the plane, with height `z` along the pull, in world coordinates.
    pub fn world(&self, p: Point2, z: f64) -> Point3 {
        self.frame.point_to_world(Point3::new(p.x, p.y, z))
    }

    /// The regions under the mould or over the cavity, each its own connected set of cells.
    pub fn regions(&self) -> Vec<Region> {
        let (nx, ny) = (self.nx, self.ny);
        let dist = edt(&self.under, nx, ny);
        let mut seen = vec![false; nx * ny];
        let mut out = vec![];
        for s in 0..nx * ny {
            if !self.under[s] || seen[s] {
                continue;
            }
            seen[s] = true;
            let mut stack = vec![s];
            let (mut depth, mut deep) = (0.0f64, s);
            let (mut wide, mut widest) = (0.0f64, s);
            while let Some(k) = stack.pop() {
                if self.z[k].abs() > depth {
                    (depth, deep) = (self.z[k].abs(), k);
                }
                if dist[k] > wide {
                    (wide, widest) = (dist[k], k);
                }
                let (i, j) = (k % nx, k / nx);
                for (di, dj) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
                    let (a, b) = (i as i64 + di, j as i64 + dj);
                    if a < 0 || b < 0 || a >= nx as i64 || b >= ny as i64 {
                        continue;
                    }
                    let q = b as usize * nx + a as usize;
                    if self.under[q] && !seen[q] {
                        seen[q] = true;
                        stack.push(q);
                    }
                }
            }
            let _ = widest;
            out.push(Region {
                at: self.centre(deep % nx, deep / nx),
                depth,
                // the cell distance is to the centre of the nearest outside cell: half a cell over
                width: 2.0 * (wide.sqrt() - 0.5).max(0.5) * self.h,
            });
        }
        out
    }

    /// Valleys between raised parts of the field along its rows and columns, deeper than `eps`:
    /// on each line, the level water would stand at (the lower of the highest crests either side)
    /// less the field, over each run where that is positive.
    pub fn valleys(&self, eps: f64) -> Vec<Valley> {
        let mut out = vec![];
        let mut line = |cells: Vec<usize>| {
            let z: Vec<f64> = cells.iter().map(|&k| self.z[k]).collect();
            let n = z.len();
            let mut left = vec![f64::NAN; n];
            let mut right = vec![f64::NAN; n];
            let mut m = f64::NEG_INFINITY;
            for i in 0..n {
                m = if z[i].is_nan() {
                    f64::NEG_INFINITY
                } else {
                    m.max(z[i])
                };
                left[i] = m;
            }
            m = f64::NEG_INFINITY;
            for i in (0..n).rev() {
                m = if z[i].is_nan() {
                    f64::NEG_INFINITY
                } else {
                    m.max(z[i])
                };
                right[i] = m;
            }
            let mut i = 0;
            while i < n {
                let wet = |i: usize| !z[i].is_nan() && left[i].min(right[i]) - z[i] > eps;
                if !wet(i) {
                    i += 1;
                    continue;
                }
                let s = i;
                let (mut depth, mut deep) = (0.0, i);
                while i < n && wet(i) {
                    let d = left[i].min(right[i]) - z[i];
                    if d > depth {
                        (depth, deep) = (d, i);
                    }
                    i += 1;
                }
                let k = cells[deep];
                out.push(Valley {
                    at: self.centre(k % self.nx, k / self.nx),
                    floor: self.z[k],
                    depth,
                    width: (i - s) as f64 * self.h,
                });
            }
        };
        for j in 0..self.ny {
            line((0..self.nx).map(|i| j * self.nx + i).collect());
        }
        for i in 0..self.nx {
            line((0..self.ny).map(|j| j * self.nx + i).collect());
        }
        out
    }

    /// The largest linear draw ratio along a row or a column: the length of the surface over the
    /// run of cells it draws (from the cell before the first drawn one to the one after the last)
    /// over that run's length.
    pub fn linear_draw(&self) -> f64 {
        let mut best = 1.0f64;
        let mut line = |cells: Vec<usize>| {
            let drawn: Vec<usize> = (0..cells.len()).filter(|&i| self.under[cells[i]]).collect();
            let (Some(&a), Some(&b)) = (drawn.first(), drawn.last()) else {
                return;
            };
            let (a, b) = (a.saturating_sub(1), (b + 1).min(cells.len() - 1));
            let mut len = 0.0;
            for w in cells[a..=b].windows(2) {
                let dz = self.z[w[1]] - self.z[w[0]];
                if dz.is_nan() {
                    return;
                }
                len += (self.h * self.h + dz * dz).sqrt();
            }
            if b > a {
                best = best.max(len / ((b - a) as f64 * self.h));
            }
        };
        for j in 0..self.ny {
            line((0..self.nx).map(|i| j * self.nx + i).collect());
        }
        for i in 0..self.nx {
            line((0..self.ny).map(|j| j * self.nx + i).collect());
        }
        best
    }

    /// Where vents go: on each low plateau (a set of cells level with each other that the sheet
    /// meets last: the table round a male tool, a cavity's floor), the cells next to its walls
    /// the most closed in by them, at least `pitch` apart, their holes (diameter `d`) clear of the
    /// walls, at most `cap` in all. Each with its height.
    pub fn vents(&self, d: f64, pitch: f64, cap: usize) -> Vec<(Point2, f64)> {
        let (nx, ny) = (self.nx, self.ny);
        let tol = (0.05 * self.h).max(1e-6);
        // a plateau cell: no 4-neighbour lower than it
        let low = |k: usize| {
            let (i, j) = (k % nx, k / nx);
            !self.z[k].is_nan()
                && [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)].iter().all(|&(di, dj)| {
                    let (a, b) = (i as i64 + di, j as i64 + dj);
                    if a < 0 || b < 0 || a >= nx as i64 || b >= ny as i64 {
                        return true;
                    }
                    let q = b as usize * nx + a as usize;
                    self.z[q].is_nan() || self.z[q] >= self.z[k] - tol
                })
        };
        let wall: Vec<bool> = (0..nx * ny)
            .map(|k| {
                let (i, j) = (k % nx, k / nx);
                (j.saturating_sub(1)..(j + 2).min(ny)).any(|b| {
                    (i.saturating_sub(1)..(i + 2).min(nx)).any(|a| {
                        let q = b * nx + a;
                        self.z[q] > self.z[k] + tol
                    })
                })
            })
            .collect();
        let clear = edt_with(&wall.iter().map(|&w| !w).collect::<Vec<bool>>(), nx, ny, true);
        // the hole's edge a cell clear of the wall, at most two cells further
        let r = (0.5 * d / self.h + 1.0).max(1.0);
        let reach = (3.0 * (pitch / self.h).clamp(2.0, 8.0)) as i64;
        let mut cand: Vec<(i64, usize)> = vec![];
        for (k, c) in clear.iter().enumerate() {
            let c = c.sqrt();
            if !low(k) || c < r || c > r + 2.0 {
                continue;
            }
            let (i, j) = ((k % nx) as i64, (k / nx) as i64);
            let mut walls = 0;
            for b in (j - reach)..=(j + reach) {
                for a in (i - reach)..=(i + reach) {
                    // walls rising from this plateau: cells above it
                    if a >= 0
                        && b >= 0
                        && a < nx as i64
                        && b < ny as i64
                        && self.z[b as usize * nx + a as usize] > self.z[k] + tol
                    {
                        walls += 1;
                    }
                }
            }
            if walls > 0 {
                cand.push((walls, k));
            }
        }
        cand.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut out: Vec<(Point2, f64)> = vec![];
        for (_, k) in cand {
            let p = self.centre(k % nx, k / nx);
            if out.iter().all(|(q, _)| (*q - p).length() >= pitch) {
                out.push((p, self.z[k]));
                if out.len() == cap {
                    break;
                }
            }
        }
        out
    }
}

/// Squared distance, in cells, from each cell to the nearest cell not in `mask` (cells outside
/// the grid count as not in it): Felzenszwalb and Huttenlocher's two passes of the lower envelope
/// of parabolas.
pub fn edt(mask: &[bool], nx: usize, ny: usize) -> Vec<f64> {
    edt_with(mask, nx, ny, false)
}

/// `edt`, with the cells outside the grid in the mask where `open` (the grid's edge is no wall).
pub fn edt_with(mask: &[bool], nx: usize, ny: usize, open: bool) -> Vec<f64> {
    let big = ((nx + ny + 2) * (nx + ny + 2)) as f64;
    let mut g: Vec<f64> = mask.iter().map(|&m| if m { big } else { 0.0 }).collect();
    let pass = |f: &[f64]| -> Vec<f64> {
        // padded with an outside cell at either end
        let n = f.len() + 2;
        let mut fp = vec![if open { big } else { 0.0 }; n];
        fp[1..n - 1].copy_from_slice(f);
        let mut v = vec![0usize; n];
        let mut z = vec![0.0f64; n + 1];
        let sect = |q: usize, p: usize| {
            ((fp[q] + (q * q) as f64) - (fp[p] + (p * p) as f64)) / (2.0 * (q as f64 - p as f64))
        };
        let mut k = 0;
        z[0] = f64::NEG_INFINITY;
        z[1] = f64::INFINITY;
        for q in 1..n {
            let mut s = sect(q, v[k]);
            while s <= z[k] {
                k -= 1;
                s = sect(q, v[k]);
            }
            k += 1;
            v[k] = q;
            z[k] = s;
            z[k + 1] = f64::INFINITY;
        }
        k = 0;
        let mut d = vec![0.0; n];
        for (q, dq) in d.iter_mut().enumerate() {
            while z[k + 1] < q as f64 {
                k += 1;
            }
            let p = v[k];
            *dq = (q as f64 - p as f64) * (q as f64 - p as f64) + fp[p];
        }
        d[1..n - 1].to_vec()
    };
    for i in 0..nx {
        let col: Vec<f64> = (0..ny).map(|j| g[j * nx + i]).collect();
        for (j, v) in pass(&col).into_iter().enumerate() {
            g[j * nx + i] = v;
        }
    }
    for j in 0..ny {
        let row = g[j * nx..(j + 1) * nx].to_vec();
        g[j * nx..(j + 1) * nx].copy_from_slice(&pass(&row));
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances_match_brute_force() {
        let (nx, ny) = (13, 9);
        let mask: Vec<bool> = (0..nx * ny)
            .map(|k| {
                let (i, j) = (k % nx, k / nx);
                !(i == 4 && j == 4) && (i + 2 * j) % 11 != 3
            })
            .collect();
        let d = edt(&mask, nx, ny);
        for (k, dk) in d.iter().enumerate() {
            let (i, j) = ((k % nx) as i64, (k / nx) as i64);
            let mut best = i64::MAX;
            for b in -1..=ny as i64 {
                for a in -1..=nx as i64 {
                    let inside = a >= 0 && b >= 0 && a < nx as i64 && b < ny as i64;
                    if inside && mask[b as usize * nx + a as usize] {
                        continue;
                    }
                    best = best.min((a - i) * (a - i) + (b - j) * (b - j));
                }
            }
            assert_eq!(*dk, best as f64, "cell {i} {j}");
        }
    }
}
