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
/// map left as the path leaves the stock. `feed`: the link feed. The number of hops made.
pub fn hops(
    moves: &[PMove],
    start: [f64; 3],
    map: &mut HeightMap,
    prof: &Profile,
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
    let rr = r + map.h;
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
            if j < moves.len()
                && top > at[2].max(b[2]) + 1e-9
                && let Some(h) = hop(map, rr, at, b, top, feed)
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
fn hop(map: &HeightMap, rr: f64, a: [f64; 3], b: [f64; 3], ceiling: f64, feed: f64) -> Option<Vec<PMove>> {
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
    let need: Vec<f64> = (0..=n)
        .map(|k| {
            let q = p(k);
            top_near(map, q[0], q[1], rr) + CLEAR
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
        for k in 1..=n / 2 {
            if (1..=k).all(|i| z0 + (hc - z0) * (i as f64 / k as f64) >= need_at(i)) {
                best = k;
            }
        }
        best
    };
    let ku = ramp(a[2], &|i| need[i]);
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
