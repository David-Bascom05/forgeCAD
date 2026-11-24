//! Error-free transformations (Knuth's two-sum, Dekker's split and two-product): the building
//! blocks of Shewchuk's floating-point expansions, and of every exact predicate in the kernel.
//!
//! These are forge-math's predicates.rs functions, the text its L4 proofs are about
//! (verify/aeneas/forge-math: `two_sum_exact`, `split_veltkamp`, `two_product_exact`), moved here
//! unchanged so that forge-math, forge-sheet-core and forge-bool share one copy. forge-math keeps
//! its debug-build precondition check in front of `two_product`. Each is `#[inline]`: they are
//! called across crates, inside the predicates' hot paths.

/// Dekker's splitter, `2²⁷ + 1`.
pub const SPLITTER: f64 = 134_217_729.0; // 2^27 + 1

/// `a + b = x + e` exactly (Knuth), for finite `a`, `b` whose sum does not overflow.
#[inline]
pub fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let x = a + b;
    let bv = x - a;
    let av = x - bv;
    (x, (a - av) + (b - bv))
}

/// `a - b = x + e` exactly.
#[inline]
pub fn two_diff(a: f64, b: f64) -> (f64, f64) {
    let x = a - b;
    let bv = a - x;
    let av = x + bv;
    (x, (a - av) + (bv - b))
}

/// Veltkamp's split of `a` into two halves of its significand, `a = hi + lo`.
#[inline]
pub fn split(a: f64) -> (f64, f64) {
    let c = SPLITTER * a;
    let hi = c - (c - a);
    (hi, a - hi)
}

/// `a · b = x + e` exactly (Dekker; no fused multiply-add), when the product's rounding error is
/// representable (no overflow or underflow).
#[inline]
pub fn two_product(a: f64, b: f64) -> (f64, f64) {
    let x = a * b;
    let (ahi, alo) = split(a);
    let (bhi, blo) = split(b);
    let err = ((x - ahi * bhi) - alo * bhi) - ahi * blo;
    (x, alo * blo - err)
}
