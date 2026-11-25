//! A build stopped part way (a newer edit's, Cancel: `forge_geom::budget::Watch`) keeps nothing
//! it made after the stop: the same text built again is built afresh and gives what a build never
//! stopped gives, bit for bit, not the refusal the stop caused.

use forge_lang::build::Cache;
use forge_lang::{Options, Run};
use std::path::{Path, PathBuf};

/// VR-14 #178820 (12 faces, from the Haas machine models with tools/kernel-stress/extract.py): a
/// bore through it is a boolean on its NURBS faces, cut short by a stop past a small slice.
fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("forge-cancel-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/vr14_bore_12faces.step"),
        d.join("vr14.step"),
    )
    .unwrap();
    d
}

fn run(src: &str, dir: &Path, cache: Cache) -> (Run, Cache) {
    let opts = Options {
        dir: Some(dir.to_path_buf()),
        ..Options::default()
    };
    forge_lang::run(src, opts, cache)
}

fn bits(r: &Run) -> Vec<u64> {
    let Some(b) = &r.body else { return vec![] };
    let mut out = vec![b.face_ids().count() as u64, b.edge_ids().count() as u64];
    for v in &b.vertices {
        out.extend([v.point.x, v.point.y, v.point.z].map(f64::to_bits));
    }
    out.extend(b.face_ids().map(|f| b.face(f).name.0));
    out
}

/// The part: the solid, bored down through its middle from its top.
fn bored(dir: &Path) -> String {
    let (r, _) = run("i = import(\"vr14.step\")\n", dir, Cache::default());
    let bx = r.body.as_ref().expect("the solid").faces_box();
    let c = bx.center();
    let small = bx.x.width().min(bx.y.width()).min(bx.z.width());
    format!(
        "i = import(\"vr14.step\")\nsketch s(on: offset(xy, {})) {{\n  o = circle(({}, {}), {})\n}}\nb = cut(s, thru)\n",
        bx.z.hi,
        c.x,
        c.y,
        0.13 * small
    )
}

#[test]
fn a_stopped_build_keeps_nothing_and_the_same_text_again_builds_right() {
    let d = dir();
    let src = bored(&d);
    // never stopped: what the part is
    let spent = {
        let _all = forge_geom::budget::scoped(u64::MAX >> 1);
        let before = forge_geom::budget::left();
        let (fresh, _) = run(&src, &d, Cache::default());
        assert!(fresh.ok(), "{:?}", fresh.errors);
        (before - forge_geom::budget::left(), bits(&fresh))
    };
    assert!(spent.0 > 1024, "spends {}", spent.0);
    // stopped (a newer edit came): with small slices, the bore refuses at its first slice's end
    let w = forge_geom::budget::Watch::new();
    w.stop();
    let (cut_short, cache) = {
        let _w = forge_geom::budget::watched(w);
        forge_geom::budget::slice_for_tests(1024);
        let r = run(&src, &d, Cache::default());
        forge_geom::budget::slice_for_tests(forge_geom::budget::SLICE);
        r
    };
    assert!(!cut_short.ok(), "a stopped build answered as a whole one");
    // the same text again, on what the stopped build left: built afresh, and right
    let (again, _) = run(&src, &d, cache);
    assert!(again.ok(), "{:?}", again.errors);
    assert!(again.rebuilt > 0, "nothing built again");
    assert_eq!(bits(&again), spent.1);
}
