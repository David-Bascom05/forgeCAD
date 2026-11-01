//! Named drags, step by step: dragplan PART NAME[+NAME] [steps] [swing]. Prints each step's Mi and each line's.
use cputime::mi;
use forge_agent::bench;
use forge_agent::tools::{Dim, Part};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let src = bench::src(&a[0]).expect("a bench part");
    let names: Vec<&str> = a[1].split('+').collect();
    let steps: usize = a.get(2).map_or(16, |s| s.parse().unwrap());
    let swing: f64 = a.get(3).map_or(0.02, |s| s.parse().unwrap());
    let mut p = Part::of(bench::model(src));
    let dims: Vec<Dim> = p.dims();
    let picked: Vec<&Dim> = names.iter().map(|n| dims.iter().find(|d| d.name == *n).expect("a number")).collect();
    let mut srcs = vec![];
    for k in 1..=steps {
        let s = forge_math::dmath::sin(k as f64 / steps as f64 * core::f64::consts::TAU);
        for d in &picked {
            p.model.edit(d.at.op(forge_agent::short(d.value * (1.0 + swing * s)))).unwrap();
        }
        srcs.push(p.model.src().to_string());
    }
    let mut m = forge_agent::model::Model::default();
    m.fonts = p.model.fonts.clone();
    m.update(p.model.src());
    let verbose = std::env::var_os("LINES").is_some();
    for (k, s) in srcs.iter().enumerate() {
        forge_bool::boolean::probe::take();
        forge_bool::boolean::probe::next_frame();
        let m0 = mi();
        m.update(s);
        let c = mi() - m0;
        let (pm, pn) = forge_bool::boolean::probe::take();
        println!("pairs {} n {:?}", pm.iter().map(|x| format!("{x:.2}")).collect::<Vec<_>>().join(" "), pn);
        println!("step {k} {c:.1} Mi rebuilt {} skipped {} err {:?}", m.rebuilt, m.skipped, m.error.as_ref().map(|e| e.msg.clone()));
        if verbose {
            for l in &m.costs {
                println!("  line {} {} {:.2} {:.2}", l.0, m.kinds[l.0], l.1, l.2);
            }
        }
    }
}
