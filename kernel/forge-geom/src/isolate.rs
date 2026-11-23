//! The roots of a function g of one variable on an interval, each certified alone in a box, or
//! a refusal naming the box it could not decide. `f` gives g's enclosure and g′'s over a box,
//! both rounded outward, each holding every value at every point of the box.
//!
//! A box goes, in order:
//! - excluded where 0 is in neither the natural enclosure g(X) nor the mean-value form
//!   g(m) + g′(X)(X − m), m the box's middle (both hold g's range, so each excludes alone);
//! - certified by `krawczyk1` on the box grown by a quarter of its width each way, Xg, so a root
//!   on a split point or an end is not missed: then g has exactly one zero in Xg, and it is in K.
//!   Where K misses Xg, Xg has none;
//! - else split at its middle.
//!
//! A root is reported once: two certified boxes K1 (in Xg1) and K2 (in Xg2) hold one root where
//! K1 ⊂ Xg2 or K2 ⊂ Xg1 (that root is in the other's grown box, whose zero is the only one there);
//! they are kept as their intersection. A root's box may reach a little past the domain's ends
//! (the growth): the caller decides there. A clearance g > h is the caller's: pass g − h.
use forge_math::Interval;

/// What `isolate` found on the domain.
#[derive(Clone, Debug, PartialEq)]
pub enum Isolated {
    /// Each holds exactly one simple root (1D Krawczyk), in increasing order; at least one.
    Roots(Vec<Interval>),
    /// g has no root in the domain: every box excluded.
    Clear,
    /// Stopped undecided at `at`.
    Refused { at: Interval, why: Refusal1 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Refusal1 {
    /// More than `budget` boxes.
    Budget,
    /// A box at the least width neither excluded nor certified: 0 ∈ g there (a double root, a
    /// tangency, or roots closer than the width).
    Tangency,
}

/// The Krawczyk operator K = m − c g(m) + (1 − c g′(X))(X − m), c = 1 / mid g′(X), with
/// |1 − c g′(X)|'s greatest value; None where c is not defined (0 ∈ g′(X)).
fn krawczyk_k(x: Interval, g_mid: Interval, dg: Interval) -> Option<(Interval, f64)> {
    let c = dg.mid();
    if dg.contains_zero() || !c.is_finite() || c == 0.0 {
        return None;
    }
    let (m, c) = (Interval::point(x.mid()), Interval::point(1.0 / c));
    let q = Interval::point(1.0) - c * dg;
    Some((m - c * g_mid + q * (x - m), q.lo.abs().max(q.hi.abs())))
}

/// The 1D Krawczyk test: g has exactly one zero in `x` when K ⊂ int X and |1 − c g′(X)| < 1
/// (verify/krawczyk's unique_zero and maps_into, n = 1, the row sum its one entry). `g_mid` is
/// g's enclosure at x's middle, `dg` g′'s over x.
pub fn krawczyk1(x: Interval, g_mid: Interval, dg: Interval) -> bool {
    krawczyk_k(x, g_mid, dg).is_some_and(|(k, q)| k.lo > x.lo && k.hi < x.hi && q < 1.0)
}

fn meet(a: Interval, b: Interval) -> Interval {
    Interval {
        lo: a.lo.max(b.lo),
        hi: a.hi.min(b.hi),
    }
}

/// g's zeros on `domain` (see the module), splitting no box narrower than `least`, looking at
/// most at `budget` boxes.
pub fn isolate(
    domain: Interval,
    f: impl Fn(Interval) -> (Interval, Interval),
    least: f64,
    budget: usize,
) -> Isolated {
    // (root box K, the grown box it is alone in)
    let mut found: Vec<(Interval, Interval)> = Vec::new();
    let mut stack = vec![domain];
    let mut spent = 0;
    while let Some(x) = stack.pop() {
        if x.is_empty() {
            continue;
        }
        spent += 1;
        if spent > budget {
            return Isolated::Refused {
                at: x,
                why: Refusal1::Budget,
            };
        }
        let m = x.mid();
        let (gm, _) = f(Interval::point(m));
        let (gx, dx) = f(x);
        let mean = gm + dx * (x - Interval::point(m));
        if !gx.contains_zero() || !mean.contains_zero() {
            continue;
        }
        let g = x.grow(0.25 * x.width());
        let (_, dg) = f(g);
        // K's centre is g's, which is x's
        if let Some((k, q)) = krawczyk_k(g, gm, dg) {
            if k.hi < g.lo || k.lo > g.hi {
                continue; // no zero in g, so none in x
            }
            if k.lo > g.lo && k.hi < g.hi && q < 1.0 {
                if k.overlaps(&x) {
                    let same = found.iter().position(|(k2, g2)| {
                        (k.lo >= g2.lo && k.hi <= g2.hi) || (k2.lo >= g.lo && k2.hi <= g.hi)
                    });
                    match same {
                        Some(i) => found[i].0 = meet(found[i].0, k),
                        None => found.push((k, g)),
                    }
                }
                continue;
            }
        }
        let (a, b) = x.split();
        if x.width() <= least || m <= x.lo || m >= x.hi {
            return Isolated::Refused {
                at: x,
                why: Refusal1::Tangency,
            };
        }
        stack.push(b);
        stack.push(a);
    }
    if found.is_empty() {
        return Isolated::Clear;
    }
    let mut roots: Vec<Interval> = found.into_iter().map(|(k, _)| k).collect();
    roots.sort_by(|a, b| a.lo.total_cmp(&b.lo));
    Isolated::Roots(roots)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(v: f64) -> Interval {
        Interval::point(v)
    }

    #[test]
    fn simple_roots_are_found_once_each() {
        // (t - 0.5)(t - 0.25)(t + 0.7) on [-1, 1]: three roots, one on the first split point
        let f = |t: Interval| {
            let g = (t - p(0.5)) * (t - p(0.25)) * (t + p(0.7));
            let d = (t - p(0.25)) * (t + p(0.7)) + (t - p(0.5)) * (t + p(0.7)) + (t - p(0.5)) * (t - p(0.25));
            (g, d)
        };
        let Isolated::Roots(r) = isolate(Interval::new(-1.0, 1.0), f, 1e-12, 10_000) else {
            panic!()
        };
        assert_eq!(r.len(), 3, "{r:?}");
        for (k, want) in r.iter().zip([-0.7, 0.25, 0.5]) {
            assert!(k.contains(want) && k.width() < 0.5, "{k:?}");
        }
        // a root at the domain's end
        let Isolated::Roots(r) = isolate(Interval::new(0.0, 1.0), |t| (t - p(1.0), p(1.0)), 1e-12, 1000)
        else {
            panic!()
        };
        assert_eq!(r.len(), 1);
        assert!(r[0].contains(1.0));
        // none
        let h = |t: Interval| (t.sqr() + p(1.0), p(2.0) * t);
        assert_eq!(isolate(Interval::new(-3.0, 3.0), h, 1e-12, 1000), Isolated::Clear);
        assert!(krawczyk1(Interval::new(0.9, 1.1), p(-0.0), p(1.0)));
        assert!(!krawczyk1(Interval::new(1.5, 2.0), p(0.75), p(1.0)));
    }

    #[test]
    fn a_tangency_is_refused_with_its_box() {
        // t² touches 0 at 0: no box certifies it, and no exclusion holds near it
        let f = |t: Interval| (t.sqr(), p(2.0) * t);
        let r = isolate(Interval::new(-1.0, 3.0), f, 1e-9, 100_000);
        let Isolated::Refused { at, why } = r else {
            panic!("{r:?}")
        };
        assert_eq!(why, Refusal1::Tangency);
        assert!(at.width() <= 1e-9 && at.lo <= 1e-9 && at.hi >= -1e-9, "{at:?}");
        // and with too small a budget, refused for the budget
        let r = isolate(Interval::new(-1.0, 3.0), f, 1e-9, 10);
        assert!(
            matches!(
                r,
                Isolated::Refused {
                    why: Refusal1::Budget,
                    ..
                }
            ),
            "{r:?}"
        );
    }
}
