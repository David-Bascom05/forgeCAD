//! The xy patterns the draped strategies follow. Plain, published forms only:
//!
//! - raster: parallel lines at an angle, one stepover apart, alternating direction (zig-zag)
//!   or all one way;
//! - radial: spokes about a centre, between two radii, at most a given spacing apart at the
//!   outer radius;
//! - spiral: an Archimedean spiral about a *point* (r = r0 + pitch·θ/2π), inward or outward. Not
//!   a spiral grown outward from a medial-axis skeleton used as a hole (Autodesk US 10,108,172:
//!   avoided by construction, the centre is a single user point).
//!
//! Angles are taken through forge_math::dmath (deterministic: native = wasm).

use forge_math::dmath;

/// Parallel lines across the box [lo, hi] at `angle_deg` from +x, `step` apart, clipped to the
/// box. `zigzag` reverses every other line.
pub fn raster(lo: [f64; 2], hi: [f64; 2], angle_deg: f64, step: f64, zigzag: bool) -> Vec<Vec<[f64; 2]>> {
    let mut out = Vec::new();
    if !(step > 0.0) || !(lo[0] < hi[0] && lo[1] < hi[1]) {
        return out;
    }
    let (s, c) = dmath::sin_cos(angle_deg * (std::f64::consts::PI / 180.0));
    // direction u = (c, s), across n = (−s, c); the box's corners' extent along n
    let corners = [lo, [hi[0], lo[1]], hi, [lo[0], hi[1]]];
    let mut nmin = f64::INFINITY;
    let mut nmax = f64::NEG_INFINITY;
    for q in corners {
        let v = -s * q[0] + c * q[1];
        nmin = nmin.min(v);
        nmax = nmax.max(v);
    }
    let count = ((nmax - nmin) / step).floor() as usize;
    // centre the lines in the span
    let off = 0.5 * ((nmax - nmin) - step * count as f64);
    let mut k = 0;
    while k <= count {
        let v = nmin + off + step * k as f64;
        // the line {p : n·p = v}, p = v n + t u; clip t to the box (Liang–Barsky)
        let base = [-s * v, c * v];
        let mut t0 = f64::NEG_INFINITY;
        let mut t1 = f64::INFINITY;
        let mut ok = true;
        let mut ax = 0;
        while ax < 2 {
            let d = if ax == 0 { c } else { s };
            let b = base[ax];
            if d.abs() < 1e-15 {
                if b < lo[ax] || b > hi[ax] {
                    ok = false;
                }
            } else {
                let a0 = (lo[ax] - b) / d;
                let a1 = (hi[ax] - b) / d;
                t0 = t0.max(a0.min(a1));
                t1 = t1.min(a0.max(a1));
            }
            ax += 1;
        }
        if ok && t1 > t0 {
            let p0 = [base[0] + c * t0, base[1] + s * t0];
            let p1 = [base[0] + c * t1, base[1] + s * t1];
            if zigzag && k % 2 == 1 {
                out.push(vec![p1, p0]);
            } else {
                out.push(vec![p0, p1]);
            }
        }
        k += 1;
    }
    out
}

/// Spokes about `c` from radius r0 to r1 (r0 < r1), so that neighbouring spokes are at most
/// `step` apart at r1. Alternate spokes run outward and inward when `zigzag`.
pub fn radial(c: [f64; 2], r0: f64, r1: f64, step: f64, zigzag: bool) -> Vec<Vec<[f64; 2]>> {
    let mut out = Vec::new();
    if !(step > 0.0 && r0 >= 0.0 && r1 > r0) {
        return out;
    }
    let n = ((2.0 * std::f64::consts::PI * r1 / step).ceil() as usize).max(3);
    let mut k = 0;
    while k < n {
        let a = 2.0 * std::f64::consts::PI * (k as f64) / (n as f64);
        let (s, co) = dmath::sin_cos(a);
        let p0 = [c[0] + r0 * co, c[1] + r0 * s];
        let p1 = [c[0] + r1 * co, c[1] + r1 * s];
        if zigzag && k % 2 == 1 {
            out.push(vec![p1, p0]);
        } else {
            out.push(vec![p0, p1]);
        }
        k += 1;
    }
    out
}

/// An Archimedean spiral about `c` from radius r0 out to r1 (or in, when `inward`), `pitch`
/// apart per turn, as a polyline whose chords stay within `tol` of the true spiral.
pub fn spiral(c: [f64; 2], r0: f64, r1: f64, pitch: f64, tol: f64, inward: bool) -> Vec<[f64; 2]> {
    let mut out = Vec::new();
    if !(pitch > 0.0 && tol > 0.0 && r0 >= 0.0 && r1 > r0) {
        return out;
    }
    let turns = (r1 - r0) / pitch;
    let theta_end = 2.0 * std::f64::consts::PI * turns;
    let mut th = 0.0;
    loop {
        let r = r0 + pitch * th / (2.0 * std::f64::consts::PI);
        let (s, co) = dmath::sin_cos(th);
        out.push([c[0] + r * co, c[1] + r * s]);
        if th >= theta_end {
            break;
        }
        // chord sag of a circle of radius r with angle step dθ: r(1 − cos(dθ/2)) ≈ r dθ²/8 ≤ tol
        let rr = r.max(pitch);
        let dth = (8.0 * tol / rr).sqrt().min(0.5);
        th = (th + dth).min(theta_end);
    }
    if inward {
        out.reverse();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_covers_the_box() {
        let ls = raster([0.0, 0.0], [10.0, 5.0], 0.0, 1.0, true);
        assert_eq!(ls.len(), 6);
        assert_eq!(ls[1][0][0], 10.0);
        let d = raster([0.0, 0.0], [10.0, 10.0], 45.0, 1.0, false);
        assert!(d.len() >= 14);
        for l in &d {
            for p in l {
                assert!(p[0] >= -1e-9 && p[0] <= 10.0 + 1e-9 && p[1] >= -1e-9 && p[1] <= 10.0 + 1e-9);
            }
        }
    }

    #[test]
    fn radial_spacing() {
        let ls = radial([0.0, 0.0], 1.0, 10.0, 1.0, false);
        let a = ls[0][1];
        let b = ls[1][1];
        assert!(((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt() <= 1.0);
    }

    #[test]
    fn spiral_radius_grows_by_pitch() {
        let s = spiral([0.0, 0.0], 0.0, 5.0, 1.0, 0.001, false);
        let last = s.last().unwrap();
        assert!(((last[0].powi(2) + last[1].powi(2)).sqrt() - 5.0).abs() < 1e-9);
        assert!(s.len() > 50);
    }
}
