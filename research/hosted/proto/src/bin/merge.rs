//! Concurrency of the typed edit ops, measured on real documents. For each .fg in the list
//! (argv[1]) it makes a set of candidate ops a user could send against the document as it is
//! (SetParam on each handle, Rename/DeleteFeature/MoveFeature/ReplaceFeature/SetArg-free on each
//! top-level feature, AddFeature of a new param at the end), takes pairs (a, b) both valid alone
//! on S, and applies them in both orders with apply_checked (the server's re-validation of a
//! rebased op). Each pair is classified:
//!   commute     both orders apply and give the same text
//!   diverge     both orders apply, texts differ (order matters: server order decides)
//!   refuse_ab   a then b: b refused after a (b's author sees a conflict) - only that order
//!   refuse_ba   b then a: a refused after b
//!   refuse_both both orders refuse the second op
//! Output: one line per op-kind pair with counts, and examples of non-commuting pairs.
use forge_lang::edit;
use forge_lang::error::Op;
use std::collections::BTreeMap;

fn kind(op: &Op) -> &'static str {
    match op {
        Op::SetParam { .. } => "SetParam",
        Op::AddFeature { .. } => "AddFeature",
        Op::DeleteFeature { .. } => "DeleteFeature",
        Op::ReplaceFeature { .. } => "ReplaceFeature",
        Op::MoveFeature { .. } => "MoveFeature",
        Op::Rename { .. } => "Rename",
        Op::SetArg { .. } => "SetArg",
        _ => "other",
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn main() {
    let list = std::env::args().nth(1).expect("list file");
    let per_doc: usize = std::env::args().nth(2).map_or(80, |s| s.parse().unwrap());
    let text = std::fs::read_to_string(&list).expect("list");
    let mut table: BTreeMap<(String, String), [usize; 5]> = BTreeMap::new();
    let mut same_target: BTreeMap<(String, String), [usize; 5]> = BTreeMap::new();
    let mut samples: Vec<String> = vec![];
    let mut rng = Rng(0x9e3779b97f4a7c15);
    let mut docs = 0;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let path = std::path::PathBuf::from(line.trim());
        let Ok(src) = std::fs::read_to_string(&path) else { continue };
        let dir = path.parent().map(|p| p.to_path_buf());
        let o = forge_lang::Options { build: false, dir: dir.clone(), ..Default::default() };
        let (run, _) = forge_lang::run(&src, o, Default::default());
        if !run.errors.is_empty() {
            continue;
        }
        docs += 1;
        let mut ops: Vec<(Op, String)> = vec![];
        for (k, lit) in edit::handles(&src) {
            let target = k.to_string();
            for f in [1.1, 0.9] {
                let mut v = lit;
                v.value = if v.value == 0.0 { f - 1.0 } else { v.value * f };
                ops.push((Op::SetParam { param: k.clone(), value: v }, target.clone()));
            }
        }
        let feats: Vec<_> = run.feats.iter().filter(|f| f.top_level).collect();
        for f in &feats {
            let id = f.id.clone();
            let stmt = src.get(f.stmt.lo as usize..f.stmt.hi as usize).unwrap_or("").to_string();
            ops.push((Op::Rename { from: id.clone(), to: format!("{id}_a") }, id.clone()));
            ops.push((Op::Rename { from: id.clone(), to: format!("{id}_b") }, id.clone()));
            ops.push((Op::DeleteFeature { id: id.clone() }, id.clone()));
            ops.push((Op::MoveFeature { id: id.clone(), before: None }, id.clone()));
            if !stmt.is_empty() {
                // a dialog's re-write of the statement: the same text with a trailing comment
                ops.push((Op::ReplaceFeature { id: id.clone(), text: format!("{stmt}  # edited") }, id.clone()));
            }
        }
        ops.push((Op::AddFeature { before: None, text: "shared hosted_a = 1".into() }, "+a".into()));
        ops.push((Op::AddFeature { before: None, text: "shared hosted_b = 2".into() }, "+b".into()));
        // ops valid alone on S
        let valid: Vec<(Op, String, String)> = ops
            .into_iter()
            .filter_map(|(op, t)| edit::apply_checked_in(&src, &op, dir.clone()).ok().map(|(s, _)| (op, t, s)))
            .collect();
        if valid.len() < 2 {
            continue;
        }
        for _ in 0..per_doc {
            let i = (rng.next() % valid.len() as u64) as usize;
            let j = (rng.next() % valid.len() as u64) as usize;
            if i == j {
                continue;
            }
            let (a, ta, sa) = &valid[i];
            let (b, tb, sb) = &valid[j];
            let ab = edit::apply_checked_in(sa, b, dir.clone()).map(|x| x.0);
            let ba = edit::apply_checked_in(sb, a, dir.clone()).map(|x| x.0);
            let c = match (&ab, &ba) {
                (Ok(x), Ok(y)) if x == y => 0,
                (Ok(_), Ok(_)) => 1,
                (Err(_), Ok(_)) => 2,
                (Ok(_), Err(_)) => 3,
                (Err(_), Err(_)) => 4,
            };
            let (ka, kb) = (kind(a).to_string(), kind(b).to_string());
            let key = if ka <= kb { (ka, kb) } else { (kb, ka) };
            let tab = if ta == tb { &mut same_target } else { &mut table };
            tab.entry(key.clone()).or_default()[c] += 1;
            if c != 0 && samples.len() < 60 && rng.next() % 4 == 0 {
                let why = |r: &Result<String, forge_lang::error::Error>| match r {
                    Ok(_) => "ok".to_string(),
                    Err(e) => e.kind().to_string(),
                };
                samples.push(format!(
                    "{} | {} vs {} | class={} | ab={} ba={}",
                    path.file_name().unwrap().to_string_lossy(),
                    a.json(),
                    b.json(),
                    ["commute", "diverge", "refuse_ab", "refuse_ba", "refuse_both"][c],
                    why(&ab),
                    why(&ba)
                ));
            }
        }
    }
    println!("docs {docs}");
    for (name, tab) in [("DIFFERENT TARGETS", &table), ("SAME TARGET (same feature/param)", &same_target)] {
        println!("== {name}\npair commute diverge refuse_ab refuse_ba refuse_both total");
        for ((a, b), c) in tab {
            let t: usize = c.iter().sum();
            println!("{a}+{b} {} {} {} {} {} {t}", c[0], c[1], c[2], c[3], c[4]);
        }
    }
    println!("== samples");
    for s in samples {
        println!("{s}");
    }
}
