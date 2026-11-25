//! `x^n` for an integer `n`, the same bits on every target whether the compiler folds it or not.
//!
//! `f64::powi` lowers to LLVM's `llvm.powi`. At run time that is compiler-builtins' `__powidf2`
//! (or libgcc's or compiler-rt's, all the same multiplies in the same order), or, for a constant
//! `n`, LLVM's own expansion into those multiplies: the same bits on x86-64, aarch64 and wasm32.
//! But when `x` and `n` are both constants after inlining, LLVM folds `llvm.powi` with the build
//! machine's C++ `std::pow`, one rounding of the exact power where the run-time path rounds at each
//! multiply: `95.65f64.powi(3)` is `0x412ab4acdfced918` folded at `-O`, `0x412ab4acdfced917` at
//! run time. Whether a call folds depends on the target's inlining and the host's libm, so bits
//! would too (research/forge-lang/determinism.md).
//!
//! Here the power is ordinary multiplies, written out: LLVM folds an `fmul` exactly as IEEE
//! rounds it, on any host, so a folded result is the run-time one. The order is
//! compiler-builtins' (`src/float/pow.rs`): square-and-multiply from the low bit, `1 / x^|n|` for a
//! negative `n`. So every `powi` that did not fold keeps its bits. `f64::powi` and `f32::powi`
//! are disallowed (clippy.toml): call `x.ipow(n)` through [`Powi`].
//!
//! Not in `dmath`: Charon translates that whole module (verify/aeneas/forge-math), and this one
//! needs no Lean model of its own (it is multiplies).

/// `x^n` as compiler-builtins' `__powidf2` computes it, foldable without changing its bits.
#[inline(always)]
pub fn powi(x: f64, n: i32) -> f64 {
    let m = pow_u(x, n.unsigned_abs());
    if n < 0 { 1.0 / m } else { m }
}

/// `powi` in `f32` (`__powisf2`'s order).
#[inline(always)]
pub fn powi_f32(x: f32, n: i32) -> f32 {
    let m = pow_u32(x, n.unsigned_abs());
    if n < 0 { 1.0 / m } else { m }
}

/// `x^p` for `p >= 0`: square-and-multiply from the low bit, the product starting at 1 (`1 * a`
/// is exact, so the first factor is `a` itself). Written out to 8, as LLVM expanded a constant
/// power: no loop left to unroll where `p` is known.
macro_rules! pow_u {
    ($name:ident, $t:ty) => {
        #[inline(always)]
        fn $name(x: $t, p: u32) -> $t {
            match p {
                0 => 1.0,
                1 => x,
                2 => x * x,
                3 => x * (x * x),
                4 => {
                    let x2 = x * x;
                    x2 * x2
                }
                5 => {
                    let x2 = x * x;
                    x * (x2 * x2)
                }
                6 => {
                    let x2 = x * x;
                    x2 * (x2 * x2)
                }
                7 => {
                    let x2 = x * x;
                    (x * x2) * (x2 * x2)
                }
                8 => {
                    let x2 = x * x;
                    let x4 = x2 * x2;
                    x4 * x4
                }
                _ => {
                    let (mut a, mut p, mut m) = (x, p, 1.0);
                    loop {
                        if p & 1 != 0 {
                            m *= a;
                        }
                        p >>= 1;
                        if p == 0 {
                            break;
                        }
                        a *= a;
                    }
                    m
                }
            }
        }
    };
}
pow_u!(pow_u, f64);
pow_u!(pow_u32, f32);

/// `x.ipow(n)`: [`powi`] as a method, where `x.powi(n)` was.
pub trait Powi {
    fn ipow(self, n: i32) -> Self;
}

impl Powi for f64 {
    #[inline(always)]
    fn ipow(self, n: i32) -> f64 {
        powi(self, n)
    }
}

impl Powi for f32 {
    #[inline(always)]
    fn ipow(self, n: i32) -> f32 {
        powi_f32(self, n)
    }
}
