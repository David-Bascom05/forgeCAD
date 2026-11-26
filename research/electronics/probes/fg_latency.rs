use forge_lang::{Options, build::Cache, run};
use std::{env, fs, time::Instant};

fn main() {
    let file = env::args()
        .nth(1)
        .expect("path to interface/forge-lang/parts/sheet/hat.fg");
    let src = fs::read_to_string(file).unwrap();
    let n: usize = env::args()
        .nth(2)
        .unwrap_or_else(|| "100".into())
        .parse()
        .unwrap();
    assert!(n > 0 && src.contains("shared w = 25"));
    for case in ["cold", "unchanged", "width", "undo", "type-error"] {
        let opts = Options {
            checks: false,
            ..Options::default()
        };
        let mut cache = Cache::default();
        if case != "cold" {
            let (r, c) = run(&src, opts.clone(), cache);
            assert!(r.ok());
            cache = c;
        }
        let mut elapsed = Vec::new();
        let mut reused = Vec::new();
        let mut rebuilt = Vec::new();
        let mut remade = Vec::new();
        for i in 0..n {
            let next = match case {
                "width" => src.replace(
                    "shared w = 25",
                    &format!("shared w = {:.3}", 25.001 + i as f64 * 0.001),
                ),
                "type-error" => src.replace("shared w = 25", &format!("shared w = {}deg", 26 + i)),
                _ => src.clone(),
            };
            if case == "cold" {
                cache = Cache::default();
            }
            if case == "undo" {
                let changed = src.replace("shared w = 25", "shared w = 26");
                let (r, c) = run(&changed, opts.clone(), cache);
                assert!(r.ok());
                cache = c;
            }
            if case == "type-error" {
                let (r, c) = run(&src, opts.clone(), cache);
                assert!(r.ok());
                cache = c;
            }
            let args = opts.clone();
            let t = Instant::now();
            let (r, c) = run(&next, args, cache);
            elapsed.push(t.elapsed().as_secs_f64() * 1000.);
            cache = c;
            if case == "type-error" {
                assert!(r.errors.iter().any(|e| e.kind() == "WrongDim"), "{:?}", r.errors);
                if i == 0 {
                    for e in &r.errors {
                        println!("{}: {} {:?}", e.kind(), e.text(), e.span());
                    }
                }
            } else {
                assert!(r.ok(), "{:?}", r.errors);
            }
            reused.push(r.reused);
            rebuilt.push(r.rebuilt);
            remade.push(r.remade);
        }
        elapsed.sort_by(f64::total_cmp);
        let p = |q: usize| elapsed[(n * q).div_ceil(100) - 1];
        println!(
            "{case} n={n} p50={:.3} p95={:.3} p99={:.3} ms reused={:?} rebuilt={:?} remade={:?}",
            p(50),
            p(95),
            p(99),
            range(reused),
            range(rebuilt),
            range(remade)
        );
    }
}
fn range(values: Vec<usize>) -> (usize, usize) {
    (*values.iter().min().unwrap(), *values.iter().max().unwrap())
}
