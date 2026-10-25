//! Transcendental functions built only from IEEE `+ - * /` and `sqrt`, which
//! round identically on every target. The platform's `sin`, `cos` and friends
//! do not (glibc, macOS and the wasm runtime each have their own), so the kernel
//! must never call them. Algorithms and coefficients follow FreeBSD's msun
//! (fdlibm); accuracy is within about 1 ulp.

#![allow(clippy::excessive_precision, clippy::approx_constant)]

pub const PI: f64 = core::f64::consts::PI;
pub const TAU: f64 = core::f64::consts::TAU;
pub const FRAC_PI_2: f64 = core::f64::consts::FRAC_PI_2;

const TOINT: f64 = 1.5 / f64::EPSILON;
const INVPIO2: f64 = 6.366_197_723_675_813_824_33e-1;
const PIO2_1: f64 = 1.570_796_326_734_125_614_17;
const PIO2_1T: f64 = 6.077_100_506_506_192_249_32e-11;
const PIO2_2: f64 = 6.077_100_506_303_965_976_60e-11;
const PIO2_2T: f64 = 2.022_266_248_795_950_631_54e-21;
const PIO2_3: f64 = 2.022_266_248_711_166_455_80e-21;
const PIO2_3T: f64 = 8.478_427_660_368_899_569_97e-32;

fn exponent(x: f64) -> i32 {
    ((x.to_bits() >> 52) & 0x7ff) as i32
}

/// x = n * pi/2 + (y0 + y1). Cody-Waite with three-part pi/2: exact enough for
/// |x| up to ~1e6, the range a parameter angle can reach. Larger inputs still
/// give a deterministic (but less accurate) answer.
fn rem_pio2(x: f64) -> (i32, f64, f64) {
    let f = x * INVPIO2 + TOINT - TOINT;
    let n = f as i32;
    let mut r = x - f * PIO2_1;
    let mut w = f * PIO2_1T;
    let mut y0 = r - w;
    let ex = exponent(x);
    if ex - exponent(y0) > 16 {
        let t = r;
        w = f * PIO2_2;
        r = t - w;
        w = f * PIO2_2T - ((t - r) - w);
        y0 = r - w;
        if ex - exponent(y0) > 49 {
            let t = r;
            w = f * PIO2_3;
            r = t - w;
            w = f * PIO2_3T - ((t - r) - w);
            y0 = r - w;
        }
    }
    let y1 = (r - y0) - w;
    (n, y0, y1)
}

fn k_sin(x: f64, y: f64) -> f64 {
    const S1: f64 = -1.666_666_666_666_663_243_48e-1;
    const S2: f64 = 8.333_333_333_322_489_461_24e-3;
    const S3: f64 = -1.984_126_982_985_794_931_34e-4;
    const S4: f64 = 2.755_731_370_707_006_767_89e-6;
    const S5: f64 = -2.505_076_025_340_686_341_95e-8;
    const S6: f64 = 1.589_690_995_211_550_102_21e-10;
    let z = x * x;
    let w = z * z;
    let r = S2 + z * (S3 + z * S4) + z * w * (S5 + z * S6);
    let v = z * x;
    x - ((z * (0.5 * y - v * r) - y) - v * S1)
}

fn k_cos(x: f64, y: f64) -> f64 {
    const C1: f64 = 4.166_666_666_666_660_190_37e-2;
    const C2: f64 = -1.388_888_888_887_410_957_49e-3;
    const C3: f64 = 2.480_158_728_947_672_941_78e-5;
    const C4: f64 = -2.755_731_435_139_066_330_35e-7;
    const C5: f64 = 2.087_572_321_298_174_827_90e-9;
    const C6: f64 = -1.135_964_755_778_819_482_65e-11;
    let z = x * x;
    let w = z * z;
    let r = z * (C1 + z * (C2 + z * C3)) + w * w * (C4 + z * (C5 + z * C6));
    let hz = 0.5 * z;
    let w = 1.0 - hz;
    w + (((1.0 - w) - hz) + (z * r - x * y))
}

pub fn sin_cos(x: f64) -> (f64, f64) {
    if !x.is_finite() {
        return (f64::NAN, f64::NAN);
    }
    if x.abs() < 1.0 / 134_217_728.0 {
        return (x, 1.0);
    }
    let (n, y0, y1) = rem_pio2(x);
    let (s, c) = (k_sin(y0, y1), k_cos(y0, y1));
    match n & 3 {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    }
}

pub fn sin(x: f64) -> f64 {
    sin_cos(x).0
}

pub fn cos(x: f64) -> f64 {
    sin_cos(x).1
}

pub fn atan(x: f64) -> f64 {
    const ATANHI: [f64; 4] = [
        4.636_476_090_008_060_935_15e-1,
        7.853_981_633_974_482_789_99e-1,
        9.827_937_232_473_290_540_82e-1,
        1.570_796_326_794_896_558_00,
    ];
    const ATANLO: [f64; 4] = [
        2.269_877_745_296_168_709_24e-17,
        3.061_616_997_868_383_017_93e-17,
        1.390_331_103_123_099_845_16e-17,
        6.123_233_995_736_766_035_87e-17,
    ];
    const AT: [f64; 11] = [
        3.333_333_333_333_293_180_27e-1,
        -1.999_999_999_987_648_324_76e-1,
        1.428_571_427_250_346_637_11e-1,
        -1.111_111_040_546_235_578_80e-1,
        9.090_887_133_436_506_561_96e-2,
        -7.691_876_205_044_829_994_95e-2,
        6.661_073_137_387_531_206_69e-2,
        -5.833_570_133_790_573_486_45e-2,
        4.976_877_994_615_932_360_17e-2,
        -3.653_157_274_421_691_552_70e-2,
        1.628_582_011_536_578_236_23e-2,
    ];
    if x.is_nan() {
        return x;
    }
    let neg = x.is_sign_negative();
    let mut ax = x.abs();
    if ax >= 7.378_697_629_483_820_6e19 {
        let z = ATANHI[3] + ATANLO[3];
        return if neg { -z } else { z };
    }
    let id: i32;
    if ax < 0.4375 {
        if ax < 7.450_580_596_923_828e-9 {
            return x;
        }
        id = -1;
    } else if ax < 1.1875 {
        if ax < 0.6875 {
            id = 0;
            ax = (2.0 * ax - 1.0) / (2.0 + ax);
        } else {
            id = 1;
            ax = (ax - 1.0) / (ax + 1.0);
        }
    } else if ax < 2.4375 {
        id = 2;
        ax = (ax - 1.5) / (1.0 + 1.5 * ax);
    } else {
        id = 3;
        ax = -1.0 / ax;
    }
    let z = ax * ax;
    let w = z * z;
    let s1 = z * (AT[0] + w * (AT[2] + w * (AT[4] + w * (AT[6] + w * (AT[8] + w * AT[10])))));
    let s2 = w * (AT[1] + w * (AT[3] + w * (AT[5] + w * (AT[7] + w * AT[9]))));
    if id < 0 {
        let r = ax - ax * (s1 + s2);
        return if neg { -r } else { r };
    }
    let i = id as usize;
    let r = ATANHI[i] - ((ax * (s1 + s2) - ATANLO[i]) - ax);
    if neg { -r } else { r }
}

pub fn atan2(y: f64, x: f64) -> f64 {
    const PI_LO: f64 = 1.224_646_799_147_353_177_2e-16;
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    if y == 0.0 {
        return if x.is_sign_negative() {
            if y.is_sign_negative() { -PI } else { PI }
        } else {
            y
        };
    }
    if x == 0.0 {
        return if y < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 };
    }
    if x.is_infinite() {
        let q = if y.is_infinite() {
            if x > 0.0 { PI / 4.0 } else { 3.0 * PI / 4.0 }
        } else if x > 0.0 {
            0.0
        } else {
            PI
        };
        return if y < 0.0 { -q } else { q };
    }
    if y.is_infinite() {
        return if y < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 };
    }
    let k = exponent(y) - exponent(x);
    let z = if k > 60 {
        FRAC_PI_2 + 0.5 * PI_LO
    } else if x < 0.0 && k < -60 {
        0.0
    } else {
        atan((y / x).abs())
    };
    match (x < 0.0, y < 0.0) {
        (false, false) => z,
        (false, true) => -z,
        (true, false) => PI - (z - PI_LO),
        (true, true) => (z - PI_LO) - PI,
    }
}

/// NaN outside [-1, 1].
pub fn asin(x: f64) -> f64 {
    if !(-1.0..=1.0).contains(&x) {
        return f64::NAN;
    }
    atan2(x, ((1.0 - x) * (1.0 + x)).sqrt())
}

/// NaN outside [-1, 1].
pub fn acos(x: f64) -> f64 {
    if !(-1.0..=1.0).contains(&x) {
        return f64::NAN;
    }
    atan2(((1.0 - x) * (1.0 + x)).sqrt(), x)
}

/// acos with the argument clamped into [-1, 1], for cosines that rounding
/// pushed just past the end.
pub fn acos_clamped(x: f64) -> f64 {
    acos(x.clamp(-1.0, 1.0))
}

pub fn cbrt(x: f64) -> f64 {
    const B1: u32 = 715_094_163;
    const B2: u32 = 696_219_795;
    const P0: f64 = 1.875_951_824_271_770_096_43;
    const P1: f64 = -1.884_979_795_433_771_698_75;
    const P2: f64 = 1.621_429_720_105_354_466_140;
    const P3: f64 = -0.758_397_934_778_766_047_437;
    const P4: f64 = 0.145_996_192_886_612_446_982;
    let bits = x.to_bits();
    let mut hx = ((bits >> 32) as u32) & 0x7fff_ffff;
    if hx >= 0x7ff0_0000 {
        return x + x;
    }
    let mut ubits;
    if hx < 0x0010_0000 {
        ubits = (x * f64::from_bits(0x4350_0000_0000_0000)).to_bits();
        hx = ((ubits >> 32) as u32) & 0x7fff_ffff;
        if hx == 0 {
            return x;
        }
        hx = hx / 3 + B2;
    } else {
        hx = hx / 3 + B1;
    }
    ubits = bits & (1u64 << 63);
    ubits |= (hx as u64) << 32;
    let mut t = f64::from_bits(ubits);
    let r = (t * t) * (t / x);
    t *= (P0 + r * (P1 + r * P2)) + ((r * r) * r) * (P3 + r * P4);
    t = f64::from_bits((t.to_bits().wrapping_add(0x8000_0000)) & 0xffff_ffff_c000_0000);
    let s = t * t;
    let r = x / s;
    let w = t + t;
    let r = (r - t) / (w + r);
    t + t * r
}

/// sin and cos of `start + k * step` for k in 0..n, into `out`. Steps by
/// rotation (two multiplies and adds per angle) and re-seeds from an exact
/// sin_cos every 32 angles, so the error stays within a few ulps.
pub fn sin_cos_steps(start: f64, step: f64, n: usize, out: &mut Vec<(f64, f64)>) {
    out.clear();
    out.reserve(n);
    let (sd, cd) = sin_cos(step);
    let (mut s, mut c) = (0.0, 1.0);
    for k in 0..n {
        if k % 32 == 0 {
            (s, c) = sin_cos(start + step * k as f64);
        } else {
            (s, c) = (s * cd + c * sd, c * cd - s * sd);
        }
        out.push((s, c));
    }
}

/// Angle in [0, 2pi).
pub fn wrap_tau(a: f64) -> f64 {
    let mut r = a - TAU * (a / TAU).floor();
    if r >= TAU {
        r -= TAU;
    }
    if r < 0.0 {
        r = 0.0;
    }
    r
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    fn ulps(a: f64, b: f64) -> u64 {
        if a == b {
            return 0;
        }
        let ia = a.to_bits() as i64;
        let ib = b.to_bits() as i64;
        let ia = if ia < 0 { i64::MIN - ia } else { ia };
        let ib = if ib < 0 { i64::MIN - ib } else { ib };
        ia.abs_diff(ib)
    }

    #[test]
    fn sin_cos_match_platform() {
        let mut rng = Rng::new(1);
        for i in 0..200_000 {
            let x = match i % 3 {
                0 => rng.range(-7.0, 7.0),
                1 => rng.range(-1e4, 1e4),
                _ => rng.range(-1e-3, 1e-3),
            };
            let (s, c) = sin_cos(x);
            assert!(
                ulps(s, x.sin()) <= 1 || (s - x.sin()).abs() < 1e-300,
                "sin {x}: {s} vs {}",
                x.sin()
            );
            assert!(ulps(c, x.cos()) <= 1, "cos {x}: {c} vs {}", x.cos());
        }
        assert_eq!(sin(0.0), 0.0);
        assert_eq!(cos(0.0), 1.0);
        assert!(sin(f64::INFINITY).is_nan());
    }

    #[test]
    fn atan2_matches_platform() {
        let mut rng = Rng::new(2);
        for _ in 0..200_000 {
            let y = rng.range(-1e3, 1e3) * if rng.bool() { 1e-6 } else { 1.0 };
            let x = rng.range(-1e3, 1e3);
            assert!(ulps(atan2(y, x), y.atan2(x)) <= 1, "atan2 {y} {x}");
            assert!(ulps(atan(y), y.atan()) <= 1, "atan {y}");
        }
        for &(y, x) in &[
            (0.0, 1.0),
            (0.0, -1.0),
            (-0.0, -1.0),
            (1.0, 0.0),
            (-1.0, 0.0),
            (0.0, 0.0),
            (0.0, -0.0),
        ] {
            assert_eq!(
                atan2(y, x).to_bits(),
                f64::atan2(y, x).to_bits(),
                "atan2({y},{x})"
            );
        }
    }

    #[test]
    fn asin_acos_cbrt() {
        let mut rng = Rng::new(3);
        for _ in 0..100_000 {
            let x = rng.range(-1.0, 1.0);
            assert!((asin(x) - x.asin()).abs() <= 4.0 * f64::EPSILON, "asin {x}");
            assert!((acos(x) - x.acos()).abs() <= 4.0 * f64::EPSILON * 4.0, "acos {x}");
            let y = rng.range(-1e9, 1e9);
            assert!(ulps(cbrt(y), y.cbrt()) <= 1, "cbrt {y}");
        }
        assert_eq!(cbrt(27.0), 3.0);
        assert!(acos(1.5).is_nan());
        assert_eq!(acos_clamped(1.0 + 1e-15), 0.0);
    }

    #[test]
    fn wrap() {
        assert_eq!(wrap_tau(0.0), 0.0);
        assert!((wrap_tau(-0.5) - (TAU - 0.5)).abs() < 1e-15);
        assert!(wrap_tau(TAU) < 1e-15);
        assert!((wrap_tau(7.0 * TAU + 1.0) - 1.0).abs() < 1e-13);
    }

    #[test]
    fn stepped_angles_stay_accurate() {
        let mut out = Vec::new();
        sin_cos_steps(0.3, 0.017, 1000, &mut out);
        for (k, &(s, c)) in out.iter().enumerate() {
            let a = 0.3 + 0.017 * k as f64;
            assert!((s - a.sin()).abs() < 1e-14 && (c - a.cos()).abs() < 1e-14, "{k}");
        }
    }
}
