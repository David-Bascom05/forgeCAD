//! The server's per-op CPU if it validates each op as `fg edit` does (apply_checked: the op
//! applied to the text, both texts evaluated, no kernel build), and the cheaper apply alone.
//! One line per doc: apply_us check_ms (median of the first 5 handles' SetParams).
use forge_lang::edit;
use forge_lang::error::Op;
use hosted_proto::now_ms;

fn main() {
    let list = std::env::args().nth(1).expect("list");
    for line in std::fs::read_to_string(list).unwrap().lines() {
        let path = std::path::PathBuf::from(line.trim());
        let Ok(src) = std::fs::read_to_string(&path) else { continue };
        let dir = path.parent().map(|p| p.to_path_buf());
        let (mut ap, mut ck) = (vec![], vec![]);
        for (k, mut lit) in edit::handles(&src).into_iter().take(5) {
            lit.value = if lit.value == 0.0 { 0.5 } else { lit.value * 1.05 };
            let op = Op::SetParam { param: k, value: lit };
            let t = now_ms();
            if edit::apply(&src, &op).is_err() { continue; }
            ap.push((now_ms() - t) * 1e3);
            let t = now_ms();
            let _ = edit::apply_checked_in(&src, &op, dir.clone());
            ck.push(now_ms() - t);
        }
        if ap.is_empty() { continue; }
        ap.sort_by(|a, b| a.partial_cmp(b).unwrap());
        ck.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!("{} bytes={} apply_us={:.1} checked_ms={:.3}", path.display(), src.len(), ap[ap.len() / 2], ck[ck.len() / 2]);
    }
}
