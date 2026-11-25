//! RSS after N runs of one document (validation's own call), with edits like the load test's.
fn rss_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status").unwrap().lines()
        .find(|l| l.starts_with("VmRSS:")).and_then(|l| l.split_whitespace().nth(1)).unwrap().parse().unwrap()
}
const DOC: &str = "shared w = 40\nshared h = 12\nsketch plan(on: xy) {\n  o = rect(center: (0, 0), size: (w, 30))\n}\nbase = extrude(plan, h)\nsketch ring(on: base.end) {\n  c = circle((0, 0), 6)\n}\nparam boss.d = 8\nboss = extrude(ring, d)\nsketch tab(on: xy) {\n  o = rect(center: (40, 0), size: (8, 8))\n}\nparam lid.t = 2\nlid = extrude(tab, t, new: true)\npost = extrude(tab, 9, new: true)\n";
fn main() {
    let opts = || forge_lang::Options { build: false, ..Default::default() };
    let mut last = rss_kb();
    println!("start {last} KB");
    for round in 0..6 {
        for i in 0..500 {
            let text = DOC.replace("shared w = 40", &format!("shared w = {}", 30.0 + ((round * 500 + i) % 200) as f64 / 10.0));
            let (run, _) = forge_lang::run(&text, opts(), Default::default());
            std::hint::black_box(&run);
            drop(run);
            if std::env::var_os("RELEASE").is_some() {
                for x in forge_keep::take_all() {
                    if let Ok(sk) = x.downcast::<std::cell::RefCell<forge_lang::sketch::Sk>>() {
                        let named = std::mem::take(&mut sk.borrow_mut().named);
                        drop(named);
                    }
                }
            }
        }
        let now = rss_kb();
        println!("after {} runs: {now} KB (+{} KB, {:.1} KB/run)", (round + 1) * 500, now as i64 - last as i64, (now as f64 - last as f64) / 500.0);
        last = now;
    }
}
