//! Every consistent sign pattern of the box-box family through the kernel, against the exact answer.
//! cargo run --release -p forge-exact --example box_table -- [unit] [--all]
//! One line per pattern the kernel gets wrong (with --all, every pattern): label, kernel, exact.
use forge_exact::{Pattern, exact, kernel};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let unit: f64 = args.iter().find_map(|a| a.parse().ok()).unwrap_or(10.0);
    let all = args.iter().any(|a| a == "--all");
    let (mut n, mut wrong) = (0, 0);
    for p in Pattern::all() {
        let (k, e) = (kernel(&p, unit), exact(&p, unit));
        n += 1;
        let bad = !k.agrees(&e);
        wrong += bad as usize;
        if bad || all {
            println!("{}\t{}\t{}", p.label(), k.word(), e.word());
        }
    }
    eprintln!("{wrong} of {n} patterns differ from the exact answer (unit {unit})");
}
