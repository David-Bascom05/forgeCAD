//! Named drags, what a frame's regen costs: dragcost PART NAME[+NAME] STEPS PLAYED. Copy to interface/forge-agent/examples/.
//! PLAYED 0 measures the set-up: a frame is (count at N - count at 0) / N, `research/drag-speed/run.sh`.
use forge_agent::bench;
use forge_agent::tools::{Dim, Part};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let src = bench::src(&a[0]).expect("a bench part");
    let names: Vec<&str> = a[1].split('+').collect();
    let steps: usize = a.get(2).map_or(16, |s| s.parse().unwrap());
    let mut p = Part::of(bench::model(src));
    let dims: Vec<Dim> = p.dims();
    let picked: Vec<&Dim> = names
        .iter()
        .map(|n| dims.iter().find(|d| d.name == *n).expect("a number"))
        .collect();
    let mut srcs = vec![];
    for k in 1..=steps {
        let s = forge_math::dmath::sin(k as f64 / steps as f64 * core::f64::consts::TAU);
        for d in &picked {
            p.model.edit(d.at.op(forge_agent::short(d.value * (1.0 + 0.02 * s)))).unwrap();
        }
        srcs.push(p.model.src().to_string());
    }
    let mut m = forge_agent::model::Model::default();
    m.fonts = p.model.fonts.clone();
    m.update(p.model.src());
    let played: usize = a.get(3).map_or(steps, |s| s.parse().unwrap());
    for s in &srcs[..played] {
        m.update(s);
        assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    }
}
