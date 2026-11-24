//! Where lines and circles in the plane meet: decided exactly, placed in f64.
//!
//! Whether a line and a circle, or two circles, cross (two points), touch (one) or miss is the
//! sign of a polynomial in the inputs. It is read from f64 where a forward error bound proves it,
//! else computed exactly in dyadics ([`Dy`]): no tolerance decides it. The points themselves are
//! f64 (they are irrational in general), from one formula per case.

use crate::dy::Dy;

/// How two curves meet, and where.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Meet2 {
    /// They do not meet (or the case is degenerate: a line with no direction, concentric circles).
    None,
    /// They touch at one point.
    Touch([f64; 2]),
    /// They cross at two points.
    Cross([f64; 2], [f64; 2]),
}

impl Meet2 {
    /// The points, in order: none, one, or two.
    pub fn points(&self) -> ([[f64; 2]; 2], usize) {
        match *self {
            Meet2::None => ([[0.0; 2]; 2], 0),
            Meet2::Touch(p) => ([p, p], 1),
            Meet2::Cross(p, q) => ([p, q], 2),
        }
    }
}

/// `2⁻⁵³`.
const EPS: f64 = f64::EPSILON * 0.5;

/// The sign of `x`, with `x`'s computed value `v` and `m` a bound on the magnitudes it was formed
/// from: decided from `v` when `|v| > k ε m`, else None (exact arithmetic decides).
#[inline]
fn filtered(v: f64, m: f64, k: f64) -> Option<i32> {
    if !(v.is_finite() && m.is_finite()) {
        return None;
    }
    if v.abs() > k * EPS * m {
        Some(if v > 0.0 { 1 } else { -1 })
    } else {
        None
    }
}

/// The line `a0 + au x + av y = 0` and the circle about `c` of radius `r`.
///
/// They cross, touch or miss as `r² (au² + av²) − (a0 + au cx + av cy)²` is positive, zero or
/// negative. The points: the foot of the centre on the line, `±` half the chord along the line.
pub fn line_circle(l: [f64; 3], c: [f64; 2], r: f64) -> Meet2 {
    let [a0, au, av] = l;
    if au == 0.0 && av == 0.0 {
        return Meet2::None;
    }
    let s = line_circle_sign(l, c, r);
    if s < 0 {
        return Meet2::None;
    }
    let nl = (au * au + av * av).sqrt();
    let dist = (a0 + au * c[0] + av * c[1]) / nl;
    let n = [au / nl, av / nl];
    let foot = [c[0] - n[0] * dist, c[1] - n[1] * dist];
    if s == 0 {
        return Meet2::Touch(foot);
    }
    let half = (r * r - dist * dist).max(0.0).sqrt();
    // the line's direction: n turned a quarter
    let t = [-n[1], n[0]];
    Meet2::Cross(
        [foot[0] - t[0] * half, foot[1] - t[1] * half],
        [foot[0] + t[0] * half, foot[1] + t[1] * half],
    )
}

/// The sign of `r² (au² + av²) − (a0 + au cx + av cy)²`.
fn line_circle_sign(l: [f64; 3], c: [f64; 2], r: f64) -> i32 {
    let [a0, au, av] = l;
    let g = a0 + au * c[0] + av * c[1];
    let gm = a0.abs() + (au * c[0]).abs() + (av * c[1]).abs();
    let w = r * r * (au * au + av * av);
    let v = w - g * g;
    // each side within a few ulps of its magnitude (sums of at most three products, squared)
    if let Some(s) = filtered(v, w.abs() + gm * gm, 16.0) {
        return s;
    }
    let (a0, au, av) = (Dy::from_f64(a0), Dy::from_f64(au), Dy::from_f64(av));
    let (cx, cy, r) = (Dy::from_f64(c[0]), Dy::from_f64(c[1]), Dy::from_f64(r));
    let g = a0.add(&au.mul(&cx)).add(&av.mul(&cy));
    let w = r.mul(&r).mul(&au.mul(&au).add(&av.mul(&av)));
    w.sub(&g.mul(&g)).signum()
}

/// The circles about `c1` and `c2` of radii `r1` and `r2`.
///
/// With `d` the distance of the centres, they cross where `(r1 − r2)² < d² < (r1 + r2)²`, touch
/// where `d²` is either bound (`d > 0`), and otherwise miss; concentric circles never meet here.
/// The points: along the line of centres at `a = (r1² − r2² + d²) / 2d` from `c1`, `±` the half
/// chord `h = √(r1² − a²)` across it.
pub fn circle_circle(c1: [f64; 2], r1: f64, c2: [f64; 2], r2: f64) -> Meet2 {
    let s = circle_circle_sign(c1, r1, c2, r2);
    if s < 0 {
        return Meet2::None;
    }
    let d = [c2[0] - c1[0], c2[1] - c1[1]];
    let dl = (d[0] * d[0] + d[1] * d[1]).sqrt();
    let a = (r1 * r1 - r2 * r2 + dl * dl) / (2.0 * dl);
    let n = [d[0] / dl, d[1] / dl];
    let m = [c1[0] + n[0] * a, c1[1] + n[1] * a];
    if s == 0 {
        return Meet2::Touch(m);
    }
    let h = (r1 * r1 - a * a).max(0.0).sqrt();
    let t = [-n[1], n[0]];
    Meet2::Cross(
        [m[0] - t[0] * h, m[1] - t[1] * h],
        [m[0] + t[0] * h, m[1] + t[1] * h],
    )
}

/// -1 where the circles miss (or are concentric), 0 where they touch, 1 where they cross: the
/// smaller of the signs of `(r1 + r2)² − d²` and `d² − (r1 − r2)²`.
fn circle_circle_sign(c1: [f64; 2], r1: f64, c2: [f64; 2], r2: f64) -> i32 {
    let (dx, dy) = (c2[0] - c1[0], c2[1] - c1[1]);
    let d2 = dx * dx + dy * dy;
    let (sp, sm) = (r1 + r2, r1 - r2);
    let m = (c1[0].abs() + c2[0].abs()).powi(2) + (c1[1].abs() + c2[1].abs()).powi(2);
    let rm = (r1.abs() + r2.abs()).powi(2);
    let outer = filtered(sp * sp - d2, rm + m, 16.0);
    let inner = filtered(d2 - sm * sm, rm + m, 16.0);
    let exact = || {
        let (dx, dy) = (
            Dy::from_f64(c2[0]).sub(&Dy::from_f64(c1[0])),
            Dy::from_f64(c2[1]).sub(&Dy::from_f64(c1[1])),
        );
        let d2 = dx.mul(&dx).add(&dy.mul(&dy));
        (d2, Dy::from_f64(r1), Dy::from_f64(r2))
    };
    let (o, i, z) = match (outer, inner) {
        (Some(o), Some(i)) => (o, i, d2 == 0.0 && dx == 0.0 && dy == 0.0),
        _ => {
            let (d2, r1, r2) = exact();
            let (sp, sm) = (r1.add(&r2), r1.sub(&r2));
            (
                sp.mul(&sp).sub(&d2).signum(),
                d2.sub(&sm.mul(&sm)).signum(),
                d2.is_zero(),
            )
        }
    };
    if z {
        return -1;
    }
    o.min(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_and_circle() {
        // y = 0 against the unit circle: crosses at (1, 0) and (-1, 0), in that order
        assert_eq!(
            line_circle([0.0, 0.0, 1.0], [0.0, 0.0], 1.0),
            Meet2::Cross([1.0, 0.0], [-1.0, 0.0])
        );
        // y = 1: touches at (0, 1), decided exactly
        assert_eq!(
            line_circle([-1.0, 0.0, 1.0], [0.0, 0.0], 1.0),
            Meet2::Touch([0.0, 1.0])
        );
        // y = 1 + 2^-50: misses, though only by an ulp-sized gap
        assert_eq!(
            line_circle([-(1.0 + 2f64.powi(-50)), 0.0, 1.0], [0.0, 0.0], 1.0),
            Meet2::None
        );
        assert_eq!(line_circle([1.0, 0.0, 0.0], [0.0, 0.0], 0.5), Meet2::None);
        assert_eq!(line_circle([1.0, 0.0, 0.0], [0.0, 0.0], 0.0), Meet2::None);
    }

    #[test]
    fn two_circles() {
        // externally tangent at (1, 0)
        assert_eq!(
            circle_circle([0.0, 0.0], 1.0, [2.0, 0.0], 1.0),
            Meet2::Touch([1.0, 0.0])
        );
        // internally tangent at (2, 0)
        assert_eq!(
            circle_circle([0.0, 0.0], 2.0, [1.0, 0.0], 1.0),
            Meet2::Touch([2.0, 0.0])
        );
        // concentric, even equal: no meeting
        assert_eq!(circle_circle([1.0, 1.0], 1.0, [1.0, 1.0], 1.0), Meet2::None);
        assert_eq!(circle_circle([0.0, 0.0], 1.0, [3.0, 0.0], 1.0), Meet2::None);
        let Meet2::Cross(p, q) = circle_circle([0.0, 0.0], 1.0, [1.0, 0.0], 1.0) else {
            panic!("two unit circles a unit apart cross")
        };
        assert!((p[0] - 0.5).abs() < 1e-15 && (q[0] - 0.5).abs() < 1e-15);
        assert!((p[1] + 0.75f64.sqrt()).abs() < 1e-15 && (q[1] - 0.75f64.sqrt()).abs() < 1e-15);
        // a gap of one ulp in the sum of radii: they miss
        let r = 1.0 + f64::EPSILON;
        assert_eq!(
            circle_circle([0.0, 0.0], 1.0, [2.0 + 4.0 * f64::EPSILON, 0.0], r),
            Meet2::None
        );
    }

    /// The filter never disagrees with the exact sign, on random and near-tangent inputs.
    #[test]
    fn filter_agrees_with_exact() {
        let mut s = 0x1357_9bdf_2468_ace0u64;
        let mut next = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            ((s >> 11) as f64 / (1u64 << 53) as f64) * 4.0 - 2.0
        };
        for _ in 0..20_000 {
            let (c1, r1) = ([next(), next()], next().abs());
            // a second circle tangent to the first, nudged by a few ulps or not at all
            let ang = next();
            let r2 = next().abs();
            let dd = (r1 + r2) * (1.0 + (next() * 4.0).round() * f64::EPSILON);
            // a unit direction, rationally: ((1 - t²), 2t) / (1 + t²)
            let q = 1.0 + ang * ang;
            let c2 = [c1[0] + dd * (1.0 - ang * ang) / q, c1[1] + dd * 2.0 * ang / q];
            let exact = {
                let dx = Dy::from_f64(c2[0]).sub(&Dy::from_f64(c1[0]));
                let dy = Dy::from_f64(c2[1]).sub(&Dy::from_f64(c1[1]));
                let d2 = dx.mul(&dx).add(&dy.mul(&dy));
                let (a, b) = (Dy::from_f64(r1), Dy::from_f64(r2));
                let (sp, sm) = (a.add(&b), a.sub(&b));
                if d2.is_zero() {
                    -1
                } else {
                    sp.mul(&sp).sub(&d2).signum().min(d2.sub(&sm.mul(&sm)).signum())
                }
            };
            assert_eq!(circle_circle_sign(c1, r1, c2, r2), exact);
            let l = [next(), next(), next()];
            let want = {
                let (a0, au, av) = (Dy::from_f64(l[0]), Dy::from_f64(l[1]), Dy::from_f64(l[2]));
                let g = a0
                    .add(&au.mul(&Dy::from_f64(c1[0])))
                    .add(&av.mul(&Dy::from_f64(c1[1])));
                let r = Dy::from_f64(r1);
                r.mul(&r)
                    .mul(&au.mul(&au).add(&av.mul(&av)))
                    .sub(&g.mul(&g))
                    .signum()
            };
            assert_eq!(line_circle_sign(l, c1, r1), want);
        }
    }
}
