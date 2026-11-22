//! Named drags, step by step: dragplan PART NAME[+NAME] [steps] [swing]. Prints each step's Mi,
//! the boolean's face pairs (probe.patch's counters), and what the run rebuilt and reused.
//! PART is a Forge bench part (`parts/bench/PART.fg`), NAME its params (`L`, `bolt.cb`), each value
//! a SetParam as a drag of its field writes it, each step `forge_lang::run` on the session's
//! cache, the part's shape only. Copy to interface/forge-lang/examples/.
use cputime::mi;
use forge_lang::bench;
use forge_lang::error::{Literal, Op};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let src = bench::src(&a[0]).expect("a bench part");
    let names: Vec<&str> = a[1].split('+').collect();
    let steps: usize = a.get(2).map_or(16, |s| s.parse().unwrap());
    let swing: f64 = a.get(3).map_or(0.02, |s| s.parse().unwrap());
    let nums = forge_lang::edit::handles(src);
    let picked: Vec<_> = names.iter().map(|n| nums.iter().find(|(k, _)| k.to_string() == *n).expect("a number")).collect();
    let mut srcs = vec![];
    let mut text = src.to_string();
    for k in 1..=steps {
        let s = forge_math::dmath::sin(k as f64 / steps as f64 * core::f64::consts::TAU);
        for (key, was) in &picked {
            let value = Literal { value: was.value * (1.0 + swing * s), dim: was.dim };
            text = forge_lang::edit::apply(&text, &Op::SetParam { param: key.clone(), value }).unwrap();
        }
        srcs.push(text.clone());
    }
    let dir = std::env::temp_dir().join(format!("dragplan-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    std::fs::write(dir.join(bench::FONT), bench::font_bytes()).expect("the font");
    let opts = forge_lang::Options { dir: Some(dir.clone()), checks: false, ..Default::default() };
    let (_, mut cache) = forge_lang::run(src, opts.clone(), Default::default());
    let verbose = std::env::var_os("LINES").is_some();
    for (k, s) in srcs.iter().enumerate() {
        forge_bool::boolean::probe::take();
        forge_bool::boolean::probe::next_frame();
        let m0 = mi();
        let (run, c) = forge_lang::run(s, opts.clone(), cache);
        cache = c;
        let c = mi() - m0;
        let (pm, pn) = forge_bool::boolean::probe::take();
        println!("pairs {} n {:?}", pm.iter().map(|x| format!("{x:.2}")).collect::<Vec<_>>().join(" "), pn);
        println!(
            "step {k} {c:.1} Mi rebuilt {} reused {} err {:?}",
            run.rebuilt,
            run.reused,
            run.errors.first().map(|e| e.text())
        );
        if verbose {
            // each feature and how it came out; forge-lang keeps no cost per feature
            for f in &run.feats {
                println!("  feature {} {} {:?}", f.id, f.kind, f.status);
            }
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}
