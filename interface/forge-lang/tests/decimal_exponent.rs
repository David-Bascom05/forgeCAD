//! `units::decimal_exponent`, the leading digit's power of ten that the size-fit suggestion and the
//! taper lint round to: exact at and around every power of ten a double reaches (where a platform
//! libm's `log10` may be an ulp off and flip the floor), so their suggested values are the same
//! bits on every target.

use forge_lang::units::decimal_exponent;

#[test]
fn exact_at_and_around_the_powers_of_ten() {
    for k in -307..=308 {
        let p: f64 = format!("1e{k}").parse().unwrap();
        assert_eq!(decimal_exponent(p), k, "10^{k}");
        assert_eq!(decimal_exponent(-p), k, "-10^{k}");
        assert_eq!(decimal_exponent(p.next_up()), k, "just above 10^{k}");
        assert_eq!(decimal_exponent(p.next_down()), k - 1, "just below 10^{k}");
        // the largest value of each decade a person types: 9.99..
        let nines: f64 = format!("9.999e{}", k - 1).parse().unwrap();
        assert_eq!(decimal_exponent(nines), k - 1, "9.999e{}", k - 1);
    }
}

#[test]
fn zero_and_non_finite_are_zero() {
    for x in [0.0, -0.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(decimal_exponent(x), 0, "{x}");
    }
}

#[test]
fn the_floor_of_log10_away_from_the_powers() {
    // away from a power of ten log10's floor is not in doubt: the two agree
    let mut x = 1.0e-9_f64;
    while x < 1.0e9 {
        for m in [1.5, 2.0, 3.7, 5.0, 7.25, 9.5] {
            let v = x * m;
            #[allow(clippy::disallowed_methods)]
            let want = v.log10().floor() as i32;
            assert_eq!(decimal_exponent(v), want, "{v}");
        }
        x *= 10.0;
    }
}

#[test]
fn the_ceiling_is_the_power_itself_or_one_more() {
    use forge_lang::units::decimal_exponent_ceil;
    for k in -307..=308 {
        let p: f64 = format!("1e{k}").parse().unwrap();
        assert_eq!(decimal_exponent_ceil(p), k, "10^{k}");
        assert_eq!(decimal_exponent_ceil(p.next_up()), k + 1, "just above 10^{k}");
        assert_eq!(decimal_exponent_ceil(p.next_down()), k, "just below 10^{k}");
    }
    for (x, want) in [(0.05, -1), (0.5, 0), (6.0, 1), (12.0, 2), (0.0, 0), (f64::NAN, 0)] {
        assert_eq!(decimal_exponent_ceil(x), want, "{x}");
    }
}
