//! A component of a subassembly file lists its own features in the browser, read from that
//! file: a call by its definition (nested calls too), a body by its feature.

use forge_render::shell::asmpart::{infile_rows, read_feats};
use std::path::Path;

#[test]
fn a_subassembly_files_components_list_their_features() {
    let ex = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/unified"));
    let feats = read_feats("nested_boxes.fg", Some(ex)).unwrap();
    let ids =
        |rows: Vec<forge_render::shell::fg::FeatRow>| rows.into_iter().map(|r| r.id).collect::<Vec<_>>();
    // the lid inside a box, by its call's key there
    let lid = ids(infile_rows(&feats, "#box(w=40,h=20)/lid(w=40)", &[], &[]));
    assert!(lid.contains(&"plate".to_string()), "{lid:?}");
    let bx = ids(infile_rows(&feats, "#box(w=40,h=20)", &[], &[]));
    assert!(bx.contains(&"base".to_string()), "{bx:?}");
    // a file that is no Forge file says so, and nothing of it
    let d = std::env::temp_dir().join(format!("asm-sub-rows-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("x.fg"), "CANARY-99 not forge at all\n").unwrap();
    let e = read_feats("x.fg", Some(&d)).unwrap_err();
    assert!(!e.contains("CANARY"), "{e}");
}
