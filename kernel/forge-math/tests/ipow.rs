//! `ipow`: the same bits folded as at run time, and the run-time bits `f64::powi` gave (so every
//! call that did not fold keeps its value). Run in release too (`cargo test --release`): only an
//! optimised build folds, which is where `f64::powi` took the host's `std::pow`.
#![allow(clippy::disallowed_methods)] // f64::powi: the run-time reference

use forge_math::ipow::{Powi, powi, powi_f32};
use std::hint::black_box;

/// Each `(x, n)` written as constants (foldable) and through `black_box` (never folded).
macro_rules! both {
    ($(($x:literal, $n:expr)),* $(,)?) => {
        [$(($x as f64, $n, ($x as f64).ipow($n), black_box($x as f64).ipow(black_box($n)))),*]
    };
}

#[test]
fn folded_is_run_time() {
    // 95.65^3: f64::powi folded to ...918 with glibc's pow at -O, ...917 at run time
    assert_eq!(95.65f64.ipow(3).to_bits(), 0x412ab4acdfced917);
    assert_eq!(
        black_box(95.65f64).ipow(black_box(3)).to_bits(),
        0x412ab4acdfced917
    );
    for (x, n, folded, run) in both![
        (95.65, 3),
        (1.1, 7),
        (24.15, 2),
        (20.95, 2),
        (0.3, 5),
        (3.7, 6),
        (2.5, -3),
        (7.25, 4),
        (1.0001, 100),
        (0.1, 9),
        (10.0, -7),
        (6.02, 23),
        (9.81, 2),
        (25.4, 3),
        (0.73, 3),
        (1.5, -11),
    ] {
        assert_eq!(
            folded.to_bits(),
            run.to_bits(),
            "{x}^{n}: folded {folded:e}, run {run:e}"
        );
        let reference = black_box(x).powi(black_box(n));
        assert_eq!(
            run.to_bits(),
            reference.to_bits(),
            "{x}^{n}: f64::powi at run time {reference:e}"
        );
    }
}

#[test]
fn the_run_time_bits_of_f64_powi() {
    // a deterministic spread of bases and every exponent -40..=40, through black_box: f64::powi's
    // run-time path (__powidf2, or LLVM's expansion) is what every unfolded call had
    let mut s = 0x9e37_79b9_7f4a_7c15u64;
    for _ in 0..20_000 {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        let x = (s >> 11) as f64 / (1u64 << 53) as f64 * 200.0 - 100.0;
        for n in -40..=40 {
            let want = black_box(x).powi(black_box(n));
            let got = powi(black_box(x), black_box(n));
            assert!(
                want.to_bits() == got.to_bits() || (want.is_nan() && got.is_nan()),
                "{x}^{n}: {want:e} vs {got:e}"
            );
            let (xf, wf) = (x as f32, black_box(x as f32).powi(black_box(n)));
            let gf = powi_f32(black_box(xf), black_box(n));
            assert!(
                wf.to_bits() == gf.to_bits() || (wf.is_nan() && gf.is_nan()),
                "{xf}^{n} f32"
            );
        }
    }
    for (x, n) in [
        (0.0, -1),
        (-0.0, -3),
        (f64::INFINITY, 2),
        (f64::NAN, 0),
        (2.0, i32::MIN),
        (0.5, i32::MAX),
    ] {
        let (want, got) = (black_box(x).powi(black_box(n)), powi(black_box(x), black_box(n)));
        assert!(
            want.to_bits() == got.to_bits() || (want.is_nan() && got.is_nan()),
            "{x}^{n}"
        );
    }
}
