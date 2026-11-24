//! forge-bb: ForgeCAD's certified branch and bound. A clearance is proven over cells (of a
//! piece's parameters, and of the motion), each with a proven lower bound on the distance over it,
//! and a witness where one can be named: a real point with a proven upper bound.
//!
//! [`certify`] searches the cells least bound first and answers:
//! - **Clear**: every point is proven at least the limit away;
//! - **Near**: every point is proven at least `lower` away, with `limit − res ≤ lower < limit`,
//!   and no point was found under the limit: within the resolution, not past it (the owner's
//!   rule, 2026-11-24: a caller warns of it, it is not a pass);
//! - **Hit**: a real point proven under the limit;
//! - **Undecided**: neither; a caller refuses it.
//!
//! The soundness of the search, for any order of the cells and any evaluator meeting the
//! enclosure, witness and cover contracts, is verify/sheet `Sheet/BranchBound.lean`
//! (`search_sound`, `certify_sound`, `clear_past_limit`).
//!
//! Its first client is the press brake (forge-brake `bb.rs`, whose `Problem` is adapted to
//! [`Search`]). The search is the one forge-brake had, cell for cell; see kernel/forge-num
//! DESIGN.md for the clients and what each keeps.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// A cell search: its cells, their bounds, witnesses and splits.
pub trait Search {
    type Cell: Copy;
    type Witness: Copy;
    /// Whether the cell may hold a point of the piece (false only when it surely holds none: it
    /// is dropped).
    fn may_hold(&self, c: &Self::Cell) -> bool;
    /// A proven lower bound on the distance over the cell, and the size it rests on (the cell is
    /// a leaf, not split further, once that is at most half the resolution).
    fn bound(&self, c: &Self::Cell) -> (f64, f64);
    /// A real point of the piece in or near the cell: a proven upper bound on its distance, and
    /// the point. None if none can be named.
    fn witness(&self, c: &Self::Cell) -> Option<(f64, Self::Witness)>;
    /// The cell in two halves that cover it.
    fn split(&self, c: &Self::Cell) -> [Self::Cell; 2];
}

/// What a search is held to.
#[derive(Clone, Copy, Debug)]
pub struct Policy {
    /// The distance to prove.
    pub limit: f64,
    /// The resolution: a cell whose bound rests on a size at most half of it is a leaf.
    pub res: f64,
    /// The most cells popped.
    pub budget: usize,
    /// Whether to stop once the limit is proven, or go on to measure the least distance.
    pub measure: Measure,
    /// What a search that would end near does first.
    pub near: OnNear,
}

/// When a search may stop past the limit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Measure {
    /// As soon as the limit is proven (a decision: Clear, Near, Hit or Undecided).
    Decide,
    /// Go on past the limit until the least distance itself is known to within `res` (or a fifth
    /// of its margin over the limit), with a tenth of the budget; no finer than `cut`: a distance
    /// known to be at least `cut` is not measured further (another piece is already nearer;
    /// infinite for none).
    To { cut: f64 },
}

/// What a search that would end near does before it says so.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OnNear {
    /// Report it as found.
    Report,
    /// Press it further: a search its budget would end near goes on once with the budget again,
    /// from where it is (as a search with twice the budget would go, nothing searched again);
    /// one that ends near with its cells settled at the resolution is searched again `refine`
    /// times, each at an eighth of the last resolution. Only near searches pay.
    Press { refine: usize },
}

/// The answer. `upper` is at least a real point's distance (infinite when none was named).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Verdict<W> {
    /// Proven: every point at least `lower >= limit` away.
    Clear { lower: f64, upper: f64 },
    /// Proven: every point at least `lower` away, `limit - res <= lower < limit`; no point found
    /// under the limit.
    Near { lower: f64, upper: f64 },
    /// Proven: a real point at most `d < limit` away.
    Hit { d: f64, witness: W },
    /// Neither: `lower` is what was proven.
    Undecided { lower: f64 },
}

/// The press brake's `OnNear::Press { refine }`: twice, to a sixty-fourth of its resolution.
pub const REFINE: usize = 2;

#[derive(Clone, Copy)]
struct Item<C> {
    lb: f64,
    size: f64,
    cell: C,
}

impl<C> PartialEq for Item<C> {
    fn eq(&self, o: &Item<C>) -> bool {
        self.lb.total_cmp(&o.lb) == Ordering::Equal
    }
}
impl<C> Eq for Item<C> {}
impl<C> PartialOrd for Item<C> {
    fn partial_cmp(&self, o: &Item<C>) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl<C> Ord for Item<C> {
    fn cmp(&self, o: &Item<C>) -> Ordering {
        // the least bound first
        o.lb.total_cmp(&self.lb)
    }
}

/// The verdict of a search that found no point under the limit, from what it proved: clear only
/// past the limit, near within the resolution of it, else undecided (a leaf under the resolution
/// with no witness below the limit decides nothing).
#[inline]
fn settle<W>(lower: f64, upper: f64, limit: f64, res: f64) -> Verdict<W> {
    if lower >= limit {
        Verdict::Clear {
            lower,
            upper: upper.max(lower),
        }
    } else if lower >= limit - res {
        Verdict::Near {
            lower,
            upper: upper.max(lower),
        }
    } else {
        Verdict::Undecided { lower }
    }
}

/// One search of `roots`, the cells `spent` counted: under [`OnNear::Press`], a search its budget
/// would end near goes on once with the budget again (no finer search: that is [`certify`]'s).
/// `budget_ended`: set when the budget ended it.
pub fn search<S: Search>(
    s: &S,
    roots: &[S::Cell],
    p: &Policy,
    spent: &mut usize,
    budget_ended: &mut bool,
) -> Verdict<S::Witness> {
    let (limit, res) = (p.limit, p.res);
    let (measure, cut) = match p.measure {
        Measure::Decide => (false, f64::INFINITY),
        Measure::To { cut } => (true, cut),
    };
    let mut budget = p.budget;
    let mut extended = p.near == OnNear::Report;
    let mut heap = BinaryHeap::new();
    for c in roots {
        if s.may_hold(c) {
            let (lb, size) = s.bound(c);
            heap.push(Item { lb, size, cell: *c });
        }
    }
    let mut lower = f64::INFINITY;
    let mut upper = f64::INFINITY;
    while let Some(Item { lb, size, cell }) = heap.pop() {
        // the least bound left settles it: so does every other
        // measured to `res`, or to a fifth of the margin over the limit when that is more
        let tol = res.max(0.2 * (upper - limit));
        if (!measure && lb >= limit) || (measure && lb >= limit && lb >= upper.min(cut) - tol) {
            lower = lower.min(lb);
            break;
        }
        *spent += 1;
        // measuring past a proven pass gets a tenth of the budget; what is left is proven clear
        // of the limit, settled as far as it was measured
        if (measure && lb >= limit && *spent > budget / 10) || *spent > budget {
            let v = settle(lower.min(lb), upper, limit, res);
            if extended || !matches!(v, Verdict::Near { .. }) {
                *budget_ended = true;
                return v;
            }
            extended = true;
            budget *= 2;
        }
        if let Some((d, w)) = s.witness(&cell) {
            // a hit only if its proven upper bound is under the limit
            if d < limit {
                return Verdict::Hit { d, witness: w };
            }
            upper = upper.min(d);
        }
        if size <= 0.5 * res {
            lower = lower.min(lb);
            continue;
        }
        for half in s.split(&cell) {
            if s.may_hold(&half) {
                let (lb, size) = s.bound(&half);
                heap.push(Item { lb, size, cell: half });
            }
        }
    }
    // every cell is settled
    settle(lower, upper, limit, res)
}

/// The search under its policy, a near verdict pressed further as [`OnNear`] says: Clear, Hit, or
/// the best Near proven; and the cells spent.
pub fn certify<S: Search>(s: &S, roots: &[S::Cell], p: &Policy) -> (Verdict<S::Witness>, usize) {
    let mut spent = 0usize;
    let mut ended = false;
    let mut v = search(s, roots, p, &mut spent, &mut ended);
    let refine = match p.near {
        OnNear::Report => 0,
        OnNear::Press { refine } => refine,
    };
    // (not one the budget ended, twice: a finer search proves no more there)
    if let Verdict::Near { lower: mut best, .. } = v
        && !ended
    {
        let mut q = *p;
        for _ in 0..refine {
            q.res *= 0.125;
            // its own budget: the first search may have spent all of its own
            let mut more = 0usize;
            let w = search(s, roots, &q, &mut more, &mut ended);
            spent += more;
            match w {
                // finer, the budget ran out: what was proven stands
                Verdict::Undecided { .. } => break,
                Verdict::Near { lower, .. } => {
                    if lower > best {
                        best = lower;
                        v = w;
                    }
                }
                w => {
                    v = w;
                    break;
                }
            }
        }
    }
    (v, spent)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Points t in [0, 1] against a target at x: the distance |t - x| over a cell [a, b], bounded
    /// loosely (the midpoint's distance less the width, as a coarse enclosure would), witnessed at
    /// the midpoint when `named`.
    struct Line {
        x: f64,
        named: bool,
    }

    impl Search for Line {
        type Cell = (f64, f64);
        type Witness = f64;
        fn may_hold(&self, _: &(f64, f64)) -> bool {
            true
        }
        fn bound(&self, c: &(f64, f64)) -> (f64, f64) {
            let m = 0.5 * (c.0 + c.1);
            let hd = 0.5 * (c.1 - c.0);
            ((m - self.x).abs() - 2.0 * hd, 2.0 * hd)
        }
        fn witness(&self, c: &(f64, f64)) -> Option<(f64, f64)> {
            let m = 0.5 * (c.0 + c.1);
            self.named.then_some(((m - self.x).abs(), m))
        }
        fn split(&self, c: &(f64, f64)) -> [(f64, f64); 2] {
            let m = 0.5 * (c.0 + c.1);
            [(c.0, m), (m, c.1)]
        }
    }

    fn p(limit: f64, res: f64, budget: usize, measure: bool) -> Policy {
        Policy {
            limit,
            res,
            budget,
            measure: if measure {
                Measure::To { cut: f64::INFINITY }
            } else {
                Measure::Decide
            },
            near: OnNear::Press { refine: REFINE },
        }
    }

    #[test]
    fn the_four_verdicts() {
        let far = Line { x: 3.0, named: true };
        assert!(
            matches!(certify(&far, &[(0.0, 1.0)], &p(1.0, 1e-3, 10_000, false)).0, Verdict::Clear { lower, .. } if lower >= 1.0)
        );
        let inside = Line { x: 0.3, named: true };
        assert!(
            matches!(certify(&inside, &[(0.0, 1.0)], &p(0.1, 1e-3, 10_000, false)).0, Verdict::Hit { d, .. } if d < 0.1)
        );
        // 0.0001 under the limit, nothing named: near, at every resolution
        let blind = Line {
            x: 1.4999,
            named: false,
        };
        match certify(&blind, &[(0.0, 1.0)], &p(0.5, 1e-3, 1_000_000, false)).0 {
            Verdict::Near { lower, .. } => assert!((0.5 - 1e-3..0.5).contains(&lower), "{lower}"),
            v => panic!("{v:?}"),
        }
        // a budget too small to prove anything near: undecided
        let (v, n) = certify(&blind, &[(0.0, 1.0)], &p(0.5, 1e-9, 3, false));
        assert!(matches!(v, Verdict::Undecided { .. }), "{v:?}");
        assert!(n >= 4);
    }

    #[test]
    fn clear_is_past_the_limit_always() {
        for x in [1.4, 1.49, 1.499, 1.5, 1.5001, 1.6, 2.0] {
            for res in [1e-1, 1e-2, 1e-4] {
                for budget in [5, 50, 100_000] {
                    for measure in [false, true] {
                        let l = Line { x, named: false };
                        if let (Verdict::Clear { lower, .. }, _) =
                            certify(&l, &[(0.0, 1.0)], &p(0.5, res, budget, measure))
                        {
                            assert!(lower >= 0.5, "{x} {res} {budget} {measure}: {lower}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_search_the_budget_ends_near_goes_on_once() {
        // budgets the search runs out of while near: it goes on as a search with twice the budget
        // would, and some of those come out clear
        let run = |budget: usize, measure: bool, extend: bool| {
            let (mut n, mut ended) = (0, false);
            let l = Line {
                x: 1.53,
                named: false,
            };
            let mut q = p(0.5, 0.05, budget, measure);
            if !extend {
                q.near = OnNear::Report;
            }
            let v = search(&l, &[(0.0, 1.0)], &q, &mut n, &mut ended);
            (v, n, ended)
        };
        let (mut went_on, mut cleared) = (0, 0);
        for budget in 1..200 {
            for measure in [false, true] {
                let (w, _, ended) = run(budget, measure, false);
                let v = run(budget, measure, true);
                if matches!(w, Verdict::Near { .. }) && ended {
                    assert_eq!(v, run(2 * budget, measure, false), "{budget} {measure}");
                    went_on += 1;
                    cleared += matches!(v.0, Verdict::Clear { .. }) as usize;
                } else {
                    assert_eq!(v.0, w, "{budget} {measure}");
                }
            }
        }
        assert!(went_on > 0 && cleared > 0, "{went_on} {cleared}");
    }
}
