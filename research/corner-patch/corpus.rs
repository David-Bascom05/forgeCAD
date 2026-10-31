//! One fillet-corpus part's regen, for instruction counting with and without the rational
//! corner patch: `corpus list` names every case (cases, variant and variable), `corpus <name>`
//! regenerates one and prints its faces.
use forge_model::fillets;

fn main() {
    let want = std::env::args().nth(1).expect("a case name or list");
    let all = fillets::cases()
        .into_iter()
        .chain(fillets::variant_cases())
        .chain(fillets::variable_cases());
    for (name, part, _) in all {
        if want == "list" {
            println!("{name}");
        } else if name == want {
            match part.regenerate() {
                Ok(b) => println!("{name} {} faces", b.faces.len()),
                Err(e) => println!("{name} failed: {e:?}"),
            }
        }
    }
}
