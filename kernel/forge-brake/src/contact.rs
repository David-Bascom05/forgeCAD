//! Where a collision is: every part of a piece, through a stretch of the motion, that is not proven
//! clear of a tool.
//!
//! `bb::certify` answers one question (is the least distance at least the limit?) and keeps one
//! witness. This module answers the next: which cells. It walks the same cells the same way (the
//! problem's enclosures, its lower bound past the distance's error budget, the same split), keeps
//! every cell not proven clear of the limit down to the search's leaf size, and marks a cell proven
//! when a real point of it, at a real instant, is under the limit (its distance's error budget
//! included). The union of the cells kept is never smaller than the true contact (a cell is only
//! dropped when every point of it, at every instant of it, is proven at least the limit away); the
//! proven ones are exactly in it. A budget spent keeps what is left whole (conservative).
//!
//! It costs what a search does, so it is run only for a check that failed or is Near.
use crate::bb::{Cell, Problem, SLACK, sd_err};
use forge_math::Interval;
use forge_math::ipow::Powi as _;

/// A kept cell: its parameters (u, v, w, s), its certified lower bound, the enclosure's half
/// size that bound rests on, and a real point under the limit when it holds one (its distance at
/// most `d`, error budget included; parameters and image).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegionCell {
    pub cell: Cell,
    pub lower: f64,
    pub size: f64,
    pub proven: Option<(f64, [f64; 4], [f64; 3])>,
}

/// The region's cells, and whether every cell was settled within the budget.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub cells: Vec<RegionCell>,
    pub complete: bool,
}

impl Region {
    /// The least distance a real point of the region is proven to have (negative: that deep into
    /// the tool), if any cell holds one.
    pub fn depth(&self) -> Option<f64> {
        self.cells
            .iter()
            .filter_map(|c| c.proven.map(|p| p.0))
            .min_by(|a, b| a.total_cmp(b))
    }

    /// The least lower bound over the region: no point of it is nearer than this.
    pub fn lower(&self) -> f64 {
        self.cells.iter().map(|c| c.lower).fold(f64::INFINITY, f64::min)
    }

    /// The motion's span over the region (s: the bend's turn, or the ram's height).
    pub fn span(&self) -> Option<(f64, f64)> {
        let mut out: Option<(f64, f64)> = None;
        for c in &self.cells {
            let s = c.cell.p[3];
            out = Some(match out {
                None => (s.lo, s.hi),
                Some((a, b)) => (a.min(s.lo), b.max(s.hi)),
            });
        }
        out
    }
}

fn magnitude(e: &[Interval; 3]) -> f64 {
    e.iter().map(|i| i.lo.abs().max(i.hi.abs())).fold(0.0, f64::max)
}

/// The cell's certified lower bound and the size it rests on (as `bb`'s search bounds it).
fn bound<P: Problem>(pb: &P, c: &Cell) -> (f64, f64) {
    let e = pb.enclose(c);
    let (lb, size) = pb.lower(&e);
    ((lb - sd_err(magnitude(&e))).next_down(), size)
}

/// The parameter whose width moves the image most (as `bb`'s search splits).
fn split_dim<P: Problem>(pb: &P, c: &Cell) -> usize {
    let m = c.mid();
    let mut best = (0usize, -1.0f64);
    for k in 0..4 {
        if c.p[k].width() <= 0.0 {
            continue;
        }
        let (mut lo, mut hi) = (m, m);
        lo[k] = c.p[k].lo;
        hi[k] = c.p[k].hi;
        let (a, b) = (pb.image(lo), pb.image(hi));
        let wt = pb.weights();
        let mv = (0..3)
            .map(|i| (wt[i] * (a[i] - b[i])).ipow(2))
            .sum::<f64>()
            .sqrt();
        let score = mv.max(1e-3 * c.p[k].width());
        if score > best.1 {
            best = (k, score);
        }
    }
    best.0
}

/// Every cell of `roots` not proven at least `limit` away, split down to the search's leaf size
/// (half of `res`), at most `budget` cells examined.
pub fn region<P: Problem>(pb: &P, roots: &[Cell], limit: f64, res: f64, budget: usize) -> Region {
    let mut stack: Vec<Cell> = roots.iter().copied().filter(|c| pb.may_hold(c)).collect();
    stack.reverse();
    let mut cells = Vec::new();
    let mut spent = 0usize;
    let mut complete = true;
    while let Some(cell) = stack.pop() {
        let (lb, hd) = bound(pb, &cell);
        if lb >= limit {
            // every point of it, at every instant of it, proven at least the limit away
            continue;
        }
        spent += 1;
        let proven = pb.witness(&cell).and_then(|q| {
            let at = pb.image(q);
            let m = at.iter().map(|x| x.abs()).fold(0.0, f64::max);
            let d = (pb.sd_bounds(at).1 + sd_err(m)).next_up();
            (d < limit).then_some((d, q, at))
        });
        if hd <= 0.5 * res || spent > budget {
            if spent > budget {
                complete = false;
            }
            cells.push(RegionCell {
                cell,
                lower: lb,
                size: hd + SLACK,
                proven,
            });
            continue;
        }
        let k = split_dim(pb, &cell);
        let (a, b) = cell.p[k].split();
        for half in [b, a] {
            let mut c = cell;
            c.p[k] = half;
            if pb.may_hold(&c) {
                stack.push(c);
            }
        }
    }
    Region { cells, complete }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit segment moving along x by s, against a ball of radius 1 at (3, 0, 0).
    struct Seg1;
    impl Problem for Seg1 {
        fn may_hold(&self, _: &Cell) -> bool {
            true
        }
        fn enclose(&self, c: &Cell) -> [Interval; 3] {
            [c.p[0] + c.p[3], c.p[1], Interval::point(0.0)]
        }
        fn witness(&self, c: &Cell) -> Option<[f64; 4]> {
            Some(c.mid())
        }
        fn image(&self, q: [f64; 4]) -> [f64; 3] {
            [q[0] + q[3], q[1], 0.0]
        }
        fn sd(&self, p: [f64; 3]) -> f64 {
            ((p[0] - 3.0).ipow(2) + p[1].ipow(2) + p[2].ipow(2)).sqrt() - 1.0
        }
    }

    /// The segment's points within 0.6 of the ball (x + s >= 1.4): the region holds every one of
    /// them (never smaller), its proven cells only points truly under the limit, its depth the
    /// true least distance (0.5 here, at x = 1 when s = 0.5) to within the leaf size.
    #[test]
    fn the_region_holds_every_point_under_the_limit() {
        let root = Cell {
            p: [
                Interval::new(0.0, 1.0),
                Interval::point(0.0),
                Interval::point(0.0),
                Interval::new(0.0, 0.5),
            ],
            tag: 0,
        };
        let (limit, res) = (0.6, 0.01);
        let r = region(&Seg1, &[root], limit, res, 1_000_000);
        assert!(r.complete && !r.cells.is_empty());
        for i in 0..=200 {
            for j in 0..=50 {
                let q = [i as f64 / 200.0, 0.0, 0.0, 0.5 * j as f64 / 50.0];
                if Seg1.sd(Seg1.image(q)) < limit {
                    assert!(
                        r.cells
                            .iter()
                            .any(|c| (0..4).all(|k| c.cell.p[k].lo <= q[k] && q[k] <= c.cell.p[k].hi)),
                        "{q:?} is under the limit and in no cell"
                    );
                }
            }
        }
        for c in &r.cells {
            if let Some((d, q, _)) = c.proven {
                assert!(Seg1.sd(Seg1.image(q)) < limit && d < limit);
            }
            assert!(c.lower < limit);
        }
        let d = r.depth().expect("a proven point");
        assert!((d - 0.5).abs() < 0.02, "{d}");
        // nothing is kept far from the ball
        for c in &r.cells {
            assert!(c.cell.p[0].hi + c.cell.p[3].hi >= 1.4 - 0.02);
        }
    }
}
