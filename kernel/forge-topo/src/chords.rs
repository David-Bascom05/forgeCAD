//! Which chords of a drawing's loops can meet: every pair of chords whose boxes overlap,
//! found without comparing every pair (the one enumeration behind `uv::loops_cross` and the
//! tessellator's crossing pre-pass).
//!
//! The chords are cut into chains monotone in the order of x then y, by exact comparisons of
//! their ends. A chord lies between its ends in that order, so two chords of one chain that
//! share no end lie in disjoint intervals of it and cannot meet. Chains whose boxes overlap are
//! walked together, chord by chord in that order, from where each reaches the other's box:
//! every pair of their chords whose intervals overlap is visited (Shamos and Hoey's monotone
//! pieces). So every pair of chords that meet, other than two neighbours of one chain, is
//! visited, and the caller decides each exactly.

use forge_math::Point2;

/// A chain: its box and where its points start in the chain points (`len + 1` of them).
struct Chain {
    x: (f64, f64),
    y: (f64, f64),
    at: usize,
    len: usize,
}

/// The chain being drawn: its first point's place, whether it runs up, its box.
type Open = Option<(usize, bool, (f64, f64), (f64, f64))>;

fn before(p: Point2, q: Point2) -> bool {
    p.x < q.x || (p.x == q.x && p.y < q.y)
}

/// The order of keys of `x`: an integer comparison for each float's.
fn key(x: f64) -> u64 {
    let b = x.to_bits();
    if b >> 63 == 1 { !b } else { b | 1 << 63 }
}

/// Calls `zero(a, b)` for each chord of no length (its ends one place), and `pair(s, t)` for
/// each pair of chords `s`, `t` (each as its two point indices, low end first in that order)
/// whose closed boxes overlap, except two neighbours of one chain; an `Err` stops the walk.
/// `loops` are cyclic sequences of indices into `pts`.
pub fn near_pairs<L: AsRef<[u32]>, E>(
    pts: &[Point2],
    loops: &[L],
    mut zero: impl FnMut(u32, u32),
    mut pair: impl FnMut((u32, u32), (u32, u32)) -> Result<(), E>,
) -> Result<(), E> {
    let total: usize = loops.iter().map(|l| l.as_ref().len()).sum();
    // each chain's points, in the order of x then y
    let mut cp: Vec<u32> = Vec::with_capacity(total + loops.len());
    let mut chains: Vec<Chain> = Vec::new();
    for lp in loops {
        let lp = lp.as_ref();
        let n = lp.len();
        // (this chain: its first point's place in cp, whether it runs up, its box)
        let mut open: Open = None;
        let close = |open: &mut Open, cp: &mut Vec<u32>, chains: &mut Vec<Chain>| {
            if let Some((at, up, x, y)) = open.take() {
                if !up {
                    cp[at..].reverse();
                }
                chains.push(Chain {
                    x,
                    y,
                    at,
                    len: cp.len() - at - 1,
                });
            }
        };
        for k in 0..n {
            let (a, b) = (lp[k], lp[(k + 1) % n]);
            let (p, r) = (pts[a as usize], pts[b as usize]);
            if p == r {
                zero(a, b);
                continue;
            }
            let up = before(p, r);
            match &mut open {
                Some((_, u, x, y)) if *u == up && cp.last() == Some(&a) => {
                    *x = (x.0.min(r.x), x.1.max(r.x));
                    *y = (y.0.min(r.y), y.1.max(r.y));
                    cp.push(b);
                }
                _ => {
                    close(&mut open, &mut cp, &mut chains);
                    open = Some((
                        cp.len(),
                        up,
                        (p.x.min(r.x), p.x.max(r.x)),
                        (p.y.min(r.y), p.y.max(r.y)),
                    ));
                    cp.push(a);
                    cp.push(b);
                }
            }
        }
        close(&mut open, &mut cp, &mut chains);
    }
    let mut order: Vec<(u64, u32)> = (0..chains.len())
        .map(|i| (key(chains[i].x.0), i as u32))
        .collect();
    order.sort_unstable();
    let chord = |c: &Chain, k: usize| (cp[c.at + k], cp[c.at + k + 1]);
    // the first chord of `c` whose high end reaches x
    let first = |c: &Chain, x: f64| {
        let (mut l, mut h) = (0, c.len);
        while l < h {
            let m = (l + h) / 2;
            if pts[cp[c.at + m + 1] as usize].x < x {
                l = m + 1;
            } else {
                h = m;
            }
        }
        l
    };
    for n in 0..order.len() {
        let ci = &chains[order[n].1 as usize];
        for &(_, j) in &order[n + 1..] {
            let cj = &chains[j as usize];
            if cj.x.0 > ci.x.1 {
                break;
            }
            if ci.y.1 < cj.y.0 || cj.y.1 < ci.y.0 {
                continue;
            }
            let (mut p, mut q) = (first(ci, cj.x.0), first(cj, ci.x.0));
            while p < ci.len && q < cj.len {
                let (s, t) = (chord(ci, p), chord(cj, q));
                let (ls, hs) = (pts[s.0 as usize], pts[s.1 as usize]);
                let (lt, ht) = (pts[t.0 as usize], pts[t.1 as usize]);
                if ls.x > cj.x.1 || lt.x > ci.x.1 {
                    break;
                }
                if ls.x <= ht.x
                    && lt.x <= hs.x
                    && ls.y.min(hs.y) <= lt.y.max(ht.y)
                    && lt.y.min(ht.y) <= ls.y.max(hs.y)
                {
                    pair(s, t)?;
                }
                if before(hs, ht) {
                    p += 1;
                } else if before(ht, hs) {
                    q += 1;
                } else {
                    // both end at one place: the next of each starts there, and meets the other
                    if p + 1 < ci.len {
                        let u = chord(ci, p + 1);
                        let (lu, hu) = (pts[u.0 as usize], pts[u.1 as usize]);
                        if lu.x <= ht.x
                            && lt.x <= hu.x
                            && lu.y.min(hu.y) <= lt.y.max(ht.y)
                            && lt.y.min(ht.y) <= lu.y.max(hu.y)
                        {
                            pair(u, t)?;
                        }
                    }
                    q += 1;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::rng::Rng;

    /// Every pair of chords whose boxes overlap and that meet (exactly, by orient2d), other than
    /// two neighbours of one loop, is visited: against all pairs, on random loops.
    #[test]
    fn every_meeting_pair_is_visited() {
        use forge_math::predicates::orient2d;
        let sign = |x: f64| (x > 0.0) as i8 - (x < 0.0) as i8;
        let meet = |a: Point2, b: Point2, c: Point2, d: Point2| {
            let (o1, o2) = (sign(orient2d(a, b, c)), sign(orient2d(a, b, d)));
            let (o3, o4) = (sign(orient2d(c, d, a)), sign(orient2d(c, d, b)));
            !(o1 * o2 > 0 || o3 * o4 > 0)
                && (o1 != 0 || o2 != 0 || {
                    let s = |p: Point2| if a.x != b.x { p.x } else { p.y };
                    s(c).max(s(d)) >= s(a).min(s(b)) && s(c).min(s(d)) <= s(a).max(s(b))
                })
        };
        let mut r = Rng::new(11);
        for round in 0..400 {
            // points on a small grid (ties of x and y, verticals, collinear runs) or anywhere
            let grid = round % 2 == 0;
            let n = 3 + r.below(40);
            let pts: Vec<Point2> = (0..n)
                .map(|_| {
                    if grid {
                        Point2::new(r.below(6) as f64, r.below(6) as f64)
                    } else {
                        Point2::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0))
                    }
                })
                .collect();
            let cut = 1 + r.below(n - 1);
            let loops: Vec<Vec<u32>> = vec![(0..cut as u32).collect(), (cut as u32..n as u32).collect()];
            let chords: Vec<(u32, u32)> = loops
                .iter()
                .flat_map(|l| (0..l.len()).map(move |k| (l[k], l[(k + 1) % l.len()])))
                .filter(|&(a, b)| pts[a as usize] != pts[b as usize])
                .collect();
            let mut seen = std::collections::BTreeSet::new();
            let norm = |s: (u32, u32)| (s.0.min(s.1), s.0.max(s.1));
            near_pairs::<_, ()>(
                &pts,
                &loops,
                |_, _| {},
                |s, t| {
                    let (s, t) = (norm(s), norm(t));
                    seen.insert((s.min(t), s.max(t)));
                    Ok(())
                },
            )
            .unwrap();
            for i in 0..chords.len() {
                for j in i + 1..chords.len() {
                    let ((a, b), (c, d)) = (chords[i], chords[j]);
                    if a == c || a == d || b == c || b == d {
                        continue;
                    }
                    let p = |k: u32| pts[k as usize];
                    if meet(p(a), p(b), p(c), p(d)) {
                        let (s, t) = (norm(chords[i]), norm(chords[j]));
                        assert!(
                            seen.contains(&(s.min(t), s.max(t))),
                            "round {round}: {:?} {:?} meet, not visited",
                            chords[i],
                            chords[j]
                        );
                    }
                }
            }
        }
    }
}
