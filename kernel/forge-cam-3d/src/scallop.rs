//! Cusp (scallop) heights between adjacent finishing passes, and the stepover that bounds them.
//!
//! Everything here is a closed form evaluated in outward-rounded intervals, so the interval a
//! function returns holds the exact real value for the exact real inputs (f64 inputs are exact
//! reals). The geometry (research/cam/toolpath-quality/surface-finish.md §1):
//!
//! Two passes of one tool over a plane, with contact points `d` apart *along the plane* in the
//! section across the feed. When that section holds the tool axis (waterline passes, and raster
//! passes fed along the contour), it is the tool's own profile: a convex curve. The cusp is the
//! height above the plane where the two copies of that curve meet.
//!
//! - Ball, radius R: the section is a circle in every plane through its centre, so on a plane of
//!   any slope and for any feed direction h = R − √(R² − d²/4) = a²/(R + √(R² − a²)), a = d/2.
//!   This is exact. Two bounds without a square root follow (S1), d²/(8R) ≤ h ≤ d²R/(8R² − d²);
//!   the common d²/(8R) is a lower bound and therefore never a safe estimate.
//! - Bull nose (flat radius R_f, corner r) on a floor normal to its axis: h = 0 for d ≤ 2R_f,
//!   else r − √(r² − (d/2 − R_f)²). Exact.
//! - Bull nose on a slope θ > 0 (axial section): both contacts are on the corner arc, and the
//!   cusp is at most that of a ball of radius r. It equals it when the meeting point lies on the
//!   arc; when it would lie past the arc's ends, the profile there is the flat or the side, each
//!   outside the arc's circle, so the true cusp is smaller. An upper bound.
//! - Flat end on a slope θ: each pass leaves a tread. With contact spacing d along the plane the
//!   deviation normal to the plane is d·sin θ·cos θ (= s_xy·sin θ for horizontal spacing s_xy).
//!   Exact while the tread is narrower than the tool (d·cos θ ≤ 2R).
//!
//! From the strategy's spacing to `d`: raster passes with horizontal stepover s_xy on slope θ
//! give d = s_xy / cos θ; waterline passes with step-down d_z give d = d_z / sin θ.
//!
//! On a curved section (radius ρ, convex or concave), a ball's cusp is the circle-on-circle
//! formula [`ball_on_circle`], exact.

use forge_cam_geom::ival::Ival;
use forge_cam_geom::tool::Cutter;

/// The contact configuration across the feed, which fixes the cusp formula.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Contact {
    /// A ball of radius r, on a plane of any slope, any feed direction. Exact.
    Ball { r: f64 },
    /// A bull nose (flat radius rf ≥ 0, corner radius r) on a floor normal to its axis. Exact.
    BullFloor { rf: f64, r: f64 },
    /// A bull nose's corner (radius r) on a slope, axial section. An upper bound.
    BullSlope { r: f64 },
    /// A flat end of radius r on a slope with the given sin θ and cos θ (exact reals given as
    /// enclosures). Exact while d·cos θ ≤ 2r.
    FlatSlope { r: f64, sin: Ival, cos: Ival },
}

/// Why a cusp or stepover could not be given.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScallopError {
    /// A dimension is not a positive finite number (or a slope not in its range).
    Bad(&'static str),
    /// The target cusp is reached only at stepover 0 (a flat end on a floor leaves none at any
    /// stepover below its diameter; a target ≤ 0 has no positive stepover).
    NoStep,
}

impl Contact {
    /// The contact for a cutter on a plane, from the slope (radians would need a libm; the
    /// caller gives sin θ and cos θ as enclosures, θ ∈ [0, π/2]). A slope of exactly 0 (sin
    /// θ = 0) is the floor case.
    pub fn of(c: &Cutter, sin: Ival, cos: Ival) -> Result<Contact, ScallopError> {
        if !(sin.lo >= 0.0 && cos.lo >= 0.0 && sin.hi <= 1.0 && cos.hi <= 1.0) {
            return Err(ScallopError::Bad("slope"));
        }
        let floor = sin.hi == 0.0;
        match *c {
            Cutter::Ball { d } => {
                if !(d > 0.0 && d.is_finite()) {
                    return Err(ScallopError::Bad("d"));
                }
                Ok(Contact::Ball { r: 0.5 * d })
            }
            Cutter::Bull { d, re } => {
                if !(d > 0.0 && d.is_finite() && re > 0.0 && re <= 0.5 * d) {
                    return Err(ScallopError::Bad("re"));
                }
                if floor {
                    // 0.5·d − re is exact when both are exact halves; enclose it anyway
                    Ok(Contact::BullFloor {
                        rf: Ival::pt(0.5 * d).sub(Ival::pt(re)).lo.max(0.0),
                        r: re,
                    })
                } else {
                    Ok(Contact::BullSlope { r: re })
                }
            }
            Cutter::Flat { d } => {
                if !(d > 0.0 && d.is_finite()) {
                    return Err(ScallopError::Bad("d"));
                }
                Ok(Contact::FlatSlope { r: 0.5 * d, sin, cos })
            }
            Cutter::Chamfer { .. } | Cutter::Drill { .. } => Err(ScallopError::Bad("cutter")),
        }
    }

    /// Whether [`cusp`] is the exact cusp (otherwise an upper bound).
    pub fn exact(&self) -> bool {
        !matches!(self, Contact::BullSlope { .. })
    }
}

/// The cusp of a circle of radius r with spacing d: a²/(r + √(r² − a²)), a = d/2 (the form
/// without cancellation). Unbounded above once a ≥ r might hold: the passes need not meet.
fn circle(r: Ival, d: Ival) -> Ival {
    let a = d.scale(0.5);
    let disc = r.sqr().sub(a.sqr());
    if !(disc.lo > 0.0) {
        // the passes may not meet: the cusp is at least r (where they would just touch) only
        // when a ≥ r; we claim no upper bound at all
        return Ival {
            lo: 0.0,
            hi: f64::INFINITY,
        };
    }
    a.sqr().div(r.add(disc.sqrt()))
}

/// The cusp height for contact spacing `d` along the plane, as an enclosure of the exact value
/// (or of an upper bound, see [`Contact::exact`]). `hi` is infinite when the passes may not meet.
pub fn cusp(c: Contact, d: Ival) -> Ival {
    let d = d.max(Ival::pt(0.0));
    match c {
        Contact::Ball { r } | Contact::BullSlope { r } => circle(Ival::pt(r), d),
        Contact::BullFloor { rf, r } => {
            // the flats overlap while d ≤ 2 rf
            let e = d.sub(Ival::pt(rf).scale(2.0));
            if e.hi <= 0.0 {
                return Ival::pt(0.0);
            }
            let h = circle(Ival::pt(r), e.max(Ival::pt(0.0)));
            if e.lo < 0.0 { h.hull(Ival::pt(0.0)) } else { h }
        }
        Contact::FlatSlope { r, sin, cos } => {
            // exact while the tread d cos θ is within the diameter
            let tread = d.mul(cos);
            if !(tread.hi <= 2.0 * r) {
                return Ival {
                    lo: 0.0,
                    hi: f64::INFINITY,
                };
            }
            d.mul(sin).mul(cos)
        }
    }
}

/// S1: the bounds d²/(8R) ≤ h ≤ d²R/(8R² − d²) for a ball of radius R (d < 2√2 R for the upper
/// one), as enclosures of the two bounds.
pub fn ball_bounds(r: f64, d: f64) -> (Ival, Ival) {
    let (r, d) = (Ival::pt(r), Ival::pt(d));
    let d2 = d.sqr();
    let lo = d2.div(r.scale(8.0));
    let hi = d2.mul(r).div(r.sqr().scale(8.0).sub(d2));
    (lo, hi)
}

/// The largest spacing `d` (to within a few ulps, rounded down) whose cusp is *proven* at most
/// `h`: `cusp(c, d).hi ≤ h`. `max_d` caps it (e.g. the tool diameter, or what the strategy
/// allows). The closed-form inverse gives the candidate; it is then stepped down until the
/// enclosure certifies it, so the answer never relies on the inverse's rounding.
pub fn step_for(c: Contact, h: f64, max_d: f64) -> Result<f64, ScallopError> {
    if !(h > 0.0 && h.is_finite()) {
        return Err(ScallopError::NoStep);
    }
    if !(max_d > 0.0) {
        return Err(ScallopError::Bad("max_d"));
    }
    let cand = match c {
        // d = 2√(h(2r − h)), or 2r when h ≥ r (the passes then meet at height ≤ r: capped below)
        Contact::Ball { r } | Contact::BullSlope { r } => {
            if h >= r {
                2.0 * r
            } else {
                2.0 * (h * (2.0 * r - h)).sqrt()
            }
        }
        Contact::BullFloor { rf, r } => {
            let e = if h >= r {
                2.0 * r
            } else {
                2.0 * (h * (2.0 * r - h)).sqrt()
            };
            2.0 * rf + e
        }
        Contact::FlatSlope { r, sin, cos } => {
            let k = sin.mul(cos);
            if k.hi <= 0.0 {
                // a floor: no cusp below the diameter
                2.0 * r
            } else {
                (h / k.hi).min(2.0 * r / cos.hi.max(f64::MIN_POSITIVE))
            }
        }
    };
    let mut d = cand.min(max_d);
    if !(d > 0.0) {
        return Err(ScallopError::NoStep);
    }
    // certify, stepping down geometrically: each round shrinks d by 2^-40 relative first, then
    // faster; 200 rounds reach any d the enclosure can prove
    let mut k = 0;
    let mut shrink = 1.0 - 1.0 / 1099511627776.0;
    while k < 200 {
        let e = cusp(c, Ival::pt(d));
        if e.hi <= h {
            return Ok(d);
        }
        d *= shrink;
        shrink = shrink * shrink;
        k += 1;
    }
    Err(ScallopError::NoStep)
}

/// The cusp a ball of radius `r` leaves between two passes whose centres are an angle `phi`
/// apart about the centre of a section circle of radius `rho` (contact arc length rho·phi);
/// `convex` when the tool is outside the circle. Exact (surface-finish.md §1.4):
///   convex:  d = (ρ+R)cos(φ/2) − √(R² − (ρ+R)²sin²(φ/2)),  h = d − ρ
///   concave: d = (ρ−R)cos(φ/2) + √(R² − (ρ−R)²sin²(φ/2)),  h = ρ − d   (needs ρ > R)
/// The half-angle's sine and cosine are given as enclosures (`s2`, `c2`).
pub fn ball_on_circle(r: f64, rho: f64, convex: bool, s2: Ival, c2: Ival) -> Ival {
    let (ri, p) = (Ival::pt(r), Ival::pt(rho));
    let unbounded = Ival {
        lo: 0.0,
        hi: f64::INFINITY,
    };
    if !(r > 0.0 && rho > 0.0) || (!convex && !(rho > r)) {
        return unbounded;
    }
    let k = if convex { p.add(ri) } else { p.sub(ri) };
    let disc = ri.sqr().sub(k.sqr().mul(s2.sqr()));
    if !(disc.lo > 0.0) {
        return unbounded;
    }
    if convex {
        k.mul(c2).sub(disc.sqrt()).sub(p).max(Ival::pt(0.0))
    } else {
        p.sub(k.mul(c2).add(disc.sqrt())).max(Ival::pt(0.0))
    }
}

/// The horizontal raster stepover that gives contact spacing at most `d` on slopes up to θ
/// (cos θ given as an enclosure): s_xy = d·cos θ, rounded down.
pub fn raster_step(d: f64, cos: Ival) -> f64 {
    Ival::pt(d).mul(Ival::pt(cos.lo)).lo.max(0.0)
}

/// The waterline step-down that gives contact spacing at most `d` on slopes down to θ (sin θ
/// given as an enclosure): d_z = d·sin θ, rounded down.
pub fn waterline_step(d: f64, sin: Ival) -> f64 {
    Ival::pt(d).mul(Ival::pt(sin.lo)).lo.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::dmath::{cos, sin};

    /// Brute-force cusp of a convex profile y = f(x) (lowest point at x = 0): the maximum over
    /// x ∈ [0, d] of min(f(x), f(x − d)).
    fn brute(f: &dyn Fn(f64) -> f64, d: f64) -> f64 {
        let n = 200_000;
        let mut best: f64 = 0.0;
        let mut i = 0;
        while i <= n {
            let x = d * (i as f64) / (n as f64);
            let v = f(x).min(f(x - d));
            if v > best {
                best = v;
            }
            i += 1;
        }
        best
    }

    fn ball_profile(r: f64) -> impl Fn(f64) -> f64 {
        move |x: f64| {
            if x.abs() >= r {
                r
            } else {
                r - (r * r - x * x).sqrt()
            }
        }
    }

    #[test]
    fn ball_example_from_the_note() {
        // R = 3 mm, s = 0.3 mm: 3.752 µm exact, 3.750 µm by s²/8R
        let h = cusp(Contact::Ball { r: 3.0 }, Ival::pt(0.3));
        assert!(h.lo > 3.7515e-3 && h.hi < 3.7525e-3, "{h:?}");
        let (lo, hi) = ball_bounds(3.0, 0.3);
        assert!(lo.hi <= h.hi && h.lo <= hi.hi);
        assert!((lo.lo - 3.75e-3).abs() < 1e-12);
    }

    #[test]
    fn s1_bounds_hold() {
        let mut r = 0.25;
        while r < 13.0 {
            let mut k = 1;
            while k < 60 {
                let d = 2.0 * r * (k as f64) / 61.0;
                let h = cusp(Contact::Ball { r }, Ival::pt(d));
                let (lo, hi) = ball_bounds(r, d);
                assert!(lo.lo <= h.hi, "lower bound r={r} d={d}");
                assert!(h.lo <= hi.hi, "upper bound r={r} d={d}");
                k += 1;
            }
            r *= 1.7;
        }
    }

    #[test]
    fn ball_matches_brute_force() {
        for &(r, d) in &[(3.0, 0.3), (1.0, 1.2), (6.0, 2.5), (0.5, 0.05)] {
            let f = ball_profile(r);
            let b = brute(&f, d);
            let h = cusp(Contact::Ball { r }, Ival::pt(d));
            assert!(h.lo - 1e-9 <= b && b <= h.hi + 1e-9, "r={r} d={d} b={b} h={h:?}");
        }
    }

    #[test]
    fn bull_floor_matches_brute_force() {
        let (rf, r) = (2.0, 1.0);
        let f = move |x: f64| {
            let e = x.abs() - rf;
            if e <= 0.0 {
                0.0
            } else if e >= r {
                r
            } else {
                r - (r * r - e * e).sqrt()
            }
        };
        for &d in &[1.0, 4.0, 4.5, 5.5] {
            let b = brute(&f, d);
            let h = cusp(Contact::BullFloor { rf, r }, Ival::pt(d));
            assert!(h.lo - 1e-9 <= b && b <= h.hi + 1e-9, "d={d} b={b} h={h:?}");
        }
    }

    #[test]
    fn bull_slope_is_an_upper_bound() {
        // bull D=10, re=1 on slopes from 1° to 89°: rotate its axial section and brute-force
        let (rf, r) = (4.0, 1.0);
        let mut deg = 1.0;
        while deg < 90.0 {
            let t = deg * std::f64::consts::PI / 180.0;
            let (s, c) = (sin(t), cos(t));
            // lower boundary of the rotated section, sampled from its boundary points
            let mut pts = Vec::new();
            let n = 40000;
            let mut i = 0;
            while i <= n {
                let u = (i as f64) / (n as f64);
                // flat from -rf to rf at y = 0
                pts.push((-rf + 2.0 * rf * u, 0.0));
                // arcs about (±rf, r), lower quarter
                let a = u * std::f64::consts::FRAC_PI_2;
                pts.push((rf + r * sin(a), r - r * cos(a)));
                pts.push((-rf - r * sin(a), r - r * cos(a)));
                // sides up to 3r
                pts.push((rf + r, r + 2.0 * r * u));
                pts.push((-rf - r, r + 2.0 * r * u));
                i += 1;
            }
            // rotate so the plane of slope θ is horizontal
            let rot: Vec<(f64, f64)> = pts
                .iter()
                .map(|&(x, y)| (c * x + s * y, -s * x + c * y))
                .collect();
            let ymin = rot.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
            let xmin = rot.iter().find(|p| p.1 == ymin).unwrap().0;
            let bins = 20000;
            let span = 2.0 * (rf + r);
            let mut low = vec![f64::INFINITY; bins + 1];
            for &(x, y) in &rot {
                let k = (((x - xmin) / span + 0.5) * bins as f64).round();
                if k >= 0.0 && (k as usize) <= bins {
                    let k = k as usize;
                    low[k] = low[k].min(y - ymin);
                }
            }
            // fill bins no sample fell in from their neighbours (the shape is convex)
            let mut j = 1;
            while j <= bins {
                if !low[j].is_finite() && low[j - 1].is_finite() && j < bins && low[j + 1].is_finite() {
                    low[j] = 0.5 * (low[j - 1] + low[j + 1]);
                }
                j += 1;
            }
            let f = |x: f64| {
                let k = ((x / span + 0.5) * bins as f64).round();
                if k < 0.0 || k as usize > bins {
                    return 3.0 * r;
                }
                let v = low[k as usize];
                if v.is_finite() { v } else { 3.0 * r }
            };
            for &d in &[0.2, 0.6, 1.2] {
                let b = brute(&f, d);
                let h = cusp(Contact::BullSlope { r }, Ival::pt(d));
                assert!(b <= h.hi + 2e-3, "deg={deg} d={d} b={b} h={h:?}");
            }
            deg += 7.0;
        }
    }

    #[test]
    fn flat_on_a_slope() {
        let t: f64 = 30.0 * std::f64::consts::PI / 180.0;
        let c = Contact::FlatSlope {
            r: 5.0,
            sin: Ival::pt(sin(t)),
            cos: Ival::pt(cos(t)),
        };
        let sxy = 0.5;
        let d = sxy / cos(t);
        let h = cusp(c, Ival::pt(d));
        assert!((h.mid() - sxy * sin(t)).abs() < 1e-12);
    }

    #[test]
    fn step_for_is_certified_and_tight() {
        let cases = [
            Contact::Ball { r: 3.0 },
            Contact::Ball { r: 0.5 },
            Contact::BullFloor { rf: 4.0, r: 1.0 },
            Contact::BullSlope { r: 1.0 },
            Contact::FlatSlope {
                r: 5.0,
                sin: Ival::pt(0.5),
                cos: Ival::pt(0.8660254037844386),
            },
        ];
        for c in cases {
            for &h in &[1e-4, 1e-3, 5e-3, 0.02] {
                let d = step_for(c, h, 100.0).unwrap();
                assert!(cusp(c, Ival::pt(d)).hi <= h, "{c:?} h={h}");
                // tight: 1 ppm more spacing is not provable or exceeds h
                let more = d * (1.0 + 1e-6);
                assert!(
                    cusp(c, Ival::pt(more)).lo > h || more > 2.0 * 6.0,
                    "{c:?} h={h} d={d}"
                );
            }
        }
        assert_eq!(
            step_for(Contact::Ball { r: 1.0 }, 0.0, 1.0),
            Err(ScallopError::NoStep)
        );
    }

    #[test]
    fn circle_on_circle_tends_to_the_plane() {
        // a huge section radius: the cusp tends to the flat-plane value for the same arc length
        let (r, rho): (f64, f64) = (3.0, 1e6);
        let d: f64 = 0.3;
        let phi = d / rho;
        let s2 = Ival::pt(sin(phi / 2.0));
        let c2 = Ival::pt(cos(phi / 2.0));
        let flat = cusp(Contact::Ball { r }, Ival::pt(d)).mid();
        let vex = ball_on_circle(r, rho, true, s2, c2).mid();
        let cave = ball_on_circle(r, rho, false, s2, c2).mid();
        assert!((vex - flat).abs() < 1e-7 && (cave - flat).abs() < 1e-7);
        // convex needs more cusp than flat, concave less (second-order: (1/ρ ± 1/R) d²/8)
        let (rho, d): (f64, f64) = (10.0, 0.5);
        let phi = d / rho;
        let s2 = Ival::pt(sin(phi / 2.0));
        let c2 = Ival::pt(cos(phi / 2.0));
        let flat = cusp(Contact::Ball { r }, Ival::pt(d)).mid();
        assert!(ball_on_circle(r, rho, true, s2, c2).mid() > flat);
        assert!(ball_on_circle(r, rho, false, s2, c2).mid() < flat);
    }

    #[test]
    fn spacing_conversions() {
        let c = Ival::pt(0.5);
        assert!(raster_step(1.0, c) <= 0.5);
        assert!(waterline_step(1.0, Ival::pt(0.25)) <= 0.25);
    }
}
