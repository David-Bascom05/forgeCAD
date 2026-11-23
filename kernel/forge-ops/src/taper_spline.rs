//! Tapers of outlines with splines in them, the smallest exact step (research/kernel-audit/
//! RESUME-taper-splines.md): drafted where it is proved that no event comes within the depth,
//! refused (`TaperError::SplineEvent`) otherwise. Nothing is searched for; the proof is finite:
//! - every joint of a loop with a spline is smooth (tangent), so each point moves exactly δ along
//!   its normal and each section is the parallel curve of the outline;
//! - no part curls: δ κ < 1 on the side it moves to over the whole reach, κ's extremes per Bezier
//!   span from the roots of a polynomial (κ' = 0) and the span's ends; an arc's radius beyond the
//!   reach;
//! - no part meets another: two parts joined by a run of the loop whose tangents all lie in one
//!   open half-plane (the hodograph's control vectors, a line's direction, a short arc's end
//!   tangents) are a graph over that half-plane's direction, and so is their parallel curve, which
//!   cannot cross itself; any other two have the boxes they sweep apart (control hulls grown by
//!   the reach, or by the reach times the mitre's speed in a loop of lines);
//! - loops of lines and arcs only: `first_event`, exactly, with no event within the reach.

use crate::profile::{Profile, Segment};
use crate::taper::{ends_dir, unit};
use crate::taper_events::first_event;
use forge_math::{Vec2, dmath, tol};

/// A run of a loop: the tangents it can have (generators of their cone) and the box it sweeps.
struct Part {
    lp: usize,
    tangents: Vec<Vec2>,
    lo: Vec2,
    hi: Vec2,
}

/// Whether the vectors lie strictly within one open half-plane.
fn half_plane(v: &[Vec2]) -> bool {
    let mut a: Vec<f64> = v
        .iter()
        .filter(|x| x.length_sq() > 0.0)
        .map(|x| dmath::atan2(x.y, x.x))
        .collect();
    if a.is_empty() {
        return false;
    }
    a.sort_by(f64::total_cmp);
    let wrap = a[0] + dmath::TAU - a[a.len() - 1];
    let gap = a.windows(2).map(|w| w[1] - w[0]).fold(wrap, f64::max);
    gap > dmath::PI + 1e-9
}

fn binomial(n: usize, k: usize) -> f64 {
    (0..k).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64)
}

/// A Bezier's power-basis coefficients in s on [0, 1].
fn power(b: &[f64]) -> Vec<f64> {
    let n = b.len() - 1;
    (0..=n)
        .map(|k| {
            let s: f64 = (0..=k)
                .map(|i| {
                    let sign = if (k - i) % 2 == 0 { 1.0 } else { -1.0 };
                    sign * binomial(k, i) * b[i]
                })
                .sum();
            binomial(n, k) * s
        })
        .collect()
}

fn deriv(p: &[f64]) -> Vec<f64> {
    p.iter().enumerate().skip(1).map(|(k, c)| k as f64 * c).collect()
}

fn mul(a: &[f64], b: &[f64]) -> Vec<f64> {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let mut out = vec![0.0; a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (o, y) in out.iter_mut().skip(i).zip(b) {
            *o += x * y;
        }
    }
    out
}

fn add(a: &[f64], b: &[f64], k: f64) -> Vec<f64> {
    (0..a.len().max(b.len()))
        .map(|i| a.get(i).copied().unwrap_or(0.0) + k * b.get(i).copied().unwrap_or(0.0))
        .collect()
}

/// The largest signed curvature (positive turning left) over a polynomial plane Bezier, times
/// `sign`: at the span's ends or where κ' = 0, i.e. 2 num' den - 3 num den' = 0, num = x'y'' -
/// y'x'', den = x'² + y'². None where the curve stops (den reaching 0).
fn most_bend(xs: &[f64], ys: &[f64], sign: f64) -> Option<f64> {
    let (x, y) = (power(xs), power(ys));
    let (x1, y1) = (deriv(&x), deriv(&y));
    let (x2, y2) = (deriv(&x1), deriv(&y1));
    let num = add(&mul(&x1, &y2), &mul(&y1, &x2), -1.0);
    let den = add(&mul(&x1, &x1), &mul(&y1, &y1), 1.0);
    let crit = add(&mul(&deriv(&num), &den), &mul(&num, &deriv(&den)), -1.5);
    let lead = crit.iter().rposition(|c| *c != 0.0).map_or(0, |i| i + 1);
    let mut at = vec![0.0, 1.0];
    if lead > 1 {
        at.extend(
            forge_math::poly::real_roots(&crit[..lead], 0.0, 1.0)
                .into_iter()
                .map(|r| r.x),
        );
    }
    let mut most = f64::NEG_INFINITY;
    for s in at {
        let d = forge_math::poly::eval(&den, s);
        if d.is_nan() || d <= 0.0 {
            return None;
        }
        most = most.max(sign * forge_math::poly::eval(&num, s) / (d * d.sqrt()));
    }
    Some(most)
}

/// A segment's tangent at its start and end, for the smoothness of a loop with a spline.
fn tangents(s: &Segment) -> (Vec2, Vec2) {
    match s {
        Segment::Spline(c) => {
            let d = c.c.domain();
            let (a, b) = (c.c.deriv(d.lo).d1, c.c.deriv(d.hi).d1);
            (Vec2::new(a.x, a.y), Vec2::new(b.x, b.y))
        }
        Segment::Circle { .. } => (Vec2::new(1.0, 0.0), Vec2::new(1.0, 0.0)),
        s => ends_dir(s),
    }
}

fn smooth(a: Vec2, b: Vec2) -> bool {
    let (a, b) = (unit(a), unit(b));
    (a.x * b.y - a.y * b.x).abs() <= 1e-9 && a.dot(b) > 0.0
}

/// Whether the taper of prepared profile `p` (offset `sign` to its left, as far as `reach`) is
/// proved to meet no event: its sections all parallel curves of the outline, or the exact offsets
/// of loops of lines and arcs, none touching another.
pub(crate) fn clear(p: &Profile, sign: f64, reach: f64) -> bool {
    let has_spline =
        |l: &crate::profile::ProfileLoop| l.segments.iter().any(|s| matches!(s.0, Segment::Spline(_)));
    // loops of lines and arcs: exactly, together
    let plain: Vec<_> = p.loops.iter().filter(|l| !has_spline(l)).cloned().collect();
    if !plain.is_empty() {
        let sub = Profile {
            plane: p.plane,
            loops: plain,
        };
        if !matches!(first_event(&sub, sign, reach), Some(None)) {
            return false;
        }
    }
    let mut parts: Vec<Part> = Vec::new();
    // each loop's parts in order, and whether its joints are all smooth
    let mut runs: Vec<(usize, usize, bool)> = Vec::new();
    for (li, l) in p.loops.iter().enumerate() {
        let segs = &l.segments;
        let n = segs.len();
        let curvy = has_spline(l);
        let first = parts.len();
        // each joint's speed (1 where smooth; a mitre's 1 / cos of half its turn between lines)
        let mut speed = vec![1.0; n];
        for (i, sp) in speed.iter_mut().enumerate() {
            let (a, b) = (tangents(&segs[(i + n - 1) % n].0).1, tangents(&segs[i].0).0);
            if n == 1 || smooth(a, b) {
                continue;
            }
            if curvy {
                return false;
            }
            match (&segs[(i + n - 1) % n].0, &segs[i].0) {
                (Segment::Line { .. }, Segment::Line { .. }) => {
                    let (na, nb) = (unit(a).perp(), unit(b).perp());
                    let den = 1.0 + na.dot(nb);
                    if den <= 1e-12 {
                        return false;
                    }
                    *sp = (na + nb).length() / den;
                }
                _ => return false,
            }
        }
        for (i, (s, _)) in segs.iter().enumerate() {
            let grow = reach * speed[i].max(speed[(i + 1) % n]) + tol::LINEAR;
            let mut push = |pts: &[Vec2], tangents: Vec<Vec2>| {
                let lo = pts.iter().fold(Vec2::new(f64::INFINITY, f64::INFINITY), |m, q| {
                    Vec2::new(m.x.min(q.x), m.y.min(q.y))
                });
                let hi = pts
                    .iter()
                    .fold(Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY), |m, q| {
                        Vec2::new(m.x.max(q.x), m.y.max(q.y))
                    });
                parts.push(Part {
                    lp: li,
                    tangents,
                    lo: lo - Vec2::new(grow, grow),
                    hi: hi + Vec2::new(grow, grow),
                });
            };
            match s {
                Segment::Line { a, b } => push(&[a.to_vec(), b.to_vec()], vec![*b - *a]),
                Segment::Arc {
                    center,
                    radius,
                    start,
                    sweep,
                } => {
                    // curling: an arc turning to the side it moves to, its radius past the reach
                    let turn = if *sweep > 0.0 { 1.0 } else { -1.0 };
                    if turn * sign > 0.0 && reach >= *radius * (1.0 - 1e-9) {
                        return false;
                    }
                    let k = (sweep.abs() / 1.5).ceil().max(1.0) as usize;
                    for j in 0..k {
                        let (a0, a1) = (
                            start + sweep * j as f64 / k as f64,
                            start + sweep * (j + 1) as f64 / k as f64,
                        );
                        let at = |a: f64| {
                            let (sn, cs) = dmath::sin_cos(a);
                            Vec2::new(center.x + radius * cs, center.y + radius * sn)
                        };
                        let tan = |a: f64| {
                            let (sn, cs) = dmath::sin_cos(a);
                            Vec2::new(-sn * turn, cs * turn)
                        };
                        // the hull of a sub-arc under a quarter turn: its ends and where their tangents meet
                        let (mid, half) = (0.5 * (a0 + a1), 0.5 * (a1 - a0).abs());
                        let (sn, cs) = dmath::sin_cos(mid);
                        let r = radius / dmath::sin_cos(half).1;
                        let apex = Vec2::new(center.x + r * cs, center.y + r * sn);
                        push(&[at(a0), at(a1), apex], vec![tan(a0), tan(a1)]);
                    }
                }
                Segment::Circle { center, radius, .. } => {
                    let c = center.to_vec();
                    push(
                        &[c - Vec2::new(*radius, *radius), c + Vec2::new(*radius, *radius)],
                        vec![],
                    );
                }
                Segment::Spline(c) => {
                    if c.c.rational() {
                        return false;
                    }
                    for b in c.c.bezier_pieces(c.c.domain()) {
                        let xs: Vec<f64> = b.iter().map(|h| h[0]).collect();
                        let ys: Vec<f64> = b.iter().map(|h| h[1]).collect();
                        match most_bend(&xs, &ys, sign) {
                            Some(k) if k * reach < 1.0 - 1e-9 => {}
                            _ => return false,
                        }
                        let pts: Vec<Vec2> = b.iter().map(|h| Vec2::new(h[0], h[1])).collect();
                        let hodo: Vec<Vec2> = pts.windows(2).map(|w| w[1] - w[0]).collect();
                        push(&pts, hodo);
                    }
                }
            }
        }
        runs.push((first, parts.len(), curvy));
    }
    // how far forward from each part of a smooth loop its run stays in one half-plane
    let apart = |a: &Part, b: &Part| a.hi.x < b.lo.x || b.hi.x < a.lo.x || a.hi.y < b.lo.y || b.hi.y < a.lo.y;
    for &(lo, hi, curvy) in &runs {
        if !curvy {
            continue;
        }
        let m = hi - lo;
        // ahead[i]: the most parts after i (cyclically) whose union with i lies in a half-plane
        let mut ahead = vec![0usize; m];
        for (i, a) in ahead.iter_mut().enumerate() {
            let mut v = parts[lo + i].tangents.clone();
            if !half_plane(&v) {
                return false;
            }
            for k in 1..m {
                v.extend(parts[lo + (i + k) % m].tangents.iter().copied());
                if !half_plane(&v) {
                    break;
                }
                *a = k;
            }
        }
        for i in 0..m {
            for j in 0..m {
                let fwd = (j + m - i) % m;
                if i == j || fwd <= ahead[i] || (m - fwd) <= ahead[j] {
                    continue;
                }
                if !apart(&parts[lo + i], &parts[lo + j]) {
                    return false;
                }
            }
        }
    }
    // parts of different loops, one of them with a spline
    for (i, a) in parts.iter().enumerate() {
        for b in &parts[i + 1..] {
            if a.lp == b.lp || !(runs[a.lp].2 || runs[b.lp].2) {
                continue;
            }
            if !apart(a, b) {
                return false;
            }
        }
    }
    true
}
