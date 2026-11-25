//! Node census of Parasolid transmit files: `xtcensus FILE..`, one JSON line each.
fn main() {
    for p in std::env::args().skip(1) {
        let b = std::fs::read(&p).expect("readable file");
        let t = std::time::Instant::now();
        match forge_step::xt::census(&b, &forge_step::Limits::default()) {
            Ok((h, c)) => {
                let counts: Vec<String> = c.iter().map(|(t, n)| format!("\"{t}\":{n}")).collect();
                println!(
                    "{{\"file\":{p:?},\"ok\":true,\"ms\":{},\"appl\":{:?},\"schema\":{:?},\"binary\":{},\"types\":{{{}}}}}",
                    t.elapsed().as_millis(),
                    h.key("APPL").unwrap_or(""),
                    h.schema,
                    h.binary,
                    counts.join(",")
                );
            }
            Err(e) => println!("{{\"file\":{p:?},\"ok\":false,\"error\":{:?}}}", e.to_string()),
        }
    }
}
