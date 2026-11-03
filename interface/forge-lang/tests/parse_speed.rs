//! Parse time of the three parts (lex and parse, best of many runs). Prints microseconds; run in
//! release with --nocapture to read them.

use forge_lang::syntax;
use std::time::Instant;

#[test]
fn parse_time_of_the_parts() {
    for name in ["bracket", "crank", "manifold"] {
        let path = format!("{}/parts/{name}.fg", env!("CARGO_MANIFEST_DIR"));
        let src = std::fs::read_to_string(path).expect("part");
        let mut best = f64::MAX;
        let mut total = 0.0;
        let n = 2000;
        for _ in 0..n {
            let t = Instant::now();
            let p = syntax::parse(&src).expect("parses");
            let dt = t.elapsed().as_secs_f64() * 1e6;
            std::hint::black_box(p);
            best = best.min(dt);
            total += dt;
        }
        let mut lex_best = f64::MAX;
        for _ in 0..n {
            let t = Instant::now();
            std::hint::black_box(syntax::lex(&src).expect("lexes"));
            lex_best = lex_best.min(t.elapsed().as_secs_f64() * 1e6);
        }
        println!("lex {name}: best {lex_best:.1} us");
        println!("parse {name}: best {best:.1} us, mean {:.1} us", total / n as f64);
    }
}
