//! (scratch, not committed) the regen error of fuzz seeds: `zz_fallback_err <from> <to> [count]`
use forge_model::suite;

fn main() {
    let a: Vec<u64> = std::env::args().skip(1).map(|x| x.parse().unwrap()).collect();
    let count = a.get(2).copied().unwrap_or(80) as usize;
    for s in a[0]..=a[1] {
        let p = suite::fuzz(s, count);
        let seq = p.regenerate().err().map(|e| format!("{e:?}"));
        let nary = p.regenerate_nary(1).err().map(|e| format!("{e:?}"));
        if seq.is_some() || nary.is_some() {
            println!("fuzz{s}: seq {seq:?}\n  nary1 {nary:?}");
        }
    }
}
