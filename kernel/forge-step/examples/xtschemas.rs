//! Each embedded-schema text file's records, one line a node type: `schema<TAB>type<TAB>hex`.
//! research/parasolid/known_schemas.py merges them into src/xt/known.rs.
fn main() {
    for f in std::env::args().skip(1) {
        let Ok(d) = std::fs::read(&f) else { continue };
        match forge_step::xt::schema_records(&d, &forge_step::Limits::default()) {
            Ok((schema, recs)) => {
                for (ty, r) in recs {
                    let hex: String = r.iter().map(|b| format!("{b:02x}")).collect();
                    println!("{schema}\t{ty}\t{hex}");
                }
            }
            Err(e) => eprintln!("{f}: {e}"),
        }
    }
}
