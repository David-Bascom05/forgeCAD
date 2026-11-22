//! Named drags, what a frame's rebuild costs: dragcost PART NAME[+NAME] STEPS PLAYED. Copy to interface/forge-lang/examples/.
//! NAME is a param of the Forge bench part (`parts/bench/PART.fg`): `L`, `bolt.cb`. Each value is
//! a SetParam on the text, as a drag of its field writes it; each frame `forge_lang::run` on the
//! session's cache, the part's shape only (no checks); run with FORGECAD_CACHE=off, as run.sh does,
//! so nothing comes from results kept on disk.
//! PLAYED 0 measures the set-up: a frame is (count at N - count at 0) / N, `research/drag-speed/run.sh`.
use forge_lang::bench;
use forge_lang::error::{Literal, Op};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let src = bench::src(&a[0]).expect("a bench part");
    let names: Vec<&str> = a[1].split('+').collect();
    let steps: usize = a.get(2).map_or(16, |s| s.parse().unwrap());
    let nums = forge_lang::edit::handles(src);
    let picked: Vec<_> = names
        .iter()
        .map(|n| nums.iter().find(|(k, _)| k.to_string() == *n).expect("a number"))
        .collect();
    let mut srcs = vec![];
    let mut text = src.to_string();
    for k in 1..=steps {
        let s = forge_math::dmath::sin(k as f64 / steps as f64 * core::f64::consts::TAU);
        for (key, was) in &picked {
            let value = Literal { value: was.value * (1.0 + 0.02 * s), dim: was.dim };
            text = forge_lang::edit::apply(&text, &Op::SetParam { param: key.clone(), value }).unwrap();
        }
        srcs.push(text.clone());
    }
    let dir = std::env::temp_dir().join(format!("dragcost-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    std::fs::write(dir.join(bench::FONT), bench::font_bytes()).expect("the font");
    let opts = forge_lang::Options { dir: Some(dir.clone()), checks: false, ..Default::default() };
    let (_, mut cache) = forge_lang::run(src, opts.clone(), Default::default());
    let played: usize = a.get(3).map_or(steps, |s| s.parse().unwrap());
    for s in &srcs[..played] {
        let (run, c) = forge_lang::run(s, opts.clone(), cache);
        cache = c;
        assert!(run.errors.is_empty(), "{:?}", run.errors.first().map(|e| e.text()));
    }
    let _ = std::fs::remove_dir_all(dir);
}
