//! (fuzz comparison) the fuzzer's verdict on each program given, and how many features built.
fn main() {
    for p in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&p).unwrap();
        let (v, b) = forge_lang::fuzz::judge_built(&src);
        println!("{p}: {:?} built {}", v, b.map_or(0, |b| b.built()));
    }
}
