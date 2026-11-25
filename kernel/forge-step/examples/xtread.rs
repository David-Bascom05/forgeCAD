//! One JSON line per model file: what `forge_step::import` makes of it (solids, their names,
//! volumes and whether each checked out, refusals, the writer) and how long it took.
//!     cargo run --release -p forge-step --example xtread -- FILE...
use forge_step::{Limits, import};

fn esc(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '"' => vec!['\\', '"'],
            '\\' => vec!['\\', '\\'],
            c if (c as u32) < 32 => vec![' '],
            c => vec![c],
        })
        .collect()
}

fn main() {
    for f in std::env::args().skip(1) {
        let d = std::fs::read(&f).expect("read");
        let t = std::time::Instant::now();
        let r = import(&d, &Limits::default());
        let ms = t.elapsed().as_secs_f64() * 1e3;
        match r {
            Err(e) => println!(
                "{{\"file\":\"{}\",\"error\":\"{}\",\"ms\":{ms:.1}}}",
                esc(&f),
                esc(&e.to_string())
            ),
            Ok(imp) => {
                let solids: Vec<String> = imp
                    .solids
                    .iter()
                    .map(|s| {
                        format!(
                            "{{\"name\":\"{}\",\"faces\":{},\"volume\":{},\"verified\":{}{}}}",
                            esc(&s.name),
                            s.body.face_ids().count(),
                            s.mass
                                .as_ref()
                                .map_or("null".into(), |m| format!("{:.6}", m.volume)),
                            s.body.unverified.is_none(),
                            s.body
                                .unverified
                                .as_ref()
                                .map_or(String::new(), |w| format!(",\"why\":\"{}\"", esc(w)))
                        )
                    })
                    .collect();
                let refused: Vec<String> = imp
                    .refusals
                    .iter()
                    .map(|r| format!("\"{} #{}: {}\"", esc(&r.kind), r.entity, esc(&r.why)))
                    .collect();
                println!(
                    "{{\"file\":\"{}\",\"system\":\"{}\",\"schema\":\"{}\",\"ms\":{ms:.1},\"solids\":[{}],\"refused\":[{}]}}",
                    esc(&f),
                    esc(&imp.system),
                    esc(&imp.schema),
                    solids.join(","),
                    refused.join(",")
                );
            }
        }
    }
}
