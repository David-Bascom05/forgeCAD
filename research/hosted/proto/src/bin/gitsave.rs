//! Cost of today's history write per edit: forge-history's save (blob, tree, commit, ref, each
//! fsynced by forge-git) of a real document after each small edit. argv: DOC.fg N DIR
use hosted_proto::now_ms;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).expect("doc");
    let n: usize = a[2].parse().unwrap();
    let dir = std::path::PathBuf::from(&a[3]);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = forge_history::History::turn_on(&dir).expect("init");
    h.set_identity("bench", "bench@localhost").unwrap();
    let doc = dir.join("part.fg");
    let mut lat = vec![];
    for i in 0..n {
        std::fs::write(&doc, format!("{src}\nshared hosted_{i} = {i}\n")).unwrap();
        let t = now_ms();
        let r = h.save(&doc, &[], "SetParam", None).expect("save");
        lat.push(now_ms() - t);
        assert!(matches!(r, forge_history::Saved::New(_)), "{r:?}");
    }
    lat.sort_by(|x, y| x.partial_cmp(y).unwrap());
    println!(
        "forge-history save: doc {} bytes, n={n}, p50={:.3} ms p99={:.3} ms",
        src.len(),
        lat[n / 2],
        lat[(n * 99 / 100).min(n - 1)]
    );
}
