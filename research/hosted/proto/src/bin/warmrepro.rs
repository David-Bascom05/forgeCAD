//! warmrepro DOC PARAM VALUE warm|cold: one run of DOC, SetParam PARAM to VALUE (the param's
//! own unit), then a second run with the first run's Cache (warm) or an empty one (cold).
//! Prints both times, error kinds, and the second run's body hash.
use forge_lang::edit;
use forge_lang::error::Op;
use hosted_proto::*;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let path = std::path::PathBuf::from(&a[1]);
    let src = std::fs::read_to_string(&path).unwrap();
    let (key, mut lit) = edit::handles(&src).into_iter().find(|(k, _)| k.to_string() == a[2]).expect("param");
    lit.value = a[3].parse().unwrap();
    let t = now_ms();
    let (r1, cache) = forge_lang::run(&src, opts(&path), Default::default());
    let t1 = now_ms() - t;
    let next = edit::apply(&src, &Op::SetParam { param: key, value: lit }).unwrap();
    let cache = if a[4] == "warm" { cache } else { Default::default() };
    let t = now_ms();
    let (r2, _) = forge_lang::run(&next, opts(&path), cache);
    let t2 = now_ms() - t;
    let ek = |r: &forge_lang::Run| r.errors.iter().map(|e| format!("{}: {}", e.kind(), e.text().lines().next().unwrap_or(""))).collect::<Vec<_>>();
    let h = hash_run(&r2, 0.0);
    println!("first {t1:.1} ms errs {:?}", ek(&r1));
    println!("second ({}) {t2:.1} ms body {:016x} errs {:?}", a[4], h.body_hash, ek(&r2));
    println!("asm timings: {:?}", r2.asm.as_ref().map(|x| format!("{:?}", x.timings)));
}
