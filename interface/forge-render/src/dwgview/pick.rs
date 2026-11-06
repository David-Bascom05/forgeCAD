//! A uniform grid over a scene's segments, for the item nearest a point: each segment is filed in
//! every cell it passes through (walked cell by cell, not by its box), so a long diagonal costs
//! the cells it touches and a query reads only the cells round the point. It holds segment
//! numbers only: the segments themselves are the caller's (the GPU's instance bytes, in `Geom`).

pub struct Grid {
    lo: [f32; 2],
    cell: f32,
    nx: usize,
    ny: usize,
    /// Cell k's segments are `ids[starts[k]..starts[k + 1]]`.
    starts: Vec<u32>,
    ids: Vec<u32>,
    n: usize,
}

/// The distance from p to the segment ab.
pub fn seg_dist(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let (px, py) = (p[0] - a[0], p[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        ((px * dx + py * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (ex, ey) = (px - dx * t, py - dy * t);
    (ex * ex + ey * ey).sqrt()
}

/// Most cells a query reads before it reads every segment instead.
const QUERY_CELLS: usize = 1 << 14;

impl Grid {
    /// About two segments a cell, inside the box lo..hi that holds them all; `seg(i)` is the
    /// i-th of `n`.
    pub fn new(n: usize, seg: impl Fn(usize) -> ([f32; 2], [f32; 2]), lo: [f32; 2], hi: [f32; 2]) -> Grid {
        let w = [(hi[0] - lo[0]).max(1e-6), (hi[1] - lo[1]).max(1e-6)];
        let want = (n / 2).clamp(1, 1 << 22) as f32;
        let cell = (w[0] * w[1] / want).sqrt().max(w[0].max(w[1]) / 4096.0).max(1e-6);
        let nx = (w[0] / cell) as usize + 1;
        let ny = (w[1] / cell) as usize + 1;
        let mut starts = vec![0u32; nx * ny + 1];
        for i in 0..n {
            let (a, b) = seg(i);
            walk_cells(lo, cell, nx, ny, a, b, |k| starts[k] += 1);
        }
        let mut sum = 0u32;
        for c in starts.iter_mut() {
            let k = *c;
            *c = sum;
            sum += k;
        }
        let mut ids = vec![0; sum as usize];
        let mut fill = starts.clone();
        for i in 0..n {
            let (a, b) = seg(i);
            walk_cells(lo, cell, nx, ny, a, b, |k| {
                ids[fill[k] as usize] = i as u32;
                fill[k] += 1;
            });
        }
        Grid {
            lo,
            cell,
            nx,
            ny,
            starts,
            ids,
            n,
        }
    }

    fn cell_of(&self, p: [f32; 2]) -> (usize, usize) {
        cell_of(self.lo, self.cell, self.nx, self.ny, p)
    }

    /// Every segment that might be within `r` of `p` (some more than once).
    pub fn near(&self, p: [f32; 2], r: f32, mut f: impl FnMut(u32)) {
        let (x0, y0) = self.cell_of([p[0] - r, p[1] - r]);
        let (x1, y1) = self.cell_of([p[0] + r, p[1] + r]);
        if (x1 - x0 + 1) * (y1 - y0 + 1) > QUERY_CELLS {
            (0..self.n as u32).for_each(f);
            return;
        }
        for y in y0..=y1 {
            for x in x0..=x1 {
                let k = y * self.nx + x;
                for &i in &self.ids[self.starts[k] as usize..self.starts[k + 1] as usize] {
                    f(i);
                }
            }
        }
    }
}

fn cell_of(lo: [f32; 2], cell: f32, nx: usize, ny: usize, p: [f32; 2]) -> (usize, usize) {
    let c = |v: f32, lo: f32, n: usize| (((v - lo) / cell).floor().max(0.0) as usize).min(n - 1);
    (c(p[0], lo[0], nx), c(p[1], lo[1], ny))
}

/// The cells the segment ab passes through, from a's to b's (Amanatides and Woo's walk): as many
/// steps as the cells are apart, so it always ends in b's.
fn walk_cells(
    lo: [f32; 2],
    cell: f32,
    nx: usize,
    ny: usize,
    a: [f32; 2],
    b: [f32; 2],
    mut f: impl FnMut(usize),
) {
    let (mut cx, mut cy) = cell_of(lo, cell, nx, ny, a);
    let (ex, ey) = cell_of(lo, cell, nx, ny, b);
    f(cy * nx + cx);
    let steps = cx.abs_diff(ex) + cy.abs_diff(ey);
    if steps == 0 {
        return;
    }
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let axis = |d: f32, from: f32, c: usize, lo: f32| -> (f32, f32) {
        if d > 0.0 {
            ((lo + (c + 1) as f32 * cell - from) / d, cell / d)
        } else if d < 0.0 {
            ((lo + c as f32 * cell - from) / d, -cell / d)
        } else {
            (f32::INFINITY, f32::INFINITY)
        }
    };
    let (mut tx, ddx) = axis(dx, a[0], cx, lo[0]);
    let (mut ty, ddy) = axis(dy, a[1], cy, lo[1]);
    for _ in 0..steps {
        // a step only ever goes towards b's cell
        let x_left = cx != ex;
        let y_left = cy != ey;
        if x_left && (!y_left || tx < ty) {
            if ex > cx {
                cx += 1;
            } else {
                cx -= 1;
            }
            tx += ddx;
        } else {
            if ey > cy {
                cy += 1;
            } else {
                cy -= 1;
            }
            ty += ddy;
        }
        f(cy * nx + cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_of_many() {
        let mut segs = vec![];
        for k in 0..1000 {
            let x = (k % 40) as f32 * 10.0;
            let y = (k / 40) as f32 * 10.0;
            segs.push(([x, y], [x + 5.0, y + 5.0]));
        }
        // a long one across them all
        segs.push(([0.0, 3.0], [400.0, 243.0]));
        let g = Grid::new(segs.len(), |i| segs[i], [0.0, 0.0], [400.0, 250.0]);
        let nearest = |p: [f32; 2], r: f32, ok: &dyn Fn(u32) -> bool| {
            let mut best: Option<(u32, f32)> = None;
            g.near(p, r, |i| {
                let d = seg_dist(p, segs[i as usize].0, segs[i as usize].1);
                if d <= r && best.is_none_or(|b| d < b.1) && ok(i) {
                    best = Some((i, d));
                }
            });
            best.map(|b| b.0)
        };
        assert_eq!(nearest([102.6, 52.4], 1.0, &|_| true), Some(210));
        assert_eq!(nearest([200.0, 123.0], 0.5, &|_| true), Some(1000));
        assert_eq!(nearest([200.0, 123.0], 0.5, &|i| i != 1000), None);
        assert_eq!(nearest([8.0, 1.0], 0.5, &|_| true), None);
    }

    #[test]
    fn a_walk_touches_every_cell_the_segment_crosses() {
        // by brute force: sample the segment finely
        let (lo, cell, nx, ny) = ([0.0f32, 0.0], 1.0f32, 50, 50);
        for (a, b) in [
            ([0.5f32, 0.5], [47.3f32, 12.9]),
            ([40.0, 2.0], [3.1, 44.4]),
            ([10.5, 10.5], [10.5, 40.5]),
            ([2.2, 30.0], [49.9, 30.0]),
        ] {
            let mut got = std::collections::BTreeSet::new();
            walk_cells(lo, cell, nx, ny, a, b, |k| {
                got.insert(k);
            });
            for s in 0..=10000 {
                let t = s as f32 / 10000.0;
                let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
                let (x, y) = cell_of(lo, cell, nx, ny, p);
                assert!(got.contains(&(y * nx + x)), "{a:?} {b:?} at {p:?}");
            }
        }
    }
}
