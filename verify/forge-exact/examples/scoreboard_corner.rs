//! The reliability scoreboard's `plane_plane/vertex_on_vertex/Add` (invalid there) against the bare
//! boolean on the same two boxes (valid): which step of forge-model's regen makes the difference.
//! cargo run --release -p forge-exact --example scoreboard_corner [-- NAME]
use forge_exact::{Outcome, block, kernel_on};
use forge_model::adversarial::{canonical, degenerate_parts};

fn main() {
    let want = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "plane_plane/vertex_on_vertex/Add".into());
    let (_, raw) = degenerate_parts()
        .into_iter()
        .find(|(n, _)| *n == want)
        .expect("a degenerate part");
    // the scoreboard runs `canonical(part)` (every frame rebuilt through `fr` until it is a fixed point)
    let canon = canonical(&raw);
    for (path, r) in [
        ("raw sequential", raw.regenerate()),
        ("raw nary", raw.regenerate_nary(1)),
        ("canonical sequential", canon.regenerate()),
        ("canonical nary", canon.regenerate_nary(1)),
    ] {
        match r {
            Err(e) => println!("{path}: error {e:?}"),
            Ok(b) => {
                let pr = forge_topo::check(&b);
                println!(
                    "{path}: {} faces {} edges {} vertices, {} problems",
                    b.face_ids().count(),
                    b.edge_ids().count(),
                    b.vertex_ids().count(),
                    pr.len()
                );
                for p in pr.iter().take(6) {
                    println!("  {p:?}");
                }
            }
        }
    }
    // the bare boolean on the same boxes: base (-20,-20,-20)..(20,20,0), tool (20,20,0)..(30,30,10)
    let a = block([[-20.0, 20.0], [-20.0, 20.0], [-20.0, 0.0]], 1);
    let b = block([[20.0, 30.0], [20.0, 30.0], [0.0, 10.0]], 2);
    let got = kernel_on(a, b, forge_bool::Op::Union);
    println!("bare boolean on forge-exact's blocks: {}", got.word());
    let _ = Outcome::Empty;
}
