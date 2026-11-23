//! The fold's arithmetic, op for op as forge-sheet's (shop.rs `setback`), with the sine and cosine
//! passed in: the caller computes them with `forge_math::dmath::sin_cos`, whose error contract
//! is verify/ieee754's `Dmath.sinCos_err` (within 2⁻⁵² of `sin`, `cos` for finite `|θ| ≤ 2²⁰`).
//! Kept free of forge-math so the crate stays dependency-free and its translation complete.

/// The setback from mold line to tangent line on a skin of radius `rho`, from the turn's sine
/// `s` and cosine `c`: `rho s / (1 + c)`, which is `rho tan(θ / 2)` (shop.rs `setback`, op for op).
pub fn setback(rho: f64, s: f64, c: f64) -> f64 {
    rho * s / (1.0 + c)
}
