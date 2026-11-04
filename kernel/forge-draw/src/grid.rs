use forge_math::Point2;

/// Items binned by the 2D boxes they cover, for finding what is near a point or a box.
pub struct Grid {
    lo: Point2,
    cell: f64,
    nx: usize,
    ny: usize,
    cells: Vec<Vec<u32>>,
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
        &self.cells[y * self.nx + x]
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
                out.extend_from_slice(&self.cells[y * self.nx + x]);
            }
        }
        out.sort_unstable();
        out.dedup();
    }
}
