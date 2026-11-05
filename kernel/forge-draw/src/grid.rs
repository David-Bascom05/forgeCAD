use forge_math::Point2;

/// Items binned by the 2D boxes they cover, for finding what is near a point or a box.
pub struct Grid {
    lo: Point2,
    cell: f64,
    nx: usize,
    ny: usize,
    cells: Vec<Vec<u32>>,
    /// Made all at once (`packed`): cell k's ids are `ids[start[k]..start[k + 1]]`, `cells` empty.
    packed: Option<(Vec<u32>, Vec<u32>)>,
}

impl Grid {
    /// A grid over the box `lo..hi` sized for about `items` entries.
    pub fn new(lo: Point2, hi: Point2, items: usize) -> Grid {
        let (w, h) = ((hi.x - lo.x).max(1e-9), (hi.y - lo.y).max(1e-9));
        let n = (items.max(1) as f64).sqrt().clamp(1.0, 512.0);
        let cell = (w.max(h) / n).max(1e-9);
        let nx = ((w / cell).ceil() as usize).clamp(1, 1024);
        let ny = ((h / cell).ceil() as usize).clamp(1, 1024);
        Grid {
            lo,
            cell,
            nx,
            ny,
            cells: vec![Vec::new(); nx * ny],
            packed: None,
        }
    }

    fn ix(&self, x: f64, n: usize, lo: f64) -> usize {
        let k = ((x - lo) / self.cell).floor();
        if k.is_nan() || k < 0.0 {
            0
        } else {
            (k as usize).min(n - 1)
        }
    }

    /// Cell k's ids, in the order they went in.
    fn cell_ids(&self, k: usize) -> &[u32] {
        match &self.packed {
            Some((start, ids)) => &ids[start[k] as usize..start[k + 1] as usize],
            None => &self.cells[k],
        }
    }

    /// The grid `new` makes for `n` items, item `i` (its box `boxes(i)`) put in as `insert`
    /// would, in order: the same cells holding the same ids in the same order, made on all cores
    /// (each run of items counted, then placed, into one array).
    pub fn packed(
        lo: Point2,
        hi: Point2,
        n: usize,
        boxes: impl Fn(usize) -> (Point2, Point2) + Sync,
    ) -> Grid {
        let mut g = Grid::new(lo, hi, n);
        let cells = g.nx * g.ny;
        g.cells = Vec::new();
        const RUN: usize = 16384;
        let runs = n.div_ceil(RUN);
        let span = |g: &Grid, i: usize| {
            let (a, b) = boxes(i);
            (
                g.ix(a.x, g.nx, g.lo.x),
                g.ix(b.x, g.nx, g.lo.x),
                g.ix(a.y, g.ny, g.lo.y),
                g.ix(b.y, g.ny, g.lo.y),
            )
        };
        // how many of each run's items go in each cell
        let counts: Vec<Vec<u32>> = forge_math::par::map(runs, |r| {
            let mut c = vec![0u32; cells];
            for i in r * RUN..((r + 1) * RUN).min(n) {
                let (x0, x1, y0, y1) = span(&g, i);
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        c[y * g.nx + x] += 1;
                    }
                }
            }
            c
        });
        // where each run starts in each cell: the cells in order, the runs in order within each
        let mut start = vec![0u32; cells + 1];
        let mut at = vec![vec![0u32; cells]; runs];
        let mut total = 0u32;
        for k in 0..cells {
            start[k] = total;
            for r in 0..runs {
                at[r][k] = total;
                total += counts[r][k];
            }
        }
        start[cells] = total;
        let placed: Vec<Vec<(u32, u32)>> = forge_math::par::map(runs, |r| {
            let mut next = at[r].clone();
            let mut out = Vec::new();
            for i in r * RUN..((r + 1) * RUN).min(n) {
                let (x0, x1, y0, y1) = span(&g, i);
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        let k = y * g.nx + x;
                        out.push((next[k], i as u32));
                        next[k] += 1;
                    }
                }
            }
            out
        });
        let mut ids = vec![0u32; total as usize];
        for run in placed {
            for (slot, id) in run {
                ids[slot as usize] = id;
            }
        }
        g.packed = Some((start, ids));
        g
    }

    pub fn insert(&mut self, id: u32, lo: Point2, hi: Point2) {
        let (x0, x1) = (
            self.ix(lo.x, self.nx, self.lo.x),
            self.ix(hi.x, self.nx, self.lo.x),
        );
        let (y0, y1) = (
            self.ix(lo.y, self.ny, self.lo.y),
            self.ix(hi.y, self.ny, self.lo.y),
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.cells[y * self.nx + x].push(id);
            }
        }
    }

    /// The ids in the cell holding `p` (each once per cell).
    pub fn at(&self, p: Point2) -> &[u32] {
        let (x, y) = (self.ix(p.x, self.nx, self.lo.x), self.ix(p.y, self.ny, self.lo.y));
        self.cell_ids(y * self.nx + x)
    }

    /// The ids in every cell the box meets, sorted, each once.
    pub fn near(&self, lo: Point2, hi: Point2, out: &mut Vec<u32>) {
        out.clear();
        let (x0, x1) = (
            self.ix(lo.x, self.nx, self.lo.x),
            self.ix(hi.x, self.nx, self.lo.x),
        );
        let (y0, y1) = (
            self.ix(lo.y, self.ny, self.lo.y),
            self.ix(hi.y, self.ny, self.lo.y),
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                out.extend_from_slice(self.cell_ids(y * self.nx + x));
            }
        }
        out.sort_unstable();
        out.dedup();
    }

    /// Each id in every cell the box meets, as often as it is in them (unsorted).
    pub fn each_near(&self, lo: Point2, hi: Point2, mut f: impl FnMut(u32)) {
        let (x0, x1) = (
            self.ix(lo.x, self.nx, self.lo.x),
            self.ix(hi.x, self.nx, self.lo.x),
        );
        let (y0, y1) = (
            self.ix(lo.y, self.ny, self.lo.y),
            self.ix(hi.y, self.ny, self.lo.y),
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                for &i in self.cell_ids(y * self.nx + x) {
                    f(i);
                }
            }
        }
    }

    /// The number of cells.
    pub fn cells(&self) -> usize {
        self.nx * self.ny
    }

    /// Cell `k` (row by row): its column, row and ids, in the order they went in.
    pub fn cell_at(&self, k: usize) -> ((usize, usize), &[u32]) {
        ((k % self.nx, k / self.nx), self.cell_ids(k))
    }

    /// The cell holding `p`.
    pub fn cell(&self, p: Point2) -> (usize, usize) {
        (self.ix(p.x, self.nx, self.lo.x), self.ix(p.y, self.ny, self.lo.y))
    }

    /// Each id in every cell the box meets, with that cell (an id in several of them once for
    /// each).
    pub fn each_near_in(&self, lo: Point2, hi: Point2, mut f: impl FnMut(u32, (usize, usize))) {
        let (x0, x1) = (
            self.ix(lo.x, self.nx, self.lo.x),
            self.ix(hi.x, self.nx, self.lo.x),
        );
        let (y0, y1) = (
            self.ix(lo.y, self.ny, self.lo.y),
            self.ix(hi.y, self.ny, self.lo.y),
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                for &i in self.cell_ids(y * self.nx + x) {
                    f(i, (x, y));
                }
            }
        }
    }

    /// Whether any id in a cell the box meets is one `f` wants.
    pub fn any_near(&self, lo: Point2, hi: Point2, f: impl Fn(u32) -> bool) -> bool {
        let (x0, x1) = (
            self.ix(lo.x, self.nx, self.lo.x),
            self.ix(hi.x, self.nx, self.lo.x),
        );
        let (y0, y1) = (
            self.ix(lo.y, self.ny, self.lo.y),
            self.ix(hi.y, self.ny, self.lo.y),
        );
        (y0..=y1).any(|y| (x0..=x1).any(|x| self.cell_ids(y * self.nx + x).iter().any(|&i| f(i))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A packed grid holds what inserting the same boxes one by one gives: every cell, its ids,
    /// their order.
    #[test]
    fn a_packed_grid_is_the_inserted_one() {
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        for n in [1usize, 7, 1000, 40_000] {
            let boxes: Vec<(Point2, Point2)> = (0..n)
                .map(|_| {
                    let (x, y) = (100.0 * rnd(), 60.0 * rnd());
                    let (w, h) = (if rnd() < 0.1 { 50.0 } else { 2.0 } * rnd(), 2.0 * rnd());
                    (Point2::new(x, y), Point2::new(x + w, y + h))
                })
                .collect();
            let (lo, hi) = (Point2::new(0.0, 0.0), Point2::new(100.0, 60.0));
            let mut one = Grid::new(lo, hi, n);
            for (i, (a, b)) in boxes.iter().enumerate() {
                one.insert(i as u32, *a, *b);
            }
            let all = Grid::packed(lo, hi, n, |i| boxes[i]);
            assert_eq!(one.cells(), all.cells());
            for k in 0..one.cells() {
                assert_eq!(one.cell_at(k), all.cell_at(k), "cell {k} of {n}");
            }
        }
    }
}
