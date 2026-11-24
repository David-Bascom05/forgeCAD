//! forge-bb: the certified branch and bound of ForgeCAD's clearance checks (CAM's tool against
//! the part, the press brake's part against its tooling), one search for all of them.
//!
//! A [`Search`] is cells over some parameter space, each with a proven enclosure of the signed
//! distance over it. [`certify`] searches them least bound first and answers:
//! - **Clear** only when every cell's proven lower bound is at least the limit plus the margin
//!   (and no cell was left too small to split): the true distance is at least the limit, proven;
//! - **Hit** only at a real point whose proven upper bound is below the limit minus the margin;
//! - **Undecided** otherwise (a cell reached the resolution undecided, or the budget ran out),
//!   which callers refuse.
//!
//! There is no "within the resolution" pass: a bound that is not proven above the limit is not
//! a Clear (the owner's ruling, 2026-11-24). Its soundness is verify/sheet's
//! BranchBound.lean `search_sound`, for any order of the cells and any evaluator that meets the
//! enclosure, witness and cover contracts.
//!
//! The heap is an array (no closures in the loop), so the search is in the style Aeneas takes.

use forge_num::ival::Ival;

/// How a cell stands against the region it is searched over (a face's trim, say).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holds {
    /// No point of the cell is in the region: the cell is dropped.
    Out,
    /// Some points may be, some may not.
    Maybe,
    /// Every point is.
    In,
}

/// A cell search: its cells, their bounds, witnesses and splits.
pub trait Search {
    type Cell: Clone;
    type Witness;
    /// A proven enclosure of the signed distance over the cell: `lo` is its lower bound, the
    /// search's key.
    fn bound(&self, c: &Self::Cell) -> Ival;
    /// Whether the cell's points are in the region.
    fn holds(&self, c: &Self::Cell) -> Holds;
    /// A tighter proven lower bound than `sd` (the cell's bound), where the search knows one
    /// (e.g. a cell crossing the region's edge, bounded through a point surely inside); `sd`
    /// unchanged otherwise.
    #[inline]
    fn refine(&self, _c: &Self::Cell, _h: Holds, sd: Ival, _clear: f64) -> Ival {
        sd
    }
    /// A real point of the cell whose proven upper bound is below `below`: that bound and the
    /// witness. None where there is none to offer.
    fn witness(&self, c: &Self::Cell, below: f64) -> Option<(f64, Self::Witness)>;
    /// Whether the cell is at the resolution (not split further: undecided if not cleared).
    fn leaf(&self, c: &Self::Cell) -> bool;
    /// The cell in two halves that cover it.
    fn split(&self, c: &Self::Cell) -> (Self::Cell, Self::Cell);
}

/// The rule a search is decided by.
#[derive(Clone, Copy, Debug)]
pub struct Policy {
    /// The distance to prove: Clear needs every bound at least `limit + margin`.
    pub limit: f64,
    /// What the bounds may be off by beyond their own rounding (e.g. a lowered face's error):
    /// Clear needs `limit + margin`, Hit `limit - margin`.
    pub margin: f64,
    /// The most cells popped before the search gives up.
    pub budget: usize,
    /// None: the strict rule (CAM): a cell left at the resolution, or the budget spent, is
    /// Undecided. `Some(res)`: a stated resolution (the press brake's contract, docs/bendsim.md):
    /// the search may also end Clear when every bound left is at least `limit + margin - res`,
    /// checked the same way at every exit (the end, and the budget spent).
    pub resolution: Option<f64>,
}

/// The answer.
#[derive(Clone, Debug, PartialEq)]
pub enum Verdict<W> {
    /// Every point is at least `lower` away (proven; `lower >= limit`).
    Clear { lower: f64 },
    /// A real point at most `d` away (`d < limit`).
    Hit { d: f64, witness: W },
    /// Neither proven: `lower` bounds the distance from below (`-inf` when the budget ran out).
    Undecided { lower: f64, cells: usize },
}

/// A binary min-heap of cells by their lower bound (ties kept in push order as a binary heap
/// keeps them: the same order on every target).
struct Heap<C> {
    v: Vec<(f64, C, Ival)>,
}

impl<C> Heap<C> {
    #[inline]
    fn push(&mut self, sd: Ival, c: C) {
        let k = sd.lo;
        self.v.push((k, c, sd));
        let mut i = self.v.len() - 1;
        while i > 0 {
            let p = (i - 1) / 2;
            if self.v[p].0 <= self.v[i].0 {
                break;
            }
            self.v.swap(p, i);
            i = p;
        }
    }
    #[inline]
    fn pop(&mut self) -> Option<(f64, C, Ival)> {
        if self.v.is_empty() {
            return None;
        }
        let n = self.v.len() - 1;
        self.v.swap(0, n);
        let top = self.v.pop();
        let len = self.v.len();
        let mut i = 0;
        loop {
            let (l, r) = (2 * i + 1, 2 * i + 2);
            let mut m = i;
            if l < len && self.v[l].0 < self.v[m].0 {
                m = l;
            }
            if r < len && self.v[r].0 < self.v[m].0 {
                m = r;
            }
            if m == i {
                break;
            }
            self.v.swap(i, m);
            i = m;
        }
        top
    }
}

/// The search, least bound first, from `root`; the verdict and the cells popped.
pub fn certify<S: Search>(s: &S, root: S::Cell, p: &Policy) -> (Verdict<S::Witness>, usize) {
    let lim_clear = p.limit + p.margin;
    let lim_hit = p.limit - p.margin;
    let mut heap = Heap { v: Vec::new() };
    let b = s.bound(&root);
    heap.push(b, root);
    let mut lower = f64::INFINITY;
    let mut spent = 0usize;
    let mut undecided = false;
    while let Some((key, c, sd0)) = heap.pop() {
        if key >= lim_clear {
            // the least bound left clears: so does every other cell
            if key < lower {
                lower = key;
            }
            break;
        }
        spent += 1;
        if spent > p.budget {
            let Some(res) = p.resolution else {
                return (
                    Verdict::Undecided {
                        lower: f64::NEG_INFINITY,
                        cells: spent,
                    },
                    spent,
                );
            };
            // every cell left is at least this cell's bound (least first)
            if key < lower {
                lower = key;
            }
            return (within(lower, lim_clear, res, p.margin, spent), spent);
        }
        let h = s.holds(&c);
        if h == Holds::Out {
            continue;
        }
        let sd = s.refine(&c, h, sd0, lim_clear);
        if sd.lo >= lim_clear {
            if sd.lo < lower {
                lower = sd.lo;
            }
            continue;
        }
        if let Some((up, w)) = s.witness(&c, lim_hit) {
            return (
                Verdict::Hit {
                    d: up + p.margin,
                    witness: w,
                },
                spent,
            );
        }
        if s.leaf(&c) {
            undecided |= p.resolution.is_none();
            if sd.lo < lower {
                lower = sd.lo;
            }
            continue;
        }
        let (a, b) = s.split(&c);
        let (ba, bb) = (s.bound(&a), s.bound(&b));
        heap.push(ba, a);
        heap.push(bb, b);
    }
    if undecided {
        return (Verdict::Undecided { lower, cells: spent }, spent);
    }
    if let Some(res) = p.resolution {
        return (within(lower, lim_clear, res, p.margin, spent), spent);
    }
    (
        Verdict::Clear {
            lower: lower - p.margin,
        },
        spent,
    )
}

/// Under a stated resolution: Clear when `lower` is within it of the bound to prove, else
/// Undecided. The one check every exit makes.
fn within<W>(lower: f64, lim_clear: f64, res: f64, margin: f64, spent: usize) -> Verdict<W> {
    if lower >= lim_clear - res {
        Verdict::Clear {
            lower: lower - margin,
        }
    } else {
        Verdict::Undecided { lower, cells: spent }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The distance from points t in [0, 1] to a target at x: |t - x| over a cell [a, b],
    /// enclosed exactly; a toy search to check the rules.
    struct Line {
        x: f64,
        res: f64,
    }

    impl Search for Line {
        type Cell = (f64, f64);
        type Witness = f64;
        fn bound(&self, c: &(f64, f64)) -> Ival {
            let (a, b) = *c;
            let lo = if self.x < a {
                a - self.x
            } else if self.x > b {
                self.x - b
            } else {
                0.0
            };
            Ival::new(lo, (a - self.x).abs().max((b - self.x).abs()))
        }
        fn holds(&self, _c: &(f64, f64)) -> Holds {
            Holds::In
        }
        fn witness(&self, c: &(f64, f64), below: f64) -> Option<(f64, f64)> {
            let m = 0.5 * (c.0 + c.1);
            let d = (m - self.x).abs();
            (d < below).then_some((d, m))
        }
        fn leaf(&self, c: &(f64, f64)) -> bool {
            c.1 - c.0 <= self.res
        }
        fn split(&self, c: &(f64, f64)) -> ((f64, f64), (f64, f64)) {
            let m = 0.5 * (c.0 + c.1);
            ((c.0, m), (m, c.1))
        }
    }

    #[test]
    fn the_three_verdicts() {
        let p = |limit| Policy {
            limit,
            margin: 0.0,
            budget: 10_000,
            resolution: None,
        };
        // the target 2 away: clear of 1, proven
        let (v, _) = certify(&Line { x: 3.0, res: 1e-6 }, (0.0, 1.0), &p(1.0));
        assert_eq!(v, Verdict::Clear { lower: 2.0 });
        // the target inside: a hit, at a real point closer than the limit
        let (v, _) = certify(&Line { x: 0.3, res: 1e-6 }, (0.0, 1.0), &p(0.1));
        let Verdict::Hit { d, witness } = v else {
            panic!("{v:?}")
        };
        assert!(d < 0.1 && (witness - 0.3).abs() == d);
        // exactly at the limit: proven (the distance is at least the limit)
        let (v, _) = certify(&Line { x: 1.5, res: 1e-3 }, (0.0, 1.0), &p(0.5));
        assert_eq!(v, Verdict::Clear { lower: 0.5 });
        // just under the limit, closer than the resolution, and no midpoint below it: never a
        // Clear "within the resolution", but Undecided
        let (v, _) = certify(&Line { x: 1.4999, res: 1e-3 }, (0.0, 1.0), &p(0.5));
        assert!(matches!(v, Verdict::Undecided { .. }), "{v:?}");
        // a target no midpoint reaches, a limit below every split's reach: the budget is spent,
        // Undecided with no bound
        let (v, n) = certify(
            &Line {
                x: 1.0 / 3.0,
                res: 0.0,
            },
            (0.0, 1.0),
            &Policy {
                limit: 1e-300,
                margin: 0.0,
                budget: 5,
                resolution: None,
            },
        );
        assert_eq!(n, 6);
        assert_eq!(
            v,
            Verdict::Undecided {
                lower: f64::NEG_INFINITY,
                cells: 6
            }
        );
    }

    /// Under a stated resolution, a target just under the limit but within it is Clear (with
    /// its true lower bound reported), one further under is not.
    #[test]
    fn a_stated_resolution() {
        let p = |res| Policy {
            limit: 0.5,
            margin: 0.0,
            budget: 10_000,
            resolution: Some(res),
        };
        let (v, _) = certify(&Line { x: 1.4999, res: 1e-3 }, (0.0, 1.0), &p(1e-3));
        let Verdict::Clear { lower } = v else {
            panic!("{v:?}")
        };
        assert!(lower <= 0.4999 && lower >= 0.5 - 1e-3, "{lower}");
        // 0.49 away: beyond the resolution under the limit, so not Clear (a Hit here)
        let (v, _) = certify(&Line { x: 1.49, res: 1e-3 }, (0.0, 1.0), &p(1e-3));
        assert!(!matches!(v, Verdict::Clear { .. }), "{v:?}");
    }
}
