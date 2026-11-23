//! Certified least distance by branch and bound over a piece's parameters and the motion.
//!
//! A cell is a box of parameters (u, v in the strip, w through the material, s along the
//! motion). The problem encloses the cell's image by interval arithmetic, so every point of the
//! piece over every instant of the cell's s interval lies in the enclosure; the target's signed
//! distance is 1-Lipschitz, so over the whole cell it is at least d(centre) - half-diagonal. A
//! cell is cleared when that bound reaches the limit, and that clears its whole stretch of the
//! motion: nothing between samples is skipped. A witness (a real point of the piece at a real
//! instant) under the limit is a proven violation. Cells are split until one of the two holds or
//! they are smaller than the resolution, where the bound is within it of the truth.
use forge_math::Interval;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    /// u, v, w, s.
    pub p: [Interval; 4],
    /// A flag the problem sets on its roots (for example: in the moving bend's span).
    pub tag: u8,
}

impl Cell {
    pub fn mid(&self) -> [f64; 4] {
        self.p.map(|i| i.mid())
    }
}

/// What the search needs to know about one piece against one target.
pub trait Problem {
    /// Whether the cell may hold material (a lower bound test: false only when it surely holds none).
    fn may_hold(&self, c: &Cell) -> bool;
    /// An enclosure of the image of every point of the cell, in the target's frame.
    fn enclose(&self, c: &Cell) -> [Interval; 3];
    /// A real point of the piece in or near the cell, at an instant in its interval: its
    /// parameters. None if none can be named.
    fn witness(&self, c: &Cell) -> Option<[f64; 4]>;
    /// The image of a parameter point.
    fn image(&self, q: [f64; 4]) -> [f64; 3];
    /// The target's signed distance at a point.
    fn sd(&self, p: [f64; 3]) -> f64;
    /// Bounds on the signed distance, where it is not known exactly: (at most it, at least it).
    /// A cell is cleared on the first, a witness proven on the second.
    fn sd_bounds(&self, p: [f64; 3]) -> (f64, f64) {
        let d = self.sd(p);
        (d, d)
    }
    /// A lower bound on the distance over an enclosure, and the enclosure's size that bound
    /// depends on (the cell is a leaf once that is under the resolution). By default: the
    /// distance at its centre less its half-diagonal (the distance is 1-Lipschitz).
    fn lower(&self, e: &[Interval; 3]) -> (f64, f64) {
        let mid = e.map(|i| i.mid());
        let hd = ((e.iter().map(|i| (0.5 * i.width()).powi(2)).sum::<f64>()).sqrt() + SLACK).next_up();
        (self.sd_bounds(mid).0 - hd, hd)
    }
    /// How much each coordinate of the image counts when choosing what to split.
    fn weights(&self) -> [f64; 3] {
        [1.0; 3]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Verdict {
    /// Proven: the distance is at least `lower` >= limit - res at every point of the piece at every
    /// instant of the motion; `upper` is at least a real point's distance.
    Clear { lower: f64, upper: f64 },
    /// Proven: a real point of the piece, at a real instant (parameters `q`, image `at`), whose
    /// distance is at most `d` < limit (the error budget included).
    Hit { d: f64, q: [f64; 4], at: [f64; 3] },
    /// Neither proven (the budget ran out, or cells under the resolution held no witness below the
    /// limit); `lower` is what was proven. A caller refuses it.
    Undecided { lower: f64 },
}

#[derive(Clone, Copy, Debug)]
struct Item {
    lb: f64,
    cell: Cell,
}

impl PartialEq for Item {
    fn eq(&self, o: &Item) -> bool {
        self.lb.total_cmp(&o.lb) == Ordering::Equal
    }
}
impl Eq for Item {}
impl PartialOrd for Item {
    fn partial_cmp(&self, o: &Item) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Item {
    fn cmp(&self, o: &Item) -> Ordering {
        // the least bound first
        o.lb.total_cmp(&self.lb)
    }
}

/// Cells spent by every search on this thread (for tracing).
pub static CELLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Float rounding in the coefficients (rigid motions with double entries), widened past.
pub const SLACK: f64 = 1e-9;

/// The error budget of a double signed distance evaluated at a point of magnitude `m` (a few
/// dozen roundings, each within half an ulp of numbers no larger than the model): the bounds
/// below are widened by it, outward. An assumption to discharge (L5: proven predicates).
pub fn sd_err(m: f64) -> f64 {
    1e-9 + 64.0 * f64::EPSILON * (1.0 + m)
}

fn magnitude(e: &[Interval; 3]) -> f64 {
    e.iter().map(|i| i.lo.abs().max(i.hi.abs())).fold(0.0, f64::max)
}

/// The cell's certified lower bound (rounded down past the distance's error budget) and the size
/// it rests on.
fn bound<P: Problem>(pb: &P, c: &Cell) -> (f64, f64) {
    let e = pb.enclose(c);
    let (lb, size) = pb.lower(&e);
    ((lb - sd_err(magnitude(&e))).next_down(), size)
}

/// Searches `roots` for the least distance to the problem's target against `limit`, to
/// resolution `res`, spending at most `budget` cells. `measure`: go on past the limit until the
/// least distance itself is known within `res`; else stop as soon as the limit is proven.
pub fn certify<P: Problem>(
    pb: &P,
    roots: &[Cell],
    limit: f64,
    res: f64,
    budget: usize,
    measure: bool,
) -> Verdict {
    certify_below(pb, roots, limit, res, budget, measure, f64::INFINITY)
}

/// `certify`, measuring no further than `cut`: a distance known to be at least `cut` is not
/// measured more finely (another piece is already nearer).
pub fn certify_below<P: Problem>(
    pb: &P,
    roots: &[Cell],
    limit: f64,
    res: f64,
    budget: usize,
    measure: bool,
    cut: f64,
) -> Verdict {
    let mut spent = 0usize;
    let v = search(pb, roots, limit, res, budget, measure, cut, &mut spent);
    CELLS.fetch_add(spent, std::sync::atomic::Ordering::Relaxed);
    v
}

#[allow(clippy::too_many_arguments)]
fn search<P: Problem>(
    pb: &P,
    roots: &[Cell],
    limit: f64,
    res: f64,
    budget: usize,
    measure: bool,
    cut: f64,
    spent: &mut usize,
) -> Verdict {
    let mut heap = BinaryHeap::new();
    for c in roots {
        if pb.may_hold(c) {
            let (lb, _) = bound(pb, c);
            heap.push(Item { lb, cell: *c });
        }
    }
    let mut lower = f64::INFINITY;
    let mut upper = f64::INFINITY;
    while let Some(Item { lb, cell }) = heap.pop() {
        // the least bound left settles it: so does every other
        // measured to `res`, or to a fifth of the margin over the limit when that is more
        let tol = res.max(0.2 * (upper - limit));
        if (!measure && lb >= limit) || (measure && lb >= limit && lb >= upper.min(cut) - tol) {
            lower = lower.min(lb);
            break;
        }
        *spent += 1;
        // measuring past a proven pass gets a tenth of the budget
        if measure && lb >= limit && *spent > budget / 10 {
            return Verdict::Clear {
                lower: lower.min(lb),
                upper: upper.max(lower.min(lb)),
            };
        }
        if *spent > budget {
            lower = lower.min(lb);
            // what is left is proven clear of the limit: a pass, measured less finely
            if lower >= limit - res {
                return Verdict::Clear {
                    lower,
                    upper: upper.max(lower),
                };
            }
            return Verdict::Undecided { lower };
        }
        if let Some(q) = pb.witness(&cell) {
            let at = pb.image(q);
            // at most this, past the error budget: a hit only if that is under the limit
            let m = at.iter().map(|x| x.abs()).fold(0.0, f64::max);
            let d = (pb.sd_bounds(at).1 + sd_err(m)).next_up();
            if d < limit {
                return Verdict::Hit { d, q, at };
            }
            upper = upper.min(d);
        }
        let (_, hd) = bound(pb, &cell);
        if hd <= 0.5 * res {
            lower = lower.min(lb);
            continue;
        }
        let k = split_dim(pb, &cell);
        let (a, b) = cell.p[k].split();
        for half in [a, b] {
            let mut c = cell;
            c.p[k] = half;
            if pb.may_hold(&c) {
                let (lb, _) = bound(pb, &c);
                heap.push(Item { lb, cell: c });
            }
        }
    }
    // every cell is settled: clear only if each was proven to within the resolution, else
    // undecided (a leaf under it with no witness below the limit decides nothing)
    if lower >= limit - res {
        Verdict::Clear {
            lower,
            upper: upper.max(lower),
        }
    } else {
        Verdict::Undecided { lower }
    }
}

/// The parameter whose width moves the image most: the spatial ones count as their width, the
/// motion by how far the cell's centre travels across it.
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
            .map(|i| (wt[i] * (a[i] - b[i])).powi(2))
            .sum::<f64>()
            .sqrt();
        // a parameter that moves nothing at the centre may still curve: count its width a little
        let score = mv.max(1e-3 * c.p[k].width());
        if score > best.1 {
            best = (k, score);
        }
    }
    best.0
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
            ((p[0] - 3.0).powi(2) + p[1].powi(2) + p[2].powi(2)).sqrt() - 1.0
        }
    }

    fn root(s1: f64) -> Cell {
        Cell {
            p: [
                Interval::new(0.0, 1.0),
                Interval::point(0.0),
                Interval::point(0.0),
                Interval::new(0.0, s1),
            ],
            tag: 0,
        }
    }

    #[test]
    fn clears_through_the_motion() {
        // the segment's far end reaches x = 1 + 0.5 = 1.5: the ball's surface at 2 is 0.5 away
        match certify(&Seg1, &[root(0.5)], 0.4, 0.01, 100_000, false) {
            Verdict::Clear { lower, .. } => assert!((0.39..=0.5 + 1e-9).contains(&lower), "{lower}"),
            v => panic!("{v:?}"),
        }
        match certify(&Seg1, &[root(0.5)], 0.4, 0.001, 100_000, true) {
            Verdict::Clear { lower, upper } => {
                assert!(
                    lower <= 0.5 + 1e-9
                        && upper >= 0.5 - 1e-9
                        && upper - lower <= 0.001f64.max(0.2 * (upper - 0.4)) + 1e-12,
                    "{lower} {upper}"
                )
            }
            v => panic!("{v:?}"),
        }
        match certify(&Seg1, &[root(0.5)], 0.6, 0.01, 100_000, false) {
            Verdict::Hit { d, .. } => assert!(d < 0.6),
            v => panic!("{v:?}"),
        }
    }

    #[test]
    fn just_inside_and_just_outside_the_limit() {
        // the true least distance is 1 - s1: limit 0.5, resolution 1e-4
        match certify(&Seg1, &[root(0.498)], 0.5, 1e-4, 1_000_000, false) {
            Verdict::Clear { lower, .. } => assert!((0.5 - 1e-4..=0.502).contains(&lower), "{lower}"),
            v => panic!("{v:?}"),
        }
        match certify(&Seg1, &[root(0.502)], 0.5, 1e-4, 1_000_000, false) {
            Verdict::Hit { d, .. } => assert!((0.498 - 1e-6..0.5).contains(&d), "{d}"),
            v => panic!("{v:?}"),
        }
    }

    /// The same, with no point ever named: nothing can be proven against it.
    struct Blind;
    impl Problem for Blind {
        fn may_hold(&self, _: &Cell) -> bool {
            true
        }
        fn enclose(&self, c: &Cell) -> [Interval; 3] {
            Seg1.enclose(c)
        }
        fn witness(&self, _: &Cell) -> Option<[f64; 4]> {
            None
        }
        fn image(&self, q: [f64; 4]) -> [f64; 3] {
            Seg1.image(q)
        }
        fn sd(&self, p: [f64; 3]) -> f64 {
            Seg1.sd(p)
        }
    }

    #[test]
    fn unproven_is_undecided_never_clear() {
        // too near to clear, and no witness: undecided, at the end of the search and at the budget
        assert!(matches!(
            certify(&Blind, &[root(0.502)], 0.5, 1e-3, 1_000_000, false),
            Verdict::Undecided { .. }
        ));
        assert!(matches!(
            certify(&Blind, &[root(0.502)], 0.5, 1e-6, 50, false),
            Verdict::Undecided { .. }
        ));
        // far enough, it still clears
        assert!(matches!(
            certify(&Blind, &[root(0.3)], 0.5, 1e-3, 1_000_000, false),
            Verdict::Clear { .. }
        ));
    }

    #[test]
    fn a_hit_between_samples_is_found() {
        // only the very end of the motion comes within the limit
        match certify(&Seg1, &[root(0.97)], 0.05, 0.005, 100_000, false) {
            Verdict::Hit { d, q, .. } => assert!(d < 0.05 && q[3] > 0.9),
            v => panic!("{v:?}"),
        }
    }
}
