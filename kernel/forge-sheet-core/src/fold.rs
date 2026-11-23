//! The fold's arithmetic, op for op as forge-sheet's (shop.rs `setback`), with the sine and cosine
//! passed in: the caller computes them with `forge_math::dmath::sin_cos`, whose error contract
//! is verify/ieee754's `Dmath.sinCos_err` (within 2⁻⁵² of `sin`, `cos` for finite `|θ| ≤ 2²⁰`).
//! Kept free of forge-math so the crate stays dependency-free and its translation complete.

/// The setback from mold line to tangent line on a skin of radius `rho`, from the turn's sine
/// `s` and cosine `c`: `rho s / (1 + c)`, which is `rho tan(θ / 2)` (shop.rs `setback`, op for op).
pub fn setback(rho: f64, s: f64, c: f64) -> f64 {
    rho * s / (1.0 + c)
}

/// The angle at `u` across a bend (build.rs `phi`, op for op): its far tangent line, `u = ba`,
/// is the bend's angle exactly.
pub fn phi(u: f64, rn: f64, theta: f64, ba: f64) -> f64 {
    if u == ba { theta } else { u / rn }
}

/// A zone point in the bend's axis frame (build.rs `Geo::Round`, op for op): radius
/// `rho0 + sigma w`, at the angle whose sine and cosine are `s`, `c`, at height `-sigma y`. The
/// caller maps it to the world with the axis frame's `point_to_world`.
pub fn round_local(rho0: f64, sigma: f64, s: f64, c: f64, y: f64, w: f64) -> (f64, f64, f64) {
    let r = rho0 + sigma * w;
    (r * c, r * s, -sigma * y)
}
