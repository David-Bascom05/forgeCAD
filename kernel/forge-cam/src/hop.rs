//! Hops: a retract between two cuts (up, across at a safe height, down: four or five blocks,
//! each starting and stopping) replaced by one up-over-down move at the link feed, as low as the
//! material it crosses allows.
//!
//! The path is walked in order on a height map of the op's starting stock, every move swept into
//! it (the stock as the op leaves it at that point). A run of links between two cuts that rises
//! and comes back down (rapids, or lines in the link phase) becomes: up (a ramp where the
//! material beside the start allows one, else straight up), across at the crossing height, down
//! (likewise) to the same end, every move a G1 at the link feed. The tip is, at every sample of
//! the way (a map cell apart), at least `CLEAR` above the highest material the map holds within
//! r + h of it: the disk test every link at depth passes (`adaptive`'s `link_clear`), at
//! height, the r + h band covering the map's cells (each a sample at its centre). The map holds
//! at least the material left: it starts from the in-process stock (the voids earlier ops
//! proved, or the uncut stock when the caller cannot trust them) and is lowered only by the
//! moves swept here. A hop never rises above the run it replaces, and lands only where the
//! tool's disk at the end is clear of material above it (a start in cleared air): a run that
//! ends by entering material keeps its own descent.
//!
//! Every hop is a G1 (a defined path: a control's G0 may dog-leg, each axis at its own rapid)
//! and is checked like any posted move afterwards (the certificate's bodies, the feed
//! scheduler's contact rule: a hop that met material would be fed at the chip rule).
use crate::engage::{HeightMap, Profile};
use crate::ops::{PMove, Phase};
use forge_math::dmath;

/// The tip's clearance over the material it crosses (mm).
pub const CLEAR: f64 = 0.5;

/// `moves` from `start` with every retract between cuts that a hop can replace replaced, the
/// map left as the path leaves the stock. `feed`: the link feed. `bodies`: the tool's bodies
/// above the cutter (shank, holder), each as (its lowest point above the tip, its largest
/// radius). The number of hops made.
pub fn hops(
    moves: &[PMove],
    start: [f64; 3],
    map: &mut HeightMap,
    prof: &Profile,
    bodies: &[(f64, f64)],
    feed: f64,
) -> (Vec<PMove>, usize) {
    // canned cycles and probe moves: not walked here (left as they are)
    if moves
        .iter()
        .any(|m| matches!(m, PMove::Cycle { .. }))
    {
        return (moves.to_vec(), 0);
    }
    let r = prof.r();
    // r + 2h: a point between two samples (h apart) is within h/2 of one, and a cell's centre
    // stands within h/√2 of any point of it: 1.21 h in all, under 2 h
    let rr = r + 2.0 * map.h;
    let mut out: Vec<PMove> = Vec::with_capacity(moves.len());
    let mut at = start;
    let mut n_hops = 0;
    let is_link = |m: &PMove| {
        matches!(
            m,
            PMove::Rapid { .. }
                | PMove::Line {
                    phase: Phase::Link,
                    ..
                }
        )
    };
    let mut i = 0;
    while i < moves.len() {
        // a run of links between two moves that are not, after something was cut
        if is_link(&moves[i]) && i > 0 && !is_link(&moves[i - 1]) {
            let mut j = i;
            let mut top = at[2];
            let mut b = at;
            while j < moves.len() && is_link(&moves[j]) {
                b = end(&moves[j]);
                top = top.max(b[2]);
                j += 1;
            }
            // a run that ends above a plunge straight down (an entry into material): the hop
            // lands just over the material the map holds there, and the plunge goes on from
            // there (its feed only through the stock, not through the air above it)
            if let Some(PMove::Line {
                to: p,
                f: fp,
                phase: Phase::Plunge,
            }) = moves.get(j).copied()
                && p[0] == b[0]
                && p[1] == b[1]
                && p[2] < b[2]
                && std::env::var_os("FORGE_HOP_ENTRY").is_some()
            {
                let zt = (top_near(map, b[0], b[1], rr) + CLEAR).max(p[2]);
                let land = [b[0], b[1], zt];
                if zt < b[2] - 1e-9
                    && top > at[2] + 1e-9
                    && let Some(h) = hop(map, rr, bodies, at, land, top, feed)
                {
                    let plunge = PMove::Line {
                        to: p,
                        f: fp,
                        phase: Phase::Plunge,
                    };
                    for m in h.iter().chain(std::iter::once(&plunge)) {
                        sweep(map, prof, at, m);
                        at = end(m);
                    }
                    out.extend(h);
                    out.push(plunge);
                    n_hops += 1;
                    i = j + 1;
                    continue;
                }
            }
            if j < moves.len()
                && top > at[2].max(b[2]) + 1e-9
                && let Some(h) = hop(map, rr, bodies, at, b, top, feed)
            {
                for m in &h {
                    sweep(map, prof, at, m);
                    at = end(m);
                }
                out.extend(h);
                n_hops += 1;
                i = j;
                continue;
            }
        }
        sweep(map, prof, at, &moves[i]);
        at = end(&moves[i]);
        out.push(moves[i]);
        i += 1;
    }
    (out, n_hops)
}

/// The highest material on the map's cells meeting the box [lo, hi] (−∞: none).
fn box_top(map: &HeightMap, lo: [f64; 2], hi: [f64; 2]) -> f64 {
    let cell = |v: f64, o: f64| ((v - o) / map.h).floor();
    let i0 = cell(lo[0], map.x0).max(0.0) as usize;
    let j0 = cell(lo[1], map.y0).max(0.0) as usize;
    let i1 = cell(hi[0], map.x0);
    let j1 = cell(hi[1], map.y0);
    if i1 < 0.0 || j1 < 0.0 {
        return f64::NEG_INFINITY;
    }
    let (i1, j1) = ((i1 as usize).min(map.nx - 1), (j1 as usize).min(map.ny - 1));
    let mut t = f64::NEG_INFINITY;
    for j in j0..=j1 {
        for i in i0..=i1 {
            t = t.max(map.z[j * map.nx + i]);
        }
    }
    t
}

/// The highest material within rr of (x, y) on the map (−∞: none).
fn top_near(map: &HeightMap, x: f64, y: f64, rr: f64) -> f64 {
    let h = map.h;
    let n = (rr / h).ceil() as i64;
    let mut t = f64::NEG_INFINITY;
    for j in -n..=n {
        for i in -n..=n {
            let (dx, dy) = (i as f64 * h, j as f64 * h);
            if dx * dx + dy * dy <= rr * rr {
                t = t.max(map.top(x + dx, y + dy));
            }
        }
    }
    t
}

/// The hop from a to b (no higher than `ceiling`), or None: the end's disk meets material above
/// it, or the way needs more height than the run it replaces had.
fn hop(
    map: &HeightMap,
    rr: f64,
    bodies: &[(f64, f64)],
    a: [f64; 3],
    b: [f64; 3],
    ceiling: f64,
    feed: f64,
) -> Option<Vec<PMove>> {
    if top_near(map, b[0], b[1], rr) > b[2] + 1e-9 {
        return None;
    }
    let l = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
    let n = (l / map.h).ceil().max(1.0) as usize;
    let p = |k: usize| {
        let t = k as f64 / n as f64;
        [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]
    };
    // the tip's least height at each sample
    // the bodies above the cutter (wider: a holder): the tip high enough that each body's lowest
    // point clears by CLEAR the highest material in the box round the way, widened by its radius
    // and 2h (a bound on the material within that radius of any point of the way)
    let floor = bodies
        .iter()
        .map(|&(z0, rb)| {
            let w = rb + 2.0 * map.h;
            box_top(
                map,
                [a[0].min(b[0]) - w, a[1].min(b[1]) - w],
                [a[0].max(b[0]) + w, a[1].max(b[1]) + w],
            ) + CLEAR
                - z0
        })
        .fold(f64::NEG_INFINITY, f64::max);
    let need: Vec<f64> = (0..=n)
        .map(|k| {
            let q = p(k);
            (top_near(map, q[0], q[1], rr) + CLEAR).max(floor)
        })
        .collect();
    let hc = need.iter().copied().fold(a[2].max(b[2]), f64::max);
    if hc > ceiling + 1e-9 {
        return None;
    }
    // the ramp from (0, z0) to (k, hc) farthest along that stays above the need, at most half
    // the way (from the end: the same, reversed)
    let ramp = |z0: f64, need_at: &dyn Fn(usize) -> f64| -> usize {
        let mut best = 0;
        // (the end itself is exempt: the tool stands there, beside the wall it cut, or lands
        // there on a clear disk; from the next sample on the line clears the need)
        for k in 1..=n / 2 {
            if (1..=k).all(|i| z0 + (hc - z0) * (i as f64 / k as f64) >= need_at(i)) {
                best = k;
            }
        }
        best
    };
    // an end ramps only where no material within rr of it stands above its level (else the tool
    // stands beside a wall and goes straight up from it, or down onto it): then a point between
    // two samples is within h of both, the material near it counted at both, and the line there
    // no lower than either sample
    let ku = if top_near(map, a[0], a[1], rr) <= a[2] + 1e-9 {
        ramp(a[2], &|i| need[i])
    } else {
        0
    };
    let kd = ramp(b[2], &|i| need[n - i]);
    let line = |q: [f64; 2], z: f64| PMove::Line {
        to: [q[0], q[1], z],
        f: feed,
        phase: Phase::Link,
    };
    let mut h = vec![line(p(ku), hc)];
    if n - kd > ku {
        h.push(line(p(n - kd), hc));
    }
    h.push(PMove::Line {
        to: b,
        f: feed,
        phase: Phase::Link,
    });
    Some(h)
}

fn end(m: &PMove) -> [f64; 3] {
    match *m {
        PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } => to,
        // (not walked: `hops` leaves a path with a cycle as it is)
        PMove::Cycle { .. } => [f64::NAN; 3],
    }
}

/// The map lowered by the move from `at` (an arc as chords no longer than a cell).
fn sweep(map: &mut HeightMap, prof: &Profile, at: [f64; 3], m: &PMove) {
    match *m {
        PMove::Rapid { to } | PMove::Line { to, .. } => {
            map.sweep(at, to, prof);
        }
        PMove::Arc { to, c, cw, .. } => {
            let (a0, a1) = (
                dmath::atan2(at[1] - c[1], at[0] - c[0]),
                dmath::atan2(to[1] - c[1], to[0] - c[0]),
            );
            let tau = std::f64::consts::TAU;
            let mut sw = a1 - a0;
            if cw {
                while sw >= 0.0 {
                    sw -= tau;
                }
            } else {
                while sw <= 0.0 {
                    sw += tau;
                }
            }
            let rad = ((at[0] - c[0]).powi(2) + (at[1] - c[1]).powi(2)).sqrt();
            let k = ((sw.abs() * rad / map.h).ceil() as usize).max(1);
            let mut p = at;
            for i in 1..=k {
                let t = i as f64 / k as f64;
                let (sn, cs) = dmath::sin_cos(a0 + t * sw);
                let q = [c[0] + rad * cs, c[1] + rad * sn, at[2] + t * (to[2] - at[2])];
                map.sweep(p, q, prof);
                p = q;
            }
        }
        PMove::Cycle { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: f64 = 3.0;

    /// A 60 × 20 block 10 deep, top at 0, with two pockets 5 deep cut at its ends, the material
    /// between them standing at 0.
    fn stock() -> (HeightMap, Profile) {
        let prof = Profile::Bull { r: R, re: 0.0 };
        let mut m = HeightMap::new([0.0, 0.0, -10.0], [60.0, 20.0, 0.0], 0.25);
        for x in [[4.0, 16.0], [44.0, 56.0]] {
            for y in [6.0, 10.0, 14.0] {
                m.sweep([x[0], y, -5.0], [x[1], y, -5.0], &prof);
            }
        }
        (m, prof)
    }

    fn line(to: [f64; 3], phase: Phase) -> PMove {
        PMove::Line { to, f: 500.0, phase }
    }

    /// A cut in the left pocket, a retract to 5 over the material, down into the right pocket
    /// (or, `into`, onto a point of the material), a cut there.
    fn path(into: [f64; 3]) -> (Vec<PMove>, [f64; 3]) {
        let start = [6.0, 10.0, -5.0];
        let mv = vec![
            line([14.0, 10.0, -5.0], Phase::Cut),
            PMove::Rapid {
                to: [14.0, 10.0, 5.0],
            },
            PMove::Rapid {
                to: [into[0], into[1], 5.0],
            },
            PMove::Rapid { to: into },
            line([into[0] + 4.0, into[1], into[2]], Phase::Cut),
        ];
        (mv, start)
    }

    /// Replays `out` on a fresh map, checking every hop: at each sample (a cell apart) the tip
    /// is CLEAR above the material within r + h, it lands where the disk has no material above
    /// it, and it rises no higher than `ceiling`.
    fn check(out: &[PMove], start: [f64; 3], ceiling: f64) -> usize {
        let (mut m, prof) = stock();
        let rr = R + 2.0 * m.h;
        let mut at = start;
        let mut hops = 0;
        let mut a0 = start;
        for (i, mv) in out.iter().enumerate() {
            let first = !matches!(
                i.checked_sub(1).and_then(|j| out.get(j)),
                Some(PMove::Line {
                    phase: Phase::Link,
                    ..
                })
            );
            if first {
                a0 = at;
            }
            if let PMove::Line {
                to,
                phase: Phase::Link,
                ..
            } = *mv
            {
                let l = ((to[0] - at[0]).powi(2) + (to[1] - at[1]).powi(2)).sqrt();
                let n = (l / m.h).ceil().max(1.0) as usize;
                for k in 0..=n {
                    let t = k as f64 / n as f64;
                    let q = [
                        at[0] + t * (to[0] - at[0]),
                        at[1] + t * (to[1] - at[1]),
                        at[2] + t * (to[2] - at[2]),
                    ];
                    let need = top_near(&m, q[0], q[1], rr) + CLEAR;
                    // exempt: the climb straight up from the cut's end (where the retract it
                    // replaced climbed too), and the landing (checked below)
                    let at_a = q[0] == a0[0] && q[1] == a0[1];
                    let landing = q == to
                        && !matches!(
                            out.get(i + 1),
                            Some(PMove::Line {
                                phase: Phase::Link,
                                ..
                            })
                        );
                    assert!(
                        q[2] >= need - 1e-9 || at_a || landing,
                        "hop {i} sample {k}: tip {} under {need}",
                        q[2]
                    );
                    assert!(q[2] <= ceiling + 1e-9, "hop {i} above the retract it replaced");
                }
                let last = !matches!(
                    out.get(i + 1),
                    Some(PMove::Line {
                        phase: Phase::Link,
                        ..
                    })
                );
                if last {
                    assert!(
                        top_near(&m, to[0], to[1], rr) <= to[2] + 1e-9,
                        "hop {i} ends on a disk with material above it"
                    );
                    hops += 1;
                }
            }
            sweep(&mut m, &prof, at, mv);
            at = end(mv);
        }
        hops
    }

    #[test]
    fn a_retract_over_material_becomes_one_hop_clear_of_it() {
        let (mv, start) = path([46.0, 10.0, -5.0]);
        let (mut m, prof) = stock();
        let (out, n) = hops(&mv, start, &mut m, &prof, &[], 9000.0);
        assert_eq!(n, 1);
        assert!(out.iter().all(|m| !matches!(m, PMove::Rapid { .. })), "{out:?}");
        assert_eq!(check(&out, start, 5.0), 1);
        // it crosses just over the material (top 0), not at the retract's 5
        let top = out
            .iter()
            .filter_map(|m| match *m {
                PMove::Line {
                    to,
                    phase: Phase::Link,
                    ..
                } => Some(to[2]),
                _ => None,
            })
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((top - CLEAR).abs() < 1e-9, "crossing at {top}");
        // both ends ramp (the pockets leave room beside the cut's end and the landing): no
        // move of the hop is vertical
        let mut at = [14.0, 10.0, -5.0];
        for m in out.iter().skip(1) {
            if let PMove::Line {
                to,
                phase: Phase::Link,
                ..
            } = *m
            {
                assert!(
                    to[0] != at[0] || to[1] != at[1],
                    "a vertical move in the hop: {out:?}"
                );
                at = to;
            }
        }
    }

    #[test]
    fn a_cut_ending_against_a_wall_climbs_straight_up() {
        // the cut ends at x = 16, its disk reaching the wall of the material at 19
        let start = [6.0, 10.0, -5.0];
        let mv = vec![
            line([16.0, 10.0, -5.0], Phase::Cut),
            PMove::Rapid {
                to: [16.0, 10.0, 5.0],
            },
            PMove::Rapid {
                to: [46.0, 10.0, 5.0],
            },
            PMove::Rapid {
                to: [46.0, 10.0, -5.0],
            },
            line([50.0, 10.0, -5.0], Phase::Cut),
        ];
        let (mut m, prof) = stock();
        let (out, n) = hops(&mv, start, &mut m, &prof, &[], 9000.0);
        assert_eq!(n, 1);
        match out[1] {
            PMove::Line {
                to,
                phase: Phase::Link,
                ..
            } => {
                assert!(to[0] == 16.0 && to[1] == 10.0, "not straight up: {out:?}")
            }
            _ => panic!("{out:?}"),
        }
        assert_eq!(check(&out, start, 5.0), 1);
    }

    #[test]
    fn a_retract_onto_an_entry_hops_down_to_just_over_the_material() {
        // the next start stands in the material between the pockets (top 0): the retract
        // crosses at 5 and plunges from there; the hop lands at 0.5 and the plunge goes on
        let start = [6.0, 10.0, -5.0];
        let mv = vec![
            line([14.0, 10.0, -5.0], Phase::Cut),
            PMove::Rapid {
                to: [14.0, 10.0, 5.0],
            },
            PMove::Rapid {
                to: [30.0, 10.0, 5.0],
            },
            line([30.0, 10.0, -5.0], Phase::Plunge),
            line([34.0, 10.0, -5.0], Phase::Cut),
        ];
        let (mut m, prof) = stock();
        let (out, n) = hops(&mv, start, &mut m, &prof, &[], 9000.0);
        assert_eq!(n, 1);
        let k = out
            .iter()
            .position(|m| {
                matches!(
                    m,
                    PMove::Line {
                        phase: Phase::Plunge,
                        ..
                    }
                )
            })
            .unwrap();
        assert_eq!(end(&out[k - 1]), [30.0, 10.0, CLEAR], "{out:?}");
        assert_eq!(out[k], mv[3]);
        assert!(out[1..k].iter().all(|m| matches!(
            m,
            PMove::Line {
                phase: Phase::Link,
                ..
            }
        )));
        assert_eq!(check(&out, start, 5.0), 1);
    }

    #[test]
    fn a_retract_that_enters_material_keeps_its_descent() {
        // the next start stands in the material between the pockets
        let (mv, start) = path([30.0, 10.0, -5.0]);
        let (mut m, prof) = stock();
        let (out, n) = hops(&mv, start, &mut m, &prof, &[], 9000.0);
        assert_eq!(n, 0);
        assert_eq!(out, mv);
    }
}
