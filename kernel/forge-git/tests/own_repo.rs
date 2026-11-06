//! ForgeCAD's own repository through this crate: objects read (deltas applied), hashed, and
//! written back byte for byte. The default test takes every pack and every 50th object; the
//! ignored one takes them all (`cargo test --release -p forge-git --test own_repo -- --ignored`).

use forge_git::object::{self, Kind};
use forge_git::{Oid, Repo};

fn check(r: &Repo, id: &Oid, o: &forge_git::odb::Object) {
    assert_eq!(object::id_of(r.hash, o.kind, &o.data), *id, "hash of {id}");
    let back = match o.kind {
        Kind::Tree => object::write_tree(&object::parse_tree(r.hash, &o.data).unwrap()),
        Kind::Commit | Kind::Tag => object::Headed::parse(&o.data).unwrap().write(),
        Kind::Blob => return,
    };
    assert_eq!(back, o.data, "round trip of {id}");
}

fn run(step: usize) -> usize {
    let r = Repo::discover(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
    let mut n = 0;
    for (pi, p) in r.odb.packs().iter().enumerate() {
        let mut at: Vec<(u64, usize)> = (0..p.len()).map(|i| (p.offset_at(i), i)).collect();
        at.sort_unstable();
        for (k, (off, i)) in at.into_iter().enumerate() {
            if k % step != 0 {
                continue;
            }
            let o = r.odb.read_packed(pi, p, off).unwrap();
            check(&r, &p.id_at(i), &o);
            n += 1;
        }
    }
    for (k, id) in r.odb.loose_ids().iter().enumerate() {
        if k % step != 0 {
            continue;
        }
        // a loose object git packs and prunes meanwhile is no failure
        if let Ok(Some(o)) = r.odb.read_loose(id) {
            check(&r, id, &o);
            n += 1;
        }
    }
    n
}

#[test]
fn every_pack_and_a_slice_of_objects() {
    assert!(run(50) > 100);
}

#[test]
#[ignore]
fn every_object() {
    let t = std::time::Instant::now();
    let n = run(1);
    eprintln!("{n} objects in {:.2?}", t.elapsed());
}
