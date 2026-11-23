//! PROBE (temporary, not committed): per part and path, the result's counts, validity and volume.
use forge_model::{suite, torture};

fn main() {
    let mut parts = suite::parts();
    parts.push(("torture", torture::part(221)));
    let fuzz: Vec<_> = (1..=40).map(|s| suite::fuzz(s, 80)).collect();
    for (name, part) in parts.iter().map(|(n, p)| (n.to_string(), p)).chain(
        fuzz.iter()
            .enumerate()
            .map(|(i, p)| (format!("fuzz{}", i + 1), p)),
    ) {
        for (path, r) in [("seq", part.regenerate()), ("nary", part.regenerate_nary(4))] {
            match r {
                Ok(b) => {
                    let b = b.compact();
                    let probs = forge_topo::check(&b).len();
                    let v = forge_topo::mass::mass(&b).map(|m| m.volume).unwrap_or(f64::NAN);
                    println!(
                        "{name} {path} F{} E{} V{} bad{} vol {:.9}",
                        b.faces.len(),
                        b.edges.len(),
                        b.vertices.len(),
                        probs,
                        v
                    );
                }
                Err(e) => println!("{name} {path} ERR {e:?}"),
            }
        }
    }
}
