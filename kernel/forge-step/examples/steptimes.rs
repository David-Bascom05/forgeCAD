//! Where a read's time goes, check by check (`--features times`): wall time of `import_drawn`,
//! then each phase's time summed over solids and threads. `steptimes FILE...`; with
//! `STEPTIMES_EVALS=k`, under k times the default eval budget (the app reads machine models at 16).
fn main() {
    for path in std::env::args().skip(1) {
        let text = std::fs::read(&path).expect("read");
        forge_step::read::times::counting(std::env::var_os("STEPTIMES_OFF").is_none());
        let _ = forge_step::read::times::take();
        let t = std::time::Instant::now();
        let d = forge_step::Limits::default();
        let k: u64 = std::env::var("STEPTIMES_EVALS").ok().and_then(|k| k.parse().ok()).unwrap_or(1);
        let lim = forge_step::Limits {
            max_evals: d.max_evals * k,
            ..d
        };
        let r = forge_step::import_drawn(&text, &lim);
        let wall = t.elapsed().as_secs_f64();
        let ph = forge_step::read::times::take();
        let mut line = format!("{path} wall {wall:.3}");
        for (n, s) in forge_step::read::times::NAMES.iter().zip(ph) {
            line += &format!(" | {n} {s:.3}");
        }
        if r.is_err() {
            line += " | err";
        }
        println!("{line}");
    }
}
