//! Prints near-degenerate inputs and the kernel's `orient2d` and `cross_sign` answers, one case
//! per line in hex, for `../check_exact.py` to compare with exact rational arithmetic. Most
//! cases fail the fast test and go through the kernel's adaptive and exact paths.
//!
//! `cargo run --release --example dump -- N SCALE_LO SCALE_HI`
use forge_math::predicates::{cross_sign, orient2d};
use forge_math::Point2;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: u64 = args.get(1).map_or(100_000, |s| s.parse().unwrap());
    let lo: i32 = args.get(2).map_or(-40, |s| s.parse().unwrap());
    let hi: i32 = args.get(3).map_or(40, |s| s.parse().unwrap());
    let mut s = 0x2545F4914F6CDD1Du64;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    let mut unit = |next: &mut dyn FnMut() -> u64| (next() >> 11) as f64 / (1u64 << 53) as f64;
    for _ in 0..n {
        let scale = 2f64.powi(lo + (next() % (hi - lo + 1) as u64) as i32);
        let a = Point2::new((unit(&mut next) - 0.5) * scale, (unit(&mut next) - 0.5) * scale);
        let b = Point2::new((unit(&mut next) - 0.5) * scale, (unit(&mut next) - 0.5) * scale);
        // c on the line through a and b, rounded, then nudged by a few ulps
        let t = match next() % 4 {
            0 => unit(&mut next),
            1 => (next() % 9) as f64 / 8.0,
            2 => 2f64.powi(-((next() % 60) as i32)),
            _ => 1.0 + unit(&mut next) * 1e3,
        };
        let mut c = Point2::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y));
        for _ in 0..(next() % 3) {
            let k = (next() % 5) as i64 - 2;
            if next() % 2 == 0 {
                c.x = f64::from_bits((c.x.to_bits() as i64 + k) as u64);
            } else {
                c.y = f64::from_bits((c.y.to_bits() as i64 + k) as u64);
            }
        }
        let o = orient2d(a, b, c);
        // cross_sign on products that nearly cancel
        let (p, q) = ((unit(&mut next) - 0.5) * scale, (unit(&mut next) - 0.5) * scale);
        let r = p * (1.0 + (next() % 5) as f64 * f64::EPSILON);
        let w = q * (1.0 + ((next() % 5) as f64 - 2.0) * f64::EPSILON);
        let x = cross_sign(p, q, r, w);
        println!(
            "{:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x}",
            a.x.to_bits(), a.y.to_bits(), b.x.to_bits(), b.y.to_bits(), c.x.to_bits(), c.y.to_bits(),
            o.to_bits(), p.to_bits(), q.to_bits(), r.to_bits(), w.to_bits(), x.to_bits()
        );
    }
}
