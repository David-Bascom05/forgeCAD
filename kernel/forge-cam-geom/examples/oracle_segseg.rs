//! L6 against the Lean oracle (verify/forge-cam/Oracle.lean on the -cam-lean branch):
//! seg–seg distances from slab::dist, checked against the proven exact segSegLo.
//!
//!   cargo run --release -p forge-cam-geom --example oracle_segseg -- gen 2000 cases ours
//!   (cd verify/forge-cam && lake env lean --run Oracle.lean < cases > answers)
//!   cargo run --release -p forge-cam-geom --example oracle_segseg -- check ours answers
//!
//! Points lie on the grid of 2^-20 with |x| < 2^10, so b − a is exact in f64 and every number
//! is written exactly as n/2^20. A case passes when the oracle's squared distance D (an exact
//! rational) lies in slab::dist's interval squared: lo² ≤ D ≤ hi² (hi = ∞ allowed). The
//! comparison rounds D to f64 once and allows one part in 2^40 for that rounding and the
//! squaring of the interval's ends: a test, not a proof.
use forge_cam_geom::slab::{Sec, dist};
use std::io::Write;

const K: f64 = 1048576.0; // 2^20

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    /// A grid point in [-span, span].
    fn g(&mut self, span: f64) -> f64 {
        ((self.next() * 2.0 - 1.0) * span * K).round() / K
    }
}

fn q(x: f64) -> String {
    format!("{}/{}", (x * K) as i64, K as i64)
}

fn cases(n: usize) -> Vec<[f64; 8]> {
    let mut r = Rng(0x9e3779b97f4a7c15);
    let mut out = Vec::new();
    for i in 0..n {
        let span = if i % 3 == 0 { 1.0 } else { 100.0 };
        let a = [r.g(span), r.g(span)];
        let b = [r.g(span), r.g(span)];
        let (c, d) = match i % 6 {
            // parallel, offset
            1 => {
                let o = [r.g(span / 10.0), r.g(span / 10.0)];
                ([a[0] + o[0], a[1] + o[1]], [b[0] + o[0], b[1] + o[1]])
            }
            // sharing an end
            2 => (b, [r.g(span), r.g(span)]),
            // collinear, overlapping or not
            3 => {
                let t = (r.next() * 4.0 - 1.5).round() / 2.0;
                let u = t + 0.5;
                let p = |s: f64| [a[0] + s * (b[0] - a[0]), a[1] + s * (b[1] - a[1])];
                (p(t), p(u))
            }
            // a point
            4 => {
                let p = [r.g(span), r.g(span)];
                (p, p)
            }
            _ => ([r.g(span), r.g(span)], [r.g(span), r.g(span)]),
        };
        // keep everything on the grid (the collinear case's points may not be)
        let on = |p: [f64; 2]| [(p[0] * K).round() / K, (p[1] * K).round() / K];
        out.push([a[0], a[1], b[0], b[1], on(c)[0], on(c)[1], on(d)[0], on(d)[1]]);
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("gen") => {
            let n: usize = args[1].parse().unwrap();
            let mut cf = std::fs::File::create(&args[2]).unwrap();
            let mut of = std::fs::File::create(&args[3]).unwrap();
            for c in cases(n) {
                let (a, b, cc, d) = ([c[0], c[1]], [c[2], c[3]], [c[4], c[5]], [c[6], c[7]]);
                writeln!(
                    cf,
                    "segseg {} {} {} {} {} {} {} {}",
                    q(a[0]),
                    q(a[1]),
                    q(b[0] - a[0]),
                    q(b[1] - a[1]),
                    q(cc[0]),
                    q(cc[1]),
                    q(d[0] - cc[0]),
                    q(d[1] - cc[1])
                )
                .unwrap();
                let iv = dist(&Sec::Seg { a, b }, &Sec::Seg { a: cc, b: d });
                writeln!(of, "{:e} {:e}", iv.lo, iv.hi).unwrap();
            }
        }
        Some("check") => {
            let ours = std::fs::read_to_string(&args[1]).unwrap();
            let ans = std::fs::read_to_string(&args[2]).unwrap();
            let (mut n, mut bad) = (0, 0);
            for (o, a) in ours.lines().zip(ans.lines()) {
                n += 1;
                let mut it = o.split_whitespace().map(|x| x.parse::<f64>().unwrap());
                let (lo, hi) = (it.next().unwrap(), it.next().unwrap());
                let a = a.trim();
                let d = match a.split_once('/') {
                    Some((p, q)) => p.parse::<f64>().unwrap() / q.parse::<f64>().unwrap(),
                    None => a.parse::<f64>().unwrap(),
                };
                let slack = 1.0 + 1.0 / (1u64 << 40) as f64;
                let ok = lo * lo <= d * slack && (hi.is_infinite() || d <= hi * hi * slack);
                if !ok {
                    bad += 1;
                    if bad <= 10 {
                        println!("case {n}: oracle {a} ({d:e}), ours [{lo:e}, {hi:e}]");
                    }
                }
            }
            println!("{n} cases, {bad} outside");
            if bad > 0 {
                std::process::exit(1);
            }
        }
        _ => eprintln!("gen N CASES OURS | check OURS ANSWERS"),
    }
}
