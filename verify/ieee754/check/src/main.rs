//! Hardware check of the Lean IEEE 754 model (`ieee754-eval`).
//!
//! Generates random and edge-case operands, computes each operation with this CPU's `f64` and
//! `f32`, asks the model for the same operations over a pipe, and compares the answers bit for
//! bit. NaN payloads are not modelled: a NaN result matches any NaN.
//!
//! Usage: `ieee754-check [--exe PATH] [--n PER_OP] [--seed S]`.
//! Exit status 1 if any answer differs.

use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Command, Stdio};

/// xoshiro256** — deterministic, no dependencies.
struct Rng([u64; 4]);
impl Rng {
    fn new(seed: u64) -> Self {
        let mut z = seed;
        let mut s = [0u64; 4];
        for v in &mut s {
            z = z.wrapping_add(0x9e3779b97f4a7c15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
            *v = x ^ (x >> 31);
        }
        Rng(s)
    }
    fn next(&mut self) -> u64 {
        let s = &mut self.0;
        let r = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        r
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn coin(&mut self) -> bool {
        self.next() & 1 == 1
    }
}

/// A binary format's layout, for generating encodings.
#[derive(Clone, Copy)]
struct Fmt {
    tag: &'static str,
    ebits: u32,
    fbits: u32,
}
const D: Fmt = Fmt {
    tag: "d",
    ebits: 11,
    fbits: 52,
};
const S: Fmt = Fmt {
    tag: "s",
    ebits: 8,
    fbits: 23,
};

impl Fmt {
    fn width(self) -> u32 {
        1 + self.ebits + self.fbits
    }
    fn emask(self) -> u64 {
        (1 << self.ebits) - 1
    }
    fn fmask(self) -> u64 {
        (1u64 << self.fbits) - 1
    }
    fn make(self, s: bool, bexp: u64, frac: u64) -> u64 {
        ((s as u64) << (self.ebits + self.fbits))
            | ((bexp & self.emask()) << self.fbits)
            | (frac & self.fmask())
    }
    fn bexp(self, x: u64) -> u64 {
        (x >> self.fbits) & self.emask()
    }
    fn is_nan(self, x: u64) -> bool {
        self.bexp(x) == self.emask() && x & self.fmask() != 0
    }
    /// The special and boundary encodings.
    fn edges(self) -> Vec<u64> {
        let top = self.emask();
        let f = self.fmask();
        let mut v = vec![];
        for s in [false, true] {
            for (e, fr) in [
                (0, 0), // zero
                (0, 1), // least subnormal
                (0, 2),
                (0, f), // greatest subnormal
                (0, f >> 1),
                (1, 0), // least normal
                (1, 1),
                (top - 1, f), // greatest finite
                (top - 1, f - 1),
                (top - 1, 0),
                (top, 0),                                // infinity
                (top, 1 << (self.fbits - 1)),            // quiet NaN
                (top, 1),                                // signalling NaN pattern
                (top >> 1, 0),                           // 1
                (top >> 1, 1),                           // 1 + ulp
                (top >> 1, f),                           // 2 - ulp
                ((top >> 1) - 1, f),                     // 1 - ulp/2
                ((top >> 1) + 1, 0),                     // 2
                ((top >> 1) + 1, 1 << (self.fbits - 1)), // 3
                (top >> 2, 0),
                ((top >> 1) + (top >> 2), 0),
            ] {
                v.push(self.make(s, e, fr));
            }
        }
        v
    }
    /// A random operand: uniform bits, an edge value, or a value whose exponent is drawn from a
    /// region (subnormal, near 1, near overflow) and whose significand is random or sparse.
    fn operand(self, r: &mut Rng, edges: &[u64]) -> u64 {
        let s = r.coin();
        let top = self.emask();
        match r.below(10) {
            0 => edges[r.below(edges.len() as u64) as usize],
            1 | 2 => r.next() & ((1u128 << self.width()) - 1) as u64,
            3 => self.make(s, r.below(3), r.next()), // subnormal / tiny
            4 => self.make(s, top - 1 - r.below(3), r.next()), // near overflow
            5 => self.make(s, (top >> 1) - 3 + r.below(7), r.next()), // near 1
            6 => {
                // short significand: exact products and many ties
                let k = r.below(self.fbits as u64 / 2 + 2);
                let fr = (r.next() & ((1u64 << k) - 1)) << (self.fbits as u64 - k);
                self.make(s, 1 + r.below(top - 1), fr)
            }
            7 => self.make(s, 1 + r.below(top - 1), r.next() | 0x1), // odd significand
            _ => self.make(s, 1 + r.below(top - 1), r.next()),
        }
    }
    /// A second operand related to the first: the same exponent region (cancellation), a
    /// half-ulp-sized partner (ties in addition), or independent.
    fn partner(self, r: &mut Rng, a: u64, edges: &[u64]) -> u64 {
        let ea = self.bexp(a);
        let top = self.emask();
        let s = r.coin();
        match r.below(6) {
            0 => {
                let e = (ea as i64 + r.below(5) as i64 - 2).clamp(0, top as i64 - 1) as u64;
                self.make(s, e, r.next())
            }
            1 => {
                // about ulp(a)/2: exponent fbits+1 lower
                let e = ea as i64 - self.fbits as i64 - 1 + r.below(3) as i64 - 1;
                let e = e.clamp(0, top as i64 - 1) as u64;
                let k = r.below(4);
                self.make(s, e, (r.next() & ((1u64 << k) - 1)) << (self.fbits as u64 - k))
            }
            2 => a ^ (1u64 << (self.ebits + self.fbits)) ^ (r.below(4)), // ≈ -a
            _ => self.operand(r, edges),
        }
    }
}

#[derive(Clone, Copy)]
enum Want {
    Bits(Fmt, u64),
    Bool(bool),
    Int(i64),
}

#[target_feature(enable = "fma")]
fn fma64_hw(a: f64, b: f64, c: f64) -> f64 {
    a.mul_add(b, c)
}
#[target_feature(enable = "fma")]
fn fma32_hw(a: f32, b: f32, c: f32) -> f32 {
    a.mul_add(b, c)
}

/// Whether the exact `x + y` (resp. `x * y`) lies exactly halfway between two doubles: the
/// rounding error (exact by TwoSum / FMA) is half the spacing at the result.
fn is_tie(r: f64, err: f64) -> bool {
    if err == 0.0 || !r.is_finite() || !err.is_finite() {
        return false;
    }
    let a = r.abs();
    let up = a.next_up() - a;
    let down = a - a.next_down();
    2.0 * err.abs() == up || 2.0 * err.abs() == down
}

struct Gen {
    ties: usize,
    queries: Vec<String>,
    wants: Vec<Want>,
    ops: Vec<&'static str>,
    hw_fma: bool,
}

impl Gen {
    fn push(&mut self, q: String, w: Want, op: &'static str) {
        self.queries.push(q);
        self.wants.push(w);
        self.ops.push(op);
    }

    fn binop(&mut self, fm: Fmt, op: &'static str, a: u64, b: u64) {
        let q = format!("{} {} {:x} {:x}", fm.tag, op, a, b);
        let w = if fm.tag == "d" {
            let (x, y) = (f64::from_bits(a), f64::from_bits(b));
            let tie = match op {
                "add" => {
                    let s = x + y;
                    let bb = s - x;
                    is_tie(s, (x - (s - bb)) + (y - bb))
                }
                "mul" => {
                    let p = x * y;
                    is_tie(
                        p,
                        if self.hw_fma {
                            unsafe { fma64_hw(x, y, -p) }
                        } else {
                            x.mul_add(y, -p)
                        },
                    )
                }
                _ => false,
            };
            self.ties += tie as usize;
            match op {
                "add" => Want::Bits(fm, (x + y).to_bits()),
                "sub" => Want::Bits(fm, (x - y).to_bits()),
                "mul" => Want::Bits(fm, (x * y).to_bits()),
                "div" => Want::Bits(fm, (x / y).to_bits()),
                "eq" => Want::Bool(x == y),
                "lt" => Want::Bool(x < y),
                "le" => Want::Bool(x <= y),
                _ => unreachable!(),
            }
        } else {
            let (x, y) = (f32::from_bits(a as u32), f32::from_bits(b as u32));
            match op {
                "add" => Want::Bits(fm, (x + y).to_bits() as u64),
                "sub" => Want::Bits(fm, (x - y).to_bits() as u64),
                "mul" => Want::Bits(fm, (x * y).to_bits() as u64),
                "div" => Want::Bits(fm, (x / y).to_bits() as u64),
                "eq" => Want::Bool(x == y),
                "lt" => Want::Bool(x < y),
                "le" => Want::Bool(x <= y),
                _ => unreachable!(),
            }
        };
        self.push(q, w, op);
    }

    fn fma(&mut self, fm: Fmt, a: u64, b: u64, c: u64) {
        let q = format!("{} fma {:x} {:x} {:x}", fm.tag, a, b, c);
        let w = if fm.tag == "d" {
            let (x, y, z) = (f64::from_bits(a), f64::from_bits(b), f64::from_bits(c));
            let r = if self.hw_fma {
                unsafe { fma64_hw(x, y, z) }
            } else {
                x.mul_add(y, z)
            };
            Want::Bits(fm, r.to_bits())
        } else {
            let (x, y, z) = (
                f32::from_bits(a as u32),
                f32::from_bits(b as u32),
                f32::from_bits(c as u32),
            );
            let r = if self.hw_fma {
                unsafe { fma32_hw(x, y, z) }
            } else {
                x.mul_add(y, z)
            };
            Want::Bits(fm, r.to_bits() as u64)
        };
        self.push(q, w, "fma");
    }

    fn unop(&mut self, fm: Fmt, op: &'static str, a: u64) {
        let q = format!("{} {} {:x}", fm.tag, op, a);
        let w = if fm.tag == "d" {
            let x = f64::from_bits(a);
            match op {
                "sqrt" => Want::Bits(fm, x.sqrt().to_bits()),
                "neg" => Want::Bits(fm, (-x).to_bits()),
                "abs" => Want::Bits(fm, x.abs().to_bits()),
                "f2i" => Want::Int(x as i64),
                _ => unreachable!(),
            }
        } else {
            let x = f32::from_bits(a as u32);
            match op {
                "sqrt" => Want::Bits(fm, x.sqrt().to_bits() as u64),
                "neg" => Want::Bits(fm, (-x).to_bits() as u64),
                "abs" => Want::Bits(fm, x.abs().to_bits() as u64),
                "f2i" => Want::Int(x as i64),
                _ => unreachable!(),
            }
        };
        self.push(q, w, op);
    }

    fn i2f(&mut self, fm: Fmt, n: i64) {
        let q = format!("{} i2f {}", fm.tag, n);
        let w = if fm.tag == "d" {
            Want::Bits(fm, (n as f64).to_bits())
        } else {
            Want::Bits(fm, (n as f32).to_bits() as u64)
        };
        self.push(q, w, "i2f");
    }
}

fn main() {
    let mut exe = String::from("../.lake/build/bin/ieee754-eval");
    let mut n: u64 = 200_000;
    let mut seed: u64 = 0x1eee754;
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--exe" => {
                exe = args[i + 1].clone();
                i += 1;
            }
            "--n" => {
                n = args[i + 1].parse().expect("--n");
                i += 1;
            }
            "--seed" => {
                seed = args[i + 1].parse().expect("--seed");
                i += 1;
            }
            a => panic!("unknown argument {a}"),
        }
        i += 1;
    }
    let hw_fma = std::arch::is_x86_feature_detected!("fma");
    let mut r = Rng::new(seed);
    let mut g = Gen {
        ties: 0,
        queries: vec![],
        wants: vec![],
        ops: vec![],
        hw_fma,
    };

    for (fm, per) in [(D, n), (S, n / 4)] {
        let edges = fm.edges();
        // every pair of edge values, for every binary operation
        for &a in &edges {
            for &b in &edges {
                for op in ["add", "sub", "mul", "div", "eq", "lt", "le"] {
                    g.binop(fm, op, a, b);
                }
            }
            for op in ["sqrt", "neg", "abs", "f2i"] {
                g.unop(fm, op, a);
            }
        }
        for _ in 0..per {
            let a = fm.operand(&mut r, &edges);
            let b = fm.partner(&mut r, a, &edges);
            for op in ["add", "sub", "mul", "div"] {
                g.binop(fm, op, a, b);
            }
            if r.below(4) == 0 {
                for op in ["eq", "lt", "le"] {
                    g.binop(fm, op, a, b);
                }
            }
            g.unop(fm, "sqrt", a);
            // fma: an independent addend, or minus the rounded product (exposes its error)
            let c = match r.below(3) {
                0 => {
                    let p = if fm.tag == "d" {
                        (f64::from_bits(a) * f64::from_bits(b)).to_bits()
                    } else {
                        (f32::from_bits(a as u32) * f32::from_bits(b as u32)).to_bits() as u64
                    };
                    p ^ (1u64 << (fm.ebits + fm.fbits))
                }
                1 => fm.partner(&mut r, a, &edges),
                _ => fm.operand(&mut r, &edges),
            };
            g.fma(fm, a, b, c);
            if r.below(8) == 0 {
                g.unop(fm, "f2i", a);
                // integers: any i64, small, around 2^p and around ±2^63
                let k = match r.below(4) {
                    0 => r.next() as i64,
                    1 => (r.next() as i64) >> r.below(64),
                    2 => (1i64 << (fm.fbits + 1 + r.below(4) as u32)) + r.below(64) as i64 - 32,
                    _ => i64::MAX - r.below(1 << 12) as i64,
                };
                g.i2f(fm, if r.coin() { k } else { k.wrapping_neg() });
            }
        }
        // f2i near the saturation bounds
        for k in 0..2000u64 {
            let x = (2f64.powi(63) * (1.0 - (k as f64) * 2f64.powi(-53))).to_bits();
            g.unop(D, "f2i", x);
            g.unop(D, "f2i", x ^ (1 << 63));
        }
    }

    let total = g.queries.len();
    eprintln!(
        "ieee754-check: {total} queries (fma on this CPU: {})",
        if hw_fma { "vfmadd" } else { "libm" }
    );
    let start = std::time::Instant::now();
    let mut child = Command::new(&exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot run {exe}: {e}"));
    let mut stdin = BufWriter::new(child.stdin.take().unwrap());
    let queries = std::mem::take(&mut g.queries);
    let writer = std::thread::spawn(move || {
        for q in &queries {
            writeln!(stdin, "{q}").unwrap();
        }
        stdin.flush().unwrap();
        queries
    });
    let reader = BufReader::new(child.stdout.take().unwrap());
    let mut bad = 0usize;
    let mut answers = 0usize;
    let mut per_op: std::collections::BTreeMap<&str, (usize, usize)> = Default::default();
    let mut lines = reader.lines();
    let mut mismatches = vec![];
    // what the hardware results were: zero, subnormal, normal, infinite, NaN
    let mut kinds = [0usize; 5];
    for (k, w) in g.wants.iter().enumerate() {
        let got = lines.next().expect("model ended early").unwrap();
        answers += 1;
        let ok = match *w {
            Want::Bits(fm, b) => match u64::from_str_radix(&got, 16) {
                Ok(m) => (fm.is_nan(b) && fm.is_nan(m)) || b == m,
                Err(_) => false,
            },
            Want::Bool(v) => got == if v { "1" } else { "0" },
            Want::Int(v) => got.parse::<i64>().ok() == Some(v),
        };
        if let Want::Bits(fm, b) = *w {
            let frac = b & fm.fmask();
            let kind = match fm.bexp(b) {
                0 if frac == 0 => 0,
                0 => 1,
                e if e == fm.emask() && frac == 0 => 3,
                e if e == fm.emask() => 4,
                _ => 2,
            };
            kinds[kind] += 1;
        }
        let e = per_op.entry(g.ops[k]).or_default();
        e.0 += 1;
        if !ok {
            e.1 += 1;
            bad += 1;
            mismatches.push((k, got, *w));
        }
        if answers % 1_000_000 == 0 {
            eprintln!("  {answers} answered, {bad} mismatches, {:.0?}", start.elapsed());
        }
    }
    let queries = writer.join().unwrap();
    child.wait().unwrap();
    for (k, got, w) in mismatches.iter().take(30) {
        let want = match w {
            Want::Bits(_, b) => format!("{b:x}"),
            Want::Bool(v) => format!("{}", *v as u8),
            Want::Int(v) => format!("{v}"),
        };
        println!("MISMATCH {} -> model {got}, hardware {want}", queries[*k]);
    }
    println!("{:>6} {:>10} {:>10}", "op", "cases", "mismatch");
    for (op, (c, b)) in &per_op {
        println!("{op:>6} {c:>10} {b:>10}");
    }
    println!("binary64 add/mul cases whose exact result is a tie: {}", g.ties);
    println!(
        "float results: {} zero, {} subnormal, {} normal, {} infinite, {} NaN",
        kinds[0], kinds[1], kinds[2], kinds[3], kinds[4]
    );
    println!(
        "total {total} cases, {bad} mismatches, model time {:.1?} (fma: {})",
        start.elapsed(),
        if hw_fma { "hardware vfmadd" } else { "libm" }
    );
    if bad > 0 {
        std::process::exit(1);
    }
}
