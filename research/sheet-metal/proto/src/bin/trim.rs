//! Option A against Option B on the same cases.
//!   trim verify OPT KIND N SEED0 SEED1   one line per case: verdict, area against the reference
//!   trim all KIND N SEED0 SEED1          verify for a, bseq and bnary against one reference
//!   trim cost OPT KIND N SEED0 SEED1     the work alone, for `perf stat -e instructions:u`
//! OPT: a | bseq | bnary | none (generating the cases only, to subtract).
use sheet_proto::cases::{Case, Kind, case};
use sheet_proto::prism::{read_back, subtract_nary, subtract_seq};
use sheet_proto::region::{Region, subtract};
use sheet_proto::verify::{truth, verdict};
use std::panic::{AssertUnwindSafe, catch_unwind};

fn run(opt: &str, c: &Case) -> Result<Region, String> {
    match opt {
        "a" => subtract(&c.wall, &c.cuts).map_err(|e| format!("{e:?}")),
        "bseq" => subtract_seq(&c.wall, &c.cuts)
            .and_then(|b| read_back(&b))
            .map_err(|e| format!("{e:?}")),
        "bnary" => subtract_nary(&c.wall, &c.cuts)
            .and_then(|b| read_back(&b))
            .map_err(|e| format!("{e:?}")),
        _ => Ok(Region::default()),
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mode = a[1].as_str();
    if mode == "bits" {
        // Option A's answers, bit for bit: the same lines on every target
        for kind in Kind::ALL {
            for n in [2, 20, 200] {
                for seed in 0..5 {
                    let c = case(kind, n, seed);
                    let mut h = forge_math::hash::BitHasher::new();
                    match subtract(&c.wall, &c.cuts) {
                        Ok(r) => {
                            h.f64(r.area());
                            for part in &r.parts {
                                for l in std::iter::once(&part.outer).chain(part.holes.iter()) {
                                    for p in l {
                                        h.p2(p.at(0.0));
                                        h.p2(p.at(0.5));
                                    }
                                }
                            }
                        }
                        Err(_) => h.u64(u64::MAX),
                    }
                    println!("{} {n} {seed} {:016x}", kind.name(), h.finish());
                }
            }
        }
        return;
    }
    let a: Vec<&str> = if mode == "all" {
        [mode, "all"]
            .into_iter()
            .chain(a[2..].iter().map(String::as_str))
            .collect()
    } else {
        a[1..].iter().map(String::as_str).collect()
    };
    let opt = a[1];
    let kind = Kind::parse(a[2]).expect("a kind");
    let n: usize = a[3].parse().expect("a count");
    let (s0, s1): (u64, u64) = (a[4].parse().expect("seed"), a[5].parse().expect("seed"));
    let opts: Vec<&str> = if opt == "all" {
        vec!["a", "bseq", "bnary"]
    } else {
        vec![opt]
    };
    for seed in s0..s1 {
        let c = case(kind, n, seed);
        if mode == "cost" {
            let r = run(opt, &c);
            std::hint::black_box(r.map(|r| r.parts.len()).ok());
            continue;
        }
        if mode == "dump" {
            for (i, l) in c.cuts.iter().enumerate() {
                println!(
                    "cut {i}: {:?}",
                    l.segments.iter().map(|s| &s.0).collect::<Vec<_>>()
                );
            }
        }
        let want = truth(&c, 300_000);
        for opt in &opts {
            let out = catch_unwind(AssertUnwindSafe(|| run(opt, &c)));
            let line = match out {
                Err(_) => "PANIC".to_string(),
                Ok(Err(e)) => format!("REFUSED {e}"),
                Ok(Ok(r)) => {
                    let got = r.area();
                    format!(
                        "{:?} area {got:.9} ref {:.9} err {:.1e} comps {}",
                        verdict(&c, &r, want, seed),
                        want.0,
                        want.1,
                        r.parts.len()
                    )
                }
            };
            println!("{opt} {} {n} {seed} {line}", kind.name());
        }
    }
}
