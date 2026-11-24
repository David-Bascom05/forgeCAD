//! Fewer, longer moves: a dense polyline of tool positions fitted by lines and arcs (G1, G2/G3)
//! within a chord tolerance. One fitter for every op that emits point paths (3D finishing,
//! adaptive).
//!
//! The method is the plain greedy one. From each piece's start it finds the longest run of input
//! vertices that one line, or one circular arc, stays within the tolerance of. The arc is in xy
//! with its height linear in the angle, which is `PMove::Arc`'s form. The run's end is found by
//! doubling and then bisecting, on the geometry alone. A piece is kept only when the caller's
//! `accept` agrees: that is the caller's proof (certification of the move, a cusp check), asked
//! once for the longest run; a refused run backs off, bisecting with proofs below it.
//! One input segment is always kept as it is. So the output never has more moves than the input,
//! and it is the input where nothing longer fits or is accepted.
//!
//! The bound, for every kept piece: each point of it is within `tol` of the input polyline, and
//! each point of the polyline it replaces is within `tol` of it (3D).
//! - A line from p_i to p_j: every vertex's projection onto it is within tol of the vertex, and
//!   the projections advance along the line. Between two consecutive vertices, the line's
//!   section and the input segment are both linear in one parameter, and the distance of two
//!   such segments is convex in it, so it is at most the larger of the two vertex distances.
//! - An arc: the same, with each section's chord standing in for the section. The section is
//!   within its sagitta of its chord at the same parameter, so each vertex is held to tol minus
//!   the sagittas of the sections either side of it.
//!
//! Everything is f64 and deterministic: no trig beyond forge_math::dmath, the same on native and
//! wasm.

use crate::ops::{PMove, Phase};
use forge_math::dmath;

/// One fitted move, from the previous piece's end (or the polyline's start).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Piece {
    Line {
        to: [f64; 3],
    },
    /// In xy about c, clockwise or not; the height linear in the angle (PMove::Arc).
    Arc {
        to: [f64; 3],
        c: [f64; 2],
        cw: bool,
    },
}

impl Piece {
    pub fn to(&self) -> [f64; 3] {
        match *self {
            Piece::Line { to } | Piece::Arc { to, .. } => to,
        }
    }
}

/// The fit's limits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    /// The chord tolerance (mm): every kept piece within this of the input, both ways.
    pub tol: f64,
    /// Arcs are tried as well as lines.
    pub arcs: bool,
    /// Arc radii outside [min_r, max_r] are not used (a large radius is a line to a control; a
    /// small one is a corner).
    pub min_r: f64,
    pub max_r: f64,
    /// The largest height change an arc may have (mm): 0 for planar arcs only.
    pub arc_dz: f64,
}

impl Fit {
    /// Lines only, within `tol`.
    pub fn lines(tol: f64) -> Fit {
        Fit {
            tol,
            arcs: false,
            min_r: 0.0,
            max_r: 0.0,
            arc_dz: 0.0,
        }
    }

    /// Lines and arcs of radius 0.05..200 mm, within `tol`; an arc's ends at most `tol` apart in
    /// height (planar but for rounding: a level pass's heights are computed, not copied). A
    /// larger radius is a line to a control (and a long reach to a check that boxes the circle).
    pub fn arcs(tol: f64) -> Fit {
        Fit {
            tol,
            arcs: true,
            min_r: 0.05,
            max_r: 200.0,
            arc_dz: tol,
        }
    }
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn finite(p: [f64; 3]) -> bool {
    p[0].is_finite() && p[1].is_finite() && p[2].is_finite()
}

/// The line pts[0] → pts[last], if every vertex is within `tol` of it and their projections
/// advance along it.
fn line_ok(pts: &[[f64; 3]], tol: f64) -> bool {
    let a = pts[0];
    let b = pts[pts.len() - 1];
    let d = sub(b, a);
    let l2 = dot(d, d);
    // (finite input: fit refuses non-finite points)
    if l2 <= 0.0 {
        return false;
    }
    let mut prev = 0.0;
    for &p in &pts[1..pts.len() - 1] {
        let v = sub(p, a);
        let s = dot(v, d) / l2;
        if !(s >= prev && s <= 1.0) {
            return false;
        }
        let q = [a[0] + s * d[0], a[1] + s * d[1], a[2] + s * d[2]];
        if norm(sub(p, q)) > tol {
            return false;
        }
        prev = s;
    }
    true
}

/// The circle through three points in xy: (centre, radius).
fn circle(a: [f64; 3], m: [f64; 3], b: [f64; 3]) -> Option<([f64; 2], f64)> {
    let (ax, ay) = (m[0] - a[0], m[1] - a[1]);
    let (bx, by) = (b[0] - a[0], b[1] - a[1]);
    let den = 2.0 * (ax * by - ay * bx);
    let scale = (ax * ax + ay * ay).max(bx * bx + by * by);
    if den.abs() <= 1e-12 * scale {
        return None;
    }
    let a2 = ax * ax + ay * ay;
    let b2 = bx * bx + by * by;
    let ux = (by * a2 - ay * b2) / den;
    let uy = (ax * b2 - bx * a2) / den;
    let r = (ux * ux + uy * uy).sqrt();
    let c = [a[0] + ux, a[1] + uy];
    (r.is_finite() && c[0].is_finite() && c[1].is_finite()).then_some((c, r))
}

/// The arc pts[0] → pts[last] through the middle vertex, if every vertex is within `tol` of it
/// (each section's sagitta taken off), the vertices advance along it, and it is within the
/// limits.
fn arc_fit(pts: &[[f64; 3]], f: &Fit) -> Option<Piece> {
    let n = pts.len();
    let a = pts[0];
    let b = pts[n - 1];
    if (b[2] - a[2]).abs() > f.arc_dz {
        return None;
    }
    let (c, r) = circle(a, pts[n / 2], b)?;
    if !(r >= f.min_r && r <= f.max_r) {
        return None;
    }
    let (ax, ay) = (a[0] - c[0], a[1] - c[1]);
    let (mx, my) = (pts[n / 2][0] - c[0], pts[n / 2][1] - c[1]);
    // the turn a → middle about c: clockwise when negative
    let cw = ax * my - ay * mx < 0.0;
    let tau = std::f64::consts::TAU;
    let th0 = dmath::atan2(ay, ax);
    // each vertex's angle from a in the direction of travel, in [0, τ)
    let ang = |p: [f64; 3]| {
        let t = dmath::atan2(p[1] - c[1], p[0] - c[0]);
        let mut u = if cw { th0 - t } else { t - th0 };
        while u < 0.0 {
            u += tau;
        }
        while u >= tau {
            u -= tau;
        }
        u
    };
    let sweep = ang(b);
    // at most three quarters of a turn, and not a vanishing one
    if !(sweep > 0.0 && sweep <= 1.5 * std::f64::consts::PI) {
        return None;
    }
    let mut us = Vec::with_capacity(n);
    us.push(0.0);
    for &p in &pts[1..n - 1] {
        us.push(ang(p));
    }
    us.push(sweep);
    // each vertex's distance from the arc's point at its angle (radial and height errors)
    let mut err = Vec::with_capacity(n);
    for k in 0..n {
        if k > 0 && us[k] < us[k - 1] {
            return None;
        }
        let p = pts[k];
        let er = (((p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2)).sqrt() - r).abs();
        let zl = a[2] + (b[2] - a[2]) * us[k] / sweep;
        let ez = (p[2] - zl).abs();
        err.push((er * er + ez * ez).sqrt());
    }
    // each section's sagitta, bounded above by r Δ²/8 widened (Δ at most half a radian)
    for k in 0..n - 1 {
        let du = us[k + 1] - us[k];
        if du > 0.5 {
            return None;
        }
        let sag = 1.01 * r * du * du / 8.0 + 1e-12;
        if err[k].max(err[k + 1]) + sag > f.tol {
            return None;
        }
    }
    Some(Piece::Arc { to: b, c, cw })
}

/// The piece covering pts[i..=j], if one fits and is accepted.
fn piece(
    pts: &[[f64; 3]],
    i: usize,
    j: usize,
    f: &Fit,
    accept: &mut dyn FnMut([f64; 3], &Piece) -> bool,
) -> Option<Piece> {
    let run = &pts[i..=j];
    if line_ok(run, f.tol) {
        let p = Piece::Line { to: pts[j] };
        if accept(pts[i], &p) {
            return Some(p);
        }
    }
    if f.arcs
        && j >= i + 2
        && let Some(p) = arc_fit(run, f)
        && accept(pts[i], &p)
    {
        return Some(p);
    }
    None
}

/// The polyline `pts` fitted (see the module). `accept(from, piece)` is asked about every piece
/// longer than one input segment before it is kept.
pub fn fit(pts: &[[f64; 3]], f: Fit, accept: &mut dyn FnMut([f64; 3], &Piece) -> bool) -> Vec<Piece> {
    let n = pts.len();
    let mut out = Vec::new();
    if n < 2 {
        return out;
    }
    // non-finite input or tolerance: the input as it is
    if !(f.tol > 0.0 && f.tol.is_finite()) || !pts.iter().all(|&p| finite(p)) {
        return pts[1..].iter().map(|&to| Piece::Line { to }).collect();
    }
    // the geometry alone (no proof): a line or an arc fits pts[i..=j]
    let geo = |i: usize, j: usize| {
        let run = &pts[i..=j];
        line_ok(run, f.tol) || (f.arcs && j >= i + 2 && arc_fit(run, &f).is_some())
    };
    let mut i = 0;
    while i + 1 < n {
        // 1. the longest run that fits, by its geometry alone: doubling, then bisecting
        let mut fits = i + 1;
        let mut k = 2;
        let mut bad = n;
        loop {
            let j = (i + k).min(n - 1);
            if j <= fits {
                break;
            }
            if geo(i, j) {
                fits = j;
                if j == n - 1 {
                    break;
                }
                k *= 2;
            } else {
                bad = j;
                break;
            }
        }
        while bad < n && bad > fits + 1 {
            let j = fits + (bad - fits) / 2;
            if geo(i, j) {
                fits = j;
            } else {
                bad = j;
            }
        }
        // 2. that piece proven (one ask, two with an arc after a line); if refused, the longest
        //    proven piece below it, bisecting with proofs
        let mut good = (i + 1, Piece::Line { to: pts[i + 1] });
        if fits > i + 1 {
            match piece(pts, i, fits, &f, accept) {
                Some(p) => good = (fits, p),
                None => {
                    let mut bad = fits;
                    while bad > good.0 + 1 {
                        let j = good.0 + (bad - good.0) / 2;
                        match piece(pts, i, j, &f, accept) {
                            Some(p) => good = (j, p),
                            None => bad = j,
                        }
                    }
                }
            }
        }
        out.push(good.1);
        i = good.0;
    }
    out
}

/// A point of the piece from `at`, at the fraction t of it (an arc's height linear in the angle).
fn point(at: [f64; 3], p: &Piece, t: f64) -> [f64; 3] {
    match *p {
        Piece::Line { to } => [
            at[0] + (to[0] - at[0]) * t,
            at[1] + (to[1] - at[1]) * t,
            at[2] + (to[2] - at[2]) * t,
        ],
        Piece::Arc { to, c, cw } => {
            let r = ((at[0] - c[0]).powi(2) + (at[1] - c[1]).powi(2)).sqrt();
            let a0 = dmath::atan2(at[1] - c[1], at[0] - c[0]);
            let mut a1 = dmath::atan2(to[1] - c[1], to[0] - c[0]);
            let tau = std::f64::consts::TAU;
            if cw {
                while a1 >= a0 {
                    a1 -= tau;
                }
            } else {
                while a1 <= a0 {
                    a1 += tau;
                }
            }
            let (s, co) = dmath::sin_cos(a0 + (a1 - a0) * t);
            [c[0] + r * co, c[1] + r * s, at[2] + (to[2] - at[2]) * t]
        }
    }
}

fn seg_dist(p: [f64; 3], a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = sub(b, a);
    let l2 = dot(d, d);
    let s = if l2 > 0.0 {
        (dot(sub(p, a), d) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    norm(sub(p, [a[0] + s * d[0], a[1] + s * d[1], a[2] + s * d[2]]))
}

/// A point's distance to the piece from `at`: exact for a line; for an arc, to the arc's point
/// at the point's own angle (clamped to the sweep), which is at least the true distance.
fn piece_dist(q: [f64; 3], at: [f64; 3], p: &Piece) -> f64 {
    match *p {
        Piece::Line { to } => seg_dist(q, at, to),
        Piece::Arc { to, c, cw } => {
            let tau = std::f64::consts::TAU;
            let a0 = dmath::atan2(at[1] - c[1], at[0] - c[0]);
            // the angle from the start in the direction of travel, in [0, τ)
            let fwd = |x: f64, y: f64| {
                let t = dmath::atan2(y - c[1], x - c[0]);
                let mut u = if cw { a0 - t } else { t - a0 };
                while u < 0.0 {
                    u += tau;
                }
                while u >= tau {
                    u -= tau;
                }
                u
            };
            let sweep = fwd(to[0], to[1]);
            if sweep <= 0.0 {
                return norm(sub(q, at)).min(norm(sub(q, to)));
            }
            let u = fwd(q[0], q[1]);
            // past the end: the nearer end of the sweep (by angle on the far side)
            let t = if u <= sweep {
                u / sweep
            } else if u - sweep < tau - u {
                1.0
            } else {
                0.0
            };
            norm(sub(q, point(at, p, t)))
        }
    }
}

fn poly_dist(p: [f64; 3], poly: &[[f64; 3]]) -> f64 {
    let mut m = f64::INFINITY;
    for w in poly.windows(2) {
        m = m.min(seg_dist(p, w[0], w[1]));
    }
    m
}

/// The fitted pieces' measured distance from the polyline they replace, both ways: points
/// sampled along each piece (at most `h` apart) against the input points it covers, and those
/// points (and their chords' midpoints) against the piece itself (an arc's point at the same
/// angle: at least the true distance). A measurement, beside the
/// bound `fit` keeps by construction; NaN when the pieces do not end on the input's vertices
/// in order.
pub fn deviation(pts: &[[f64; 3]], ps: &[Piece], h: f64) -> f64 {
    let mut worst: f64 = 0.0;
    let mut i = 0;
    for p in ps {
        let to = p.to();
        let Some(dj) = pts[i + 1..].iter().position(|&q| q == to) else {
            return f64::NAN;
        };
        let j = i + 1 + dj;
        let run = &pts[i..=j];
        let at = pts[i];
        // the piece's length bounds its samples' count
        let mut len = 0.0;
        for w in run.windows(2) {
            len += norm(sub(w[1], w[0]));
        }
        let n = ((len / h).ceil() as usize).clamp(2, 100_000);
        let samples: Vec<[f64; 3]> = (0..=n).map(|k| point(at, p, k as f64 / n as f64)).collect();
        for &q in &samples {
            worst = worst.max(poly_dist(q, run));
        }
        for w in run.windows(2) {
            let m = [
                0.5 * (w[0][0] + w[1][0]),
                0.5 * (w[0][1] + w[1][1]),
                0.5 * (w[0][2] + w[1][2]),
            ];
            worst = worst.max(piece_dist(w[0], at, p)).max(piece_dist(m, at, p));
        }
        i = j;
    }
    worst
}

/// The pieces as moves at feed `f` (arcs are cutting moves to the checker, whatever `phase`).
pub fn to_moves(ps: &[Piece], f: f64, phase: Phase) -> Vec<PMove> {
    ps.iter()
        .map(|p| match *p {
            Piece::Line { to } => PMove::Line { to, f, phase },
            Piece::Arc { to, c, cw } => PMove::Arc { to, c, cw, f },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yes(_: [f64; 3], _: &Piece) -> bool {
        true
    }

    /// The largest distance from the input's vertices and chord midpoints to the fitted path,
    /// and from the fitted path's sampled points to the input (both by dense sampling).
    fn deviation(pts: &[[f64; 3]], ps: &[Piece]) -> f64 {
        let mut fitted = vec![pts[0]];
        let mut at = pts[0];
        for p in ps {
            match *p {
                Piece::Line { to } => {
                    for s in 1..=50 {
                        let t = s as f64 / 50.0;
                        fitted.push([
                            at[0] + (to[0] - at[0]) * t,
                            at[1] + (to[1] - at[1]) * t,
                            at[2] + (to[2] - at[2]) * t,
                        ]);
                    }
                }
                Piece::Arc { to, c, cw } => {
                    let r = ((at[0] - c[0]).powi(2) + (at[1] - c[1]).powi(2)).sqrt();
                    let a0 = dmath::atan2(at[1] - c[1], at[0] - c[0]);
                    let mut a1 = dmath::atan2(to[1] - c[1], to[0] - c[0]);
                    let tau = std::f64::consts::TAU;
                    if cw {
                        while a1 >= a0 {
                            a1 -= tau;
                        }
                    } else {
                        while a1 <= a0 {
                            a1 += tau;
                        }
                    }
                    for s in 1..=200 {
                        let t = s as f64 / 200.0;
                        let (sn, cs) = dmath::sin_cos(a0 + (a1 - a0) * t);
                        fitted.push([c[0] + r * cs, c[1] + r * sn, at[2] + (to[2] - at[2]) * t]);
                    }
                }
            }
            at = p.to();
        }
        let seg_d = |p: [f64; 3], a: [f64; 3], b: [f64; 3]| {
            let d = sub(b, a);
            let l2 = dot(d, d);
            let s = if l2 > 0.0 {
                (dot(sub(p, a), d) / l2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            norm(sub(p, [a[0] + s * d[0], a[1] + s * d[1], a[2] + s * d[2]]))
        };
        let poly_d = |p: [f64; 3], poly: &[[f64; 3]]| {
            let mut m = f64::INFINITY;
            for w in poly.windows(2) {
                m = m.min(seg_d(p, w[0], w[1]));
            }
            m
        };
        let mut worst: f64 = 0.0;
        for &q in &fitted {
            worst = worst.max(poly_d(q, pts));
        }
        for w in pts.windows(2) {
            for s in 0..=10 {
                let t = s as f64 / 10.0;
                let q = [
                    w[0][0] + (w[1][0] - w[0][0]) * t,
                    w[0][1] + (w[1][1] - w[0][1]) * t,
                    w[0][2] + (w[1][2] - w[0][2]) * t,
                ];
                worst = worst.max(poly_d(q, &fitted));
            }
        }
        worst
    }

    #[test]
    fn a_straight_run_is_one_line() {
        let pts: Vec<[f64; 3]> = (0..=100)
            .map(|k| [k as f64 * 0.1, 2.0, 1.0 - 0.01 * k as f64])
            .collect();
        let ps = fit(&pts, Fit::lines(0.001), &mut yes);
        assert_eq!(ps, vec![Piece::Line { to: pts[100] }]);
    }

    #[test]
    fn a_planar_circle_is_a_few_arcs_within_tolerance() {
        let n = 400;
        let pts: Vec<[f64; 3]> = (0..=n)
            .map(|k| {
                let (s, c) = dmath::sin_cos(std::f64::consts::TAU * k as f64 / n as f64);
                [10.0 + 5.0 * c, -3.0 + 5.0 * s, 4.0]
            })
            .collect();
        let tol = 0.002;
        let ps = fit(&pts, Fit::arcs(tol), &mut yes);
        assert!(ps.len() <= 4, "{}", ps.len());
        assert!(ps.iter().all(|p| matches!(p, Piece::Arc { cw: false, .. })));
        assert!(deviation(&pts, &ps) <= tol * 1.0001, "{}", deviation(&pts, &ps));
        // the public measurement agrees (sampled more finely here than there)
        let m = super::deviation(&pts, &ps, 0.001);
        assert!(m <= tol * 1.0001 && m > 0.0, "{m}");
        // lines only: many more pieces, still within tolerance
        let pl = fit(&pts, Fit::lines(tol), &mut yes);
        assert!(pl.len() > 20 && pl.len() < n, "{}", pl.len());
        assert!(deviation(&pts, &pl) <= tol * 1.0001);
    }

    #[test]
    fn a_curved_profile_stays_within_tolerance() {
        // a draped line over a bump: x straight, z = a sine
        let pts: Vec<[f64; 3]> = (0..=600)
            .map(|k| {
                let x = k as f64 * 0.05;
                [x, 1.0, 3.0 * dmath::sin(x * 0.4)]
            })
            .collect();
        for tol in [0.0005, 0.002, 0.01] {
            let ps = fit(&pts, Fit::arcs(tol), &mut yes);
            assert!(ps.len() < 600);
            let d = deviation(&pts, &ps);
            assert!(d <= tol * 1.0001, "tol {tol}: {d}");
            assert_eq!(ps.last().unwrap().to(), pts[600]);
        }
    }

    #[test]
    fn the_longest_run_is_proven_once() {
        // a straight run: found by its geometry, then one ask
        let pts: Vec<[f64; 3]> = (0..=1000).map(|k| [k as f64 * 0.1, 0.0, 0.0]).collect();
        let mut asks = 0;
        let ps = fit(&pts, Fit::lines(0.001), &mut |_, _| {
            asks += 1;
            true
        });
        assert_eq!(ps.len(), 1);
        assert_eq!(asks, 1);
    }

    #[test]
    fn a_corner_is_kept() {
        let mut pts: Vec<[f64; 3]> = (0..=10).map(|k| [k as f64, 0.0, 0.0]).collect();
        pts.extend((1..=10).map(|k| [10.0, k as f64, 0.0]));
        let ps = fit(&pts, Fit::arcs(0.01), &mut yes);
        assert_eq!(
            ps,
            vec![
                Piece::Line { to: [10.0, 0.0, 0.0] },
                Piece::Line {
                    to: [10.0, 10.0, 0.0]
                }
            ]
        );
    }

    #[test]
    fn rejected_pieces_back_off_to_the_input() {
        let pts: Vec<[f64; 3]> = (0..=20).map(|k| [k as f64, 0.0, 0.0]).collect();
        // nothing longer than 3 segments is accepted
        let mut ask = 0;
        let ps = fit(&pts, Fit::lines(0.01), &mut |a, p| {
            ask += 1;
            p.to()[0] - a[0] <= 3.0
        });
        assert_eq!(ps.len(), 7);
        assert!(ask > 0);
        // nothing accepted: exactly the input
        let ps = fit(&pts, Fit::arcs(0.01), &mut |_, _| false);
        let input: Vec<Piece> = pts[1..].iter().map(|&to| Piece::Line { to }).collect();
        assert_eq!(ps, input);
    }

    #[test]
    fn degenerate_input_is_the_input() {
        assert!(fit(&[], Fit::lines(0.01), &mut yes).is_empty());
        assert!(fit(&[[0.0; 3]], Fit::lines(0.01), &mut yes).is_empty());
        let pts = [[0.0, 0.0, 0.0], [1.0, f64::NAN, 0.0], [2.0, 0.0, 0.0]];
        assert_eq!(fit(&pts, Fit::lines(0.01), &mut yes).len(), 2);
        let pts = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
        assert_eq!(fit(&pts, Fit::lines(f64::NAN), &mut yes).len(), 2);
        // a run back on itself is not one line (the projections must advance)
        let pts = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
        assert_eq!(fit(&pts, Fit::lines(0.01), &mut yes).len(), 2);
    }

    #[test]
    fn helical_arcs_only_within_arc_dz() {
        let n = 100;
        let pts: Vec<[f64; 3]> = (0..=n)
            .map(|k| {
                let (s, c) = dmath::sin_cos(std::f64::consts::PI * k as f64 / n as f64);
                [5.0 * c, 5.0 * s, -0.01 * k as f64]
            })
            .collect();
        assert!(
            fit(&pts, Fit::arcs(0.002), &mut yes)
                .iter()
                .all(|p| matches!(p, Piece::Line { .. }))
        );
        let mut f = Fit::arcs(0.002);
        f.arc_dz = 2.0;
        let ps = fit(&pts, f, &mut yes);
        assert!(ps.len() <= 2, "{ps:?}");
        assert!(deviation(&pts, &ps) <= 0.002 * 1.0001);
    }

    #[test]
    fn moves_carry_the_feed() {
        let ps = [
            Piece::Line { to: [1.0, 0.0, 0.0] },
            Piece::Arc {
                to: [2.0, 1.0, 0.0],
                c: [1.0, 1.0],
                cw: false,
            },
        ];
        let m = to_moves(&ps, 900.0, Phase::Cut);
        assert_eq!(
            m[0],
            PMove::Line {
                to: [1.0, 0.0, 0.0],
                f: 900.0,
                phase: Phase::Cut
            }
        );
        assert!(matches!(m[1], PMove::Arc { f, cw: false, .. } if f == 900.0));
    }
}
