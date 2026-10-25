//! cargo run --release -p forge-model --example shrink -- <suite part name | fuzz seed>
//! Shrinks a failing history to a minimal one and prints it as a literal.
use forge_model::{adversarial, suite};
fn main() {
    let arg = std::env::args().nth(1).expect("suite part name or fuzz seed");
    let part = match arg.parse::<u64>() {
        Ok(seed) => suite::fuzz(seed, 80),
        Err(_) => {
            suite::parts()
                .into_iter()
                .find(|(n, _)| *n == arg)
                .expect("no such part")
                .1
        }
    };
    let small = adversarial::shrink(&part);
    println!("{:?}", adversarial::outcome(&small));
    println!("{}", adversarial::literal(&small));
    // with FORGE_DEBUG set, the diagnostics after this line are the minimal part's
    eprintln!("=== rerun");
    println!("{:?}", small.regenerate().err());
}
