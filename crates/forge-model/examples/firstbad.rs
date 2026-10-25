//! Regenerates the torture part feature by feature, checking every intermediate solid.
use forge_model::{Part, torture};
fn main() {
    let all = torture::part(usize::MAX).features;
    for n in 1..=all.len() {
        let part = Part {
            features: all[..n].to_vec(),
        };
        match part.regenerate() {
            Ok(b) => {
                let p = forge_topo::check(&b);
                if !p.is_empty() {
                    println!(
                        "feature {} produces an invalid body: {:?}",
                        n - 1,
                        &p[..p.len().min(4)]
                    );
                    return;
                }
            }
            Err(e) => {
                println!("feature {} fails: {e:?}", n - 1);
                return;
            }
        }
    }
    println!("all {} features valid", all.len());
}
